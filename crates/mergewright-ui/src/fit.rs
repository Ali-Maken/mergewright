//! Clamping untrusted text to an exact number of terminal cells.
//!
//! `term::sanitize` makes repository text inert but leaves its width alone:
//! a fullwidth character takes two cells, and a stack of combining marks
//! ("Zalgo") takes one cell yet can paint far outside it. A column that is
//! off by one cell misaligns every column to its right, so every widget that
//! draws repository text into a fixed-width column goes through [`fit`].

use unicode_width::UnicodeWidthChar;

/// Combining marks kept on one base character. Two covers real scripts
/// (Vietnamese stacks two diacritics); more is decoration or an attack.
pub const MAX_MARKS: usize = 2;

/// Marks the cut when text is longer than its column.
const ELLIPSIS: char = '…';

/// `text` laid into exactly `width` cells: padded with spaces when short,
/// cut with `…` when long.
///
/// - A character that needs two cells where only one is left becomes a
///   space rather than spilling into the next column.
/// - At most [`MAX_MARKS`] zero-width characters stay on each base; the
///   rest are dropped. Zero-width characters before any base are dropped too:
///   there is nothing for them to sit on.
/// - A character with no defined width (a control) cannot reach here through
///   `term::sanitize`; if one does, it is dropped rather than guessed at.
#[must_use]
pub fn fit(text: &str, width: usize) -> String {
    let total: usize = text.chars().filter_map(UnicodeWidthChar::width).sum();
    let room = if total > width {
        width.saturating_sub(1)
    } else {
        width
    };

    let mut out = String::with_capacity(width);
    let mut used = 0;
    let mut marks = 0;
    let mut has_base = false;
    let mut cut = false;
    for character in text.chars() {
        match character.width() {
            Some(0) => {
                if has_base && marks < MAX_MARKS {
                    out.push(character);
                    marks += 1;
                }
            }
            Some(cells) => {
                if used + cells > room {
                    cut = true;
                    break;
                }
                out.push(character);
                used += cells;
                marks = 0;
                has_base = true;
            }
            None => {}
        }
    }
    if cut && width > 0 {
        out.push(ELLIPSIS);
        used += 1;
    }
    while used < width {
        out.push(' ');
        used += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use unicode_width::UnicodeWidthStr;

    use super::*;

    #[test]
    fn short_text_is_padded_to_the_width() {
        assert_eq!(fit("main", 6), "main  ");
    }

    #[test]
    fn long_text_is_cut_with_an_ellipsis_inside_the_width() {
        assert_eq!(fit("feature/login", 8), "feature…");
    }

    #[test]
    fn a_cut_never_splits_a_wide_character() {
        // Three cells before the ellipsis: "日" fits, "本" would straddle.
        let fitted = fit("日本語", 4);
        assert_eq!(fitted, "日… ");
        assert_eq!(fitted.width(), 4);
    }

    #[test]
    fn a_stack_of_combining_marks_keeps_only_two_per_base() {
        let zalgo = "a\u{0301}\u{0302}\u{0303}\u{0304}b";
        assert_eq!(fit(zalgo, 2), "a\u{0301}\u{0302}b");
    }

    #[test]
    fn marks_with_nothing_under_them_are_dropped() {
        assert_eq!(fit("\u{0301}x", 1), "x");
    }

    #[test]
    fn the_result_is_exactly_the_width_whatever_goes_in() {
        for text in [
            "",
            "x",
            "日本語テキスト",
            "a\u{0301}\u{0301}\u{0301}",
            "ｗｉｄｅ ok",
        ] {
            for width in 0..10 {
                assert_eq!(fit(text, width).width(), width, "{text:?} in {width}");
            }
        }
    }
}
