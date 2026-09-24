//! The finish summary (`c` on a merge, m6-conflicts/19): what will land —
//! the files the user resolved, the files git combined on its own, and the
//! message — before `⏎` commits it
//! (`docs/design-notes/m6-conflicts.md` Amendment 2, "Finish summary").
//!
//! Owns no state and reads no repository. `graft::update::finish` holds the
//! state and `graft::view::finish` turns it into the [`FinishView`] this
//! draws. The layout is pure arithmetic over the view and the area
//! ([`layout`], [`list_lines`]), and the app's keys and mouse ask the same
//! functions where a row is ([`hit`], [`top_for_cursor`]), so what is drawn
//! and what a click lands on cannot disagree.
//!
//! Every fact is in **characters**: the cursor row is `▸`, a problem notice
//! says so in words. A capture records the grid and nothing else
//! (CLAUDE.md rule 5); colour ships unverified.

use std::sync::Arc;

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::detail::{DiffView, LineKindView};
use crate::detail_pane::{DiffFocus, diff_body};
use crate::fit::fit;
use crate::paint::foreground;
use crate::text_input::{AreaView, AreaWidget, FOCUS, FieldView, FieldWidget};
use crate::theme::{Theme, Token};

/// The smallest area the summary draws in, `(columns, rows)`. Below it the
/// widget draws one line, `finish needs 60x14 · esc back`, and the app
/// answers only `Esc` (and `Ctrl+C`).
pub const FINISH_MIN: (u16, u16) = (60, 14);

/// From this width the file lists take the left 60 % and the message the
/// right; below it, one column.
pub const TWO_COLUMN_MIN: u16 = 160;

/// The most rows an expanded diff draws under its row. More than that adds
/// one row, `… N more lines · space hides`.
pub const DIFF_ROWS_MAX: usize = 12;

/// Cells an expanded diff is indented by, under its row.
const DIFF_INDENT: u16 = 4;

/// Rows the message block needs at the least in one column: `MESSAGE`, the
/// subject, the body's label and one body line.
const MESSAGE_MIN_ROWS: u16 = 4;

/// One file row: its path and what the summary says about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryRow {
    /// The path.
    pub path: SanitizedText,
    /// `theirs · feature in 2 of 2 conflicts`, `modified  +1 −1`, ….
    pub what: SanitizedText,
}

/// What has the keyboard in the summary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SummaryFocus {
    /// The file lists.
    Files,
    /// The subject field.
    Subject,
    /// The body area.
    Body,
}

/// The diff drawn under an expanded row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpandedDiff {
    /// Asked for, not arrived.
    Loading,
    /// The read failed, and why: drawn in place of the diff.
    Failed(SanitizedText),
    /// Arrived.
    Ready(Arc<DiffView>),
}

/// Whether the summary itself has been read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SummaryState {
    /// `reading what this merge changes…`.
    Loading,
    /// The lists are the summary's.
    Ready,
    /// The read failed: `summary unavailable: <why> · ⏎ commits with git's
    /// message`.
    Unavailable(SanitizedText),
}

/// Everything the summary draws.
#[derive(Clone, Debug)]
pub struct FinishView<'a> {
    /// `Finish · merging feature → main · nothing is written until ⏎`.
    pub title: SanitizedText,
    /// The files the merge conflicted on, worded by what the user chose.
    pub by_you: Vec<SummaryRow>,
    /// Every other staged change: the files git combined on its own.
    pub by_git: Vec<SummaryRow>,
    /// Whether it is known which files were conflicts. `false`: one section,
    /// `CHANGED BY THIS MERGE`, and every row is in `by_git`.
    pub recorded: bool,
    /// The cursor row, counted over `by_you` then `by_git`.
    pub cursor: usize,
    /// The row whose diff is shown under it, and the diff.
    pub expanded: Option<(usize, ExpandedDiff)>,
    /// The subject field.
    pub subject: FieldView<'a>,
    /// The body area.
    pub body: AreaView<'a>,
    /// What has the keyboard.
    pub focus: SummaryFocus,
    /// A line above the bar, and whether it is a problem.
    pub notice: Option<(SanitizedText, bool)>,
    /// The commit is running.
    pub busy: bool,
    /// Whether the summary has been read.
    pub state: SummaryState,
    /// The first list line on screen.
    pub top: usize,
}

