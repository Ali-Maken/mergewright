//! How wide the graph column is and how many lanes it holds at a width —
//! split out of `layout` for size (ease-pass/08s M6).

use mergewright_graph::fold::Fold;
use ratatui::layout::Rect;

use super::layout::body_width;
use crate::graph_column::{CELLS_PER_LANE, GraphColumn};

/// Cells the graph column can take at most, at this screen width: half the
/// width — the subject is what makes a row readable. Shared with
/// [`lanes_that_fit`] so a fold's cap and the column's actual bound cannot
/// drift apart (ADR-0025 decision 3).
fn column_budget(columns: u16) -> u16 {
    columns / 2
}

/// Cells the graph column takes in the rows' `area`: what the lanes drawn
/// through `fold` want, half the width at most — the subject is what makes
/// a row readable.
pub(super) fn graph_width(area: Rect, fold: &Fold) -> u16 {
    GraphColumn::width_through(fold).min(column_budget(area.width))
}

/// The most lane columns, the rail included, the graph column can hold at
/// `columns` screen width — never below two (lane 0 and the rail). Uses the
/// same half-screen bound `graph_width` draws to, and the same cell cost per
/// lane, so a fold planned to this cap never gets cut by the column it is
/// drawn into (ADR-0025 decision 3).
/// `columns` is the screen's (or the graph pane's) width; the gutter and
/// the margin come off it first, as they do for the rows.
#[must_use]
pub fn lanes_that_fit(columns: u16) -> usize {
    let lanes = column_budget(body_width(columns)) / CELLS_PER_LANE;
    usize::from(lanes).max(2)
}

/// `graph.max_lanes` under 100 columns (DESIGN.md §1.1's breakpoint).
const NARROW_LANES: usize = 8;
/// `graph.max_lanes` at 100–159 columns (DESIGN.md §1.2's breakpoint).
const MEDIUM_LANES: usize = 10;
/// `graph.max_lanes` at 160 columns and above (DESIGN.md §1.3's breakpoint).
/// The ceiling: the lane palette has 12 hues, so more lanes add width
/// without adding a colour to tell them apart by (ADR-0022 decision 3).
const WIDE_LANES: usize = 12;

/// The lane cap for this frame: `configured` if the config file set one,
/// else the width breakpoint (ADR-0022 decision 3); then clamped to what the
/// graph column can actually hold at `columns` (ADR-0025 decision 3), so a
/// configured number larger than the screen never cuts the rail.
#[must_use]
pub fn max_lanes_for(columns: u16, configured: Option<usize>) -> usize {
    configured
        .unwrap_or(breakpoint_lanes(columns))
        .min(lanes_that_fit(columns))
}

/// The width breakpoint's lane count: `NARROW_LANES` under 100 columns,
/// `MEDIUM_LANES` at 100–159, `WIDE_LANES` at 160 and above.
fn breakpoint_lanes(columns: u16) -> usize {
    if columns < 100 {
        NARROW_LANES
    } else if columns < 160 {
        MEDIUM_LANES
    } else {
        WIDE_LANES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // m1-the-graph/44: the lane cap follows the terminal's width (ADR-0022
    // decision 3, ADR-0025 decision 3).

    #[test]
    fn lanes_that_fit_matches_graph_widths_half_screen_bound() {
        // Half of 8 columns is 4 cells, one lane at 2 cells each — but the rail
        // always fits alongside lane 0, so the floor is 2, not 1.
        assert_eq!(lanes_that_fit(8), 2);
        // 80 columns less the gutter and margin is 77; half of that is 38
        // cells, 19 lanes at 2 cells each (ADR-0058 §4).
        assert_eq!(lanes_that_fit(80), 19);
    }

    #[test]
    fn max_lanes_for_follows_the_width_breakpoints_when_unconfigured() {
        assert_eq!(max_lanes_for(79, None), 8);
        assert_eq!(max_lanes_for(100, None), 10);
        assert_eq!(max_lanes_for(159, None), 10);
        assert_eq!(max_lanes_for(160, None), 12);
        assert_eq!(max_lanes_for(200, Some(5)), 5);
    }

    #[test]
    fn max_lanes_for_clamps_a_configured_value_to_what_the_screen_holds() {
        assert_eq!(max_lanes_for(80, Some(30)), lanes_that_fit(80));
        assert_eq!(max_lanes_for(80, Some(30)), 19);
    }
}
