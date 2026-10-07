//! The command palette and help (`?`, `⌃P`, `:`; mock-ups §1.16, keymap
//! §2.12, ADR-0056): one box, two ways in. `?` lists this place's keys in
//! groups (`IN PROGRESS`, `HERE`, `EVERYWHERE`, `YOUR CONFIG` first when
//! his config had problems); `⌃P`/`:` lists every action.
//!
//! Pure drawing. `graft` owns the state, builds the groups from the live
//! key table, filters them and hands over a [`PaletteView`] with every
//! string worded and sanitized. The scroll is worked out here from the
//! cursor, never stored, so it cannot go stale behind a resize (the
//! picker's rule, [`crate::picker::first_drawn`]).
//!
//! The palette is a text field (§2.1): its own keys spell from
//! [`Keymap::shipped`] whatever his config says.

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::frame_box::{cells, content_row};
use crate::keymap::{ActionId, ContextId, KeyLabel, Keymap};
use crate::paint::foreground;
use crate::text_input::draw_line;
use crate::theme::{Theme, Token};

/// The widest the box gets (§1.16).
pub const MAX_WIDTH: u16 = 96;

/// Below this many frame columns the box takes the frame's full width and
/// rows drop their config id (§1.16).
pub const WIDE_FROM: u16 = 100;

/// The one line drawn when the query keeps no row; the key that closes is
/// spelled after it (`nothing matches · esc close`).
pub const NOTHING_MATCHES: &str = "nothing matches";

/// Cells the key column takes (§1.16).
const KEY_CELLS: usize = 6;

/// Lines that are not list lines: the two borders and the query.
const CHROME_ROWS: u16 = 3;

/// Which way the palette was opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteMode {
    /// `?`: the keys of the place it was opened on.
    Here(ContextId),
    /// `⌃P` / `:`: every action.
    Everything,
}

/// What a row stands for. Only actions and app rows exist so far; ticket
/// 12 adds the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// A key-table action.
    Action,
    /// An app row (`Theme: light`) or a note about his config.
    App,
    /// A branch (ticket 12).
    Branch,
    /// A tag (ticket 12).
    Tag,
    /// A file (ticket 12).
    File,
    /// A commit (ticket 12).
    Commit,
    /// A recently run command (ticket 12).
    Recent,
}

/// One row, worded.
#[derive(Clone, Debug, PartialEq)]
pub struct PaletteRow {
    /// What it does (`Merge the selected branch into yours`).
    pub label: SanitizedText,
    /// Its key in the place the palette was opened on; `Some("—")` for an
    /// action with no key there, `None` for a row that has no key at all
    /// (an app row).
    pub key: Option<KeyLabel>,
    /// Its id in his config (`graph.merge`), drawn muted at the right.
    pub config: Option<ActionId>,
    /// What it stands for.
    pub kind: RowKind,
}

/// Everything the box draws.
#[derive(Clone, Debug, PartialEq)]
pub struct PaletteView {
    /// Which way it was opened: the title.
    pub mode: PaletteMode,
    /// What he has typed.
    pub query: SanitizedText,
    /// Where the cursor stands in `query`, in characters.
    pub query_cursor: usize,
    /// The groups in drawn order, each a header and its rows; empty when
    /// the query keeps nothing.
    pub groups: Vec<(SanitizedText, Vec<PaletteRow>)>,
    /// The row under the cursor: an index over every row, groups
    /// flattened.
    pub cursor: usize,
    /// The frame is at least [`WIDE_FROM`] columns: rows carry their
    /// config id.
    pub wide: bool,
}

impl PaletteView {
    /// How many rows there are, groups flattened.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.groups.iter().map(|(_, rows)| rows.len()).sum()
    }
}

/// One line of the list: a group's header, a row (by its flattened index),
/// or the line that says nothing matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Line {
    Header(usize),
    Row(usize, usize, usize),
    Nothing,
}

fn lines(view: &PaletteView) -> Vec<Line> {
    if view.groups.iter().all(|(_, rows)| rows.is_empty()) {
        return vec![Line::Nothing];
    }
    let mut out = Vec::new();
    let mut flat = 0;
    for (group, (_, rows)) in view.groups.iter().enumerate() {
        if rows.is_empty() {
            continue;
        }
        out.push(Line::Header(group));
        for at in 0..rows.len() {
            out.push(Line::Row(flat, group, at));
            flat += 1;
        }
    }
    out
}

