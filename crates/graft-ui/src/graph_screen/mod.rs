//! The screen Graft opens on: the history graph, one commit per row.
//!
//! M1's first graph screen, deliberately plain: a header line saying where
//! the walk is, the rows the viewport shows (the Unicode graph column and one
//! subject column), and a footer of key hints. Columns and badges replace the
//! subject column in later tickets; the full chrome of DESIGN.md §1.1–§1.3
//! arrives with them.
//!
//! Like every screen it renders plain data: a [`GraphView`] that `app`
//! assembles from the rows the walk last sent. It never asks for rows and
//! never scrolls — the viewport arrives already cut.

pub mod badges;
pub mod columns;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use graft_core::capabilities::ColorDepth;
use graft_graph::fold::Fold;
use graft_graph::identity::BranchIdentity;
use graft_graph::row::Row;
use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};
use unicode_width::UnicodeWidthStr;

pub use badges::{Badge, BadgeKind, Icons, Tracking};
pub use columns::{initials, relative_date, short_sha};

use crate::fit::fit;
use crate::graph_column::{GraphColumn, GraphLine, RowPaint};
use crate::paint::{color, foreground};
use crate::theme::{Theme, Token};

/// Cells the marker slot always takes: its glyph, then a gap. Reserved
/// whether or not a row has a marker (`graph` never sets one; `ops` will,
/// M2), so the columns after it never move when markers start appearing.
const MARKER_WIDTH: u16 = 2;

/// Cells the author chip takes: two initials, then a gap.
const AUTHOR_WIDTH: u16 = 3;

/// Cells the relative-date column takes: its widest form (`"999d"`), then a
/// gap.
const WHEN_WIDTH: u16 = 5;

/// Cells the short-SHA column takes, and how many hex digits `short_sha`
/// should be called with to fill it exactly (`DESIGN.md`'s mock-ups show
/// `a1b2c3d`) — rightmost, so no trailing gap.
pub const SHA_WIDTH: u16 = 7;

/// The gap left of the trailing columns, between them and the subject.
const SUBJECT_GAP: u16 = 1;

/// Below this width the author chip drops (DESIGN.md §1.2/§1.3's
/// narrow/medium boundary); ref badges (m1-the-graph/11) drop at the same
/// width.
pub const AUTHOR_BREAKPOINT: u16 = 100;

/// Screen rows that are not graph rows: the header above, the key hints
/// below.
pub const CHROME_ROWS: u16 = 2;

/// Everything the graph screen draws.
#[derive(Clone, Debug)]
pub struct GraphView<'a> {
    /// The repository's path, once it is open.
    pub location: Option<SanitizedText>,
    /// Where the history walk is.
    pub walk: WalkView,
    /// Something went wrong and has not been superseded. Takes the footer's
    /// place while it lasts; absent when nothing is wrong.
    pub problem: Option<SanitizedText>,
    /// The rows in the viewport, top first. The cursor's row is the one
    /// painted [`RowPaint::Highlighted`].
    pub rows: Vec<GraphEntry<'a>>,
    /// Which glyphs ref badges draw with (`ui.icons`).
    pub icons: Icons,
    /// How many lanes the graph may draw before folding the rest behind
    /// `+N`, and whether the user has expanded them (DESIGN.md §3.5).
    pub compaction: Compaction,
    /// What the history shown leaves out; said in the header so a folded or
    /// first-parent graph is never mistaken for the whole one.
    pub filters: Filters,
    /// The commit tooltip, open over the graph: hover or the keyboard opened
    /// it on the same content (PROMPT.md §1.3, DESIGN.md §2.15). `None` when
    /// closed.
    pub tooltip: Option<Tooltip>,
    /// The order notice, while the rows are not yet in their final order.
    pub order: Option<OrderNotice>,
    /// Draw the graph column with the ASCII glyph table rather than Unicode
    /// box-drawing (`graph.renderer`, ticket m1-the-graph/19; DESIGN.md §4.2,
    /// ADR-0016).
    pub ascii: bool,
    /// The pixel tier draws the graph: the column keeps its width but is left
    /// blank, for the terminal's images to fill (ticket m1-the-graph/24c).
    pub pixel: bool,
    /// How the terminal took the last graph images, while the pixel tier
    /// draws them; `None` at every other tier.
    pub graphics: Option<GraphicsStatus>,
    /// What the app layer did to the renderer tier the user did not ask to
    /// be told twice — the tier-drop key said what it did. Takes the
    /// footer's place under [`GraphView::problem`], above the hints.
    pub notice: Option<SanitizedText>,
    /// The search prompt or the search's state, in the footer above a notice
    /// and under a problem; `None` when there is no search (m1-the-graph/26).
    pub search: Option<SearchLine>,
}

