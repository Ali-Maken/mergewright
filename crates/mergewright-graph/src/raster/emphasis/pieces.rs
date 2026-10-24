//! A strip's emphasis in pieces (ADR-0053, amendment 2026-10-10, amended
//! after m8 05 continuation 1).
//!
//! One emphasis image per strip is keyed on the whole strip's paint, so a
//! `j` that changes any lit bit anywhere in 32 rows × the whole graph's
//! width made a new picture and the terminal was sent all of it again. Here
//! the same emphasis is cut into **pieces of three kinds**, each holding
//! exactly the elements it owns:
//! - a **run** of lane column `c`: the straight edges drawn in `c`;
//! - an **arc** of `c` on row `r`: the arc the pass draws in `c` on `r`
//!   (a row draws at most one per column, closing or opening);
//! - a **node** of lane column `c`: the node, lane stubs and tail of every
//!   row whose lane is `c`, and, when `c` is the marker column, the rail —
//!   the pass draws the rail with the nodes (`Pen::row`), not the edges.
//!
//! A piece draws its elements alone at `(a − b) / (1 − b)`, through the same
//! pen as the whole emphasis (node discs on its rows still punched), cropped
//! to the whole cells its pixels touch, columns and rows.
//!
//! **Key** = hash of exactly what the pixels are drawn from: kind, cell
//! size, backdrop alpha, and per owned element its geometry relative to the
//! piece's anchor (`(c, 0)` for runs and nodes, `(c, r)` for an arc), its
//! colour and the alpha byte it draws at. No strip key, no absolute row or
//! column: one picture anywhere on screen is one key.
//!
//! **Stacking** — [`emphasis_pieces`] lists pieces bottom first, in the
//! whole pass's order: edges column by column (each column's run, then its
//! arcs top to bottom; those never share a pixel), then nodes column by
//! column. Pieces composited in that order give the single emphasis
//! (`raster_pieces::pieces_composite_equals_single_emphasis`).

mod extent;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use mergewright_core::capabilities::Rgb;

use super::super::geometry::goes_down;
use super::super::{
    CELLS_PER_LANE, Edge, Inked, LaneStroke, Part, RasterError, RasterPool, STRIP_LEAD_CELLS,
    alpha_byte, edge_at, lifted, paint_part,
};
use super::{Backdrop, Emphasised, PlainStrip, Selection, crop, painted_cells_in, painted_rows};
use crate::identity::BranchIdentity;
use extent::{extent, node_cells, signed, signed_cells};

/// Which elements of a lane column a piece holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PieceKind {
    /// The straight edges drawn in the column.
    Run,
    /// The nodes, lane stubs and tails of rows on the column's lane; the
    /// rail when the column is the marker.
    Node,
    /// The arc drawn in the column on strip row `row`.
    Arc {
        /// Strip-relative row.
        row: u16,
    },
}

/// One piece of a strip's emphasis: the lane column it belongs to, its
/// kind, and the key its pixels are known by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Piece {
    /// The lane column the piece is anchored on.
    pub column: usize,
    /// Which of the column's elements it holds.
    pub kind: PieceKind,
    /// Hash of what its pixels are drawn from, relative to its anchor:
    /// equal keys are equal pixels, wherever they sit.
    pub key: u64,
}

/// The pieces `strip`'s emphasis needs under `selection`, bottom first in
/// stacking order; nothing is rasterised. Empty when nothing draws above
/// the backdrop — always under [`Backdrop::Full`] — or when `selection`
/// does not have one entry per strip row (as [`rasterise_piece`] reports).
#[must_use]
pub fn emphasis_pieces(strip: &PlainStrip<'_>, selection: &Selection<'_>) -> Vec<Piece> {
    if selection.backdrop == Backdrop::Full
        || selection.rows.len() != strip.rows.len()
        || u16::try_from(strip.rows.len()).is_err()
    {
        return Vec::new();
    }
    let keys = Keys { strip, selection };
    let widest = strip
        .rows
        .iter()
        .map(|plain| plain.row.lanes.len().max(plain.row.lane + 1))
        .chain(strip.marker.map(|marker| marker.column + 1))
        .max()
        .unwrap_or(0);
    let mut pieces = Vec::new();
    for column in 0..widest {
        let mut run = keys.hasher(PieceKind::Run);
        let mut runs = false;
        let mut arcs = Vec::new();
        for (index, plain, alpha, identity, edge) in keys.edges(column) {
            let rgb = keys.colour(Some(identity));
            match edge {
                Edge::Straight => {
                    runs = true;
                    (index, rgb, alpha).hash(&mut run);
                }
                Edge::Arc { incoming } => {
                    // Fits: a strip of more rows has no pieces (above).
                    let row = u16::try_from(index).unwrap_or(u16::MAX);
                    let mut arc = keys.hasher(PieceKind::Arc { row });
                    let toward = signed(plain.row.lane) - signed(column);
                    let head = matches!(plain.ring, super::super::Ring::Head);
                    (incoming, toward, head, rgb, alpha).hash(&mut arc);
                    arcs.push(Piece {
                        column,
                        kind: PieceKind::Arc { row },
                        key: arc.finish(),
                    });
                }
            }
        }
        if runs {
            pieces.push(Piece {
                column,
                kind: PieceKind::Run,
                key: run.finish(),
            });
        }
        pieces.extend(arcs);
    }
    for column in 0..widest {
        if let Some(key) = keys.node(column) {
            pieces.push(Piece {
                column,
                kind: PieceKind::Node,
                key,
            });
        }
    }
    pieces
}

/// The reading of a strip both the keys and the extents make.
struct Keys<'k, 's> {
    strip: &'k PlainStrip<'s>,
    selection: &'k Selection<'s>,
}

