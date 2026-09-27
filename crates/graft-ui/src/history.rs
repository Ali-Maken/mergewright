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
use crate::theme::{Theme, Token};

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

/// Draws the history place into `area`.
pub fn draw_history(frame: &mut Frame, area: Rect, view: &HistoryView) {
    draw_into(frame.buffer_mut(), area, view);
}

fn draw_into(buf: &mut Buffer, area: Rect, view: &HistoryView) {
    if area.is_empty() {
        return;
    }
    put(
        buf,
        area.x,
        area.y,
        area.width,
        &title(view),
        Style::default().add_modifier(Modifier::BOLD),
    );
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
fn title(view: &HistoryView) -> String {
    let mut title = format!(" HISTORY \u{b7} {}", view.title);
    if view.tracing {
        title.push_str(" \u{b7} tracing\u{2026} \u{b7} esc stops");
    } else if let HistoryList::Rows { rows, .. } = &view.list
        && !rows.is_empty()
    {
        let changes = if rows.len() == 1 { "change" } else { "changes" };
        title.push_str(&format!(" \u{b7} {} {changes}", rows.len()));
    }
    title
}

fn draw_list(buf: &mut Buffer, area: Rect, view: &HistoryView) {
    if view.tracing {
        return;
    }
    match &view.list {
        HistoryList::Failed(words) => {
            let text = format!(" couldn't trace: {words}");
            put(buf, area.x, area.y, area.width, &text, Style::default());
        }
        HistoryList::Rows {
            rows,
            selected,
            top,
            end,
        } => {
            let initials_only = area.width < LIST_WIDE;
            let height = usize::from(area.height);
            let mut line = 0usize;
            for (index, row) in rows.iter().enumerate().skip(*top) {
                if line >= height {
                    return;
                }
                let chosen = *selected == Some(index);
                let text = list_row(
                    row,
                    chosen,
                    initials_only,
                    usize::from(area.width),
                    view.now,
                );
                let style = if chosen && view.focus == HistoryFocus::List {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else if chosen {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                put(buf, area.x, row_y(area, line), area.width, &text, style);
                line += 1;
                if let Some(was) = row.was {
                    if line >= height {
                        return;
                    }
                    let text = format!("   \u{21b3} was {was}");
                    put(buf, area.x, row_y(area, line), area.width, &text, dim());
                    line += 1;
                }
            }
            // The end row wraps: its words are the reason the list stops,
            // and at 40 columns git's reason would otherwise be cut.
            let indent = 3;
            let room = usize::from(area.width).saturating_sub(indent).max(1);
            for words in end.map(|end| wrap(end, room)).unwrap_or_default() {
                if line >= height {
                    return;
                }
                let text = format!("{}{words}", " ".repeat(indent));
                put(buf, area.x, row_y(area, line), area.width, &text, dim());
                line += 1;
            }
        }
    }
}

fn row_y(area: Rect, line: usize) -> u16 {
    area.y + u16::try_from(line).unwrap_or(u16::MAX)
}

/// One commit: `▸ sha7  subject  who age +a −r`, fit to `width`.
fn list_row(row: &HistoryRow, chosen: bool, initials_only: bool, width: usize, now: i64) -> String {
    const WHO: usize = 6;
    const AGE: usize = 4;
    const COUNTS: usize = 6;
    let marker = if chosen { SELECTED } else { ' ' };
    let who = if initials_only {
        initials(row.who)
    } else {
        fit(row.who, WHO)
    };
    let who_cells = if initials_only { 2 } else { WHO };
    // ` ▸ ` + sha7 + a space, then the subject; after it ` who age counts`.
    let fixed = 3 + 7 + 1 + 1 + who_cells + 1 + AGE + 1 + COUNTS;
    let subject_cells = width.saturating_sub(fixed).max(1);
    format!(
        " {marker} {sha:<7} {subject} {who} {age:>AGE$} {counts:<COUNTS$}",
        sha = row.sha7,
        subject = pad(&fit(row.subject, subject_cells), subject_cells),
        who = pad(&who, who_cells),
        age = relative_date(row.when, now),
        counts = counts(row.added, row.removed),
    )
}

/// `+a −r`, each only when non-zero.
fn counts(added: u32, removed: u32) -> String {
    let mut parts = Vec::new();
    if added > 0 {
        parts.push(format!("+{added}"));
    }
    if removed > 0 {
        parts.push(format!("\u{2212}{removed}"));
    }
    parts.join(" ")
}

fn draw_file(buf: &mut Buffer, area: Rect, view: &HistoryView) {
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
        FilePane::Failed(words) => put(buf, area.x, area.y, area.width, words, Style::default()),
        FilePane::Reading { header } => {
            put(buf, area.x, area.y, area.width, header, bold);
            if !below.is_empty() {
                put(buf, below.x, below.y, below.width, "reading\u{2026}", dim());
            }
        }
        FilePane::Binary { header, .. } => {
            put(buf, area.x, area.y, area.width, header, bold);
            if !below.is_empty() {
                let (words, style) = binary_words(&view.file);
                put(buf, below.x, below.y, below.width, &words, style);
            }
        }
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

/// A binary file's line: `binary · <old> → <new>`, or, dim, why the sizes
/// could not be read (21c).
/// Empty for any other pane.
fn binary_words(pane: &FilePane) -> (String, Style) {
    match pane {
        FilePane::Binary {
            sizes_failed: Some(why),
            ..
        } => (format!("binary \u{b7} couldn't read sizes: {why}"), dim()),
        FilePane::Binary { old, new, .. } => (
            format!("binary \u{b7} {} \u{2192} {}", size(*old), size(*new)),
            Style::default(),
        ),
        FilePane::Empty
        | FilePane::NoChange
        | FilePane::Reading { .. }
        | FilePane::Failed(_)
        | FilePane::Lines { .. } => (String::new(), Style::default()),
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
fn wrap(text: &str, room: usize) -> Vec<String> {
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
