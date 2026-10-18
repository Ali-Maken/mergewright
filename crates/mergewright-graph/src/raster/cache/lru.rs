//! The strip cache's eviction: an LRU map that weighs its entries, and the
//! pass that drops the least recently used entry of any layer until the
//! cache is within its byte budget, giving evicted buffers to the pool.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use mergewright_core::raster::RasterFrame;

use super::StripCache;
use crate::raster::{Emphasised, RasterPool};

/// The least an entry weighs, whatever its pixels: an emphasis lookup that
/// paints nothing caches `None`, which holds no pixels but must not make the
/// cache unbounded. A narrow strip (two lanes at an 8×16 cell) weighs 64 KiB.
const ENTRY_FLOOR: usize = 64 * 1024;

/// A map that knows each entry's weight and last use, so the cache can
/// drop its least recently used. A linear scan to find that one: a layer
/// holds a few viewports of strips, a dozen or so, where a scan beats
/// keeping an ordered index up to date on every hit.
#[derive(Debug)]
pub(super) struct Lru<K, V> {
    entries: HashMap<K, Entry<V>>,
    /// Sum of every entry's weight.
    weight: usize,
}

#[derive(Debug)]
struct Entry<V> {
    value: V,
    last_used: u64,
    weight: usize,
}

impl<K: Copy + Eq + Hash, V: Clone> Lru<K, V> {
    pub(super) fn new() -> Self {
        Lru {
            entries: HashMap::new(),
            weight: 0,
        }
    }

    /// The value under `key`, marked used at `now`.
    pub(super) fn touch(&mut self, key: &K, now: u64) -> Option<V> {
        self.entries.get_mut(key).map(|entry| {
            entry.last_used = now;
            entry.value.clone()
        })
    }

    /// Insert at `now`, weighing `weight` (never less than [`ENTRY_FLOOR`]).
    pub(super) fn insert(&mut self, key: K, value: V, now: u64, weight: usize) {
        let weight = weight.max(ENTRY_FLOOR);
        let entry = Entry {
            value,
            last_used: now,
            weight,
        };
        if let Some(old) = self.entries.insert(key, entry) {
            self.weight -= old.weight;
        }
        self.weight += weight;
    }

    /// When the least recently used entry was last used.
    fn oldest(&self) -> Option<u64> {
        self.entries.values().map(|entry| entry.last_used).min()
    }

    /// Drop the least recently used entry, and hand it back.
    fn evict_oldest(&mut self) -> Option<V> {
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(key, _)| *key);
        let entry = oldest.and_then(|key| self.entries.remove(&key))?;
        self.weight -= entry.weight;
        Some(entry.value)
    }

    pub(super) fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// The bytes every value holds, by `size`.
    pub(super) fn bytes(&self, size: impl Fn(&V) -> usize) -> usize {
        self.entries.values().map(|entry| size(&entry.value)).sum()
    }

    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Drop every entry, handing each value to `give`.
    pub(super) fn clear(&mut self, mut give: impl FnMut(V)) {
        for (_, entry) in self.entries.drain() {
            give(entry.value);
        }
        self.weight = 0;
    }
}

/// The layer an eviction takes from.
#[derive(Clone, Copy)]
enum Layer {
    Backdrops,
    Emphases,
    Composites,
}

impl StripCache {
    /// Evict least recently used entries, from any layer holding more than
    /// the floor, until the cache weighs no more than its budget.
    pub(super) fn shrink(&mut self) {
        self.minimum_wins = false;
        while self.weight() > self.budget.0 {
            let floor = self.min_strips;
            let candidates = [
                (
                    Layer::Backdrops,
                    self.backdrops.len(),
                    self.backdrops.oldest(),
                ),
                (Layer::Emphases, self.emphases.len(), self.emphases.oldest()),
                (
                    Layer::Composites,
                    self.composites.len(),
                    self.composites.oldest(),
                ),
            ];
            let victim = candidates
                .into_iter()
                .filter(|(_, len, _)| *len > floor)
                .filter_map(|(layer, _, oldest)| oldest.map(|oldest| (layer, oldest)))
                .min_by_key(|(_, oldest)| *oldest)
                .map(|(layer, _)| layer);
            let evicted = match victim {
                Some(Layer::Backdrops) => self
                    .backdrops
                    .evict_oldest()
                    .map(|f| give_frame(&mut self.pool, f)),
                Some(Layer::Emphases) => self
                    .emphases
                    .evict_oldest()
                    .map(|i| give_image(&mut self.pool, i)),
                Some(Layer::Composites) => self
                    .composites
                    .evict_oldest()
                    .map(|f| give_frame(&mut self.pool, f)),
                None => None,
            }
            .is_some();
            if !evicted {
                self.minimum_wins = true;
                return;
            }
            self.stats.evicted += 1;
        }
    }

    /// What every layer weighs, floors included.
    fn weight(&self) -> usize {
        self.backdrops.weight + self.emphases.weight + self.composites.weight
    }
}

/// An evicted strip's buffer back to `pool`, when nothing else holds the
/// strip. A frame still drawing it keeps it, and its buffer is freed when
/// that frame drops it: a lost reuse, not a leak.
fn give_frame(pool: &mut RasterPool, frame: Arc<RasterFrame>) {
    if let Ok(frame) = Arc::try_unwrap(frame) {
        pool.give(frame.pixels);
    }
}

/// [`give_frame`] for an evicted emphasis entry; "no image" holds no buffer.
fn give_image(pool: &mut RasterPool, image: Option<Arc<Emphasised>>) {
    if let Some(Ok(image)) = image.map(Arc::try_unwrap) {
        give_frame(pool, image.frame);
    }
}