/// One drawn edge: strip row, its row, the alpha byte, branch, shape.
type Drawn<'k, 's> = (usize, &'k super::PlainRow<'s>, u8, BranchIdentity, Edge);

impl<'k, 's> Keys<'k, 's> {
    fn backdrop(&self) -> f32 {
        self.selection.backdrop.alpha()
    }

    /// The alpha byte an element at ADR-0024 alpha `alpha` draws at in the
    /// emphasis, or `None` when it is not above the backdrop.
    fn byte(&self, alpha: f32) -> Option<u8> {
        lifted(alpha, self.backdrop()).map(alpha_byte)
    }

    /// The colour and stroke the pen draws a branch in, or the muted hue.
    /// An empty palette draws nothing: `rasterise_piece` reports it.
    fn colour(&self, identity: Option<BranchIdentity>) -> (Rgb, LaneStroke) {
        identity.map_or((self.strip.muted, LaneStroke::Thin), |identity| {
            self.strip.look().lane(identity)
        })
    }

    /// A hasher holding what every piece of `kind`'s pixels depend on: the
    /// kind (not its row), the cell size and the backdrop.
    fn hasher(&self, kind: PieceKind) -> DefaultHasher {
        let mut hasher = DefaultHasher::new();
        let tag: u8 = match kind {
            PieceKind::Run => 0,
            PieceKind::Arc { .. } => 1,
            PieceKind::Node => 2,
        };
        (tag, self.strip.cell_width, self.strip.cell_height).hash(&mut hasher);
        self.backdrop().to_bits().hash(&mut hasher);
        hasher
    }

    /// Every edge lane column `column` draws above the backdrop, top to
    /// bottom.
    fn edges(&self, column: usize) -> impl Iterator<Item = Drawn<'k, 's>> + '_ {
        let marker = self.strip.marker;
        self.strip
            .rows
            .iter()
            .zip(self.selection.rows)
            .enumerate()
            .filter_map(move |(index, (plain, lit))| {
                let (identity, edge) = edge_at(plain.row, marker, column)?;
                let alpha = self.byte(lit.alpha_at(column))?;
                Some((index, plain, alpha, identity, edge))
            })
    }

    /// The key of lane column `column`'s node piece, or `None` when it draws
    /// nothing above the backdrop.
    fn node(&self, column: usize) -> Option<u64> {
        let mut hasher = self.hasher(PieceKind::Node);
        let is_marker = self
            .strip
            .marker
            .is_some_and(|marker| marker.column == column);
        let mut draws = false;
        for (index, (plain, lit)) in self.strip.rows.iter().zip(self.selection.rows).enumerate() {
            let row = plain.row;
            if row.lane == column {
                // The disc is punched on every node row, lit or not.
                let lit = self.byte(lit.alpha_at(column)).map(|alpha| {
                    let above = match index.checked_sub(1) {
                        Some(previous) => Some(self.strip.rows[previous].row),
                        None => self.strip.above,
                    };
                    let upper = above.is_some_and(|above| goes_down(above, column));
                    let lower = row.outgoing.iter().any(|&(out, _)| out == column);
                    let tail = plain
                        .stubs
                        .first()
                        .map(|stub| (self.colour(Some(stub.identity)), stub.toward));
                    let ink = self.colour(row.lanes.get(column).copied().flatten());
                    (alpha, ink, upper, lower, tail)
                });
                draws |= lit.is_some();
                (index, 0u8, plain.ring, lit).hash(&mut hasher);
            }
            if is_marker && let Some(alpha) = self.byte(lit.rail_alpha()) {
                draws = true;
                (index, 1u8, alpha, self.strip.muted).hash(&mut hasher);
            }
        }
        // What of the cells around the anchor the strip has room for: a node
        // on the first or last lane reaches past the strip's edge, clipped.
        let cells = node_cells(self.strip, column);
        let anchor = signed_cells(STRIP_LEAD_CELLS) + signed(column) * signed_cells(CELLS_PER_LANE);
        (cells.start - anchor, cells.end - anchor).hash(&mut hasher);
        draws.then(|| hasher.finish())
    }
}

/// One piece's pixels, cropped to the whole cells they touch in both axes,
/// placed at the strip's cell column `first_column` and row `first_row`;
/// `None` if it draws nothing.
///
/// Drawn in a pixmap only as large as the piece's elements reach, never a
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
    let keys = Keys { strip, selection };
    let Some((rows, cells)) = extent(&keys, piece) else {
        return Ok(None);
    };
    let (Ok(first_cell), Ok(end_cell)) = (u32::try_from(cells.start), u32::try_from(cells.end))
    else {
        return Err(too_large(strip));
    };
    let inked: Vec<Inked<'_>> = strip
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
    let part = Part {
        column: piece.column,
        kind: piece.kind,
        first_cell,
        cells: end_cell - first_cell,
        first_row: rows.start,
        rows: rows.len(),
    };
    let frame = paint_part(
        &strip.look(),
        &inked,
        Some(keys.backdrop()),
        Some(part),
        &mut RasterPool::new(),
    )?;
    let Some((first, last)) = painted_cells_in(&frame, strip.cell_width, 0..usize::MAX) else {
        return Ok(None);
    };
    let Some((top, bottom)) = painted_rows(&frame) else {
        return Ok(None);
    };
    let first_column = u16::try_from(first_cell + first).map_err(|_| too_large(strip))?;
    let first_row = u16::try_from(rows.start + top).map_err(|_| too_large(strip))?;
    Ok(Some(Emphasised {
        frame: Arc::new(crop(frame, strip.cell_width, (first, last), (top, bottom))),
        first_column,
        first_row,
    }))
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
