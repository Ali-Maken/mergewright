//! The column beside the document (`THREE_PANE_MIN` columns and up):
//! header, blank, one file per row with the states aligned. Planned here,
//! drawn and hit-tested by `files` through the same [`Plan`].

use unicode_width::UnicodeWidthStr;

use super::{Cell, FileListView, Glyphs, Plan, WordsTone, cells_u16, fit_left, window};
use crate::fit::fit;

/// The column's narrowest and widest (design note, "≥180 columns").
const COLUMN_MIN: u16 = 22;
/// See [`COLUMN_MIN`].
const COLUMN_MAX: u16 = 32;
/// Rows the column spends above its files: the `FILES · n` header and the
/// blank row where ticket 14's pane titles sit.
const COLUMN_HEAD: u16 = 2;

/// The column's width: the longest name beside the widest state, clamped
/// to 22..=32. The same rule [`column_plan`] lays rows out by (states
/// aligned in one column), so a name is only cut when the clamp bites
/// (m6-conflicts/42).
pub(super) fn column_width(view: &FileListView) -> u16 {
    let g = Glyphs::UNICODE;
    let (name_w, state_w) = name_and_state_widths(view, g);
    let row = if view.rows.is_empty() {
        0
    } else {
        3 + name_w + 2 + state_w
    };
    cells_u16(row.max(column_header(view, g).width())).clamp(COLUMN_MIN, COLUMN_MAX)
}

/// The longest name and the widest state, each over every row.
fn name_and_state_widths(view: &FileListView, g: Glyphs) -> (usize, usize) {
    view.rows.iter().fold((0, 0), |(n, s), row| {
        (
            n.max(row.name.as_str().width()),
            s.max(row.state.words(false, g).width()),
        )
    })
}

fn column_header(view: &FileListView, g: Glyphs) -> String {
    if view.unavailable {
        format!("FILES {} git's list unavailable", g.dot)
    } else if super::all_done(view) {
        format!("FILES {} all {} done {}", g.dot, view.rows.len(), g.check)
    } else {
        format!("FILES {} {} conflicted", g.dot, view.rows.len())
    }
}

/// `words` at column `at` of a row `full` cells wide, as `(column, width)`;
/// `None` when the row's cut at `full` reaches them (left uncoloured).
fn words_within(at: usize, words: &str, full: usize) -> Option<(u16, u16)> {
    let width = words.width();
    (at + width <= full).then(|| (cells_u16(at), cells_u16(width)))
}

/// The column: header, blank, then one file per row, states aligned.
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
    let slots = usize::from(height.saturating_sub(COLUMN_HEAD));
    let (first, shown, hidden) = window(view.rows.len(), view.cursor, slots);
    let mut cells = Vec::new();
    for (slot, i) in (first..first + shown).enumerate() {
        let row = &view.rows[i];
        let cursor = i == view.cursor;
        let (gutter, mark) = if cursor {
            (g.gutter, g.mark)
        } else {
            (' ', ' ')
        };
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
        cells.push(Cell {
            x: 0,
            y: COLUMN_HEAD + cells_u16(shown),
            width,
            text: fit(&format!("   {} {hidden} more", g.more), full),
            index: None,
            cursor: false,
            words: None,
            tone: WordsTone::Plain,
        });
    }
    Plan {
        label: Some((0, fit(&column_header(view, g), full))),
        cells,
        rows: height,
    }
}
