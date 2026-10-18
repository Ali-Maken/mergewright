//! The merge's review (ease-pass/12; m6-conflicts/19 before it): what will
//! land before `y` commits it — every clashed file and how it was solved,
//! how many files git combined on its own, and the message, read-only. The
//! mock's box (`mergewright-merge-screen.html:155-162`), drawn inside a
//! rounded frame with the keys in its bottom border.
//!
//! Owns no state and reads no repository. `graft::update::finish` holds the
//! state and `graft::view::finish` turns it into the [`ReviewView`] this
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
use crate::theme::{Theme, Token};

/// The smallest area the review draws in, `(columns, rows)`. Below it the
/// widget draws one line, `finish needs 62x18 · esc back`, and the app
/// answers only `Esc` (and `Ctrl+C`).
pub const FINISH_MIN: (u16, u16) = (62, 18);

/// The most rows an expanded diff draws under its row. More than that adds
/// one row, `… N more lines · ⏎ hides`.
pub const DIFF_ROWS_MAX: usize = 12;

/// Cells an expanded diff is indented by, under its row.
const DIFF_INDENT: u16 = 4;

/// What had the keyboard in the m6 finish summary's fields. The review has
/// no fields now (ease-pass/12): kept only as the retiring
/// `HintContext::Finish`'s field, which nothing picks.
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

/// Whether the review itself has been read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SummaryState {
    /// `reading what this merge changes…`.
    Loading,
    /// The lines are the review's.
    Ready,
    /// The read failed: `summary unavailable: <why> · y commits with git's
    /// message`.
    Unavailable(SanitizedText),
}

/// One clashed file's picks, by kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PickCounts {
    /// `◀ yours`.
    pub yours: u32,
    /// `▶ theirs`.
    pub theirs: u32,
    /// `◆ both`.
    pub both: u32,
}

/// How one clashed file was solved (ticket decision 4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HowSolved {
    /// Clash by clash: `you chose: ◀ yours (1) · ▶ theirs (1)`.
    Picks(PickCounts),
    /// A whole-file card's choice: `you chose: <the card's word>`.
    Whole(SanitizedText),
    /// No picks recorded — edited by hand, or a relaunch since: `solved in
    /// the file`.
    ByHand,
}

impl HowSolved {
    /// The words after the file's name. One kind alone has no count; with
    /// several each has its own, in `◀ ▶ ◆` order, zero kinds left out.
    #[must_use]
    pub fn words(&self) -> String {
        match self {
            Self::Picks(counts) => {
                let kinds: Vec<(&str, u32)> = [
                    ("◀ yours", counts.yours),
                    ("▶ theirs", counts.theirs),
                    ("◆ both", counts.both),
                ]
                .into_iter()
                .filter(|(_, n)| *n > 0)
                .collect();
                match kinds.as_slice() {
                    [] => "solved in the file".to_owned(),
                    [(kind, _)] => format!("you chose: {kind}"),
                    _ => format!(
                        "you chose: {}",
                        kinds
                            .iter()
                            .map(|(kind, n)| format!("{kind} ({n})"))
                            .collect::<Vec<_>>()
                            .join(" · ")
                    ),
                }
            }
            Self::Whole(word) => format!("you chose: {}", word.as_str()),
            Self::ByHand => "solved in the file".to_owned(),
        }
    }

    /// The clashes this file counts for in the title: each pick, or one
    /// for a whole-file choice or a file solved by hand.
    #[must_use]
    pub fn clashes(&self) -> u32 {
        match self {
            Self::Picks(counts) => (counts.yours + counts.theirs + counts.both).max(1),
            Self::Whole(_) | Self::ByHand => 1,
        }
    }
}

/// One clashed file's row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewFile {
    /// The path, whole; the row draws its last segment unless two clashed
    /// files share one.
    pub path: SanitizedText,
    /// How it was solved.
    pub how: HowSolved,
}

/// What the cursor can be on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewRow {
    /// A clashed file, by its index in [`ReviewView::clashed`].
    Clashed(usize),
    /// `✓ N files git combined on its own`.
    Combined,
}