/// What the terminal said about the graph images, named by state: `gfx ok`
/// when it reported nothing wrong, `gfx: <message>` when it did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphicsStatus {
    /// No image command failed since the last wake.
    Ok,
    /// The terminal refused an image command, and said this.
    Failed(SanitizedText),
}

/// The commit tooltip: the full message and the parents, reachable
/// identically from a hover and from the keyboard. "Contained in"
/// (PROMPT.md §1.3) is left for M2 (`KNOWN_GAPS`) — nothing here fakes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tooltip {
    /// The commit's subject line, sanitised.
    pub subject: SanitizedText,
    /// The rest of the message, sanitised; empty when the commit has none.
    pub body: SanitizedText,
    /// Parent short SHAs, first parent first. Hex from a fixed alphabet is
    /// safe as-is, like [`GraphEntry::sha`] — no sanitising needed.
    pub parents: Vec<String>,
}

/// What the history on screen leaves out (PROMPT.md §1.3). The default is
/// nothing: every commit, every parent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Filters {
    /// Only first parents are followed.
    pub first_parent: bool,
    /// How many merges are drawn folded.
    pub folded: usize,
    /// The stash is hidden.
    pub stash_hidden: bool,
    /// Focus mode is on (DESIGN.md §5.2): rows off the selection's thread
    /// fade. Nothing is left out, but the header says so, with its way out.
    pub focused: bool,
}

impl Filters {
    /// The header's words for these filters: empty when nothing is left out.
    #[must_use]
    pub fn text(self) -> String {
        let mut parts = Vec::new();
        if self.first_parent {
            parts.push("first-parent".to_owned());
        }
        match self.folded {
            0 => {}
            1 => parts.push("1 merge folded".to_owned()),
            many => parts.push(format!("{many} merges folded")),
        }
        if self.stash_hidden {
            parts.push("stash hidden".to_owned());
        }
        if self.focused {
            parts.push("focus (F/Esc off)".to_owned());
        }
        parts.join(" · ")
    }
}

/// Lane compaction as the screen applies it (DESIGN.md §3.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compaction {
    /// `graph.max_lanes`: the most lanes drawn, the `+N` marker included.
    pub max_lanes: usize,
    /// The user pressed the expand key: every lane draws in its real column
    /// until they press it again. The rows are the same either way — the
    /// fold is a lens over them, never a re-layout.
    pub expanded: bool,
    /// Columns that never fold beyond lane 0: the reserved integration lanes
    /// and, once selection exists, the selected commit's ancestry.
    pub pinned: BTreeSet<usize>,
}

impl Compaction {
    /// Compaction at `max_lanes`, collapsed, with nothing pinned but lane 0.
    #[must_use]
    pub fn at(max_lanes: usize) -> Self {
        Compaction {
            max_lanes,
            expanded: false,
            pinned: BTreeSet::new(),
        }
    }

    /// The fold for `rows`, whether or not it is applied.
    fn plan<'a>(&self, rows: impl IntoIterator<Item = &'a Row>) -> Fold {
        Fold::plan(rows, self.max_lanes, &self.pinned)
    }
}

/// One row of the graph screen.
#[derive(Clone, Debug)]
pub struct GraphEntry<'a> {
    /// The graph column's cells for this row, and how strongly it paints.
    pub line: GraphLine<'a>,
    /// The refs on this commit, in the order they are drawn. Empty for
    /// most rows.
    pub badges: Vec<Badge>,
    /// The commit's subject, sanitised.
    pub subject: SanitizedText,
    /// The author's chip: initials and the identity that colours it.
    pub author: AuthorChip,
    /// How long ago the commit was made, already formatted (`"2m"`, `"3h"`).
    /// Not repository text — a clock reading `app` computed — so it needs no
    /// sanitising.
    pub when: String,
    /// The commit's short SHA, lowercase hex. Not repository text either:
    /// hex drawn from a fixed digit alphabet is safe as-is
    /// (`graft_core::oid::Oid`'s own doc comment).
    pub sha: String,
}

