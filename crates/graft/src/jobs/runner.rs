//! [`JobRunner`]: the pool as callers see it — start it, submit or dispatch
//! work, cancel a job, shut it down.

use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use graft_core::event::Event;
use graft_core::job::{JobFailure, JobId};

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;

use super::MAX_WORKERS;
use super::kinds::{
    check_system_git, decline_commit_graph, open_repository, read_history, search,
    write_commit_graph,
};
use super::pool::{Alive, Assignment, Live, Shutdown, Worker};
use super::types::{Cancel, Dispatch, JobEvents, Outcome, Work};
use crate::history;
use crate::history::RowRequest;
use crate::jobs::radar::RadarContext;
use crate::repos::RepoStore;
use crate::update::{AppEvent, Command, Product, Progress, Task};
use graft_ops::mistake_radar::probes::{Duplicates, EvilVerdict, ForcePushVerdict};

/// The Mistake Radar's context job: reads the default branch's tip and
/// table from the store `run_radar_context` needs and joins any open error
/// to the same error string every other detail job reports.
fn radar_context(
    repos: &RepoStore,
    default_tip: Oid,
    default_ref: &RefName,
    stop: &Cancel,
) -> Result<Arc<RadarContext>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let table = repos.table().map_err(|error| error.to_string())?;
    super::radar::run_radar_context(&handle, &table, default_tip, default_ref, stop)
}

/// The evil-merge probe: opens the repository the probe needs and reports an
/// open failure as the verdict's own `NotCheckable`, because "could not
/// check" is an answer the pane shows, not an error to report elsewhere.
fn evil_probe(repos: &RepoStore, merge: Oid, stop: &Cancel) -> EvilVerdict {
    let repo = match repos.shared() {
        Ok(repo) => repo,
        Err(error) => return EvilVerdict::NotCheckable(error.to_string()),
    };
    super::radar::run_evil_probe(&repo.handle(), merge, stop)
}

/// The force-push probe, with the same open-failure rule as [`evil_probe`].
fn force_push_probe(
    repos: &RepoStore,
    branch: &str,
    current: Oid,
    stop: &Cancel,
) -> ForcePushVerdict {
    let repo = match repos.shared() {
        Ok(repo) => repo,
        Err(error) => return ForcePushVerdict::NotCheckable(error.to_string()),
    };
    super::radar::run_force_push_probe(&repo.handle(), branch, current, stop)
}

