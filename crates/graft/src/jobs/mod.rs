//! The job runner: the half of the loop that does the work.
//!
//! ```text
//! update ──▶ [Command] ──▶ JobRunner ──▶ (worker thread) ──▶ Event ──▶ update
//! ```
//!
//! `update` decides *what* should happen and returns [`Command`]s; nothing in
//! this module decides anything. It starts a small pool of worker threads, runs
//! one [`Job`](graft_core::job::JobId) per command on whichever worker is free,
//! and sends the result back down the same channel the keyboard uses. The UI
//! thread hands a command over and returns to drawing — it never joins a
//! worker, never blocks on a result, and never waits on a job (ARCHITECTURE.md,
//! PROMPT.md non-negotiable principle 3). That is why a two-minute fetch does
//! not freeze the keyboard.
//!
//! Every job ends in exactly one event, whatever happens to it:
//!
//! | What happened | What the loop is told |
//! |---|---|
//! | The work finished | `Event::JobFinished` |
//! | The work finished and brought something back | [`AppEvent::Produced`] |
//! | The work reported an error | `Event::JobFailed` with [`JobFailure::Failed`] |
//! | The work panicked | `Event::JobFailed` with [`JobFailure::Panicked`] |
//! | It was cancelled, before or during | `Event::JobCancelled` |
//! | It could not be started at all | `Event::JobFailed` |
//!
//! A job may also report [`AppEvent::Progress`] any number of times before
//! that one ending event — the history walk sends each chunk of laid-out rows
//! as it lands. Progress never ends a job and never stands in for its ending.
//!
//! There is no sixth outcome where a job simply disappears. A job that is over
//! and says nothing is a failure that ships (CLAUDE.md), so the only case where
//! no event is delivered is the one where the application itself has already
//! gone and there is nobody left to tell.
//!
//! Cancellation is cooperative, and it has to be: a thread cannot be safely
//! stopped from outside. Each job gets a [`Cancel`] token; long work checks it
//! between steps and returns [`Outcome::Cancelled`]. A job that is cancelled
//! before a worker picks it up never runs at all.

use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use graft_core::event::Event;
use graft_core::job::{JobFailure, JobId};

use graft_git::history::Order;

use crate::crash;
use crate::history::{self, Ended, RowRequest};
use crate::repos::RepoStore;
use crate::repository;
use crate::update::{AppEvent, Command, GitCheck, Product, Progress, Task};

#[cfg(test)]
mod tests;

/// The most workers Graft ever starts.
///
/// Four, on a laptop that is also running an editor and a browser (PROMPT.md
/// §7.1). Git work is mostly disk-bound anyway: more threads past this buys
/// little and costs the battery.
pub const MAX_WORKERS: usize = 4;

/// How long [`JobRunner::shutdown`] waits for jobs to notice they were
/// cancelled before it gives up on them.
///
/// Short on purpose. Quitting must feel immediate, and the terminal cannot be
/// restored until the loop returns (ticket 03), so a slow shutdown shows up as
/// a terminal that stays broken for as long as the wait.
pub const SHUTDOWN_GRACE: Duration = Duration::from_millis(200);

/// How a job ended, as the job itself sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The work is done.
    Done,
    /// The work is done and brought something back for the application to
    /// keep. Distinct from [`Outcome::Done`] only in that there is a value:
    /// "this path is not a repository" arrives here too, because it is an
    /// answer to the question that was asked, not a job that broke.
    Produced(Product),
    /// The work stopped early because its [`Cancel`] token was set.
    Cancelled,
    /// The work could not be completed, and this is what to tell the user.
    Failed(String),
}

/// One job's work: whatever the command asked for, as a closure a worker can
/// run.
///
/// Boxed because two commands rarely mean the same closure type, and owned
/// (`'static`) because it outlives the `update` call that asked for it.
pub type Work = Box<dyn FnOnce(&Cancel) -> Outcome + Send + 'static>;