impl FinishView<'_> {
    /// Rows in both lists together: what the cursor counts over.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.by_you.len() + self.by_git.len()
    }

    /// The row the cursor index names.
    fn row(&self, index: usize) -> Option<&SummaryRow> {
        self.by_you
            .get(index)
            .or_else(|| self.by_git.get(index.checked_sub(self.by_you.len())?))
    }
}

/// [`FinishView`] drawn into the area under the header and above the bar.
#[derive(Debug)]
pub struct FinishScreen<'a> {
    /// What to draw.
    pub view: &'a FinishView<'a>,
    /// Colours.
    pub theme: &'a Theme,
    /// The ASCII renderer tier: `>` for the cursor, `->` in the title.
    pub ascii: bool,
}

impl Widget for &FinishScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        // The summary replaces the graph and the detail pane both, so it
        // owns its background.
        buf.set_style(
            area,
            Style::new().bg(crate::paint::color(self.theme.token(Token::Bg))),
        );
        let Some(placed) = layout(self.view, area) else {
            let (columns, rows) = FINISH_MIN;
            buf.set_string(
                area.x,
                area.y,
                fit(
                    &format!("finish needs {columns}x{rows} · esc back"),
                    usize::from(area.width),
                ),
                foreground(self.theme, Token::Warning),
            );
            return;
        };
        let title = self.ascii_safe(self.view.title.as_str());
        self.text(buf, placed.title, &title, Token::Accent, true);
        self.draw_list(buf, placed.list);
        self.draw_message(buf, &placed);
        if let (Some(rect), Some((words, problem))) = (placed.notice, &self.view.notice) {
            let token = if *problem {
                Token::Warning
            } else {
                Token::TextMuted
            };
            self.text(buf, rect, words.as_str(), token, false);
        }
    }
}

impl FinishScreen<'_> {
    /// `text` on one row of `rect`, fitted to its width.
    fn text(&self, buf: &mut Buffer, rect: Rect, text: &str, token: Token, bold: bool) {
        let mut style = foreground(self.theme, token);
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        buf.set_string(rect.x, rect.y, fit(text, usize::from(rect.width)), style);
    }

    /// The title's arrow at the ASCII tier.
    fn ascii_safe(&self, text: &str) -> String {
        if self.ascii {
            text.replace('→', "->")
        } else {
            text.to_owned()
        }
    }

    /// The cursor glyph.
    fn focus_glyph(&self) -> char {
        if self.ascii { '>' } else { FOCUS }
    }

    /// The file list from `view.top`, into `rect`.
    fn draw_list(&self, buf: &mut Buffer, rect: Rect) {
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
                    let words = format!("… {hidden} more lines · space hides");
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
                    "summary unavailable: {} · ⏎ commits with git's message",
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
                "COMBINED BY GIT ON ITS OWN · {} · space shows the change",
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

    /// `MESSAGE · ⇥ edit`, the subject and the body.
    fn draw_message(&self, buf: &mut Buffer, placed: &FinishLayout) {
        if placed.message.height == 0 {
            return;
        }
        self.text(
            buf,
            placed.message,
            "MESSAGE · ⇥ edit",
            Token::TextMuted,
            true,
        );
        if placed.subject.height > 0 {
            (&FieldWidget {
                view: &self.view.subject,
                theme: self.theme,
                ascii: self.ascii,
            })
                .render(placed.subject, buf);
        }
        if placed.body.height > 0 {
            (&AreaWidget {
                view: &self.view.body,
                theme: self.theme,
            })
                .render(placed.body, buf);
        }
    }
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

mod layout;

pub use layout::{
    FinishHit, FinishLayout, ListLine, Section, fits, hit, layout, list_height, list_lines,
    max_top, top_for_cursor,
};

#[cfg(test)]
mod tests;
