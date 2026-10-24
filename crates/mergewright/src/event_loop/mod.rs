//! The loop that turns a terminal into an application.
//!
//! ```text
//! Event ──▶ update(state) ──▶ (new state, [Command]) ──▶ view(state)
//! ```
//!
//! Three things can wake it, and they all arrive down one channel as a
//! [`Wake`]: the user did something, a signal said stop, or reading the
//! terminal failed. Between wakes the loop blocks — an idle Mergewright uses no CPU,
//! because nothing polls and nothing ticks.
//!
//! [`run`] is the whole application: it sets up the terminal, installs the
//! panic hook, asks the terminal what it can do, starts the input and signal
//! threads, and hands control to [`pump`], which is the part that can be
//! tested without a terminal at all.
//!
//! Split by concern (m1-the-graph/71): [`startup`] builds the terminal
//! session and runs [`run`]; [`headless`] is the no-terminal path for a
//! piped stdout; [`running`] is the loop's body ([`pump`]); [`outside`] hands
//! the terminal to `$EDITOR` and takes it back. This file keeps what both
//! startup and headless share: the wake types, and `opening_repository`, the
//! state both paths start from.

mod exit_wait;
mod graphics;
pub use graphics::{Placed, Room, placements};
mod headless;
pub mod input;
mod label;
pub mod outside;
mod pace;
pub mod places;
pub mod redetect;
mod rest;
mod row_clear;
mod running;
pub mod signals;
mod startup;

use std::io;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::time::Instant;

use mergewright_core::event::{Event, MouseKind};

use crate::jobs::{Delivery, Dispatch, JobEvents};
use crate::session::TerminalChange;
use crate::update::{AppEvent, AppState};

pub use exit_wait::waiting_line;
pub use places::{OutsideStart, Place, drive, locate};
pub use running::pump;
pub use startup::{Ran, Terminal, run};

/// Why the loop stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Exit {
    /// The user asked to leave.
    Quit,
    /// The user pressed one of `Harness::hand_back`'s keys; the caller
    /// asked for it back and acts on it now that the terminal is restored.
    HandedBack(mergewright_core::event::KeyCode),
    /// This signal arrived. The caller re-raises it once the terminal is back,
    /// so the process dies the way it was told to.
    Signal(i32),
    /// Mergewright could not go on, and this is the reason to print. Carried out of
    /// the loop rather than printed inside it: a message written into the
    /// alternate screen disappears with the alternate screen, which is how a
    /// program comes to "exit without saying anything".
    Fatal(String),
    /// The quit stop box's `y` (m8/13b review fix): the write `job`, `what`,
    /// was stopped after `after` of waiting. Once the terminal is restored,
    /// the pool's stop prints its line, saying where git was left
    /// (`update::quit::exit_line`, 13d).
    StoppedHalfWay {
        /// The write being stopped.
        job: mergewright_core::job::JobId,
        /// The write's description.
        what: String,
        /// How long Mergewright had waited on it.
        after: std::time::Duration,
    },
    /// Open this place next, on the same terminal (m8-polish-and-ship/08).
    /// [`places::drive`] handles it and never returns it; [`run`], which
    /// runs one repository and gives the terminal back, never produces it.
    Open(Open),
    /// Build a practice repository and open it, coming back to `from` when
    /// the tour ends (m8-polish-and-ship/19b): the palette's `Practice tour`
    /// row, or `t` on the empty state. Handled by [`places::drive`].
    Practise {
        /// Where he was, and his cursor there.
        from: Open,
    },
    /// Leave the practice repository for where the tour was started from:
    /// `Esc` on the graph with nothing else to close, or every task done
    /// (`finished`). Handled by [`places::drive`].
    LeaveTour {
        /// All eight tasks ticked: the next place says so.
        finished: bool,
    },
}

/// A place to open next: a folder, and the commit his cursor goes to there
/// (ticket m8/19 Amendment §A4). `cursor: None` lands as every launch and
/// open does — on `HEAD`; only the tour's way back sets it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Open {
    /// The folder: a repository, or one `locate` turns into the empty state.
    pub(crate) path: std::path::PathBuf,
    /// The commit to land on there, if it can still be found.
    pub(crate) cursor: Option<mergewright_core::oid::Oid>,
}

impl Open {
    /// `path`, landing on `HEAD`.
    pub(crate) fn at(path: std::path::PathBuf) -> Self {
        Self { path, cursor: None }
    }

