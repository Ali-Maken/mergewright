//! Geometry: where the header, the graph column and the trailing columns
//! land, and how many lanes fit at a given width.

use graft_graph::fold::Fold;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::{
    AUTHOR_BREAKPOINT, AUTHOR_WIDTH, CHROME_ROWS, Filters, GraphEntry, GraphScreen, GraphicsStatus,
    MARKER_WIDTH, OrderNotice, SHA_WIDTH, SUBJECT_GAP, WHEN_WIDTH, WalkView,
};
use crate::fit::fit;
use crate::graph_column::{CELLS_PER_LANE, GraphColumn};
use crate::paint::foreground;
use crate::theme::Token;

impl<'a> GraphScreen<'a> {
    /// Rows the banner takes under the header: one per line. Saturates, so a
    /// banner taller than any terminal still leaves the arithmetic sound.
    fn banner_rows(&self) -> u16 {
        u16::try_from(self.view.banner.len()).unwrap_or(u16::MAX)
    }

    /// Where the graph column lands inside `area`, the whole screen: the
    /// rectangle the pixel tier places its images in, every body row tall.
    #[must_use]
    pub fn graph_area(&self, area: Rect) -> Rect {
        let (body, rows, applied) = self.body_layout(area);
        Rect {
            width: graph_width(body, rows, applied.as_ref()),
            ..body
        }
    }

    /// The rows' rectangle, the rows that fit in it, and the fold they are
    /// drawn through (`None` while expanded).
    pub(super) fn body_layout(&self, area: Rect) -> (Rect, &[GraphEntry<'a>], Option<Fold>) {
        let banner = self.banner_rows();
        let offer = u16::from(self.view.commit_graph.is_some());
        let body = Rect {
            y: area.y.saturating_add(1).saturating_add(banner),
            height: area
                .height
                .saturating_sub(CHROME_ROWS.saturating_add(banner).saturating_add(offer)),
            ..area
        };
        let rows = &self.view.rows[..self.view.rows.len().min(usize::from(body.height))];
        let fold = self
            .view
            .compaction
            .plan(rows.iter().map(|entry| &*entry.line.row));
        let applied = (!self.view.compaction.expanded).then_some(fold);
        (body, rows, applied)
    }

    /// The header line; `folded_lanes` is how many lanes the drawn fold
    /// hides behind the rail (0 when nothing folds or lanes are expanded).
    pub(super) fn header(&self, width: usize, folded_lanes: usize) -> Line<'static> {
        let muted = foreground(self.theme, Token::TextMuted);
        let brand = " graft ".len().min(width);
        let mut spans = vec![Span::styled(
            fit(" graft ", brand),
            foreground(self.theme, Token::Accent),
        )];
        let mut used = brand;
        let mut push = |text: String, style: Style, spans: &mut Vec<Span<'static>>| {
            let room = width.saturating_sub(used);
            if room == 0 {
                return;
            }
            let text = fit(&text, room.min(text_width(&text)));
            used += text_width(&text);
            spans.push(Span::styled(text, style));
        };
        if let Some(location) = &self.view.location {
            push(
                format!("{location}  "),
                foreground(self.theme, Token::Text),
                &mut spans,
            );
        }
        let (walk, token) = walk_text(&self.view.walk);
        push(walk, foreground(self.theme, token), &mut spans);
        let filters = left_out(self.view.filters, folded_lanes);
        if !filters.is_empty() {
            push(
                format!("  {filters}"),
                foreground(self.theme, Token::Accent),
                &mut spans,
            );
        }
        if matches!(self.view.walk, WalkView::Jumping { .. }) {
            push("  Esc cancels".to_owned(), muted, &mut spans);
        }
        match self.view.order {
            Some(OrderNotice::Sorting) => {
                push("  date order, sorting…".to_owned(), muted, &mut spans);
            }
            Some(OrderNotice::Provisional) => push(
                "  order stays provisional (date)".to_owned(),
                foreground(self.theme, Token::Warning),
                &mut spans,
            ),
            None => {}
        }
        match &self.view.graphics {
            Some(GraphicsStatus::Ok) => push("  gfx ok".to_owned(), muted, &mut spans),
            Some(GraphicsStatus::Failed(message)) => push(
                format!("  gfx: {message}"),
                foreground(self.theme, Token::Warning),
                &mut spans,
            ),
            None => {}
        }
        Line::from(spans)
    }
}

/// The header's account of what the drawn fold and the filters leave out:
/// the filters, then `+N lanes` while the drawn fold hides any. The count is
/// the screen's to say — it planned the fold — and shows only while
/// something folds, never on a row (ADR-0022).
fn left_out(filters: Filters, folded_lanes: usize) -> String {
    let mut words = filters.text();
    if folded_lanes > 0 {
        if !words.is_empty() {
            words.push_str(" · ");
        }
        words.push_str(&format!("+{folded_lanes} lanes"));
    }
    words
}

