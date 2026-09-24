//! The column beside the document (`THREE_PANE_MIN` columns and up):
//! header, blank, one file per row with the states aligned. Planned here,
//! drawn and hit-tested by `files` through the same [`Plan`].

use unicode_width::UnicodeWidthStr;

use super::{Cell, FileListView, Glyphs, Plan, cells_u16, fit_left, window};
use crate::fit::fit;

/// The column's narrowest and widest (design note, "≥180 columns").
const COLUMN_MIN: u16 = 22;
/// See [`COLUMN_MIN`].
const COLUMN_MAX: u16 = 32;
/// Rows the column spends above its files: the `FILES · n` header and the
/// blank row where ticket 14's pane titles sit.
const COLUMN_HEAD: u16 = 2;

/// The column's width: its longest row, clamped to 22..=32.
pub(super) fn column_width(view: &FileListView) -> u16 {
    let g = Glyphs::UNICODE;
    let longest = view
        .rows
        .iter()
        .map(|row| 3 + row.name.as_str().width() + 2 + row.state.words(false, g).width())
        .chain(std::iter::once(column_header(view, g).width()))
        .max()
        .unwrap_or(0);
    cells_u16(longest).clamp(COLUMN_MIN, COLUMN_MAX)
}

fn column_header(view: &FileListView, g: Glyphs) -> String {
    if view.unavailable {
        format!("FILES {} git's list unavailable", g.dot)
    } else {
        format!("FILES {} {} conflicted", g.dot, view.rows.len())
    }
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
        cells.push(Cell {
            x: 0,
            y: COLUMN_HEAD + cells_u16(slot),
            width,
            text,
            index: Some(i),
            cursor,
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
        });
    }
    Plan {
        label: Some((0, fit(&column_header(view, g), full))),
        cells,
        rows: height,
    }
}
