//! What a strip is cached under: keys built from exactly what makes its
//! pixels, and the hashes that go into them.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use mergewright_core::capabilities::Rgb;

use super::{CacheError, StripCache};
use crate::raster::{Backdrop, PlainStrip, Ring, Selection};
use crate::row::Row;

/// Exactly what a strip's pixels depend on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct StripKey {
    first_row: usize,
    rows: usize,
    layout: u64,
    theme: u64,
    cell_width: u32,
    cell_height: u32,
}

/// A backdrop strip's key: its layout-only strip key and the one alpha it
/// is drawn at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct LayerKey {
    strip: StripKey,
    backdrop: Backdrop,
}

/// An emphasis or composite's key: its backdrop's key and the selection's
/// [`lit_hash`].
pub(super) type PaintKey = (LayerKey, u64);

impl StripCache {
    /// What the backdrop strip at `first_row` is, as one number: the key
    /// it is cached under. Equal keys are equal pixels, so an upload of the
    /// strip is known by it (m8 04), and a strip evicted and drawn again is
    /// still the image the terminal holds.
    pub fn backdrop_key(
        &self,
        first_row: usize,
        strip: &PlainStrip<'_>,
        backdrop: Backdrop,
    ) -> Result<u64, CacheError> {
        let key = self.layer_key(first_row, strip, backdrop)?;
        Ok(one_number(&key))
    }

    /// The same for the selection's paint over that strip: its emphasis
    /// image, or iTerm2's composited band.
    pub fn paint_key(
        &self,
        first_row: usize,
        strip: &PlainStrip<'_>,
        sel: &Selection<'_>,
    ) -> Result<u64, CacheError> {
        let key: PaintKey = (
            self.layer_key(first_row, strip, sel.backdrop)?,
            lit_hash(sel),
        );
        Ok(one_number(&key))
    }

    /// The key a layout-only strip is cached under at `backdrop`.
    pub(super) fn layer_key(
        &self,
        first_row: usize,
        strip: &PlainStrip<'_>,
        backdrop: Backdrop,
    ) -> Result<LayerKey, CacheError> {
        let history_rows = strip
            .rows
            .iter()
            .filter(|row| row.ring != Ring::Uncommitted)
            .count();
        self.aligned(first_row, strip.rows.len(), history_rows)?;
        Ok(LayerKey {
            strip: StripKey {
                first_row,
                rows: strip.rows.len(),
                layout: plain_layout_hash(strip),
                theme: theme_id(strip.palette, strip.muted),
                cell_width: strip.cell_width,
                cell_height: strip.cell_height,
            },
            backdrop,
        })
    }

    /// `Ok` when a strip of `rows` rows (`history_rows` of them history)
    /// starting at `first_row` is one aligned strip.
    fn aligned(
        &self,
        first_row: usize,
        rows: usize,
        history_rows: usize,
    ) -> Result<(), CacheError> {
        if !first_row.is_multiple_of(self.strip_rows) || rows == 0 || history_rows > self.strip_rows
        {
            return Err(CacheError::Misaligned {
                first_row,
                rows,
                strip_rows: self.strip_rows,
            });
        }
        Ok(())
    }
}

/// A hash of everything in a layout-only strip the rasteriser draws from,
/// colours and cell size aside: per row its commit id, columns, edges, lane
/// identities, Mistake Radar marker, ring and stubs; the edges of the row
/// above that reach into the strip; the strip's width and rail column.
/// Nothing else — a commit message the rasteriser never reads does not
/// change it, and no selection can reach the backdrop's key.
///
/// Layout is deterministic (DESIGN.md §3.7), so an evicted and rebuilt row
/// chunk hashes the same and costs no strip-cache hit. Rust's default hasher
/// with its fixed keys: stable within a process, which is all an in-memory
/// cache needs.
pub fn plain_layout_hash(strip: &PlainStrip<'_>) -> u64 {
    let mut hasher = DefaultHasher::new();
    strip.lane_columns.hash(&mut hasher);
    strip.marker.hash(&mut hasher);
    strip.rows.len().hash(&mut hasher);
    for plain in strip.rows {
        let row = plain.row;
        row.commit.id.hash(&mut hasher);
        hash_edges(row, &mut hasher);
        row.marker.hash(&mut hasher);
        plain.ring.hash(&mut hasher);
        plain.stubs.hash(&mut hasher);
    }
    hash_above(strip.above, &mut hasher);
    hasher.finish()
}

/// Hash of the selection's paint over a strip: its backdrop and, per row,
/// what the paint draws from — the thread's lit columns when it has one,
/// else the row's own emphasis — and whether it has one (`Some(∅)` and
/// `None` paint differently). Under a thread a row's own emphasis is
/// never read, so it is not hashed: toggling it is not a
/// new selection.
pub fn lit_hash(sel: &Selection<'_>) -> u64 {
    let mut hasher = DefaultHasher::new();
    sel.backdrop.hash(&mut hasher);
    sel.rows.len().hash(&mut hasher);
    for lit in sel.rows {
        match lit.thread {
            Some(thread) => thread.hash(&mut hasher),
            None => lit.emphasis.hash(&mut hasher),
        }
        lit.thread.is_some().hash(&mut hasher);
    }
    hasher.finish()
}

/// A cache key hashed to one number, for an upload to be known by.
fn one_number(key: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// The edges of the row above a strip that reach into it.
fn hash_above(above: Option<&Row>, hasher: &mut DefaultHasher) {
    match above {
        Some(above) => {
            true.hash(hasher);
            hash_edges(above, hasher);
        }
        None => false.hash(hasher),
    }
}

/// A row's placement: its column, the lanes through it and its edges.
fn hash_edges(row: &Row, hasher: &mut DefaultHasher) {
    row.lane.hash(hasher);
    row.incoming.hash(hasher);
    for (column, _) in &row.outgoing {
        column.hash(hasher);
    }
    row.outgoing.len().hash(hasher);
    row.lanes.hash(hasher);
}

/// A theme's identity as the rasteriser sees it: its lane palette and muted
/// colour.
fn theme_id(palette: &[Rgb], muted: Rgb) -> u64 {
    let mut hasher = DefaultHasher::new();
    palette.hash(&mut hasher);
    muted.hash(&mut hasher);
    hasher.finish()
}
