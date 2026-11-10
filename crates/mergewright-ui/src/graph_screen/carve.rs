//! The graph screen's row carve in plain numbers (m8-polish-and-ship/40):
//! the one body [`super::screen_rows`] wraps, and the one `mergewright`'s
//! hit-tests read, so the rows a click lands on are the rows drawn.

use ratatui::layout::Rect;

use super::GraphView;
use super::hints::StatusRow;
use super::layout::{body_width, inset};
use crate::{Cells, cells_of, rect_of};

/// Rows [`GraphView::banner`](super::GraphView::banner) takes: one per
/// harness-pinned line. Saturates, so a banner taller than any terminal
/// still leaves the arithmetic sound.
///
/// The operation banner adds none. It had a `[ Continue ] [ Skip ]
/// [ Abort ]` row here at 160 columns and up — a row of mouse targets
/// nothing answers a click on (`banner_click` was never built) and that the
/// design note's own mockup does not have; the verbs it drew are the hint
/// bar's, which works at every width and on the keyboard the user already
/// has. Removed rather than made clickable: a row that costs a graph row to
/// teach three keys twice is not worth the row (m5/16, review C1).
fn banner_rows(view: &GraphView<'_>) -> u16 {
    u16::try_from(view.banner.len()).unwrap_or(u16::MAX)
}

/// Everything [`row_carve`] reads of a [`GraphView`], as plain values, so a
/// caller that cannot build a view (`mergewright::update`, which hit-tests clicks
/// and must not name the rendering library) feeds the same carve.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CarveInputs {
    /// [`GraphView::spaced`]: the blank row and the rule are wanted.
    pub spaced: bool,
    /// Rows [`GraphView::banner`] asks for under the header.
    pub banner_rows: u16,
    /// [`GraphView::commit_graph`] is `Some`: the offer wants its row.
    pub commit_graph: bool,
    /// [`GraphView::tour`] is `Some`: the tour card wants its rows.
    pub tour: bool,
    /// Rows the still-to-apply block asks for
    /// ([`crate::still_to_apply::still_to_apply_height`] of
    /// [`GraphView::pending`]); 0 when nothing is pending.
    pub pending_rows: u16,
    /// [`GraphView::status_row`]: [`StatusRow::AboveBar`] wants the done
    /// line's own row above the bar (ease-pass/08s M4).
    pub status_row: StatusRow,
}

impl CarveInputs {
    /// What `view` asks of the carve.
    #[must_use]
    pub fn of(view: &GraphView<'_>) -> Self {
        Self {
            status_row: view.status_row,
            spaced: view.spaced,
            banner_rows: banner_rows(view),
            commit_graph: view.commit_graph.is_some(),
            tour: view.tour.is_some(),
            pending_rows: view
                .pending
                .as_ref()
                .map_or(0, crate::still_to_apply::still_to_apply_height),
        }
    }
}

/// The screen's rows in plain numbers: [`ScreenRows`](super::ScreenRows)
/// with every `Rect` a [`Cells`]. Same bands, same meaning, same
/// zero-height rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowCarve {
    /// [`ScreenRows::header`](super::ScreenRows::header).
    pub header: Cells,
    /// [`ScreenRows::gap`](super::ScreenRows::gap).
    pub gap: Cells,
    /// [`ScreenRows::banner`](super::ScreenRows::banner).
    pub banner: Cells,
    /// [`ScreenRows::tour`](super::ScreenRows::tour).
    pub tour: Cells,
    /// [`ScreenRows::pending`](super::ScreenRows::pending).
    pub pending: Cells,
    /// [`ScreenRows::body`](super::ScreenRows::body).
    pub body: Cells,
    /// [`ScreenRows::rule`](super::ScreenRows::rule).
    pub rule: Cells,
    /// [`ScreenRows::offer`](super::ScreenRows::offer).
    pub offer: Cells,
    /// [`ScreenRows::status`](super::ScreenRows::status).
    pub status: Cells,
    /// [`ScreenRows::footer`](super::ScreenRows::footer).
    pub footer: Cells,
}

