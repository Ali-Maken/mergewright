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

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

use graft_core::capabilities::Rgb;
use graft_core::raster::RasterFrame;

use super::{RasterError, Ring, Strip, rasterise};
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
    /// Lookups answered from a resident strip.
    pub hits: u64,
    /// Lookups that rasterised a strip.
    pub rasterised: u64,
    /// Strips dropped to stay within capacity.
    pub evicted: u64,
    /// Wholesale flushes.
    pub flushes: u64,
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

#[derive(Debug)]
struct Entry {
    frame: Arc<RasterFrame>,
    last_used: u64,
}

/// A bounded LRU of rasterised strips, private to the pixel renderer.
#[derive(Debug)]
pub struct StripCache {
    strip_rows: usize,
    capacity: usize,
    entries: HashMap<StripKey, Entry>,
    clock: u64,
    stats: CacheStats,
}

impl StripCache {
    /// A cache of `strip_rows`-row strips holding at most `capacity` of them.
    /// Both are raised to at least 1.
    pub fn new(strip_rows: usize, capacity: usize) -> Self {
        StripCache {
            strip_rows: strip_rows.max(1),
            capacity: capacity.max(1),
            entries: HashMap::new(),
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

    /// The strip starting at `first_row`, from the cache or rasterised into
    /// it. `strip.rows` must start on a multiple of the strip height and hold
    /// between one row and a full strip (a short one ends the history).
    ///
    /// A [`Ring::Uncommitted`] row is not a history row, so it does not count
    /// toward the strip height: the strip holding `HEAD` carries the virtual
    /// row on top of its own full strip, and stays aligned on history rows.
    pub fn get(
        &mut self,
        first_row: usize,
        strip: &Strip<'_>,
    ) -> Result<Arc<RasterFrame>, CacheError> {
        let key = self.key(first_row, strip)?;
        self.clock += 1;
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = self.clock;
            self.stats.hits += 1;
            return Ok(Arc::clone(&entry.frame));
        }
        let frame = Arc::new(rasterise(strip)?);
        self.stats.rasterised += 1;
        if self.entries.len() >= self.capacity {
            self.evict_least_recently_used();
        }
        self.entries.insert(
            key,
            Entry {
                frame: Arc::clone(&frame),
                last_used: self.clock,
            },
        );
        Ok(frame)
    }

    /// Is this strip already rasterised? A look, not a use: it moves neither
    /// the counters nor the strip's place in the eviction order. A strip
    /// [`StripCache::get`] would refuse is never there.
    pub fn contains(&self, first_row: usize, strip: &Strip<'_>) -> bool {
        self.key(first_row, strip)
            .is_ok_and(|key| self.entries.contains_key(&key))
    }

    /// The key `strip` is cached under, or why it cannot be cached.
    fn key(&self, first_row: usize, strip: &Strip<'_>) -> Result<StripKey, CacheError> {
        let rows = strip.rows.len();
        let history_rows = strip
            .rows
            .iter()
            .filter(|row| row.ring != Ring::Uncommitted)
            .count();
        if !first_row.is_multiple_of(self.strip_rows) || rows == 0 || history_rows > self.strip_rows
        {
            return Err(CacheError::Misaligned {
                first_row,
                rows,
                strip_rows: self.strip_rows,
            });
        }
        Ok(StripKey {
            first_row,
            rows,
            layout: layout_hash(strip),
            theme: theme_id(strip.palette, strip.muted),
            cell_width: strip.cell_width,
            cell_height: strip.cell_height,
        })
    }

    /// Throw every strip away.
    pub fn flush(&mut self, why: Flush) {
        // Every reason flushes the same way; the reason names the event at
        // the call site and in a debugger.
        match why {
            Flush::Theme | Flush::CellSize | Flush::Redetection | Flush::Resize => {}
        }
        self.entries.clear();
        self.stats.flushes += 1;
    }

    /// Strips resident.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no strip is resident.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Pixel bytes held by resident strips.
    pub fn resident_bytes(&self) -> usize {
        self.entries
            .values()
            .map(|entry| entry.frame.pixels.len())
            .sum()
    }

    /// Counters since the cache was made.
    pub fn stats(&self) -> CacheStats {
        self.stats
    }

    /// A linear scan: capacity is a few viewports of strips, a dozen or so,
    /// where a scan beats keeping an ordered index up to date on every hit.
    fn evict_least_recently_used(&mut self) {
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(key, _)| *key);
        if let Some(key) = oldest {
            self.entries.remove(&key);
            self.stats.evicted += 1;
        }
    }
}

/// A hash of everything in a strip the rasteriser draws from, colours and
/// cell size aside: per row its commit id, columns, edges, lane identities,
/// Mistake Radar marker, ring, and emphasis (the thread's lit columns when
/// it has one, else the row's own); the edges of the row above that
/// reach into the strip; the strip's width and `+N` marker. Nothing else — a
/// commit message the rasteriser never reads does not change it.
///
/// Layout is deterministic (DESIGN.md §3.7), so an evicted and rebuilt row
/// chunk hashes the same and costs no strip-cache hit. Rust's default hasher
/// with its fixed keys: stable within a process, which is all an in-memory
/// cache needs.
pub fn layout_hash(strip: &Strip<'_>) -> u64 {
    let mut hasher = DefaultHasher::new();
    strip.lane_columns.hash(&mut hasher);
    strip.marker.hash(&mut hasher);
    strip.rows.len().hash(&mut hasher);
    for raster_row in strip.rows {
        let row = raster_row.row;
        row.commit.id.hash(&mut hasher);
        hash_edges(row, &mut hasher);
        row.marker.hash(&mut hasher);
        raster_row.ring.hash(&mut hasher);
        raster_row.stubs.hash(&mut hasher);
        // Under a thread the rasteriser draws per-column emphasis and never
        // reads the row's own, so hashing it would turn every cursor move
        // into a miss and a retransmit with identical pixels (30b).
        match raster_row.thread {
            Some(thread) => thread.hash(&mut hasher),
            None => raster_row.emphasis.hash(&mut hasher),
        }
        raster_row.thread.is_some().hash(&mut hasher);
    }
    match strip.above {
        Some(above) => {
            true.hash(&mut hasher);
            hash_edges(above, &mut hasher);
        }
        None => false.hash(&mut hasher),
    }
    hasher.finish()
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
