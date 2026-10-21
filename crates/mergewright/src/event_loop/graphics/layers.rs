//! What one strip uploads and where it goes: the backdrop and its emphasis
//! pieces (ADR-0053 amendment 2026-10-10), or iTerm2's composited band.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use mergewright_core::raster::RasterFrame;
use mergewright_graph::raster::cache::{CacheError, StripCache};
use mergewright_graph::raster::{
    CELLS_PER_LANE, PieceKind, PlainStrip, Selection, emphasis_pieces, rasterise_piece,
};
use mergewright_term::kitty::{CellRect, KittyImages, Layer, Place, UploadKey};
use ratatui::layout::Rect;

/// What the terminal holds of the emphasis pieces, and where each sits.
pub(super) struct Held<'a> {
    pub(super) images: &'a KittyImages,
    pub(super) pieces: &'a mut HashMap<UploadKey, Crop>,
}

/// Where a held piece's image sits relative to its anchor (its lane
/// column's own cell, and row 0 or its arc's row), and its size in cells:
/// a function of its key (ADR-0053 amendment), so one held image is placed
/// wherever its key turns up, with no raster.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Crop {
    columns_from: i64,
    rows_from: i64,
    columns: u32,
    rows: usize,
}

/// A piece's anchor in the strip: (cell column, row).
fn anchor(column: usize, kind: PieceKind) -> (i64, i64) {
    let cell = i64::try_from(column).unwrap_or(i64::MAX) * i64::from(CELLS_PER_LANE);
    match kind {
        PieceKind::Arc { row } => (cell, i64::from(row)),
        PieceKind::Run | PieceKind::Node => (cell, 0),
    }
}

/// The strip at `first_row` from the cache, and what its upload is: the
/// backdrop and its emphasis pieces (ADR-0053 amendment 2026-10-10), or for
/// iTerm2 one band with both composited (ADR-0053 5: it cannot stack images
/// in a cell).
///
/// A piece the terminal holds is placed with no pixels; one it does not is
/// rasterised here, sent once, and its pixels dropped with the frame.
pub(super) fn layers(
    cache: &mut StripCache,
    held: &mut Held<'_>,
    first_row: usize,
    plain: &PlainStrip<'_>,
    selection: &Selection<'_>,
    iterm2: bool,
) -> Result<Layers, CacheError> {
    if iterm2 {
        let paint_key = UploadKey(cache.paint_key(first_row, plain, selection)?);
        let band = cache.composited(first_row, plain, selection)?;
        return Ok(((band, paint_key), Vec::new()));
    }
    let strip_key = cache.backdrop_key(first_row, plain, selection.backdrop)?;
    let base = cache.backdrop(first_row, plain, selection.backdrop)?;
    let mut pieces = Vec::new();
    // `emphasis_pieces` lists bottom first, in the whole pass's order; the
    // index is the piece's rank, so the terminal stacks it by that rank.
    // Past 65,535 pieces in one strip (3 kinds × lanes × 32 rows can't
    // reach it) ranks would tie at the top, not reorder.
    for (rank, piece) in emphasis_pieces(plain, selection).into_iter().enumerate() {
        let stack = u16::try_from(rank).unwrap_or(u16::MAX);
        let key = UploadKey(piece.key);
        let (cell, row) = anchor(piece.column, piece.kind);
        if held.images.holds(key)
            && let Some(&crop) = held.pieces.get(&key)
            && let (Ok(column), Ok(first_row)) = (
                u16::try_from(cell + crop.columns_from),
                usize::try_from(row + crop.rows_from),
            )
        {
            pieces.push(Piece {
                frame: None,
                column,
                columns: crop.columns,
                rows: first_row..first_row + crop.rows,
                key,
                stack,
            });
            continue;
        }
        let Some(top) = rasterise_piece(plain, selection, piece).map_err(CacheError::Raster)?
        else {
            continue;
        };
        let first_row = usize::from(top.first_row);
        held.pieces.insert(
            key,
            Crop {
                columns_from: i64::from(top.first_column) - cell,
                rows_from: i64::from(top.first_row) - row,
                columns: top.frame.columns,
                rows: top.frame.rows,
            },
        );
        pieces.push(Piece {
            column: top.first_column,
            columns: top.frame.columns,
            rows: first_row..first_row + top.frame.rows,
            frame: Some(top.frame),
            key,
            stack,
        });
    }
    Ok(((base, UploadKey(strip_key)), pieces))
}

/// A strip's image and its key, and its emphasis pieces.
pub(super) type Layers = ((Arc<RasterFrame>, UploadKey), Vec<Piece>);

/// One emphasis piece this frame places: its pixels when the terminal
/// lacks them, the strip cell column it starts on, its width in cells, the
/// strip rows it covers, its key, and its rank in the strip's stack.
#[derive(Clone)]
pub(super) struct Piece {
    frame: Option<Arc<RasterFrame>>,
    column: u16,
    columns: u32,
    rows: Range<usize>,
    key: UploadKey,
    stack: u16,
}

/// One strip this frame draws: its backdrop (or iTerm2's composited band),
/// the emphasis over it and the strip cell column it starts on, which of
/// its rows show, and the graph row the first of them lands on.
pub(super) struct Drawn {
    pub(super) first_row: usize,
    pub(super) frame: Arc<RasterFrame>,
    /// What `frame` is, for the upload cache.
    pub(super) key: UploadKey,
    pub(super) emphasis: Vec<Piece>,
    pub(super) rows: Range<usize>,
    pub(super) at: u16,
}

