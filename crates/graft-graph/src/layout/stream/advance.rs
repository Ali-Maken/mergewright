//! Building a [`StreamingLayout`] and advancing its frontier.

use std::collections::HashMap;

use graft_core::commit_handle::CommitHandle;

use crate::layout::LayoutCursor;
use crate::layout::cache::RowCache;
use crate::layout::checkpoint::Checkpoints;
use crate::layout::order::{EmittedOrder, RepeatedCommit};
use crate::layout::source::{CommitSource, Pull};
use crate::seeds::Seeds;

use super::materialise::{links_exactly, materialise};
use super::{Advance, StreamConfig, StreamError, StreamingLayout, Work};

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
    /// If `order` repeats a commit.
    ///
    /// # Panics
    ///
    /// As [`StreamingLayout::new`].
    pub fn from_order(
        source: S,
        order: Vec<CommitHandle>,
        seeds: Seeds,
        config: StreamConfig,
    ) -> Result<Self, StreamError<S::Error>> {
        let order = EmittedOrder::from_handles(order)?;
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
        let base = LayoutCursor::new(seeds, &source);
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
            if pull.links.is_empty() && !pull.done && !pull.stopped {
                return Err(StreamError::Stalled);
            }

            let shapes = self
                .frontier
                .extend(&pull.links, &self.source, &mut self.order);
            emitted += shapes.len();
            self.cache.note(start, &shapes);
            let chunk = start / self.config.chunk_rows;
            let offset = start % self.config.chunk_rows;
            let continues_held = self.cache.held_len(chunk) == Some(offset);
            if continues_held || (offset == 0 && self.materialises(chunk)) {
                let rows = materialise(&mut self.source, &mut self.work, shapes)?;
                self.cache.push_frontier(start, rows);
            }
            // Otherwise the chunk passes through: lane state, order and its
            // fingerprint moved on; no commit was read.
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

    /// The next links for the frontier: re-served by handle when the order
    /// already knows them, pulled from the walk otherwise.
    fn pull(
        &mut self,
        limit: usize,
        stop: &dyn Fn() -> bool,
    ) -> Result<Pull, StreamError<S::Error>> {
        let start = self.frontier.rows_emitted();
        if self.order.len() > start {
            let handles = self.order.handles(start..start + limit).to_vec();
            let links = links_exactly(&mut self.source, &mut self.work, &handles)?;
            let done = start + links.len() >= self.order.len();
            return Ok(Pull {
                links,
                done,
                stopped: false,
            });
        }

        let pull = self.source.pull(limit, stop).map_err(StreamError::Source)?;
        self.work.commits_pulled += pull.links.len();
        self.work.links_read += pull.links.len();
        let mut fresh = HashMap::with_capacity(pull.links.len());
        for (offset, link) in pull.links.iter().enumerate() {
            let again = start + offset;
            let handle = link.commit;
            let first = self
                .order
                .row_of(handle)
                .or_else(|| fresh.insert(handle, again));
            if let Some(first) = first {
                return Err(RepeatedCommit {
                    commit: handle,
                    first,
                    again,
                }
                .into());
            }
        }
        Ok(pull)
    }

    /// Whether `chunk`'s rows are read when the frontier lays it out: it
    /// meets the viewport widened by `buffer_rows` either way — the rows a
    /// scroll of a screen or so asks for next.
    fn materialises(&self, chunk: usize) -> bool {
        let chunk_rows = self.config.chunk_rows;
        let top = self.viewport.start.saturating_sub(self.config.buffer_rows);
        let bottom = self.viewport.end.saturating_add(self.config.buffer_rows);
        let first = chunk * chunk_rows;
        first < bottom && first + chunk_rows > top
    }
}
