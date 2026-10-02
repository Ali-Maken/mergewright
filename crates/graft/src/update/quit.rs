//! `q`: quit, unless something would be lost or left behind by it
//! (keymap §2.2, "confirms if an operation is in progress";
//! m8-polish-and-ship/13, ruling 4).
//!
//! Two things count as "in progress":
//! - **a write running** — quitting stops it, so the box says so and its
//!   `[ Quit ]` cancels the job before the loop ends;
//! - **git stopped mid-operation** (the banner is up). The planner proposed
//!   quitting without a box here, since relaunch offers continue/abort
//!   (PROMPT §9); that awaits the user, so until then DESIGN.md is followed
//!   literally and it boxes too. It is one branch, [`stopped_box`], with its
//!   own test, so it can be flipped alone.
//!
//! `Ctrl+C` never boxes: it quits from anywhere (keymap §2.1).

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_term::sanitize_line;
use graft_ui::confirm::ConfirmView;

use super::confirm::{ConfirmThen, PendingConfirm};
use super::events::Command;
use super::state::{AppState, Quit};

/// A quit key (`q` or `Ctrl+C`) reached the graph: quit now, or open the
/// box that asks first.
pub(super) fn asked(state: &mut AppState, key: KeyPress) {
    let control_c = key.code == KeyCode::Char('c') && key.modifiers.control;
    let ask = if control_c {
        None
    } else {
        running_box(state).or_else(|| stopped_box(state))
    };
    match ask {
        Some(pending) => state.confirm = Some(pending),
        None => state.quit = Some(Quit::Requested),
    }
}

/// The box's `[ Quit ]`: stop the write it named, if it still runs, then
/// quit. A write that cannot be stopped safely keeps running and the app
/// stays — `stop_write` has put why on the status line; quitting would cut
/// it off halfway.
pub(super) fn confirmed(state: &mut AppState, commands: &mut Vec<Command>, stops: Option<JobId>) {
    if let Some(id) = stops.filter(|id| state.operating == Some(*id)) {
        let asked = super::remote::stop_write(state, commands);
        // Asked, yet no cancel recorded: it cannot be stopped safely.
        if asked && state.remote.cancelling != Some(id) {
            return;
        }
    }
    state.quit = Some(Quit::Requested);
}

/// A write is running: `<op> is still running · quit stops it`.
fn running_box(state: &AppState) -> Option<PendingConfirm> {
    let id = state.operating?;
    let what = state
        .remote
        .running
        .as_ref()
        .filter(|running| running.id == id)
        .map_or("a write", |running| running.description.as_str());
    Some(quit_box(
        &format!("{what} is still running \u{b7} quit stops it"),
        Some(id),
    ))
}

/// **Pending the user's ruling** (see the module doc): git is stopped
/// mid-operation, so `q` boxes as DESIGN.md §2.2 says. Return `None` here
/// to make it quit straight away.
fn stopped_box(state: &AppState) -> Option<PendingConfirm> {
    let kind = state.in_progress.as_ref()?.kind;
    Some(quit_box(
        &format!(
            "{} is stopped \u{b7} next launch offers continue or abort",
            kind.name()
        ),
        None,
    ))
}

fn quit_box(line: &str, stops: Option<JobId>) -> PendingConfirm {
    PendingConfirm {
        view: ConfirmView::new(
            sanitize_line("Quit?"),
            vec![sanitize_line(line)],
            // No git command runs: quitting only stops what is running.
            sanitize_line(""),
        )
        .with_labels("Cancel", "Quit"),
        then: ConfirmThen::Quit { stops },
    }
}

#[cfg(test)]
mod tests;
