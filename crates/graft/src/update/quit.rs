//! `q`: quit, unless a write is running (keymap §2.2, "confirms if an
//! operation is in progress"; m8-polish-and-ship/13, ruling 4; 13b).
//!
//! A write that can stop (a fetch, an undo): the box says quitting stops
//! it, and its `[ Quit ]` cancels the job before the loop ends. One that
//! **cannot** stop halfway (`Stoppable::No`: a merge, a rebase, a
//! `--continue`): the box says quitting waits for it, and its `[ Quit ]`
//! leaves it running and sets [`QuitWait`]. Graft then quits when the write
//! ends done ([`write_ended`]), and stays when it stops, fails or ends
//! incomplete, so he lands where that answer lands.
//!
//! Git **stopped** mid-operation with nothing running (the banner is up)
//! quits with no box: the stop stays on disk and the next launch offers
//! continue or abort (PROMPT §9; the user's ruling on 13b, 2026-10-02).
//!
//! `Ctrl+C` never boxes, **except** over a write that cannot stop: there it
//! is the same box as `q`, because quitting at once would SIGTERM git
//! half-way through (conductor decision on 13b's Question). The pool keeps
//! the same promise underneath: no cancel and no shutdown signals that write
//! (`jobs::JobRunner::shutdown`).
//!
//! **A stuck write** (13b review fix, keymap §2.2): a git hook can hang, and
//! waiting must not mean waiting for ever. The loop owns the clock — no
//! time is held here, `update` stays pure: it starts counting the turn it
//! first sees [`offer_pending`] name a wait and, at [`STOP_OFFER_AFTER`],
//! calls [`waited_long`]. From then on `q` or `Ctrl+C` opens `Quit now?`,
//! whose `y` is the one key in the app that stops such a write half-way
//! (`Command::ForceStop`, `Quit::StoppedHalfWay`).

use core::time::Duration;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_term::sanitize_line;
use graft_ui::confirm::ConfirmView;
use graft_ui::keymap::ActionId;

use super::confirm::{ConfirmThen, PendingConfirm};
use super::events::Command;
use super::remote::Stoppable;
use super::state::{AppState, Quit, Status};

/// How long Graft waits on a write before `q` offers to stop it half-way.
/// A merge or rebase with ordinary hooks ends inside it; an earlier offer
/// invites stopping a write that was about to land (keymap §2.2).
pub(crate) const STOP_OFFER_AFTER: Duration = Duration::from_secs(10);

/// What the quit box's `[ Quit ]` (or `y`) does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuitThen {
    /// Nothing is running: quit.
    Now,
    /// A write that can stop (`Stoppable::Yes`): stop it, then quit.
    Stop(JobId),
    /// A write that cannot (`Stoppable::No`): quit once it ends done.
    WaitFor(JobId),
    /// The stop box over a write waited on past [`STOP_OFFER_AFTER`]: stop
    /// it half-way (`Command::ForceStop`), then quit.
    StopNow(JobId),
}

/// Graft waiting to quit until `job` ends (`AppState::quit_when_done`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuitWait {
    /// The write waited for.
    pub job: JobId,
    /// `<op> · quitting when it finishes · esc stays`, drawn over the
    /// status while the wait stands (`AppState::shown_status`); after
    /// [`waited_long`], `<op> · still running after 10 s · q stop it now ·
    /// esc stays`.
    pub line: Status,
    /// [`STOP_OFFER_AFTER`] has passed (the loop called [`waited_long`]):
    /// `q` and `Ctrl+C` open the stop box.
    pub long: bool,
}

/// The write whose wait the loop is counting toward the stop offer: a wait
/// stands and has not been called long. `None` otherwise, so nothing wakes
/// the loop for it after the offer. The loop starts its count when this
/// first names a job, and starts again if it names another.
pub(crate) fn offer_pending(state: &AppState) -> Option<JobId> {
    waiting(state)
        .filter(|wait| !wait.long)
        .map(|wait| wait.job)
}

