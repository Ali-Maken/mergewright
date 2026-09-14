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

use crate::row::{Row, RowShape};

/// Rows in chunks, with what is needed to evict and re-admit them.
#[derive(Debug)]
pub(crate) struct RowCache {
    chunk_rows: usize,
    budget: usize,
    chunks: HashMap<usize, Held>,
    /// One entry per chunk laid out, held or not.
    laid_out: HashMap<usize, LaidOut>,
    /// The running fingerprint of the chunk still growing at the frontier.
    growing: Option<(usize, Running)>,
    clock: u64,
}

/// One chunk that is currently held.
#[derive(Debug)]
struct Held {
    rows: Vec<Row>,
    last_used: u64,
}

/// What is known about a chunk laid out: the fingerprint of its shapes, and
/// how many of its rows were ever materialised.
#[derive(Clone, Copy, Debug)]
struct LaidOut {
    print: Fingerprint,
    materialised: usize,
}

/// What a chunk's shapes looked like, enough to recognise them again. The
/// length is kept because a chunk can be evicted while still growing at the
/// frontier; a rebuild is checked against the rows the fingerprint covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fingerprint {
    /// Rows covered.
    pub(crate) len: usize,
    /// Hash of those rows' shapes, in order.
    pub(crate) hash: u64,
}

/// A fingerprint built a batch at a time: adding `a` then `b` gives the
/// fingerprint of `a` followed by `b`.
#[derive(Clone, Debug, Default)]
struct Running {
    len: usize,
    hasher: DefaultHasher,
}

impl Running {
    fn add(&mut self, shapes: &[RowShape]) {
        self.len += shapes.len();
        for shape in shapes {
            shape.hash(&mut self.hasher);
        }
    }

    fn finish(&self) -> Fingerprint {
        Fingerprint {
            len: self.len,
            hash: self.hasher.finish(),
        }
    }
}

/// The fingerprint of `shapes`: row count and a hash of each shape's lane,
/// edges and lane identities. Lane identities hash their handles, so two
/// identical layouts — a pass-through and its rebuild, or two runs of the
/// process ([`DefaultHasher::new`] uses fixed keys) — share it.
pub(crate) fn fingerprint(shapes: &[RowShape]) -> Fingerprint {
    let mut running = Running::default();
    running.add(shapes);
    running.finish()
}

