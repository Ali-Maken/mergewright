//! The resolver's file list (m6-conflicts/13, ADR-0043 §1): every file this
//! step conflicted on, with its state and the open one marked.
//!
//! Two placements. Below [`INLINE_FROM`] columns it is a **strip** above
//! the document: one row when every `name · state` cell fits, then the same
//! row with compact states, then a grid of equal cells, at most three rows,
//! with `… N more` last. From [`INLINE_FROM`] it is a **column**
//! [`LIST_COLUMN`] wide beside the document, headed `Clashing files`, one
//! `? name  N open` / `✓ name  done` row per file (the merge mock's list,
//! ease-pass/11), a merge's `✓ Review the merge  y` last once nothing is
//! open ([`ListTail`]), and in a merge the files git combined on its own
//! ([`CombinedList`]).
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

use mergewright_term::SanitizedText;

mod column;
mod combined;
mod strip;
mod tail;
mod words;

use super::draw::set_text;
use super::{INLINE_FROM, LIST_COLUMN};
use crate::paint::foreground;
use crate::theme::{Theme, Token};
use column::column_plan;
pub use combined::CombinedList;
use strip::strip_plan;
pub use tail::{ListTail, review_row_at};
pub use words::FileRowLook;
use words::{WordsTone, paint_words, words_at_end};

/// One file in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRow {
    /// The path, as git names it.
    pub name: SanitizedText,
    /// Where it stands.
    pub state: FileRowLook,
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
    /// What the list draws after its files.
    pub tail: ListTail,
    /// A merge's files git combined on its own, last in the column.
    pub combined: CombinedList,
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

/// Where the list goes at this frame width: a column [`LIST_COLUMN`] wide
/// from [`INLINE_FROM`], else a strip as tall as it
/// needs.
#[must_use]
pub fn placement(view: &FileListView, frame_columns: u16) -> Placement {
    if frame_columns >= INLINE_FROM {
        Placement::Column { width: LIST_COLUMN }
    } else {
        Placement::Strip {
            rows: strip_plan(view, frame_columns, Glyphs::UNICODE).rows + view.tail.rows(),
        }
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
            paint_words(buf, at, cell, self.theme);
        }
        tail::draw_tail(self.view, self.placement, area, buf, self.theme, g);
        combined::draw_combined(self.view, self.placement, area, buf, self.theme, g);
    }
}

// ------------------------------------------------------------------ plan

/// Every listed file is done (and there is at least one): the list's head
/// says `all … done ✓` (m6-conflicts/60).
fn all_done(view: &FileListView) -> bool {
    !view.rows.is_empty() && view.rows.iter().all(|row| row.state == FileRowLook::Done)
}

/// The widget's own glyphs, one cell each in both tiers.
#[derive(Clone, Copy)]
struct Glyphs {
    mark: char,
    gutter: char,
    check: char,
    dot: char,
    more: char,
    enter: char,
    /// A file not read yet. Its own glyph: `more` also marks overflow and
    /// a left-cut path.
    unread: char,
}

impl Glyphs {
    const UNICODE: Glyphs = Glyphs {
        mark: '▸',
        gutter: '\u{258f}',
        check: '✓',
        dot: '·',
        more: '…',
        enter: '⏎',
        unread: '•',
    };
    const ASCII: Glyphs = Glyphs {
        mark: '>',
        gutter: '|',
        check: '*',
        dot: '-',
        more: '~',
        enter: '<',
        unread: '~',
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
    /// Where the state words sit in `text`, `(column, width)` in cells;
    /// `None` when there are none, or a cut took them.
    words: Option<(u16, u16)>,
    tone: WordsTone,
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

#[cfg(test)]
mod tests;
