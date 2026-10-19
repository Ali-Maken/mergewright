//! The mouse while the picker is shown (keymap §2.16).

use mergewright_core::event::{MouseButton, MouseInput, MouseKind};

use super::{DOUBLE_CLICK_MS, enter, shown};
use crate::update::events::Command;
use crate::update::state::AppState;

/// The mouse while the picker is shown: a click on a row selects it, a
/// second on the same row within [`DOUBLE_CLICK_MS`] switches to it — `⏎`'s
/// own path. The picker is modal (keymap §2.16), so it claims every mouse
/// event: nothing reaches the graph behind it. `true` when it claimed one.
pub(in crate::update) fn picker_mouse(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    mouse: MouseInput,
) -> bool {
    if !shown(state) {
        return false;
    }
    if mouse.kind != MouseKind::Down(MouseButton::Left) {
        return true;
    }
    let Some(area) = crate::update::geometry::picker_bounds(state) else {
        return true;
    };
    if !area.contains(mouse.column, mouse.row) {
        return true;
    }
    let now = state.harness.clock.now_millis();
    let Some(picker) = state.picker.as_mut() else {
        return true;
    };
    let count = picker.rows().len();
    let Some(row) =
        mergewright_ui::picker::row_at(area.height, count, picker.selected, mouse.row - area.y)
    else {
        return true;
    };
    let double = picker
        .last_click
        .is_some_and(|(at, when)| at == row && now - when <= DOUBLE_CLICK_MS);
    picker.selected = row;
    picker.note = None;
    if double {
        picker.last_click = None;
        enter(state, commands);
    } else {
        picker.last_click = Some((row, now));
    }
    true
}
