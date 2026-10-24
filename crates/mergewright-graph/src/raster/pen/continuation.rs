//! The cursor row's second line in pixels (ease-pass/13, mock G9): one cell
//! row under the row, every lane that leaves the row's band through its
//! bottom drawn straight through it, and the rail when anything folds.
//!
//! Its own small image, not part of any strip: the strip keeps its pixels,
//! and the Pixel tier places the rows after the cursor one cell row lower
//! with this band in the gap. Each lane draws at the alpha the row's own
//! paint gives its column, so the thread stays lit through the gap.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use mergewright_core::raster::RasterFrame;
use tiny_skia::{IntSize, Pixmap};

use super::super::{
    CELLS_PER_LANE, Edge, Ink, Inked, Pen, PlainStrip, RasterError, STRIP_LEAD_CELLS, Selection,
    edge_at,
};

/// Strip row `row`'s inked row, or `None` past the strip.
fn inked_at<'a>(
    strip: &'a PlainStrip<'a>,
    sel: &Selection<'a>,
    row: usize,
) -> Result<Option<Inked<'a>>, RasterError> {
    if sel.rows.len() != strip.rows.len() {
        return Err(RasterError::Selection {
            rows: strip.rows.len(),
            lit: sel.rows.len(),
        });
    }
    Ok(strip
        .rows
        .get(row)
        .zip(sel.rows.get(row))
        .map(|(plain, lit)| Inked {
            row: plain.row,
            ring: plain.ring,
            stubs: &plain.stubs,
            lit: *lit,
        }))
}

impl Pen<'_> {
    /// Lanes leaving row 0 of this pen through its band's bottom: the
    /// node's own lane when an edge goes on down it, every lane passing
    /// straight through and every lane opening from the node. The marker
    /// column draws no edge of its own (the rail is drawn apart).
    fn leaving(&self) -> Vec<(usize, Ink)> {
        let inked = &self.rows[0];
        let row = inked.row;
        let widest = row.lanes.len().max(row.lane + 1);
        (0..widest)
            .filter_map(|column| {
                let identity = if column == row.lane {
                    if !row.outgoing.iter().any(|&(out, _)| out == column) {
                        return None;
                    }
                    row.lanes.get(column).copied().flatten()
                } else {
                    match edge_at(row, self.look.marker, column)? {
                        (_, Edge::Arc { incoming: true }) => return None,
                        (identity, Edge::Straight | Edge::Arc { incoming: false }) => {
                            Some(identity)
                        }
                    }
                };
                Some((column, self.ink(identity, inked.lit.alpha_at(column))))
            })
            .collect()
    }

    /// The rail's ink on the band, when anything folds.
    fn rail_ink(&self) -> Option<Ink> {
        self.look
            .marker
            .map(|_| self.ink(None, self.rows[0].lit.rail_alpha()))
    }
}

/// Everything the band's pixels are made of, as one number: the upload key
/// a terminal holding it is asked by. `None` when nothing leaves the row
/// and nothing folds (no band), or `row` is past the strip.
///
/// # Errors
///
/// [`RasterError::Selection`] when `sel` does not have one entry per row.
pub fn continuation_key(
    strip: &PlainStrip<'_>,
    sel: &Selection<'_>,
    row: usize,
) -> Result<Option<u64>, RasterError> {
    let Some(inked) = inked_at(strip, sel, row)? else {
        return Ok(None);
    };
    let look = strip.look();
    let rows = [inked];
    let pen = Pen::new(&look, &rows, None, None, 0..0);
    let leaving = pen.leaving();
    let rail = pen.rail_ink();
    if leaving.is_empty() && rail.is_none() {
        return Ok(None);
    }
    let mut hasher = DefaultHasher::new();
    "continuation".hash(&mut hasher);
    (strip.cell_width, strip.cell_height, strip.lane_columns).hash(&mut hasher);
    for (column, ink) in leaving {
        hash_ink(column, ink, &mut hasher);
    }
    if let (Some(marker), Some(ink)) = (strip.marker, rail) {
        "rail".hash(&mut hasher);
        hash_ink(marker.column, ink, &mut hasher);
    }
    Ok(Some(hasher.finish()))
}

/// One drawn line's part of the key: its column and its ink, the alpha as
/// the byte drawn.
fn hash_ink(column: usize, ink: Ink, hasher: &mut DefaultHasher) {
    column.hash(hasher);
    (ink.rgb, ink.alpha, ink.width.to_bits(), ink.shape).hash(hasher);
}

/// The band under strip row `row`: `lane_columns` lanes wide, one cell row
/// tall, every lane leaving the row drawn straight down through it at its
/// own paint. `None` when nothing leaves the row and nothing folds, or `row`
/// is past the strip.
///
/// # Errors
///
/// [`RasterError::Selection`] as [`continuation_key`];
/// [`RasterError::EmptyPalette`]; [`RasterError::TooLarge`] for a band too
/// wide to draw.
pub fn rasterise_continuation(
    strip: &PlainStrip<'_>,
    sel: &Selection<'_>,
    row: usize,
) -> Result<Option<RasterFrame>, RasterError> {
    if strip.palette.is_empty() {
        return Err(RasterError::EmptyPalette);
    }
    let Some(inked) = inked_at(strip, sel, row)? else {
        return Ok(None);
    };
    let look = strip.look();
    let rows = [inked];
    let pen = Pen::new(&look, &rows, None, None, 0..0);
    let leaving = pen.leaving();
    if leaving.is_empty() && look.marker.is_none() {
        return Ok(None);
    }
    let too_large = || RasterError::TooLarge {
        rows: 1,
        lane_columns: strip.lane_columns,
        cell_width: strip.cell_width,
        cell_height: strip.cell_height,
    };
    let columns = u32::try_from(strip.lane_columns)
        .ok()
        .and_then(|lanes| lanes.checked_mul(CELLS_PER_LANE))
        .and_then(|cells| cells.checked_add(STRIP_LEAD_CELLS))
        .ok_or_else(too_large)?;
    let width = columns
        .checked_mul(strip.cell_width)
        .ok_or_else(too_large)?;
    let height = strip.cell_height;
    let size = IntSize::from_wh(width, height).ok_or_else(too_large)?;
    let mut pixmap = Pixmap::new(size.width(), size.height()).ok_or_else(too_large)?;
    let band = pen.band(0);
    for (column, ink) in leaving {
        pen.straight(&mut pixmap, pen.x(column), band.top, band.bottom, ink);
    }
    if let Some(marker) = look.marker {
        pen.rail(&mut pixmap, &rows[0], marker, band);
    }
    Ok(Some(RasterFrame {
        width,
        height,
        rows: 1,
        columns,
        pixels: pixmap.take(),
    }))
}
