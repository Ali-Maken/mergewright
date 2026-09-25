//! The finish summary's file list: the section headings, the rows with
//! their tinted words, and the diff expanded under a row. Where each line
//! stands is `layout`'s arithmetic; this only draws it.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use super::{
    DIFF_INDENT, ExpandedDiff, FinishScreen, ListLine, Section, SummaryFocus, SummaryState,
    list_lines,
};
use crate::detail_pane::{DiffFocus, diff_body};
use crate::fit::fit;
use crate::paint::foreground;
use crate::text_input::FOCUS;
use crate::theme::Token;

impl FinishScreen<'_> {
    /// The cursor glyph.
    fn focus_glyph(&self) -> char {
        if self.ascii { '>' } else { FOCUS }
    }

    /// The file list from `view.top`, into `rect`.
    pub(super) fn draw_list(&self, buf: &mut Buffer, rect: Rect) {
        let lines = list_lines(self.view);
        let path_width = self
            .view
            .by_you
            .iter()
            .chain(&self.view.by_git)
            .map(|row| row.path.as_str().chars().count())
            .max()
            .unwrap_or(0);
        let mut y = rect.y;
        let mut at = self.view.top;
        while y < rect.bottom() {
            let Some(line) = lines.get(at) else {
                break;
            };
            let one = Rect {
                y,
                height: 1,
                ..rect
            };
            match *line {
                ListLine::Status => self.draw_status(buf, one),
                ListLine::Section(section) => {
                    let words = self.section_words(section);
                    self.text(buf, one, &words, Token::TextMuted, true);
                }
                ListLine::Row(index) => self.draw_row(buf, one, index, path_width),
                ListLine::More { hidden, .. } => {
                    let indented = indent(one);
                    let words = format!("… {hidden} more lines · ⏎ hides");
                    self.text(buf, indented, &words, Token::TextMuted, false);
                }
                ListLine::Diff { nth, .. } => {
                    // The whole visible run of this diff in one call, so a
                    // hunk heading is drawn where `diff_body` puts it.
                    let run = lines[at..]
                        .iter()
                        .take_while(|line| matches!(line, ListLine::Diff { .. }))
                        .count();
                    let height = run.min(usize::from(rect.bottom() - y));
                    let height = u16::try_from(height).unwrap_or(u16::MAX);
                    self.draw_diff(
                        buf,
                        Rect {
                            height,
                            ..indent(one)
                        },
                        nth,
                    );
                    at += usize::from(height);
                    y += height;
                    continue;
                }
            }
            at += 1;
            y += 1;
        }
    }

    /// The list's one line while there are no lists.
    fn draw_status(&self, buf: &mut Buffer, rect: Rect) {
        match &self.view.state {
            SummaryState::Loading | SummaryState::Ready => {
                self.text(
                    buf,
                    rect,
                    "reading what this merge changes…",
                    Token::TextMuted,
                    false,
                );
            }
            SummaryState::Unavailable(why) => {
                let words = format!(
                    "summary unavailable: {} · y commits with git's message",
                    why.as_str()
                );
                self.text(buf, rect, &words, Token::Warning, false);
            }
        }
    }

    /// A section heading's words.
    fn section_words(&self, section: Section) -> String {
        match section {
            Section::ByYou => format!("RESOLVED BY YOU · {}", self.view.by_you.len()),
            Section::ByGit if self.view.by_git.is_empty() => {
                "COMBINED BY GIT ON ITS OWN · none".to_owned()
            }
            Section::ByGit => format!(
                "COMBINED BY GIT ON ITS OWN · {} · ⏎ shows the change",
                self.view.by_git.len()
            ),
            Section::Unrecorded => format!(
                "CHANGED BY THIS MERGE · {} · which were conflicts is not recorded",
                self.view.by_git.len()
            ),
        }
    }

    /// `▸ path  what`, the path padded to the widest one.
    fn draw_row(&self, buf: &mut Buffer, rect: Rect, index: usize, path_width: usize) {
        let Some(row) = self.view.row(index) else {
            return;
        };
        let cursor = index == self.view.cursor;
        let marker = if cursor { self.focus_glyph() } else { ' ' };
        let path = row.path.as_str();
        let pad = path_width.saturating_sub(path.chars().count());
        let line = format!("{marker} {path}{}  {}", " ".repeat(pad), row.what.as_str());
        let token = if cursor && self.view.focus == SummaryFocus::Files {
            Token::Text
        } else {
            Token::TextMuted
        };
        let mut style = foreground(self.theme, token);
        if cursor {
            style = style.bg(crate::paint::color(self.theme.token(Token::Selection)));
        }
        buf.set_string(rect.x, rect.y, fit(&line, usize::from(rect.width)), style);
        // The words take the colours that tie them back to the panes and to
        // diffs; the characters already say it all (rule 5).
        let before = crate::frame_box::cells(&format!("{marker} {path}{}  ", " ".repeat(pad)));
        let what = row.what.as_str();
        for (start, end, token) in tints(what) {
            let x = before + crate::frame_box::cells(&what[..start]);
            let width = crate::frame_box::cells(&what[start..end]);
            let x = rect.x.saturating_add(u16::try_from(x).unwrap_or(u16::MAX));
            if x >= rect.right() {
                continue;
            }
            let width = u16::try_from(width)
                .unwrap_or(u16::MAX)
                .min(rect.right() - x);
            buf.set_style(
                Rect {
                    x,
                    y: rect.y,
                    width,
                    height: 1,
                },
                Style::new().fg(crate::paint::color(self.theme.token(token))),
            );
        }
    }

    /// The expanded diff's lines from `nth`, into `rect`.
    fn draw_diff(&self, buf: &mut Buffer, rect: Rect, nth: usize) {
        let Some((_, diff)) = &self.view.expanded else {
            return;
        };
        match diff {
            ExpandedDiff::Loading => {
                self.text(buf, rect, "reading the diff…", Token::TextMuted, false);
            }
            ExpandedDiff::Failed(why) => {
                self.text(buf, rect, why.as_str(), Token::Warning, false);
            }
            ExpandedDiff::Ready(view) => match &view.binary {
                Some(note) => self.text(buf, rect, note.as_str(), Token::TextMuted, false),
                None if view.hunks.is_empty() => {
                    self.text(buf, rect, "no change to show", Token::TextMuted, false);
                }
                // `truncated: None`: the summary draws its own `… N more
                // lines · space hides`, and `L load all` is not one of its
                // keys (ticket 19 S5). The row count `diff_body` returns is
                // for scroll clamping; `list_lines` already counted the rows
                // (`diff_extent`), so it is not needed here.
                None => {
                    let _rows: usize = diff_body(
                        &view.hunks,
                        nth,
                        None,
                        rect,
                        buf,
                        self.theme,
                        DiffFocus {
                            focused: false,
                            word_tint: true,
                            cursor: None,
                        },
                    );
                }
            },
        }
    }
}