/// Where each drawn strip goes in `graph`: the strip across its own columns,
/// then each emphasis piece from the cell column it starts on.
pub(super) fn places(drawn: &[Drawn], graph: Rect) -> Vec<Place<'_>> {
    let mut places = Vec::with_capacity(2 * drawn.len());
    for strip in drawn {
        let cells = |column: u16, columns: u32| CellRect {
            column: graph.x + column,
            row: graph.y + strip.at,
            columns: placed_columns(graph.width.saturating_sub(column), columns),
            rows: u16::try_from(strip.rows.len()).unwrap_or(u16::MAX),
        };
        places.push(Place {
            first_row: strip.first_row,
            key: strip.key,
            frame: Some(&strip.frame),
            rows: strip.rows.clone(),
            cells: cells(0, strip.frame.columns),
            layer: Layer::Strip,
            stack: 0,
        });
        // Starting right of the graph column, none of it would show.
        for piece in strip
            .emphasis
            .iter()
            .filter(|piece| piece.column < graph.width)
        {
            // The strip rows on screen that the piece covers, counted from
            // the piece's own first row, and the screen row they land on.
            let shown = piece.rows.start.max(strip.rows.start)..piece.rows.end.min(strip.rows.end);
            if shown.is_empty() {
                continue;
            }
            let below = u16::try_from(shown.start - strip.rows.start).unwrap_or(u16::MAX);
            let mut rect = cells(piece.column, piece.columns);
            rect.row = rect.row.saturating_add(below);
            rect.rows = u16::try_from(shown.len()).unwrap_or(u16::MAX);
            places.push(Place {
                first_row: strip.first_row,
                key: piece.key,
                frame: piece.frame.as_deref(),
                rows: shown.start - piece.rows.start..shown.end - piece.rows.start,
                cells: rect,
                layer: Layer::Emphasis,
                stack: piece.stack,
            });
        }
    }
    places
}

/// Cells a strip is placed across: its own lane columns, never more than the
/// graph column has. Placing a narrower strip across the whole column would
/// make the terminal stretch it sideways.
pub(super) fn placed_columns(graph_width: u16, frame_columns: u32) -> u16 {
    u16::try_from(frame_columns).map_or(graph_width, |columns| columns.min(graph_width))
}

/// The strip rows that land in a `height`-row graph column whose first row
/// shows history row `top`, and the column row the first of them lands on.
/// `lowered`: the uncommitted row sits above this strip's rows on screen, in
/// the strip or as the pinned image: every strip row shows one line lower, and
/// the strip's own rows start at history row `top` (a row above it would land
/// on the pinned row's line).
pub(super) fn on_screen(
    first_row: usize,
    row_count: usize,
    lowered: bool,
    top: usize,
    height: u16,
) -> Option<(Range<usize>, u16)> {
    // Strip row `j` is drawn on column row `start + j - top`.
    let start = first_row + usize::from(lowered);
    let first = top.saturating_sub(first_row);
    let end = (top + usize::from(height))
        .saturating_sub(start)
        .min(row_count);
    let at = u16::try_from(start + first - top).ok()?;
    (first < end).then_some((first..end, at))
}

#[cfg(test)]
mod tests {
    use super::{on_screen, placed_columns};

    #[test]
    fn a_strip_is_placed_across_its_own_columns_not_stretched() {
        assert_eq!(placed_columns(20, 6), 6);
        assert_eq!(placed_columns(20, 20), 20);
        // Wider than the column: clamped (and squeezed; KNOWN_GAPS).
        assert_eq!(placed_columns(20, 30), 20);
    }

    #[test]
    fn a_strip_starting_above_the_viewport_is_cropped_to_it() {
        // Rows 50..70 of the strip 32..64: strip rows 18..32 on rows 0..14.
        assert_eq!(on_screen(32, 32, false, 50, 20), Some((18..32, 0)));
        // The next strip fills the rest: rows 64..70 on rows 14..20.
        assert_eq!(on_screen(64, 32, false, 50, 20), Some((0..6, 14)));
    }

    #[test]
    fn the_uncommitted_row_takes_the_first_screen_row() {
        // Top 40, uncommitted row inserted at strip index 8 of 32..64 (33 rows).
        assert_eq!(on_screen(32, 33, false, 40, 20), Some((8..28, 0)));
        // A later strip sits one row lower than its history rows say.
        assert_eq!(on_screen(64, 32, true, 50, 20), Some((0..5, 15)));
    }

    #[test]
    fn a_strip_holding_top_under_the_pinned_row_starts_at_top() {
        // Top 40 in strip 32..64, the pinned row an image of its own: history
        // row 40 (strip row 8) on line 1, the line under the pinned row, and
        // row 39 (strip row 7) not drawn on the pinned row's line.
        assert_eq!(on_screen(32, 32, true, 40, 20), Some((8..27, 1)));
        // At a strip boundary: the strip starts on its first row, line 1.
        assert_eq!(on_screen(64, 32, true, 64, 20), Some((0..19, 1)));
    }

    #[test]
    fn a_strip_off_screen_places_nothing() {
        assert_eq!(on_screen(96, 32, false, 50, 20), None);
        assert_eq!(on_screen(0, 32, false, 50, 20), None);
    }
}