/// The duplicates probe, with the same open-failure rule as [`evil_probe`],
/// reported as an error since [`Duplicates`] has no `NotCheckable` slot of
/// its own (the pane renders it as "duplicates: not checked — <line>").
fn duplicates_probe(
    repos: &RepoStore,
    window: u32,
    stop: &Cancel,
) -> Result<Arc<Duplicates>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    super::radar::run_duplicates_probe(&repo.handle(), window, stop)
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
    pub(super) ahead_behind: Arc<crate::refs::AheadBehindCache>,
    /// The credential questions waiting for the user (m4-remotes/06), by
    /// job. `answer_credential` is the only way in.
    waiting: super::askpass::Waiting,
    /// The binary git runs as its askpass helper. `None` means Graft's own
    /// (`current_exe`), which is right for the shipped binary and wrong
    /// under `cargo test`, where `current_exe` is the test binary: the
    /// integration tests point this at `CARGO_BIN_EXE_graft`.
    askpass_helper: Option<std::path::PathBuf>,
    /// The clock every journal entry this runner writes is stamped from
    /// (`crate::session::Clock`). `Wall` for the shipped binary; a capture
    /// that writes to the repository it is photographing sets `Fixed`, so
    /// the entry it draws afterwards reads the same on every run.
    clock: crate::session::Clock,
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
                        // the queue is dropped, so this cannot hang. `Err`
                        // means the worker panicked; `crash::install`'s hook
                        // already reported that to the user, so there is
                        // nothing left to do with it here.
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
            waiting: super::askpass::Waiting::default(),
            askpass_helper: None,
            clock: crate::session::Clock::Wall,
        })
    }

    /// Run `helper` as git's askpass helper instead of Graft's own binary.
    ///
    /// For the integration tests: under `cargo test` `current_exe` is the
    /// test binary, which has no askpass mode, so the tests point this at
    /// `CARGO_BIN_EXE_graft`. The shipped binary never calls it.
    #[must_use]
    pub fn with_askpass_helper(mut self, helper: std::path::PathBuf) -> Self {
        self.askpass_helper = Some(helper);
        self
    }

    /// Stamp the journal entries this runner writes from `clock` rather
    /// than the wall clock.
    ///
    /// For a capture whose repository the app writes **during** the
    /// capture: the entry it then draws would otherwise carry a different
    /// instant on every run. See [`crate::session::Clock`] for the rule.
    #[must_use]
    pub fn with_clock(mut self, clock: crate::session::Clock) -> Self {
        self.clock = clock;
        self
    }

    /// Hands the user's answer to the credential question job `id` is
    /// waiting on; `None` is "the user cancelled".
    ///
    /// `false` when no question under `id` is waiting — the job ended while
    /// the key was in flight. The answer is dropped there and then (a
    /// `Secret` zeroes itself on drop), which is the whole handling: there
    /// is nobody left to tell, and nothing to log, least of all this.
    pub fn answer_credential(
        &self,
        id: JobId,
        answer: Option<graft_git::run::askpass::Secret>,
    ) -> bool {
        self.waiting.answer(id, answer)
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
        self.queue_as(id, task, build)
    }

    /// The detail pane's reads (`dispatch` routes only those here).
    fn dispatch_detail(&self, command: Command) {
        match command {
            Command::ListFiles(id, spec, options) => {
                let repos = Arc::clone(&self.repos);
                self.detail_job(id, Task::ListFiles { spec }, move |cancel| {
                    Product::Files(super::detail::read_files(&repos, spec, options, cancel))
                });
            }
            Command::ReadFileDiff(id, key, entry) => {
                let repos = Arc::clone(&self.repos);
                self.detail_job(id, Task::FileDiff { key }, move |cancel| {
                    let result = super::detail::read_file_diff(&repos, &key, &entry, cancel);
                    Product::FileDiff(key, result)
                });
            }
            Command::ReadBlame(id, request) => self.blame_job(id, *request),
            command @ (Command::TraceHistory { .. } | Command::ReadWholeFile { .. }) => {
                self.dispatch_history(command);
            }
            Command::ContainedIn(id, oid) => {
                let repos = Arc::clone(&self.repos);
                self.detail_job(id, Task::ContainedIn { oid }, move |cancel| {
                    Product::ContainedIn(oid, super::detail::read_contained(&repos, oid, cancel))
                });
            }
            Command::EvilProbe(id, merge) => {
                let repos = Arc::clone(&self.repos);
                self.detail_job(id, Task::EvilProbe { merge }, move |cancel| {
                    Product::EvilProbe(merge, evil_probe(&repos, merge, cancel))
                });
            }
            Command::ForcePushProbe(id, branch, current) => {
                let repos = Arc::clone(&self.repos);
                let task = Task::ForcePushProbe {
                    branch: branch.clone(),
                    current,
                };
                self.detail_job(id, task, move |cancel| {
                    let verdict = force_push_probe(&repos, &branch, current, cancel);
                    Product::ForcePushProbe(branch, current, verdict)
                });
            }
            Command::RadarContext(id, default_tip, default_ref) => {
                let repos = Arc::clone(&self.repos);
                let task = Task::RadarContext {
                    default_tip,
                    default_ref: default_ref.clone(),
                };
                self.detail_job(id, task, move |cancel| {
                    Product::RadarContext(radar_context(&repos, default_tip, &default_ref, cancel))
                });
            }
            Command::DuplicatesProbe(id, window) => {
                let repos = Arc::clone(&self.repos);
                self.detail_job(id, Task::DuplicatesProbe { window }, move |cancel| {
                    Product::DuplicatesProbe(duplicates_probe(&repos, window, cancel))
                });
            }
            // `dispatch` routes only the ones above here; anything else is a
            // routing bug, not a runtime condition.
            other => unreachable!("not a detail command: {other:?}"),
        }
    }

    /// The history place's reads: the trace, and one commit's whole file.
    fn dispatch_history(&self, command: Command) {
        let repos = Arc::clone(&self.repos);
        match command {
            Command::TraceHistory { id, request } => {
                self.detail_job(id, Task::Trace, move |cancel| {
                    Product::Trace(super::detail::read_trace(&repos, &request, cancel))
                });
            }
            Command::ReadWholeFile {
                id,
                commit,
                path,
                old_path,
                max_lines,
            } => {
                self.detail_job(id, Task::WholeFile, move |cancel| {
                    let read = super::detail::read_whole_file(
                        &repos,
                        commit,
                        &path,
                        old_path.as_deref(),
                        max_lines,
                        cancel,
                    );
                    Product::WholeFile(read)
                });
            }
            // `dispatch_detail` routes only the two above here.
            other => unreachable!("not a history command: {other:?}"),
        }
    }

    /// Which commits only a stash reaches (m7-power-tools/16), under the
    /// id `update` chose, so an answer to a superseded read is dropped there.
    fn read_stash_only(&self, id: JobId, stashes: Vec<Oid>) {
        let repos = Arc::clone(&self.repos);
        self.detail_job(id, Task::StashOnly, move |cancel| {
            Product::StashOnly(super::stash_only::read_stash_only(&repos, &stashes, cancel))
        });
    }

    /// A blame place's read (m7-power-tools/20), cancellable like the
    /// pane's.
    fn blame_job(&self, id: JobId, request: crate::update::place::BlameRequest) {
        let repos = Arc::clone(&self.repos);
        self.detail_job(id, Task::Blame, move |cancel| {
            Product::Blame(super::detail::read_blame(&repos, &request, cancel))
        });
    }

    /// Queue one of the detail pane's reads under `update`'s id. A job
    /// stopped part way reports `Cancelled`, never its half-read product.
    fn detail_job(
        &self,
        id: JobId,
        task: Task,
        read: impl FnOnce(&Cancel) -> Product + Send + 'static,
    ) {
        self.queue_as(id, Some(task), |_| {
            Box::new(move |cancel: &Cancel| {
                let product = read(cancel);
                if cancel.is_cancelled() {
                    Outcome::Cancelled
                } else {
                    Outcome::Produced(product)
                }
            })
        });
    }

    /// Queue a job under an id `update` already chose (the detail pane's,
    /// `update::detail::LOCAL_JOB_BIT` set), so it can cancel or drop the
    /// job's answer before the job has even started.
    fn queue_as(&self, id: JobId, task: Option<Task>, build: impl FnOnce(JobId) -> Work) -> JobId {
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
    ///
    /// The history walk is the one job that can be *idle* rather than
    /// running: between row requests it parks in `Receiver::recv`, where it
    /// cannot see its token at all. Dropping its request sender is what wakes
    /// it — `serve` reads a closed channel as the end of the walk. The token
    /// is set first, so a walk woken mid-chunk finds `stop()` already true.
    ///
    /// Both halves live here rather than in `Command::CancelJob`'s arm, which
    /// is where the sender-drop used to be: `cancel` is public, and a caller
    /// that reached it any other way left the walk parked on a worker
    /// (m5-history-and-ghost-preview/05).
    pub fn cancel(&self, id: JobId) {
        self.live.cancel(id);
        drop(self.row_requests().remove(&id));
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

impl<S: JobEvents> JobRunner<S> {
    /// Opens (or, on `Ctrl+R`, re-opens) the repository. The refresh runs
    /// here, on the dispatching thread, not in the job: the jobs that read
    /// refs are dispatched right after and may start first on another
    /// worker, and must not be served refs read before it (m1-the-graph/55).
    ///
    /// A refresh failure (the repository disappeared, or its commit-graph
    /// could not be read while checking whether it changed) is not swallowed:
    /// it is reported through the same `Product::Repository` the job below
    /// would otherwise produce, so it reaches the status line exactly as an
    /// open failure does.
    fn open_repository(&self) {
        if let Err(error) = self.repos.refresh() {
            let message = error.to_string();
            self.announce(Task::OpenRepository, move |_| {
                Outcome::Produced(Product::Repository(Err(message)))
            });
            return;
        }
        self.announce(
            Task::OpenRepository,
            open_repository(Arc::clone(&self.repos)),
        );
    }

    /// Sorts the whole range topologically off the loop's thread, for a walk
    /// whose order is provisional (ADR-0008).
    fn promote_history(&self) {
        let repos = Arc::clone(&self.repos);
        self.announce(Task::PromoteHistory, move |cancel| {
            let stop = || cancel.is_cancelled();
            if stop() {
                return Outcome::Cancelled;
            }
            match history::topological_order(&repos, &stop) {
                Ok(Some(order)) => Outcome::Produced(Product::TopologicalOrder(order)),
                Ok(None) => Outcome::Cancelled,
                Err(message) => Outcome::Failed(message),
            }
        });
    }

    /// Builds the commit-graph with system git off the loop's thread
    /// (m1-the-graph/54).
    fn write_commit_graph(&self) {
        let repos = Arc::clone(&self.repos);
        self.announce(Task::WriteCommitGraph, move |cancel| {
            // Checked once, before git starts: a write not yet begun can be
            // dropped (nothing is recorded, the next open offers again). Once
            // git runs it is never cancelled nor killed — see
            // `graft_git::commit_graph`.
            if cancel.is_cancelled() {
                return Outcome::Cancelled;
            }
            Outcome::Produced(Product::CommitGraph(write_commit_graph(&repos)))
        });
    }

    /// Remembers the declined commit-graph offer off the loop's thread.
    fn decline_commit_graph(&self) {
        let repos = Arc::clone(&self.repos);
        self.announce(Task::DeclineCommitGraph, move |cancel| {
            if cancel.is_cancelled() {
                return Outcome::Cancelled;
            }
            Outcome::Produced(Product::CommitGraphDeclined(decline_commit_graph(&repos)))
        });
    }
    /// Reads the refs off the loop's thread, under the id `update` chose
    /// for it (m6-conflicts/12: a landing waits on one particular read).
    fn read_refs(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        let cache = Arc::clone(&self.ahead_behind);
        self.queue_as(id, Some(Task::ReadRefs), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Refs(crate::refs::read(&repos, &cache)))
            })
        });
    }

    /// Reads the working tree's full status off the loop's thread, under an
    /// id `update` chose so the read a write asked for can be told from an
    /// older one (`update::status_read`).
    fn read_working_tree(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadWorkingTree), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::WorkingTree(
                    crate::working_tree::read(&repos, cancel).map(Box::new),
                ))
            })
        });
    }

    /// Reads one conflicted path for the resolver, under an id `update`
    /// chose so a stale answer can be dropped (m6-conflicts/04).
    fn open_conflict(&self, id: JobId, path: std::path::PathBuf) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::OpenConflict), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let opened = super::conflict::open(&repos, &path, cancel);
                Outcome::Produced(Product::ConflictOpened(opened))
            })
        });
    }

    /// Takes one side of one conflict block and writes the file back, under
    /// an id `update` chose (m6-conflicts/04).
    ///
    /// Not cancellable part-way on purpose: the write is one temp-file
    /// rename deep inside `ops`, and a token checked between "decide" and
    /// "rename" would only ever report a take that had in fact already
    /// landed.
    fn take_side(&self, id: JobId, request: crate::update::TakeRequest) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::TakeSide), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let crate::update::TakeRequest {
                    path,
                    file,
                    at,
                    take,
                } = request;
                let taken = super::conflict::take(&repos, &path, &file, at, take).map(Arc::new);
                Outcome::Produced(Product::Taken(taken))
            })
        });
    }

    /// Attaches the merge base to every block of the open file, under an id
    /// `update` chose so a stale answer can be dropped (`B`, m6-conflicts/05).
    fn load_base(
        &self,
        id: JobId,
        path: std::path::PathBuf,
        file: graft_git::conflict::ConflictFile,
    ) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::LoadBase), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let loaded = super::conflict::load_base(&repos, &path, file)
                    .map(|(file, n)| (Arc::new(file), n));
                Outcome::Produced(Product::BaseLoaded(loaded))
            })
        });
    }

    /// Reads the set of files the operation in progress conflicted on, for
    /// the resolver's file list, under an id `update` chose
    /// (m6-conflicts/13).
    fn read_conflict_set(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadConflictSet), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let set = super::conflict::conflict_set(&repos, cancel).map(Arc::new);
                Outcome::Produced(Product::ConflictSet(set))
            })
        });
    }

    /// Reads what the merge about to be committed changes, for the finish
    /// summary, under an id `update` chose (m6-conflicts/19).
    fn read_finish_summary(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadFinishSummary), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::FinishSummary(super::finish::summary(&repos)))
            })
        });
    }

    /// Reads one staged file's diff for the finish summary's `space`, under
    /// an id `update` chose (m6-conflicts/19).
    fn read_finish_diff(&self, id: JobId, path: std::path::PathBuf) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadFinishDiff), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let diff = super::finish::staged_diff(&repos, &path, cancel);
                Outcome::Produced(Product::FinishDiff(diff))
            })
        });
    }

    /// Reads `commit.template`'s file for the commit editor's empty body,
    /// under an id `update` chose (m3-working-tree-undo-and-stash/05).
    fn read_commit_template(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadCommitTemplate), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::CommitTemplate(super::commit_message::template(
                    &repos,
                )))
            })
        });
    }

    /// Reads `HEAD`'s commit message for an amend's prefill, under an id
    /// `update` chose (m3-working-tree-undo-and-stash/05).
    fn read_head_message(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadHeadMessage), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::HeadMessage(super::commit_message::head_message(
                    &repos,
                )))
            })
        });
    }

    /// Runs one working-tree write, under an id `update` already chose
    /// (m3-working-tree-undo-and-stash/04): the runner reports
    /// [`Progress::Operating`] as soon as the job starts, then `ops::apply`
    /// does the rest — the seam rule (`tests/it/seam_rule.rs`).
    /// `c`/`s`/`a` on the operation banner: carry the live operation
    /// forward, skip its step, or abort it
    /// (m5-history-and-ghost-preview/09).
    ///
    /// Simpler than [`Self::operate`]: no progress stream and no askpass,
    /// because `git <kind> --continue|--skip|--abort` talks to no remote
    /// and asks no credential.
    fn resume(&self, id: JobId, what: graft_ops::apply::Resume) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        let clock = self.clock;
        let description = match what {
            graft_ops::apply::Resume::Continue | graft_ops::apply::Resume::ContinueWith { .. } => {
                "continue"
            }
            graft_ops::apply::Resume::Skip => "skip",
            graft_ops::apply::Resume::Abort => "abort",
        };
        self.queue_as(id, Some(Task::Resume), move |id| {
            Box::new(move |cancel: &Cancel| {
                sink.deliver(AppEvent::Progress(
                    id,
                    Progress::Operating {
                        description: description.to_owned(),
                    },
                ));
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Resumed(Box::new(super::operate::run_resume(
                    &repos, what, clock, cancel,
                ))))
            })
        });
    }

    fn operate(&self, id: JobId, op: Box<graft_ops::operation::Operation>) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        let clock = self.clock;
        let description = op.describe();
        let waiting = self.waiting.clone();
        // Settled here, on the loop's thread, not inside the job: what git
        // must run to ask is a fact about this process.
        let helper = match self.askpass_helper.clone() {
            Some(helper) => Ok(helper),
            None => std::env::current_exe().map_err(|error| error.to_string()),
        };
        self.queue_as(id, Some(Task::Operate), move |id| {
            Box::new(move |cancel: &Cancel| {
                sink.deliver(AppEvent::Progress(id, Progress::Operating { description }));
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                // A fetch, pull or push streams git's progress lines to the
                // status line (m4-remotes/05). The sink is `Send` but not
                // `Sync`, and the hook is called from the executor's stderr
                // thread, hence the lock.
                let sink = Arc::new(std::sync::Mutex::new(sink));
                let say = move |sink: &std::sync::Mutex<S>, progress: Progress| {
                    // A poisoned lock means another progress call panicked;
                    // that panic is the job's failure and is reported as
                    // one. Dropping this one line of progress is safe.
                    if let Ok(sink) = sink.lock() {
                        // `Delivery::NobodyListening`: nobody is left to
                        // show it.
                        let _delivery = sink.deliver(AppEvent::Progress(id, progress));
                    }
                };
                let brief_sink = Arc::clone(&sink);
                let progress = move |brief: String| {
                    say(&brief_sink, Progress::RemoteProgress { brief });
                };
                // The credential question travels the same way the progress
                // line does; the answer comes back through `waiting`.
                let askpass = Arc::new(super::askpass::ChannelAskPass::new(
                    id,
                    Box::new(move |prompt: String| {
                        say(
                            &sink,
                            Progress::CredentialNeeded {
                                prompt,
                                pulse_seed: super::askpass::pulse_seed(),
                            },
                        );
                    }),
                    waiting,
                    cancel.clone(),
                ));
                Outcome::Produced(Product::Operated(Box::new(super::operate::run_operation(
                    &repos,
                    &op,
                    clock,
                    cancel,
                    super::operate::Hooks {
                        progress: Box::new(progress),
                        askpass,
                        helper,
                    },
                ))))
            })
        });
    }

    /// The user's answer to git's credential question, or their cancelling
    /// of it (m4-remotes/06).
    ///
    /// A `false` from `answer_credential` is the job having ended while the
    /// key was in flight: the answer is dropped there and then, zeroed, and
    /// there is nobody left to tell.
    fn credential(&self, id: JobId, answer: Option<crate::update::credential::Answer>) {
        let _delivered: bool = self.answer_credential(
            id,
            answer.map(crate::update::credential::Answer::into_secret),
        );
    }

    /// Starts the history walk and keeps the channel its row requests go down.
    ///
    /// Under the id `update` already chose, like the detail pane's reads: a
    /// `Ctrl+R` pressed before this walk's `Started` event lands still has a
    /// name to cancel (m5-history-and-ghost-preview/05).
    fn read_history(&self, id: JobId, order: graft_git::history::Order) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        let (asks, requests) = std::sync::mpsc::channel();
        // Registered before `update` can see the job has started: the
        // announcement travels through the loop's queue, and this runs
        // on the loop's own thread before it reads that queue again.
        let id = self.queue_as(id, Some(Task::ReadHistory), |id| {
            read_history(repos, order, requests, sink, id)
        });
        self.row_requests().insert(id, asks);
    }

    /// Runs one undo or redo, under an id `update` already chose
    /// (m3-working-tree-undo-and-stash/06). A write like any other: it takes
    /// `AppState::operating`, so the same `Progress::Operating` line names it
    /// while it runs.
    fn run_undo(&self, id: JobId, redo: bool, force: bool) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        let clock = self.clock;
        let description = if redo { "redo" } else { "undo" };
        self.queue_as(id, Some(Task::Undo), move |id| {
            Box::new(move |cancel: &Cancel| {
                sink.deliver(AppEvent::Progress(
                    id,
                    Progress::Operating {
                        description: description.to_owned(),
                    },
                ));
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Undone(Box::new(super::undo::run_undo(
                    &repos, redo, force, clock, cancel,
                ))))
            })
        });
    }

    /// Draws one Ghost Preview, under an id `update` already chose.
    ///
    /// A read like the detail pane's, not a write: it takes no
    /// `AppState::operating`, announces no progress, and `Esc` cancels it
    /// the way it cancels any other read.
    fn plan_preview(&self, id: JobId, request: Box<crate::update::preview::PreviewRequest>) {
        let repos = Arc::clone(&self.repos);
        let clock = self.clock;
        self.queue_as(id, Some(Task::PlanPreview), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Preview(super::preview::run_preview(
                    &repos, &request, clock, cancel,
                )))
            })
        });
    }

    /// Runs the journal's time machine, under an id `update` already chose.
    ///
    /// A write, like an undo: it takes `AppState::operating`, so the same
    /// `Progress::Operating` line names it while it runs.
    fn run_revert_to(&self, id: JobId, target: graft_core::journal::EntryId, force: bool) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        let clock = self.clock;
        self.queue_as(id, Some(Task::RevertTo), move |id| {
            Box::new(move |cancel: &Cancel| {
                sink.deliver(AppEvent::Progress(
                    id,
                    Progress::Operating {
                        description: "revert".to_owned(),
                    },
                ));
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::RevertedTo(Box::new(super::undo::run_revert_to(
                    &repos, &target, force, clock, cancel,
                ))))
            })
        });
    }

    /// Reads the undo journal, under an id `update` already chose. Closes
    /// whatever a previous run left open before it answers.
    fn read_journal(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        let clock = self.clock;
        self.queue_as(id, Some(Task::ReadJournal), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Journal(super::undo::read_journal(
                    &repos, clock, cancel,
                )))
            })
        });
    }

    /// Reads the stash list for the `S` overlay, under an id `update`
    /// already chose.
    fn read_stashes(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadStashes), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Stashes(super::stash::read_stashes(&repos, cancel)))
            })
        });
    }

    /// Reads the index now for a stash clash's tie, under an id `update`
    /// already chose (ADR-0045 §2, amended 2026-09-25).
    fn read_index_state(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadIndexState), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::IndexState(super::stash::read_index_state(
                    &repos, cancel,
                )))
            })
        });
    }

    /// Starts a search. The id names the search on screen; `update` tells a
    /// superseded one apart by the order searches start in.
    fn start_search(&self, query: crate::search::Query) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        self.queue_job(Some(Task::Search), |id| search(repos, query, sink, id));
    }

    /// Reads the remote facts `f`/`p`/`P` decide from, under an id `update`
    /// already chose (m4-remotes/05).
    fn read_remotes(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadRemotes), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Remotes(super::remote::read_remotes(
                    &repos, cancel,
                )))
            })
        });
    }

    /// Reads what a force-with-lease push would take off the remote, under
    /// an id `update` already chose (m7-power-tools/16b). The answer
    /// carries its lease key; `update` drops one a newer key superseded.
    fn read_leaves(&self, id: JobId, sent: Oid, expected: Oid) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, None, move |_| {
            Box::new(move |cancel: &Cancel| {
                let result = super::remote::read_leaves(&repos, sent, expected, cancel);
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Leaves {
                    sent,
                    expected,
                    result,
                })
            })
        });
    }

    /// Builds the Map view's per-branch bucket counts, under an id `update`
    /// already chose (`update::map::MapState::open`, mirroring the detail
    /// pane's own jobs — see `queue_as`'s doc).
    fn build_map(&self, id: JobId, req: crate::map::MapRequest) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        self.queue_as(id, Some(Task::Map), |id| {
            Box::new(move |cancel: &Cancel| {
                let mut report = |done, total| {
                    sink.deliver(AppEvent::Progress(id, Progress::Map { done, total }));
                };
                let product = super::map::build_map(&repos, &req, &mut report, cancel);
                Outcome::Produced(Product::Map(product))
            })
        });
    }

    /// Hand a running history job the window of rows the view now wants.
    fn show_rows(&self, id: JobId, request: RowRequest) {
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
}

