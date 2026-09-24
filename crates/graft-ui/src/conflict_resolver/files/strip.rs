//! The strip above the document (below `THREE_PANE_MIN` columns): one row
//! when every `name · state` cell fits, then one compact row, then a grid
//! of equal cells with `… N more` last. Planned here, drawn and hit-tested
//! by `files` through the same [`Plan`].

use unicode_width::UnicodeWidthStr;

use super::super::SIDE_BY_SIDE_MIN;
use super::{Cell, FileListView, FileRow, FileState, Glyphs, Plan, cells_u16, fit_left, window};
use crate::fit::fit;

/// The most rows the strip takes, label row included.
const STRIP_MAX_ROWS: u16 = 3;
/// Cells between the strip's label and its first cell, and between grid
/// cells and compact one-row cells.
const GAP: usize = 3;
/// Cells between full-form cells on the one-row strip.
const WIDE_GAP: usize = 4;
/// A grid cell narrower than this beside the label puts the label on a row
/// of its own instead.
const GRID_CELL_MIN: usize = 20;

fn strip_label(view: &FileListView, grid: bool, g: Glyphs) -> String {
    if view.unavailable {
        return format!(
            "files: git's list unavailable {} the unmerged ones below",
            g.dot
        );
    }
    if !grid {
        return "files".to_owned();
    }
    let done = view
        .rows
        .iter()
        .filter(|row| row.state == FileState::Done)
        .count();
    let n = view.rows.len();
    if done == 0 {
        format!("files {} {n}", g.dot)
    } else {
        format!("files {} {n} {} {done} done", g.dot, g.dot)
    }
}

/// One cell of the one-row strip: `▸ notes.txt · 1 of 2 open`, or compact
/// `✓ gone.txt` / `▸ notes.txt 1/2 open`.
fn one_row_cell(row: &FileRow, cursor: bool, compact: bool, g: Glyphs) -> String {
    let mark = if cursor {
        format!("{} ", g.mark)
    } else {
        String::new()
    };
    let name = row.name.as_str();
    match (compact, &row.state) {
        (true, FileState::Done) => format!("{mark}{} {name}", g.check),
        (true, state) => format!("{mark}{name} {}", state.words(true, g)),
        (false, state) => format!("{mark}{name} {} {}", g.dot, state.words(false, g)),
    }
}

/// The strip: one row, one compact row, or a grid.
pub(super) fn strip_plan(view: &FileListView, width: u16, g: Glyphs) -> Plan {
    let full = usize::from(width);
    let forms: &[bool] = if width < SIDE_BY_SIDE_MIN {
        &[true]
    } else {
        &[false, true]
    };
    let label = strip_label(view, false, g);
    for &compact in forms {
        let gap = if compact { GAP } else { WIDE_GAP };
        let texts: Vec<String> = view
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| one_row_cell(row, i == view.cursor, compact, g))
            .collect();
        let used = label.width()
            + GAP
            + texts.iter().map(|t| t.width()).sum::<usize>()
            + gap * texts.len().saturating_sub(1);
        if used <= full {
            let mut x = label.width() + GAP;
            let cells = texts
                .into_iter()
                .enumerate()
                .map(|(i, text)| {
                    let w = text.width();
                    let cell = Cell {
                        x: cells_u16(x),
                        y: 0,
                        width: cells_u16(w),
                        text,
                        index: Some(i),
                        cursor: i == view.cursor,
                    };
                    x += w + gap;
                    cell
                })
                .collect();
            return Plan {
                label: Some((0, label)),
                cells,
                rows: 1,
            };
        }
    }
    grid_plan(view, full, width < SIDE_BY_SIDE_MIN, g)
}

/// One grid cell: the mark, the name shortened from the left so the file
/// name survives, then the state — or the name alone when the state leaves
/// it no room.
fn grid_cell_text(name: &str, state: &str, cursor: bool, cell_w: usize, g: Glyphs) -> String {
    let mark = if cursor { g.mark } else { ' ' };
    let name_room = cell_w.saturating_sub(2 + 1 + state.width());
    if name_room == 0 {
        fit(&format!("{mark} {name}"), cell_w)
    } else {
        format!("{mark} {} {state}", fit_left(name, name_room, g))
    }
}

/// The strip as a grid of equal cells, row-major, at most
/// [`STRIP_MAX_ROWS`] rows with the label, the cursor's row kept on screen.
fn grid_plan(view: &FileListView, full: usize, compact: bool, g: Glyphs) -> Plan {
    let label = strip_label(view, true, g);
    let states: Vec<String> = view
        .rows
        .iter()
        .map(|row| row.state.words(compact, g))
        .collect();
    let widest = view
        .rows
        .iter()
        .zip(&states)
        .map(|(row, state)| 2 + row.name.as_str().width() + 1 + state.width())
        .max()
        .unwrap_or(0);
    let beside = label.width() + GAP;
    let (x0, top, max_rows) = if full.saturating_sub(beside) >= widest.min(GRID_CELL_MIN) {
        (beside, 0u16, usize::from(STRIP_MAX_ROWS))
    } else {
        (0, 1u16, usize::from(STRIP_MAX_ROWS) - 1)
    };
    let room = full.saturating_sub(x0).max(1);
    let cell_w = widest.min(room).max(1);
    let cols = ((room + GAP) / (cell_w + GAP)).max(1);
    let (first, shown, hidden) = window(view.rows.len(), view.cursor, cols * max_rows);
    let mut cells = Vec::new();
    for (slot, i) in (first..first + shown).enumerate() {
        let cursor = i == view.cursor;
        cells.push(Cell {
            x: cells_u16(x0 + (slot % cols) * (cell_w + GAP)),
            y: top + cells_u16(slot / cols),
            width: cells_u16(cell_w),
            text: grid_cell_text(view.rows[i].name.as_str(), &states[i], cursor, cell_w, g),
            index: Some(i),
            cursor,
        });
    }
    if hidden > 0 {
        cells.push(Cell {
            x: cells_u16(x0 + (shown % cols) * (cell_w + GAP)),
            y: top + cells_u16(shown / cols),
            width: cells_u16(cell_w),
            text: fit(&format!("{} {hidden} more", g.more), cell_w),
            index: None,
            cursor: false,
        });
    }
    let slots = shown + usize::from(hidden > 0);
    let rows = top + cells_u16(slots.div_ceil(cols).max(1));
    Plan {
        label: Some((0, fit(&label, full))),
        cells,
        rows,
    }
}
