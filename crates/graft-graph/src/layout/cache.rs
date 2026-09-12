//! The chunked row cache: DESIGN.md §3.6's resident set.
//!
//! Rows live in fixed-size chunks keyed by chunk index. Chunks outside the
//! window the streaming layout protects are evicted oldest-touched first once
//! the cache is past its budget, and never before — an evicted chunk is a
//! rebuild away, so eviction is purely a memory decision. Each eviction leaves
//! a fingerprint behind; a rebuild that does not match it is reported, never
//! displayed (§3.7's determinism, checked at runtime rather than assumed).

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

use crate::row::Row;

/// Rows in chunks, with what is needed to evict and re-admit them.
#[derive(Debug)]
pub(crate) struct RowCache {
    chunk_rows: usize,
    budget: usize,
    chunks: HashMap<usize, Resident>,
    evicted: HashMap<usize, Fingerprint>,
    clock: u64,
}

/// One chunk that is currently held.
#[derive(Debug)]
struct Resident {
    rows: Vec<Row>,
    last_used: u64,
}

/// What an evicted chunk looked like, enough to recognise it again. The
/// length is kept because a chunk can be evicted while still growing at the
/// frontier; a rebuild is checked against the rows the fingerprint covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fingerprint {
    len: usize,
    hash: u64,
}

/// A rebuilt chunk that did not reproduce the rows evicted earlier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Differs {
    /// The chunk's first row.
    pub(crate) start: usize,
    /// One past the last row the evicted fingerprint covered.
    pub(crate) end: usize,
}

impl RowCache {
    /// An empty cache of `chunk_rows`-row chunks that keeps at most `budget`
    /// chunks resident outside the protected window.
    pub(crate) fn new(chunk_rows: usize, budget: usize) -> Self {
        Self {
            chunk_rows,
            budget,
            chunks: HashMap::new(),
            evicted: HashMap::new(),
            clock: 0,
        }
    }

    /// Whether `chunk` is held right now.
    pub(crate) fn contains(&self, chunk: usize) -> bool {
        self.chunks.contains_key(&chunk)
    }

    /// The rows of `chunk`, if held; touching it for the eviction order.
    pub(crate) fn get(&mut self, chunk: usize) -> Option<&[Row]> {
        self.clock += 1;
        let clock = self.clock;
        self.chunks.get_mut(&chunk).map(|resident| {
            resident.last_used = clock;
            resident.rows.as_slice()
        })
    }

    /// Adds rows freshly laid out at the frontier, starting at `start_row`.
    ///
    /// They join the chunk they belong to if it is held and ends exactly
    /// where they start; they open a new chunk if they start one. Rows that
    /// would extend a chunk already evicted are dropped: a chunk is either
    /// held whole (as far as the frontier has reached) or not at all, and
    /// that one is a rebuild away like any other evicted chunk.
    pub(crate) fn push_frontier(&mut self, start_row: usize, rows: Vec<Row>) {
        if rows.is_empty() {
            return;
        }
        let chunk = start_row / self.chunk_rows;
        let offset = start_row % self.chunk_rows;
        self.clock += 1;
        match self.chunks.get_mut(&chunk) {
            Some(resident) if resident.rows.len() == offset => {
                resident.rows.extend(rows);
                resident.last_used = self.clock;
            }
            None if offset == 0 => {
                self.chunks.insert(
                    chunk,
                    Resident {
                        rows,
                        last_used: self.clock,
                    },
                );
            }
            // Held at some other length, or evicted while still growing:
            // the rows are dropped and the chunk rebuilt on demand.
            Some(_) | None => {}
        }
    }

    /// Admits a rebuilt chunk, first checking it against the fingerprint of
    /// the copy that was evicted (if there was one). A chunk already held is
    /// left as it is.
    pub(crate) fn admit(&mut self, chunk: usize, rows: Vec<Row>) -> Result<(), Differs> {
        if self.chunks.contains_key(&chunk) {
            return Ok(());
        }
        if let Some(expected) = self.evicted.get(&chunk) {
            let covered = rows.get(..expected.len);
            if covered.map(fingerprint) != Some(expected.hash) {
                return Err(Differs {
                    start: chunk * self.chunk_rows,
                    end: chunk * self.chunk_rows + expected.len,
                });
            }
        }
        self.clock += 1;
        self.chunks.insert(
            chunk,
            Resident {
                rows,
                last_used: self.clock,
            },
        );
        Ok(())
    }

    /// Evicts chunks outside `keep` (a range of chunk indices), oldest
    /// touched first, until no more than the budget remain.
    pub(crate) fn evict_outside(&mut self, keep: Range<usize>) {
        while self.chunks.len() > self.budget {
            let Some(victim) = self
                .chunks
                .iter()
                .filter(|(index, _)| !keep.contains(index))
                .min_by_key(|(index, resident)| (resident.last_used, **index))
                .map(|(index, _)| *index)
            else {
                return;
            };
            if let Some(resident) = self.chunks.remove(&victim) {
                self.evicted.insert(
                    victim,
                    Fingerprint {
                        len: resident.rows.len(),
                        hash: fingerprint(&resident.rows),
                    },
                );
            }
        }
    }

    /// How many chunks are held.
    pub(crate) fn resident_chunks(&self) -> usize {
        self.chunks.len()
    }

    /// How many rows are held, across every chunk.
    pub(crate) fn resident_rows(&self) -> usize {
        self.chunks
            .values()
            .map(|resident| resident.rows.len())
            .sum()
    }
}

/// A hash of `rows` that two identical rebuilds share and two runs of the
/// process share too: [`DefaultHasher::new`] uses fixed keys.
fn fingerprint(rows: &[Row]) -> u64 {
    let mut hasher = DefaultHasher::new();
    rows.hash(&mut hasher);
    hasher.finish()
}
