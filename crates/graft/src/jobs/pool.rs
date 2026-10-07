//! The pool's own bookkeeping: what is queued, what is still running, and the
//! worker threads that turn one into the other.
//!
//! [`super::runner::JobRunner`] is the seam callers see; everything here is
//! what it is built from.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use graft_core::event::Event;
use graft_core::job::{JobFailure, JobId};

use graft_git::run::Stop;
use graft_ops::apply::StoppedAt;

use super::types::{Cancel, Delivery, JobEvents, Outcome, Work};
use crate::crash;
use crate::update::AppEvent;

/// How often a bounded wait for unstoppable writes asks its `force`
/// whether to stop waiting: a second signal is heard within this.
const FORCE_POLL: Duration = Duration::from_millis(50);

/// A queued job, on its way to whichever worker is free first.
pub(super) struct Assignment {
    pub(super) id: JobId,
    pub(super) cancel: Cancel,
    pub(super) work: Work,
}

/// The jobs that have been submitted and are not over yet, so that a
/// cancellation can find its token.
///
/// Some of them must never be cancelled: a write that cannot stop halfway
/// (`update::remote::Stoppable::No` — a merge, a rebase, a `--continue`, the
/// journal's time machine). Cancelling one SIGTERMs git part-way through and
/// leaves its state on disk with the journal entry `Pending`
/// (m8-polish-and-ship/13b). Those are marked [`Live::unstoppable`]: no
/// cancel reaches them, not one by id and not shutdown's, and shutdown waits
/// for them to end instead ([`Live::wait_for_unstoppable`]) — up to a bound.
/// Only [`Live::force_stop`] (the quit stop box's `y`) and
/// [`Live::force_all_unstoppable`] (shutdown, once its bound or a second
/// signal says so) lift the mark (13b review fix). Both remember the write
/// as forced, and its answer — where the stop left git — is kept when it
/// ends, for the line the shell gets ([`Live::wait_for_forced`], 13c
/// item 11).
#[derive(Default)]
pub(super) struct Live {
    jobs: Mutex<Jobs>,
    /// Signalled whenever a job is over, for [`Live::wait_for_unstoppable`].
    ended: Condvar,
}

#[derive(Default)]
struct Jobs {
    tokens: HashMap<JobId, Cancel>,
    /// Marked before or after the job is registered — the mark is by id, so
    /// either order holds. Removed when the job is over. The value is the
    /// write's description, for the line a forced stop prints.
    unstoppable: HashMap<JobId, String>,
    /// The writes a stop was forced on, in id order, and their answers.
    forced: BTreeMap<JobId, ForcedJob>,
    /// The jobs a worker has taken off the queue ([`Live::begin`]), until
    /// they are over. A forced write not among them never runs (`run_job`
    /// finds it cancelled), so it has no answer to wait for.
    started: HashSet<JobId>,
}

/// A forced write: its description, and its answer.
struct ForcedJob {
    what: String,
    answer: Answer,
}

/// Where a forced write's stop left git, once it has said.
#[derive(Clone, Copy)]
enum Answer {
    /// Still running: its git being stopped, or its after-state reads.
    Awaited,
    /// Ended. `None`: nothing to say — never run, or no stop reached git.
    Given(Option<StoppedAt>),
}

impl ForcedJob {
    /// `what`, just forced. One no worker has started never will — its
    /// cancel is seen before it runs — so it is answered now, with nothing.
    fn new(what: String, started: bool) -> Self {
        Self {
            what,
            answer: if started {
                Answer::Awaited
            } else {
                Answer::Given(None)
            },
        }
    }
}

impl Live {
    pub(super) fn register(&self, id: JobId, cancel: Cancel) {
        self.lock().tokens.insert(id, cancel);
    }

    /// A worker has taken `id` off the queue and is about to run it — or,
    /// if it was cancelled while queued, to answer that it never ran. Taken
    /// under the same lock as a force, so a write forced before this is
    /// seen cancelled by `run_job` and never runs.
    pub(super) fn begin(&self, id: JobId) {
        self.lock().started.insert(id);
    }

