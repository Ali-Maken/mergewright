//! The commit screen's layout (design note "New UI"): header, message box
//! (subject 1, body 3), summary line, then `files 44 | diff` — files alone
//! under [`DIFF_COLUMNS`] — and the bar at the bottom.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use super::words::{bar, header_line, summary_line, whole_parts};
use super::{CommitScreen, Summary};
use crate::fit::fit;
use crate::paint::foreground;
use crate::text_input::{FOCUS, draw_line};
use crate::theme::Token;

mod split;

/// The subject's placeholder (C1).
pub(super) const SUBJECT_PLACEHOLDER: &str = "What did you change? (one line)";
/// The body's placeholder (C3).
pub(super) const BODY_PLACEHOLDER: &str = "More detail, if it needs it (optional)";

/// The file cursor's mark while the files do not have focus: the row the
/// diff shows, with the keys elsewhere.
const CURSOR_AWAY: char = '\u{203a}';

/// Body rows inside the message box.
const BODY_ROWS: u16 = 3;

/// Rows the screen takes before the file list: header, the box's two
/// borders, the subject, the summary and the rule over the files.
const CHROME: u16 = 6;

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
        let width = usize::from(area.width);
        buf.set_string(
            area.x,
            area.y,
            fit(&format!(" {}", header_line(&self.view.header)), width),
            foreground(self.theme, Token::Text).add_modifier(Modifier::BOLD),
        );
        if area.height == 1 {
            return;
        }
        let bar_row = area.bottom() - 1;
        self.draw_bar(buf, band(area, bar_row, 1));
        // Rows between the header and the bar.
        let room = area.height - 2;
        // The body gives way first, a row at a time, keeping one file row.
        let body_rows = BODY_ROWS.min(room.saturating_sub(CHROME));
        let mut y = area.y + 1;
        let box_rows = 3 + body_rows;
        if room < box_rows {
            return;
        }
        self.draw_message_box(buf, band(area, y, box_rows), body_rows);
        y += box_rows;
        if y >= bar_row {
            return;
        }
        self.draw_summary(buf, band(area, y, 1));
        y += 1;
        if y >= bar_row {
            return;
        }
        self.draw_split(buf, band(area, y, bar_row - y));
    }
}

impl CommitScreen<'_> {
    /// The bar, or the failure that takes its row (hook fails: C13's
    /// "the failure on its status row").
    fn draw_bar(&self, buf: &mut Buffer, row: Rect) {
        let (line, token) = match &self.view.status {
            Some(status) => {
                let dismiss = self
                    .keys
                    .key_verb(&[crate::keymap::ActionId("app.back")], "dismisses");
                let line = match dismiss {
                    Some(dismiss) => format!("{} \u{b7} {dismiss}", status.as_str()),
                    None => status.as_str().to_owned(),
                };
                (line, Token::Warning)
            }
            // An operation went live while the screen was up: `⏎` would be
            // refused, so its words — where the operation is finished —
            // take the bar's row (m6-conflicts/48).
            None => match &self.view.live {
                Some(live) => (live.as_str().to_owned(), Token::Warning),
                // Whole keys or none: the bar drops its last parts first.
                None => (
                    whole_parts(
                        &bar(self.view, self.keys),
                        usize::from(row.width).saturating_sub(1),
                    ),
                    Token::TextMuted,
                ),
            },
        };
        buf.set_string(
            row.x,
            row.y,
            fit(&format!(" {line}"), usize::from(row.width)),
            foreground(self.theme, token),
        );
    }

    /// `┌─┐`, the subject with its `n/72`, the body rows, `└─┘`.
    fn draw_message_box(&self, buf: &mut Buffer, area: Rect, body_rows: u16) {
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
        // Text starts two cells in: the focus mark, then a space.
        let text = Rect {
            x: area.x + 3,
            y: area.y + 1,
            width: area.width - 5,
            height: 1,
        };
        self.draw_subject(buf, text);
        let cursor_line = self.view.body.cursor.map_or(0, |(line, _)| line);
        let top = cursor_line.saturating_sub(usize::from(body_rows.max(1)) - 1);
        for row in 0..body_rows {
            let rect = Rect {
                y: text.y + 1 + row,
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

/// `height` rows of `area` from row `y`, its full width.
fn band(area: Rect, y: u16, height: u16) -> Rect {
    Rect { y, height, ..area }
}

/// The text cursor over a placeholder's first cell: the placeholder stays
/// readable and the cursor shows where typing starts.
fn reverse(buf: &mut Buffer, x: u16, y: u16) {
    buf[(x, y)].set_style(Style::new().add_modifier(Modifier::REVERSED));
}
