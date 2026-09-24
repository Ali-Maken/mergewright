//! The resolver's file list (m6-conflicts/13, ADR-0043 §1): every file this
//! step conflicted on, with its state and the open one marked.
//!
//! Two placements. Below [`super::THREE_PANE_MIN`] columns it is a **strip**
//! above the document: one row when every `name · state` cell fits, then
//! the same row with compact states, then a grid of equal cells, at most
//! three rows, with `… N more` last. At [`super::THREE_PANE_MIN`] and wider
//! it is a **column** beside the document.
//!
//! **One geometry.** [`FileList`] draws and [`file_at`] hit-tests through
//! the same private plan (`strip_plan` / `column_plan`), so a click lands on
//! the file whose name is drawn under it by construction. Every glyph the
//! plan places is one cell wide in both tiers, so the ASCII tier draws the
//! same geometry with different glyphs.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use graft_term::SanitizedText;

use super::draw::set_text;
use super::{Arrangement, SIDE_BY_SIDE_MIN, arrangement};
use crate::fit::fit;
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// The column's narrowest and widest (design note, "≥180 columns").
const COLUMN_MIN: u16 = 22;
/// See [`COLUMN_MIN`].
const COLUMN_MAX: u16 = 32;
/// Rows the column spends above its files: the `FILES · n` header and the
/// blank row where ticket 14's pane titles sit.
const COLUMN_HEAD: u16 = 2;
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

/// One file in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRow {
    /// The path, as git names it.
    pub name: SanitizedText,
    /// Where it stands.
    pub state: FileState,
}

/// Where one listed file stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileState {
    /// Open in the resolver (or seen there) with `open` of `of` blocks left.
    Open {
        /// Blocks still open.
        open: usize,
        /// Blocks in the file.
        of: usize,
    },
    /// Every block taken; `⏎` marks it done.
    AllTaken,
    /// git no longer calls it unmerged.
    Done,
    /// A whole-file shape (modify/delete, add/add without markers, …).
    WholeFile {
        /// The shape, in words with each side's one name — for ticket 14's
        /// panes; the list itself draws `keep or delete`.
        how: SanitizedText,
    },
    /// A binary file: kept whole from one side.
    Binary,
    /// Not read yet.
    Unread,
}

impl FileState {
    /// The state's words: full, or compact (`1/2 open`, `✓`).
    fn words(&self, compact: bool, g: Glyphs) -> String {
        match (self, compact) {
            (FileState::Open { open, of }, false) => format!("{open} of {of} open"),
            (FileState::Open { open, of }, true) => format!("{open}/{of} open"),
            (FileState::AllTaken, false) => format!("{} marks done", g.enter),
            (FileState::AllTaken, true) => g.enter.to_string(),
            (FileState::Done, false) => format!("done {}", g.check),
            (FileState::Done, true) => g.check.to_string(),
            (FileState::WholeFile { .. }, false) => "keep or delete".to_owned(),
            (FileState::WholeFile { .. }, true) => "keep?".to_owned(),
            (FileState::Binary, false) => "binary".to_owned(),
            (FileState::Binary, true) => "bin".to_owned(),
            (FileState::Unread, _) => g.more.to_string(),
        }
    }
}

/// The list, ready to draw.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileListView {
    /// Every listed file, in the app's order.
    pub rows: Vec<FileRow>,
    /// The open file's index in `rows`.
    pub cursor: usize,
    /// git's list could not be read: `rows` are the unmerged paths only.
    pub unavailable: bool,
}

/// Where the list sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Beside the document, `width` cells wide.
    Column {
        /// The column's width, without its `│`.
        width: u16,
    },
    /// Above the document, `rows` rows tall, without its rule row.
    Strip {
        /// The strip's rows.
        rows: u16,
    },
}

/// Where the list goes at this frame width: a column at
/// [`super::THREE_PANE_MIN`] and up, else a strip as tall as it needs.
#[must_use]
pub fn placement(view: &FileListView, frame_columns: u16) -> Placement {
    match arrangement(frame_columns) {
        Arrangement::ThreePane => Placement::Column {
            width: column_width(view),
        },
        Arrangement::Card => Placement::Strip {
            rows: strip_plan(view, frame_columns, Glyphs::UNICODE).rows,
        },
    }
}

/// The resolver's area split between the list and the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolverAreas {
    /// Where [`FileList`] draws.
    pub list: Rect,
    /// Where the title, notice and document draw.
    pub doc: Rect,
}

/// Splits `area`: a strip, one rule row, the document; or a column, one
/// `│` column, the document at full height. The draw and every click go
/// through this one function.
#[must_use]
pub fn resolver_areas(area: Rect, placement: Placement) -> ResolverAreas {
    match placement {
        // No strip, no rule: the whole area is the document's.
        Placement::Strip { rows: 0 } => ResolverAreas {
            list: Rect { height: 0, ..area },
            doc: area,
        },
        Placement::Strip { rows } => {
            let rows = rows.min(area.height);
            let taken = rows.saturating_add(1).min(area.height);
            ResolverAreas {
                list: Rect {
                    height: rows,
                    ..area
                },
                doc: Rect {
                    y: area.y.saturating_add(taken),
                    height: area.height - taken,
                    ..area
                },
            }
        }
        Placement::Column { width } => {
            let width = width.min(area.width);
            let taken = width.saturating_add(1).min(area.width);
            ResolverAreas {
                list: Rect { width, ..area },
                doc: Rect {
                    x: area.x.saturating_add(taken),
                    width: area.width - taken,
                    ..area
                },
            }
        }
    }
}