/// The detail pane's reads, which `dispatch` hands to `dispatch_detail`.
fn is_detail(command: &Command) -> bool {
    matches!(
        command,
        Command::ListFiles(..)
            | Command::ReadFileDiff(..)
            | Command::ReadBlame(..)
            | Command::TraceHistory { .. }
            | Command::ReadWholeFile { .. }
            | Command::ContainedIn(..)
            | Command::RadarContext(..)
            | Command::EvilProbe(..)
            | Command::ForcePushProbe(..)
            | Command::DuplicatesProbe(..)
    )
}

impl<S: JobEvents> Dispatch for JobRunner<S> {
    fn dispatch(&self, command: Command) {
        match command {
            Command::OpenRepository => self.open_repository(),
            Command::CheckSystemGit => {
                self.announce(Task::CheckSystemGit, check_system_git());
            }
            Command::ReadRefs(id) => self.read_refs(id),
            Command::ReadWorkingTree(id) => self.read_working_tree(id),
            Command::PromoteHistory => self.promote_history(),
            Command::WriteCommitGraph => self.write_commit_graph(),
            Command::DeclineCommitGraph => self.decline_commit_graph(),
            Command::ReadHistory(id, order) => self.read_history(id, order),
            Command::ShowRows(id, request) => self.show_rows(id, request),
            Command::Search(query) => self.start_search(query),
            command if is_detail(&command) => self.dispatch_detail(command),
            Command::ReadStashOnly(id, stashes) => self.read_stash_only(id, stashes),
            Command::BuildMap(id, req) => self.build_map(id, req),
            Command::ReadCommitTemplate(id) => self.read_commit_template(id),
            Command::ReadHeadMessage(id) => self.read_head_message(id),
            Command::Operate(id, op) => self.operate(id, op),
            Command::Resume(id, what) => self.resume(id, what),
            Command::Undo(id, force) => self.run_undo(id, false, force),
            Command::Redo(id, force) => self.run_undo(id, true, force),
            Command::ReadJournal(id) => self.read_journal(id),
            Command::PlanPreview(id, request) => self.plan_preview(id, request),
            Command::RevertTo(id, target, force) => self.run_revert_to(id, target, force),
            Command::ReadStashes(id) => self.read_stashes(id),
            Command::ReadIndexState { id } => self.read_index_state(id),
            Command::OpenConflict(id, path) => self.open_conflict(id, path),
            Command::TakeSide(id, request) => self.take_side(id, request),
            Command::LoadBase(id, path, file) => self.load_base(id, path, *file),
            Command::ReadConflictSet(id) => self.read_conflict_set(id),
            Command::ReadFinishSummary(id) => self.read_finish_summary(id),
            Command::ReadFinishDiff(id, path) => self.read_finish_diff(id, path),
            Command::ReadRemotes(id) => self.read_remotes(id),
            Command::ReadLeaves { id, sent, expected } => self.read_leaves(id, sent, expected),
            Command::CancelJob(id) => self.cancel(id),
            Command::AnswerCredential(id, answer) => self.credential(id, Some(answer)),
            Command::CancelCredential(id) => self.credential(id, None),
            other => not_a_job(&other),
        }
    }
}

/// The commands the loop carries out itself; one reaching the runner is a
/// routing bug, not a runtime condition. The detail pane's reads go by
/// `is_detail`'s guard, so they cannot land here.
fn not_a_job(command: &Command) -> ! {
    match command {
        // The loop writes this to the terminal itself (`event_loop`).
        Command::CopyToClipboard(_) => {
            unreachable!("CopyToClipboard is terminal output, never a job")
        }
        // The loop writes this to disk itself (`event_loop::running`).
        Command::SaveLayout(_) => {
            unreachable!("SaveLayout is written by the loop, never a job")
        }
        // Handing the terminal over and taking it back is the loop's own
        // business (`event_loop::outside`): a worker thread has no claim on
        // the modes the loop drew through (m3-working-tree-undo-and-stash/05).
        Command::OpenExternalEditor { .. }
        | Command::OpenMergetool { .. }
        | Command::OpenConflictEditor { .. }
        | Command::EditCommitMessage { .. }
        | Command::Suspend => {
            unreachable!("the terminal is handed over by the loop, never by a job")
        }
        other => unreachable!("not routed: {other:?}"),
    }
}
