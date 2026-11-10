//! The command palette and help (`?`, `⌃P`, `:`; mock-ups §1.16, keymap
//! §2.12, ADR-0056): one box, two ways in. `?` lists this place's keys in
//! groups (`IN PROGRESS`, `HERE`, `EVERYWHERE`, `YOUR CONFIG` first when
//! his config had problems); `⌃P`/`:` lists every action.
//!
//! Pure drawing. `mergewright` owns the state, builds the groups from the live
//! key table, filters them and hands over a [`PaletteView`] with every
//! string worded and sanitized. The scroll is worked out here from the
//! cursor, never stored, so it cannot go stale behind a resize (the
//! picker's rule, [`crate::picker::first_drawn`]).
//!
//! The palette is a text field (§2.1): its own keys spell from
//! [`Keymap::shipped`] whatever his config says.

use mergewright_term::SanitizedText;
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

/// What a row stands for.
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
    /// A file (ticket 12b).
    File,
    /// A commit (ticket 12b).
    Commit,
    /// A recently run command (ticket 12): drawn as an action row.
    Recent,
    /// A find group's muted line: `+N more`, `reading…`, `searching…`, a
    /// read failure (ticket 12).
    More,
    /// One choice of a setting: `●` (on, accent) / `○` (muted) drawn in the
    /// key column by kind, `(on)` after the chosen one; `⏎` picks it. Its
    /// [`PaletteRow::key`] is `None`.
    Setting {
        /// Whether this choice is the one in force.
        on: bool,
    },
}

/// What the id column shows: an action's config id, or a setting's config key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigId {
    /// An action's id (`graph.merge`).
    Action(ActionId),
    /// A setting's key in his config (`graph.merged_branches`).
    Key(&'static str),
}

impl ConfigId {
    /// The id as drawn.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Action(id) => id.0,
            Self::Key(key) => key,
        }
    }
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
    /// Its id in his config (`graph.merge`, `graph.merged_branches`), drawn
    /// muted at the right.
    pub config: Option<ConfigId>,
    /// Muted text after the label (setting rows); cut from its end before
    /// the label is.
    pub detail: Option<&'static str>,
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
    /// Drawn muted in the empty query field, gone on the first character
    /// (`⌃P` in a repository: what the box finds, ADR-0061).
    pub placeholder: Option<&'static str>,
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
    draw_query(buf, drawn, view, theme);
    let lines = lines(view);
    let visible = usize::from(drawn.height - CHROME_ROWS);
    let top = first_drawn(&lines, view.cursor, visible);
    for (offset, line) in lines.iter().skip(top).take(visible).enumerate() {
        let y = drawn.y + 2 + u16::try_from(offset).unwrap_or(0);
        draw_list_line(buf, drawn, y, *line, view, theme);
    }
}

/// The query row: `› ` and the field, its cursor placed by the text field's
/// own scroll rule; the placeholder, muted, while the field is empty.
fn draw_query(buf: &mut Buffer, drawn: Rect, view: &PaletteView, theme: &Theme) {
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
    if let Some(words) = view.placeholder.filter(|_| view.query.as_str().is_empty()) {
        // One cell past the field's cursor, which stays where typing goes.
        buf.set_string(
            drawn.x + 5,
            drawn.y + 1,
            fit(words, usize::from(drawn.width.saturating_sub(7))).trim_end(),
            foreground(theme, Token::TextMuted),
        );
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

/// `<key> <verb>` for a palette key, from the shipped table: the palette
/// is a text field, its keys fixed (ADR-0055).
fn shipped_key_verb(action: &'static str, verb: &str) -> Option<String> {
    Keymap::shipped().key_verb(&[ActionId(action)], verb)
}

mod row;

use row::draw_list_line;

#[cfg(test)]
mod tests;
