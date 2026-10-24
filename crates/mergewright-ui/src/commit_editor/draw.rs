//! Drawing the commit screen (design note "New UI") into the rectangles of
//! [`commit_layout`]: header, message box (subject 1, body 3), summary
//! line, then `files 44 | diff` — files alone under
//! [`DIFF_COLUMNS`](super::DIFF_COLUMNS) — and the bar at the bottom.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use super::layout::{CommitLayout, commit_layout};
use super::words::{fitted_bar, header_line, summary_line};
use super::{CommitScreen, Summary};
use crate::fit::fit;
use crate::graph_screen::{HeaderView, header_line as top_line};
use crate::paint::foreground;
use crate::text_input::{FOCUS, draw_line};
use crate::theme::Token;
use crate::{cells_of, rect_of};

mod split;

/// The subject's placeholder (C1).
pub(super) const SUBJECT_PLACEHOLDER: &str = "What did you change? (one line)";
/// The body's placeholder (C3).
pub(super) const BODY_PLACEHOLDER: &str = "More detail, if it needs it (optional)";

/// The file cursor's mark while the files do not have focus: the row the
/// diff shows, with the keys elsewhere.
const CURSOR_AWAY: char = '\u{203a}';

/// Cells between the repository words and the commit's (the mock's gap).
const HEADER_GAP: usize = 2;

impl Widget for &CommitScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        // The screen replaces whatever was there, so it owns its background.
        buf.set_style(
            area,
            Style::new().bg(crate::paint::color(self.theme.token(Token::Bg))),
        );
        // The rectangles a click is tested against, so a click lands on
        // what is drawn (08s M5).
        let layout = commit_layout(cells_of(area));
        self.draw_header(buf, rect_of(layout.header));
        if layout.bar.height > 0 {
            let diff_shown = layout.diff.is_some_and(|diff| diff.height > 0);
            self.draw_bar(buf, rect_of(layout.bar), diff_shown);
        }
        if layout.message_box.height > 0 {
            self.draw_message_box(buf, &layout);
        }
        if layout.summary.height > 0 {
            self.draw_summary(buf, rect_of(layout.summary));
        }
        if layout.rule.height > 0 {
            self.draw_split(buf, &layout);
        }
    }
}

