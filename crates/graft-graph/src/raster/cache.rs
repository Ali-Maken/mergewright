//! The strip cache: rasterised strips kept across frames, so a scroll frame
//! rasterises at most the one strip entering the viewport (DESIGN.md §3.8).
//!
//! Strips are [`STRIP_ROWS`] rows tall and aligned to multiples of it, so a
//! viewport scrolled by a row asks for the same strips as before. An entry is
//! keyed on exactly what makes its pixels: the strip's row range, a hash of
//! the layout it draws, the theme's colours and the cell size.
//!
//! Invalidation is mostly implicit. New content — a refresh after a commit,
//! a fold, a different selection — hashes differently, so the stale strip
//! stops being hit and ages out of the bounded LRU. Theme, cell-size,
//! re-detection and resize events [`flush`](StripCache::flush) everything at
//! once: those change every strip, and holding the old ones only wastes
//! memory until they age out. Scrolling never invalidates.
//!
//! Strips are layered (ADR-0053): [`backdrop`](StripCache::backdrop) strips
//! keyed on layout and one backdrop alpha — no selection, so moving the
//! cursor never misses one — and [`emphasis`](StripCache::emphasis) images
//! keyed on that plus [`lit_hash`]; [`composited`](StripCache::composited)
//! joins the two for a terminal that cannot layer images.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

use graft_core::capabilities::Rgb;
use graft_core::raster::RasterFrame;

use super::{
    Backdrop, Emphasised, PlainStrip, RasterError, Ring, Selection, composite, rasterise_backdrop,
    rasterise_emphasis,
};
use crate::row::Row;

/// Rows per strip. Measured at the user's 17×37 px cell with 12 lanes in a
/// release build on an Apple M5 (m1-the-graph/23, `strip_height_numbers`):
/// 16 rows rasterise in 0.80 ms (943 KiB), 32 in 1.36 ms (1.8 MiB), 64 in
/// 1.74 ms (3.7 MiB). 32 keeps a miss under a tenth of the 16 ms frame while
/// halving how often a scroll misses against 16; 64 saves little time but
/// doubles the bytes every miss must encode and send, and the rows a one-row
/// layout change redraws.
pub const STRIP_ROWS: usize = 32;

/// Why every strip was thrown away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flush {
    /// The theme's colours changed.
    Theme,
    /// The terminal's cell size changed.
    CellSize,
    /// Capabilities were detected again.
    Redetection,
    /// The terminal was resized, so strips may change width.
    Resize,
}

/// Why a strip could not come from the cache.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CacheError {
    /// The strip does not start on a multiple of the strip height, is empty,
    /// or is taller than one strip. A caller bug: strips come from
    /// [`StripCache::strips_for`].
    #[error(
        "a strip of {rows} rows starting at row {first_row} is not aligned to {strip_rows}-row strips"
    )]
    Misaligned {
        /// Row the strip starts at.
        first_row: usize,
        /// Rows the strip holds.
        rows: usize,
        /// The cache's strip height.
        strip_rows: usize,
    },
    /// Rasterising the strip on a miss failed.
    #[error(transparent)]
    Raster(#[from] RasterError),
}

/// What the cache has done since it was made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// Backdrop lookups answered from a resident strip.
    pub hits: u64,
    /// Backdrop lookups that rasterised a strip.
    pub rasterised: u64,
    /// Entries dropped, from any layer, to stay within capacity.
    pub evicted: u64,
    /// Wholesale flushes.
    pub flushes: u64,
    /// Emphasis lookups that drew the selection's paint.
    pub emphasis_rasterised: u64,
    /// Emphasis lookups answered from a resident image (or a resident "no
    /// image").
    pub emphasis_hits: u64,
}

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
struct LayerKey {
    strip: StripKey,
    backdrop: Backdrop,
}

/// An emphasis or composite's key: its backdrop's key and the selection's
/// [`lit_hash`].
type PaintKey = (LayerKey, u64);

/// A bounded map whose least recently used entry goes first. A linear scan
/// to evict: capacity is a few viewports of strips, a dozen or so, where a
/// scan beats keeping an ordered index up to date on every hit.
#[derive(Debug)]
struct Lru<K, V> {
    entries: HashMap<K, (V, u64)>,
}

impl<K: Copy + Eq + Hash, V: Clone> Lru<K, V> {
    fn new() -> Self {
        Lru {
            entries: HashMap::new(),
        }
    }

