//! The history place's picture (m7-power-tools/21; design note pass 1b
//! "History — the whole file, each change marked", pass 2b H1): a title,
//! then the commits that changed the lines (or the file) on the left and the
//! whole file at the selected one on the right, that commit's adds and
//! removes marked in place.
//!
//! A pure draw from [`HistoryView`]; no state, no keys. The bar under it is
//! the frame's bottom row, drawn by the hint catalogue, not here.
//!
//! The file pane's unified body is this module's own (ruling (c) of 21): a
//! `┃` column for the traced range, **one** number column (the new side's;
//! a removed row has none), a sign column, then the text. Each row's look —
//! sign colour, backgrounds, word tint, syntax — is the detail pane's, drawn
//! by the helpers in `detail_pane::line_render`, so the two diffs match by
//! construction. Side by side is the detail pane's own renderer, unchanged,
//! with no `┃`.

use std::ops::RangeInclusive;

use graft_core::capabilities::ColorDepth;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::detail::{HunkView, LineKindView, LineView};
use crate::detail_pane::diff_body_side_by_side;
use crate::detail_pane::line_render::{
    DiffRowStyle, draw_styled_text, gutter_style, line_style_at,
};
use crate::fit::fit;
use crate::graph_screen::{initials, relative_date};
use crate::keymap::{ActionId, Keymap};
use crate::theme::{Theme, Token};

mod file_pane;
mod list;

use file_pane::{draw_file, wrap};
use list::{draw_list, row_y};

/// The list's width at ≥180 columns (pass 1b's breakpoints).
pub const LIST_WIDE: u16 = 56;

/// The list's width at 120–179 columns, the author as initials.
pub const LIST_MIDDLE: u16 = 40;

/// Below this width one pane is shown at a time, and `Tab` swaps them.
pub const ONE_PANE_BELOW: u16 = 120;

/// The bracket marking the traced range, leftmost in the file pane.
pub const RANGE_MARK: char = '\u{2503}';

/// The selected commit's marker in the list.
pub const SELECTED: char = '\u{25b8}';

/// Which pane has the keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryFocus {
    /// The commits.
    List,
    /// The whole file at the selected commit.
    File,
}

/// One commit in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryRow<'a> {
    /// Short sha, 7 hex digits.
    pub sha7: &'a str,
    /// Subject line, sanitised.
    pub subject: &'a str,
    /// Author name, sanitised.
    pub who: &'a str,
    /// Author time, Unix seconds.
    pub when: i64,
    /// Lines added (the range's, or the file's).
    pub added: u32,
    /// Lines removed, the same way.
    pub removed: u32,
    /// The path before, when this commit renamed the file: a
    /// `↳ was <path>` row under it.
    pub was: Option<&'a str>,
}

/// The list, by state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryList<'a> {
    /// The rows (none while tracing, or on a clone with nothing to show),
    /// the selected one, the first drawn, and the end row's words.
    Rows {
        /// Newest first.
        rows: &'a [HistoryRow<'a>],
        /// The selected row; `None` when there is none to select.
        selected: Option<usize>,
        /// The first row drawn (a commit's index; its `was` row follows it).
        top: usize,
        /// The last row's words (`the file starts here …`), drawn after the
        /// last commit.
        end: Option<&'a str>,
    },
    /// The trace failed: its words (`couldn't trace: …` is added here).
    Failed(&'a str),
}

/// The file pane, by state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilePane<'a> {
    /// Nothing to show yet (tracing, or the trace failed).
    Empty,
    /// No commit to show the file at: `no change to show`, dim.
    NoChange,
    /// The file is being read.
    Reading {
        /// The selected commit's header.
        header: &'a str,
    },
    /// The read failed; its words.
    Failed(&'a str),
    /// A binary file: its sizes before and after, when known.
    Binary {
        /// The selected commit's header.
        header: &'a str,
        /// Bytes before the commit.
        old: Option<u64>,
        /// Bytes after it.
        new: Option<u64>,
        /// Why the sizes could not be read; drawn dim in their place.
        sizes_failed: Option<&'a str>,
    },
    /// The whole file.
    Lines {
        /// The selected commit's header.
        header: &'a str,
        /// The file as one hunk (`graft_git::trace::whole_file`).
        hunks: &'a [HunkView],
        /// The first body row drawn ([`file_rows`]' index).
        top: usize,
        /// The traced lines where they sit in this commit: the `┃` bracket.
        range: Option<RangeInclusive<u32>>,
        /// `v`: the detail pane's side-by-side body, no `┃`.
        side_by_side: bool,
        /// Changed words get the stronger tint.
        word_tint: bool,
    },
}

