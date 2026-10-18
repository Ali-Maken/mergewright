//! The resolver's arrangements by frame width (ADR-0043 §1, ease-pass/11):
//! where the list goes and whether a clash's sides sit side by side.
//! Inline at every width from [`INLINE_FROM`]: the three panes are retired
//! (ease-pass/11b, §1.5 edit 2026-10-17).

/// Frame columns from which the list is a column beside the document and
/// an open clash draws yours | theirs side by side (ease-pass/11, design
/// note "Clash screen"). Below it the list is a strip and the sides stack.
/// Measured on the frame, not the document: beside a 30-cell list the
/// document at 120 columns is 89 wide, and the sides still fit.
pub const INLINE_FROM: u16 = 100;

/// The list column's width, without its `│` (the merge mock's column).
pub const LIST_COLUMN: u16 = 30;

/// The resolver's arrangements (ADR-0043 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrangement {
    /// From [`INLINE_FROM`] up, at every width: a list column, then the
    /// document, an open clash's sides side by side.
    Inline,
    /// Below [`INLINE_FROM`]: a list strip, then the document, an open
    /// clash's sides stacked.
    Card,
}

/// Which arrangement a frame this wide gets.
#[must_use]
pub fn arrangement(frame_columns: u16) -> Arrangement {
    if frame_columns >= INLINE_FROM {
        Arrangement::Inline
    } else {
        Arrangement::Card
    }
}
