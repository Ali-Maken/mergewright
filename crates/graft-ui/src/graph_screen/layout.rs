//! Geometry: where the header, the graph column and the trailing columns
//! land, and how many lanes fit at a given width.

use graft_graph::fold::Fold;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::carve::{CarveInputs, row_carve};
use super::header_tail::{Drop, TAIL_SEPARATOR, TailItem, fitted_tail};
use super::{
    AUTHOR_BREAKPOINT, AUTHOR_WIDTH, Filters, GUTTER, GraphEntry, GraphScreen, GraphView,
    GraphicsStatus, HeaderView, MARKER_WIDTH, OrderNotice, RIGHT_MARGIN, SHA_WIDTH, SUBJECT_GAP,
    WHEN_WIDTH, WalkView, banner_line, header_line, tail_room,
};
use crate::graph_column::{CELLS_PER_LANE, GraphColumn};
use crate::paint::foreground;
use crate::theme::Token;
use crate::{cells_of, rect_of};

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
    /// The header's row: the repository, its path and branch, and the walk
    /// or the operation banner.
    pub header: Rect,
    /// The blank row under the header ([`GraphView::spaced`](super::GraphView::spaced));
    /// zero-height below 30 rows.
    pub gap: Rect,
    /// The rows [`GraphView::banner`](super::GraphView::banner) takes under
    /// the header, clipped to what is free above the rest of the chrome.
    pub banner: Rect,
    /// The practice tour's card under the banner
    /// ([`GraphView::tour`](super::GraphView::tour), ticket m8/19b): its
    /// three rows when the body keeps at least one, else zero-height.
    pub tour: Rect,
    /// The still-to-apply block's rows, under the banner
    /// ([`GraphView::pending`](super::GraphView::pending)). At most half of
    /// what the banner leaves, so the graph keeps the other half; zero when
    /// there is nothing pending or no room for a box of three rows.
    pub pending: Rect,
    /// The rows the commit rows are drawn in: inside the two-cell gutter on
    /// the left and the one-cell margin on the right (ADR-0058 §4).
    pub body: Rect,
    /// The `─` rule between the rows and the bottom chrome
    /// ([`GraphView::spaced`](super::GraphView::spaced)); zero-height below
    /// 30 rows.
    pub rule: Rect,
    /// The commit-graph offer's row, directly above the hint bar.
    pub offer: Rect,
    /// The hint bar's row: the screen's last.
    pub footer: Rect,
}