/// Everything the history place draws.
#[derive(Clone, Debug)]
pub struct HistoryView<'a> {
    /// The title's words after `HISTORY · `: the path and the range.
    pub title: &'a str,
    /// git is still tracing: the title says so, the panes are blank.
    pub tracing: bool,
    /// The commits.
    pub list: HistoryList<'a>,
    /// The file at the selected commit.
    pub file: FilePane<'a>,
    /// Which pane has the keys (and, below 120 columns, which is shown).
    pub focus: HistoryFocus,
    /// The clock the ages are read against, Unix seconds.
    pub now: i64,
    /// Colours for the file pane's rows (21: a field added to the note's
    /// signature — the rows are drawn by the detail pane's styling helpers).
    pub theme: &'a Theme,
    /// `w`: whitespace-only changes are ignored; the title's tail says so,
    /// muted (m8-polish-and-ship/17 item 8).
    pub whitespace_ignored: bool,
}

/// Where the two panes sit across the width: `(x offset, width)` each,
/// `None` for a pane not shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns {
    /// The list.
    pub list: Option<(u16, u16)>,
    /// The file pane.
    pub file: Option<(u16, u16)>,
}

/// The breakpoints: ≥180 list 56, the file the rest; 120–179 list 40;
/// below, the focused pane alone. A `│` rule sits between two panes.
#[must_use]
pub fn columns(width: u16, focus: HistoryFocus) -> Columns {
    let list = match width {
        180.. => LIST_WIDE,
        ONE_PANE_BELOW..=179 => LIST_MIDDLE,
        _ => {
            return match focus {
                HistoryFocus::List => Columns {
                    list: Some((0, width)),
                    file: None,
                },
                HistoryFocus::File => Columns {
                    list: None,
                    file: Some((0, width)),
                },
            };
        }
    };
    Columns {
        list: Some((0, list)),
        file: Some((list + 1, width.saturating_sub(list + 1))),
    }
}

/// The file pane's body rows: every line of the hunks but their headings,
/// in order. The index [`FilePane::Lines`]' `top` counts in.
#[must_use]
pub fn file_rows(hunks: &[HunkView]) -> Vec<&LineView> {
    hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| line.kind != LineKindView::HunkHeading)
        .collect()
}

/// Which of [`file_rows`] are changes (added or removed), for `n`/`N`.
#[must_use]
pub fn changed_rows(hunks: &[HunkView]) -> Vec<bool> {
    file_rows(hunks)
        .iter()
        .map(|line| matches!(line.kind, LineKindView::Added | LineKindView::Removed))
        .collect()
}

/// The body row showing new-side line `line` (the first at or after it);
/// the last row when the file is shorter.
#[must_use]
pub fn row_of_new_line(hunks: &[HunkView], line: u32) -> Option<usize> {
    let rows = file_rows(hunks);
    rows.iter()
        .position(|row| row.new_no.is_some_and(|no| no >= line))
        .or_else(|| rows.len().checked_sub(1))
}

/// The body row showing old-side line `line` (a removed line, or a line
/// the commit kept), the first at or after it.
#[must_use]
pub fn row_of_old_line(hunks: &[HunkView], line: u32) -> Option<usize> {
    let rows = file_rows(hunks);
    rows.iter()
        .position(|row| row.old_no.is_some_and(|no| no >= line))
        .or_else(|| rows.len().checked_sub(1))
}

/// How many screen rows the list's commit `index` takes: one, and one more
/// for its `↳ was` row.
fn rows_for(row: &HistoryRow) -> usize {
    1 + usize::from(row.was.is_some())
}