    /// The value under `key`, marked used at `now`.
    fn touch(&mut self, key: &K, now: u64) -> Option<V> {
        self.entries.get_mut(key).map(|(value, last_used)| {
            *last_used = now;
            value.clone()
        })
    }

    /// Insert at `now`, first evicting the least recently used entry when
    /// `capacity` are held. Whether one was evicted.
    fn insert(&mut self, key: K, value: V, now: u64, capacity: usize) -> bool {
        let evicted = self.entries.len() >= capacity && {
            let oldest = self
                .entries
                .iter()
                .min_by_key(|(_, (_, last_used))| *last_used)
                .map(|(key, _)| *key);
            oldest.is_some_and(|key| self.entries.remove(&key).is_some())
        };
        self.entries.insert(key, (value, now));
        evicted
    }

    fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// The bytes every value holds, by `size`.
    fn bytes(&self, size: impl Fn(&V) -> usize) -> usize {
        self.entries.values().map(|(value, _)| size(value)).sum()
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn clear(&mut self) {
        self.entries.clear();
    }
}

/// A bounded LRU of rasterised strips, private to the pixel renderer.
#[derive(Debug)]
pub struct StripCache {
    strip_rows: usize,
    capacity: usize,
    /// Layout-only strips at one backdrop alpha.
    backdrops: Lru<LayerKey, Arc<RasterFrame>>,
    /// Emphasis images; `None` where the selection paints nothing above the
    /// backdrop, so that answer is not recomputed either.
    emphases: Lru<PaintKey, Option<Arc<Emphasised>>>,
    /// Backdrop and emphasis composited, for iTerm2.
    composites: Lru<PaintKey, Arc<RasterFrame>>,
    clock: u64,
    stats: CacheStats,
}

impl StripCache {
    /// A cache of `strip_rows`-row strips holding at most `capacity` of them
    /// in each layer. Both are raised to at least 1.
    pub fn new(strip_rows: usize, capacity: usize) -> Self {
        StripCache {
            strip_rows: strip_rows.max(1),
            capacity: capacity.max(1),
            backdrops: Lru::new(),
            emphases: Lru::new(),
            composites: Lru::new(),
            clock: 0,
            stats: CacheStats::default(),
        }
    }

    /// Strips needed to keep a `viewport_rows` viewport and `buffer_rows`
    /// above and below it resident at any scroll offset.
    pub fn capacity_for(strip_rows: usize, viewport_rows: usize, buffer_rows: usize) -> usize {
        let strip_rows = strip_rows.max(1);
        // A window of `n` rows touches at most ceil(n / strip) + 1 aligned
        // strips when it straddles a boundary.
        (viewport_rows + 2 * buffer_rows).div_ceil(strip_rows) + 1
    }

    /// Rows per strip.
    pub fn strip_rows(&self) -> usize {
        self.strip_rows
    }

    /// The aligned strips covering `viewport`, top to bottom. The last may
    /// run past the history's end; the caller clips it to the rows it has.
    pub fn strips_for(&self, viewport: Range<usize>) -> impl Iterator<Item = Range<usize>> + use<> {
        let strip_rows = self.strip_rows;
        let first = viewport.start / strip_rows;
        let end = if viewport.is_empty() {
            first
        } else {
            viewport.end.div_ceil(strip_rows)
        };
        (first..end).map(move |index| index * strip_rows..(index + 1) * strip_rows)
    }

    /// The layout-only strip starting at `first_row`, drawn at `backdrop`'s
    /// alpha, from the cache or rasterised into it. `strip.rows` must start on
    /// a multiple of the strip height and hold between one row and a full
    /// strip (a short one ends the history). A [`Ring::Uncommitted`] row is
    /// not a history row, so it does not count toward the strip height: the
    /// strip holding `HEAD` carries the virtual row on top of its own full
    /// strip. Nothing about the selection is in its key, so a
    /// cursor move never misses it (ADR-0053).
    pub fn backdrop(
        &mut self,
        first_row: usize,
        strip: &PlainStrip<'_>,
        backdrop: Backdrop,
    ) -> Result<Arc<RasterFrame>, CacheError> {
        let key = self.layer_key(first_row, strip, backdrop)?;
        self.clock += 1;
        if let Some(frame) = self.backdrops.touch(&key, self.clock) {
            self.stats.hits += 1;
            return Ok(frame);
        }
        let frame = Arc::new(rasterise_backdrop(strip, backdrop)?);
        self.stats.rasterised += 1;
        let evicted = self
            .backdrops
            .insert(key, Arc::clone(&frame), self.clock, self.capacity);
        self.stats.evicted += u64::from(evicted);
        Ok(frame)
    }