impl<'a> GraphScreen<'a> {
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
        screen_rows(self.view, area)
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
        let mut xs =
            Columns::compute(body, graph_width(body, rows, applied.as_ref())).boundaries(body);
        // The gutter and the margin are blank cells beside the rows, so the
        // frame's own edges are boundaries (ADR-0058 §4). The body's right
        // edge is not: [`snap_to_columns`] rests a right edge one cell
        // before a boundary inside the frame, which there is the SHA's last
        // cell — the margin beyond it is what the frame's edge reaches.
        xs.retain(|x| *x != body.right() || body.right() == area.right());
        xs.extend([area.x, area.right()]);
        xs.sort_unstable();
        xs.dedup();
        xs
    }

    /// The rows' rectangle, the rows that fit in it, and the fold they are
    /// drawn through (`None` while expanded).
    pub(super) fn body_layout(&self, area: Rect) -> (Rect, &[GraphEntry<'a>], Option<Fold>) {
        body_layout(self.view, area)
    }

    /// The header line; `folded_lanes` is how many lanes the drawn fold
    /// hides behind the rail (0 when nothing folds or lanes are expanded).
    pub(super) fn header(&self, width: u16, folded_lanes: usize) -> Line<'static> {
        let location = self.view.location.as_ref();
        let mut header = HeaderView {
            repo: location.map_or("", |at| at.repo.as_str()),
            path: location.map_or("", |at| at.path.as_str()),
            branch: location.map_or("", |at| at.branch.as_str()),
            tail: None,
            practice: location.is_some_and(|at| at.practice),
        };
        // The tail's room comes out before the path's: `header_line` cuts
        // the path first, so a long checkout path never cuts the banner to
        // `MERG…` — the state gone, and the banner is the urgent half
        // (m5/16, review C1).
        let room = usize::from(tail_room(&header, width));
        header.tail = Some(match &self.view.banner_op {
            // The operation banner takes the walk's place — the walk state,
            // filters, order notice and graphics status all answer "is
            // history still loading", which is not the question
            // mid-operation (DESIGN.md §1.13, ADR-0038 §3). `focused`
            // reverses it so a conflict landing is noticed without a click.
            // A standing search keeps its place left of it while both fit
            // whole; the banner is safety, so the search gives way first
            // (m8/15b, ticket 15 A2).
            Some(banner) => {
                let room16 = u16::try_from(room).unwrap_or(u16::MAX);
                let mut style = foreground(self.theme, Token::Warning);
                if banner.focused {
                    style = style.add_modifier(ratatui::style::Modifier::REVERSED);
                }
                let banner = banner_line(banner, room16, self.view.keys);
                let mut spans = Vec::new();
                if let Some(search) = self.search_item()
                    && text_width(&search.text) + TAIL_SEPARATOR.len() + text_width(&banner) <= room
                {
                    spans.push(Span::styled(search.text, search.style));
                    spans.push(Span::raw(TAIL_SEPARATOR));
                }
                spans.push(Span::styled(banner, style));
                Line::from(spans)
            }
            None => Line::from(fitted_tail(self.tail_items(folded_lanes, room), room)),
        });
        header_line(&header, width, self.theme)
    }

    /// The search's line as a tail item, styled as the footer drew it
    /// before it moved here: a warning when the search failed.
    fn search_item(&self) -> Option<TailItem> {
        self.view.search_tail.as_ref().map(|search| TailItem {
            text: search.text.as_str().to_owned(),
            style: foreground(
                self.theme,
                if search.failed {
                    Token::Warning
                } else {
                    Token::Info
                },
            ),
            drop: Drop::Search,
        })
    }

    /// The header's informational tail when no operation banner takes the
    /// walk's place, left to right: the search (m8/15b), the walk state,
    /// filters, `Esc cancels`, the order notice and the graphics status.
    fn tail_items(&self, folded_lanes: usize, room: usize) -> Vec<TailItem> {
        let muted = foreground(self.theme, Token::TextMuted);
        let mut items: Vec<TailItem> = self.search_item().into_iter().collect();
        let mut push = |text: String, style: Style, drop: Drop| {
            if !text.is_empty() {
                items.push(TailItem { text, style, drop });
            }
        };
        let (walk, token) = walk_text_within(&self.view.walk, room);
        push(walk, foreground(self.theme, token), Drop::Walk);
        let filters = left_out(self.view.filters, folded_lanes, self.view.keys);
        push(
            filters,
            foreground(self.theme, Token::Accent),
            Drop::Filters,
        );
        if matches!(self.view.walk, WalkView::Jumping { .. }) {
            push("Esc cancels".to_owned(), muted, Drop::Walk);
        }
        match self.view.order {
            Some(OrderNotice::Sorting) => {
                push("date order, sorting…".to_owned(), muted, Drop::Walk);
            }
            Some(OrderNotice::Provisional) => push(
                "order stays provisional (date)".to_owned(),
                foreground(self.theme, Token::Warning),
                Drop::Walk,
            ),
            None => {}
        }
        match &self.view.graphics {
            Some(GraphicsStatus::Ok) => push("gfx ok".to_owned(), muted, Drop::Gfx),
            Some(GraphicsStatus::Failed(message)) => push(
                format!("gfx: {message}"),
                foreground(self.theme, Token::Warning),
                Drop::Gfx,
            ),
            None => {}
        }
        items
    }
}