/// Cells the graph column can take at most, at this screen width: half the
/// width — the subject is what makes a row readable. Shared with
/// [`lanes_that_fit`] so a fold's cap and the column's actual bound cannot
/// drift apart (ADR-0025 decision 3).
fn column_budget(columns: u16) -> u16 {
    columns / 2
}

/// Cells the graph column takes in the rows' `area`: what its lanes want,
/// half the width at most — the subject is what makes a row readable.
pub(super) fn graph_width(area: Rect, rows: &[GraphEntry<'_>], fold: Option<&Fold>) -> u16 {
    let full_width = || {
        let lanes = rows
            .iter()
            .map(|entry| entry.line.row.lanes.len().max(entry.line.row.lane + 1))
            .max()
            .unwrap_or(1);
        GraphColumn::width_for(lanes)
    };
    fold.map_or_else(full_width, GraphColumn::width_through)
        .min(column_budget(area.width))
}

/// The most lane columns, the rail included, the graph column can hold at
/// `columns` screen width — never below two (lane 0 and the rail). Uses the
/// same half-screen bound `graph_width` draws to, and the same cell cost per
/// lane, so a fold planned to this cap never gets cut by the column it is
/// drawn into (ADR-0025 decision 3).
#[must_use]
pub fn lanes_that_fit(columns: u16) -> usize {
    let lanes = column_budget(columns) / CELLS_PER_LANE;
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

/// The header's account of the walk, and the token it is drawn in.
pub(super) fn walk_text(walk: &WalkView) -> (String, Token) {
    match walk {
        WalkView::Opening => ("opening…".to_owned(), Token::TextMuted),
        WalkView::Reading { laid_out: 0 } => ("reading history…".to_owned(), Token::TextMuted),
        WalkView::Reading { laid_out } => (format!("{laid_out} rows so far"), Token::TextMuted),
        WalkView::Jumping { laid_out } => (format!("laying out… {laid_out} rows"), Token::Accent),
        WalkView::Complete { rows: 0 } => ("no commits yet".to_owned(), Token::TextMuted),
        WalkView::Complete { rows: 1 } => ("1 commit".to_owned(), Token::TextMuted),
        WalkView::Complete { rows } => (format!("{rows} commits"), Token::TextMuted),
        WalkView::Cancelled { laid_out } => {
            (format!("stopped at {laid_out} rows"), Token::TextMuted)
        }
        WalkView::Failed(reason) => (format!("history failed: {reason}"), Token::Warning),
    }
}

pub(super) fn text_width(text: &str) -> usize {
    text.width()
}

/// Where each trailing column sits and how wide it is, once the graph and
/// the subject have taken their share.
///
/// Computed right to left from `area.right()`, each column clamped by
/// `min`/`saturating_sub` against what is left: at any width, down to zero,
/// a column simply stops taking space rather than the arithmetic
/// underflowing. The author chip is the one column that can drop entirely,
/// below [`AUTHOR_BREAKPOINT`] (DESIGN.md §1.2/§1.3); the rest only narrow.
pub(super) struct Columns {
    pub(super) marker_x: u16,
    pub(super) marker_width: u16,
    pub(super) subject_x: u16,
    pub(super) subject_width: u16,
    pub(super) author_x: u16,
    pub(super) author_width: u16,
    pub(super) when_x: u16,
    pub(super) when_width: u16,
    pub(super) sha_x: u16,
    pub(super) sha_width: u16,
}

impl Columns {
    pub(super) fn compute(area: Rect, graph_width: u16) -> Self {
        let marker_x = area.x + graph_width.min(area.width);
        let marker_width = MARKER_WIDTH.min(area.width.saturating_sub(graph_width));
        let content_x = marker_x + marker_width;

        // Right to left: the rightmost column claims space first, so a
        // narrow terminal drops the leftmost of these (the author chip)
        // before it ever eats into the SHA or the date.
        let mut right = area.right();
        let sha_width = SHA_WIDTH.min(right.saturating_sub(content_x));
        right -= sha_width;
        let sha_x = right;

        let when_width = WHEN_WIDTH.min(right.saturating_sub(content_x));
        right -= when_width;
        let when_x = right;

        let show_author = area.width >= AUTHOR_BREAKPOINT;
        let author_width = if show_author {
            AUTHOR_WIDTH.min(right.saturating_sub(content_x))
        } else {
            0
        };
        right -= author_width;
        let author_x = right;

        let gap_width = if sha_width + when_width + author_width > 0 {
            SUBJECT_GAP.min(right.saturating_sub(content_x))
        } else {
            0
        };
        right -= gap_width;

        Columns {
            marker_x,
            marker_width,
            subject_x: content_x,
            subject_width: right.saturating_sub(content_x),
            author_x,
            author_width,
            when_x,
            when_width,
            sha_x,
            sha_width,
        }
    }
}
