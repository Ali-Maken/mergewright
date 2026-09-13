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

use graft_graph::identity::BranchIdentity;
use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
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
        if matches!(self.view.walk, WalkView::Jumping { .. }) {
            push("  Esc cancels".to_owned(), muted, &mut spans);
        }
        Line::from(spans)
    }
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

/// The key hints the footer lists. Leaving and the status screen first: a
/// narrow terminal cuts from the right.
const HINTS: &str = " q quit  F2 status  j/k move  PgUp/PgDn page  g/G top/bottom";

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

        let body = Rect {
            y: area.y + 1,
            height: area.height.saturating_sub(CHROME_ROWS),
            ..area
        };
        if body.height > 0 {
            self.body(body, buf);
        }

        if area.height > 1 {
            let footer = match &self.view.problem {
                Some(problem) => Line::styled(
                    fit(&format!(" {problem}"), width),
                    foreground(self.theme, Token::Warning),
                ),
                None => Line::styled(fit(HINTS, width), foreground(self.theme, Token::TextMuted)),
            };
            buf.set_line(area.x, area.bottom() - 1, &footer, area.width);
        }
    }
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
    fn body(&self, area: Rect, buf: &mut Buffer) {
        let rows = &self.view.rows[..self.view.rows.len().min(usize::from(area.height))];
        let lanes = rows
            .iter()
            .map(|entry| entry.line.row.lanes.len().max(entry.line.row.lane + 1))
            .max()
            .unwrap_or(1);
        // Half the width at most: the subject is what makes a row readable.
        let graph_width = GraphColumn::width_for(lanes).min(area.width / 2);
        let graph_lines: Vec<GraphLine<'_>> = rows.iter().map(|entry| entry.line.clone()).collect();
        GraphColumn::new(&graph_lines, self.theme).render(
            Rect {
                width: graph_width,
                ..area
            },
            buf,
        );

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
        let mut style = foreground(self.theme, Token::Text);
        let mut trailing_style = foreground(self.theme, Token::TextMuted);
        if entry.line.paint == RowPaint::Highlighted {
            style = style.add_modifier(Modifier::BOLD);
            trailing_style = trailing_style.add_modifier(Modifier::BOLD);
            let raised = color(self.theme.token(Token::SurfaceRaised));
            buf.set_style(
                Rect {
                    y,
                    height: 1,
                    ..area
                },
                Style::new().bg(raised),
            );
        } else if entry.line.paint == RowPaint::Dimmed {
            style = style.add_modifier(Modifier::DIM);
            trailing_style = trailing_style.add_modifier(Modifier::DIM);
        }

        let glyph = if entry.line.row.marker.is_some() {
            "! "
        } else {
            "  "
        };
        draw_cell(buf, columns.marker_x, y, columns.marker_width, glyph, style);
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