impl CommitScreen<'_> {
    /// The top row (08s m12): the repository words the graph's top line
    /// starts with, drawn and cut by that line's own function — the path
    /// first, from the left — in the room the commit's words leave; then
    /// those words, which give way only to the repository's name.
    fn draw_header(&self, buf: &mut Buffer, row: Rect) {
        let words = header_line(&self.view.header);
        let bold = foreground(self.theme, Token::Text).add_modifier(Modifier::BOLD);
        let width = usize::from(row.width);
        let Some(location) = &self.view.header.location else {
            buf.set_string(row.x, row.y, fit(&format!(" {words}"), width), bold);
            return;
        };
        let repo = HeaderView {
            repo: location.repo.as_str(),
            path: location.path.as_str(),
            // The branch is in the commit's words: `committing to <branch>`.
            branch: "",
            tail: None,
            practice: location.practice,
        };
        // The name alone, and the cell of margin `top_line` keeps.
        let name_only = HeaderView {
            path: "",
            ..repo.clone()
        };
        let name = top_line(&name_only, row.width, self.theme).width() + 1;
        let room = width.saturating_sub(words.width() + HEADER_GAP).max(name);
        let line = top_line(&repo, u16::try_from(room).unwrap_or(u16::MAX), self.theme);
        buf.set_line(row.x, row.y, &line, row.width);
        let at = line.width() + HEADER_GAP;
        // One cell of right margin, as the graph's top line keeps.
        let left = width.saturating_sub(at + 1);
        if left > 0 {
            let x = row.x + u16::try_from(at).unwrap_or(u16::MAX);
            buf.set_string(x, row.y, fit(&words, left), bold);
        }
    }

    /// The bar, or the failure that takes its row (hook fails: C13's
    /// "the failure on its status row"). `diff_shown`: a diff is on screen
    /// for `↑/↓` to show.
    fn draw_bar(&self, buf: &mut Buffer, row: Rect, diff_shown: bool) {
        if let Some(status) = &self.view.status {
            // A failure leads with the renderer's mark; the dismiss key is
            // the line's keys, kept whole while the reason is cut.
            let dismiss = self
                .keys
                .key_verb(&[crate::keymap::ActionId("app.back")], "dismisses");
            let problem = crate::graph_screen::ProblemLine {
                keys: dismiss.map(|words| mergewright_term::sanitize_line(&words)),
                ..status.clone()
            };
            let line = crate::failure::status_line(
                &problem,
                self.ascii,
                row.width,
                self.theme,
                foreground(self.theme, Token::Warning),
            );
            buf.set_line(row.x, row.y, &line, row.width);
            return;
        }
        let (line, token) = match &self.view.live {
            // An operation went live while the screen was up: `⏎` would be
            // refused, so its words — where the operation is finished —
            // take the bar's row (m6-conflicts/48).
            Some(live) => (live.as_str().to_owned(), Token::Warning),
            // Whole keys or none, the least needed dropped first.
            None => (
                fitted_bar(
                    self.view,
                    self.keys,
                    usize::from(row.width).saturating_sub(1),
                    diff_shown,
                ),
                Token::TextMuted,
            ),
        };
        buf.set_string(
            row.x,
            row.y,
            fit(&format!(" {line}"), usize::from(row.width)),
            foreground(self.theme, token),
        );
    }

    /// `┌─┐`, the subject with its `n/72`, the body rows, `└─┘`.
    fn draw_message_box(&self, buf: &mut Buffer, layout: &CommitLayout) {
        let area = rect_of(layout.message_box);
        let body_rows = layout.body.height;
        let focused = self.view.focus.types();
        let border = foreground(
            self.theme,
            if focused { Token::Text } else { Token::Border },
        );
        let inner = usize::from(area.width.saturating_sub(2));
        let horizontal = "\u{2500}".repeat(inner);
        buf.set_string(
            area.x,
            area.y,
            format!("\u{250c}{horizontal}\u{2510}"),
            border,
        );
        buf.set_string(
            area.x,
            area.bottom() - 1,
            format!("\u{2514}{horizontal}\u{2518}"),
            border,
        );
        for y in area.y + 1..area.bottom() - 1 {
            buf.set_string(area.x, y, "\u{2502}", border);
            if area.width > 1 {
                buf.set_string(area.right() - 1, y, "\u{2502}", border);
            }
        }
        if area.width < 6 {
            return;
        }
        // Text starts two cells inside the borders: the focus mark, then a
        // space, and keeps one cell clear of the right border.
        let subject = rect_of(layout.subject);
        let text = Rect {
            x: subject.x + 2,
            width: subject.width - 3,
            ..subject
        };
        self.draw_subject(buf, text);
        let cursor_line = self.view.body.cursor.map_or(0, |(line, _)| line);
        let top = cursor_line.saturating_sub(usize::from(body_rows.max(1)) - 1);
        for row in 0..body_rows {
            let rect = Rect {
                y: layout.body.y + row,
                ..text
            };
            self.draw_body_row(buf, rect, top + usize::from(row), row == 0);
        }
    }

    /// The subject, bold, with its placeholder when empty and the counter
    /// at the right end once anything is typed (C1, C2).
    fn draw_subject(&self, buf: &mut Buffer, area: Rect) {
        let subject = &self.view.subject;
        if subject.focused {
            buf.set_string(
                area.x - 1,
                area.y,
                FOCUS.to_string(),
                foreground(self.theme, Token::Text).add_modifier(Modifier::BOLD),
            );
        }
        let typed = subject.counter.map_or(0, |(typed, _)| typed);
        let counter = subject
            .counter
            .filter(|(typed, _)| *typed > 0)
            .map(|(typed, limit)| (format!("{typed}/{limit}"), typed > limit));
        let counter_width = counter
            .as_ref()
            .map_or(0, |(text, _)| u16::try_from(text.width() + 1).unwrap_or(0));
        let text_width = area.width.saturating_sub(counter_width);
        let text_area = Rect {
            width: text_width,
            ..area
        };
        if typed == 0 {
            buf.set_string(
                area.x,
                area.y,
                fit(SUBJECT_PLACEHOLDER, usize::from(text_width)),
                foreground(self.theme, Token::TextMuted),
            );
            if subject.cursor.is_some() {
                reverse(buf, area.x, area.y);
            }
        } else {
            draw_line(
                buf,
                text_area,
                subject.text.as_str(),
                subject.cursor,
                None,
                self.theme,
            );
            buf.set_style(text_area, Style::new().add_modifier(Modifier::BOLD));
        }
        if let Some((text, over)) = counter {
            let token = if over {
                Token::Warning
            } else {
                Token::TextMuted
            };
            buf.set_string(
                area.x + text_width + 1,
                area.y,
                fit(&text, usize::from(counter_width.saturating_sub(1))),
                foreground(self.theme, token),
            );
        }
    }

    /// One body row: line `line` of the body, its placeholder on the first
    /// row while the body is empty (C3).
    fn draw_body_row(&self, buf: &mut Buffer, area: Rect, line: usize, first: bool) {
        let body = &self.view.body;
        let empty = body.lines.iter().all(|line| line.as_str().is_empty());
        let cursor = body
            .cursor
            .filter(|(at, _)| *at == line)
            .map(|(_, column)| column);
        if body.focused && cursor.is_some() {
            buf.set_string(
                area.x - 1,
                area.y,
                FOCUS.to_string(),
                foreground(self.theme, Token::Text).add_modifier(Modifier::BOLD),
            );
        }
        if empty && first {
            buf.set_string(
                area.x,
                area.y,
                fit(BODY_PLACEHOLDER, usize::from(area.width)),
                foreground(self.theme, Token::TextMuted),
            );
            if cursor.is_some() {
                reverse(buf, area.x, area.y);
            }
            return;
        }
        let Some(text) = body.lines.get(line) else {
            return;
        };
        draw_line(buf, area, text.as_str(), cursor, body.guide_at, self.theme);
    }

    /// The one sentence above the files (C4).
    fn draw_summary(&self, buf: &mut Buffer, row: Rect) {
        let token = match self.view.summary {
            Summary::Nothing => Token::Warning,
            Summary::Goes { .. } | Summary::MessageOnly { .. } => Token::Text,
        };
        buf.set_string(
            row.x,
            row.y,
            fit(
                &format!(" {}", summary_line(&self.view.summary)),
                usize::from(row.width),
            ),
            foreground(self.theme, token),
        );
    }
}

/// The text cursor over a placeholder's first cell: the placeholder stays
/// readable and the cursor shows where typing starts.
fn reverse(buf: &mut Buffer, x: u16, y: u16) {
    buf[(x, y)].set_style(Style::new().add_modifier(Modifier::REVERSED));
}