/// An author's chip: two initials, coloured by the author's identity.
///
/// The colour comes from [`BranchIdentity`] and the same lane palette a
/// branch draws in ([`Theme::branch`](crate::theme::Theme::branch)) — reused
/// rather than given its own palette, since both are "a stable colour for a
/// hash of some bytes" and a second palette would only have to be proven
/// colourblind-safe all over again.
#[derive(Clone, Debug)]
pub struct AuthorChip {
    /// The two initials drawn in the chip, sanitised.
    pub initials: SanitizedText,
    /// The identity the chip's colour comes from — of the author's
    /// `.mailmap`-resolved email, so two identities mapped to one person
    /// share a colour along with their chip.
    pub identity: BranchIdentity,
}

/// What the header says about the order the rows are in, when it is not the
/// final one (ADR-0008). Absent when there is nothing to say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderNotice {
    /// Rows are in commit-date order while the topological order is worked
    /// out; `Esc` stops that and keeps this order.
    Sorting,
    /// The sort was stopped or failed: the rows stay in commit-date order.
    Provisional,
}

/// Where the history walk is, as the header says it. Each state is worded
/// for itself: a walk that is fine never reads as an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalkView {
    /// No walk has started yet: the repository is still opening.
    Opening,
    /// Rows are laid out as the screen asks; this many so far.
    Reading {
        /// Rows laid out so far.
        laid_out: usize,
    },
    /// `G` is walking to the end of history; this many rows so far.
    Jumping {
        /// Rows laid out so far.
        laid_out: usize,
    },
    /// Every commit has a row.
    Complete {
        /// How many rows history has.
        rows: usize,
    },
    /// The user stopped the walk; the rows already laid out stay.
    Cancelled {
        /// Rows laid out when it stopped.
        laid_out: usize,
    },
    /// The walk broke, and this is why.
    Failed(SanitizedText),
}

/// The graph screen, ready to render.
#[derive(Clone, Copy, Debug)]
pub struct GraphScreen<'a> {
    view: &'a GraphView<'a>,
    theme: &'a Theme,
}