/// Everything the review draws.
#[derive(Clone, Debug)]
pub struct ReviewView<'a> {
    /// His keymap: every key the review names is spelled from it.
    pub keys: &'a crate::keymap::Keymap,
    /// The branch merged in.
    pub from: SanitizedText,
    /// The branch it lands on.
    pub into: SanitizedText,
    /// The title's `N clashes`.
    pub clashes: u32,
    /// `HEAD..MERGE_HEAD`, the commits the merge brings in; `None` when
    /// the count failed (`?`).
    pub incoming: Option<u32>,
    /// The clashed files, in the merge's order.
    pub clashed: Vec<ReviewFile>,
    /// Files git combined on its own; no row when 0.
    pub combined: u32,
    /// The message's subject, as the merge will commit it.
    pub message: SanitizedText,
    /// The cursor, counted over the clashed rows then the combined row.
    pub cursor: usize,
    /// The row whose diff is shown under it, and the diff.
    pub expanded: Option<(usize, ExpandedDiff)>,
    /// The frame has the keyboard: its border is `accent` and its keys are
    /// drawn. `false` under a live box, which claims the next key.
    pub focused: bool,
    /// A line above the bottom border, and whether it is a problem.
    pub notice: Option<(SanitizedText, bool)>,
    /// The commit is running.
    pub busy: bool,
    /// Whether the review has been read.
    pub state: SummaryState,
    /// The first list line on screen.
    pub top: usize,
}

impl ReviewView<'_> {
    /// Rows the cursor counts over: each clashed file, and the combined
    /// row when git combined anything.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.clashed.len() + usize::from(self.combined > 0)
    }

    /// The row the cursor index names.
    #[must_use]
    pub fn row(&self, index: usize) -> Option<ReviewRow> {
        if index < self.clashed.len() {
            Some(ReviewRow::Clashed(index))
        } else if index < self.row_count() {
            Some(ReviewRow::Combined)
        } else {
            None
        }
    }
}

/// The title (mock `:155`): `All N clashes solved. Nothing committed yet.`
#[must_use]
pub fn review_title(clashes: u32) -> String {
    match clashes {
        0 => "Nothing committed yet.".to_owned(),
        1 => "1 clash solved. Nothing committed yet.".to_owned(),
        n => format!("All {n} clashes solved. Nothing committed yet."),
    }
}

/// The count line (mock `:157`): `N commits from <from>`, `?` when the
/// count failed (06's rule).
#[must_use]
pub fn count_line(incoming: Option<u32>, from: &SanitizedText) -> String {
    let from = from.as_str();
    match incoming {
        Some(1) => format!("1 commit from {from}"),
        Some(n) => format!("{n} commits from {from}"),
        None => format!("? commits from {from}"),
    }
}

/// [`ReviewView`] drawn into the area under the header and above the bar.
#[derive(Debug)]
pub struct FinishScreen<'a> {
    /// What to draw.
    pub view: &'a ReviewView<'a>,
    /// Colours.
    pub theme: &'a Theme,
    /// The ASCII renderer tier: `>` for the cursor.
    pub ascii: bool,
}

impl Widget for &FinishScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        // The review replaces the graph and the detail pane both, so it
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
                    &self.view.keys.after(
                        &format!("finish needs {columns}x{rows}"),
                        crate::keymap::ActionId("app.back"),
                        "back",
                    ),
                    usize::from(area.width),
                ),
                foreground(self.theme, Token::Warning),
            );
            return;
        };
        self.draw_frame(buf, placed.frame);
        self.draw_head(buf, placed.head);
        self.draw_list(buf, placed.list);
        if matches!(self.view.state, SummaryState::Ready) {
            let words = format!("Message  {}", self.view.message.as_str());
            self.text(buf, placed.message, &words, Token::Text, false);
        }
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
        if rect.height == 0 {
            return;
        }
        let mut style = foreground(self.theme, token);
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        buf.set_string(rect.x, rect.y, fit(text, usize::from(rect.width)), style);
    }

    /// The title, then — once read — what the merge puts where and how
    /// many commits it brings (mock `:155-157`).
    fn draw_head(&self, buf: &mut Buffer, rect: Rect) {
        let line = |nth: u16| Rect {
            y: rect.y + nth,
            height: u16::from(nth < rect.height),
            ..rect
        };
        let ready = matches!(self.view.state, SummaryState::Ready);
        let clashes = if ready { self.view.clashes } else { 0 };
        self.text(buf, line(0), &review_title(clashes), Token::Added, true);
        if !ready {
            return;
        }
        let into = format!("This merge will put into {}:", self.view.into.as_str());
        self.text(buf, line(2), &into, Token::Text, false);
        let count = format!("  {}", count_line(self.view.incoming, &self.view.from));
        self.text(buf, line(3), &count, Token::Text, false);
    }
}

mod frame;
mod layout;
mod list;

pub use frame::{footer_hints, footer_spans};
pub use layout::{
    FinishLayout, ListLine, fits, hit, layout, list_height, list_lines, max_top, top_for_cursor,
};

#[cfg(test)]
mod tests;