    /// The selection's emphasis image over the strip at `first_row`, from
    /// the cache or rasterised into it; `None` when it paints nothing above
    /// the backdrop (see [`rasterise_emphasis`]).
    pub fn emphasis(
        &mut self,
        first_row: usize,
        strip: &PlainStrip<'_>,
        sel: &Selection<'_>,
    ) -> Result<Option<Arc<Emphasised>>, CacheError> {
        let key = (
            self.layer_key(first_row, strip, sel.backdrop)?,
            lit_hash(sel),
        );
        self.clock += 1;
        if let Some(image) = self.emphases.touch(&key, self.clock) {
            self.stats.emphasis_hits += 1;
            return Ok(image);
        }
        let image = rasterise_emphasis(strip, sel)?.map(Arc::new);
        self.stats.emphasis_rasterised += 1;
        let evicted = self
            .emphases
            .insert(key, image.clone(), self.clock, self.capacity);
        self.stats.evicted += u64::from(evicted);
        Ok(image)
    }

    /// iTerm2: one image per strip, the backdrop with the emphasis drawn
    /// over it, cached on (backdrop key, [`lit_hash`]). With no emphasis it
    /// is the backdrop strip itself. Counted through the backdrop and
    /// emphasis lookups a miss makes.
    pub fn composited(
        &mut self,
        first_row: usize,
        strip: &PlainStrip<'_>,
        sel: &Selection<'_>,
    ) -> Result<Arc<RasterFrame>, CacheError> {
        let key = (
            self.layer_key(first_row, strip, sel.backdrop)?,
            lit_hash(sel),
        );
        self.clock += 1;
        if let Some(frame) = self.composites.touch(&key, self.clock) {
            return Ok(frame);
        }
        let base = self.backdrop(first_row, strip, sel.backdrop)?;
        let Some(top) = self.emphasis(first_row, strip, sel)? else {
            return Ok(base);
        };
        let frame = Arc::new(composite(&base, &top));
        let evicted = self
            .composites
            .insert(key, Arc::clone(&frame), self.clock, self.capacity);
        self.stats.evicted += u64::from(evicted);
        Ok(frame)
    }

    /// Is this backdrop strip already rasterised? A look, not a use: it
    /// moves neither the counters nor the strip's place in the eviction
    /// order. For idle prefetch, which draws backdrops
    /// only (emphasis waits for the cursor).
    pub fn contains_backdrop(
        &self,
        first_row: usize,
        strip: &PlainStrip<'_>,
        backdrop: Backdrop,
    ) -> bool {
        self.layer_key(first_row, strip, backdrop)
            .is_ok_and(|key| self.backdrops.contains(&key))
    }

    /// The key a layout-only strip is cached under at `backdrop`.
    fn layer_key(
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

    /// Throw every strip and layer away.
    pub fn flush(&mut self, why: Flush) {
        // Every reason flushes the same way; the reason names the event at
        // the call site and in a debugger.
        match why {
            Flush::Theme | Flush::CellSize | Flush::Redetection | Flush::Resize => {}
        }
        self.backdrops.clear();
        self.emphases.clear();
        self.composites.clear();
        self.stats.flushes += 1;
    }

    /// Backdrop strips resident.
    pub fn len(&self) -> usize {
        self.backdrops.len()
    }

    /// Whether nothing is resident in any layer.
    pub fn is_empty(&self) -> bool {
        self.len() == 0 && self.emphases.len() == 0 && self.composites.len() == 0
    }

    /// Pixel bytes held by every layer: backdrop strips, emphasis images and composited strips.
    pub fn resident_bytes(&self) -> usize {
        let frame = |frame: &Arc<RasterFrame>| frame.pixels.len();
        let image = |image: &Option<Arc<Emphasised>>| {
            image.as_ref().map_or(0, |image| image.frame.pixels.len())
        };
        self.backdrops.bytes(frame) + self.emphases.bytes(image) + self.composites.bytes(frame)
    }

    /// Counters since the cache was made.
    pub fn stats(&self) -> CacheStats {
        self.stats
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