/// A job's cancellation token.
///
/// Cloned into the job; the runner keeps the other half. Long work is expected
/// to check [`Cancel::is_cancelled`] between steps — a rev-walk once per batch
/// of commits, a `git` invocation between reading chunks of output.
#[derive(Clone, Debug)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Has this job been asked to stop?
    ///
    /// `Acquire`/`Release` rather than `Relaxed` so that whatever the
    /// cancelling thread did before it cancelled is visible to the job that
    /// sees the flag — cancellation is usually followed by tearing down the
    /// thing the job was working on.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
}

/// Where a job's result goes.
///
/// A seam. The event loop's channel carries [`Wake`](crate::event_loop::Wake)s
/// because the keyboard and the signal thread share it; a test wants plain
/// [`Event`]s and no loop at all. Neither should have to take the other's
/// shape, and the runner does not care which it is talking to.
///
/// `Clone` rather than `Sync`: each worker gets its own handle, which is what
/// an `mpsc::Sender` — the real implementation — actually supports.
pub trait JobEvents: Clone + Send + 'static {
    /// Deliver one job result.
    fn deliver(&self, event: AppEvent) -> Delivery;
}

/// What became of a delivered event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    /// The loop has it.
    Received,
    /// The other end is gone: the loop has already finished. A normal end to a
    /// run, not a failure — but it does mean no further result will be read by
    /// anyone.
    NobodyListening,
}

impl JobEvents for Sender<AppEvent> {
    fn deliver(&self, event: AppEvent) -> Delivery {
        match self.send(event) {
            Ok(()) => Delivery::Received,
            Err(_returned) => Delivery::NobodyListening,
        }
    }
}

/// Somewhere for the loop to hand a [`Command`].
///
/// A seam, so the loop can be driven in a test with a recorder instead of four
/// threads — and so that the one rule the loop must not break is visible in a
/// signature: `dispatch` takes a command and returns, with nothing a caller
/// could wait on.
pub trait Dispatch {
    /// Start the work this command names (or stop the job it names), and
    /// return immediately.
    fn dispatch(&self, command: Command);
}

/// A queued job, on its way to whichever worker is free first.
struct Assignment {
    id: JobId,
    cancel: Cancel,
    work: Work,
}

/// The jobs that have been submitted and are not over yet, so that a
/// cancellation can find its token.
#[derive(Default)]
struct Live {
    tokens: Mutex<HashMap<JobId, Cancel>>,
}

impl Live {
    fn register(&self, id: JobId, cancel: Cancel) {
        self.lock().insert(id, cancel);
    }

    /// Forget a job that is over. Its token is no longer anyone's business.
    fn finish(&self, id: JobId) {
        self.lock().remove(&id);
    }

    fn cancel(&self, id: JobId) {
        if let Some(token) = self.lock().get(&id) {
            token.cancel();
        }
    }

    fn cancel_all(&self) {
        for token in self.lock().values() {
            token.cancel();
        }
    }

    /// A panicking job never holds this lock — the worker takes it only to
    /// touch the map — so poisoning can only mean a panic somewhere that left
    /// the map itself intact. Refusing to cancel jobs because of that would
    /// turn one bug into a hang.
    fn lock(&self) -> MutexGuard<'_, HashMap<JobId, Cancel>> {
        self.tokens.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// How many workers are still running, so shutdown can wait for them without
/// blocking forever inside a `join`.
#[derive(Default)]
struct Alive {
    count: Mutex<usize>,
    all_stopped: Condvar,
}

impl Alive {
    fn started(&self) {
        *self.count.lock().unwrap_or_else(PoisonError::into_inner) += 1;
    }

    fn stopped(&self) {
        let mut count = self.count.lock().unwrap_or_else(PoisonError::into_inner);
        *count = count.saturating_sub(1);
        if *count == 0 {
            self.all_stopped.notify_all();
        }
    }

