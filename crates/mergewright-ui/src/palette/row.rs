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
    let label_style = match (row.kind, chosen) {
        (RowKind::More, _) => muted,
        (_, true) => text.add_modifier(Modifier::BOLD),
        (_, false) => text,
    };
    let fitted = fit(row.label.as_str(), label_room);
    let label_x = x + u16::try_from(label_at).unwrap_or(0);
    buf.set_string(label_x, y, &fitted, label_style);
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
        | RowKind::More => None,
    }
}
