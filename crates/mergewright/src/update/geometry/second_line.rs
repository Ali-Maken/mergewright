//! Where the cursor row's second line sits among the graph's drawn rows
//! (ease-pass/07b, mock G9), and so where every history row is drawn.

use super::rows_above_graph;
use crate::update::state::AppState;

/// The screen row a history row is drawn on: under the uncommitted row
/// when it is shown, and one lower below the cursor row, whose second line
/// takes a line (ease-pass/07b). `None` above the viewport. The inverse of
/// `graph_row_at`.
pub fn history_row_line(state: &AppState, row: usize) -> Option<u16> {
    let history = state.history.as_ref()?;
    let mut offset =
        row.checked_sub(history.top)? + usize::from(crate::view::uncommitted_shown(state));
    if cursor_body_line(state).is_some_and(|cursor| offset > usize::from(cursor)) {
        offset += 1;
    }
    Some(rows_above_graph(state).saturating_add(u16::try_from(offset).unwrap_or(u16::MAX)))
}

/// The cursor row's index among the graph pane's drawn rows — the
/// uncommitted row first when it is shown — while it is drawn with its
/// second line (ease-pass/07b): `None` off screen and under a Ghost
/// Preview, whose rows draw none.
pub(crate) fn cursor_body_line(state: &AppState) -> Option<u16> {
    if state.preview.is_some() {
        return None;
    }
    let pinned = crate::view::uncommitted_shown(state);
    if state.on_working_tree {
        return pinned.then_some(0);
    }
    let history = state.history.as_ref()?;
    let at = history.cursor.checked_sub(history.top)?;
    if at >= state.graph_rows() {
        return None;
    }
    u16::try_from(at + usize::from(pinned)).ok()
}