/// The file drawn at `(dx, dy)` from the list's origin, in a list
/// `width` x `height` cells — `None` on the label, a blank, or `… N more`.
///
/// Takes `height` as well as the ticket's `width`: a column longer than its
/// rows windows its files, and which file sits on a row depends on how many
/// rows there are.
#[must_use]
pub fn file_at(
    view: &FileListView,
    placement: Placement,
    width: u16,
    height: u16,
    dx: u16,
    dy: u16,
) -> Option<usize> {
    plan(view, placement, width, height, Glyphs::UNICODE)
        .cells
        .iter()
        .find(|cell| cell.y == dy && dx >= cell.x && dx < cell.x.saturating_add(cell.width))
        .and_then(|cell| cell.index)
}

/// The list widget. Draws only inside its `Rect`.
pub struct FileList<'a> {
    /// What to draw.
    pub view: &'a FileListView,
    /// Strip or column.
    pub placement: Placement,
    /// The theme to draw in.
    pub theme: &'a Theme,
    /// Draw the ASCII tier's glyphs.
    pub ascii: bool,
}

impl Widget for FileList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let g = if self.ascii {
            Glyphs::ASCII
        } else {
            Glyphs::UNICODE
        };
        let laid = plan(self.view, self.placement, area.width, area.height, g);
        if let Some((y, label)) = &laid.label {
            set_text(
                buf,
                area.x,
                area.y.saturating_add(*y),
                area,
                label,
                foreground(self.theme, Token::TextMuted),
            );
        }
        for cell in &laid.cells {
            let style = match (cell.index, cell.cursor) {
                (None, _) => foreground(self.theme, Token::TextMuted),
                (Some(_), true) => foreground(self.theme, Token::Text).add_modifier(Modifier::BOLD),
                (Some(_), false) => foreground(self.theme, Token::Text),
            };
            let at = Rect {
                x: area.x.saturating_add(cell.x),
                y: area.y.saturating_add(cell.y),
                width: cell.width,
                height: 1,
            }
            .intersection(area);
            set_text(buf, at.x, at.y, at, &cell.text, style);
        }
    }
}

// ------------------------------------------------------------------ plan

/// The widget's own glyphs, one cell each in both tiers.
#[derive(Clone, Copy)]
struct Glyphs {
    mark: char,
    gutter: char,
    check: char,
    dot: char,
    more: char,
    enter: char,
}

impl Glyphs {
    const UNICODE: Glyphs = Glyphs {
        mark: '▸',
        gutter: '\u{258f}',
        check: '✓',
        dot: '·',
        more: '…',
        enter: '⏎',
    };
    const ASCII: Glyphs = Glyphs {
        mark: '>',
        gutter: '|',
        check: '*',
        dot: '-',
        more: '~',
        enter: '<',
    };
}

/// One drawn cell: where, how wide, what, and which file it is.
struct Cell {
    x: u16,
    y: u16,
    width: u16,
    text: String,
    /// `None` for `… N more`.
    index: Option<usize>,
    cursor: bool,
}

/// Everything the list draws, positioned.
struct Plan {
    /// `(row, text)`.
    label: Option<(u16, String)>,
    cells: Vec<Cell>,
    /// Rows used.
    rows: u16,
}

fn plan(view: &FileListView, placement: Placement, width: u16, height: u16, g: Glyphs) -> Plan {
    match placement {
        Placement::Strip { .. } => strip_plan(view, width, g),
        Placement::Column { .. } => column_plan(view, width, height, g),
    }
}

fn cells_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// `text` cut from the **left** with `…` to `width` cells: the file name at
/// the end of a path survives.
fn fit_left(text: &str, width: usize, g: Glyphs) -> String {
    if text.width() <= width {
        return text.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut tail: Vec<char> = Vec::new();
    let mut used = 1;
    for c in text.chars().rev() {
        let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if used + w > width {
            break;
        }
        used += w;
        tail.push(c);
    }
    let mut out = String::with_capacity(width);
    out.push(g.more);
    out.extend(tail.into_iter().rev());
    out
}

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
fn strip_plan(view: &FileListView, width: u16, g: Glyphs) -> Plan {
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

/// Which files `slots` places show: `(first, shown, hidden)`. Everything
/// when it all fits; else `slots - 1` consecutive files and a `… N more`
/// slot, starting as early as keeps `cursor` among them.
fn window(n: usize, cursor: usize, slots: usize) -> (usize, usize, usize) {
    if n <= slots {
        return (0, n, 0);
    }
    if slots <= 1 {
        return (0, 0, n);
    }
    let files = slots - 1;
    let first = if cursor < files {
        0
    } else {
        (cursor + 1 - files).min(n - files)
    };
    (first, files, n - files)
}

/// The column's width: its longest row, clamped to 22..=32.
fn column_width(view: &FileListView) -> u16 {
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
fn column_plan(view: &FileListView, width: u16, height: u16, g: Glyphs) -> Plan {
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

#[cfg(test)]
mod tests;
