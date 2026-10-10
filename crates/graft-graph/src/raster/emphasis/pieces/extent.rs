//! Where a piece is drawn: the strip rows and cells its elements reach.

use std::ops::Range;

use super::super::super::CELLS_PER_LANE;
use super::super::super::Edge;
use super::super::PlainStrip;
use super::{Keys, Piece, PieceKind};

/// The strip rows and cells `piece` is drawn in, or `None` when it draws
/// nothing above the backdrop. Rows: those its elements are on (each stays
/// in its own band). Cells: a cell left of its leftmost lane's own cell to
/// the end of the cell right of its rightmost lane's — a node's halo and
/// every stroke stay inside — and for a node piece one lane more, for a
/// tail; within the strip.
pub(super) fn extent(keys: &Keys<'_, '_>, piece: Piece) -> Option<(Range<usize>, Range<i64>)> {
    let column = piece.column;
    let strip = keys.strip;
    let (rows, lanes): (Option<(usize, usize)>, (usize, usize)) = match piece.kind {
        PieceKind::Run => {
            let rows = keys
                .edges(column)
                .filter(|&(.., edge)| edge == Edge::Straight)
                .fold(None, |span, (index, ..)| Some(widen(span, index)));
            (rows, (column, column))
        }
        PieceKind::Arc { row } => {
            let row = usize::from(row);
            let drawn = keys.edges(column).find(|&(index, ..)| index == row)?;
            let (_, plain, ..) = drawn;
            let lane = plain.row.lane;
            (Some((row, row)), (column.min(lane), column.max(lane)))
        }
        PieceKind::Node => {
            let cells = node_cells(strip, column);
            let is_marker = strip.marker.is_some_and(|marker| marker.column == column);
            let rows = strip
                .rows
                .iter()
                .zip(keys.selection.rows)
                .enumerate()
                .filter(|(_, (plain, lit))| {
                    (plain.row.lane == column && keys.byte(lit.alpha_at(column)).is_some())
                        || (is_marker && keys.byte(lit.rail_alpha()).is_some())
                })
                .fold(None, |span, (index, _)| Some(widen(span, index)));
            let (first, last) = rows?;
            return (cells.start < cells.end).then_some((first..last + 1, cells));
        }
    };
    let (first, last) = rows?;
    let cells = lane_cells(strip, lanes.0, lanes.1, 0);
    (cells.start < cells.end).then_some((first..last + 1, cells))
}

/// `span` widened to hold `index`.
fn widen(span: Option<(usize, usize)>, index: usize) -> (usize, usize) {
    span.map_or((index, index), |(first, last)| {
        (first.min(index), last.max(index))
    })
}

/// The cells a node piece of lane column `column` is drawn in.
pub(super) fn node_cells(strip: &PlainStrip<'_>, column: usize) -> Range<i64> {
    lane_cells(strip, column, column, 1)
}

/// Strip cells from one left of lane `first`'s own cell to the end of the
/// cell right of lane `last + extra`'s, within the strip.
fn lane_cells(strip: &PlainStrip<'_>, first: usize, last: usize, extra: usize) -> Range<i64> {
    let lane = signed_cells(CELLS_PER_LANE);
    let strip_cells = signed(strip.lane_columns) * lane;
    let start = (signed(first) * lane - 1).max(0);
    let end = (signed(last + extra) * lane + 2).min(strip_cells);
    start..end
}

/// A lane column or row as a signed count. Lane columns are far below
/// `i64::MAX` (a strip that wide fails `TooLarge` long before), so the
/// saturation never happens.
pub(super) fn signed(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Cells per lane as a signed count.
pub(super) fn signed_cells(cells: u32) -> i64 {
    i64::from(cells)
}