/// The commit drawn at `line` rows under the list's first row, when the
/// list starts at commit `top` — its `↳ was` row counts as the commit's.
/// `None` past the last commit (the end row, or nothing).
#[must_use]
pub fn list_index_at(rows: &[HistoryRow], top: usize, line: usize) -> Option<usize> {
    let mut at = 0;
    for (index, row) in rows.iter().enumerate().skip(top) {
        let next = at + rows_for(row);
        if line < next {
            return Some(index);
        }
        at = next;
    }
    None
}

/// Screen rows the commits `top..=index` take, for keeping `index` in view.
#[must_use]
pub fn rows_through(rows: &[HistoryRow], top: usize, index: usize) -> usize {
    rows.iter()
        .enumerate()
        .skip(top)
        .take_while(|(at, _)| *at <= index)
        .map(|(_, row)| rows_for(row))
        .sum()
}

/// Draws the history place into `area`, its keys spelled from `keys`.
pub fn draw_history(frame: &mut Frame, area: Rect, view: &HistoryView, keys: &Keymap) {
    draw_into(frame.buffer_mut(), area, view, keys);
}

fn draw_into(buf: &mut Buffer, area: Rect, view: &HistoryView, keys: &Keymap) {
    if area.is_empty() {
        return;
    }
    let title = title(view, keys);
    put(
        buf,
        area.x,
        area.y,
        area.width,
        &title,
        Style::default().add_modifier(Modifier::BOLD),
    );
    if view.whitespace_ignored {
        crate::blame::put_whitespace_tail(buf, area, &title, put);
    }
    if area.height < 2 {
        return;
    }
    let body_y = area.y + 1;
    let body_height = area.height - 1;
    // A failed trace has no list and no file: its words take the whole
    // row, on one line (H1), not the list's 40 or 56 columns.
    if let HistoryList::Failed(_) = view.list {
        let whole = Rect {
            x: area.x,
            y: body_y,
            width: area.width,
            height: body_height,
        };
        draw_list(buf, whole, view);
        return;
    }
    let columns = columns(area.width, view.focus);
    if let (Some((list_x, list_width)), Some(_)) = (columns.list, columns.file) {
        let rule_x = area.x + list_x + list_width;
        for y in body_y..body_y + body_height {
            put(buf, rule_x, y, 1, "\u{2502}", dim());
        }
    }
    if let Some((x, width)) = columns.list {
        let list_area = Rect {
            x: area.x + x,
            y: body_y,
            width,
            height: body_height,
        };
        draw_list(buf, list_area, view);
    }
    if let Some((x, width)) = columns.file {
        // One cell of air after the rule.
        let (x, width) = if columns.list.is_some() {
            (x + 1, width.saturating_sub(1))
        } else {
            (x, width)
        };
        let file_area = Rect {
            x: area.x + x,
            y: body_y,
            width,
            height: body_height,
        };
        draw_file(buf, file_area, view);
    }
}

/// The title row: the path and range, then the state.
fn title(view: &HistoryView, keys: &Keymap) -> String {
    let mut title = format!(" HISTORY \u{b7} {}", view.title);
    if view.tracing {
        title = keys.after(
            &format!("{title} \u{b7} tracing\u{2026}"),
            ActionId("app.back"),
            "stops",
        );
    } else if let HistoryList::Rows { rows, .. } = &view.list
        && !rows.is_empty()
    {
        let changes = if rows.len() == 1 { "change" } else { "changes" };
        title.push_str(&format!(" \u{b7} {} {changes}", rows.len()));
    }
    title
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// `text` padded with spaces to `cells` (already fit).
fn pad(text: &str, cells: usize) -> String {
    use unicode_width::UnicodeWidthStr;
    let width = text.width();
    format!("{text}{}", " ".repeat(cells.saturating_sub(width)))
}

/// Writes `text` at (`x`, `y`), clipped to `width` cells.
fn put(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }
    buf.set_stringn(x, y, text, usize::from(width), style);
}

#[cfg(test)]
mod tests;
