//! Room for the cursor row's second line among the strips (ease-pass/13,
//! M3): the strip holding the cursor row is placed twice from its one
//! raster — its rows up to the cursor where they were, the rows after it
//! one cell row lower — with the continuation band in the gap; strips
//! wholly below sit one row lower, strips above stay. A `j` inside one
//! strip moves placements only, never the strip's pixels.

use std::ops::Range;
use std::sync::Arc;

use mergewright_core::raster::RasterFrame;
use mergewright_graph::raster::{
    CELLS_PER_LANE, PlainStrip, RasterError, Selection, continuation_key, rasterise_continuation,
};
use mergewright_term::kitty::{CellRect, KittyImages, Layer, Place, UploadKey};
use ratatui::layout::Rect;

use crate::view::strips::TallRow;

/// Where one strip's shown rows go with the second line made room for:
/// each part's strip rows and the graph line the first lands on, and the
/// strip row whose second line falls in this strip, with that line.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Room {
    pub(super) parts: Vec<(Range<usize>, u16)>,
    pub(super) band: Option<(usize, u16)>,
}

/// `rows` of a strip shown from graph line `at`, as placed around `tall`.
pub(super) fn make_room(rows: Range<usize>, at: u16, tall: Option<TallRow>) -> Room {
    let whole = |at| Room {
        parts: vec![(rows.clone(), at)],
        band: None,
    };
    let Some(TallRow { line }) = tall else {
        return whole(at);
    };
    let shown = u16::try_from(rows.len()).unwrap_or(u16::MAX);
    if line < at {
        return whole(at.saturating_add(1));
    }
    if line >= at.saturating_add(shown) {
        return whole(at);
    }
    let cursor = rows.start + usize::from(line - at);
    let mut parts = vec![(rows.start..cursor + 1, at)];
    if cursor + 1 < rows.end {
        parts.push((cursor + 1..rows.end, line.saturating_add(2)));
    }
    Room {
        parts,
        band: Some((cursor, line.saturating_add(1))),
    }
}

/// The continuation band this frame places: its key, its pixels when the
/// terminal does not hold them, the strip it belongs to and its line.
pub(super) struct Band {
    first_row: usize,
    key: UploadKey,
    frame: Option<Arc<RasterFrame>>,
    at: u16,
    /// Cells wide: the strip's lane columns.
    columns: u32,
}

impl Band {
    /// Strip row `row`'s band at graph line `at`, or `None` when nothing
    /// passes through it. Kitty places a band it holds by key, with no
    /// raster; iTerm2 is handed the pixels every time (its bands are
    /// cached by key on its side, `ItermImages`).
    pub(super) fn of(
        plain: &PlainStrip<'_>,
        selection: &Selection<'_>,
        (first_row, row, at): (usize, usize, u16),
        held: Option<&KittyImages>,
    ) -> Result<Option<Band>, RasterError> {
        let Some(key) = continuation_key(plain, selection, row)?.map(UploadKey) else {
            return Ok(None);
        };
        let frame = if held.is_some_and(|images| images.holds(key)) {
            None
        } else {
            match rasterise_continuation(plain, selection, row)? {
                Some(frame) => Some(Arc::new(frame)),
                None => return Ok(None),
            }
        };
        let lanes = u32::try_from(plain.lane_columns).unwrap_or(u32::MAX);
        Ok(Some(Band {
            first_row,
            key,
            frame,
            at,
            columns: lanes.saturating_mul(CELLS_PER_LANE),
        }))
    }

    /// Where the band goes in `graph`: one row, across its own columns.
    /// Known by its key alone, like an emphasis piece (the strip layer is
    /// one upload per strip), and nothing lies under it.
    pub(super) fn place(&self, graph: Rect) -> Place<'_> {
        Place {
            first_row: self.first_row,
            key: self.key,
            frame: self.frame.as_deref(),
            rows: 0..1,
            cells: CellRect {
                column: graph.x,
                row: graph.y + self.at,
                columns: super::layers::placed_columns(graph.width, self.columns),
                rows: 1,
            },
            layer: Layer::Emphasis,
            stack: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Room, TallRow, make_room};

    #[test]
    fn no_tall_row_places_the_strip_whole() {
        let room = make_room(4..20, 3, None);
        assert_eq!(room.parts, [(4..20, 3)]);
        assert_eq!(room.band, None);
    }

    #[test]
    fn a_strip_above_the_cursor_stays_and_one_below_drops_a_row() {
        // Shown on lines 0..16; the cursor on line 20, in a later strip.
        assert_eq!(
            make_room(16..32, 0, Some(TallRow { line: 20 })).parts,
            [(16..32, 0)]
        );
        // Shown from line 21; the cursor on line 20, above it.
        assert_eq!(
            make_room(0..10, 21, Some(TallRow { line: 20 })).parts,
            [(0..10, 22)]
        );
    }

    #[test]
    fn the_cursor_strip_is_placed_twice_with_disjoint_rows_and_a_band_between() {
        // Strip rows 6..20 on lines 0..14; the cursor on line 4 (strip row 10).
        let room = make_room(6..20, 0, Some(TallRow { line: 4 }));
        assert_eq!(
            room,
            Room {
                parts: vec![(6..11, 0), (11..20, 6)],
                band: Some((10, 5)),
            }
        );
    }

    #[test]
    fn a_cursor_on_the_strip_s_last_shown_row_has_no_lower_part() {
        let room = make_room(0..8, 2, Some(TallRow { line: 9 }));
        assert_eq!(room.parts, [(0..8, 2)]);
        assert_eq!(room.band, Some((7, 10)));
    }
}
