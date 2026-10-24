//! The history place's file pane: one version of the file, its changed lines marked, or a binary's sizes.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

pub(super) fn draw_file(buf: &mut Buffer, area: Rect, view: &HistoryView, ascii: bool) {
    if view.tracing || area.is_empty() {
        return;
    }
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let below = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };
    match &view.file {
        FilePane::Empty => {}
        FilePane::NoChange => put(buf, area.x, area.y, area.width, "no change to show", dim()),
        FilePane::Failed(words) => put(
            buf,
            area.x,
            area.y,
            area.width,
            &crate::failure::marked(words, ascii),
            Style::default(),
        ),
        FilePane::Reading { header } => {
            header_and_line(buf, area, header, ("reading\u{2026}".to_owned(), dim()));
        }
        FilePane::Binary {
            header,
            old,
            new,
            sizes_failed,
        } => header_and_line(buf, area, header, binary_words(*old, *new, *sizes_failed)),
        FilePane::Lines {
            header,
            hunks,
            top,
            range,
            side_by_side,
            word_tint,
        } => {
            put(buf, area.x, area.y, area.width, header, bold);
            if below.is_empty() {
                return;
            }
            let focused = view.focus == HistoryFocus::File;
            if *side_by_side {
                // Its row count is for the caller's scroll clamping; the
                // place clamps against `file_rows` itself.
                let _rows: usize = diff_body_side_by_side(
                    hunks, *top, below, buf, view.theme, focused, *word_tint,
                );
            } else {
                let style = DiffRowStyle {
                    focused,
                    palette16: matches!(
                        view.theme.color_depth(),
                        ColorDepth::Palette16 | ColorDepth::None
                    ),
                    word_tint: *word_tint,
                    cursor: false,
                };
                draw_unified(buf, below, hunks, *top, range.as_ref(), view.theme, style);
            }
        }
    }
}

/// `header` in bold on the pane's first row, and `(words, style)` on the
/// row under it when there is one.
fn header_and_line(buf: &mut Buffer, area: Rect, header: &str, (words, style): (String, Style)) {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    put(buf, area.x, area.y, area.width, header, bold);
    if area.height > 1 {
        put(buf, area.x, area.y + 1, area.width, &words, style);
    }
}

/// A binary file's line: `binary · <old> → <new>`, or, dim, why the sizes
/// could not be read (21c).
fn binary_words(old: Option<u64>, new: Option<u64>, sizes_failed: Option<&str>) -> (String, Style) {
    match sizes_failed {
        Some(why) => (format!("binary \u{b7} couldn't read sizes: {why}"), dim()),
        None => (
            format!("binary \u{b7} {} \u{2192} {}", size(old), size(new)),
            Style::default(),
        ),
    }
}

/// `1234`, `12 KB`, `3 MB`; `none` for a side the file did not exist on.
fn size(bytes: Option<u64>) -> String {
    match bytes {
        None => "none".to_owned(),
        Some(bytes) if bytes < 1024 => format!("{bytes} B"),
        Some(bytes) if bytes < 1024 * 1024 => format!("{} KB", bytes.div_ceil(1024)),
        Some(bytes) => format!("{} MB", bytes.div_ceil(1024 * 1024)),
    }
}

/// Where each row sits among the new side's lines, for the `┃` range: a
/// removed row sits just after the last kept line before it.
fn sits_among_new_lines(rows: &[&LineView]) -> Vec<u32> {
    let mut position = 0u32;
    rows.iter()
        .map(|line| match line.new_no {
            Some(no) => {
                position = no;
                no
            }
            None => position + 1,
        })
        .collect()
}

/// The whole file, one row per line: `┃` on the range, the new line's
/// number (none on a removed row), the sign, the text.
fn draw_unified(
    buf: &mut Buffer,
    area: Rect,
    hunks: &[HunkView],
    top: usize,
    range: Option<&RangeInclusive<u32>>,
    theme: &Theme,
    style: DiffRowStyle,
) {
    let rows = file_rows(hunks);
    let number_cells = rows
        .iter()
        .filter_map(|line| line.new_no)
        .max()
        .unwrap_or(0)
        .to_string()
        .len()
        .max(3);
    let sits = sits_among_new_lines(&rows);
    for (offset, (line, at)) in rows
        .iter()
        .zip(sits)
        .skip(top)
        .take(usize::from(area.height))
        .enumerate()
    {
        let y = row_y(area, offset);
        let covered = range.is_some_and(|range| range.contains(&at));
        let mark = if covered { RANGE_MARK } else { ' ' };
        let number = line.new_no.map_or(String::new(), |no| no.to_string());
        let prefix = format!("{mark} {number:>number_cells$} ");
        let prefix_cells = u16::try_from(prefix.chars().count()).unwrap_or(u16::MAX);
        put(
            buf,
            area.x,
            y,
            area.width,
            &prefix,
            gutter_style(style, theme),
        );
        if prefix_cells >= area.width {
            continue;
        }
        let (sign, kind) = match line.kind {
            LineKindView::Added => ('+', Some(Token::Added)),
            LineKindView::Removed => ('\u{2212}', Some(Token::Removed)),
            LineKindView::Context | LineKindView::HunkHeading => (' ', None),
        };
        let sign_style = line_style_at(line, usize::MAX, theme, style, kind);
        let sign_x = area.x + prefix_cells;
        put(buf, sign_x, y, 1, &sign.to_string(), sign_style);
        if prefix_cells + 2 >= area.width {
            continue;
        }
        put(buf, sign_x + 1, y, 1, " ", sign_style);
        let text_x = sign_x + 2;
        let text_width = area.width - prefix_cells - 2;
        draw_styled_text(
            buf,
            text_x,
            y,
            text_width,
            line.text.as_str(),
            sign_style,
            |index| line_style_at(line, index, theme, style, kind),
        );
    }
}

/// `text` broken at spaces into lines of at most `room` cells; a word
/// longer than `room` is left to be cut where it is drawn.
pub(super) fn wrap(text: &str, room: usize) -> Vec<String> {
    use unicode_width::UnicodeWidthStr;
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        let wanted = if current.is_empty() {
            word.width()
        } else {
            current.width() + 1 + word.width()
        };
        if wanted > room && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
