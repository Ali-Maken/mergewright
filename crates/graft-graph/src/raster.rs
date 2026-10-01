//! The pixel tier's rasteriser: laid-out rows in, a premultiplied RGBA strip
//! out (PROMPT.md §1.1, DESIGN.md §3.8).
//!
//! Pure drawing: no I/O, no cache (that is the strip cache's job), no escape
//! sequences (that is `term`'s). Each row is one band `cell_height` pixels
//! tall; each lane column is [`CELLS_PER_LANE`] cells wide, with the lane's
//! centre over the first cell, where the Unicode tier puts its glyph.
//!
//! What a band holds, read from the row alone (plus the row above, for the
//! node's upper stub):
//! - a lane passing straight through: a vertical line, band top to bottom;
//! - a lane closing into the node: a quarter-ellipse from the band's top
//!   edge into the node, leaving vertically and arriving level;
//! - a lane opening from the node: the mirror image, down to the band's
//!   bottom edge;
//! - an edge into a folded lane: a short tail leaving the node's right edge
//!   and fading to nothing before the next column (ADR-0022);
//! - the rail, when anything folds: a 1 px dotted vertical in the muted hue
//!   on the marker column's centre line, three dashes a band;
//! - the node itself, on a disc punched clear of every line, drawn in its
//!   [`Ring`] style.
//!
//! The background is never painted (ADR-0003): a pixel no line, node or
//! rail touches stays `[0, 0, 0, 0]`, and the terminal composites.
//!
//! Colour depth never dashes a lane here: an image carries full RGB whatever
//! the terminal's text palette, so the Unicode tier's odd-column dashing
//! (low colour depth only) has no pixel counterpart.

pub mod cache;
mod emphasis;
mod geometry;
mod node;
mod pen;

use std::cell::RefCell;
use std::collections::BTreeSet;

use graft_core::capabilities::Rgb;
use graft_core::raster::RasterFrame;
use tiny_skia::Pixmap;

use crate::fold::Toward;
use crate::identity::BranchIdentity;
use crate::row::Row;

pub use emphasis::{
    Backdrop, Emphasised, Lit, PlainRow, PlainStrip, Selection, composite, rasterise_backdrop,
    rasterise_emphasis,
};
pub use node::Ring;

/// Terminal cells one lane column takes, matching the Unicode tier: the
/// lane's own cell and the gap right of it.
pub const CELLS_PER_LANE: u32 = 2;

/// Alpha for a column off the selection's thread (ADR-0024): the thread is
/// emphasised by contrast, never by a heavier stroke.
pub const OFF_THREAD_ALPHA: f32 = 0.5;

/// Alpha for a whole row faded by Focus mode ([`Emphasis::Dimmed`]).
pub const FADED_ALPHA: f32 = 0.3;

/// The one stroke width every line and ring in a strip draws at: lanes,
/// curves and node rings alike (ADR-0024). Emphasis is never a wider
/// stroke, only alpha, so a lane reads as a line to follow at any size.
pub(crate) fn stroke_width(cell_width: u32) -> f32 {
    #[allow(clippy::cast_precision_loss)] // Cell widths are far below f32's exact range.
    let cell_width = cell_width as f32;
    (cell_width / 8.0).clamp(1.5, 2.5)
}

/// How strongly a whole row is drawn when no thread singles out its columns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Emphasis {
    /// Drawn as laid out.
    #[default]
    Normal,
    /// Faded: the row is off the selection's thread while Focus is on.
    Dimmed,
}

impl Emphasis {
    fn alpha(self) -> f32 {
        match self {
            Emphasis::Normal => 1.0,
            Emphasis::Dimmed => FADED_ALPHA,
        }
    }
}

/// An edge a fold took out of a row (`fold::FoldedEdge`), to draw as a short
/// tail on the node rather than a line to the rail (ADR-0022).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Stub {
    /// The folded branch: the tail's colour.
    pub identity: BranchIdentity,
    /// Whether the tail leads down or up.
    pub toward: Toward,
}