    /// Wait until every worker has stopped, or `grace` runs out. Returns how
    /// many are still going.
    fn wait_for_all(&self, grace: Duration) -> usize {
        let deadline = Instant::now() + grace;
        let mut count = self.count.lock().unwrap_or_else(PoisonError::into_inner);
        while *count > 0 {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return *count;
            };
            let (guard, _timeout) = self
                .all_stopped
                .wait_timeout(count, remaining)
                .unwrap_or_else(PoisonError::into_inner);
            count = guard;
        }
        0
    }
}

/// What happened when the runner was asked to stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shutdown {
    /// Every worker stopped and every job is accounted for.
    Clean,
    /// This many workers were still inside a job when the wait ran out.
    ///
    /// They were cancelled and did not notice in time — a job that does not
    /// check its token often enough, which is a bug worth hearing about. Their
    /// threads are left to die with the process rather than held on to, so
    /// that the terminal can be given back now.
    Stalled {
        /// How many workers were still busy.
        workers: usize,
    },
}

/// The pool: a queue, a few threads, and a table of what is still running.
///
/// Not `Sync`, and that is the design: one thread — the UI thread — owns the
/// runner and hands work to it. Everything shared with the workers is behind
/// the channel.
pub struct JobRunner<S: JobEvents> {
    next_id: AtomicU64,
    queue: Sender<Assignment>,
    live: Arc<Live>,
    alive: Arc<Alive>,
    workers: Vec<JoinHandle<()>>,
    /// Kept so the runner can report a job it could not even start.
    sink: S,
    /// The repository every job reads from, opened at most once for the whole
    /// run (`crate::repos`).
    repos: Arc<RepoStore>,
    /// Where each live history job takes its row requests from. Dropping a
    /// sender is what ends that job once it is idle, so a cancelled walk
    /// releases its worker instead of waiting for requests forever.
    row_requests: Mutex<HashMap<JobId, Sender<RowRequest>>>,
    /// Ahead/behind counts every refs job shares, so a refresh walks only
    /// the branches whose tips moved.
    ahead_behind: Arc<crate::refs::AheadBehindCache>,
}

impl<S: JobEvents> JobRunner<S> {
    /// Start a pool of `workers` threads, reporting results to `sink`.
    ///
    /// `workers` is clamped to at least one and at most [`MAX_WORKERS`]: a pool
    /// of none would accept jobs and never run them, and a pool of fifty would
    /// be a laptop with no battery left (PROMPT.md §7.1).
    pub fn start(sink: S, workers: usize, repos: Arc<RepoStore>) -> io::Result<Self> {
        let (queue, receiver) = std::sync::mpsc::channel();
        // `mpsc` has one receiver, and every worker needs to take from the same
        // queue, so the receiver itself is the thing they share. The lock is
        // held only across `recv` — never while a job runs.
        let receiver = Arc::new(Mutex::new(receiver));
        let live = Arc::new(Live::default());
        let alive = Arc::new(Alive::default());

        let mut handles = Vec::new();
        for number in 0..workers.clamp(1, MAX_WORKERS) {
            let worker = Worker {
                queue: Arc::clone(&receiver),
                live: Arc::clone(&live),
                alive: Arc::clone(&alive),
                sink: sink.clone(),
            };
            alive.started();
            match thread::Builder::new()
                .name(format!("graft-job-{number}"))
                .spawn(move || worker.run())
            {
                Ok(handle) => handles.push(handle),
                Err(error) => {
                    alive.stopped();
                    // A half-sized pool would work, and would quietly be slower
                    // forever. Better to fail where the cause is still visible:
                    // drop the queue so the workers that did start end, and let
                    // the caller say so.
                    drop(queue);
                    for handle in handles {
                        // These threads are idle in `recv` and end as soon as
                        // the queue is dropped, so this cannot hang.
                        handle.join().unwrap_or(());
                    }
                    return Err(error);
                }
            }
        }

        Ok(Self {
            next_id: AtomicU64::new(1),
            queue,
            live,
            alive,
            workers: handles,
            sink,
            repos,
            row_requests: Mutex::new(HashMap::new()),
            ahead_behind: Arc::default(),
        })
    }

