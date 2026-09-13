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

#[cfg(test)]
mod tests;

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::graph_column::{GraphColumn, GraphLine, RowPaint};
use crate::paint::{color, foreground};
use crate::theme::{Theme, Token};

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
}

/// One row of the graph screen.
#[derive(Clone, Debug)]
pub struct GraphEntry<'a> {
    /// The graph column's cells for this row, and how strongly it paints.
    pub line: GraphLine<'a>,
    /// The commit's subject, sanitised.
    pub subject: SanitizedText,
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

impl GraphScreen<'_> {
    /// The rows: graph column, a gap, the subject.
    fn body(&self, area: Rect, buf: &mut Buffer) {
        let rows = &self.view.rows[..self.view.rows.len().min(usize::from(area.height))];
        let lanes = rows
            .iter()
            .map(|entry| entry.line.row.lanes.len().max(entry.line.row.lane + 1))
            .max()
            .unwrap_or(1);
        // Half the width at most: the subject is what makes a row readable.
        let graph_width = GraphColumn::width_for(lanes).min(area.width / 2);
        let graph_lines: Vec<GraphLine<'_>> = rows.iter().map(|entry| entry.line).collect();
        GraphColumn::new(&graph_lines, self.theme).render(
            Rect {
                width: graph_width,
                ..area
            },
            buf,
        );

        let subject_x = area.x + graph_width.saturating_add(1).min(area.width);
        let subject_width = area.right().saturating_sub(subject_x);
        for (entry, y) in rows.iter().zip(area.top()..) {
            let mut style = foreground(self.theme, Token::Text);
            if entry.line.paint == RowPaint::Highlighted {
                style = style.add_modifier(Modifier::BOLD);
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
            }
            if subject_width > 0 {
                let text = fit(entry.subject.as_str(), usize::from(subject_width));
                buf.set_stringn(subject_x, y, text, usize::from(subject_width), style);
            }
        }
    }
}
