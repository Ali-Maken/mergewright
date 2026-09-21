//! Geometry: where the header, the graph column and the trailing columns
//! land, and how many lanes fit at a given width.

use graft_graph::fold::Fold;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::{
    AUTHOR_BREAKPOINT, AUTHOR_WIDTH, Filters, GraphEntry, GraphScreen, GraphicsStatus,
    MARKER_WIDTH, OrderNotice, SHA_WIDTH, SUBJECT_GAP, WHEN_WIDTH, WalkView, banner_line,
};
use crate::fit::fit;
use crate::graph_column::{CELLS_PER_LANE, GraphColumn};
use crate::paint::foreground;
use crate::theme::Token;

/// The screen's rows, carved once by [`GraphScreen::screen_rows`]: one
/// rectangle per part of the screen that owns rows of its own, none of which
/// ever overlaps another.
///
/// A zero-height rectangle means that part is not drawn at this size — the
/// hint bar on a one-row screen, the commit-graph offer with no row to spare,
/// the banner when there is none. Callers that draw from these skip the empty
/// ones rather than special-casing each size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenRows {
    /// The header's row: the brand, the location, the walk or the operation
    /// banner.
    pub header: Rect,
    /// The rows [`GraphView::banner`](super::GraphView::banner) takes under
    /// the header, clipped to what is free above the rest of the chrome.
    pub banner: Rect,
    /// The rows the commit rows are drawn in.
    pub body: Rect,
    /// The commit-graph offer's row, directly above the hint bar.
    pub offer: Rect,
    /// The hint bar's row: the screen's last.
    pub footer: Rect,
}

impl<'a> GraphScreen<'a> {
    /// Rows the banner takes under the header: one per harness-pinned line.
    /// Saturates, so a banner taller than any terminal still leaves the
    /// arithmetic sound.
    ///
    /// The operation banner adds none. It had a `[ Continue ] [ Skip ]
    /// [ Abort ]` row here at 160 columns and up — a row of mouse targets
    /// nothing answers a click on (`banner_click` was never built) and that
    /// the design note's own mockup does not have; the verbs it drew are the
    /// hint bar's, which works at every width and on the keyboard the user
    /// already has. Removed rather than made clickable: a row that costs a
    /// graph row to teach three keys twice is not worth the row
    /// (m5/16, review C1).
    fn banner_rows(&self) -> u16 {
        u16::try_from(self.view.banner.len()).unwrap_or(u16::MAX)
    }