/// The one carve of the graph screen's rows: `area` cut into the bands of
/// [`RowCarve`] for what `inputs` asks. [`screen_rows`](super::screen_rows) is this over a
/// [`GraphView`]; `mergewright`'s hit-tests read it over plain state.
#[must_use]
pub fn row_carve(area: Cells, inputs: CarveInputs) -> RowCarve {
    let area = rect_of(area);
    let header = Rect {
        height: 1.min(area.height),
        ..area
    };
    // Bottom up. The hint bar always has the last row; the offer takes
    // the one above it when a row is going spare; the banner takes what
    // is left between the header and them. The banner outranks the offer
    // because it is modal and the offer is a suggestion, so where only
    // one row is free it is the offer that goes without.
    let footer = Rect {
        y: area.bottom().saturating_sub(1),
        height: u16::from(area.height > 1),
        ..area
    };
    // The blank row and the rule (ADR-0058 §4) are only drawn where the
    // terminal is tall enough that the list can spare them, so they are
    // carved before anything that could compete for a row.
    let spaced = inputs.spaced && area.height >= 4;
    let gap = Rect {
        y: header.bottom(),
        height: u16::from(spaced),
        ..area
    };
    let top = gap.bottom().min(footer.y);
    let free = footer
        .y
        .saturating_sub(top)
        .saturating_sub(u16::from(spaced));
    let banner = Rect {
        y: top,
        height: inputs.banner_rows.min(free),
        ..area
    };
    // The done line's own row, directly above the bar (ease-pass/08s M4),
    // when a row is going spare under the banner; the offer above it.
    let spare = free.saturating_sub(banner.height);
    let status = Rect {
        y: footer.y.saturating_sub(1),
        height: inputs.status_row.rows_taken().min(spare),
        ..area
    };
    // Each of the two is a row or none: what stands above it starts that
    // many rows higher.
    let rule_floor = footer.y.saturating_sub(status.height);
    let offer = Rect {
        y: rule_floor.saturating_sub(1),
        height: u16::from(inputs.commit_graph && free > banner.height + status.height),
        ..area
    };
    let above_offer = rule_floor.saturating_sub(offer.height);
    let rule = Rect {
        y: above_offer.saturating_sub(1),
        height: u16::from(spaced),
        ..area
    };
    let body_floor = if spaced { rule.y } else { above_offer };
    let tour = tour_rect(inputs.tour, banner.bottom(), body_floor, area);
    let pending = pending_rect(inputs.pending_rows, tour.bottom(), body_floor, area);
    let body = body_rect(area, pending.bottom(), body_floor);
    RowCarve {
        header: cells_of(header),
        gap: cells_of(gap),
        banner: cells_of(banner),
        tour: cells_of(tour),
        pending: cells_of(pending),
        body: cells_of(body),
        rule: cells_of(rule),
        offer: cells_of(offer),
        status: cells_of(status),
        footer: cells_of(footer),
    }
}

/// The graph rows, from `top` down to `floor`. The gutter and the margin
/// come off only where the row can afford them: a screen a few cells wide
/// keeps what it has.
fn body_rect(area: Rect, top: u16, floor: u16) -> Rect {
    Rect {
        x: area.x + inset(area.width).0,
        y: top,
        width: body_width(area.width),
        height: floor.saturating_sub(top),
    }
}

/// The tour card's rows: from `top` (under the banner), as many as
/// [`tour_rows`] gives out of what is left above `floor`.
fn tour_rect(on: bool, top: u16, floor: u16, area: Rect) -> Rect {
    Rect {
        y: top,
        height: tour_rows(on, floor.saturating_sub(top)),
        ..area
    }
}

/// Rows the tour card takes out of `left` (the rows between the banner and
/// the body's floor): all three, or none when that would leave the graph
/// no row of its own.
fn tour_rows(on: bool, left: u16) -> u16 {
    let wanted = if on {
        crate::tour_card::TOUR_CARD_ROWS
    } else {
        0
    };
    if left > wanted { wanted } else { 0 }
}

/// The "still to apply" block under the banner: at most half the rows
/// between `top` and `floor`, and none at all when that leaves it under the
/// three rows it needs to say anything. `wanted` is the block's own height.
fn pending_rect(wanted: u16, top: u16, floor: u16, area: Rect) -> Rect {
    let left = floor.saturating_sub(top);
    let height = wanted.min(left / 2);
    Rect {
        y: top,
        height: if height >= 3 { height } else { 0 },
        ..area
    }
}