/// [`GraphScreen::screen_rows`] as a plain function of the view: the row
/// carve needs no theme and no widget, and a caller that has to know where
/// the body rows are (a mouse hit-test) must read the very same carve rather
/// than a second copy of it. A thin wrapper: the carve itself is
/// [`row_carve`], which a caller with no [`GraphView`] reads directly.
#[must_use]
pub fn screen_rows(view: &GraphView<'_>, area: Rect) -> ScreenRows {
    let carve = row_carve(cells_of(area), CarveInputs::of(view));
    ScreenRows {
        header: rect_of(carve.header),
        gap: rect_of(carve.gap),
        banner: rect_of(carve.banner),
        tour: rect_of(carve.tour),
        pending: rect_of(carve.pending),
        body: rect_of(carve.body),
        rule: rect_of(carve.rule),
        offer: rect_of(carve.offer),
        footer: rect_of(carve.footer),
    }
}

/// The cells a row of `columns` keeps left and right of its content: the
/// gutter and the margin (ADR-0058 §4), or none on a screen too narrow to
/// spare them.
pub(super) fn inset(columns: u16) -> (u16, u16) {
    if columns > GUTTER + RIGHT_MARGIN {
        (GUTTER, RIGHT_MARGIN)
    } else {
        (0, 0)
    }
}

/// The cells a list row of a screen `columns` wide has for its content.
pub(super) fn body_width(columns: u16) -> u16 {
    let (left, right) = inset(columns);
    columns - left - right
}

/// [`GraphScreen::body_layout`] as a plain function of the view.
pub(super) fn body_layout<'a>(
    view: &'a GraphView<'a>,
    area: Rect,
) -> (Rect, &'a [GraphEntry<'a>], Option<Fold>) {
    let body = screen_rows(view, area).body;
    let rows = &view.rows[..view.rows.len().min(usize::from(body.height))];
    let fold = view
        .compaction
        .plan(rows.iter().map(|entry| &*entry.line.row));
    let applied = (!view.compaction.expanded).then_some(fold);
    (body, rows, applied)
}

/// Where the commit rows land in `area`, and the columns each of them is
/// drawn to.
///
/// The one reading of that geometry outside the widget. `graft`'s mouse
/// hit-test needs it to say which badge a cell belongs to, and the rule
/// ADR-0040 already learned — two copies of a layout with nothing testing
/// one against the other — is why it is handed out rather than mirrored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowLayout {
    /// The rectangle the commit rows are drawn in: its first row is
    /// `rows[0]`, its second `rows[1]`, and so on.
    pub body: Rect,
    /// Where each of a row's columns starts and how wide it is.
    pub columns: Columns,
}

/// [`RowLayout`] for `view` drawn into `area` — the same carve
/// [`GraphScreen`]'s own `render` draws from.
#[must_use]
pub fn row_layout(view: &GraphView<'_>, area: Rect) -> RowLayout {
    let (body, rows, applied) = body_layout(view, area);
    RowLayout {
        body,
        columns: Columns::compute(body, graph_width(body, rows, applied.as_ref())),
    }
}

/// The entry drawn at `screen_row`, and the columns it was drawn to.
///
/// `None` above or below the commit rows. Together with
/// [`super::badge_spans`] this is everything a hit-test needs: the entry
/// says which badges the row has, the columns say where they start, and the
/// spans say which cell is which — all three read off the same carve the
/// renderer draws from, so there is nothing here to keep in step by hand.
#[must_use]
pub fn row_at<'a>(
    view: &'a GraphView<'a>,
    area: Rect,
    screen_row: u16,
) -> Option<(&'a GraphEntry<'a>, Columns)> {
    let (body, rows, applied) = body_layout(view, area);
    if body.height == 0 {
        return None;
    }
    let offset = usize::from(screen_row.checked_sub(body.y)?);
    if screen_row >= body.bottom() {
        return None;
    }
    let entry = rows.get(offset)?;
    Some((
        entry,
        Columns::compute(body, graph_width(body, rows, applied.as_ref())),
    ))
}

