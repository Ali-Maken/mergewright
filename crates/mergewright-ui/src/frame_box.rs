//! Shared rounded-border box drawing, used by every modal box that frames
//! itself: the title in the top border, `│ text │` content rows, hints in
//! the bottom border. [`crate::credential_prompt`] drew this pattern first;
//! [`crate::confirm`], [`crate::preview_box`] and [`crate::name_prompt`]
//! share it here rather than each re-deriving the same three helpers
//! (m5-history-and-ghost-preview/04, the confirm box's missing frame,
//! `KNOWN_GAPS` row 484).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
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
///
/// A label too long to fit is cut with `…` (`fit`'s own rule), and when it
/// is, one blank cell — **not** a dash — is reserved between the cut label
/// and `right`: a dash is a border glyph itself, so putting one directly
/// after the `…` would still leave that ellipsis touching a border glyph,
/// only a different one. Before this, a label that needed cutting claimed
/// every inner cell (`used = cells(&labelled).min(inner)`, nothing left
/// over), so its own `…` sat straight against the corner:
/// `╰ ⏎ send  ⌃U clear  esc cancel the pus…╯`. A label that fits without
/// cutting is unaffected — it already ends in `labelled`'s own trailing
/// space.
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
        format!(" {label} ")
    };
    let full = cells(&labelled);
    if full > inner {
        let used = inner.saturating_sub(1);
        let clamped = fit(&labelled, used);
        return format!("{left}{clamped} {right}");
    }
    let clamped = fit(&labelled, full);
    let dashes = "─".repeat(inner - full);
    format!("{left}{clamped}{dashes}{right}")
}

/// [`border_row`] with styled words: the same cells, but each word keeps
/// its own style and the corners and dashes take `line`'s. A label cut with
/// `…` is cut by the same rule, and the `…` takes the style of the word it
/// cut. The finish summary draws its title and its keys with it
/// (m6-conflicts/59): the frame in `accent`, one key in `accent` BOLD, the
/// rest muted.
#[must_use]
pub(crate) fn border_row_spans(
    left: char,
    right: char,
    words: &[(&str, Style)],
    line: Style,
    width: u16,
) -> Line<'static> {
    let width = usize::from(width);
    let plain: String = words.iter().map(|(word, _)| *word).collect();
    // Below six cells there is no room for a word and its gutters: the
    // plain row, in the line's style.
    if width < 6 || plain.is_empty() {
        return Line::from(Span::styled(border_row(left, right, &plain, width), line));
    }
    // One style per character of `plain`, in order.
    let styles: Vec<Style> = words
        .iter()
        .flat_map(|(word, style)| word.chars().map(move |_| *style))
        .collect();
    let inner = width - 2;
    let full = cells(&plain) + 2;
    let cut = full > inner;
    let label = if cut {
        fit(&plain, inner - 2)
    } else {
        plain.clone()
    };
    let tail = if cut {
        " ".repeat(inner - 1 - cells(&label))
    } else {
        format!(" {}", "─".repeat(inner - full))
    };
    let mut spans = vec![Span::styled(format!("{left} "), line)];
    let mut run = String::new();
    let mut run_style = styles.first().copied().unwrap_or(line);
    for (at, character) in label.chars().enumerate() {
        let style = styles.get(at).or(styles.last()).copied().unwrap_or(line);
        if style != run_style && !run.is_empty() {
            spans.push(Span::styled(std::mem::take(&mut run), run_style));
        }
        run_style = style;
        run.push(character);
    }
    if !run.is_empty() {
        spans.push(Span::styled(run, run_style));
    }
    spans.push(Span::styled(format!("{tail}{right}"), line));
    Line::from(spans)
}

/// A plain content row: `│ text │`, `text` fit to what is left.
///
/// One cell of padding is reserved on **both** sides of `text`, not just the
/// left — the invariant this ticket exists for: no text cell is ever
/// adjacent to a border glyph. Before this, `fit` was only ever asked to
/// clamp `" {text}"` to `inner` cells, so a line exactly `inner - 1` cells
/// long filled every cell up to the right border and left it no gutter at
/// all: the confirm box's longest line ran `…what you send│`, asymmetric
/// against the `│ 1 commit …` on its left. Below three inner cells there is
/// no room to reserve both and still leave the text one cell of its own, so
/// this falls back to the old single-left-pad shape — a caller reaching that
/// width is already past the point where the box should have cleared rather
/// than drawn (ADR-0040 §2).
#[must_use]
pub(crate) fn content_row(text: &str, width: usize) -> String {
    if width < 2 {
        return "│".repeat(width);
    }
    let inner = width - 2;
    if inner < 3 {
        return format!("│{}│", fit(&format!(" {text}"), inner));
    }
    format!("│ {} │", fit(text, inner - 2))
}

/// The gap between two buttons on a choices row.
const BUTTON_GAP: &str = "  ";

/// Where the cursor's button sits on a choices row: cells from the start of
/// the row, and how many cells it takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Cursor {
    /// Cells from the start of the returned row to the cursor's button.
    pub(crate) offset: usize,
    /// The cursor button's own width in cells.
    pub(crate) width: usize,
}