/// One row to rasterise and how to paint it.
#[derive(Clone, Debug)]
pub struct RasterRow<'a> {
    /// The row, in drawn columns (a `Fold::view` when anything folds).
    pub row: &'a Row,
    /// The node's ring style.
    pub ring: Ring,
    /// The whole row's emphasis when no thread is given.
    pub emphasis: Emphasis,
    /// Drawn columns carrying the selection's thread on this row. When set,
    /// those columns draw at full alpha and every other column at
    /// [`OFF_THREAD_ALPHA`], overriding `emphasis` (the Unicode tier's
    /// bold/dim rule, ticket 16). Never a wider stroke (ADR-0024).
    pub thread: Option<&'a BTreeSet<usize>>,
    /// Edges into folded lanes, drawn as stubs on the node; empty when
    /// nothing on this row folds.
    pub stubs: Vec<Stub>,
}

impl<'a> RasterRow<'a> {
    /// A row with the ring its parent count implies and normal emphasis.
    pub fn plain(row: &'a Row) -> Self {
        RasterRow {
            row,
            ring: Ring::of(row),
            emphasis: Emphasis::Normal,
            thread: None,
            stubs: Vec::new(),
        }
    }

    /// The row as the pen draws it.
    fn inked(&self) -> Inked<'_> {
        Inked {
            row: self.row,
            ring: self.ring,
            stubs: &self.stubs,
            lit: Lit {
                emphasis: self.emphasis,
                thread: self.thread,
            },
        }
    }
}

/// The rail: the drawn column folded lanes sit behind. It carries no count
/// (ADR-0022); the screen header says how many lanes fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Marker {
    /// The drawn column the rail takes (`Fold::marker`).
    pub column: usize,
}

/// Everything one strip is drawn from.
#[derive(Clone, Copy, Debug)]
pub struct Strip<'a> {
    /// The strip's rows, top to bottom.
    pub rows: &'a [RasterRow<'a>],
    /// The row just above the strip, if any: whether the first node's lane
    /// arrives from above depends on it.
    pub above: Option<&'a Row>,
    /// Lane columns the strip is wide, marker included.
    pub lane_columns: usize,
    /// The rail column, if anything folds.
    pub marker: Option<Marker>,
    /// Cell width in pixels.
    pub cell_width: u32,
    /// Cell height in pixels.
    pub cell_height: u32,
    /// Lane colours; an identity draws in `palette[identity.slot(len)]`.
    pub palette: &'a [Rgb],
    /// The rail's colour.
    pub muted: Rgb,
}

/// Why a strip could not be rasterised.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RasterError {
    /// No lane colours were given, so no lane could be drawn.
    #[error("the lane palette is empty, so no lane has a colour to draw in")]
    EmptyPalette,
    /// The pixel buffer would not fit in memory's addressable size.
    #[error(
        "a strip of {rows} rows and {lane_columns} lanes at a {cell_width}×{cell_height} px cell is too large to rasterise"
    )]
    TooLarge {
        /// Rows asked for.
        rows: usize,
        /// Lane columns asked for.
        lane_columns: usize,
        /// Cell width asked for.
        cell_width: u32,
        /// Cell height asked for.
        cell_height: u32,
    },
    /// A selection's paint does not have one entry per strip row. A caller
    /// bug: the selection is built from the same rows as the strip.
    #[error("a selection of {lit} rows was given for a strip of {rows} rows")]
    Selection {
        /// Rows in the strip.
        rows: usize,
        /// Rows in the selection.
        lit: usize,
    },
}

/// Rasterise a strip of rows into a premultiplied RGBA frame.
pub fn rasterise(strip: &Strip<'_>) -> Result<RasterFrame, RasterError> {
    let rows: Vec<Inked<'_>> = strip.rows.iter().map(RasterRow::inked).collect();
    paint(&strip.look(), &rows, None)
}

impl Strip<'_> {
    fn look(&self) -> Look<'_> {
        Look {
            above: self.above,
            lane_columns: self.lane_columns,
            marker: self.marker,
            cell_width: self.cell_width,
            cell_height: self.cell_height,
            palette: self.palette,
            muted: self.muted,
        }
    }
}

/// Everything a strip is drawn from except its rows: shared by today's
/// [`Strip`] and the layout-only [`PlainStrip`].
#[derive(Clone, Copy, Debug)]
struct Look<'a> {
    above: Option<&'a Row>,
    lane_columns: usize,
    marker: Option<Marker>,
    cell_width: u32,
    cell_height: u32,
    palette: &'a [Rgb],
    muted: Rgb,
}

