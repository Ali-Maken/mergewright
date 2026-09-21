//! Which drawn thing a screen cell belongs to.
//!
//! The graph's rows are packed by `graft_ui`, and where a badge lands is a
//! function of the lane count, the pane width and the badges themselves —
//! not something `update` can work out from a row number. So this half of
//! the hit-test lives here, where the view model and the pane rectangle both
//! are, and it asks `graft_ui` the same two questions the renderer asks:
//! `row_at` for the entry and its columns, `badge_spans` for where each
//! badge's cells are. **Not a second copy of the packing** — the same
//! functions, called again, with `view::tests::badge_spans_match_the_drawn_cells`
//! drawing a row and reading the cells back to prove the two agree
//! (ADR-0040's lesson, applied before it could be re-learned).
//!
//! The *row* half stays in `update::geometry`, which already owns it: this
//! only answers "which badge on that row".

use graft_ui::graph_screen::{badge_spans, row_at};
use ratatui::layout::Rect;

use crate::update::AppState;

use super::frame::primary_pane_area;
use super::rows::graph_view;

/// The badge drawn at `(column, row)`, as its position in the row's badge
/// list — which is also its position in `RefBadges::on(oid)`, because
/// [`super::entries::row_entry`] maps that slice one for one.
///
/// `None` off a badge, including the gap between two of them: the separator
/// is drawn by the row, not owned by either neighbour, so a drag started
/// there picks up nothing rather than guessing.
#[must_use]
pub(crate) fn badge_index_at(
    state: &AppState,
    column: u16,
    row: u16,
    now_seconds: i64,
) -> Option<usize> {
    let area = Rect {
        x: 0,
        y: 0,
        width: state.size.columns,
        height: state.size.rows,
    };
    let pane = primary_pane_area(state, area)?;
    let view = graph_view(state, now_seconds);
    let (entry, columns) = row_at(&view, pane, row)?;
    badge_spans(entry, &columns, view.icons)
        .into_iter()
        .find_map(|(span, index)| span.contains(&column).then_some(index))
}
