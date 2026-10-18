//! The column beside the document (from `INLINE_FROM` columns): the
//! `Clashing files` heading, blank, one file per row — its mark, name and
//! state words, states aligned — as the merge mock's list draws it
//! (ease-pass/11). Planned here, drawn and hit-tested by `files` through
//! the same [`Plan`].

use unicode_width::UnicodeWidthStr;

use super::{Cell, FileListView, Glyphs, Plan, WordsTone, cells_u16, fit_left, window};
use crate::fit::fit;

/// Rows the column spends above its files: the heading and the blank row
/// where the three panes' titles sit.
pub(super) const COLUMN_HEAD: u16 = 2;

/// The heading, as the mock's list names it.
const HEADING: &str = "Clashing files";

/// Under the heading, on the row it keeps blank, when git's own list could
/// not be read: why the list may be short.
const UNAVAILABLE: &str = "git's list unavailable";

/// `words` at column `at` of a row `full` cells wide, as `(column, width)`;
/// `None` when the row's cut at `full` reaches them (left uncoloured).
fn words_within(at: usize, words: &str, full: usize) -> Option<(u16, u16)> {
    let width = words.width();
    (at + width <= full).then(|| (cells_u16(at), cells_u16(width)))
}

/// A row of words that names no file: the unavailable note, `N more`.
fn plain_cell(y: u16, width: u16, text: String) -> Cell {
    Cell {
        x: 0,
        y,
        width,
        text,
        index: None,
        cursor: false,
        words: None,
        tone: WordsTone::Plain,
    }
}

/// The column: heading, blank, then one file per row — `▏? name  2 open`,
/// the gutter on the open file's row — then a row kept for the tail.
pub(super) fn column_plan(view: &FileListView, width: u16, height: u16, g: Glyphs) -> Plan {
    let full = usize::from(width);
    let states: Vec<String> = view
        .rows
        .iter()
        .map(|row| row.state.words(false, g))
        .collect();
    let state_w = states.iter().map(|s| s.width()).max().unwrap_or(0);
    let longest_name = view
        .rows
        .iter()
        .map(|row| row.name.as_str().width())
        .max()
        .unwrap_or(0);
    let name_w = longest_name.min(full.saturating_sub(3 + 2 + state_w));
    let slots = usize::from(height.saturating_sub(COLUMN_HEAD + view.tail.rows()));
    let (first, shown, hidden) = window(view.rows.len(), view.cursor, slots);
    let mut cells = Vec::new();
    if view.unavailable {
        cells.push(plain_cell(COLUMN_HEAD - 1, width, fit(UNAVAILABLE, full)));
    }
    for (slot, i) in (first..first + shown).enumerate() {
        let row = &view.rows[i];
        let cursor = i == view.cursor;
        let gutter = if cursor { g.gutter } else { ' ' };
        let mark = row.state.glyph(g);
        let name = fit_left(row.name.as_str(), name_w.max(1), g);
        let pad = name_w.saturating_sub(name.width());
        let text = fit(
            &format!("{gutter}{mark} {name}{}  {}", " ".repeat(pad), states[i]),
            full,
        );
        // The state starts after `gutter mark space name pad two-spaces`.
        let words = words_within(3 + name.width() + pad + 2, &states[i], full);
        cells.push(Cell {
            x: 0,
            y: COLUMN_HEAD + cells_u16(slot),
            width,
            text,
            index: Some(i),
            cursor,
            words,
            tone: row.state.tone(),
        });
    }
    if hidden > 0 && slots > 0 {
        let more = fit(&format!("   {} {hidden} more", g.more), full);
        cells.push(plain_cell(COLUMN_HEAD + cells_u16(shown), width, more));
    }
    Plan {
        label: Some((0, fit(HEADING, full))),
        cells,
        rows: height,
    }
}