    /// Start a pool sized for this machine.
    pub fn with_default_size(sink: S, repos: Arc<RepoStore>) -> io::Result<Self> {
        let parallelism = thread::available_parallelism().map_or(1, std::num::NonZero::get);
        Self::start(sink, parallelism.min(MAX_WORKERS), repos)
    }

    /// How many worker threads this pool is running.
    #[must_use]
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// Queue some work, and return the id its result will carry.
    ///
    /// Returns as soon as the work is on the queue: the caller is the UI
    /// thread, and it has a frame to draw.
    pub fn submit(&self, work: impl FnOnce(&Cancel) -> Outcome + Send + 'static) -> JobId {
        self.queue_job(None, |_| Box::new(work))
    }

    /// Queue some work and tell the loop, before it is queued, which id the
    /// result will carry.
    ///
    /// The announcement goes down the same channel as the result and is sent
    /// first, so the loop always learns the id before it learns the answer.
    /// Without that, `Esc` could not cancel a walk that has not come back
    /// yet: there would be nothing to name.
    fn announce(
        &self,
        task: Task,
        work: impl FnOnce(&Cancel) -> Outcome + Send + 'static,
    ) -> JobId {
        self.queue_job(Some(task), |_| Box::new(work))
    }

    /// Queue the work `build` makes once it knows the job's id — which work
    /// that reports progress needs, because every progress event carries it.
    fn queue_job(&self, task: Option<Task>, build: impl FnOnce(JobId) -> Work) -> JobId {
        let id = JobId::new(self.next_id.fetch_add(1, Ordering::Relaxed));
        let work = build(id);
        let cancel = Cancel::new();
        self.live.register(id, cancel.clone());

        if let Some(task) = task {
            // The announcement is deliberately not acted on either way. If
            // the loop has already finished, the job is still queued: the
            // worker that picks it up finds the same dead channel, and that
            // is the one place stopping the rest belongs. Deciding it twice,
            // in two places, is how the two come to disagree.
            self.sink.deliver(AppEvent::Started(id, task));
        }

        if let Err(undelivered) = self.queue.send(Assignment { id, cancel, work }) {
            // Every worker has gone, so this job can never run. Dropping it
            // quietly is exactly the failure that ships: the screen would wait
            // for something that is never coming.
            drop(undelivered);
            self.live.finish(id);
            self.sink.deliver(AppEvent::Core(Event::JobFailed(
                id,
                JobFailure::Failed("the job runner has stopped".to_owned()),
            )));
        }

        id
    }

    /// Ask a job to stop.
    ///
    /// Cooperative: a job that has not started yet is dropped without running,
    /// and one that is running stops at its next check. An id that is already
    /// finished or was never issued is a no-op — its result event is already on
    /// its way, and `update` decides what a result it no longer wants means.
    pub fn cancel(&self, id: JobId) {
        self.live.cancel(id);
    }

    /// The row-request senders, locked.
    fn row_requests(&self) -> MutexGuard<'_, HashMap<JobId, Sender<RowRequest>>> {
        // A poisoned lock is safe to take over: every change to the map is a
        // single insert, remove or clear, none of which can leave it half-done.
        self.row_requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Stop the pool: cancel everything, then wait up to `grace` for the
    /// workers to notice.
    ///
    /// Consumes the runner, because a pool that has been shut down must not be
    /// able to accept another job. Never blocks indefinitely — the terminal
    /// cannot be restored until this returns, so a job that ignores its token
    /// costs `grace` and then is abandoned rather than waited on forever.
    #[must_use]
    pub fn shutdown(self, grace: Duration) -> Shutdown {
        self.row_requests().clear();
        self.live.cancel_all();
        // Dropping the queue is what ends a worker that is idle: its `recv`
        // fails, and it stops.
        drop(self.queue);

        let still_busy = self.alive.wait_for_all(grace);
        if still_busy > 0 {
            // The handles are dropped rather than joined, which detaches those
            // threads. They hold nothing but their own job and a channel handle,
            // so the worst they can do is finish and find nobody listening.
            return Shutdown::Stalled {
                workers: still_busy,
            };
        }

        for handle in self.workers {
            // Every worker has already reached the end of its body, so this
            // returns at once. A worker that panicked outside a job is a bug in
            // this module rather than in a job; there is nothing to recover, and
            // the pool is being torn down anyway.
            handle.join().unwrap_or(());
        }
        Shutdown::Clean
    }
}

