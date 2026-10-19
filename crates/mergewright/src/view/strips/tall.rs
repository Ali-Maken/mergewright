//! The cursor row's second line at the Pixel tier (ease-pass/13, mock G9):
//! which line of the graph column it sits under, so the strips make room.
//!
//! The text tiers draw the second line as text with lanes continuing
//! through it (07b). At the Pixel tier the text is the same; the lanes are
//! the strips', so the strip holding the cursor row is placed in two parts
//! with a one-row continuation band between them, and every strip below it
//! sits one row lower. Which line that is comes from the same
//! `cursor_body_line` the click hit-test and `history_row_line` read, so a
//! click, the text and the pixels agree on where every row is.

use crate::update::{AppState, cursor_body_line};

/// The graph column line the cursor row is drawn on while it draws its
/// second line on the line under it: counted from the column's top, the
/// uncommitted row first when it is pinned there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TallRow {
    /// The cursor row's line; its second line is `line + 1`.
    pub(crate) line: u16,
}

impl TallRow {
    /// This frame's tall row: `None` while the cursor is off screen or a
    /// Ghost Preview is open (no row draws a second line then).
    pub(crate) fn of(state: &AppState) -> Option<TallRow> {
        cursor_body_line(state).map(|line| TallRow { line })
    }
}