/// The most the box can take: over the focused pane `over`, at most
/// [`MAX_WIDTH`] wide and centred on it; the full width of `frame` when
/// `frame` is narrower than [`WIDE_FROM`]. One row below `over`'s top, at
/// most `over`'s height less 2. The box drawn in it is only as tall as its
/// lines need.
#[must_use]
pub fn palette_area(over: Rect, frame: Rect) -> Rect {
    let (x, width) = if frame.width < WIDE_FROM {
        (frame.x, frame.width)
    } else {
        let width = over.width.min(MAX_WIDTH);
        (over.x + (over.width - width) / 2, width)
    };
    Rect {
        x,
        y: over.y + u16::from(over.height > 2),
        width,
        height: over.height.saturating_sub(2),
    }
}

/// The box `view` draws in `area` (from [`palette_area`]): top-aligned,
/// as tall as its lines, no taller than `area`.
fn box_rect(area: Rect, view: &PaletteView) -> Rect {
    let want = u16::try_from(lines(view).len())
        .unwrap_or(u16::MAX)
        .saturating_add(CHROME_ROWS);
    Rect {
        height: want.min(area.height),
        ..area
    }
}

/// The first list line drawn in a box with `visible` list lines, keeping
/// the cursor's line in view (and its group's header, when there is room).
fn first_drawn(lines: &[Line], cursor: usize, visible: usize) -> usize {
    let at = lines
        .iter()
        .position(|line| matches!(line, Line::Row(flat, _, _) if *flat == cursor))
        .unwrap_or(0);
    (at + 1).saturating_sub(visible)
}

/// The row a click at (`column`, `row`) landed on, in a box drawn in
/// `area` (from [`palette_area`]): `None` on a border, the query, a
/// header or below the last row.
#[must_use]
pub fn palette_row_at(area: Rect, view: &PaletteView, column: u16, row: u16) -> Option<usize> {
    let drawn = box_rect(area, view);
    if column <= drawn.x || column + 1 >= drawn.right() || drawn.height < CHROME_ROWS {
        return None;
    }
    let visible = usize::from(drawn.height - CHROME_ROWS);
    let offset = usize::from(row.checked_sub(drawn.y + 2)?);
    if offset >= visible {
        return None;
    }
    let lines = lines(view);
    match lines.get(first_drawn(&lines, view.cursor, visible) + offset)? {
        Line::Row(flat, _, _) => Some(*flat),
        Line::Header(_) | Line::Nothing => None,
    }
}

/// The palette as a widget: draws `view` into the area it is given (from
/// [`palette_area`]), clearing only the box it takes.
pub struct Palette<'a> {
    /// What to draw.
    pub view: &'a PaletteView,
    /// The colours.
    pub theme: &'a Theme,
}

impl Widget for Palette<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        render_palette(buf, area, self.view, self.theme);
    }
}

fn render_palette(buf: &mut Buffer, area: Rect, view: &PaletteView, theme: &Theme) {
    let drawn = box_rect(area.intersection(buf.area), view);
    if drawn.width < 8 || drawn.height < CHROME_ROWS + 1 {
        return;
    }
    let width = usize::from(drawn.width);
    let text = foreground(theme, Token::Text);
    let border = foreground(theme, Token::Border);
    for y in drawn.y..drawn.bottom() {
        buf.set_string(drawn.x, y, content_row("", width), border);
    }
    let title = match view.mode {
        PaletteMode::Here(_) => "keys here",
        PaletteMode::Everything => "find anything",
    };
    let close = shipped_key_verb("app.back", "close");
    buf.set_string(
        drawn.x,
        drawn.y,
        top_border(title, close.as_deref().unwrap_or(""), width),
        border,
    );
    buf.set_string(
        drawn.x + 3,
        drawn.y,
        fit(title, width.saturating_sub(6)).trim_end(),
        text.add_modifier(Modifier::BOLD),
    );
    buf.set_string(
        drawn.x,
        drawn.bottom() - 1,
        format!("╰{}╯", "─".repeat(width - 2)),
        border,
    );
    // The query: `› ` and the field, its cursor placed by the text field's
    // own scroll rule.
    buf.set_string(
        drawn.x + 2,
        drawn.y + 1,
        "› ",
        foreground(theme, Token::Accent),
    );
    draw_line(
        buf,
        Rect::new(drawn.x + 4, drawn.y + 1, drawn.width.saturating_sub(6), 1),
        view.query.as_str(),
        Some(view.query_cursor),
        None,
        theme,
    );
    let lines = lines(view);
    let visible = usize::from(drawn.height - CHROME_ROWS);
    let top = first_drawn(&lines, view.cursor, visible);
    for (offset, line) in lines.iter().skip(top).take(visible).enumerate() {
        let y = drawn.y + 2 + u16::try_from(offset).unwrap_or(0);
        draw_list_line(buf, drawn, y, *line, view, theme);
    }
}

