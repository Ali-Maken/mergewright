//! Keeping a shown cursor shown when the graph pane loses history rows
//! (m7-power-tools/48, Decided 5).
//!
//! [`AppState::graph_rows`] changes without a key: a status read that finds
//! the tree dirty pins the uncommitted row on top and takes a history row
//! with it. A cursor on the last drawn row would then sit one row below the
//! pane. This runs once, at the end of [`super::update`], and scrolls just
//! enough to keep it drawn.

use super::events::Command;
use super::state::AppState;

/// After `update`: if the pane's history rows shrank and the cursor was on
/// screen before the event, move `top` down so it is on screen still, and
/// ask for the rows that now fit.
///
/// A cursor the wheel left off screen stays off screen (m7/37): the rule
/// only keeps what was shown. Nothing to do while `on_working_tree` is set,
/// because the cursor is then the uncommitted row itself, always drawn.
pub(crate) fn keep_cursor_in_view(
    before: &AppState,
    after: &mut AppState,
    commands: &mut Vec<Command>,
) {
    let (height_before, height_after) = (before.graph_rows(), after.graph_rows());
    if height_after >= height_before || after.on_working_tree {
        return;
    }
    let Some(was) = before.history.as_ref() else {
        return;
    };
    if !(was.top..was.top + height_before).contains(&was.cursor) {
        return;
    }
    let Some(history) = after.history.as_mut() else {
        return;
    };
    let last_drawn = history.top + height_after;
    if history.cursor < last_drawn {
        return;
    }
    history.top = history.cursor + 1 - height_after;
    if let Some(request) = history.show(height_after)
        && let Some(id) = after.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}
