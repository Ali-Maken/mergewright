//! Which drawn thing a screen cell belongs to.
//!
//! The graph's rows are packed by `mergewright_ui`, and where a badge lands is a
//! function of the lane count, the pane width and the badges themselves —
//! not something `update` can work out from a row number. So this half of
//! the hit-test lives here, where the view model and the pane rectangle both
//! are, and it asks `mergewright_ui` the same two questions the renderer asks:
//! `row_at` for the entry and its columns, `ref_spans` for where each
//! label's cells are. **Not a second copy of the packing** — the same
//! functions, called again, with
//! `mergewright_ui::graph_screen::tests::ref_spans_match_the_drawn_cells`
//! drawing a row and reading the cells back to prove the two agree
//! (ADR-0040's lesson, applied before it could be re-learned).
//!
//! And **not a second copy of the view either**: the `GraphView` here is the
//! one `view::rows::graph_view` hands the frame, glyph set, lane cap and
//! banner included. The frame used to override the first two from the
//! `Session` *after* building it, which put the drawn cells and the measured
//! cells on different geometry — with Nerd icons, or past eight lanes, a
//! drag landed on the wrong badge (m5-history-and-ghost-preview/33, review
//! M1). `view::tests::a_drag_lands_on_the_badge_the_frame_drew_under_the_pointer`
//! draws a real frame and presses the cells it wrote.
//!
//! The *row* half stays in `update::geometry`, which already owns it: this
//! only answers "which badge on that row".

use mergewright_ui::graph_screen::{ref_spans, row_at};

use crate::update::AppState;

use super::frame::{primary_pane_area, screen_area};
use super::rows::graph_view;

/// The badge drawn at `(column, row)`, as its position in the row's badge
/// list — which is also its position in `RefBadges::on(oid)`, because
/// [`super::entries::row_entry`] maps that slice one for one (its first
/// `badges_that_fit` labels: past them nothing is drawn, so nothing is hit).
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
    let pane = primary_pane_area(state, screen_area(state))?;
    // The frame's own view, from the frame's own builder — not a copy of it
    // with the two session overrides re-applied. See `view::rows::graph_view`.
    let view = graph_view(state, now_seconds);
    let (entry, columns) = row_at(&view, pane, row)?;
    ref_spans(entry, &columns, view.icons, view.keys, view.ascii)
        .into_iter()
        .find_map(|(span, index)| span.contains(&column).then_some(index))
}