impl<S: JobEvents> Dispatch for JobRunner<S> {
    fn dispatch(&self, command: Command) {
        match command {
            Command::OpenRepository => {
                self.announce(
                    Task::OpenRepository,
                    open_repository(Arc::clone(&self.repos)),
                );
            }
            Command::CheckSystemGit => {
                self.announce(Task::CheckSystemGit, check_system_git());
            }
            Command::ReadRefs => {
                let repos = Arc::clone(&self.repos);
                let cache = Arc::clone(&self.ahead_behind);
                self.announce(Task::ReadRefs, move |cancel| {
                    if cancel.is_cancelled() {
                        return Outcome::Cancelled;
                    }
                    Outcome::Produced(Product::Refs(crate::refs::read(&repos, &cache)))
                });
            }
            Command::ReadWorkingTree => {
                let repos = Arc::clone(&self.repos);
                self.announce(Task::ReadWorkingTree, move |cancel| {
                    if cancel.is_cancelled() {
                        return Outcome::Cancelled;
                    }
                    Outcome::Produced(Product::WorkingTree(crate::working_tree::read(&repos)))
                });
            }
            Command::ReadHistory(order) => {
                let repos = Arc::clone(&self.repos);
                let sink = self.sink.clone();
                let (asks, requests) = std::sync::mpsc::channel();
                // Registered before `update` can see the job has started: the
                // announcement travels through the loop's queue, and this runs
                // on the loop's own thread before it reads that queue again.
                let id = self.queue_job(Some(Task::ReadHistory), |id| {
                    read_history(repos, order, requests, sink, id)
                });
                self.row_requests().insert(id, asks);
            }
            Command::ShowRows(id, request) => {
                let mut row_requests = self.row_requests();
                // No sender means the job was cancelled or has ended; its answer
                // would be dropped as stale anyway, so there is nobody to tell.
                if let Some(asks) = row_requests.get(&id)
                    && asks.send(request).is_err()
                {
                    // The job has ended on its own (it failed, or found the loop
                    // gone). Its failure is reported through its own outcome.
                    row_requests.remove(&id);
                }
            }
            Command::CancelJob(id) => {
                self.row_requests().remove(&id);
                self.cancel(id);
            }
        }
    }
}

/// The work behind [`Command::OpenRepository`].
///
/// Opening a repository touches the disk and can take a moment on a cold cache
/// or a network filesystem, which is exactly why it is a job and not something
/// `update` does. Only the first one pays: after that the store hands out a
/// handle onto a repository that is already open.
///
/// This is also what `Ctrl+R` re-runs, so it is where anything derived from
/// refs — the default branch, ADR-0001 — is thrown away and worked out again.
fn open_repository(store: Arc<RepoStore>) -> Work {
    Box::new(move |cancel| {
        if cancel.is_cancelled() {
            return Outcome::Cancelled;
        }
        store.refresh();
        // Whether it opened or not is the answer, and both halves go back the
        // same way: "not a repository" is what the user asked about.
        let facts = store
            .shared()
            .map_err(|error| error.to_string())
            .and_then(|repo| repository::facts(&repo.handle()));
        Outcome::Produced(Product::Repository(facts))
    })
}

