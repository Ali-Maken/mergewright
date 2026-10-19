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
mod pool;
mod stroke;

use std::cell::RefCell;

use mergewright_core::capabilities::Rgb;
use mergewright_core::raster::RasterFrame;
use tiny_skia::{IntSize, Pixmap};

use crate::fold::Toward;
use crate::identity::BranchIdentity;
use crate::role::LaneRole;
use crate::row::Row;

pub use emphasis::{
    Backdrop, Emphasised, Lit, Piece, PieceKind, PlainRow, PlainStrip, Selection, composite,
    emphasis_pieces, rasterise_backdrop, rasterise_emphasis, rasterise_piece,
};
pub use node::Ring;
pub use pool::RasterPool;
pub use stroke::LaneStroke;

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
    #[allow(
        clippy::cast_precision_loss,
        reason = "Cell widths are far below f32's exact range."
    )]
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

/// The rail: the drawn column folded lanes sit behind. It carries no count
/// (ADR-0022); the screen header says how many lanes fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Marker {
    /// The drawn column the rail takes (`Fold::marker`).
    pub column: usize,
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

/// Everything a [`PlainStrip`] is drawn from except its rows.
#[derive(Clone, Copy, Debug)]
struct Look<'a> {
    above: Option<&'a Row>,
    lane_columns: usize,
    marker: Option<Marker>,
    cell_width: u32,
    cell_height: u32,
    palette: &'a [Rgb],
    muted: Rgb,
    roles: Option<&'a [(BranchIdentity, LaneRole)]>,
}

impl Look<'_> {
    /// The colour and stroke `identity`'s lane draws in
    /// ([`stroke::lane_look`]).
    fn lane(&self, identity: BranchIdentity) -> (Rgb, LaneStroke) {
        stroke::lane_look(self.roles, self.palette, self.muted, identity)
    }
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
/// `lift` is `None` for a backdrop strip: every element at its own alpha. For
/// an emphasis layer it is the backdrop's alpha `b`: only elements whose
/// alpha `a` is above it are drawn, at `(a − b) / (1 − b)`, so that drawn
/// over the backdrop they come back to `a` (ADR-0053). Either way the order
/// and the geometry are the same, so a layer holds exactly the lit part of
/// what one pass at each row's own paint would draw.
///
/// The pixels go in a buffer from `pool` (m8 04).
fn paint(
    look: &Look<'_>,
    rows: &[Inked<'_>],
    lift: Option<f32>,
    pool: &mut RasterPool,
) -> Result<RasterFrame, RasterError> {
    paint_part(look, rows, lift, None, pool)
}

/// One emphasis piece's pass (ADR-0053 amendment 2026-10-10, amended after
/// continuation 1): only the elements piece `kind` of lane column `column`
/// owns, into a pixmap covering strip cells `first_cell..first_cell +
/// cells` and strip rows `first_row..first_row + rows` alone.
#[derive(Clone, Copy, Debug)]
struct Part {
    column: usize,
    kind: PieceKind,
    first_cell: u32,
    cells: u32,
    first_row: usize,
    rows: usize,
}

/// What a row draws in a non-node lane column: a straight line through the
/// band, or an arc to or from the node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Edge {
    Straight,
    /// `incoming`: closes into the node from the band's top; else opens
    /// from the node to the band's bottom.
    Arc {
        incoming: bool,
    },
}

/// The edge `row` draws in lane column `column`, and its branch, or `None`
/// when it draws none there: an empty column, the node's own lane, or the
/// marker column passing straight through (it draws no edge of its own).
/// The one reading both [`Pen::column`] and the piece keys make.
fn edge_at(row: &Row, marker: Option<Marker>, column: usize) -> Option<(BranchIdentity, Edge)> {
    let identity = row.lanes.get(column).copied().flatten()?;
    if column == row.lane {
        return None;
    }
    let incoming = row.incoming.contains(&column);
    let outgoing = !incoming && row.outgoing.iter().any(|&(out, _)| out == column);
    if incoming || outgoing {
        Some((identity, Edge::Arc { incoming }))
    } else if marker.is_some_and(|marker| marker.column == column) {
        None
    } else {
        Some((identity, Edge::Straight))
    }
}

