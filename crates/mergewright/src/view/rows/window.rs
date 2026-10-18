//! The window cut: which row the footer reveals, the divider, the lane
//! cap and the compaction.

use mergewright_graph::row::Row;
use mergewright_ui::graph_column::RowPaint;
use mergewright_ui::graph_screen::{Compaction, GraphEntry};
use std::collections::BTreeSet;

use crate::history::History;
use crate::update::AppState;

use super::super::entries::{Thread, uncommitted_shown};
use super::keep_for;

/// Which of `rows` the footer reveals in full, if any (ADR-0023): the
/// hovered row, else none. `hover` is an absolute history row number; `rows` is indexed from
/// the top of the viewport, offset by one when the uncommitted row draws
/// above it (`visible_rows`) — the same arithmetic, so the two never
/// disagree about which row is which. `None` when the row named is not
/// actually on screen (scrolled past, or the uncommitted row itself, which
/// has no subject to reveal).
pub(super) fn reveal_index(
    state: &AppState,
    history: &History,
    rows: &[GraphEntry<'_>],
) -> Option<usize> {
    let absolute = state.hover?;
    let offset = usize::from(uncommitted_shown(state));
    let index = offset + absolute.checked_sub(history.top)?;
    (index < rows.len()).then_some(index)
}

/// How the graph folds its lanes: the session's limit is filled in by
/// [`super::super::view`], the expansion flag is the user's, and [`keep_for`]
/// weighs the reserved integration lanes (`1..=n`, after lane 0), the
/// cursor's own column and the selection's thread ahead of everything else
/// (ADR-0025) — a priority a fold still overrides once the budget runs out,
/// not a veto.
/// Is there a divider on screen for `<>` to move?
///
/// The pane has to be **open** and in a **split**: under 100 columns it is a
/// tab strip with nothing between the two tabs to drag, so `<>` is a key
/// that does nothing and the footer must not offer it. `view::frame` already
/// passes `Divider::Hidden` for the Detail tab's own bar and says why; this
/// is the same answer for the Graph tab, which keeps the graph screen's
/// footer (m5-history-and-ghost-preview/33, review M6's width sweep).
pub(crate) fn divider_shown(state: &AppState) -> bool {
    state.detail.shown
        && !super::super::frame::tabbed(state, super::super::frame::screen_area(state))
}

/// The lane cap this frame folds at: `graph.max_lanes` if the config file
/// set one, else the width breakpoint — resolved against the room the graph
/// has, not the whole screen (ADR-0039's amendment,
/// [`super::super::frame::graph_columns`]).
///
/// The room is read off `AppState::size` rather than taken as an argument on
/// purpose: the hit-test has no `Frame` to ask, and a cap the drawing path
/// could pass a different width to is the very divergence this function
/// exists to close. `event_loop::graphics` already reads the screen
/// rectangle the same way.
pub fn max_lanes(state: &AppState) -> usize {
    mergewright_ui::graph_screen::max_lanes_for(
        super::super::frame::graph_columns(state, super::super::frame::screen_area(state)),
        state.max_lanes,
    )
}

pub(super) fn compaction(
    state: &AppState,
    history: &History,
    thread: Option<&Thread>,
    rows: &[GraphEntry<'_>],
) -> Compaction {
    let integration = history
        .walk
        .as_ref()
        .map_or(0, |walk| walk.integration.len());
    let reserved: BTreeSet<usize> = (1..=integration).collect();
    let cursor = rows
        .iter()
        .position(|entry| entry.line.paint == RowPaint::Highlighted);
    let drawn: Vec<Row> = rows
        .iter()
        .map(|entry| entry.line.row.as_ref())
        .cloned()
        .collect();
    Compaction {
        max_lanes: max_lanes(state),
        expanded: state.lanes_expanded,
        keep: keep_for(
            &drawn,
            cursor,
            &reserved,
            thread.map(|thread| &thread.ancestry),
        ),
    }
}
