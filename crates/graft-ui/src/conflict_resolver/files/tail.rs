//! The list's last row, after its files (ease-pass/11, mock M6): in a merge
//! with no clash open, `✓ Review the merge  y` — the way on, where the eye
//! already is. A click on it does what its key does; [`review_row_at`]
//! hit-tests through the same placement the draw uses.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use unicode_width::UnicodeWidthStr;

use super::column::COLUMN_HEAD;
use super::{FileListView, Glyphs, Placement, cells_u16, plan};
use crate::fit::fit;
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// What the list draws after its files.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ListTail {
    /// Nothing: a clash is open, or the operation is not a merge (its
    /// finish row stays on the notice row).
    #[default]
    None,
    /// `✓ Review the merge  y`.
    Review {
        /// `resolver.review` as his keymap spells it.
        key: String,
    },
}

impl ListTail {
    /// Rows the tail takes under the files.
    pub(super) fn rows(&self) -> u16 {
        match self {
            ListTail::None => 0,
            ListTail::Review { .. } => 1,
        }
    }
}

/// The tail's words before its key.
const REVIEW: &str = "Review the merge";

/// The tail's row in a list `width` x `height` cells, and its words, key
/// last: `None` with no tail, or no row left for it.
pub(super) fn tail_row(
    view: &FileListView,
    placement: Placement,
    width: u16,
    height: u16,
    g: Glyphs,
) -> Option<(u16, String)> {
    let ListTail::Review { key } = &view.tail else {
        return None;
    };
    let laid = plan(view, placement, width, height, g);
    let y = match placement {
        // Under the last file (or `… N more`); the column keeps a row for
        // it when it windows its files.
        Placement::Column { .. } => laid
            .cells
            .iter()
            .map(|cell| cell.y.saturating_add(1))
            .max()
            .unwrap_or(COLUMN_HEAD),
        Placement::Strip { .. } => laid.rows,
    };
    let text = fit(&format!("{} {REVIEW}  {key}", g.check), usize::from(width));
    (y < height).then_some((y, text))
}

/// Whether `(dx, dy)` from the list's origin, in a list `width` x `height`
/// cells, is on the drawn tail — a click there does what its key does.
#[must_use]
pub fn review_row_at(
    view: &FileListView,
    placement: Placement,
    width: u16,
    height: u16,
    dx: u16,
    dy: u16,
) -> bool {
    tail_row(view, placement, width, height, Glyphs::UNICODE)
        .is_some_and(|(y, text)| y == dy && dx < cells_u16(text.width()))
}

/// Draws the tail into the list's `area`: the tick and words in `added`,
/// the key as the name is drawn, bold. Colour unverified by capture.
pub(super) fn draw_tail(
    view: &FileListView,
    placement: Placement,
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    g: Glyphs,
) {
    let Some((y, text)) = tail_row(view, placement, area.width, area.height, g) else {
        return;
    };
    let y = area.y.saturating_add(y);
    super::set_text(buf, area.x, y, area, &text, foreground(theme, Token::Added));
    let words = cells_u16(format!("{} {REVIEW}  ", g.check).width());
    let key = Rect {
        x: area.x.saturating_add(words),
        y,
        width: cells_u16(text.width()).saturating_sub(words),
        height: 1,
    }
    .intersection(area);
    buf.set_style(
        key,
        foreground(theme, Token::Text).add_modifier(Modifier::BOLD),
    );
}
