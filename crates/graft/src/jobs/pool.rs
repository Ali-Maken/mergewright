//! The pool's own bookkeeping: what is queued, what is still running, and the
//! worker threads that turn one into the other.
//!
//! [`super::runner::JobRunner`] is the seam callers see; everything here is
//! what it is built from.

use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use graft_core::event::Event;
use graft_core::job::{JobFailure, JobId};

use super::types::{Cancel, Delivery, JobEvents, Outcome, Work};
use crate::crash;
use crate::update::AppEvent;

/// A queued job, on its way to whichever worker is free first.
pub(super) struct Assignment {
    pub(super) id: JobId,
    pub(super) cancel: Cancel,
    pub(super) work: Work,
}

/// The jobs that have been submitted and are not over yet, so that a
/// cancellation can find its token.
#[derive(Default)]
pub(super) struct Live {
    tokens: Mutex<HashMap<JobId, Cancel>>,
}

impl Live {
    pub(super) fn register(&self, id: JobId, cancel: Cancel) {
        self.lock().insert(id, cancel);
    }

    /// Forget a job that is over. Its token is no longer anyone's business.
    pub(super) fn finish(&self, id: JobId) {
        self.lock().remove(&id);
    }

    pub(super) fn cancel(&self, id: JobId) {
        if let Some(token) = self.lock().get(&id) {
            token.cancel();
        }
    }

    pub(super) fn cancel_all(&self) {
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
