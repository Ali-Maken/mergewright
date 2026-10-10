//! What a frame asks [`super::KittyImages`] to show: one [`Place`] per
//! image, in a [`CellRect`], known by its [`UploadKey`].

use std::ops::Range;

use graft_core::raster::RasterFrame;

use super::Layer;

/// A rectangle of character cells, zero-based from the screen's top left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRect {
    /// Leftmost column.
    pub column: u16,
    /// Top row.
    pub row: u16,
    /// Width in cells.
    pub columns: u16,
    /// Height in cells. `0` means "as many as the crop has rows".
    pub rows: u16,
}

/// What a picture is, for the upload cache: equal keys in one slot are
/// equal pixels. The strip cache's own key for the strip or layer (layout,
/// theme, cell size, backdrop, selection), so a strip the cache evicted and
/// drew again is still the image the terminal holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UploadKey(pub u64);

/// One strip to show this frame: which rows of it, and in which cells.
#[derive(Clone, Debug)]
pub struct Place<'a> {
    /// The history row the strip starts on; identifies the strip's slot.
    pub first_row: usize,
    /// What the pixels are. A different key for the same slot means the
    /// pixels changed and the strip is transmitted again.
    pub key: UploadKey,
    /// The pixels; read only to transmit them. `None` only when the caller
    /// asked [`KittyImages::holds`] this frame and the terminal does: placed
    /// by key, never rasterised (an emphasis piece, ADR-0053 amendment
    /// 2026-10-10). A `None` the terminal turns out not to hold is left
    /// unplaced this frame, and `holds` says no on the next, so it is sent
    /// then — never placed blank.
    pub frame: Option<&'a RasterFrame>,
    /// Rows of the strip to show, counted from the strip's own first row.
    pub rows: Range<usize>,
    /// Where on screen those rows go.
    pub cells: CellRect,
    /// The strip itself, or an emphasis piece above it. A strip is one
    /// upload per slot; emphasis uploads are known by their key alone, any
    /// number per strip.
    pub layer: Layer,
    /// Rank in its layer, higher on top: sent as `z = 1 + stack` for an
    /// emphasis place (the protocol stacks overlapping images by `z`, and by
    /// image id only at equal `z`), so the screen order is this rank, never
    /// the order commands are written in. A strip ignores it (`z` 0); 0 for
    /// strips and iTerm2.
    pub stack: u16,
}
