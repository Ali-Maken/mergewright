//! A strip's emphasis in per-column pieces (ADR-0053, amendment
//! 2026-10-10).
//!
//! One emphasis image per strip is keyed on the whole strip's paint, so a
//! `j` that changes any lit bit anywhere in 32 rows × the whole graph's
//! width made a new picture and the terminal was sent all of it again. Here
//! the same emphasis is cut into **pieces, one per drawn lane column**, each
//! keyed only on the alphas of the elements it owns. A `j` that leaves a
//! column's alphas alone leaves its piece's key, and a terminal that holds
//! that key is only told where to put it.
//!
//! **Who owns what** — every element the pen draws is decided by exactly one
//! column's alpha, and that column owns it:
//! - an edge in column `c` (a straight line, or an arc between `c` and the
//!   node, diagonal included): column `c`, whose `alpha_at(c)` it draws at;
//! - a node, its own lane stubs and its folded-edge tail: the node's lane;
//! - the rail: the marker column.
//!
//! A piece draws its elements alone at `(a − b) / (1 − b)`, through the
//! same pen as the whole emphasis (node discs still punched), cropped to the
//! whole cells its pixels touch. So backdrop + every piece is the single
//! emphasis composited, exactly where pieces don't overlap
//! (`raster::pieces_composite_equals_single_emphasis`).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::super::{CELLS_PER_LANE, Inked, Part, RasterError, RasterPool, paint_part};
use super::{Backdrop, Emphasised, PlainStrip, Selection, crop, lit_reach, painted_cells_in};

/// One piece of a strip's emphasis: the lane column it holds the elements
/// of, and the key its pixels are known by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Piece {
    /// The drawn lane column whose elements the piece holds.
    pub column: usize,
    /// Hash of the strip's backdrop key, the column, and the alpha of each
    /// element the piece owns: equal keys are equal pixels.
    pub key: u64,
}

/// An element the pen draws, as the piece key sees it.
#[derive(Hash)]
enum Element {
    /// An edge in the owning column on this row.
    Edge,
    /// The node, its lane stubs and tail, on this row.
    Node,
    /// The rail on this row.
    Rail,
}

/// The pieces `strip`'s emphasis needs under `selection`, left to right;
/// nothing is rasterised. Empty when nothing draws above the backdrop —
/// always under [`Backdrop::Full`].
///
/// `strip_key` is the strip's backdrop key (`StripCache::backdrop_key`):
/// layout, theme, cell size and backdrop. A piece's key adds only its column
/// and the alphas of what it owns, so a cursor move that leaves those alone
/// leaves the key.
///
/// `selection` has one entry per strip row, as [`rasterise_piece`] checks;
/// built from the same rows, as `with_layers` does, it always has.
#[must_use]
pub fn emphasis_pieces(
    strip: &PlainStrip<'_>,
    selection: &Selection<'_>,
    strip_key: u64,
) -> Vec<Piece> {
    if selection.backdrop == Backdrop::Full {
        return Vec::new();
    }
    let backdrop = selection.backdrop.alpha();
    let widest = strip
        .rows
        .iter()
        .map(|plain| plain.row.lanes.len().max(plain.row.lane + 1))
        .chain(strip.marker.map(|marker| marker.column + 1))
        .max()
        .unwrap_or(0);
    let mut pieces = Vec::new();
    for column in 0..widest {
        let mut hasher = DefaultHasher::new();
        strip_key.hash(&mut hasher);
        column.hash(&mut hasher);
        let mut draws = false;
        owned(strip, selection, column, |row, element, alpha| {
            (row, element, alpha.to_bits()).hash(&mut hasher);
            draws |= alpha > backdrop;
        });
        if draws {
            pieces.push(Piece {
                column,
                key: hasher.finish(),
            });
        }
    }
    pieces
}

