//! The vocabulary shared by every job: what work looks like, how it ends, and
//! the two seams ([`JobEvents`], [`Dispatch`]) that let the pool and the loop
//! be tested apart from each other.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

use crate::update::{AppEvent, Command, Product};

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
    pub(super) fn new() -> Self {
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

    pub(super) fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
}

/// Where a job's result goes.
///
/// A seam. The event loop's channel carries [`Wake`](crate::event_loop::Wake)s
/// because the keyboard and the signal thread share it; a test wants plain
/// [`Event`](graft_core::event::Event)s and no loop at all. Neither should
/// have to take the other's shape, and the runner does not care which it is
/// talking to.
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
