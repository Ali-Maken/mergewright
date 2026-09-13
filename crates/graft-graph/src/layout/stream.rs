//! The streaming layout: DESIGN.md §3.6 as one type.
//!
//! A [`StreamingLayout`] pulls commits from a [`CommitSource`] in chunks,
//! lays them out with a [`LayoutCursor`], keeps the rows around the viewport
//! resident and evicts the rest, checkpoints the cursor every so many rows,
//! and rebuilds evicted rows from the nearest checkpoint when the viewport
//! comes back. Nothing in it blocks on the whole history: every call takes a
//! budget or works on a range, and the caller decides when to call again.
//!
//! Two ways to start. [`StreamingLayout::new`] streams a walk as it comes.
//! [`StreamingLayout::from_order`] takes a walk order already known in full
//! — the product of ADR-0008's background topological walk — and fetches
//! commits by id as it goes: the promoted view. From then on the two are
//! the same type doing the same work; the only trace of the promotion is
//! that the order runs ahead of the frontier.

use std::collections::HashMap;
use std::ops::Range;

use graft_core::commit::Commit;
use graft_core::oid::Oid;

use crate::layout::LayoutCursor;
use crate::layout::cache::{Differs, RowCache};
use crate::layout::checkpoint::Checkpoints;
use crate::layout::order::{EmittedOrder, RepeatedCommit};
use crate::layout::source::{CommitSource, Pull};
use crate::row::Row;
use crate::seeds::Seeds;

/// The knobs of DESIGN.md §3.6, with its starting values as defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamConfig {
    /// Rows per cache chunk: the unit of eviction, and the most one pull
    /// from the source asks for.
    pub chunk_rows: usize,
    /// Rows between checkpoints. A multiple of `chunk_rows`, so a checkpoint
    /// always falls on a chunk boundary.
    pub checkpoint_every: usize,
    /// How many checkpoints to keep before the least recently used goes.
    pub max_checkpoints: usize,
    /// How many chunks may stay resident outside the protected window.
    pub resident_chunks: usize,
    /// Rows above and below the viewport that stay resident with it.
    pub buffer_rows: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            chunk_rows: 500,
            checkpoint_every: 2_000,
            max_checkpoints: 64,
            resident_chunks: 16,
            buffer_rows: 1_000,
        }
    }
}

/// Why a streaming layout could not go on.
#[derive(Debug, thiserror::Error)]
pub enum StreamError<E: std::error::Error + 'static> {
    /// The source failed.
    #[error("reading history")]
    Source(#[source] E),
    /// The walk gave the same commit twice.
    #[error(transparent)]
    Repeated(#[from] RepeatedCommit),
    /// The source returned no commits without saying the walk was over or
    /// stopped — a pull that could be retried forever, so it is not.
    #[error("the walk returned nothing, but reported neither finished nor stopped")]
    Stalled,
    /// A re-read by id came back short.
    #[error("asked for {asked} commits by id, the source returned {got}")]
    ShortRead {
        /// How many ids were asked for.
        asked: usize,
        /// How many commits came back.
        got: usize,
    },
    /// A re-read by id came back with a different commit.
    #[error("asked for commit {asked} by id, the source returned {got}")]
    WrongCommit {
        /// The id asked for.
        asked: Oid,
        /// The id that came back in its place.
        got: Oid,
    },
    /// A rebuilt chunk did not reproduce the rows that were evicted.
    #[error("rows {start}..{end} did not rebuild identically to the copy that was evicted")]
    RebuildDiffers {
        /// The first row of the chunk.
        start: usize,
        /// One past the last row the evicted copy held.
        end: usize,
    },
}

/// What one [`StreamingLayout::truncate`] threw away: nothing outside it
/// was touched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Invalidated {
    /// The first row laid out again; every row before it is kept.
    pub from_row: usize,
    /// Chunk indices dropped or cut back (held or evicted), ascending.
    pub chunks: Vec<usize>,
    /// Rows of the checkpoints dropped, ascending.
    pub checkpoints: Vec<usize>,
}