/// An element's ADR-0024 alpha `alpha` as an emphasis layer over a
/// backdrop at `backdrop` draws it, `(a − b) / (1 − b)`, or `None` when it
/// is not above the backdrop and the layer leaves it out.
fn lifted(alpha: f32, backdrop: f32) -> Option<f32> {
    (alpha > backdrop).then(|| (alpha - backdrop) / (1.0 - backdrop))
}

/// An alpha in 0.0–1.0 as the byte an [`Ink`] carries.
fn alpha_byte(alpha: f32) -> u8 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "alpha is clamped to 0.0..=1.0."
    )]
    let byte = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    byte
}

/// [`paint`], or with `part` one piece of it: the same pen, the same order,
/// with every element the piece does not own left out (node discs on its
/// rows are still punched, as the whole pass punches them), drawn into a
/// pixmap that starts at `part.first_cell` and `part.first_row`. Whole
/// cells and rows from the strip's origin, so every stroke's sub-pixel
/// position, and so its anti-aliasing, is the whole pass's.
fn paint_part(
    look: &Look<'_>,
    rows: &[Inked<'_>],
    lift: Option<f32>,
    part: Option<Part>,
    pool: &mut RasterPool,
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
    let columns = match part {
        Some(part) => part.cells,
        None => u32::try_from(look.lane_columns)
            .ok()
            .and_then(|lanes| lanes.checked_mul(CELLS_PER_LANE))
            .ok_or_else(too_large)?,
    };
    let window = part.map_or(0..rows.len(), |part| {
        part.first_row..(part.first_row + part.rows).min(rows.len())
    });
    let width = columns.checked_mul(look.cell_width).ok_or_else(too_large)?;
    let height = u32::try_from(window.len())
        .ok()
        .and_then(|count| count.checked_mul(look.cell_height))
        .ok_or_else(too_large)?;
    let frame = |pixels| RasterFrame {
        width,
        height,
        rows: window.len(),
        columns,
        pixels,
    };
    if width == 0 || height == 0 {
        return Ok(frame(Vec::new()));
    }
    let size = IntSize::from_wh(width, height).ok_or_else(too_large)?;
    let bytes =
        usize::try_from(u64::from(width) * u64::from(height) * 4).map_err(|_| too_large())?;
    let mut pixmap = Pixmap::from_vec(pool.take(bytes), size).ok_or_else(too_large)?;
    Pen::new(look, rows, lift, part, window.clone()).pass(&mut pixmap);
    Ok(frame(pixmap.take()))
}

/// A row's vertical extent in pixels.
#[derive(Clone, Copy, Debug)]
struct Band {
    top: f32,
    mid: f32,
    bottom: f32,
}

/// A colour, its opacity, the stroke width it draws at and its shape.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Ink {
    rgb: Rgb,
    alpha: u8,
    width: f32,
    shape: LaneStroke,
}

/// The strip's fixed geometry, shared by every row.
struct Pen<'a> {
    look: &'a Look<'a>,
    rows: &'a [Inked<'a>],
    /// The backdrop's alpha under an emphasis layer; `None` for a whole
    /// strip (see [`paint`]).
    lift: Option<f32>,
    /// The one piece whose elements this pass draws (an emphasis piece):
    /// its lane column and kind; `None` draws everything.
    only: Option<(usize, PieceKind)>,
    /// The strip rows this pass draws: all of them, or a piece's.
    window: std::ops::Range<usize>,
    /// The pixmap's left edge in strip pixels: a piece's pixmap starts
    /// where the piece does. Whole pixels.
    origin: f32,
    /// The pixmap's top edge in strip pixels, likewise. Whole pixels.
    top: f32,
    /// Strip pixels to pixmap pixels: `-origin` across, `-top` down. Every element is
    /// built in strip pixels, exactly as the whole pass builds it, and
    /// moved after: tiny-skia strokes a path before it transforms the
    /// outline, so a piece's strokes are the whole pass's shifted by whole
    /// pixels, anti-aliasing included. Building them at the shifted x
    /// instead rounds differently (seen: alpha 191 vs 175 on an arc).
    shift: tiny_skia::Transform,
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
    #[allow(
        clippy::float_cmp,
        reason = "Exact clamp bounds and an exact division; no rounding involved."
    )]
    fn stroke_width_clamps_by_cell_width() {
        assert_eq!(stroke_width(8), 1.5);
        assert_eq!(stroke_width(24), 2.5);
        assert_eq!(stroke_width(17), 17.0 / 8.0);
    }
}
