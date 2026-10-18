//! One line of the palette's list: a group header, a row (key, label,
//! config id), or the nothing-matches line.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;

use super::{KEY_CELLS, Line, NOTHING_MATCHES, PaletteRow, PaletteView, RowKind, shipped_key_verb};
use crate::fit::fit;
use crate::frame_box::cells;
use crate::paint::foreground;
use crate::theme::{Theme, Token};

pub(super) fn draw_list_line(
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
            let slot = Rect::new(x, y, inner, 1);
            draw_row(buf, slot, row, flat == view.cursor, view.wide, theme);
        }
    }
}

/// One row of the list into `slot` (one cell high, inside the borders).
fn draw_row(
    buf: &mut Buffer,
    slot: Rect,
    row: &PaletteRow,
    chosen: bool,
    wide: bool,
    theme: &Theme,
) {
    let (x, y) = (slot.x, slot.y);
    let inner = usize::from(slot.width);
    let accent = foreground(theme, Token::Accent);
    let text = foreground(theme, Token::Text);
    let muted = foreground(theme, Token::TextMuted);
    if chosen {
        buf.set_string(x, y, "▸", accent.add_modifier(Modifier::BOLD));
    }
    // The key column, drawn by kind: a setting's choice is a dot, accent
    // when on and muted when not; every other kind's key is accent+bold.
    let (key, key_style) = match row.kind {
        RowKind::Setting { on: true } => ("●", accent.add_modifier(Modifier::BOLD)),
        RowKind::Setting { on: false } => ("○", muted),
        _ => (
            row.key.as_ref().map_or("", |key| key.0.as_str()),
            accent.add_modifier(Modifier::BOLD),
        ),
    };
    buf.set_string(x + 1, y, fit(key, KEY_CELLS - 1), key_style);
    // The config id, right-aligned with two cells to the border, only when
    // the frame is wide and the label still has room.
    let config = row
        .config
        .filter(|_| wide)
        .map_or("", super::ConfigId::as_str);
    let label_at = 1 + KEY_CELLS;
    let config_cells = cells(config);
    let room = inner.saturating_sub(label_at + 2);
    let label_room = if config.is_empty() || config_cells + 2 > room / 2 {
        room
    } else {
        room - config_cells - 2
    };
    let label_style = match (row.kind, chosen) {
        (RowKind::More, _) => muted,
        (_, true) => text.add_modifier(Modifier::BOLD),
        (_, false) => text,
    };
    let label_x = x + u16::try_from(label_at).unwrap_or(0);
    let fitted = if let RowKind::Setting { on } = row.kind {
        draw_setting(buf, (label_x, y), row, on, label_room, (label_style, muted));
        String::new()
    } else {
        let fitted = fit(row.label.as_str(), label_room);
        buf.set_string(label_x, y, &fitted, label_style);
        fitted
    };
    // 12b: what tells two rows of a find group apart drawn as text, the
    // rest muted — a file's directory, a commit's sha.
    if let Some(prefix) = muted_prefix(row)
        && let Some(drawn) = fitted.get(..prefix.len()).filter(|d| *d == prefix)
    {
        buf.set_string(label_x, y, drawn, muted);
    }
    if label_room < room {
        let config_x = inner - 2 - config_cells;
        buf.set_string(x + u16::try_from(config_x).unwrap_or(0), y, config, muted);
    }
}

/// The cells a setting's title is padded to before its detail (the mock's).
const SETTING_TITLE_CELLS: usize = 16;

/// What follows the chosen setting row; never cut — it is the state.
const ON_MARK: &str = "   (on)";

/// A setting row's text from `at`, in `room` cells: the title, its muted
/// detail, then `(on)` when chosen. The detail is cut from its end first,
/// then the title; `(on)` never.
fn draw_setting(
    buf: &mut Buffer,
    at: (u16, u16),
    row: &PaletteRow,
    on: bool,
    room: usize,
    (title_style, muted): (ratatui::style::Style, ratatui::style::Style),
) {
    let (x, y) = at;
    let mark = if on { ON_MARK } else { "" };
    let room = room.saturating_sub(cells(mark));
    let title = row.label.as_str();
    let title_cells = SETTING_TITLE_CELLS.max(cells(title) + 2);
    let mut used = if room >= title_cells {
        buf.set_string(x, y, title, title_style);
        // `fit` pads to the room; `(on)` follows the words, not the padding.
        let fitted = fit(row.detail.unwrap_or(""), room - title_cells);
        let detail = fitted.trim_end();
        let shift = u16::try_from(title_cells).unwrap_or(0);
        buf.set_string(x + shift, y, detail, muted);
        if detail.is_empty() {
            cells(title)
        } else {
            title_cells + cells(detail)
        }
    } else {
        let fitted = fit(title, room);
        buf.set_string(x, y, &fitted, title_style);
        cells(fitted.trim_end())
    };
    if on {
        used = used.min(room);
        let shift = u16::try_from(used).unwrap_or(0);
        buf.set_string(x + shift, y, mark, muted);
    }
}

/// The part of a find row's label drawn muted: a FILES row's directory (up
/// to its last `/`), a COMMITS row's sha (before its two spaces).
fn muted_prefix(row: &PaletteRow) -> Option<&str> {
    let label = row.label.as_str();
    match row.kind {
        RowKind::File => label.rfind('/').map(|at| &label[..=at]),
        RowKind::Commit => label.find("  ").map(|at| &label[..at]),
        RowKind::Action
        | RowKind::App
        | RowKind::Branch
        | RowKind::Tag
        | RowKind::Recent
        | RowKind::More
        | RowKind::Setting { .. } => None,
    }
}