    /// The folder asked for (the walks name it when `run` refuses one).
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

/// Something that wakes the loop.
#[derive(Debug)]
pub enum Wake {
    /// The user did something, or a job came back with something.
    Event(AppEvent),
    /// A signal arrived and Mergewright should stop. Carries the signal number so
    /// the exit status can be honest about which one it was.
    Shutdown(i32),
    /// Reading the terminal failed. Not recoverable — there is no input any
    /// more — and not ignorable, so it wakes the loop to be reported.
    InputFailed(io::Error),
    /// The terminal re-answered a question and the answer changed
    /// (DESIGN.md §4.5). Applied to the session, not to `update`: it changes
    /// what Mergewright draws with, not what it shows.
    Terminal(TerminalChange),
    /// Writing a re-query to the terminal, or reading its reply, failed. As
    /// unrecoverable as [`Wake::InputFailed`]: the next frame would fail too.
    TerminalFailed(io::Error),
    /// The terminal answered a frame's fence (`CSI 0 n`, ADR-0054): it has
    /// read every byte of that frame. Releases the next frame; changes no
    /// state.
    Fence,
}

/// What waiting with a deadline came to.
#[derive(Debug)]
pub enum Waited {
    /// Something happened before the deadline.
    Woke(Wake),
    /// The deadline passed and nothing happened.
    TimedOut,
    /// Nothing can happen again: every producer has gone.
    Ended,
}

/// Where the loop gets its wakes.
///
/// A seam, so [`pump`] can be driven from a test with a scripted list of wakes
/// instead of a terminal, a signal and two threads.
pub trait Wakes {
    /// Block until something happens. `None` once nothing can happen again,
    /// because every producer has gone.
    fn next_wake(&mut self) -> Option<Wake>;

    /// Block until something happens or `deadline` passes. The loop waits
    /// this way while it holds a frame it may not draw yet (a fence not
    /// answered, the 16 ms cap): the deadline is when it may. Required, with
    /// no default: a source that blocked past the deadline would hold a
    /// frame back until the next key.
    fn next_wake_by(&mut self, deadline: Instant) -> Waited;

    /// Is a wake already here? Never blocks. The loop asks between pieces of
    /// idle work (the strips a page key would show) and stops as soon as the
    /// answer is `true`; a source that has ended says `true` too, so the loop
    /// goes straight to finding that out.
    fn waiting(&mut self) -> bool;

    /// Is a wake already here, and does it [`joins_frame`]? Never blocks.
    /// The loop applies such wakes in the same turn as the one before and
    /// draws once (m7-power-tools/43): keys that pile up behind a slow frame
    /// are all shown by the next one, not one frame each. Required, with no
    /// default: a source that skipped it would silently draw every wake.
    fn joining_waiting(&mut self) -> bool;

    /// One phase of the frame for the wake just handed out has ended. A
    /// timing seam for the `frames` perf harness (m7-power-tools/04), which
    /// stamps each phase to find where a slow key's time went. The real
    /// source keeps this empty default: the call compiles to nothing.
    fn mark(&mut self, _phase: Phase) {}

    /// The loop's clock. Every time the loop reads — the pacer's, a batch's
    /// budget, when the cursor last moved (`rest`) — comes from here, so a
    /// test source can hand out wakes on a timeline of its own and the loop
    /// sees the same one, without sleeping through it. The real source keeps
    /// this default.
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Where the loop stands in the frame for one wake, in order. The frame
/// starts when [`Wakes::next_wake`] returns and ends at [`Phase::Drawn`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The terminal's replies to earlier image writes are read.
    Drained,
    /// `update` has returned the next state.
    Updated,
    /// The frame's commands are dispatched or carried out.
    CarriedOut,
    /// The frame is on the terminal.
    Drawn,
}

/// The real source: one channel fed by the input thread and the signal thread.
pub struct ChannelWakes {
    receiver: Receiver<Wake>,
    /// A wake `waiting` took off the channel to see it; handed out first.
    seen: Option<Wake>,
}

impl ChannelWakes {
    /// A channel, and the sender its producers clone.
    #[must_use]
    pub fn new() -> (Self, Sender<Wake>) {
        let (sender, receiver) = mpsc::channel();
        (
            Self {
                receiver,
                seen: None,
            },
            sender,
        )
    }

