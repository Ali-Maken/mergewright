//! Where a listed file stands ([`FileRowLook`]), its state words, and their
//! colour (ADR-0044 §1): which tone a state takes, where the words sit in
//! the cell, and the second style drawn over them.
//!
//! Split from [`super`] (m6-conflicts/64, review M2) to keep the list's
//! module within PROMPT.md §6.2's size.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use graft_term::SanitizedText;

use super::{Cell, Glyphs, cells_u16};
use crate::paint::color;
use crate::theme::{Theme, Token};

/// Where one listed file stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileRowLook {
    /// Open in the resolver (or seen there) with `open` of `of` blocks left.
    Open {
        /// Blocks still open.
        open: usize,
        /// Blocks in the file.
        of: usize,
    },
    /// Every block taken; `⏎` marks it done.
    AllTaken,
    /// A merge's file with every clash picked: still unmerged, staged by
    /// the review's `y` (ease-pass/05b) — `⏎` has nothing to mark.
    Solved,
    /// git no longer calls it unmerged.
    Done,
    /// A whole-file shape (modify/delete, add/add without markers, …).
    WholeFile {
        /// The shape, in words with each side's one name — for ticket 14's
        /// panes; the list itself draws `keep or delete`.
        how: SanitizedText,
    },
    /// A binary file: kept whole from one side.
    Binary,
    /// Not read yet.
    Unread,
}

impl FileRowLook {
    /// The state's words: full, or compact (`1/2 open`, `✓`).
    pub(super) fn words(&self, compact: bool, g: Glyphs) -> String {
        match (self, compact) {
            (FileRowLook::Open { open, of }, false) => format!("{open} of {of} open"),
            (FileRowLook::Open { open, of }, true) => format!("{open}/{of} open"),
            (FileRowLook::AllTaken, false) => format!("{} marks done", g.enter),
            (FileRowLook::AllTaken, true) => g.enter.to_string(),
            (FileRowLook::Solved, false) => format!("solved {}", g.check),
            (FileRowLook::Done, false) => format!("done {}", g.check),
            // Compact, solved and done draw the same tick; the tone tells
            // them apart no more than the words do (both muted).
            (FileRowLook::Solved | FileRowLook::Done, true) => g.check.to_string(),
            (FileRowLook::WholeFile { .. }, false) => "keep or delete".to_owned(),
            (FileRowLook::WholeFile { .. }, true) => "keep?".to_owned(),
            (FileRowLook::Binary, false) => "binary".to_owned(),
            (FileRowLook::Binary, true) => "bin".to_owned(),
            (FileRowLook::Unread, _) => g.unread.to_string(),
        }
    }
}

/// How a cell's state words are coloured, apart from its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WordsTone {
    /// `open`: `n of m open`, `keep or delete`, `⏎ marks done`.
    Open,
    /// `text-muted`: `done ✓`, `solved ✓`.
    Muted,
    /// The name's own style: `binary`, the unread `•`.
    Plain,
}

impl FileRowLook {
    /// The colour the state's words take (ADR-0044 §1): the work still to
    /// do in `open`, a done or solved file's words muted, the rest as the name.
    pub(super) fn tone(&self) -> WordsTone {
        match self {
            FileRowLook::Open { .. } | FileRowLook::AllTaken | FileRowLook::WholeFile { .. } => {
                WordsTone::Open
            }
            FileRowLook::Done | FileRowLook::Solved => WordsTone::Muted,
            FileRowLook::Binary | FileRowLook::Unread => WordsTone::Plain,
        }
    }
}

/// Where `words` sit when they end `text`: `(column, width)` in cells.
pub(super) fn words_at_end(text: &str, words: &str) -> Option<(u16, u16)> {
    let at = text.width().checked_sub(words.width())?;
    Some((cells_u16(at), cells_u16(words.width())))
}

/// The state words in their tone, restyled over the cell text just drawn
/// at `at`: the characters stay put, the cursor's BOLD stays on.
pub(super) fn paint_words(buf: &mut Buffer, at: Rect, cell: &Cell, theme: &Theme) {
    let tone = match cell.tone {
        WordsTone::Open => Some(Token::Open),
        WordsTone::Muted => Some(Token::TextMuted),
        WordsTone::Plain => None,
    };
    if let (Some(token), Some((from, width))) = (tone, cell.words) {
        let words = Rect {
            x: at.x.saturating_add(from),
            width,
            ..at
        }
        .intersection(at);
        buf.set_style(words, Style::new().fg(color(theme.token(token))));
    }
}

#[cfg(test)]
mod tests {
    use super::super::Glyphs;
    use super::{FileRowLook, WordsTone};

    /// A merge's solved file says so; `⏎` marks nothing there (ease-pass/05c).
    #[test]
    fn a_solved_file_reads_solved_muted_and_names_no_key() {
        assert_eq!(
            FileRowLook::Solved.words(false, Glyphs::UNICODE),
            "solved ✓"
        );
        assert_eq!(FileRowLook::Solved.words(true, Glyphs::UNICODE), "✓");
        assert_eq!(FileRowLook::Solved.tone(), WordsTone::Muted);
    }
}