/// What one [`StreamingLayout::advance`] call did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Advance {
    /// Rows laid out by this call.
    pub rows_emitted: usize,
    /// Whether the whole walk is now laid out.
    pub done: bool,
    /// Whether the call gave up early because `stop` asked it to.
    pub stopped: bool,
}

/// A place in the graph that survives the rows moving under it: a commit,
/// and how far down the viewport it sat (DESIGN.md §3.2, ADR-0008 — "never
/// by row index").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    /// The selected commit.
    pub commit: Oid,
    /// How many rows below the top of the viewport it was.
    pub offset: usize,
}

/// What the layout is holding right now — the numbers a memory budget is
/// checked against (PROMPT.md §7.1, ticket m1-16).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Resident {
    /// Rows held in the cache.
    pub rows: usize,
    /// Chunks held in the cache.
    pub chunks: usize,
    /// Checkpoints held.
    pub checkpoints: usize,
    /// Approximate bytes across every checkpoint.
    pub checkpoint_bytes: usize,
    /// Approximate bytes of the frontier cursor: what one more checkpoint
    /// would cost.
    pub frontier_bytes: usize,
    /// Approximate bytes of the emitted order (ids and their rows).
    pub order_bytes: usize,
}

/// How much work the layout has done, for tests and benchmarks that want to
/// know a rebuild replayed from a checkpoint rather than from the top.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    /// Commits pulled from the source's walk.
    pub commits_pulled: usize,
    /// Commits re-read from the source by id.
    pub commits_reread: usize,
    /// Rows laid out again while rebuilding evicted chunks.
    pub rows_replayed: usize,
    /// Evicted chunks rebuilt.
    pub chunks_rebuilt: usize,
}

/// A graph laid out a chunk at a time, with a bounded resident set.
#[derive(Debug)]
pub struct StreamingLayout<S> {
    source: S,
    config: StreamConfig,
    /// The cursor as it stood before the first row: what a rebuild with no
    /// earlier checkpoint replays from.
    base: LayoutCursor,
    /// The cursor at the furthest row laid out.
    frontier: LayoutCursor,
    order: EmittedOrder,
    cache: RowCache,
    checkpoints: Checkpoints,
    viewport: Range<usize>,
    /// The walk's length when it was known before the first row was laid
    /// out — on a layout built from a finished order.
    known_total: Option<usize>,
    done: bool,
    work: Work,
}

impl<S: CommitSource> StreamingLayout<S> {
    /// A layout that will stream `source`'s walk as it comes.
    ///
    /// # Panics
    ///
    /// If `config.chunk_rows` is zero or `config.checkpoint_every` is not a
    /// positive multiple of it.
    #[must_use]
    pub fn new(source: S, seeds: Seeds, config: StreamConfig) -> Self {
        Self::build(source, seeds, config, EmittedOrder::new(), None)
    }

    /// A layout over a walk order already known in full, fetching each
    /// commit by id from `source` as the frontier reaches it. `source`'s own
    /// walk is never pulled.
    ///
    /// # Errors
    ///
    /// If `ids` repeats a commit.
    ///
    /// # Panics
    ///
    /// As [`StreamingLayout::new`].
    pub fn from_order(
        source: S,
        ids: Vec<Oid>,
        seeds: Seeds,
        config: StreamConfig,
    ) -> Result<Self, StreamError<S::Error>> {
        let order = EmittedOrder::from_ids(ids)?;
        let total = order.len();
        Ok(Self::build(source, seeds, config, order, Some(total)))
    }

    fn build(
        source: S,
        seeds: Seeds,
        config: StreamConfig,
        order: EmittedOrder,
        known_total: Option<usize>,
    ) -> Self {
        assert!(config.chunk_rows > 0, "a chunk holds at least one row");
        assert!(
            config.checkpoint_every > 0
                && config.checkpoint_every.is_multiple_of(config.chunk_rows),
            "checkpoints fall on chunk boundaries: {} is not a multiple of {}",
            config.checkpoint_every,
            config.chunk_rows
        );
        let base = LayoutCursor::new(seeds);
        Self {
            source,
            frontier: base.clone(),
            base,
            done: known_total == Some(0),
            order,
            cache: RowCache::new(config.chunk_rows, config.resident_chunks),
            checkpoints: Checkpoints::new(config.checkpoint_every, config.max_checkpoints),
            viewport: 0..0,
            known_total,
            config,
            work: Work::default(),
        }
    }

