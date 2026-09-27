//! The trail's life: going back along it, leaving it, and what ends it.

use super::{BlameLoad, FileLoad, Place, Spot, TraceLoad, Trail, is_write};
use crate::update::detail::{abandon, show_line};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// `Esc` back along the trail: drop the newest place (stopping its job if
/// it is still reading); popping the last one leaves the trail. `None` when
/// there is no trail to go back along.
///
/// Not the spec's `-> bool`: leaving the trail lands the pane back where
/// `B` was pressed, which is `show_line`'s commands.
pub fn trail_back(state: &mut AppState) -> Option<Vec<Command>> {
    let popped = state.trail.places.pop()?;
    let mut commands = Vec::new();
    cancel_place(&popped, &mut commands);
    if state.trail.is_live() {
        state.trail.shown = true;
    } else {
        leave(state, &mut commands, true);
    }
    Some(commands)
}

/// End the trail: drop a `⏎` landing still on its way, pin the set-aside
/// base again and, with `restore`, put the graph and the pane back where `B`
/// was pressed if anything moved.
///
/// Every way off the trail ends that landing: `Esc` off the last place and
/// a refused read (here, `restore`), a write (`after_commands`) and a new
/// pin (`end_for_compare`) all come through here; going back to a hidden
/// trail (`show_again`) calls `abandon` itself (20n, gap M7/20k-q3).
pub(super) fn leave(state: &mut AppState, commands: &mut Vec<Command>, restore: bool) {
    let trail = std::mem::take(&mut state.trail);
    cancel_reading(&trail, commands);
    // A `⏎` landing still on its way (the walk seeking the commit) is the
    // trail's; he has left it, so it must not land later, nor its seek
    // move the graph. A landing that opened the pane on an old commit after
    // a write or a new pin would answer a request he has moved on from.
    abandon(state, commands);
    let Some(origin) = trail.origin else {
        return;
    };
    if let Some(base) = origin.base {
        state.compare_base = Some(base);
    }
    if !restore {
        return;
    }
    if Spot::of(state) == origin.spot {
        return;
    }
    commands.extend(show_line(
        state,
        origin.spec,
        &origin.path,
        origin.at,
        origin.section,
    ));
    match state.line_landing.as_mut() {
        Some(landing) => landing.then_focus = Some(origin.focus),
        None => state.detail.focus = origin.focus,
    }
}

/// Stop every job still reading on `trail`.
pub(super) fn cancel_reading(trail: &Trail, commands: &mut Vec<Command>) {
    for place in &trail.places {
        cancel_place(place, commands);
    }
}

/// Stop `place`'s jobs still running: a blame's read; a history's trace
/// (`Esc` while tracing stops git, ruling a) and its whole-file read.
fn cancel_place(place: &Place, commands: &mut Vec<Command>) {
    match place {
        Place::Blame(blame) => {
            if let BlameLoad::Reading { job, .. } = blame.body {
                commands.push(Command::CancelJob(job));
            }
        }
        Place::History(history) => {
            if let TraceLoad::Tracing { job } = history.trace {
                commands.push(Command::CancelJob(job));
            }
            if let FileLoad::Reading { job, .. } = history.file {
                commands.push(Command::CancelJob(job));
            }
        }
    }
}

/// After every event: a write among `commands` ends the trail and pins the
/// set-aside base again (20's pinned-base ruling). Nothing is restored: the
/// write lands where it lands.
pub(crate) fn after_commands(state: &mut AppState, commands: &mut Vec<Command>) {
    if state.trail.is_live() && commands.iter().any(is_write) {
        leave(state, commands, false);
    }
}

/// `Esc` on the graph screen with the trail hidden: show it again, the
/// place's cursor where `⏎` left it. `false` (nothing done) unless
/// [`esc_goes_back`] says this `Esc` is the one that goes back. Called from
/// the pane's `Esc` (`detail::keys`) and the graph's last `Esc` rung
/// (`update::keys::escape_key`).
///
/// Going back abandons a `⏎` landing still on its way, as leaving does
/// (`leave`): one rule for both ways off the landing (32i-q1). Its seek is
/// withdrawn with it (`detail::abandon`), or the walk's answer would still
/// move the graph and the pane under blame.
pub(crate) fn show_again(state: &mut AppState, commands: &mut Vec<Command>) -> bool {
    if !esc_goes_back(state) {
        return false;
    }
    state.trail.shown = true;
    abandon(state, commands);
    true
}

/// A new compare base pinned (`x`, `Shift`/`Ctrl`+click) while the trail
/// is hidden: a new task, so the trail ends and the set-aside base is
/// dropped (20's pinned-base ruling). Call before the pin; `true` when a
/// trail ended, for the caller to say so once the pin is known.
pub(crate) fn end_for_compare(state: &mut AppState, commands: &mut Vec<Command>) -> bool {
    if !state.trail.is_live() {
        return false;
    }
    if let Some(origin) = state.trail.origin.as_mut() {
        origin.base = None;
    }
    leave(state, commands, false);
    true
}

/// **`Esc` on the graph screen would show the hidden trail now** (keymap
/// §2.1; 20i): a trail is hidden and no rung before it applies. Those rungs,
/// in the order `update::keys::key_event` answers them:
/// 1. a held failure on the status line (§2.2);
/// 2. the graph's own open thing — a badge in the air, a box, a prompt or
///    other modal, the `.` menu, an overlay (the editor, Map, journal,
///    stash), a write `Esc` would stop, Explain, a pinned base, the search,
///    Focus mode.
///
/// The pane `⏎` opened is **not** one of them: with the graph or the pane
/// focused, open or not, this `Esc` goes back. A walk still seeking (`G`, the
/// landing) is not either: going back is what he asked for.
///
/// The one place it is decided: [`show_again`] asks it (so the pane's `Esc`
/// and `escape_key`'s last rung both do), `detail::keys` asks it before
/// closing the pane, and `view::rows`' `BackToBlame` wrap leads
/// `esc back to blame` exactly when it is true.
pub(crate) fn esc_goes_back(state: &AppState) -> bool {
    state.trail.hidden() && !held_failure(state) && !graph_has_one_open(state)
}

/// Rung 1: a failure `Esc` dismisses first.
fn held_failure(state: &AppState) -> bool {
    state.unseen.is_some() || matches!(state.status, Status::Failed(_) | Status::Sticky { .. })
}

/// Rung 2: something on the graph screen whose own `Esc` closes it first.
fn graph_has_one_open(state: &AppState) -> bool {
    state.drag.is_some()
        || state.modal_owns_keys()
        || state.context_menu.is_some()
        || state.graph_overlay().is_some()
        || state.operating.is_some()
        || state.detail.explain.is_some()
        || state.compare_base.is_some()
        || state.search.is_some()
        || state.focus
}
