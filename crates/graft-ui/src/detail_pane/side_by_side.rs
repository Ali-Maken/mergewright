//! [`diff_body_side_by_side`]: the `v`-toggled two-column twin of
//! [`super::diff_body`] over the same [`HunkView`] data.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use graft_core::capabilities::ColorDepth;
use graft_term::sanitize::SanitizedText;

use crate::detail::{HunkView, LineKindView, LineView};
use crate::paint::foreground;
use crate::theme::{Theme, Token};

use super::line_render::{
    DiffRowStyle, RowSlot, draw_heading, draw_styled_text, gutter_style, line_style_at,
};

/// One row of the side-by-side body: a full-width hunk heading, or a pair of
/// half-width lines. A `None` half is a run's unmatched line — blank on that
/// side, not shifted.
enum SideRow<'a> {
    Heading(&'a SanitizedText),
    Pair {
        left: Option<&'a LineView>,
        right: Option<&'a LineView>,
    },
}

/// `hunk`'s lines as side-by-side rows: context lines mirror both halves;
/// a removed/added line with a [`LineView::pair`] shares a row with its
/// partner (whichever side set the index, the other is marked consumed so it
/// is not emitted twice); an unpaired line leaves the other half blank.
fn side_rows_for_hunk(hunk: &HunkView) -> Vec<SideRow<'_>> {
    let mut rows = Vec::new();
    let mut consumed = vec![false; hunk.lines.len()];
    for (index, line) in hunk.lines.iter().enumerate() {
        if consumed[index] {
            continue;
        }
        match line.kind {
            LineKindView::HunkHeading => {}
            LineKindView::Context => rows.push(SideRow::Pair {
                left: Some(line),
                right: Some(line),
            }),
            LineKindView::Removed => {
                if let Some(pair) = line.pair {
                    let partner = usize::try_from(pair).ok();
                    if let Some(partner) = partner {
                        consumed[partner] = true;
                    }
                    rows.push(SideRow::Pair {
                        left: Some(line),
                        right: partner.and_then(|p| hunk.lines.get(p)),
                    });
                } else {
                    rows.push(SideRow::Pair {
                        left: Some(line),
                        right: None,
                    });
                }
            }
            LineKindView::Added => {
                if let Some(pair) = line.pair {
                    let partner = usize::try_from(pair).ok();
                    if let Some(partner) = partner {
                        consumed[partner] = true;
                    }
                    rows.push(SideRow::Pair {
                        left: partner.and_then(|p| hunk.lines.get(p)),
                        right: Some(line),
                    });
                } else {
                    rows.push(SideRow::Pair {
                        left: None,
                        right: Some(line),
                    });
                }
            }
        }
    }
    rows
}

fn flatten_side_by_side(hunks: &[HunkView]) -> Vec<SideRow<'_>> {
    let mut rows = Vec::new();
    for hunk in hunks {
        rows.push(SideRow::Heading(&hunk.heading));
        rows.extend(side_rows_for_hunk(hunk));
    }
    rows
}

/// The widest old (`old == true`) or new line number across every hunk, min 3.
fn gutter_width_side(hunks: &[HunkView], old: bool) -> usize {
    let widest = hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| line.kind != LineKindView::HunkHeading)
        .filter_map(|line| if old { line.old_no } else { line.new_no })
        .max()
        .unwrap_or(0);
    widest.to_string().len().max(3)
}

/// [`pin_heading`](super::unified)'s side-by-side twin: same rule, counted
/// over [`side_rows_for_hunk`] rows instead of the unified flatten.
fn pin_heading_side<'a>(
    hunks: &'a [HunkView],
    rows: &[SideRow<'a>],
    top: usize,
) -> Option<&'a SanitizedText> {
    if matches!(rows.get(top), None | Some(SideRow::Heading(_))) {
        return None;
    }
    let mut consumed = 0usize;
    for hunk in hunks {
        let hunk_rows = 1 + side_rows_for_hunk(hunk).len();
        if top < consumed + hunk_rows {
            return Some(&hunk.heading);
        }
        consumed += hunk_rows;
    }
    None
}

/// One half of a side-by-side row: `{number:>gutter} {sign}{text}`, or a
/// blank half when `line` is `None` (the other side's run ran longer).
fn draw_side_line(
    line: Option<&LineView>,
    old: bool,
    gutter: usize,
    slot: RowSlot,
    buf: &mut Buffer,
    theme: &Theme,
    style_ctx: DiffRowStyle,
) {
    let RowSlot { x, y, width } = slot;
    let Some(line) = line else {
        let blank = " ".repeat(usize::from(width));
        buf.set_stringn(x, y, &blank, usize::from(width), Style::default());
        return;
    };
    let (sign, kind_token) = match line.kind {
        LineKindView::Added => ('+', Some(Token::Added)),
        LineKindView::Removed => ('-', Some(Token::Removed)),
        LineKindView::Context | LineKindView::HunkHeading => (' ', None),
    };
    let number = if old { line.old_no } else { line.new_no };
    let num_str = number.map_or(String::new(), |n| n.to_string());
    let prefix = format!("{num_str:>gutter$} ");
    let prefix_cells = u16::try_from(prefix.chars().count()).unwrap_or(u16::MAX);
    let prefix_fits = prefix_cells.min(width);
    buf.set_stringn(
        x,
        y,
        &prefix,
        usize::from(prefix_fits),
        gutter_style(style_ctx, theme),
    );
    if prefix_fits >= width {
        return;
    }
    let sign_style = line_style_at(line, usize::MAX, theme, style_ctx, kind_token);
    let sign_x = x + prefix_fits;
    buf.set_stringn(sign_x, y, sign.to_string(), 1, sign_style);
    if prefix_fits.saturating_add(1) >= width {
        return;
    }
    buf.set_stringn(sign_x + 1, y, " ", 1, sign_style);
    let text_x = sign_x.saturating_add(2);
    let drawn = prefix_fits.saturating_add(2);
    if drawn >= width {
        return;
    }
    let text_width = width - drawn;
    draw_styled_text(
        buf,
        text_x,
        y,
        text_width,
        line.text.as_str(),
        sign_style,
        |index| line_style_at(line, index, theme, style_ctx, kind_token),
    );
}

