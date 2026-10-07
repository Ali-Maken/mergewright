//! [`diff_body`]: the real unified diff renderer (`diff-widget`,
//! m2-details-and-diffs/08). Gutter, soft/word backgrounds, syntax fg, a
//! pinned hunk heading, `␍`, and a truncation footer.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use graft_core::capabilities::ColorDepth;
use graft_term::sanitize::SanitizedText;

use crate::detail::{HunkView, LineKindView};
use crate::keymap::Keymap;
use crate::theme::Theme;

use super::line_render::{
    DiffFocus, DiffRow, DiffRowStyle, RowSlot, draw_diff_row, draw_heading, draw_truncation_row,
};

/// One row per hunk heading, then its content lines. A `HunkHeading`-kind
/// line inside `hunk.lines` (real assembled diffs put one at `lines[0]`,
/// `crates/graft/src/jobs/detail.rs`) is dropped here rather than drawn a
/// second time: `hunk.heading` already stands for it, and [`pin_heading`]
/// and [`gutter_width`] both key off this same filtered shape.
fn flatten(hunks: &[HunkView]) -> Vec<DiffRow<'_>> {
    let mut rows = Vec::new();
    for hunk in hunks {
        rows.push(DiffRow::Heading(&hunk.heading));
        rows.extend(
            hunk.lines
                .iter()
                .filter(|line| line.kind != LineKindView::HunkHeading)
                .map(DiffRow::Line),
        );
    }
    rows
}

/// Gutter width: the widest old/new line number across every hunk, min 3.
pub(crate) fn gutter_width(hunks: &[HunkView]) -> usize {
    let widest = hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| line.kind != LineKindView::HunkHeading)
        .flat_map(|line| [line.old_no, line.new_no])
        .flatten()
        .max()
        .unwrap_or(0);
    widest.to_string().len().max(3)
}

/// The hunk heading to pin at the body's top row, when `top` lands inside
/// that hunk's own lines rather than on its heading row (which is already
/// visible at row 0 in that case — normal scroll resumes).
fn pin_heading<'a>(
    hunks: &'a [HunkView],
    rows: &[DiffRow<'a>],
    top: usize,
) -> Option<&'a SanitizedText> {
    if matches!(rows.get(top), None | Some(DiffRow::Heading(_))) {
        return None;
    }
    let mut consumed = 0usize;
    for hunk in hunks {
        let hunk_rows = 1 + hunk
            .lines
            .iter()
            .filter(|line| line.kind != LineKindView::HunkHeading)
            .count();
        if top < consumed + hunk_rows {
            return Some(&hunk.heading);
        }
        consumed += hunk_rows;
    }
    None
}

/// Draw `hunks` from content row `top`: gutter numbers, sign cell, soft
/// added/removed background, a stronger background over changed words,
/// syntax foreground, a pinned hunk heading, and — when `truncated` is
/// `Some((shown, total, keys))` — a `… N more lines · L load all` footer
/// row, `L` spelled from `keys` (m8/09d).
/// Returns the total content rows including that footer, for scroll
/// clamping. At [`ColorDepth::Palette16`] or under `NO_COLOR`
/// ([`ColorDepth::None`]): sign glyph plus bold (added) / dim (removed)
/// only, no backgrounds, no syntax or word tint (m1/12c rule).
#[must_use]
pub fn diff_body(
    hunks: &[HunkView],
    top: usize,
    truncated: Option<(usize, usize, &Keymap)>,
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    focus: DiffFocus,
) -> usize {
    let rows = flatten(hunks);
    let total = rows.len() + usize::from(truncated.is_some());
    if area.width == 0 || area.height == 0 {
        return total;
    }

    let gutter = gutter_width(hunks);
    let palette16 = matches!(
        theme.color_depth(),
        ColorDepth::Palette16 | ColorDepth::None
    );
    let style_ctx = DiffRowStyle {
        focused: focus.focused,
        palette16,
        word_tint: focus.word_tint,
        cursor: false,
    };

    // The pin takes the slot the row at `top` would have used, so that row
    // is not drawn at all. When that row is the line cursor's, the pin would
    // hide the very line `Space` and `d` act on — a destructive key aimed at
    // something off screen (m3/04 review C1). Context loses to visibility:
    // the heading goes, the cursor row stays.
    let pinned = if focus.cursor == Some(top) {
        None
    } else {
        pin_heading(hunks, &rows, top)
    };
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
        let slot = RowSlot {
            x: area.x,
            y: area.y.saturating_add(used).saturating_add(offset),
            width: area.width,
        };
        if let Some(row) = rows.get(idx) {
            let style_ctx = DiffRowStyle {
                cursor: focus.cursor == Some(idx),
                ..style_ctx
            };
            draw_diff_row(row, gutter, slot, buf, theme, style_ctx);
        } else if let (true, Some((shown, of_total, keys))) = (idx == rows.len(), truncated) {
            draw_truncation_row(shown, of_total, slot, buf, theme, keys);
        }
    }

    total
}
