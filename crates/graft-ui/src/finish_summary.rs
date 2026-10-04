//! The finish summary (`c` on a merge, m6-conflicts/19): what will land —
//! the files the user resolved, the files git combined on its own, and the
//! message — before `y` commits it. It is drawn as a preview, inside the
//! same rounded frame as the merge preview: title in the top border, keys in
//! the bottom one (`docs/design-notes/m6-conflicts.md` Amendment 2, "Finish
//! summary", and Amendment 2026-09-24, "The finish summary is a preview").
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
use crate::fit::fit;
use crate::paint::foreground;
use crate::text_input::{AreaView, AreaWidget, FieldView, FieldWidget};
use crate::theme::{Theme, Token};

/// The smallest area the summary draws in, `(columns, rows)`. Below it the
/// widget draws one line, `finish needs 62x18 · esc back`, and the app
/// answers only `Esc` (and `Ctrl+C`). (60, 14) grown by the frame: its two
/// side columns, its two border rows and the two preview lines.
pub const FINISH_MIN: (u16, u16) = (62, 18);

/// From this width the file lists take the left 60 % and the message the
/// right; below it, one column.
pub const TWO_COLUMN_MIN: u16 = 160;

/// The most rows an expanded diff draws under its row. More than that adds
/// one row, `… N more lines · ⏎ hides`.
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
    /// The read failed: `summary unavailable: <why> · y commits with git's
    /// message`.
    Unavailable(SanitizedText),
}

/// Everything the summary draws.
#[derive(Clone, Debug)]
pub struct FinishView<'a> {
    /// His keymap: every key the summary names is spelled from it.
    pub keys: &'a crate::keymap::Keymap,
    /// `(from, into)`: the branch merged in and the one it lands on. `None`
    /// while the read has not landed, or when the names are not known: the
    /// title then reads `Preview: finish the merge`, never an invented name.
    pub names: Option<(SanitizedText, SanitizedText)>,
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
    /// The cursor stands on a row git combined: the footer offers `⏎ diff`
    /// (`HintContext::Finish`'s own meaning).
    pub expandable: bool,
    /// The frame has the keyboard: its border is `accent` and its keys are
    /// drawn. `false` under a live box (a confirm, a prompt), which claims
    /// the next key, so the summary's keys would do nothing (rulings 07b,
    /// Q1).
    pub focused: bool,
    /// A line above the bottom border, and whether it is a problem.
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
        self.draw_frame(buf, placed.frame);
        self.draw_preview(buf, placed.preview);
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

mod frame;
mod layout;
mod list;

pub use frame::{UNDO_LINE, footer_hints, footer_spans, preview_line, preview_title};
pub use layout::{
    FinishHit, FinishLayout, ListLine, Section, fits, hit, layout, list_height, list_lines,
    max_top, top_for_cursor,
};

#[cfg(test)]
mod tests;
