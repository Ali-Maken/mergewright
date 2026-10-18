//! The header's account of the walk, in the room it has — split out of
//! `layout` for size (ease-pass/08s M6).

use unicode_width::UnicodeWidthStr;

use super::WalkView;
use crate::theme::Token;

/// The header's account of the walk, and the token it is drawn in: the
/// widest wording, as [`walk_text_within`] gives it with room to spare.
#[cfg(test)]
pub(super) fn walk_text(walk: &WalkView) -> (String, Token) {
    walk_text_within(walk, usize::MAX)
}

/// The header's account of the walk in at most `room` columns, where a
/// state has shorter wordings: a stopped walk keeps the widest of
/// `stopped at N rows · ⌃R reloads`, `stopped at N rows`, `stopped` that
/// fits (m6-conflicts/85 A2). Every other state has one wording, cut by the
/// caller like any header text.
pub(super) fn walk_text_within(walk: &WalkView, room: usize) -> (String, Token) {
    match walk {
        WalkView::Opening => ("opening…".to_owned(), Token::TextMuted),
        WalkView::Reading { laid_out: 0 } => ("reading history…".to_owned(), Token::TextMuted),
        WalkView::Reading { laid_out } => (format!("reading… {laid_out} rows"), Token::TextMuted),
        WalkView::Paused { laid_out } => (format!("{laid_out} rows so far"), Token::TextMuted),
        WalkView::Jumping { laid_out } => (format!("laying out… {laid_out} rows"), Token::Accent),
        WalkView::Complete { rows: 0 } => ("no commits yet".to_owned(), Token::TextMuted),
        WalkView::Complete { rows: 1 } => ("1 commit".to_owned(), Token::TextMuted),
        WalkView::Complete { rows } => (format!("{rows} commits"), Token::TextMuted),
        WalkView::Cancelled { laid_out } => (stopped_words(*laid_out, room), Token::TextMuted),
        WalkView::Failed(reason) => (format!("history failed: {reason}"), Token::Warning),
    }
}

/// A stopped walk's words, widest first; the last rung when none fits (the
/// caller cuts it). `⌃` spells Ctrl as the bar does.
fn stopped_words(laid_out: usize, room: usize) -> String {
    let rungs = [
        format!("stopped at {laid_out} rows \u{b7} \u{2303}R reloads"),
        format!("stopped at {laid_out} rows"),
        "stopped".to_owned(),
    ];
    let widest = rungs.iter().find(|words| text_width(words) <= room);
    widest.unwrap_or(&rungs[2]).clone()
}

pub(super) fn text_width(text: &str) -> usize {
    text.width()
}
