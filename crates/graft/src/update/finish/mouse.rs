//! The review's mouse: wheel scroll and row clicks.

use super::keys::toggle_diff;
use super::{
    AppState, Command, JobId, MouseButton, MouseInput, MouseKind, WHEEL_LINES, fits,
    keep_cursor_on_screen, with_finish,
};

/// The mouse while the summary is open (§2.15). Claims every event: nothing
/// behind a modal answers a click.
pub(crate) fn finish_mouse(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    mouse: MouseInput,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(finish) = state.finish.as_ref() else {
        return false;
    };
    if finish.committing.is_some() || !fits(state) || crate::update::resolver::box_open(state) {
        return true;
    }
    match mouse.kind {
        MouseKind::ScrollUp => scroll(state, false),
        MouseKind::ScrollDown => scroll(state, true),
        MouseKind::Down(MouseButton::Left) => click(state, commands, mouse, next_id),
        _ => {}
    }
    true
}

/// The wheel: the list moves, the cursor stays where it is.
pub(super) fn scroll(state: &mut AppState, down: bool) {
    let most = {
        let Some(view) = crate::view::finish::finish_view(state) else {
            return;
        };
        graft_ui::finish_summary::max_top(&view, crate::view::finish::summary_area(state))
    };
    with_finish(state, |finish| {
        finish.top = if down {
            finish.top.saturating_add(WHEEL_LINES).min(most)
        } else {
            finish.top.saturating_sub(WHEEL_LINES)
        };
    });
}

/// A left click: a row takes the cursor and toggles its diff.
pub(super) fn click(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    mouse: MouseInput,
    next_id: &mut impl FnMut() -> JobId,
) {
    let landed = {
        let Some(view) = crate::view::finish::finish_view(state) else {
            return;
        };
        graft_ui::finish_summary::hit(
            &view,
            crate::view::finish::summary_area(state),
            mouse.column,
            mouse.row,
        )
    };
    if let Some(row) = landed {
        with_finish(state, |finish| finish.cursor = row);
        toggle_diff(state, commands, next_id);
        keep_cursor_on_screen(state);
    }
}