/// A rebuilt chunk that did not reproduce the shapes laid out earlier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Differs {
    /// The chunk's first row.
    pub(crate) start: usize,
    /// One past the last row the earlier fingerprint covered.
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
            laid_out: HashMap::new(),
            growing: None,
            clock: 0,
        }
    }

    /// Whether `chunk` is held right now.
    pub(crate) fn contains(&self, chunk: usize) -> bool {
        self.chunks.contains_key(&chunk)
    }

    /// How many rows of `chunk` are held, if it is.
    pub(crate) fn held_len(&self, chunk: usize) -> Option<usize> {
        self.chunks.get(&chunk).map(|held| held.rows.len())
    }

    /// The rows of `chunk`, if held; touching it for the eviction order.
    pub(crate) fn get(&mut self, chunk: usize) -> Option<&[Row]> {
        self.clock += 1;
        let clock = self.clock;
        self.chunks.get_mut(&chunk).map(|held| {
            held.last_used = clock;
            held.rows.as_slice()
        })
    }

    /// Notes shapes freshly laid out at the frontier, starting at
    /// `start_row`, into their chunk's fingerprint — whether or not their
    /// rows are materialised. Shapes that start a chunk start its
    /// fingerprint; the rest extend the chunk growing at the frontier.
    ///
    /// # Panics
    ///
    /// If the shapes do not start a chunk and do not continue the one
    /// growing: the frontier only ever moves forward, one batch at a time.
    pub(crate) fn note(&mut self, start_row: usize, shapes: &[RowShape]) {
        if shapes.is_empty() {
            return;
        }
        let chunk = start_row / self.chunk_rows;
        let offset = start_row % self.chunk_rows;
        if offset == 0 {
            self.growing = Some((chunk, Running::default()));
        }
        let Some((growing, running)) = self.growing.as_mut() else {
            unreachable!("rows {start_row}.. continue no chunk growing at the frontier");
        };
        assert!(
            *growing == chunk && running.len == offset,
            "rows {start_row}.. do not continue chunk {growing} at {}",
            running.len
        );
        running.add(shapes);
        let print = running.finish();
        self.laid_out
            .entry(chunk)
            .and_modify(|known| known.print = print)
            .or_insert(LaidOut {
                print,
                materialised: 0,
            });
    }

    /// Adds rows freshly materialised at the frontier, starting at
    /// `start_row`; their shapes must already be [noted](Self::note).
    ///
    /// They join the chunk they belong to if it is held and ends exactly
    /// where they start; they open a new chunk if they start one. Rows that
    /// would extend a chunk not held are dropped: a chunk is either held
    /// whole (as far as the frontier has reached) or not at all, and that one
    /// is a rebuild away like any other chunk not held.
    pub(crate) fn push_frontier(&mut self, start_row: usize, rows: Vec<Row>) {
        if rows.is_empty() {
            return;
        }
        let chunk = start_row / self.chunk_rows;
        let offset = start_row % self.chunk_rows;
        self.clock += 1;
        match self.chunks.get_mut(&chunk) {
            Some(held) if held.rows.len() == offset => {
                held.rows.extend(rows);
                held.last_used = self.clock;
            }
            None if offset == 0 => {
                self.chunks.insert(
                    chunk,
                    Held {
                        rows,
                        last_used: self.clock,
                    },
                );
            }
            // Held at some other length, or not held while still growing:
            // the rows are dropped and the chunk rebuilt on demand.
            Some(_) | None => return,
        }
        self.mark_materialised(chunk);
    }

    /// Checks shapes laid out again for `chunk` against the fingerprint left
    /// when it was first laid out, over the rows that fingerprint covers. A
    /// chunk with no fingerprint passes.
    pub(crate) fn check(&self, chunk: usize, shapes: &[RowShape]) -> Result<(), Differs> {
        let Some(known) = self.laid_out.get(&chunk) else {
            return Ok(());
        };
        let expected = known.print;
        let covered = shapes.get(..expected.len);
        if covered.map(fingerprint) == Some(expected) {
            Ok(())
        } else {
            Err(Differs {
                start: chunk * self.chunk_rows,
                end: chunk * self.chunk_rows + expected.len,
            })
        }
    }

    /// Admits rebuilt rows for `chunk`, already [checked](Self::check). A
    /// chunk already held is left as it is.
    pub(crate) fn admit(&mut self, chunk: usize, rows: Vec<Row>) {
        if self.chunks.contains_key(&chunk) {
            return;
        }
        self.clock += 1;
        self.chunks.insert(
            chunk,
            Held {
                rows,
                last_used: self.clock,
            },
        );
        self.mark_materialised(chunk);
    }

    fn mark_materialised(&mut self, chunk: usize) {
        let held = self.chunks.get(&chunk).map_or(0, |held| held.rows.len());
        if let Some(known) = self.laid_out.get_mut(&chunk) {
            known.materialised = known.materialised.max(held);
        }
    }

    /// Evicts chunks outside `keep` (a range of chunk indices), oldest
    /// touched first, until no more than the budget remain. The fingerprint
    /// stays.
    pub(crate) fn evict_outside(&mut self, keep: Range<usize>) {
        while self.chunks.len() > self.budget {
            let Some(victim) = self
                .chunks
                .iter()
                .filter(|(index, _)| !keep.contains(index))
                .min_by_key(|(index, held)| (held.last_used, **index))
                .map(|(index, _)| *index)
            else {
                return;
            };
            self.chunks.remove(&victim);
        }
    }

    /// Forgets every row from `row` on: chunks starting at or past it are
    /// dropped, the chunk holding it is cut back to the rows before it, and
    /// the fingerprints of every chunk from that one on are forgotten (a
    /// fingerprint of the longer chunk cannot check the shorter rebuild; the
    /// caller notes the rows before `row` again). Returns the chunk indices
    /// touched, ascending — the invalidated set.
    pub(crate) fn truncate(&mut self, row: usize) -> Vec<usize> {
        let first = row / self.chunk_rows;
        let offset = row % self.chunk_rows;
        let mut touched: Vec<usize> = self
            .chunks
            .keys()
            .chain(self.laid_out.keys())
            .copied()
            .filter(|&chunk| chunk >= first)
            .collect();
        touched.sort_unstable();
        touched.dedup();
        for &chunk in &touched {
            self.laid_out.remove(&chunk);
            if chunk == first && offset > 0 {
                if let Some(held) = self.chunks.get_mut(&chunk) {
                    held.rows.truncate(offset);
                }
            } else {
                self.chunks.remove(&chunk);
            }
        }
        self.growing = None;
        touched
    }

    /// Restores `chunk`'s record after a [truncate](Self::truncate) cut it
    /// back: `shapes` are its rows before the cut, laid out again. The chunk
    /// resumes growing from there.
    pub(crate) fn renote(&mut self, chunk: usize, shapes: &[RowShape]) {
        self.growing = None;
        self.note(chunk * self.chunk_rows, shapes);
        let held = self.chunks.get(&chunk).map_or(0, |held| held.rows.len());
        if let Some(known) = self.laid_out.get_mut(&chunk) {
            known.materialised = held;
        }
    }

    /// How many chunks are held.
    pub(crate) fn resident_chunks(&self) -> usize {
        self.chunks.len()
    }

    /// How many rows are held, across every chunk.
    pub(crate) fn resident_rows(&self) -> usize {
        self.chunks.values().map(|held| held.rows.len()).sum()
    }

    /// How many rows were laid out and never materialised.
    pub(crate) fn passed_through(&self) -> usize {
        self.laid_out
            .values()
            .map(|known| known.print.len - known.materialised.min(known.print.len))
            .sum()
    }
}
