//! The row columns beside the branch pane (ease-pass 09, New UI): the pane
//! takes its slot off the rows, but the branch column keeps the width the
//! *screen's* width gives it — 20 from 120, 14 from 100 — not the width the
//! narrowed rows would.

use ratatui::layout::Rect;

use super::super::layout::{row_layout, screen_rows};
use super::super::{BRANCH_MIDDLE, BRANCH_NARROW, BRANCH_WIDE, WalkView};
use super::{linear, view};
use crate::branch_pane::BranchPaneView;
use mergewright_term::sanitize_line;

fn with_pane(rows: &[mergewright_graph::row::Row]) -> super::super::GraphView<'_> {
    let mut view = view(rows, WalkView::Complete { rows: 3 }, 0);
    view.pane = Some(BranchPaneView::NotOnBranch {
        sha7: sanitize_line("abc1234"),
    });
    view
}

#[test]
fn the_branch_column_keeps_the_screens_width_beside_the_pane() {
    let rows = linear(3);
    let view = with_pane(&rows);
    for (columns, slot, branch) in [
        (120, 38, BRANCH_WIDE),
        (187, 38, BRANCH_WIDE),
        (110, 32, BRANCH_MIDDLE),
        (100, 32, BRANCH_MIDDLE),
        (90, 0, BRANCH_NARROW),
    ] {
        let area = Rect::new(0, 0, columns, 20);
        assert_eq!(screen_rows(&view, area).pane.width, slot, "{columns}: slot");
        assert_eq!(
            row_layout(&view, area).columns.branch_width,
            branch,
            "{columns}: branch column"
        );
    }
}
