//! Geometry: where the header, the rows and the bottom rows land, and the
//! header's words. How many lanes fit is [`lanes`](super::lanes); where the
//! row columns stand, [`row_columns`](super::row_columns).

use mergewright_graph::fold::Fold;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::carve::{CarveInputs, row_carve};
use super::header::branch_floor;
use super::header_tail::{Drop, LaneTally, TAIL_SEPARATOR, TailItem, fitted_tail, lane_phrases};
use super::lanes::graph_width;
use super::row_columns::Columns;
use super::walk_text::{text_width, walk_text_within};
use super::{
    GUTTER, GraphEntry, GraphScreen, GraphView, GraphicsStatus, HeaderView, OrderNotice,
    RIGHT_MARGIN, WalkView, banner_line, header_line, tail_room,
};
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
    /// The commit-graph offer's row, above the hint bar (and above the
    /// done line's row when that is drawn).
    pub offer: Rect,
    /// The done line's own row, directly above the hint bar, while
    /// [`GraphView::status_row`](super::GraphView::status_row) says so
    /// (ease-pass/08s M4); zero-height otherwise.
    pub status: Rect,
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
        let (body, _, fold) = self.body_layout(area);
        Rect {
            width: graph_width(body, &fold),
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
        let (body, _, fold) = self.body_layout(area);
        let mut xs = Columns::compute(body, graph_width(body, &fold)).boundaries(body);
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
    /// drawn through (uncapped while expanded).
    pub(super) fn body_layout(&self, area: Rect) -> (Rect, &[GraphEntry<'a>], Fold) {
        body_layout(self.view, area)
    }

    /// The header line; `lanes` is what the drawn fold and rows say of the
    /// lanes and folds in view (ease-pass 06b).
    pub(super) fn header(&self, width: u16, lanes: LaneTally) -> Line<'static> {
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
        // The lane phrases give way before the branch name (ease-pass 06b).
        let lanes_room = room.saturating_sub(branch_floor(&header));
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
            None => Line::from(fitted_tail(self.tail_items(lanes, room), room, lanes_room)),
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
    /// filters, `Esc cancels`, the order notice, the graphics status and the
    /// lane phrases (ease-pass 06b).
    fn tail_items(&self, lanes: LaneTally, room: usize) -> Vec<TailItem> {
        let muted = foreground(self.theme, Token::TextMuted);
        let mut items: Vec<TailItem> = self.search_item().into_iter().collect();
        let mut push = |text: String, style: Style, drop: Drop| {
            if !text.is_empty() {
                items.push(TailItem { text, style, drop });
            }
        };
        let (walk, token) = walk_text_within(&self.view.walk, room);
        push(walk, foreground(self.theme, token), Drop::Walk);
        let filters = self.view.filters.text(self.view.keys);
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
        if let Some(words) = &self.view.push {
            push(words.as_str().to_owned(), muted, Drop::Push);
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
        for phrase in lane_phrases(lanes, self.view.filters.every_branch) {
            push(phrase, muted, Drop::Lanes);
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
        status: rect_of(carve.status),
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
) -> (Rect, &'a [GraphEntry<'a>], Fold) {
    let body = screen_rows(view, area).body;
    let rows = &view.rows[..view.rows.len().min(usize::from(body.height))];
    let fold = view
        .compaction
        .plan(rows.iter().map(|entry| &*entry.line.row));
    (body, rows, fold)
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
    let (body, _, fold) = body_layout(view, area);
    RowLayout {
        body,
        columns: Columns::compute(body, graph_width(body, &fold)),
    }
}

/// The entry drawn at `screen_row`, and the columns it was drawn to.
///
/// `None` above or below the commit rows. Together with
/// [`super::ref_spans`] this is everything a hit-test needs: the entry
/// says which badges the row has, the columns say where they start, and the
/// spans say which cell is which — all three read off the same carve the
/// renderer draws from, so there is nothing here to keep in step by hand.
#[must_use]
pub fn row_at<'a>(
    view: &'a GraphView<'a>,
    area: Rect,
    screen_row: u16,
) -> Option<(&'a GraphEntry<'a>, Columns)> {
    let (body, rows, fold) = body_layout(view, area);
    if body.height == 0 {
        return None;
    }
    let offset = usize::from(screen_row.checked_sub(body.y)?);
    if screen_row >= body.bottom() {
        return None;
    }
    let entry = rows.get(offset)?;
    Some((entry, Columns::compute(body, graph_width(body, &fold))))
}