    /// Every row of the screen, carved once: which row the header takes,
    /// which rows the banner takes, which the body has left, and which the
    /// commit-graph offer and the hint bar sit on. A zero-height rectangle
    /// means that part is not on screen at this size.
    ///
    /// One carve, so no two parts of the draw can claim the same row. They
    /// used to work it out separately, each from `area.height` and its own
    /// idea of what else was on screen, and on a screen with a single row to
    /// spare all three of the banner, the offer and the footer claimed it —
    /// the offer and then the footer drew over the banner, which is the one
    /// of the three that is modal (m5/19).
    #[must_use]
    pub fn screen_rows(&self, area: Rect) -> ScreenRows {
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
        let top = header.bottom().min(footer.y);
        let free = footer.y.saturating_sub(top);
        let banner = Rect {
            y: top,
            height: self.banner_rows().min(free),
            ..area
        };
        let offer = Rect {
            y: footer.y.saturating_sub(1),
            height: u16::from(self.view.commit_graph.is_some() && free > banner.height),
            ..area
        };
        let body_floor = if offer.height > 0 { offer.y } else { footer.y };
        let body = Rect {
            y: banner.bottom(),
            height: body_floor.saturating_sub(banner.bottom()),
            ..area
        };
        ScreenRows {
            header,
            banner,
            body,
            offer,
            footer,
        }
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

    /// Every x at which one row column ends and the next begins in `area`,
    /// ascending and deduped, `area`'s own two edges included.
    ///
    /// The geometry the rows are actually drawn to ([`Columns::compute`]),
    /// handed out as plain data. A caller laying an overlay over the graph
    /// needs it to keep that overlay's edges out of the middle of a column
    /// (ADR-0040); it is given as numbers rather than as a "where should
    /// this box go" call so that the overlay widgets stay ignorant of the
    /// graph — see [`snap_to_columns`], which is the whole of the rule and
    /// knows nothing about either.
    ///
    /// Empty when `area` is, which reads as "no columns here to split".
    #[must_use]
    pub fn column_boundaries(&self, area: Rect) -> Vec<u16> {
        if area.is_empty() {
            return Vec::new();
        }
        let (body, rows, applied) = self.body_layout(area);
        Columns::compute(body, graph_width(body, rows, applied.as_ref())).boundaries(body)
    }

    /// The rows' rectangle, the rows that fit in it, and the fold they are
    /// drawn through (`None` while expanded).
    pub(super) fn body_layout(&self, area: Rect) -> (Rect, &[GraphEntry<'a>], Option<Fold>) {
        let body = self.screen_rows(area).body;
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

        // The operation banner takes the rest of the header — the walk
        // state, filters, order notice and graphics status all answer "is
        // history still loading", which is not the question mid-operation
        // (DESIGN.md §1.13, ADR-0038 §3). `focused` reverses the slot so a
        // conflict landing is noticed without a click.
        //
        // Its room comes out **before** the location's. The location used to
        // take what it liked and hand the banner the remainder, so a long
        // repository path (every capture's scratch repo, and plenty of real
        // checkouts) cut the banner to `MERG…` — the state gone, and the
        // banner is the urgent half. A path cut with an `…` still says
        // where you are (m5/16, review C1).
        let banner = self.view.banner_op.as_ref().map(|banner| {
            let text = format!("  {}", banner_line(banner));
            let mut style = foreground(self.theme, Token::Warning);
            if banner.focused {
                style = style.add_modifier(ratatui::style::Modifier::REVERSED);
            }
            (text, style)
        });
        let reserved = banner
            .as_ref()
            .map_or(0, |(text, _)| text_width(text))
            .min(width.saturating_sub(used));

        let mut push =
            |text: String, style: Style, reserved: usize, spans: &mut Vec<Span<'static>>| {
                let room = width.saturating_sub(used).saturating_sub(reserved);
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
                reserved,
                &mut spans,
            );
        }
        if let Some((text, style)) = banner {
            push(text, style, 0, &mut spans);
            return Line::from(spans);
        }
        self.push_walk_status(folded_lanes, muted, &mut push, &mut spans);
        Line::from(spans)
    }

