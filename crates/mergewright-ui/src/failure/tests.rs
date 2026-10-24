use mergewright_core::capabilities::ColorDepth;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use super::*;
use crate::fit::fit;
use crate::paint::color;
use crate::test_support::terminal;
use crate::theme::{ThemeKind, TokenColor};

#[test]
fn a_failure_reads_mark_space_words() {
    assert_eq!(marked("push refused", false), "⊗ push refused");
    assert_eq!(marked("push refused", true), "(x) push refused");
}

/// Glyphs other marks draw, read as literals because none is a public
/// const: journal `Lost` `✗` (`journal_screen/view.rs`), rewrite `Drop` `✗`
/// and its ASCII `x` (`rewrite_editor/draw.rs:104,120`), `⇢` replaced and
/// deleted, `⚠`/`ⓘ` radar (`graph_screen/render.rs:316,318`), branch-pane
/// `!` `✓` `·`, conflict `?` `◀` `▶` `◆`.
const OTHER_MARKS: [&str; 14] = [
    "✗", "x", "⇢", "->", "⚠", "ⓘ", "!", "✓", "·", "?", "◀", "▶", "◆", "i",
];

#[test]
fn the_mark_is_one_cell_and_no_other_marks_glyph() {
    assert_eq!(FAILED_MARK.width(), 1);
    // The width function `fit` uses: a width-1 cut keeps it whole.
    assert_eq!(fit(FAILED_MARK, 1), FAILED_MARK);
    for other in OTHER_MARKS {
        assert_ne!(FAILED_MARK, other);
        assert_ne!(FAILED_MARK_ASCII, other);
    }
}

#[test]
#[should_panic(expected = "must be unmarked")]
fn premarked_words_are_refused_in_debug() {
    let _ = marked("✗ push refused", false);
}

#[test]
fn the_mark_draws_in_error_ink_and_survives_no_color() {
    let caps = terminal(ColorDepth::TrueColor, false);
    let theme = Theme::new(ThemeKind::Dark, &caps);
    let words = Style::default();
    let drawn = spans("push refused", false, &theme, words);
    assert_eq!(drawn[0].content, FAILED_MARK);
    assert_eq!(drawn[0].style.fg, Some(color(theme.token(FAILED_INK))));
    assert_eq!(FAILED_INK, Token::Removed);
    assert_ne!(theme.token(FAILED_INK), TokenColor::Default);
    let plain = theme.colourless();
    let drawn = spans("push refused", false, &plain, words);
    assert_eq!(drawn[0].content, FAILED_MARK, "the glyph still draws");
    assert_eq!(drawn[0].style.fg, Some(color(TokenColor::Default)));
}

#[test]
fn the_mark_ink_differs_from_warning_in_every_theme() {
    let caps = terminal(ColorDepth::TrueColor, false);
    for kind in [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast] {
        let theme = Theme::new(kind, &caps);
        assert_ne!(
            theme.token(FAILED_INK),
            theme.token(Token::Warning),
            "{kind:?}: removed {:?} and warning {:?} share a colour",
            theme.token(FAILED_INK),
            theme.token(Token::Warning),
        );
    }
}