    /// Lays out up to `budget` more rows past the frontier, checking `stop`
    /// between chunks and handing it to the source within one.
    ///
    /// Progress is [`rows_emitted`](Self::rows_emitted) afterwards; a
    /// stopped call is resumed by calling again. Ends by evicting whatever
    /// the protected window does not cover, so streaming far past the
    /// viewport never grows the resident set.
    ///
    /// # Errors
    ///
    /// If the source fails, repeats a commit, stalls, or — on a layout built
    /// with [`from_order`](Self::from_order) — returns the wrong commits.
    pub fn advance(
        &mut self,
        budget: usize,
        stop: &dyn Fn() -> bool,
    ) -> Result<Advance, StreamError<S::Error>> {
        let mut emitted = 0;
        let mut stopped = false;
        while emitted < budget && !self.done {
            if stop() {
                stopped = true;
                break;
            }
            let start = self.frontier.rows_emitted();
            let room = self.config.chunk_rows - start % self.config.chunk_rows;
            let pull = self.pull(room.min(budget - emitted), stop)?;
            if pull.commits.is_empty() && !pull.done && !pull.stopped {
                return Err(StreamError::Stalled);
            }

            let rows = self.frontier.extend(&pull.commits, &mut self.order);
            emitted += rows.len();
            self.cache.push_frontier(start, rows);
            // A checkpoint at the very end of history would never be
            // resumed from: nothing lies past it.
            let reached = self.frontier.rows_emitted();
            if self.checkpoints.is_due(reached) && !pull.done {
                self.checkpoints.record(reached, self.frontier.clone());
            }

            self.done = pull.done;
            if pull.stopped {
                stopped = true;
                break;
            }
        }
        self.evict();
        Ok(Advance {
            rows_emitted: emitted,
            done: self.done,
            stopped,
        })
    }

    /// The next commits for the frontier: read by id when the order already
    /// knows them, pulled from the walk otherwise.
    fn pull(
        &mut self,
        limit: usize,
        stop: &dyn Fn() -> bool,
    ) -> Result<Pull, StreamError<S::Error>> {
        let start = self.frontier.rows_emitted();
        if self.order.len() > start {
            let ids = self.order.ids(start..start + limit);
            let commits = read_exactly(&mut self.source, &mut self.work, ids)?;
            let done = start + commits.len() >= self.order.len();
            return Ok(Pull {
                commits,
                done,
                stopped: false,
            });
        }

        let pull = self.source.next(limit, stop).map_err(StreamError::Source)?;
        self.work.commits_pulled += pull.commits.len();
        let mut fresh = HashMap::with_capacity(pull.commits.len());
        for (offset, commit) in pull.commits.iter().enumerate() {
            let again = start + offset;
            let first = self
                .order
                .row_of(commit.id)
                .or_else(|| fresh.insert(commit.id, again));
            if let Some(first) = first {
                return Err(RepeatedCommit {
                    id: commit.id,
                    first,
                    again,
                }
                .into());
            }
        }
        Ok(pull)
    }

    /// Tells the layout which rows are on screen, so it knows what to keep.
    pub fn set_viewport(&mut self, rows: Range<usize>) {
        self.viewport = rows;
        self.evict();
    }

    /// The rows last passed to [`set_viewport`](Self::set_viewport).
    #[must_use]
    pub fn viewport(&self) -> Range<usize> {
        self.viewport.clone()
    }

