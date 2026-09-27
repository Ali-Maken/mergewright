//! The trail's life: going back along it, leaving it, and what ends it.

use super::{BlameLoad, Place, Spot, Trail, is_write};
use crate::update::detail::show_line;
use crate::update::events::Command;
use crate::update::state::AppState;

/// `Esc` back along the trail: drop the newest place (stopping its job if
/// it is still reading); popping the last one leaves the trail. `None` when
/// there is no trail to go back along.
///
/// Not the spec's `-> bool`: leaving the trail lands the pane back where
/// `B` was pressed, which is `show_line`'s commands.
pub fn trail_back(state: &mut AppState) -> Option<Vec<Command>> {
    let popped = state.trail.places.pop()?;
    let mut commands = Vec::new();
    if let Place::Blame(blame) = &popped
        && let BlameLoad::Reading { job, .. } = blame.body
    {
        commands.push(Command::CancelJob(job));
    }
    if state.trail.is_live() {
        state.trail.shown = true;
    } else {
        leave(state, &mut commands, true);
    }
    Some(commands)
}

/// End the trail: pin the set-aside base again and, with `restore`, put
/// the graph and the pane back where `B` was pressed if anything moved.
pub(super) fn leave(state: &mut AppState, commands: &mut Vec<Command>, restore: bool) {
    let trail = std::mem::take(&mut state.trail);
    cancel_reading(&trail, commands);
    let Some(origin) = trail.origin else {
        return;
    };
    if let Some(base) = origin.base {
        state.compare_base = Some(base);
    }
    if !restore {
        return;
    }
    // A `⏎` landing still on its way (the walk seeking the commit) is the
    // trail's; he has left it, so it must not land later.
    state.line_landing = None;
    if Spot::of(state) == origin.spot {
        return;
    }
    commands.extend(show_line(state, origin.spec, &origin.path, origin.at));
    match state.line_landing.as_mut() {
        Some(landing) => landing.then_focus = Some(origin.focus),
        None => state.detail.focus = origin.focus,
    }
}

/// Stop every job still reading on `trail`.
pub(super) fn cancel_reading(trail: &Trail, commands: &mut Vec<Command>) {
    for place in &trail.places {
        let Place::Blame(blame) = place;
        if let BlameLoad::Reading { job, .. } = blame.body {
            commands.push(Command::CancelJob(job));
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

/// `Esc` with the trail hidden under the graph or the pane: show it again.
/// `false` with nothing hidden.
pub(crate) fn show_again(state: &mut AppState) -> bool {
    if !state.trail.hidden() {
        return false;
    }
    state.trail.shown = true;
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