/// The header's account of what the drawn fold and the filters leave out:
/// the filters, then `+N lanes` while the drawn fold hides any. The count is
/// the screen's to say — it planned the fold — and shows only while
/// something folds, never on a row (ADR-0022).
fn left_out(filters: Filters, folded_lanes: usize, keys: &crate::keymap::Keymap) -> String {
    let mut words = filters.text(keys);
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

/// The header's account of the walk, and the token it is drawn in: the
/// widest wording, as [`walk_text_within`] gives it with room to spare.
#[cfg(test)]
pub(super) fn walk_text(walk: &WalkView) -> (String, Token) {
    walk_text_within(walk, usize::MAX)
}

/// The header's account of the walk in at most `room` columns, where a
/// state has shorter wordings: a stopped walk keeps the widest of
/// `stopped at N rows · ⌃R reloads`, `stopped at N rows`, `stopped` that
/// fits (m6-conflicts/85 A2). Every other state has one wording, cut by the
/// caller like any header text.
pub(super) fn walk_text_within(walk: &WalkView, room: usize) -> (String, Token) {
    match walk {
        WalkView::Opening => ("opening…".to_owned(), Token::TextMuted),
        WalkView::Reading { laid_out: 0 } => ("reading history…".to_owned(), Token::TextMuted),
        WalkView::Reading { laid_out } => (format!("reading… {laid_out} rows"), Token::TextMuted),
        WalkView::Paused { laid_out } => (format!("{laid_out} rows so far"), Token::TextMuted),
        WalkView::Jumping { laid_out } => (format!("laying out… {laid_out} rows"), Token::Accent),
        WalkView::Complete { rows: 0 } => ("no commits yet".to_owned(), Token::TextMuted),
        WalkView::Complete { rows: 1 } => ("1 commit".to_owned(), Token::TextMuted),
        WalkView::Complete { rows } => (format!("{rows} commits"), Token::TextMuted),
        WalkView::Cancelled { laid_out } => (stopped_words(*laid_out, room), Token::TextMuted),
        WalkView::Failed(reason) => (format!("history failed: {reason}"), Token::Warning),
    }
}

/// A stopped walk's words, widest first; the last rung when none fits (the
/// caller cuts it). `⌃` spells Ctrl as the bar does.
fn stopped_words(laid_out: usize, room: usize) -> String {
    let rungs = [
        format!("stopped at {laid_out} rows \u{b7} \u{2303}R reloads"),
        format!("stopped at {laid_out} rows"),
        "stopped".to_owned(),
    ];
    let widest = rungs.iter().find(|words| text_width(words) <= room);
    widest.unwrap_or(&rungs[2]).clone()
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns {
    /// Where the Mistake Radar's marker slot starts.
    pub marker_x: u16,
    /// Its width; always reserved, whether or not a marker draws.
    pub marker_width: u16,
    /// Where the subject column starts — and so where the row's badges do.
    pub subject_x: u16,
    /// Its width, badges included.
    pub subject_width: u16,
    /// Where the author chip starts.
    pub author_x: u16,
    /// Its width; `0` below [`AUTHOR_BREAKPOINT`], the one column that drops.
    pub author_width: u16,
    /// Where the relative-date column starts.
    pub when_x: u16,
    /// Its width.
    pub when_width: u16,
    /// Where the short-SHA column starts.
    pub sha_x: u16,
    /// Its width.
    pub sha_width: u16,
}

impl Columns {
    // The slot is always reserved, `marker_column` or not: it already
    // carries the pinned-compare-base accent (`GraphEntry::compare_marked`,
    // m2-details-and-diffs/11), so collapsing it to zero width pre-finding
    // would move every later column out from under that feature. Stability
    // ("never shifts on scroll", m2-details-and-diffs.md) is met either way
    // — the width never changes at all; `marker_column` instead gates which
    // *glyph* the slot may draw (`GraphScreen::draw_row`).
    pub(crate) fn compute(area: Rect, graph_width: u16) -> Self {
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

        // The breakpoint is the row's whole width (DESIGN.md §1.2/§1.3);
        // `area` is the body, inside the gutter and the margin.
        let show_author = area.width + GUTTER + RIGHT_MARGIN >= AUTHOR_BREAKPOINT;
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