    /// The terminal input the loop never got to, in order: what the input
    /// forwarder sent after the wake that ended the loop. Everything else
    /// still queued (job results, signals, terminal changes) is for a loop
    /// that has finished, and is dropped with it.
    ///
    /// Only complete once the forwarder has stopped (`run` joins it first).
    #[must_use]
    pub fn unhandled_input(self) -> Vec<Event> {
        self.seen
            .into_iter()
            .chain(self.receiver.try_iter())
            .filter_map(|wake| match wake {
                Wake::Event(AppEvent::Core(event)) => Some(event),
                Wake::Event(_)
                | Wake::Shutdown(_)
                | Wake::InputFailed(_)
                | Wake::Terminal(_)
                | Wake::TerminalFailed(_)
                | Wake::Fence => None,
            })
            .collect()
    }
}

/// A job's result is a wake like any other: the loop reads it from the same
/// channel as the keyboard, in the order it arrived.
impl JobEvents for Sender<Wake> {
    fn deliver(&self, event: AppEvent) -> Delivery {
        match self.send(Wake::Event(event)) {
            Ok(()) => Delivery::Received,
            Err(_returned) => Delivery::NobodyListening,
        }
    }
}

impl Wakes for ChannelWakes {
    fn next_wake(&mut self) -> Option<Wake> {
        if let Some(seen) = self.seen.take() {
            return Some(seen);
        }
        // The only error `recv` can give is "all senders dropped", which is
        // not a failure: it means both threads have ended and no wake can ever
        // arrive again. That is exactly what `None` says.
        self.receiver.recv().ok()
    }

    fn next_wake_by(&mut self, deadline: Instant) -> Waited {
        if let Some(seen) = self.seen.take() {
            return Waited::Woke(seen);
        }
        let wait = deadline.saturating_duration_since(Instant::now());
        match self.receiver.recv_timeout(wait) {
            Ok(wake) => Waited::Woke(wake),
            Err(RecvTimeoutError::Timeout) => Waited::TimedOut,
            Err(RecvTimeoutError::Disconnected) => Waited::Ended,
        }
    }

    fn waiting(&mut self) -> bool {
        if self.seen.is_some() {
            return true;
        }
        match self.receiver.try_recv() {
            Ok(wake) => {
                self.seen = Some(wake);
                true
            }
            Err(TryRecvError::Empty) => false,
            // Every producer has gone: `next_wake` says so at once.
            Err(TryRecvError::Disconnected) => true,
        }
    }

    fn joining_waiting(&mut self) -> bool {
        self.waiting() && self.seen.as_ref().is_some_and(joins_frame)
    }
}

/// A job said something: it started, answered, reported progress, or ended.
/// Everything else — the user's input, a signal, the terminal — is not.
#[must_use]
pub fn is_job_event(wake: &Wake) -> bool {
    match wake {
        Wake::Event(event) => said_by_a_job(event),
        Wake::Shutdown(_)
        | Wake::InputFailed(_)
        | Wake::Terminal(_)
        | Wake::TerminalFailed(_)
        | Wake::Fence => false,
    }
}

/// Can this wake be applied in the same turn as the one before it, sharing
/// one draw? Job events, keys (presses and repeats: no release reaches the
/// loop) and wheel scrolls. `update` is pure and runs each in order, so the
/// state drawn is the one drawing between them would have ended on. Nothing
/// else joins: a click, drag or hover acts on what he saw, so the frame
/// before it is drawn first; paste, resize, focus, signals and terminal
/// wakes start their own turn.
#[must_use]
pub fn joins_frame(wake: &Wake) -> bool {
    match wake {
        Wake::Event(AppEvent::Core(Event::Key(_))) => true,
        Wake::Event(AppEvent::Core(Event::Mouse(mouse))) => matches!(
            mouse.kind,
            MouseKind::ScrollUp
                | MouseKind::ScrollDown
                | MouseKind::ScrollLeft
                | MouseKind::ScrollRight
        ),
        wake => is_job_event(wake),
    }
}

/// [`is_job_event`] for an event already taken out of its wake.
fn said_by_a_job(event: &AppEvent) -> bool {
    match event {
        AppEvent::Started(..) | AppEvent::Produced(..) | AppEvent::Progress(..) => true,
        // The loop's own: never in a wake.
        AppEvent::Rested | AppEvent::GraphModeKept { .. } | AppEvent::PixelsFailed(_) => false,
        AppEvent::Core(event) => matches!(
            event,
            Event::JobFinished(_) | Event::JobFailed(..) | Event::JobCancelled(_)
        ),
    }
}

/// The state Mergewright starts in, with its opening jobs already on their way.
///
/// The list itself is [`crate::update::opening`]'s, so that the id the
/// history walk runs under is minted in one place (see its doc).
fn opening_repository(
    jobs: &impl Dispatch,
    repo_path: &Path,
    size: crate::update::Size,
) -> AppState {
    let (state, commands) = crate::update::opening(repo_path.to_path_buf(), size);
    for command in commands {
        jobs.dispatch(command);
    }
    state
}

#[cfg(test)]
mod tests;