/// Byte ranges of a row's words that take a colour: `yours · <branch>` in
/// `ours`, `theirs · <branch>` in `theirs`, `+n` in `added`, `−n` in
/// `removed`.
fn tints(what: &str) -> Vec<(usize, usize, Token)> {
    let mut found = Vec::new();
    for (lead, token) in [("yours · ", Token::Ours), ("theirs · ", Token::Theirs)] {
        let mut from = 0;
        while let Some(at) = what[from..].find(lead).map(|at| at + from) {
            let name = at + lead.len();
            let end = what[name..]
                .find([' ', ','])
                .map_or(what.len(), |stop| name + stop);
            found.push((at, end, token));
            from = end;
        }
    }
    for (sign, token) in [('+', Token::Added), ('\u{2212}', Token::Removed)] {
        for (at, _) in what.match_indices(sign) {
            let digits = what[at + sign.len_utf8()..]
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(what.len() - at - sign.len_utf8());
            if digits > 0 {
                found.push((at, at + sign.len_utf8() + digits, token));
            }
        }
    }
    found
}

/// `rect` moved right by [`DIFF_INDENT`].
fn indent(rect: Rect) -> Rect {
    let by = DIFF_INDENT.min(rect.width);
    Rect {
        x: rect.x + by,
        width: rect.width - by,
        ..rect
    }
}