/// The loop, [`STOP_OFFER_AFTER`] into the wait: it has gone on long
/// enough to offer stopping the write. The line is fixed text — no ticking
/// count — so nothing needs to wake after this.
pub fn waited_long(state: &mut AppState) {
    let what = match waiting(state) {
        Some(wait) if !wait.long => running_description(state, wait.job),
        _ => return,
    };
    let quit = state.keymap.spell(&ActionId("app.quit"));
    let back = state.keymap.spell(&ActionId("app.back"));
    let line = Status::Waiting(format!(
        "{what} \u{b7} still running after {} s \u{b7} {} stop it now \u{b7} {} stays",
        STOP_OFFER_AFTER.as_secs(),
        quit.0,
        back.0,
    ));
    if let Some(wait) = state.quit_when_done.as_mut() {
        wait.long = true;
        wait.line = line;
    }
}

/// A quit key (`q` or `Ctrl+C`) reached the graph or the focused banner:
/// quit now, or open the box that asks first.
///
/// While Graft already waits to quit, neither key changes anything for the
/// first [`STOP_OFFER_AFTER`]: the status row says what is happening, and a
/// second box would ask the same. After it, either opens the stop box.
pub(super) fn asked(state: &mut AppState, key: KeyPress) {
    if let Some(wait) = waiting(state) {
        let (long, job) = (wait.long, wait.job);
        if long {
            state.confirm = Some(stop_box(state, job));
        }
        return;
    }
    let control_c = key.code == KeyCode::Char('c') && key.modifiers.control;
    let ask = if control_c {
        // Never boxed, but for the one write quitting at once would cut
        // off half-way.
        running_box(state).filter(|pending| {
            matches!(
                pending.then,
                ConfirmThen::Quit {
                    then: QuitThen::WaitFor(_)
                }
            )
        })
    } else {
        running_box(state)
    };
    match ask {
        Some(pending) => state.confirm = Some(pending),
        None => state.quit = Some(Quit::Requested),
    }
}

/// The box's `[ Quit ]`.
pub(super) fn confirmed(state: &mut AppState, commands: &mut Vec<Command>, then: QuitThen) {
    match then {
        QuitThen::Now => state.quit = Some(Quit::Requested),
        QuitThen::Stop(id) => {
            if state.operating == Some(id) {
                let asked = super::remote::stop_write(state, commands);
                // Asked, yet no cancel recorded: it cannot be stopped safely,
                // and `stop_write` has put why on the status line.
                if asked && state.remote.cancelling != Some(id) {
                    return;
                }
            }
            state.quit = Some(Quit::Requested);
        }
        QuitThen::WaitFor(id) if state.operating == Some(id) => {
            let what = running_description(state, id);
            let back = state.keymap.spell(&ActionId("app.back"));
            state.quit_when_done = Some(Box::new(QuitWait {
                job: id,
                line: Status::Waiting(format!(
                    "{what} \u{b7} quitting when it finishes \u{b7} {} stays",
                    back.0
                )),
                long: false,
            }));
        }
        // The stop box's `[ Quit ]`: the one path that stops a write that
        // cannot stop half-way. The pool lifts the write's mark and stops
        // its git (SIGTERM, SIGKILL after 2 s, reaped); Graft quits and the
        // shell gets one line saying what was stopped.
        QuitThen::StopNow(id) => {
            if !waiting(state).is_some_and(|wait| wait.job == id && wait.long) {
                // The write ended while the box was up: its answer has
                // landed and dropped or spent the wait. Nothing to stop.
                return;
            }
            let what = running_description(state, id);
            commands.push(Command::ForceStop(id));
            state.quit_when_done = None;
            state.quit = Some(Quit::StoppedHalfWay { what });
        }
        // It ended while the box was up, so its answer has landed. Quit
        // unless that answer left something to see — a stop, or its
        // failure's words — which is where the wait would have left him.
        QuitThen::WaitFor(_) => {
            if state.in_progress.is_none() && !state.status.is_failure() {
                state.quit = Some(Quit::Requested);
            }
        }
    }
}

