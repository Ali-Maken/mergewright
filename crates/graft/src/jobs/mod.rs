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

mod askpass;
pub(crate) mod commit_message;
pub(crate) mod conflict;
pub(crate) mod detail;
mod finish;
pub(crate) mod journal_list;
mod kinds;
mod map;
pub(crate) mod operate;
mod pool;
mod preview;
pub(crate) mod radar;
pub(crate) mod remote;
pub(crate) mod rewrite;
pub(crate) mod rewrite_open;
mod runner;
mod stash;
mod stash_only;
mod switch_list;
mod types;
pub(crate) mod undo;

pub use pool::Shutdown;
pub use runner::JobRunner;
pub use types::{Cancel, Delivery, Dispatch, JobEvents, Outcome, Work};

/// The most workers Graft ever starts.
///
/// Four, on a laptop that is also running an editor and a browser (PROMPT.md
/// §7.1). Git work is mostly disk-bound anyway: more threads past this buys
/// little and costs the battery.
pub const MAX_WORKERS: usize = 4;

/// How long [`JobRunner::shutdown`] waits for cancelled jobs to end before
/// it gives up on them: 2.55 s.
///
/// Long enough for a cancelled job's git child to be stopped and reaped —
/// [`graft_git::run::STOP_WITHIN`] (a 50 ms poll, then SIGTERM and 2 s before
/// SIGKILL) — plus half a second for a job to reach its next check outside
/// git. It was 200 ms, which abandoned a worker still inside that 2 s wait
/// with its git child alive (m8/13b). Most quits pay nothing for it: an idle
/// pool, or one running reads that check their token, ends at once. Only a
/// git child slow to take SIGTERM costs the wait, and that child is exactly
/// what must not outlive Graft. Unstoppable writes are outside this bound
/// altogether: shutdown waits for them to end ([`JobRunner::shutdown`]).
pub const SHUTDOWN_GRACE: Duration =
    graft_git::run::STOP_WITHIN.saturating_add(Duration::from_millis(500));