    /// The walk state, filters, order notice and graphics status — the
    /// header's informational tail when no operation banner takes its place.
    fn push_walk_status(
        &self,
        folded_lanes: usize,
        muted: Style,
        push: &mut impl FnMut(String, Style, usize, &mut Vec<Span<'static>>),
        spans: &mut Vec<Span<'static>>,
    ) {
        let (walk, token) = walk_text(&self.view.walk);
        push(walk, foreground(self.theme, token), 0, spans);
        let filters = left_out(self.view.filters, folded_lanes);
        if !filters.is_empty() {
            push(
                format!("  {filters}"),
                foreground(self.theme, Token::Accent),
                0,
                spans,
            );
        }
        if matches!(self.view.walk, WalkView::Jumping { .. }) {
            push("  Esc cancels".to_owned(), muted, 0, spans);
        }
        match self.view.order {
            Some(OrderNotice::Sorting) => {
                push("  date order, sorting…".to_owned(), muted, 0, spans);
            }
            Some(OrderNotice::Provisional) => push(
                "  order stays provisional (date)".to_owned(),
                foreground(self.theme, Token::Warning),
                0,
                spans,
            ),
            None => {}
        }
        match &self.view.graphics {
            Some(GraphicsStatus::Ok) => push("  gfx ok".to_owned(), muted, 0, spans),
            Some(GraphicsStatus::Failed(message)) => push(
                format!("  gfx: {message}"),
                foreground(self.theme, Token::Warning),
                0,
                spans,
            ),
            None => {}
        }
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
    // The slot is always reserved, `marker_column` or not: it already
    // carries the pinned-compare-base accent (`GraphEntry::compare_marked`,
    // m2-details-and-diffs/11), so collapsing it to zero width pre-finding
    // would move every later column out from under that feature. Stability
    // ("never shifts on scroll", m2-details-and-diffs.md) is met either way
    // — the width never changes at all; `marker_column` instead gates which
    // *glyph* the slot may draw (`GraphScreen::draw_row`).
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

    /// Every x where one column ends and the next begins, ascending and
    /// deduped, with `area`'s own edges at each end.
    ///
    /// The trailing columns are laid out contiguously right to left, so one
    /// column's end is the next one's start and naming the starts is enough;
    /// the subject's end is named separately because [`SUBJECT_GAP`] sits
    /// between it and the author chip, and that gap is a region of its own
    /// that an overlay may legitimately stop inside. A column that took no
    /// width (the author chip below [`AUTHOR_BREAKPOINT`]) contributes a
    /// boundary equal to its neighbour's and the dedup drops it.
    fn boundaries(&self, area: Rect) -> Vec<u16> {
        let mut xs = vec![
            area.x,
            self.marker_x,
            self.subject_x,
            self.subject_x.saturating_add(self.subject_width),
            self.author_x,
            self.when_x,
            self.sha_x,
            area.right(),
        ];
        xs.retain(|x| *x >= area.x && *x <= area.right());
        xs.sort_unstable();
        xs.dedup();
        xs
    }
}

/// `rect` grown outward until both of its vertical edges sit on a boundary
/// in `boundaries`, so every column it touches is covered whole and none of
/// them is split (ADR-0040).
///
/// Outward, never inward: reaching a boundary by shrinking would cut the
/// box's own content, which trades a legible fragment beside the box for an
/// illegible one inside it. `None` when no such rectangle fits inside
/// `bounds` — the caller then has a box that cannot cover, and ADR-0040
/// says such a box clears the band it stands on instead of splitting it.
///
/// Pure geometry: it knows nothing about the graph and nothing about the
/// box. Both halves arrive as numbers, which is what keeps the rule at the
/// layout site rather than inside either widget.
///
/// **And it stops one cell short of the column it stops at.** The right
/// edge rests on the *gutter* that precedes the next column's text, not on
/// the first cell of that text, so there is a blank column between the
/// border and whatever is beside it (ADR-0040's 2026-09-21 amendment). A
/// box flush against the column it ends at reads as one token — at 80 the
/// confirm box drew `╮60af0eb`, which is the `8443─────` fault of the same
/// ADR turned through ninety degrees. The gutter is not needed at
/// `bounds.right()`, where there is nothing beside the box at all.
///
/// The left edge needs no such step: every row column's own text is laid
/// out after a leading blank, so the cell before a boundary is already a
/// gutter belonging to the column ahead of it.
///
/// Stepping back costs a cell, so a boundary whose gutter no longer covers
/// `rect` is not a snap position at all and the search carries on outward —
/// which is how a box can run out of positions and clear where it used to
/// snap. Shrinking the box to fit one is still forbidden (clause 2).
///
/// An empty `boundaries` returns `rect` unchanged — a screen with no row
/// columns has nothing to split.
#[must_use]
pub fn snap_to_columns(rect: Rect, boundaries: &[u16], bounds: Rect) -> Option<Rect> {
    if rect.is_empty() || boundaries.is_empty() {
        return Some(rect);
    }
    let right = rect.x.saturating_add(rect.width);
    let left = boundaries.iter().copied().filter(|x| *x <= rect.x).max()?;
    let snapped = boundaries
        .iter()
        .copied()
        .filter_map(|x| {
            if x >= bounds.right() {
                Some(x)
            } else {
                x.checked_sub(1)
            }
        })
        .filter(|x| *x >= right)
        .min()?;
    if left < bounds.x || snapped > bounds.right() {
        return None;
    }
    Some(Rect {
        x: left,
        width: snapped.saturating_sub(left),
        ..rect
    })
}
