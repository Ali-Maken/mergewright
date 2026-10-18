//! A file cell's state words and their colour (ADR-0044 §1): which tone a
//! state takes, where the words sit in the cell, and the second style drawn
//! over them.
//!
//! Split from [`super`] (m6-conflicts/64, review M2) to keep the list's
//! module within PROMPT.md §6.2's size.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use super::{Cell, FileState, cells_u16};
use crate::paint::color;
use crate::theme::{Theme, Token};

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

impl FileState {
    /// The colour the state's words take (ADR-0044 §1): the work still to
    /// do in `open`, a done or solved file's words muted, the rest as the name.
    pub(super) fn tone(&self) -> WordsTone {
        match self {
            FileState::Open { .. } | FileState::AllTaken | FileState::WholeFile { .. } => {
                WordsTone::Open
            }
            FileState::Done | FileState::Solved => WordsTone::Muted,
            FileState::Binary | FileState::Unread => WordsTone::Plain,
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
    use super::super::{FileState, Glyphs};
    use super::WordsTone;

    /// A merge's solved file says so; `⏎` marks nothing there (ease-pass/05c).
    #[test]
    fn a_solved_file_reads_solved_muted_and_names_no_key() {
        assert_eq!(FileState::Solved.words(false, Glyphs::UNICODE), "solved ✓");
        assert_eq!(FileState::Solved.words(true, Glyphs::UNICODE), "✓");
        assert_eq!(FileState::Solved.tone(), WordsTone::Muted);
    }
}