/// Every element lane column `column` owns, row by row, with the alpha
/// ADR-0024 gives it: the same reading of the row the pen makes
/// (`Pen::column`, `Pen::row`, `Pen::rail`).
fn owned(
    strip: &PlainStrip<'_>,
    selection: &Selection<'_>,
    column: usize,
    mut each: impl FnMut(usize, Element, f32),
) {
    let is_marker = strip.marker.map(|marker| marker.column) == Some(column);
    for (index, (plain, lit)) in strip.rows.iter().zip(selection.rows).enumerate() {
        let row = plain.row;
        if row.lanes.get(column).copied().flatten().is_some() && column != row.lane {
            let incoming = row.incoming.contains(&column);
            let outgoing = row.outgoing.iter().any(|&(out, _)| out == column);
            // The marker column draws no straight edge of its own.
            if incoming || outgoing || !is_marker {
                each(index, Element::Edge, lit.alpha_at(column));
            }
        }
        if row.lane == column {
            each(index, Element::Node, lit.alpha_at(column));
        }
        if is_marker {
            each(index, Element::Rail, lit.rail_alpha());
        }
    }
}

/// One piece's pixels, cropped to the whole cells they touch, placed at the
/// strip cell column `first_column`; `None` if it draws nothing.
///
/// Drawn in a pixmap only as wide as the piece's elements reach, never a
/// whole strip's.
///
/// # Errors
///
/// [`RasterError::Selection`] when `selection` does not have one entry per
/// strip row; [`RasterError::EmptyPalette`] or [`RasterError::TooLarge`] as
/// for a whole strip.
pub fn rasterise_piece(
    strip: &PlainStrip<'_>,
    selection: &Selection<'_>,
    piece: Piece,
) -> Result<Option<Emphasised>, RasterError> {
    if selection.rows.len() != strip.rows.len() {
        return Err(RasterError::Selection {
            rows: strip.rows.len(),
            lit: selection.rows.len(),
        });
    }
    if selection.backdrop == Backdrop::Full {
        return Ok(None);
    }
    let backdrop = selection.backdrop.alpha();
    let rows: Vec<Inked<'_>> = strip
        .rows
        .iter()
        .zip(selection.rows)
        .map(|(plain, lit)| Inked {
            row: plain.row,
            ring: plain.ring,
            stubs: &plain.stubs,
            lit: *lit,
        })
        .collect();
    let Some((first_lane, last_lane)) =
        lit_reach(&rows, strip.marker, backdrop, Some(piece.column))
    else {
        return Ok(None);
    };
    let Some((first_cell, end_cell)) = piece_cells(strip, first_lane, last_lane)? else {
        return Ok(None);
    };
    let part = Part {
        column: piece.column,
        first_cell,
        cells: end_cell - first_cell,
    };
    let frame = paint_part(
        &strip.look(),
        &rows,
        Some(backdrop),
        Some(part),
        &mut RasterPool::new(),
    )?;
    let Some((first, last)) = painted_cells_in(&frame, strip.cell_width, 0..usize::MAX) else {
        return Ok(None);
    };
    let first_column = u16::try_from(first_cell + first).map_err(|_| too_large(strip))?;
    Ok(Some(Emphasised {
        frame: Arc::new(crop(frame, strip.cell_width, first, last)),
        first_column,
    }))
}

/// The strip cells a piece reaching lane columns `first_lane..=last_lane`
/// is drawn in, `first..end`: those `painted_cells` would scan for that
/// reach — a cell left of the first lane's own cell to the end of the cell
/// right of the last's — within the strip. `None` when that is no cell.
fn piece_cells(
    strip: &PlainStrip<'_>,
    first_lane: usize,
    last_lane: usize,
) -> Result<Option<(u32, u32)>, RasterError> {
    let cells = |lanes: usize| {
        u32::try_from(lanes)
            .ok()
            .and_then(|lanes| lanes.checked_mul(CELLS_PER_LANE))
            .ok_or_else(|| too_large(strip))
    };
    let strip_cells = cells(strip.lane_columns)?;
    let first = cells(first_lane)?.saturating_sub(1).min(strip_cells);
    let end = cells(last_lane)?.saturating_add(2).min(strip_cells);
    Ok((first < end).then_some((first, end)))
}

/// The error for a strip too large to draw.
fn too_large(strip: &PlainStrip<'_>) -> RasterError {
    RasterError::TooLarge {
        rows: strip.rows.len(),
        lane_columns: strip.lane_columns,
        cell_width: strip.cell_width,
        cell_height: strip.cell_height,
    }
}
