//! Shared rounded-border box drawing, used by every modal box that frames
//! itself: the title in the top border, `│ text │` content rows, hints in
//! the bottom border. [`crate::credential_prompt`] drew this pattern first;
//! [`crate::confirm`], [`crate::preview_box`] and [`crate::name_prompt`]
//! share it here rather than each re-deriving the same three helpers
//! (m5-history-and-ghost-preview/04, the confirm box's missing frame,
//! `KNOWN_GAPS` row 484).

use unicode_width::UnicodeWidthChar;

use crate::fit::fit;

/// Cells `text` occupies, counting a wide character as two — the same
/// measure [`fit`] clamps to.
#[must_use]
pub(crate) fn cells(text: &str) -> usize {
    text.chars().filter_map(UnicodeWidthChar::width).sum()
}

/// A border row: `left`, a space, `label` fit to what is left, dashes
/// filling the rest, a space, `right` — exactly `width` cells. `label`
/// empty draws a plain dashed border.
#[must_use]
pub(crate) fn border_row(left: char, right: char, label: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return left.to_string();
    }
    let inner = width - 2;
    if inner == 0 {
        return format!("{left}{right}");
    }
    let labelled = if label.is_empty() {
        String::new()
    } else {
        format!(" {label}")
    };
    let used = cells(&labelled).min(inner);
    let clamped = fit(&labelled, used);
    let dashes = "─".repeat(inner - used);
    format!("{left}{clamped}{dashes}{right}")
}

/// A plain content row: `│ text │`, `text` fit to what is left.
#[must_use]
pub(crate) fn content_row(text: &str, width: usize) -> String {
    if width < 2 {
        return "│".repeat(width);
    }
    let inner = width - 2;
    format!("│{}│", fit(&format!(" {text}"), inner))
}

/// One source line, wrapped on spaces to at most `width` cells. A word
/// longer than `width` on its own is placed on its own line and left to
/// [`fit`]'s own clamp at draw time rather than split mid-word.
#[must_use]
pub(crate) fn wrap_line(source: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in source.split(' ') {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if !current.is_empty() && cells(&candidate) > width {
            lines.push(std::mem::replace(&mut current, word.to_owned()));
        } else {
            current = candidate;
        }
    }
    lines.push(current);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn border_row_carries_the_title_in_its_first_row() {
        let row = border_row('╭', '╮', "Preview", 20);
        assert!(row.starts_with('╭'));
        assert!(row.ends_with('╮'));
        assert!(row.contains("Preview"));
        assert_eq!(cells(&row), 20);
    }

    #[test]
    fn content_row_is_exactly_the_width() {
        assert_eq!(cells(&content_row("hi", 10)), 10);
    }

    #[test]
    fn wrap_line_never_splits_a_word() {
        let wrapped = wrap_line("a longer sentence than the width", 10);
        let rejoined: Vec<&str> = wrapped.iter().flat_map(|l| l.split(' ')).collect();
        assert_eq!(
            rejoined,
            vec!["a", "longer", "sentence", "than", "the", "width"]
        );
    }
}