    /// Forget a job that is over. Its token is no longer anyone's business —
    /// but a forced write's answer is kept first ([`Cancel::stopped_at`]).
    pub(super) fn finish(&self, id: JobId) {
        let mut jobs = self.lock();
        jobs.started.remove(&id);
        if let Some(token) = jobs.tokens.remove(&id)
            && let Some(forced) = jobs.forced.get_mut(&id)
        {
            forced.answer = Answer::Given(token.stopped_at());
        }
        jobs.unstoppable.remove(&id);
        drop(jobs);
        self.ended.notify_all();
    }

    /// Mark `id`, described as `what`, as a job no cancel may reach (the
    /// type doc).
    pub(super) fn unstoppable(&self, id: JobId, what: String) {
        self.lock().unstoppable.insert(id, what);
    }

    /// Lift `id`'s mark and cancel it: its git is stopped half-way. The
    /// quit stop box's `y` only (`Command::ForceStop`). A write still live
    /// is remembered as forced, so the shutdown after the quit waits for
    /// its answer.
    pub(super) fn force_stop(&self, id: JobId) {
        let mut jobs = self.lock();
        let what = jobs.unstoppable.remove(&id);
        let Some(token) = jobs.tokens.get(&id) else {
            // Already over: nothing to stop, and its answer went to the
            // loop as any job's does.
            return;
        };
        token.cancel();
        if let Some(what) = what {
            let started = jobs.started.contains(&id);
            jobs.forced.insert(id, ForcedJob::new(what, started));
        }
    }

    /// Lift every live unstoppable job's mark, cancel it, and remember it
    /// as forced ([`Live::wait_for_forced`] names them). A job still queued
    /// is cancelled before it starts, so it never runs (`run_job`).
    /// Shutdown's, once its bound for writes is spent or a second signal
    /// asks for it.
    pub(super) fn force_all_unstoppable(&self) {
        let mut jobs = self.lock();
        let live: Vec<JobId> = jobs
            .unstoppable
            .keys()
            .filter(|id| jobs.tokens.contains_key(id))
            .copied()
            .collect();
        for id in live {
            if let Some(token) = jobs.tokens.get(&id) {
                token.cancel();
            }
            if let Some(what) = jobs.unstoppable.remove(&id) {
                let started = jobs.started.contains(&id);
                jobs.forced.insert(id, ForcedJob::new(what, started));
            }
        }
    }

    /// Block until every forced write has answered, or `bound` has passed;
    /// then every forced write, in id order, with its answer.
    ///
    /// A forced write's answer is read **after** its git is stopped: the
    /// after-state reads that close its entry (13c item 11). One that has
    /// not answered by `bound` is named with no answer and abandoned. **Why
    /// that is safe:** its entry is `Pending` or already closed, and the
    /// next launch's recovery closes a `Pending` one — nothing is lost by
    /// not hearing the answer, only the words that say where git was left.
    pub(super) fn wait_for_forced(&self, bound: Duration) -> Vec<Forced> {
        let deadline = Instant::now() + bound;
        let mut jobs = self.lock();
        while jobs
            .forced
            .values()
            .any(|forced| matches!(forced.answer, Answer::Awaited))
        {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            let (guard, _timeout) = self
                .ended
                .wait_timeout(jobs, remaining)
                .unwrap_or_else(PoisonError::into_inner);
            jobs = guard;
        }
        jobs.forced
            .iter()
            .map(|(id, forced)| Forced {
                job: *id,
                what: forced.what.clone(),
                at: match forced.answer {
                    Answer::Given(at) => at,
                    Answer::Awaited => None,
                },
            })
            .collect()
    }

    /// The live unstoppable writes' descriptions, in id order: what a
    /// shutdown is about to wait for.
    pub(super) fn unstoppable_live(&self) -> Vec<String> {
        let jobs = self.lock();
        let mut live: Vec<(&JobId, &String)> = jobs
            .unstoppable
            .iter()
            .filter(|(id, _)| jobs.tokens.contains_key(id))
            .collect();
        live.sort_unstable_by_key(|(id, _)| **id);
        live.into_iter().map(|(_, what)| what.clone()).collect()
    }

    /// Cancel `id`, unless it is [`Live::unstoppable`]. `update` never asks
    /// that (`stop_write` refuses in words); this is the floor under it.
    pub(super) fn cancel(&self, id: JobId) {
        let jobs = self.lock();
        if jobs.unstoppable.contains_key(&id) {
            return;
        }
        if let Some(token) = jobs.tokens.get(&id) {
            token.cancel();
        }
    }