/// `╭─ <title> ──…── <right> ╮`, exactly `width` cells; the title is
/// drawn over it in its own style.
fn top_border(title: &str, right: &str, width: usize) -> String {
    let inner = width.saturating_sub(2);
    let left = format!("─ {} ", fit(title, inner.saturating_sub(4)).trim_end());
    let right = if right.is_empty() || cells(&left) + cells(right) + 2 > inner {
        String::new()
    } else {
        format!(" {right} ")
    };
    let dashes = inner.saturating_sub(cells(&left) + cells(&right));
    format!("╭{left}{}{right}╮", "─".repeat(dashes))
}

fn draw_list_line(
    buf: &mut Buffer,
    area: Rect,
    y: u16,
    line: Line,
    view: &PaletteView,
    theme: &Theme,
) {
    let inner = area.width - 2;
    let x = area.x + 1;
    let muted = foreground(theme, Token::TextMuted);
    match line {
        Line::Nothing => {
            let words = match shipped_key_verb("app.back", "close") {
                Some(close) => format!("{NOTHING_MATCHES} · {close}"),
                None => NOTHING_MATCHES.to_owned(),
            };
            buf.set_string(x + 1, y, fit(&words, usize::from(inner - 1)), muted);
        }
        Line::Header(group) => {
            let header = view.groups[group].0.as_str().to_uppercase();
            buf.set_string(
                x + 1,
                y,
                fit(&header, usize::from(inner - 1)),
                muted.add_modifier(Modifier::BOLD),
            );
        }
        Line::Row(flat, group, at) => {
            let row = &view.groups[group].1[at];
            draw_row(buf, x, y, inner, row, flat == view.cursor, view.wide, theme);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_row(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    inner: u16,
    row: &PaletteRow,
    chosen: bool,
    wide: bool,
    theme: &Theme,
) {
    let inner = usize::from(inner);
    let accent = foreground(theme, Token::Accent);
    let text = foreground(theme, Token::Text);
    let muted = foreground(theme, Token::TextMuted);
    if chosen {
        buf.set_string(x, y, "▸", accent.add_modifier(Modifier::BOLD));
    }
    let key = row.key.as_ref().map_or("", |key| key.0.as_str());
    buf.set_string(
        x + 1,
        y,
        fit(key, KEY_CELLS - 1),
        accent.add_modifier(Modifier::BOLD),
    );
    // The config id, right-aligned with two cells to the border, only when
    // the frame is wide and the label still has room.
    let config = row.config.filter(|_| wide).map_or("", |id| id.0);
    let label_at = 1 + KEY_CELLS;
    let config_cells = cells(config);
    let room = inner.saturating_sub(label_at + 2);
    let label_room = if config.is_empty() || config_cells + 2 > room / 2 {
        room
    } else {
        room - config_cells - 2
    };
    let label_style = if chosen {
        text.add_modifier(Modifier::BOLD)
    } else {
        text
    };
    buf.set_string(
        x + u16::try_from(label_at).unwrap_or(0),
        y,
        fit(row.label.as_str(), label_room),
        label_style,
    );
    if label_room < room {
        let config_x = inner - 2 - config_cells;
        buf.set_string(x + u16::try_from(config_x).unwrap_or(0), y, config, muted);
    }
}

/// `<key> <verb>` for a palette key, from the shipped table: the palette
/// is a text field, its keys fixed (ADR-0055).
fn shipped_key_verb(action: &'static str, verb: &str) -> Option<String> {
    Keymap::shipped().key_verb(&[ActionId(action)], verb)
}

#[cfg(test)]
mod tests;