/// The `v`-toggled two-column diff over the same [`HunkView`] data
/// [`super::diff_body`] draws: old numbers and text on the left, new on the
/// right, a `│` rule between (plain at [`ColorDepth::Palette16`]/`NO_COLOR`,
/// same rule as the sign styling there). Rows come from [`side_rows_for_hunk`]:
/// [`LineView::pair`] keeps a removed/added run aligned; an unpaired line
/// leaves the other half blank on that row, never shifted. The hunk heading
/// pins full-width above both halves exactly as it does in
/// [`super::diff_body`]. Word tint, syntax fg, `␍` and the 16-colour fallback
/// are the same [`line_style_at`]/[`draw_styled_text`] [`super::diff_body`]
/// uses, so both renderers match on the same fixture by construction.
#[must_use]
pub fn diff_body_side_by_side(
    hunks: &[HunkView],
    top: usize,
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    focused: bool,
    word_tint: bool,
) -> usize {
    let rows = flatten_side_by_side(hunks);
    let total = rows.len();
    if area.width == 0 || area.height == 0 {
        return total;
    }

    let columns = SideColumns::new(hunks, area, theme, focused, word_tint);
    let pinned = pin_heading_side(hunks, &rows, top);
    let mut used: u16 = 0;
    if let Some(heading) = pinned {
        let slot = RowSlot {
            x: area.x,
            y: area.y,
            width: area.width,
        };
        draw_heading(heading, slot, buf, theme);
        used = 1;
    }
    let content_top = top.saturating_add(usize::from(pinned.is_some()));
    let remaining = area.height.saturating_sub(used);

    for offset in 0..remaining {
        let idx = content_top.saturating_add(usize::from(offset));
        let y = area.y.saturating_add(used).saturating_add(offset);
        draw_side_row(rows.get(idx), area, y, &columns, buf, theme);
    }

    total
}

/// Everything [`diff_body_side_by_side`]'s row loop needs beyond the row
/// itself: both gutters, the halves' x-ranges, the divider and the
/// [`DiffRowStyle`] both halves draw with.
struct SideColumns {
    old_gutter: usize,
    new_gutter: usize,
    style_ctx: DiffRowStyle,
    left_width: u16,
    right_width: u16,
    divider_x: u16,
    right_x: u16,
    divider: u16,
    divider_glyph: &'static str,
}

impl SideColumns {
    fn new(hunks: &[HunkView], area: Rect, theme: &Theme, focused: bool, word_tint: bool) -> Self {
        let palette16 = matches!(
            theme.color_depth(),
            ColorDepth::Palette16 | ColorDepth::None
        );
        let divider = u16::from(area.width >= 3);
        let usable = area.width.saturating_sub(divider);
        let left_width = usable / 2;
        let right_width = usable.saturating_sub(left_width);
        let divider_x = area.x + left_width;
        Self {
            old_gutter: gutter_width_side(hunks, true),
            new_gutter: gutter_width_side(hunks, false),
            style_ctx: DiffRowStyle {
                focused,
                palette16,
                word_tint,
                // The side-by-side body pairs two unified rows into one, so
                // a unified body-row cursor has no row here to sit on. `v`
                // and the line cursor are exclusive: the changes pane's
                // `Space`/`d` say so rather than acting unseen
                // (m3-working-tree-undo-and-stash/04 NOTES).
                cursor: false,
            },
            left_width,
            right_width,
            divider_x,
            right_x: divider_x + divider,
            divider,
            divider_glyph: if palette16 { "|" } else { "\u{2502}" },
        }
    }
}

/// One content row: a full-width heading, a paired-line row split across
/// [`SideColumns`]'s two halves with the divider between, or nothing past
/// the last row.
fn draw_side_row(
    row: Option<&SideRow<'_>>,
    area: Rect,
    y: u16,
    columns: &SideColumns,
    buf: &mut Buffer,
    theme: &Theme,
) {
    match row {
        Some(SideRow::Heading(heading)) => {
            let slot = RowSlot {
                x: area.x,
                y,
                width: area.width,
            };
            draw_heading(heading, slot, buf, theme);
        }
        Some(SideRow::Pair { left, right }) => {
            draw_side_line(
                *left,
                true,
                columns.old_gutter,
                RowSlot {
                    x: area.x,
                    y,
                    width: columns.left_width,
                },
                buf,
                theme,
                columns.style_ctx,
            );
            if columns.divider > 0 {
                buf.set_stringn(
                    columns.divider_x,
                    y,
                    columns.divider_glyph,
                    1,
                    foreground(theme, Token::TextMuted),
                );
            }
            draw_side_line(
                *right,
                false,
                columns.new_gutter,
                RowSlot {
                    x: columns.right_x,
                    y,
                    width: columns.right_width,
                },
                buf,
                theme,
                columns.style_ctx,
            );
        }
        None => {}
    }
}