    /// Cancel every job but the [`Live::unstoppable`] ones.
    pub(super) fn cancel_all(&self) {
        let jobs = self.lock();
        for (id, token) in &jobs.tokens {
            if !jobs.unstoppable.contains_key(id) {
                token.cancel();
            }
        }
    }

    /// Block until no unstoppable job is live, until `bound` has passed, or
    /// until `force` says stop — whichever comes first. `true` when none is
    /// live. A job is live from submit to its event, so one still queued is
    /// waited for too: it runs, since nothing cancelled it.
    ///
    /// Bounded since the 13b review fix: a write whose git hook hangs must
    /// not hold the process (and, before that fix, the terminal) for ever.
    /// `force` is polled every [`FORCE_POLL`]; nothing else is.
    pub(super) fn wait_for_unstoppable(&self, bound: Duration, force: &dyn Stop) -> bool {
        let deadline = Instant::now() + bound;
        let mut jobs = self.lock();
        loop {
            if !jobs
                .tokens
                .keys()
                .any(|id| jobs.unstoppable.contains_key(id))
            {
                return true;
            }
            if force.should_stop() {
                return false;
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            let (guard, _timeout) = self
                .ended
                .wait_timeout(jobs, remaining.min(FORCE_POLL))
                .unwrap_or_else(PoisonError::into_inner);
            jobs = guard;
        }
    }

    /// A panicking job never holds this lock — the worker takes it only to
    /// touch the map — so poisoning can only mean a panic somewhere that left
    /// the map itself intact. Refusing to cancel jobs because of that would
    /// turn one bug into a hang.
    fn lock(&self) -> MutexGuard<'_, Jobs> {
        self.jobs.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// How many workers are still running, so shutdown can wait for them without
/// blocking forever inside a `join`.
#[derive(Default)]
pub(super) struct Alive {
    count: Mutex<usize>,
    all_stopped: Condvar,
}

impl Alive {
    pub(super) fn started(&self) {
        *self.count.lock().unwrap_or_else(PoisonError::into_inner) += 1;
    }

    pub(super) fn stopped(&self) {
        let mut count = self.count.lock().unwrap_or_else(PoisonError::into_inner);
        *count = count.saturating_sub(1);
        if *count == 0 {
            self.all_stopped.notify_all();
        }
    }

    /// Wait until every worker has stopped, or `grace` runs out. Returns how
    /// many are still going.
    pub(super) fn wait_for_all(&self, grace: Duration) -> usize {
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shutdown {
    /// How many workers were still inside a job when the grace ran out.
    ///
    /// They were cancelled and did not notice in time — a job that does not
    /// check its token often enough, which is a bug worth hearing about.
    /// Their threads are left to die with the process rather than held on
    /// to, so that the process can end now.
    pub stalled: usize,
    /// The writes a stop was forced on, in id order: the quit stop box's
    /// `y`, or shutdown's own once its bound for writes was spent or a
    /// second signal asked (13b review fix) — each with where its stop left
    /// git (13c item 11). Empty on every ordinary quit.
    pub forced: Vec<Forced>,
}

/// A write a stop was forced on, and where the stop left git
/// (m8-polish-and-ship/13c item 11): what the shell's exit line says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forced {
    /// The write's job.
    pub job: JobId,
    /// The write's description.
    pub what: String,
    /// Where its stop left git, as `graft_ops` read it from disk. `None`:
    /// no answer within [`super::FORCED_WAIT`] (the job was abandoned), or
    /// nothing to say (never run, or the stop did not reach git).
    pub at: Option<StoppedAt>,
}

impl Shutdown {
    /// Every worker stopped, every job accounted for, nothing forced.
    pub const CLEAN: Self = Self {
        stalled: 0,
        forced: Vec::new(),
    };
}

/// One worker thread: take the next job, run it, say what happened, repeat.
pub(super) struct Worker<S: JobEvents> {
    pub(super) queue: Arc<Mutex<Receiver<Assignment>>>,
    pub(super) live: Arc<Live>,
    pub(super) alive: Arc<Alive>,
    pub(super) sink: S,
}

impl<S: JobEvents> Worker<S> {
    pub(super) fn run(self) {
        loop {
            let Some(assignment) = self.next_assignment() else {
                // The queue is gone: the runner has shut down. Nothing is
                // wrong, and there is nothing left to run.
                break;
            };

            let id = assignment.id;
            self.live.begin(id);
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