    /// The rows in `rows`, as far as the frontier has reached, rebuilding
    /// evicted chunks from the nearest checkpoint on the way.
    ///
    /// # Errors
    ///
    /// If a rebuild needed the source and it failed or returned the wrong
    /// commits, or if a rebuilt chunk differs from the copy that was evicted.
    pub fn rows(&mut self, rows: Range<usize>) -> Result<Vec<Row>, StreamError<S::Error>> {
        let end = rows.end.min(self.frontier.rows_emitted());
        let start = rows.start.min(end);
        if start == end {
            return Ok(Vec::new());
        }
        let chunk_rows = self.config.chunk_rows;
        let mut out = Vec::with_capacity(end - start);
        for chunk in start / chunk_rows..=(end - 1) / chunk_rows {
            if !self.cache.contains(chunk) {
                self.rebuild(chunk)?;
            }
            let chunk_start = chunk * chunk_rows;
            let low = start.max(chunk_start) - chunk_start;
            let high = end.min(chunk_start + chunk_rows) - chunk_start;
            let held = self
                .cache
                .get(chunk)
                .unwrap_or_else(|| unreachable!("chunk {chunk} was held or just rebuilt"));
            out.extend_from_slice(&held[low..high]);
        }
        self.evict();
        Ok(out)
    }

    /// Whether the rows of `row`'s chunk are held, so reading them costs no
    /// replay.
    #[must_use]
    pub fn is_resident(&self, row: usize) -> bool {
        self.cache.contains(row / self.config.chunk_rows)
    }

    /// Rebuilds `chunk` by replaying from the nearest checkpoint at or before
    /// it. Every chunk replayed on the way is admitted too — they are the
    /// ones the protected window keeps anyway — and every checkpoint passed
    /// is recorded again if it had been evicted.
    fn rebuild(&mut self, chunk: usize) -> Result<(), StreamError<S::Error>> {
        let chunk_rows = self.config.chunk_rows;
        let end = ((chunk + 1) * chunk_rows).min(self.frontier.rows_emitted());
        let (mut row, mut cursor) = self
            .checkpoints
            .nearest_at_or_before(chunk * chunk_rows)
            .unwrap_or_else(|| (0, self.base.clone()));
        let commits = read_exactly(&mut self.source, &mut self.work, self.order.ids(row..end))?;
        self.work.rows_replayed += commits.len();

        for batch in commits.chunks(chunk_rows) {
            let rows = cursor.extend(batch, &mut self.order);
            let index = row / chunk_rows;
            if !self.cache.contains(index) {
                self.work.chunks_rebuilt += 1;
            }
            self.cache
                .admit(index, rows)
                .map_err(|Differs { start, end }| StreamError::RebuildDiffers { start, end })?;
            row += batch.len();
            if self.checkpoints.is_due(row) && row < self.frontier.rows_emitted() {
                self.checkpoints.record(row, cursor.clone());
            }
        }
        Ok(())
    }

    /// Evicts chunks outside the protected window until the cache is within
    /// budget. The window runs from the nearest checkpoint at or before
    /// `viewport_top − buffer` to `viewport_bottom + buffer` (DESIGN.md §3.6):
    /// what a scroll in either direction would otherwise have to replay.
    fn evict(&mut self) {
        let chunk_rows = self.config.chunk_rows;
        let top = self.viewport.start.saturating_sub(self.config.buffer_rows);
        let bottom = self.viewport.end.saturating_add(self.config.buffer_rows);
        let first = self.checkpoints.floor(top) / chunk_rows;
        let last = bottom.saturating_sub(1) / chunk_rows;
        self.cache.evict_outside(first..last + 1);
    }

    /// How many rows are laid out so far.
    #[must_use]
    pub fn rows_emitted(&self) -> usize {
        self.frontier.rows_emitted()
    }

    /// How many rows the whole walk will have, if that is already known —
    /// it is on a layout built with [`from_order`](Self::from_order).
    #[must_use]
    pub fn expected_rows(&self) -> Option<usize> {
        self.known_total
            .or_else(|| self.done.then_some(self.frontier.rows_emitted()))
    }

    /// Whether the whole walk is laid out.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// The row `id` is on, if the order knows it. On a layout built with
    /// [`from_order`](Self::from_order) this can be a row the frontier has
    /// not reached yet; [`reanchor`](Self::reanchor) is the check for that.
    #[must_use]
    pub fn row_of(&self, id: Oid) -> Option<usize> {
        self.order.row_of(id)
    }

    /// The commit on `row`, if the order knows it.
    #[must_use]
    pub fn id_at(&self, row: usize) -> Option<Oid> {
        self.order.id_at(row)
    }