/// The write `job` answered, its answer applied and journaled. Called by
/// every answer path of a `Stoppable::No` write — `operate::operated`,
/// `banner::resumed`, `undo::reverted_to` — last, after the answer's own
/// state and words are in place. A set-aside put-back runs inside the same
/// job (ADR-0045 §4), so its answer is this one.
///
/// `done` only for a complete answer (`Operated::Done`, "up to date"
/// included; `RevertedTo::Done`): then Graft quits. Anything else — stopped,
/// failed, incomplete, refused — drops the wait and he stays, on whatever
/// the answer landed him on.
pub(crate) fn write_ended(state: &mut AppState, job: JobId, done: bool) {
    if state
        .quit_when_done
        .as_ref()
        .is_none_or(|wait| wait.job != job)
    {
        return;
    }
    state.quit_when_done = None;
    if done {
        state.quit = Some(Quit::Requested);
    }
}

/// `Esc` while Graft waits to quit: drop the wait; the write runs on.
/// `true` when there was a wait, so that `Esc` goes no further — it is
/// `stop_write`'s first question, after a held failure's dismiss (§2.2
/// `Esc`).
pub(crate) fn drop_wait(state: &mut AppState) -> bool {
    let waited = waiting(state).is_some();
    state.quit_when_done = None;
    waited
}

/// The wait, while its write still runs. A wait whose write has gone
/// without an answer reaching [`write_ended`] (a panicked job) counts as
/// none: nothing quits on it, and nothing draws it.
pub(crate) fn waiting(state: &AppState) -> Option<&QuitWait> {
    state
        .quit_when_done
        .as_deref()
        .filter(|wait| state.operating == Some(wait.job))
}

/// A write is running. One that can stop: `<op> is still running · quit
/// stops it`. One that cannot: `<op> is still running · quit waits for it
/// to finish` / `if it stops at a conflict or fails, you stay to see it`
/// (keymap §2.2, "`q` while a write runs").
fn running_box(state: &AppState) -> Option<PendingConfirm> {
    let id = state.operating?;
    let what = running_description(state, id);
    let stoppable = state
        .remote
        .running
        .as_ref()
        .filter(|running| running.id == id)
        .map_or(Stoppable::Yes, |running| running.stoppable);
    Some(match stoppable {
        Stoppable::Yes => quit_box(
            "Quit?",
            &[&format!("{what} is still running \u{b7} quit stops it")],
            QuitThen::Stop(id),
        ),
        Stoppable::No(_) => quit_box(
            "Quit?",
            &[
                &format!("{what} is still running \u{b7} quit waits for it to finish"),
                "if it stops at a conflict or fails, you stay to see it",
            ],
            QuitThen::WaitFor(id),
        ),
    })
}

/// The stop box (keymap §2.2, "a stuck write"): `Quit now?`, naming the
/// cost. Cursor on `Cancel`; `y` quits.
fn stop_box(state: &AppState, id: JobId) -> PendingConfirm {
    let what = running_description(state, id);
    quit_box(
        "Quit now?",
        &[
            &format!("{what} is still running \u{b7} a git hook may be stuck"),
            "quit now stops it half-way \u{b7} the next launch shows where it stopped",
        ],
        QuitThen::StopNow(id),
    )
}

/// What the running write `id` is called: its description, or `a write`.
fn running_description(state: &AppState, id: JobId) -> String {
    state
        .remote
        .running
        .as_ref()
        .filter(|running| running.id == id)
        .map_or_else(
            || "a write".to_owned(),
            |running| running.description.clone(),
        )
}

fn quit_box(title: &str, lines: &[&str], then: QuitThen) -> PendingConfirm {
    PendingConfirm {
        view: ConfirmView::new(
            sanitize_line(title),
            lines.iter().map(|line| sanitize_line(line)).collect(),
            // No git command runs: quitting only stops or waits for what is
            // running.
            sanitize_line(""),
        )
        .with_labels("Cancel", "Quit"),
        then: ConfirmThen::Quit { then },
    }
}

#[cfg(test)]
pub(crate) mod tests;
