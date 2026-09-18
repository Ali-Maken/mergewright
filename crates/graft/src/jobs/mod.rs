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
//! between steps and returns [`Outcome::Cancelled`].
//!
//! Split by concern (ticket m1-the-graph/71): [`types`] is the shared
//! vocabulary (what work looks like, how it ends, the two seams); [`pool`] is
//! the queue, the live-job table and the worker threads; [`kinds`] is the work
//! behind each command; [`runner`] is [`JobRunner`], the seam callers use.

use std::time::Duration;

// Not used by this module itself: kept so `jobs::tests`'s `use super::*;` can
// see them, the same way it saw them as plain top-level imports before the
// split (ticket m1-the-graph/71).
#[cfg(test)]
use std::thread;
#[cfg(test)]
use std::time::Instant;

#[cfg(test)]
use graft_core::event::Event;
#[cfg(test)]
use graft_core::job::{JobFailure, JobId};
#[cfg(test)]
use graft_git::history::Order;

#[cfg(test)]
use crate::history;
#[cfg(test)]
use crate::repos::RepoStore;
#[cfg(test)]
use crate::update::{AppEvent, Command, GitCheck, Product, Progress, Task};

#[cfg(test)]
mod tests;

mod detail;
mod kinds;
mod map;
mod pool;
pub(crate) mod radar;
mod runner;
mod types;

pub use pool::Shutdown;
pub use runner::JobRunner;
pub use types::{Cancel, Delivery, Dispatch, JobEvents, Outcome, Work};

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