    /// Where the viewport goes so that `anchor`'s commit sits at the same
    /// offset it had, if that commit is laid out. `None` means keep
    /// advancing: the commit is not on any row yet.
    #[must_use]
    pub fn reanchor(&self, anchor: &Anchor, height: usize) -> Option<Range<usize>> {
        let row = self.order.row_of(anchor.commit)?;
        if row >= self.frontier.rows_emitted() {
            return None;
        }
        let top = row.saturating_sub(anchor.offset);
        Some(top..top + height)
    }

    /// What is held right now.
    #[must_use]
    pub fn resident(&self) -> Resident {
        Resident {
            rows: self.cache.resident_rows(),
            chunks: self.cache.resident_chunks(),
            checkpoints: self.checkpoints.count(),
            checkpoint_bytes: self.checkpoints.approximate_bytes(),
            frontier_bytes: self.frontier.approximate_bytes(),
            order_bytes: self.order.approximate_bytes(),
        }
    }

    /// The rows the held checkpoints stand at, ascending.
    #[must_use]
    pub fn checkpoint_rows(&self) -> Vec<usize> {
        self.checkpoints.rows()
    }

    /// How much work has been done so far.
    #[must_use]
    pub fn work(&self) -> Work {
        self.work
    }

    /// The source, for callers that want to ask it something — which order
    /// its walk is serving, say.
    #[must_use]
    pub fn source(&self) -> &S {
        &self.source
    }

    /// The source, mutably — how `graph::inclusion` changes what an
    /// [`Included`](crate::inclusion::Included) source lets through before
    /// [`truncate`](Self::truncate) lays the rest out again.
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// Forgets every row from `row` on, so the walk lays them out again from
    /// there — the rows a change of inclusion can alter — and keeps
    /// everything before `row`: its chunks, its checkpoints and its order.
    ///
    /// The frontier becomes the lane state at `row`, replayed from the
    /// nearest checkpoint at or before it by re-reading commits already in
    /// the order. The next [`advance`](Self::advance) pulls from the source
    /// again, so the source must be ready to serve the walk from `row` on.
    /// Truncating at or past the frontier invalidates nothing.
    ///
    /// # Errors
    ///
    /// If re-reading the commits to replay fails.
    pub fn truncate(&mut self, row: usize) -> Result<Invalidated, StreamError<S::Error>> {
        let reached = self.frontier.rows_emitted();
        if row >= reached {
            return Ok(Invalidated {
                from_row: reached,
                ..Invalidated::default()
            });
        }
        let checkpoints = self.checkpoints.drop_after(row);
        let chunks = self.cache.truncate(row);
        let (start, mut cursor) = self
            .checkpoints
            .nearest_at_or_before(row)
            .unwrap_or_else(|| (0, self.base.clone()));
        let commits = read_exactly(&mut self.source, &mut self.work, self.order.ids(start..row))?;
        self.work.rows_replayed += commits.len();
        // The replayed rows are the ones the cache still holds before `row`;
        // only the cursor they leave behind is wanted.
        let _replayed = cursor.extend(&commits, &mut self.order);
        self.order.truncate(row);
        self.frontier = cursor;
        self.done = false;
        self.known_total = None;
        Ok(Invalidated {
            from_row: row,
            chunks,
            checkpoints,
        })
    }

    /// The configuration in force.
    #[must_use]
    pub fn config(&self) -> StreamConfig {
        self.config
    }
}

/// Reads `ids` from `source` and checks it returned exactly them, in order.
fn read_exactly<S: CommitSource>(
    source: &mut S,
    work: &mut Work,
    ids: &[Oid],
) -> Result<Vec<Commit>, StreamError<S::Error>> {
    let commits = source.read(ids).map_err(StreamError::Source)?;
    work.commits_reread += commits.len();
    if commits.len() != ids.len() {
        return Err(StreamError::ShortRead {
            asked: ids.len(),
            got: commits.len(),
        });
    }
    for (&asked, commit) in ids.iter().zip(&commits) {
        if commit.id != asked {
            return Err(StreamError::WrongCommit {
                asked,
                got: commit.id,
            });
        }
    }
    Ok(commits)
}

#[cfg(test)]
mod tests;
