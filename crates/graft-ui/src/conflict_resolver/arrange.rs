//! The resolver's arrangements by frame width (ADR-0043 §1, ease-pass/11):
//! where the list goes and whether a clash's sides sit side by side.

/// Frame columns from which the list is a column beside the document and
/// an open clash draws yours | theirs side by side (ease-pass/11, design
/// note "Clash screen"). Below it the list is a strip and the sides stack.
/// Measured on the frame, not the document: beside a 30-cell list the
/// document at 120 columns is 89 wide, and the sides still fit.
pub const INLINE_FROM: u16 = 100;

/// The list column's width, without its `│` (the merge mock's column).
pub const LIST_COLUMN: u16 = 30;

/// Frame columns at or above which the resolver is three panes beside a
/// file-list column (ADR-0043 §1); below it, the document inline.
pub const THREE_PANE_MIN: u16 = 180;

/// The resolver's arrangements (ADR-0043 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrangement {
    /// At [`THREE_PANE_MIN`] columns and up: a list column, then panes.
    ThreePane,
    /// From [`INLINE_FROM`] up to [`THREE_PANE_MIN`]: a list column, then
    /// the document, an open clash's sides side by side.
    Inline,
    /// Below [`INLINE_FROM`]: a list strip, then the document, an open
    /// clash's sides stacked.
    Card,
}

/// Which arrangement a frame this wide gets.
#[must_use]
pub fn arrangement(frame_columns: u16) -> Arrangement {
    if frame_columns >= THREE_PANE_MIN {
        Arrangement::ThreePane
    } else if frame_columns >= INLINE_FROM {
        Arrangement::Inline
    } else {
        Arrangement::Card
    }
}
