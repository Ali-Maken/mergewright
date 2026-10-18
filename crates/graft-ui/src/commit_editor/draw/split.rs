//! The lower half of the commit screen: the rule, the file list and the
//! cursor file's changes beside it.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use super::super::words::{PART, PART_NOTE, SECRET_WHY, counts, mark, secret_note};
use super::super::{CommitFocus, CommitScreen, DIFF_COLUMNS, FILES_WIDTH, FileGroup};
use super::CURSOR_AWAY;
use crate::detail::BodyView;
use crate::detail_pane::{DiffFocus, diff_body};
use crate::fit::fit;
use crate::paint::foreground;
use crate::text_input::FOCUS;
use crate::theme::Token;

impl CommitScreen<'_> {
    /// The rule, then `files | diff` (files alone under [`DIFF_COLUMNS`]).
    pub(super) fn draw_split(&self, buf: &mut Buffer, area: Rect) {
        let rule = foreground(self.theme, Token::Border);
        let with_diff = area.width >= DIFF_COLUMNS;
        let files_width = if with_diff { FILES_WIDTH } else { area.width };
        let mut line = "\u{2500}".repeat(usize::from(area.width));
        if with_diff {
            let at = line
                .char_indices()
                .nth(usize::from(files_width))
                .map_or(line.len(), |(at, _)| at);
            line.replace_range(at..at + '\u{2500}'.len_utf8(), "\u{252c}");
        }
        buf.set_string(area.x, area.y, line, rule);
        if area.height < 2 {
            return;
        }
        let body = Rect {
            y: area.y + 1,
            height: area.height - 1,
            ..area
        };
        self.draw_files(
            buf,
            Rect {
                width: files_width,
                ..body
            },
        );
        if !with_diff {
            return;
        }
        for y in body.y..body.bottom() {
            buf.set_string(body.x + files_width, y, "\u{2502}", rule);
        }
        let diff_x = body.x + files_width + 2;
        self.draw_diff(
            buf,
            Rect {
                x: diff_x,
                width: body.right().saturating_sub(diff_x + 1),
                ..body
            },
        );
    }

    /// `Going in (n)` and its rows, then `Staying out (n)` and its rows,
    /// scrolled so the cursor's row is on screen (C5, C7).
    fn draw_files(&self, buf: &mut Buffer, area: Rect) {
        let mut lines: Vec<(String, Style, Option<usize>)> = Vec::new();
        let muted = foreground(self.theme, Token::TextMuted);
        for (group, heading, none) in [
            (FileGroup::GoingIn, "Going in", "nothing yet"),
            (FileGroup::StayingOut, "Staying out", "nothing"),
        ] {
            let members: Vec<usize> = (0..self.view.files.len())
                .filter(|at| self.view.files[*at].group == group)
                .collect();
            lines.push((format!(" {heading} ({})", members.len()), muted, None));
            if members.is_empty() {
                lines.push((format!("   {none}"), muted, None));
            }
            for at in members {
                lines.push((
                    self.file_line(at, area.width),
                    self.file_style(at),
                    Some(at),
                ));
            }
        }
        let cursor_line = lines
            .iter()
            .position(|(_, _, at)| *at == Some(self.view.cursor))
            .unwrap_or(0);
        let height = usize::from(area.height);
        let top = cursor_line.saturating_sub(height.saturating_sub(1));
        for (row, (text, style, at)) in lines.iter().skip(top).take(height).enumerate() {
            let y = area.y + u16::try_from(row).unwrap_or(0);
            let text = fit(text, usize::from(area.width));
            buf.set_string(area.x, y, &text, *style);
            // A staged part's word is dim whatever the row's colour: it
            // says how much goes in, not what the file is (08s M3).
            let part = at.is_some_and(|at| self.view.files[at].part_only);
            if part && text.trim_end().ends_with(PART) {
                let from = text.trim_end().width().saturating_sub(PART.width());
                let x = area.x + u16::try_from(from).unwrap_or(u16::MAX);
                buf.set_string(x, y, PART, foreground(self.theme, Token::TextMuted));
            }
        }
    }

    /// `▸✓ M src/receipt.rs                 +12 −3`: cursor mark, in/out
    /// mark, change letter, path (and why it stays out), counts at the end.
    fn file_line(&self, at: usize, width: u16) -> String {
        let file = &self.view.files[at];
        let cursor = if at == self.view.cursor {
            if self.view.focus == CommitFocus::Files {
                FOCUS
            } else {
                CURSOR_AWAY
            }
        } else {
            ' '
        };
        let head = format!(" {cursor}{} {} ", mark(file), file.kind);
        let room_beside = |tail: &str| {
            usize::from(width)
                .saturating_sub(head.width())
                .saturating_sub(if tail.is_empty() { 1 } else { tail.width() + 2 })
        };
        let why = (file.secret && file.group == FileGroup::StayingOut)
            .then(|| format!("  {SECRET_WHY}"))
            .filter(|why| file.path.as_str().width() + why.width() <= room_beside(""))
            .unwrap_or_default();
        // Why it stays out outranks its counts: the diff shows those too.
        let part = if file.part_only { PART } else { "" };
        let tail = format!("{}{part}", counts(file));
        let fits = file.path.as_str().width() + why.width() <= room_beside(&tail);
        let tail = if fits || why.is_empty() {
            tail
        } else {
            String::new()
        };
        let room = room_beside(&tail);
        let middle = fit(&format!("{}{why}", file.path.as_str()), room);
        let pad =
            usize::from(width).saturating_sub(head.width() + middle.width() + tail.width() + 1);
        format!("{head}{middle}{}{tail}", " ".repeat(pad))
    }

    /// Going in reads as text; staying out is dim, a secrets file's in the
    /// warning colour.
    fn file_style(&self, at: usize) -> Style {
        let file = &self.view.files[at];
        let token = match file.group {
            FileGroup::GoingIn => Token::Text,
            FileGroup::StayingOut if file.secret => Token::Warning,
            FileGroup::StayingOut => Token::TextMuted,
        };
        foreground(self.theme, token)
    }

    /// The note over the cursor file's changes, then a blank row: C7's for
    /// a secrets file staying out, or what a staged part leaves behind (08s
    /// M3), in the same place and colour. The area left for its changes,
    /// `None` when none is.
    fn draw_note(&self, buf: &mut Buffer, area: Rect) -> Option<Rect> {
        let Some(file) = self.view.files.get(self.view.cursor) else {
            return Some(area);
        };
        let note = if file.secret && file.group == FileGroup::StayingOut {
            secret_note()
        } else if file.part_only {
            PART_NOTE.to_owned()
        } else {
            return Some(area);
        };
        buf.set_string(
            area.x,
            area.y,
            fit(&note, usize::from(area.width)),
            foreground(self.theme, Token::Warning),
        );
        (area.height >= 3).then(|| Rect {
            y: area.y + 2,
            height: area.height - 2,
            ..area
        })
    }

    /// The diff of the file under the cursor (C8), under its note
    /// ([`Self::draw_note`]).
    fn draw_diff(&self, buf: &mut Buffer, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let Some(area) = self.draw_note(buf, area) else {
            return;
        };
        let say = |buf: &mut Buffer, words: &str| {
            buf.set_string(
                area.x,
                area.y,
                fit(words, usize::from(area.width)),
                foreground(self.theme, Token::TextMuted),
            );
        };
        match &self.view.diff {
            BodyView::Diff(diff) => {
                if let Some(binary) = &diff.binary {
                    say(buf, binary.as_str());
                    return;
                }
                // The row count is for a scroll this screen does not have:
                // the diff shows from its top. Nothing is lost by dropping it.
                let _rows: usize = diff_body(
                    &diff.hunks,
                    0,
                    diff.truncated
                        .map(|(shown, total)| (shown, total, self.keys)),
                    area,
                    buf,
                    self.theme,
                    DiffFocus {
                        focused: false,
                        word_tint: false,
                        cursor: None,
                    },
                );
            }
            BodyView::Empty(words) | BodyView::Loading(words) | BodyView::Failed(words) => {
                say(buf, words.as_str());
            }
            BodyView::Message { lines } => {
                for (row, line) in lines.iter().take(usize::from(area.height)).enumerate() {
                    buf.set_string(
                        area.x,
                        area.y + u16::try_from(row).unwrap_or(0),
                        fit(line.as_str(), usize::from(area.width)),
                        foreground(self.theme, Token::Text),
                    );
                }
            }
        }
    }
}
