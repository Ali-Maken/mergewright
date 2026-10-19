//! `update` tests: the branch pane's geometry mirror (ease-pass 09c check
//! 3). `update::branch_pane::pane_holds` answers a click without a graph
//! view, and `view::frame::graph_columns` sizes the lanes without one; both
//! are checked here against the carve the drawing path reads
//! (`mergewright_ui::graph_screen::screen_rows`), not against a copy of it,
//! as `tab_strip_geometry`'s mirror test does for the detail split.

use super::*;
use ratatui::layout::Rect;

const HEIGHT: u16 = 30;

/// A walk on screen with the cursor on a row, at `columns`, the detail pane
/// open or closed.
fn at(columns: u16, detail: bool) -> AppState {
    let (mut state, _) = merge_on_screen(JobId::new(5), 0);
    state.size = crate::update::state::Size {
        columns,
        rows: HEIGHT,
    };
    state.detail.shown = detail;
    state
}

/// The graph's own area: the whole screen, or the detail split's graph rect.
fn graph_area(state: &AppState) -> Option<Rect> {
    let screen = crate::view::frame::screen_area(state);
    if !state.detail.shown {
        return Some(screen);
    }
    let layout = crate::update::detail::effective_layout(
        state.detail.focus,
        &state.detail.pane_size,
        (screen.width, screen.height),
    );
    crate::view::frame::detail_rects(state, layout, screen).0
}

fn mirror_agrees(columns: u16, detail: bool) -> Rect {
    let state = at(columns, detail);
    let area = graph_area(&state).expect("a graph is drawn");
    let view = crate::view::graph_view(&state, 0);
    let rows = mergewright_ui::graph_screen::screen_rows(&view, area);
    let at = format!("{columns}x{HEIGHT}, detail={detail}");
    for row in 0..HEIGHT {
        for column in 0..columns {
            let drawn = rows.pane.width > 0
                && column >= rows.pane.x
                && column < rows.pane.right()
                && row >= rows.pane.y
                && row < rows.pane.bottom();
            assert_eq!(
                crate::update::branch_pane::pane_holds(&state, column, row),
                drawn,
                "cell ({column}, {row}) at {at}; pane {:?}",
                rows.pane
            );
        }
    }
    // The rows' share of the graph's area: everything left of the pane's
    // slot, gutter and margin included (`body` is inside those; the lanes
    // and strips are laid out from this whole share).
    let share = if rows.pane.width > 0 {
        rows.pane.x - area.x
    } else {
        area.width
    };
    let screen = crate::view::frame::screen_area(&state);
    assert_eq!(
        crate::view::frame::graph_columns(&state, screen),
        share,
        "graph columns at {at}"
    );
    rows.pane
}

#[test]
fn the_branch_pane_mirror_matches_the_carve_the_frame_draws() {
    for columns in [90u16, 110, 120] {
        let open = mirror_agrees(columns, false);
        assert_eq!(open.width > 0, columns > 90, "pane drawn at {columns}");
        let beside_detail = mirror_agrees(columns, true);
        assert_eq!(
            beside_detail.width, 0,
            "no branch pane beside the detail pane at {columns}"
        );
    }
}
