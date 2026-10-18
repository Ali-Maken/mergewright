//! A staged part on the commit screen (08s M3): the row says ` part`,
//! dim, after its counts; its diff starts with what that means.

use super::fixture::{buffer, grid, theme, view};
use super::*;
use crate::theme::Token;

#[test]
fn a_staged_part_row_says_part() {
    let mut screen = view("fix toc", CommitFocus::Subject);
    screen.files[0].part_only = true;
    let drawn_buffer = buffer(&screen, 120, 30);
    let drawn = grid(&drawn_buffer);
    let (y, row) = drawn
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("src/receipt.rs"))
        .unwrap_or_else(|| panic!("no row:\n{drawn}"));
    let row = row
        .chars()
        .take(usize::from(FILES_WIDTH))
        .collect::<String>();
    assert!(row.trim_end().ends_with("+12 \u{2212}3 part"), "{row:?}");
    // Dim: the word, not the row, is muted.
    let at = u16::try_from(row.trim_end().chars().count() - 4).expect("narrow");
    let y = u16::try_from(y).expect("short");
    let muted = crate::paint::color(theme().token(Token::TextMuted));
    assert_eq!(drawn_buffer[(at, y)].fg, muted);
    assert_ne!(drawn_buffer[(1, y)].fg, muted, "the path reads as text");
    // The changes pane's first line: level with the files' first heading.
    let line_of = |words: &str| drawn.lines().position(|line| line.contains(words));
    let note = line_of("only the part you staged goes in; the rest stays uncommitted.");
    assert!(note.is_some(), "{drawn}");
    assert_eq!(note, line_of("Going in"), "{drawn}");
    // A whole file says neither.
    let whole = grid(&buffer(&view("fix toc", CommitFocus::Subject), 120, 30));
    assert!(!whole.contains(" part"), "{whole}");
    assert!(!whole.contains("only the part you staged"), "{whole}");
}