/// The work behind [`Command::ReadHistory`], as job `id`.
///
/// The long one, and it stays: it answers the screen's row requests from
/// `requests` for as long as the walk is on screen, laying out only as far as
/// they reach. The token is checked before every chunk, so `Esc` stops it
/// within one; dropping the request sender ends it once idle.
fn read_history<S: JobEvents>(
    store: Arc<RepoStore>,
    order: Order,
    requests: Receiver<RowRequest>,
    sink: S,
    id: JobId,
) -> Work {
    Box::new(move |cancel| {
        if cancel.is_cancelled() {
            return Outcome::Cancelled;
        }
        let mut report = |update| sink.deliver(AppEvent::Progress(id, Progress::History(update)));
        let stop = || cancel.is_cancelled();
        match history::stream(&store, order, &requests, &stop, &mut report) {
            // Stopped by its token, its requests closed, or a report found the
            // loop gone — then
            // nobody reads the ending either, and the worker delivering it
            // finds the same dead channel and stops there (`Worker::run`).
            Ok(Ended::Cancelled | Ended::NobodyListening) => Outcome::Cancelled,
            Err(message) => Outcome::Failed(message),
        }
    })
}

/// The work behind [`Command::CheckSystemGit`].
///
/// Spawns `git --version` and waits for it, which is not something the UI
/// thread may do — on a cold `PATH` over a network mount it is not instant.
fn check_system_git() -> Work {
    Box::new(move |cancel| {
        if cancel.is_cancelled() {
            return Outcome::Cancelled;
        }
        let check = match graft_git::version::system_git() {
            Ok(found) => GitCheck::Ran(found),
            // Not a job failure: the job did its work and the answer is "this
            // machine's git cannot be asked". The user still needs to hear it.
            Err(error) => GitCheck::Unavailable(error.to_string()),
        };
        Outcome::Produced(Product::SystemGit(check))
    })
}

/// One worker thread: take the next job, run it, say what happened, repeat.
struct Worker<S: JobEvents> {
    queue: Arc<Mutex<Receiver<Assignment>>>,
    live: Arc<Live>,
    alive: Arc<Alive>,
    sink: S,
}

impl<S: JobEvents> Worker<S> {
    fn run(self) {
        loop {
            let Some(assignment) = self.next_assignment() else {
                // The queue is gone: the runner has shut down. Nothing is
                // wrong, and there is nothing left to run.
                break;
            };

            let id = assignment.id;
            let event = run_job(assignment);
            self.live.finish(id);

            match self.sink.deliver(event) {
                Delivery::Received => {}
                Delivery::NobodyListening => {
                    // The loop has finished without shutting the pool down —
                    // its channel is gone. Anything still queued is for a
                    // screen that no longer exists, so stop the rest rather
                    // than spend a laptop's battery on results nobody reads.
                    self.live.cancel_all();
                    break;
                }
            }
        }
        self.alive.stopped();
    }

    /// Block until there is a job, or until the queue is gone.
    ///
    /// The lock is released before the job runs — held across `recv` only —
    /// so two workers can run two jobs at once, which is the entire point of
    /// having more than one.
    fn next_assignment(&self) -> Option<Assignment> {
        let queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        queue.recv().ok()
    }
}

/// Run one job and turn whatever happens into the single event it owes the
/// loop.
fn run_job(assignment: Assignment) -> AppEvent {
    let Assignment { id, cancel, work } = assignment;

    if cancel.is_cancelled() {
        // Cancelled while it was queued. The work never runs at all.
        return AppEvent::Core(Event::JobCancelled(id));
    }

    match crash::catching(|| work(&cancel)) {
        Err(panicked) => AppEvent::Core(Event::JobFailed(
            id,
            // A panic is reported even for a cancelled job: cancellation
            // explains a job stopping, never a bug.
            JobFailure::Panicked {
                message: panicked.message,
                report: panicked.report,
            },
        )),
        // Whatever it came back with, the application asked for it to stop and
        // is no longer expecting a result. One job, one event.
        Ok(_) if cancel.is_cancelled() => AppEvent::Core(Event::JobCancelled(id)),
        Ok(Outcome::Done) => AppEvent::Core(Event::JobFinished(id)),
        Ok(Outcome::Produced(product)) => AppEvent::Produced(id, product),
        Ok(Outcome::Cancelled) => AppEvent::Core(Event::JobCancelled(id)),
        Ok(Outcome::Failed(message)) => {
            AppEvent::Core(Event::JobFailed(id, JobFailure::Failed(message)))
        }
    }
}
