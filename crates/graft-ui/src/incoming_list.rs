//! The commits a merge brings in (ease-pass 10, mock
//! `mergewright-merge-screen.html` l.181–188): `⏎` from the merge preview.
//!
//! ```text
//! The 3 commits that come in from sara/fix-toc
//!
//! ●  Fix the table of contents links        Sara  2h
//! ●  Number the report sections             Omar  5h
//! ●  Report: move the summary up            Sara  1d
//! ```
//!
//! Newest first, one row per commit, the cursor's row reversed; docked
//! where the preview box was, in the same frame. Its footer is
//! `esc back to the preview  y merge` (`y` left out when withheld).

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::frame_box::{border_row, cells, content_row};
use crate::graph_column::NodeKind;
use crate::keymap::{ActionId, Keymap};
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// One incoming commit, as the list draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncomingRow {
    /// The subject line.
    pub subject: SanitizedText,
    /// The author's first name.
    pub who: SanitizedText,
    /// Its age, in the graph's own format (`2h`).
    pub age: SanitizedText,
    /// A merge commit: the graph's merge glyph.
    pub is_merge: bool,
}

/// The list's state: what it draws and where its cursor is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncomingView {
    /// The side that comes in: `sara/fix-toc`.
    pub from: SanitizedText,
    /// Newest first.
    pub rows: Vec<IncomingRow>,
    /// More came in than were read: a last `…` row.
    pub capped: bool,
    /// `y` is withheld (unrelated histories): the footer leaves it out.
    pub withheld: bool,
    /// The row under the cursor.
    pub cursor: usize,
    /// Draw the ASCII tier's node glyph (`*`).
    pub ascii: bool,
}

impl IncomingView {
    /// `The 3 commits that come in from sara/fix-toc`.
    #[must_use]
    pub fn header(&self) -> String {
        let n = self.rows.len();
        let plus = if self.capped { "+" } else { "" };
        let (noun, verb) = if n == 1 && !self.capped {
            ("commit", "comes")
        } else {
            ("commits", "come")
        };
        format!(
            "The {n}{plus} {noun} that {verb} in from {}",
            self.from.as_str()
        )
    }

    /// Moves the cursor one row down, clamped at the last.
    pub fn down(&mut self) {
        self.cursor = self
            .cursor
            .saturating_add(1)
            .min(self.rows.len().saturating_sub(1));
    }

    /// Moves the cursor one row up, clamped at the first.
    pub fn up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Rows the list wants: borders, header, a blank, one per commit and
    /// the `…` row when capped.
    #[must_use]
    pub fn rows_wanted(&self) -> u16 {
        let rows = 2 + 2 + self.rows.len() + usize::from(self.capped);
        u16::try_from(rows).unwrap_or(u16::MAX)
    }

    /// `esc back to the preview  y merge`, the widest that fits `inner`.
    fn footer(&self, inner: usize, keys: &Keymap) -> String {
        let back = keys.key_verb(&[ActionId("app.back")], "back to the preview");
        let merge = if self.withheld {
            None
        } else {
            keys.key_verb(&[ActionId("confirm.yes")], "merge")
        };
        let full = [back.clone(), merge]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("  ");
        if cells(&full) + 2 <= inner {
            full
        } else {
            back.unwrap_or_default()
        }
    }
}

/// [`IncomingView`] drawn into the rect the preview box was docked to.
pub struct IncomingList<'a> {
    /// What to draw.
    pub view: &'a IncomingView,
    /// Colours.
    pub theme: &'a Theme,
    /// His keymap: the footer's keys are spelled from it.
    pub keys: &'a Keymap,
}

impl IncomingList<'_> {
    /// One commit's row: glyph, subject, who, age, columns aligned.
    /// The cursor's row is marked `▸`, a cue that survives a capture
    /// (CLAUDE.md rule 5); the others carry a space so nothing shifts.
    fn row_text(&self, row: &IncomingRow, at: bool, widths: (usize, usize, usize)) -> String {
        let (inner, who_w, age_w) = widths;
        let kind = if row.is_merge {
            NodeKind::Merge
        } else {
            NodeKind::Commit
        };
        let glyph = if self.view.ascii { '*' } else { kind.glyph() };
        let mark = if at { '\u{25b8}' } else { ' ' };
        let subject_w = inner.saturating_sub(4 + 2 + who_w + 2 + age_w);
        format!(
            "{mark}{glyph}  {}  {}  {}",
            fit(row.subject.as_str(), subject_w),
            fit(row.who.as_str(), who_w),
            format_args!("{:>age_w$}", row.age.as_str()),
        )
    }
}

impl Widget for &IncomingList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 8 || area.height < 3 {
            return;
        }
        let width = usize::from(area.width);
        let inner = width.saturating_sub(4);
        let text = foreground(self.theme, Token::Text);
        let muted = foreground(self.theme, Token::TextMuted);
        let view = self.view;
        for y in area.top()..area.bottom() {
            buf.set_stringn(area.x, y, " ".repeat(width), width, Style::default());
        }
        buf.set_string(area.x, area.y, border_row('╭', '╮', "", width), text);
        let who_w = view.rows.iter().map(|r| cells(r.who.as_str())).max();
        let age_w = view.rows.iter().map(|r| cells(r.age.as_str())).max();
        let (who_w, age_w) = (who_w.unwrap_or(0), age_w.unwrap_or(0));
        let mut body: Vec<(String, Style)> = vec![(view.header(), muted), (String::new(), text)];
        let room = usize::from(area.height.saturating_sub(2));
        // Scrolled so the cursor's row is always on screen.
        let shown = room.saturating_sub(body.len()).max(1);
        let top = view.cursor.saturating_sub(shown.saturating_sub(1));
        let cursor_at = body.len() + view.cursor - top;
        for (index, row) in view.rows.iter().enumerate().skip(top) {
            let at = index == view.cursor;
            body.push((self.row_text(row, at, (inner, who_w, age_w)), text));
        }
        if view.capped {
            body.push(("…".to_owned(), muted));
        }
        let first = area.y.saturating_add(1);
        for (index, (line, style)) in body.iter().take(room).enumerate() {
            let y = first.saturating_add(u16::try_from(index).unwrap_or(u16::MAX));
            let style = if index == cursor_at {
                style.add_modifier(Modifier::REVERSED)
            } else {
                *style
            };
            buf.set_string(area.x, y, content_row(line, width), style);
        }
        for index in body.len()..room {
            let y = first.saturating_add(u16::try_from(index).unwrap_or(u16::MAX));
            buf.set_string(area.x, y, content_row("", width), text);
        }
        buf.set_string(
            area.x,
            area.bottom().saturating_sub(1),
            border_row('╰', '╯', &view.footer(width.saturating_sub(2), self.keys), width),
            muted,
        );
    }
}

#[cfg(test)]
mod tests;