/// One row as the pen draws it: its layout and the paint it draws at.
#[derive(Clone, Copy, Debug)]
struct Inked<'a> {
    row: &'a Row,
    ring: Ring,
    stubs: &'a [Stub],
    lit: Lit<'a>,
}

/// Draw `rows` into a premultiplied RGBA frame.
///
/// `lift` is `None` for a whole strip: every element at its own alpha. For
/// an emphasis layer it is the backdrop's alpha `b`: only elements whose
/// alpha `a` is above it are drawn, at `(a − b) / (1 − b)`, so that drawn
/// over the backdrop they come back to `a` (ADR-0053). Either way the order
/// and the geometry are the single pass's, so a layer holds exactly the lit
/// part of it.
fn paint(
    look: &Look<'_>,
    rows: &[Inked<'_>],
    lift: Option<f32>,
) -> Result<RasterFrame, RasterError> {
    if look.palette.is_empty() {
        return Err(RasterError::EmptyPalette);
    }
    let too_large = || RasterError::TooLarge {
        rows: rows.len(),
        lane_columns: look.lane_columns,
        cell_width: look.cell_width,
        cell_height: look.cell_height,
    };
    let columns = u32::try_from(look.lane_columns)
        .ok()
        .and_then(|lanes| lanes.checked_mul(CELLS_PER_LANE))
        .ok_or_else(too_large)?;
    let width = columns.checked_mul(look.cell_width).ok_or_else(too_large)?;
    let height = u32::try_from(rows.len())
        .ok()
        .and_then(|count| count.checked_mul(look.cell_height))
        .ok_or_else(too_large)?;
    let frame = |pixels| RasterFrame {
        width,
        height,
        rows: rows.len(),
        columns,
        pixels,
    };
    if width == 0 || height == 0 {
        return Ok(frame(Vec::new()));
    }
    let mut pixmap = Pixmap::new(width, height).ok_or_else(too_large)?;
    let pen = Pen::new(look, rows, lift);
    // Edges column by column down the whole strip, so a lane passing
    // straight through many rows is one stroke, not one per row. Within any
    // one band the order is unchanged (columns left to right), and edges
    // stay confined to their band, so the pixels are the per-row drawing's.
    let widest = rows
        .iter()
        .map(|inked| inked.row.lanes.len())
        .max()
        .unwrap_or(0);
    for column in 0..widest {
        pen.column(&mut pixmap, column);
    }
    // Then what sits on each row's node: its lane's own stubs, a folded
    // edge's tail, the rail, the node disc punched over the edges.
    for (index, inked) in rows.iter().enumerate() {
        let above = match index.checked_sub(1) {
            Some(previous) => Some(rows[previous].row),
            None => look.above,
        };
        pen.row(&mut pixmap, index, inked, above);
    }
    Ok(frame(pixmap.take()))
}

/// A row's vertical extent in pixels.
#[derive(Clone, Copy, Debug)]
struct Band {
    top: f32,
    mid: f32,
    bottom: f32,
}

/// A colour, its opacity and the stroke width it draws at.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Ink {
    rgb: Rgb,
    alpha: u8,
    width: f32,
}

/// The strip's fixed geometry, shared by every row.
struct Pen<'a> {
    look: &'a Look<'a>,
    rows: &'a [Inked<'a>],
    /// The backdrop's alpha under an emphasis layer; `None` for a whole
    /// strip (see [`paint`]).
    lift: Option<f32>,
    cell_width: f32,
    cell_height: f32,
    stroke: f32,
    node: node::NodeSize,
    /// Scanlines of straight runs already rasterised, by x and ink.
    scanlines: RefCell<Vec<Scanline>>,
}

/// One scanline of a straight vertical line, rasterised once on a clear
/// background: every scanline of the line between integer y bounds is the
/// same, so a run over a clear background is these bytes, copied.
struct Scanline {
    x: f32,
    ink: Ink,
    /// The pixmap column of `pixels`' first pixel.
    left: usize,
    /// Premultiplied RGBA, clear margin included.
    pixels: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::stroke_width;

    #[test]
    #[allow(clippy::float_cmp)] // Exact clamp bounds and an exact division; no rounding involved.
    fn stroke_width_clamps_by_cell_width() {
        assert_eq!(stroke_width(8), 1.5);
        assert_eq!(stroke_width(24), 2.5);
        assert_eq!(stroke_width(17), 17.0 / 8.0);
    }
}