impl<'a> GraphScreen<'a> {
    /// Draw `view` in `theme`.
    #[must_use]
    pub fn new(view: &'a GraphView<'a>, theme: &'a Theme) -> Self {
        GraphScreen { view, theme }
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
    fn body_layout(&self, area: Rect) -> (Rect, &[GraphEntry<'a>], Option<Fold>) {
        let body = Rect {
            y: area.y + 1,
            height: area.height.saturating_sub(CHROME_ROWS),
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

    fn header(&self, width: usize) -> Line<'static> {
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
        let filters = self.view.filters.text();
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

/// Cells the graph column takes in the rows' `area`: what its lanes want,
/// half the width at most — the subject is what makes a row readable.
fn graph_width(area: Rect, rows: &[GraphEntry<'_>], fold: Option<&Fold>) -> u16 {
    let full_width = || {
        let lanes = rows
            .iter()
            .map(|entry| entry.line.row.lanes.len().max(entry.line.row.lane + 1))
            .max()
            .unwrap_or(1);
        GraphColumn::width_for(lanes)
    };
    fold.map_or_else(full_width, GraphColumn::width_through)
        .min(area.width / 2)
}

/// The header's account of the walk, and the token it is drawn in.
fn walk_text(walk: &WalkView) -> (String, Token) {
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

/// What the footer says about search: the prompt being typed, or how the
/// search stands. Only a failed search reads as a warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchLine {
    /// The `/` prompt: the query typed so far.
    Prompt(SanitizedText),
    /// A running or finished search: its line, and whether it failed.
    Status {
        /// Query, state, position and keys.
        text: SanitizedText,
        /// The search failed; drawn as a warning.
        failed: bool,
    },
}

/// Every key the graph screen answers, in the order the footer drops them
/// (PROMPT.md principle 4: the hint bar teaches the tool). Leaving, the
/// status screen and the renderer first, then `+` while it applies
/// ([`LEADING_HINTS`] of these come before it), then the actions, and the
/// movement keys last: a narrow terminal drops whole hints from the right,
/// and the hint that comes and goes matters more than the movement keys
/// that are always there. `n`/`N` live on the search line, where they work.
const HINTS: [&str; 13] = [
    "q quit",
    "F2 status",
    "F3 renderer",
    "/ search",
    "j/k move",
    "Enter details",
    "Space fold",
    "1 first-parent",
    "F focus",
    "H stash",
    "PgUp/PgDn page",
    "g/G top/bottom",
    "Ctrl+R refresh",
];
/// How many of [`HINTS`] come before the lane-expansion hint.
const LEADING_HINTS: usize = 3;

/// The hint for the lane-expansion key, shown only while there is something
/// to expand or collapse.
fn expand_hint(fold: &Fold, expanded: bool) -> Option<&'static str> {
    match (fold.marker().is_some(), expanded) {
        (false, false) => None,
        (_, true) => Some("+ collapse lanes"),
        (true, false) => Some("+ expand lanes"),
    }
}

/// The footer's key hints within `width` columns: as many whole hints as
/// fit, dropped from the right, never one cut in half.
fn key_hints(expand: Option<&str>, width: usize) -> String {
    let (leading, rest) = HINTS.split_at(LEADING_HINTS);
    let mut line = String::new();
    for hint in leading
        .iter()
        .copied()
        .chain(expand)
        .chain(rest.iter().copied())
    {
        let separator = if line.is_empty() { " " } else { "  " };
        if !line.is_empty() && text_width(&line) + text_width(separator) + text_width(hint) > width
        {
            break;
        }
        line.push_str(separator);
        line.push_str(hint);
    }
    line
}

fn text_width(text: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    text.width()
}

impl Widget for GraphScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            // A pty that has not been sized yet (ADR-0012).
            return;
        }
        let width = usize::from(area.width);
        buf.set_line(area.x, area.y, &self.header(width), area.width);

        // Planned over exactly the rows drawn, expanded or not: the hint
        // has to know whether there is anything to collapse.
        let (body, rows, applied) = self.body_layout(area);
        let fold = self
            .view
            .compaction
            .plan(rows.iter().map(|entry| &*entry.line.row));
        if body.height > 0 {
            self.body(body, rows, applied.as_ref(), buf);
        }

        if area.height > 1 {
            let (text, token) = match (&self.view.problem, &self.view.search, &self.view.notice) {
                (Some(problem), _, _) => (format!(" {problem}"), Token::Warning),
                (None, Some(SearchLine::Prompt(typed)), _) => (format!(" /{typed}_"), Token::Info),
                (None, Some(SearchLine::Status { text, failed }), _) => (
                    format!(" {text}"),
                    if *failed { Token::Warning } else { Token::Info },
                ),
                (None, None, Some(notice)) => (format!(" {notice}"), Token::Info),
                (None, None, None) => (
                    key_hints(expand_hint(&fold, self.view.compaction.expanded), width),
                    Token::TextMuted,
                ),
            };
            let footer = Line::styled(fit(&text, width), foreground(self.theme, token));
            buf.set_line(area.x, area.bottom() - 1, &footer, area.width);
        }

        if let Some(tooltip) = &self.view.tooltip {
            render_tooltip(tooltip, area, self.theme, buf);
        }
    }
}

/// Draws the commit tooltip as a bordered box floating over the graph,
/// centred in `area`. Wrapped, so a long body never runs off the terminal.
fn render_tooltip(tooltip: &Tooltip, area: Rect, theme: &Theme, buf: &mut Buffer) {
    let mut lines = vec![Line::styled(
        tooltip.subject.as_str().to_owned(),
        foreground(theme, Token::Text).add_modifier(Modifier::BOLD),
    )];
    if !tooltip.body.as_str().is_empty() {
        lines.push(Line::raw(""));
        lines.extend(
            tooltip
                .body
                .as_str()
                .lines()
                .map(|line| Line::raw(line.to_owned())),
        );
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        if tooltip.parents.is_empty() {
            "root commit — no parents".to_owned()
        } else {
            format!("parents: {}", tooltip.parents.join(" "))
        },
        foreground(theme, Token::TextMuted),
    ));

    let width = area.width.saturating_sub(4).clamp(1, 70).min(area.width);
    let content_height = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    let height = content_height
        .saturating_add(2)
        .min(area.height.saturating_sub(2))
        .max(1)
        .min(area.height);
    let tooltip_area = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    Clear.render(tooltip_area, buf);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(foreground(theme, Token::Border))
        .style(Style::new().bg(color(theme.token(Token::SurfaceRaised))));
    Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .style(foreground(theme, Token::Text))
        .render(tooltip_area, buf);
}

/// Where each trailing column sits and how wide it is, once the graph and
/// the subject have taken their share.
///
/// Computed right to left from `area.right()`, each column clamped by
/// `min`/`saturating_sub` against what is left: at any width, down to zero,
/// a column simply stops taking space rather than the arithmetic
/// underflowing. The author chip is the one column that can drop entirely,
/// below [`AUTHOR_BREAKPOINT`] (DESIGN.md §1.2/§1.3); the rest only narrow.
struct Columns {
    marker_x: u16,
    marker_width: u16,
    subject_x: u16,
    subject_width: u16,
    author_x: u16,
    author_width: u16,
    when_x: u16,
    when_width: u16,
    sha_x: u16,
    sha_width: u16,
}

impl Columns {
    fn compute(area: Rect, graph_width: u16) -> Self {
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

impl GraphScreen<'_> {
    /// The rows: graph, the reserved marker slot, the subject, and — when
    /// there is room — the author chip, the relative date and the short SHA.
    fn body(&self, area: Rect, rows: &[GraphEntry<'_>], fold: Option<&Fold>, buf: &mut Buffer) {
        let graph_width = graph_width(area, rows, fold);
        // At the pixel tier the cells stay blank: text drawn there would sit
        // on top of the terminal's images.
        if !self.view.pixel {
            let graph_lines: Vec<GraphLine<'_>> =
                rows.iter().map(|entry| entry.line.clone()).collect();
            let mut column = GraphColumn::new(&graph_lines, self.theme).ascii(self.view.ascii);
            if let Some(fold) = fold {
                column = column.through(fold);
            }
            column.render(
                Rect {
                    width: graph_width,
                    ..area
                },
                buf,
            );
        }

        let columns = Columns::compute(area, graph_width);
        for (entry, y) in rows.iter().zip(area.top()..) {
            self.draw_row(entry, y, area, &columns, buf);
        }
    }

    /// One row's text columns: highlight/dim first, then the marker slot,
    /// the subject, and whichever trailing columns fit.
    fn draw_row(
        &self,
        entry: &GraphEntry<'_>,
        y: u16,
        area: Rect,
        columns: &Columns,
        buf: &mut Buffer,
    ) {
        let (style, trailing_style, palette16_edge) = self.row_emphasis(entry, y, area, buf);

        let (glyph, glyph_style) = if entry.line.row.marker.is_some() {
            ("! ", style)
        } else if palette16_edge {
            (
                "▏ ",
                foreground(self.theme, Token::Accent).add_modifier(Modifier::BOLD),
            )
        } else {
            ("  ", style)
        };
        draw_cell(
            buf,
            columns.marker_x,
            y,
            columns.marker_width,
            glyph,
            glyph_style,
        );
        let taken = self.draw_badges(entry, y, columns, style, buf);
        draw_cell(
            buf,
            columns.subject_x + taken,
            y,
            columns.subject_width - taken,
            entry.subject.as_str(),
            style,
        );
        let author_style = trailing_style.fg(color(self.theme.branch(entry.author.identity)));
        draw_cell(
            buf,
            columns.author_x,
            y,
            columns.author_width,
            entry.author.initials.as_str(),
            author_style,
        );
        draw_cell(
            buf,
            columns.when_x,
            y,
            columns.when_width,
            &entry.when,
            trailing_style,
        );
        draw_cell(
            buf,
            columns.sha_x,
            y,
            columns.sha_width,
            &entry.sha,
            trailing_style,
        );
    }

    /// The text style, the trailing-column style, and whether the row falls
    /// back to the Palette16 rule instead of a background fill: `Highlighted`
    /// paints bold and, where the terminal can show it, a raised background;
    /// `Dimmed` paints faint; anything else is plain.
    ///
    /// At [`ColorDepth::Palette16`], `bg`/`surface`/`surface-raised` collapse
    /// to the same index (KNOWN_GAPS.md M0), so the raised background step
    /// would paint nothing a viewer can see — the marker slot's rule
    /// (DESIGN.md §4.6) carries the highlight there instead, so the
    /// background step is skipped rather than drawn for nothing. `NO_COLOR`
    /// ([`ColorDepth::None`]) has no background step either, so it takes the
    /// same rule — uncoloured, its shape alone marks the row.
    fn row_emphasis(
        &self,
        entry: &GraphEntry<'_>,
        y: u16,
        area: Rect,
        buf: &mut Buffer,
    ) -> (Style, Style, bool) {
        let mut style = foreground(self.theme, Token::Text);
        let mut trailing_style = foreground(self.theme, Token::TextMuted);
        let highlighted = entry.line.paint == RowPaint::Highlighted;
        let palette16_edge = highlighted
            && matches!(
                self.theme.color_depth(),
                ColorDepth::Palette16 | ColorDepth::None
            );

        if highlighted {
            style = style.add_modifier(Modifier::BOLD);
            trailing_style = trailing_style.add_modifier(Modifier::BOLD);
            if !palette16_edge {
                let raised = color(self.theme.token(Token::SurfaceRaised));
                buf.set_style(
                    Rect {
                        y,
                        height: 1,
                        ..area
                    },
                    Style::new().bg(raised),
                );
            }
        } else if entry.line.paint == RowPaint::Dimmed {
            style = style.add_modifier(Modifier::DIM);
            trailing_style = trailing_style.add_modifier(Modifier::DIM);
        }

        (style, trailing_style, palette16_edge)
    }
}

impl GraphScreen<'_> {
    /// The row's ref badges, left of the subject, and how many cells they
    /// took (their trailing gap included).
    ///
    /// Badges never take more than half the subject column: at 80 columns a
    /// commit with a pile of refs still says what it is. What does not fit is
    /// cut with `…` rather than pushing the subject off the row.
    fn draw_badges(
        &self,
        entry: &GraphEntry<'_>,
        y: u16,
        columns: &Columns,
        style: Style,
        buf: &mut Buffer,
    ) -> u16 {
        if entry.badges.is_empty() {
            return 0;
        }
        let icons = self.view.icons;
        let texts: Vec<String> = entry.badges.iter().map(|badge| badge.text(icons)).collect();
        let separator = badges::separator(icons);
        let wanted = texts.iter().map(|text| text.width()).sum::<usize>()
            + separator.width() * (texts.len() - 1)
            + usize::from(SUBJECT_GAP);
        let budget = columns.subject_width / 2;
        let taken = u16::try_from(wanted).unwrap_or(u16::MAX).min(budget);
        // Room for the badges themselves; the gap after them stays blank.
        let room = taken.saturating_sub(SUBJECT_GAP);

        let mut x = columns.subject_x;
        let end = columns.subject_x + room;
        for (index, (badge, text)) in entry.badges.iter().zip(&texts).enumerate() {
            if index > 0 {
                let gap = u16::try_from(separator.width()).unwrap_or(u16::MAX);
                x = x.saturating_add(gap).min(end);
            }
            let width = u16::try_from(text.width()).unwrap_or(u16::MAX).min(end - x);
            if width == 0 {
                break;
            }
            draw_cell(buf, x, y, width, text, style.patch(self.badge_style(badge)));
            x += width;
        }
        taken
    }

    /// The colour and weight one badge draws in: branches and remotes in
    /// their lane's colour, so a badge and its line read as one thing.
    fn badge_style(&self, badge: &Badge) -> Style {
        match &badge.kind {
            BadgeKind::Branch {
                identity, current, ..
            } => {
                let style = Style::new().fg(color(self.theme.branch(*identity)));
                if *current {
                    style.add_modifier(Modifier::BOLD)
                } else {
                    style
                }
            }
            BadgeKind::Remote { identity } => Style::new().fg(color(self.theme.branch(*identity))),
            BadgeKind::Tag => foreground(self.theme, Token::Warning),
            BadgeKind::Head => foreground(self.theme, Token::Accent).add_modifier(Modifier::BOLD),
            BadgeKind::Stash => foreground(self.theme, Token::Info),
        }
    }
}

/// Draws `text`, clamped and padded to exactly `width` cells, at `(x, y)`.
/// Does nothing when `width` is zero — the column has dropped for this
/// area's size.
fn draw_cell(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }
    let text = fit(text, usize::from(width));
    buf.set_stringn(x, y, text, usize::from(width), style);
}
