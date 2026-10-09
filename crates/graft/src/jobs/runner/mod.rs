//! [`JobRunner`]: the pool as callers see it — start it, submit or dispatch
//! work, cancel a job, shut it down.

mod dispatch;
mod lists;
mod operations;
mod queue;
mod reads;

// The `jobs` modules and constants the children name as `super::<name>`.
use super::{
    clone, commit_message, conflict, detail, finish, fold_count, journal_list, map, operate,
    palette, preview, radar, remote, rewrite, rewrite_open, stash, stash_only, switch_list, undo,
};

use std::collections::HashMap;
use std::io;
use std::sync::atomic::AtomicU64;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use graft_core::job::JobId;

use super::MAX_WORKERS;
use super::pool::{Alive, Assignment, Live, Shutdown, Worker};
use super::types::JobEvents;
use crate::history::RowRequest;
use crate::repos::RepoStore;

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
    /// In-process tests and examples only: the shipped binary never calls
    /// it, so no env var reaches the journal's stamps (m8/19e §R1).
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
}

impl<S: JobEvents> JobRunner<S> {
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

    /// Mark `id`, described as `what`, as a write that cannot stop
    /// halfway (`Command::Unstoppable`): no cancel reaches it, and
    /// [`shutdown`] waits for it to end — up to its bound for writes —
    /// rather than signalling its git child.
    ///
    /// [`shutdown`]: Self::shutdown
    pub fn unstoppable(&self, id: JobId, what: String) {
        self.live.unstoppable(id, what);
    }

    /// The only path that signals a `Stoppable::No` job from inside the
    /// app: the quit stop box's `y` (`Command::ForceStop`). Lifts `id`'s
    /// mark and cancels it (its git stopped as `graft_git::run` stops a write:
    /// hook-first for a local write, the group stop for a rebase or a remote
    /// op; reaped within `STOP_WITHIN`), all within [`super::SHUTDOWN_GRACE`]. The shutdown that follows the quit waits
    /// for it as for any stopped job. (Shutdown's own forced stop, on its
    /// bound or a second signal, is the other.)
    pub fn force_stop(&self, id: JobId) {
        self.live.force_stop(id);
    }

    /// The unstoppable writes still live, by description: what a shutdown
    /// starting now would wait for (the signal path's waiting line).
    #[must_use]
    pub fn writes_running(&self) -> Vec<String> {
        self.live.unstoppable_live()
    }

    /// Stop the pool: cancel every job but the unstoppable writes, wait for
    /// those to end — up to `writes`, or until `force` says stop — then wait
    /// up to `grace` for the rest to notice.
    ///
    /// Consumes the runner, because a pool that has been shut down must not be
    /// able to accept another job. **No path waits without a bound** (13b
    /// review fix): at most `writes + grace`.
    ///
    /// - An **unstoppable** write ([`Self::unstoppable`]: a merge, a rebase,
    ///   a `--continue`) is not signalled while the wait lasts: git cut off
    ///   half-way through a rebase leaves it paused. When `writes` is spent
    ///   or `force` says stop (the signal path's second signal), every one
    ///   still live — running or still queued — is stopped half-way and
    ///   named in [`Shutdown::forced`]. A queued one is cancelled before it
    ///   starts, so it never runs.
    /// - Every **other** job is cancelled. `grace` must cover a git child's
    ///   stop — `graft_git::run::STOP_WITHIN`, SIGTERM then SIGKILL, child
    ///   reaped — or a worker still inside that stop is abandoned with its
    ///   child alive ([`super::SHUTDOWN_GRACE`] covers it). A job that
    ///   ignores its token past `grace` is abandoned rather than waited on
    ///   forever.
    /// - A **forced** write — the stop box's `y`, or the bullet above — is
    ///   waited for until it answers where its stop left git, up to
    ///   [`super::FORCED_WAIT`] (13c item 11), and named in
    ///   [`Shutdown::forced`] with that answer. The other jobs' `grace` runs
    ///   alongside that wait, not after it.
    #[must_use]
    pub fn shutdown(
        self,
        grace: Duration,
        writes: Duration,
        force: &dyn graft_git::run::Stop,
    ) -> Shutdown {
        self.shutdown_answered_within(grace, writes, super::FORCED_WAIT, force)
    }

    /// [`Self::shutdown`] with the forced writes' answers waited for up to
    /// `answers` rather than [`super::FORCED_WAIT`]: the seam a test of
    /// that bound uses, so it need not sit the real one out.
    pub(crate) fn shutdown_answered_within(
        self,
        grace: Duration,
        writes: Duration,
        answers: Duration,
        force: &dyn graft_git::run::Stop,
    ) -> Shutdown {
        self.row_requests().clear();
        self.live.cancel_all();
        // Dropping the queue is what ends a worker that is idle: its `recv`
        // fails, and it stops. A job still queued is handed out first.
        drop(self.queue);
        if !self.live.wait_for_unstoppable(writes, force) {
            self.live.force_all_unstoppable();
        }
        let answering = Instant::now();
        let forced = self.live.wait_for_forced(answers);

        let stalled = self
            .alive
            .wait_for_all(grace.saturating_sub(answering.elapsed()));
        if stalled > 0 {
            // The handles are dropped rather than joined, which detaches those
            // threads. They hold nothing but their own job and a channel handle,
            // so the worst they can do is finish and find nobody listening.
            return Shutdown { stalled, forced };
        }

        for handle in self.workers {
            // Every worker has already reached the end of its body, so this
            // returns at once. A worker that panicked outside a job is a bug in
            // this module rather than in a job; there is nothing to recover, and
            // the pool is being torn down anyway.
            handle.join().unwrap_or(());
        }
        Shutdown { stalled: 0, forced }
    }
}
