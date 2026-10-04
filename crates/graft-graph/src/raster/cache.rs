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
//! The cache is bounded by bytes, not by a count ([`ByteBudget`]): a strip
//! weighs lanes × cell size, which is 1 MB on one repo and 8 MB on another.
//! One budget covers all three layers, and the least recently used entry of
//! any layer goes first. Under it sits a floor, the strips the viewport needs
//! ([`StripCache::keep_at_least`]): a cache that cannot hold the viewport
//! rasterises and resends every strip every frame, so the floor wins over the
//! budget and [`StripCache::minimum_wins`] says when it did.
//!
//! Strips are layered (ADR-0053): [`backdrop`](StripCache::backdrop) strips
//! keyed on layout and one backdrop alpha — no selection, so moving the
//! cursor never misses one — and [`emphasis`](StripCache::emphasis) images
//! keyed on that plus [`lit_hash`]; [`composited`](StripCache::composited)
//! joins the two for a terminal that cannot layer images.

mod key;
mod lru;

use std::ops::Range;
use std::sync::Arc;

use graft_core::raster::RasterFrame;

use key::{LayerKey, PaintKey};
pub use key::{lit_hash, plain_layout_hash};
use lru::Lru;

use super::emphasis::{composite_in, rasterise_backdrop_in, rasterise_emphasis_in};
use super::{Backdrop, Emphasised, PlainStrip, RasterError, RasterPool, Selection};

/// Rows per strip. Measured at the user's 17×37 px cell with 12 lanes in a
/// release build on an Apple M5 (m1-the-graph/23, `strip_height_numbers`):
/// 16 rows rasterise in 0.80 ms (943 KiB), 32 in 1.36 ms (1.8 MiB), 64 in
/// 1.74 ms (3.7 MiB). 32 keeps a miss under a tenth of the 16 ms frame while
/// halving how often a scroll misses against 16; 64 saves little time but
/// doubles the bytes every miss must encode and send, and the rows a one-row
/// layout change redraws.
pub const STRIP_ROWS: usize = 32;

/// The pixel bytes the strip cache may hold across its three layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteBudget(pub usize);

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
    /// Pixel buffers the cache's [`RasterPool`] has made. A full cache
    /// draws into evicted strips' buffers, so held keys stop raising it.
    pub buffers_allocated: u64,
}

/// Rasterised strips in three layers under one [`ByteBudget`], private to
/// the pixel renderer.
#[derive(Debug)]
pub struct StripCache {
    strip_rows: usize,
    budget: ByteBudget,
    /// Entries each layer keeps whatever the budget: the viewport's strips.
    min_strips: usize,
    /// The last eviction pass stopped at the floor while still over budget.
    minimum_wins: bool,
    /// Buffers strips are drawn in; evictions give them back.
    pool: RasterPool,
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
    /// A cache of [`STRIP_ROWS`]-row strips holding at most `budget` pixel
    /// bytes across its layers, but never fewer than `min_strips` entries in
    /// a layer (raised to at least 1): the viewport's strips, from
    /// [`capacity_for`](Self::capacity_for).
    pub fn with_budget(budget: ByteBudget, min_strips: usize) -> Self {
        StripCache {
            strip_rows: STRIP_ROWS,
            budget,
            min_strips: min_strips.max(1),
            minimum_wins: false,
            pool: RasterPool::new(),
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

    /// The viewport changed size: keep at least `min_strips` entries in each
    /// layer from now on (raised to at least 1).
    pub fn keep_at_least(&mut self, min_strips: usize) {
        self.min_strips = min_strips.max(1);
        self.shrink();
    }

    /// The floor [`keep_at_least`](Self::keep_at_least) last set.
    pub fn min_strips(&self) -> usize {
        self.min_strips
    }

    /// The byte budget the cache was made with.
    pub fn budget(&self) -> ByteBudget {
        self.budget
    }

    /// True when the floor held the cache over its budget the last time it
    /// had to make room: the viewport's strips weigh more than the budget.
    pub fn minimum_wins(&self) -> bool {
        self.minimum_wins
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
        let frame = Arc::new(rasterise_backdrop_in(strip, backdrop, &mut self.pool)?);
        self.stats.rasterised += 1;
        let weight = frame.pixels.capacity();
        self.backdrops
            .insert(key, Arc::clone(&frame), self.clock, weight);
        self.shrink();
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
        let image = rasterise_emphasis_in(strip, sel, &mut self.pool)?.map(Arc::new);
        self.stats.emphasis_rasterised += 1;
        let weight = image
            .as_ref()
            .map_or(0, |image| image.frame.pixels.capacity());
        self.emphases.insert(key, image.clone(), self.clock, weight);
        self.shrink();
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
        let frame = Arc::new(composite_in(&base, &top, &mut self.pool));
        let weight = frame.pixels.capacity();
        self.composites
            .insert(key, Arc::clone(&frame), self.clock, weight);
        self.shrink();
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

    /// Throw every strip and layer away.
    pub fn flush(&mut self, why: Flush) {
        // Every reason flushes the same way; the reason names the event at
        // the call site and in a debugger.
        match why {
            Flush::Theme | Flush::CellSize | Flush::Redetection | Flush::Resize => {}
        }
        // Not given to the pool: strips may be another size now.
        self.backdrops.clear(drop);
        self.emphases.clear(drop);
        self.composites.clear(drop);
        self.pool.flush();
        self.minimum_wins = false;
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

    /// Pixel bytes held by every layer: backdrop strips, emphasis images
    /// and composited strips. Their buffers' capacity, which is what they
    /// hold: a pooled buffer is made at the full strip size.
    pub fn resident_bytes(&self) -> usize {
        let frame = |frame: &Arc<RasterFrame>| frame.pixels.capacity();
        let image = |image: &Option<Arc<Emphasised>>| {
            image
                .as_ref()
                .map_or(0, |image| image.frame.pixels.capacity())
        };
        self.backdrops.bytes(frame) + self.emphases.bytes(image) + self.composites.bytes(frame)
    }

    /// Counters since the cache was made.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            buffers_allocated: self.pool.allocated(),
            ..self.stats
        }
    }
}
