//! Style and glyph helpers for [`super::GraphColumn`]'s `Widget::render`:
//! per-cell colour under a selection thread, the 16-colour/`NO_COLOR` dash
//! convention (DESIGN.md §4.6, ADR-0019), and the ASCII tier's glyph table
//! (ticket m1-the-graph/19, ADR-0016).

use mergewright_core::capabilities::ColorDepth;
use ratatui::style::{Color, Modifier, Style};

use super::{CELLS_PER_LANE, GraphLine, NodeKind, RAIL, STUB_ABOVE, STUB_BELOW};

/// A Ghost Preview node's glyph: the anchor shape (ADR-0038 §5), never a
/// glyph any real [`NodeKind`] draws — `○` for an ordinary ghost, `◎` for a
/// ghost merge, so a replayed merge still reads as a merge without ever
/// reading as [`NodeKind::Merge`]'s solid `◉`.
pub(super) fn ghost_node_glyph(node: NodeKind) -> char {
    if node == NodeKind::Merge {
        '◎'
    } else {
        '○'
    }
}

/// A Ghost Preview edge cell's glyph: straight runs dash (`│`→`┊`,
/// `─`→`┈`); elbows and junctions have no dashed form in this set and stay
/// as drawn — "drawn with the dashed set" means dimmed and part of the
/// ghost row, not a literal dashed corner glyph that does not exist.
pub(super) fn ghost_edge_glyph(glyph: char) -> char {
    match glyph {
        '│' => '┊',
        '─' => '┈',
        other => other,
    }
}

/// The first cell of a drawn lane column.
pub(super) fn lane_x(column: usize) -> u16 {
    u16::try_from(column)
        .unwrap_or(u16::MAX)
        .saturating_mul(CELLS_PER_LANE)
}

/// A rail or stub cell's style on `line`. A fold never hides a thread lane
/// (the thread is its protected set), so both are off the thread: faded
/// under a selection, the row's own paint otherwise. See [`fade`].
pub(super) fn off_thread(style: Style, line: &GraphLine<'_>, muted: Option<Color>) -> Style {
    let style = style.add_modifier(line.paint.modifier());
    if line.thread.is_some() {
        fade(style, muted)
    } else {
        style
    }
}

/// A lane cell's style under a selection: bold on the thread, faded off it.
/// Without a selection, or for a cell with no real column, unchanged.
pub(super) fn on_thread(
    style: Style,
    line: &GraphLine<'_>,
    real: Option<usize>,
    muted: Option<Color>,
) -> Style {
    match (&line.thread, real) {
        (Some(thread), Some(column)) if thread.contains(&column) => style
            .remove_modifier(Modifier::DIM)
            .add_modifier(Modifier::BOLD),
        (Some(_), _) => fade(style, muted),
        (None, _) => style,
    }
}

/// An off-thread cell: its lane's hue mixed toward `bg` where the depth has
/// one (`muted`, [`crate::theme::Theme::lane_muted`]) — terminals that
/// ignore SGR 2 would otherwise show no fade at all (m8-polish-and-ship/17
/// item 3) — else DIM, as at 16 colours and `NO_COLOR`.
fn fade(style: Style, muted: Option<Color>) -> Style {
    let style = style.remove_modifier(Modifier::BOLD);
    match muted {
        Some(muted) => style.fg(muted).remove_modifier(Modifier::DIM),
        None => style.add_modifier(Modifier::DIM),
    }
}

/// Whether lanes in odd columns draw dashed at this colour depth: only where
/// hue alone cannot tell neighbours apart.
pub(super) fn dashes_odd_columns(depth: ColorDepth) -> bool {
    match depth {
        ColorDepth::TrueColor | ColorDepth::Palette256 => false,
        ColorDepth::Palette16 | ColorDepth::None => true,
    }
}

/// The dashed form of a straight run. Anything else — nodes, elbows,
/// junctions — has none and draws as it is.
pub(super) fn dashed(glyph: char) -> char {
    match glyph {
        '│' => '╎',
        '─' => '╌',
        other => other,
    }
}

/// The ASCII tier's glyph table (ticket m1-the-graph/19, ADR-0016): every
/// glyph this widget draws collapses onto one of six ASCII characters
/// (`* | / \ - +`). Node shape is lost — six glyphs leave no room for six
/// node kinds and a graph too — so every node draws as `*`; paint (bold
/// selection, dim off-thread) and the row's own text (subject, badges) carry
/// what shape used to. The dashed forms (`╎`/`╌`) collapse onto the same
/// character as their solid counterparts: ASCII has no seventh glyph to
/// spare for the odd-column lane distinction DESIGN.md §4.6 asks for at low
/// colour depth, so at this tier that redundancy is not drawn.
pub(super) fn to_ascii(glyph: char) -> char {
    match glyph {
        // The role shapes (ease-pass G1, G2): `|` and `-` for this, built
        // on and trunk; `:` and `.` for other, below with the ghost's.
        '│' | '╎' | '┃' | '║' => '|',
        '─' | '╌' | '━' | '═' => '-',
        '╯' | '╭' => '/',
        '╰' | '╮' => '\\',
        '┤' | '├' | '┴' | '┬' | '┼' => '+',
        STUB_BELOW => 'v',
        STUB_ABOVE => '^',
        // Ghost Preview's dashed rail (`┊`, ADR-0038 §5) shares the solid
        // rail's glyph at this tier — ASCII has no seventh glyph to spare.
        RAIL | '┊' => ':',
        '┈' | '┄' => '.',
        '○' => 'o',
        '◎' => 'O',
        ' ' => ' ',
        _ => '*',
    }
}