/// The choices row: **every** choice bracketed, so each one reads as a
/// button (DESIGN.md §1.7 draws them all bracketed), and the cursor's marked
/// with a leading `▸` — a cue that survives `NO_COLOR` and a screenshot,
/// where a reversed background does not (CLAUDE.md rule 5). Unselected
/// choices carry a leading space instead, so every button keeps its width
/// and the row never shifts as the cursor moves.
///
/// Returns the row and where the cursor's button is in it, so the caller can
/// add `REVERSED` on top of the `▸` for terminals that have colour. That
/// modifier is **not** what tells the two apart in a capture.
#[must_use]
pub(crate) fn buttons_row(labels: &[&str], cursor: usize) -> (String, Cursor) {
    let mut row = String::new();
    let mut offset = 0;
    let mut at = Cursor {
        offset: 0,
        width: 0,
    };
    for (index, label) in labels.iter().enumerate() {
        if index > 0 {
            row.push_str(BUTTON_GAP);
            offset += cells(BUTTON_GAP);
        }
        let mark = if index == cursor { '▸' } else { ' ' };
        let button = format!("{mark}[ {label} ]");
        let width = cells(&button);
        if index == cursor {
            at = Cursor { offset, width };
        }
        offset += width;
        row.push_str(&button);
    }
    (row, at)
}

/// Paints the cursor's button reversed, on top of the `▸` cue, for the
/// terminals that have colour. `at` is where [`buttons_row`] put it inside
/// the row; the row itself is drawn through [`content_row`], which prefixes
/// `│` and a space — hence the two cells added here. Clipped to the frame's
/// own right border; `Buffer::set_style` clips to the buffer.
pub(crate) fn reverse_button(buf: &mut Buffer, area: Rect, y: u16, at: Cursor, style: Style) {
    let offset = u16::try_from(at.offset).unwrap_or(u16::MAX);
    let start = area.x.saturating_add(2).saturating_add(offset);
    let limit = area.right().saturating_sub(1);
    let width = u16::try_from(at.width)
        .unwrap_or(u16::MAX)
        .min(limit.saturating_sub(start));
    if width == 0 {
        return;
    }
    buf.set_style(
        Rect {
            x: start,
            y,
            width,
            height: 1,
        },
        style.add_modifier(Modifier::REVERSED),
    );
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

    /// The mockups draw `… onto main ───╮`: a space on both sides of the
    /// label, so the title never runs straight into the dashes.
    #[test]
    fn border_row_puts_a_space_on_both_sides_of_the_label() {
        let row = border_row('╭', '╮', "main", 20);
        assert!(row.starts_with("╭ main ─"), "{row:?}");
    }

    /// A label longer than the row has room for is cut with `…`, and one
    /// blank cell — not a dash — sits between the cut label and the right
    /// corner, so the ellipsis never touches a border glyph (the fix this
    /// module's doc comment describes). The sweep only ever reached this
    /// branch by coincidence, where some widget's title happened to
    /// overflow at one of the widths it tests — pinned directly here so the
    /// branch stays covered regardless (review m5-10-30-31 m7).
    #[test]
    fn border_row_reserves_a_blank_cell_before_a_cut_labels_corner() {
        let row = border_row('╭', '╮', "a label far longer than this row is wide", 20);
        assert_eq!(cells(&row), 20, "{row:?}");
        assert!(row.contains('…'), "the label was cut: {row:?}");
        assert!(
            row.ends_with(" ╮"),
            "one blank cell, not a dash, before the corner: {row:?}"
        );
    }

    /// Every choice is a button; only the cursor's carries `▸`. Both are the
    /// same width, so the row does not shift as the cursor moves, and the
    /// reported offset/width really is where the cursor's button sits.
    #[test]
    fn buttons_row_brackets_every_choice_and_marks_the_cursor() {
        let (row, at) = buttons_row(&["Cancel", "Undo locally"], 0);
        assert_eq!(row, "▸[ Cancel ]   [ Undo locally ]");
        assert_eq!(at.offset, 0);
        assert_eq!(at.width, cells("▸[ Cancel ]"));

        let (moved, at) = buttons_row(&["Cancel", "Undo locally"], 1);
        assert_eq!(moved, " [ Cancel ]  ▸[ Undo locally ]");
        assert_eq!(cells(&moved), cells(&row), "the row must not shift");
        assert_eq!(at.offset, cells(" [ Cancel ]  "));
        assert_eq!(at.width, cells("▸[ Undo locally ]"));
    }

    /// The styled row reads exactly as [`border_row`] does, whole or cut,
    /// and each word keeps its own style.
    #[test]
    fn border_row_spans_reads_as_border_row_and_keeps_each_style() {
        let bold = Style::new().add_modifier(Modifier::BOLD);
        let plain = Style::new();
        let words = [("y", bold), (" commit  esc back", plain)];
        for width in [4_u16, 6, 12, 20, 40] {
            let line = border_row_spans('╰', '╯', &words, plain, width);
            let text: String = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();
            assert_eq!(
                text,
                border_row('╰', '╯', "y commit  esc back", usize::from(width)),
                "width {width}"
            );
        }
        let line = border_row_spans('╰', '╯', &words, plain, 40);
        let y = line
            .spans
            .iter()
            .find(|span| span.content == "y")
            .expect("the key is its own span");
        assert_eq!(y.style, bold);
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
