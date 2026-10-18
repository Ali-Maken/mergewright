//! Reading rows out of a [`StreamingLayout`], rebuilding evicted or
//! passed-through chunks from the nearest checkpoint, and truncating on an
//! inclusion change.

use std::ops::Range;

use crate::layout::cache::Differs;
use crate::layout::source::CommitSource;
use crate::row::{Row, RowShape};

use super::materialise::{links_exactly, materialise, rows_per_chunk};
use super::{Invalidated, StreamConfig, StreamError, StreamingLayout};

impl<S: CommitSource> StreamingLayout<S> {
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
    /// chunks not held — evicted or passed through — from the nearest
    /// checkpoint on the way. Only the chunks asked for are materialised.
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
        let wanted = start / chunk_rows..(end - 1) / chunk_rows + 1;
        let missing: Vec<usize> = wanted
            .clone()
            .filter(|&chunk| !self.cache.contains(chunk))
            .collect();
        if let (Some(&first), Some(&last)) = (missing.first(), missing.last()) {
            self.rebuild(first..last + 1)?;
        }
        let mut out = Vec::with_capacity(end - start);
        for chunk in wanted {
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

    /// Rebuilds the chunks in `wanted` by replaying from the nearest
    /// checkpoint at or before the first. Every chunk replayed is checked
    /// against its fingerprint; only the wanted ones not held are
    /// materialised, each run of adjacent ones as one batch so a merge lane
    /// crossing between them is named once. Every checkpoint passed is
    /// recorded again if it had been evicted.
    fn rebuild(&mut self, wanted: Range<usize>) -> Result<(), StreamError<S::Error>> {
        let chunk_rows = self.config.chunk_rows;
        let reached = self.frontier.rows_emitted();
        let end = (wanted.end * chunk_rows).min(reached);
        let (mut row, mut cursor) = self
            .checkpoints
            .nearest_at_or_before(wanted.start * chunk_rows)
            .unwrap_or_else(|| (0, self.base.clone()));
        let handles = self.order.handles(row..end).to_vec();
        let links = links_exactly(&mut self.source, &mut self.work, &handles)?;
        self.work.rows_replayed += links.len();

        // A run of adjacent chunks to materialise: their first index and the
        // shapes laid out for them, in order.
        let mut run: Option<(usize, Vec<RowShape>)> = None;
        for batch in links.chunks(chunk_rows) {
            let shapes = cursor.extend(batch, &self.source, &mut self.order);
            let index = row / chunk_rows;
            self.cache
                .check(index, &shapes)
                .map_err(|Differs { start, end }| StreamError::RebuildDiffers { start, end })?;
            if wanted.contains(&index) && !self.cache.contains(index) {
                run.get_or_insert_with(|| (index, Vec::new()))
                    .1
                    .extend(shapes);
            } else if let Some((first, shapes)) = run.take() {
                self.admit_run(first, shapes)?;
            }
            row += batch.len();
            if self.checkpoints.is_due(row) && row < reached {
                self.checkpoints.record(row, cursor.clone());
            }
        }
        if let Some((first, shapes)) = run {
            self.admit_run(first, shapes)?;
        }
        Ok(())
    }

    /// Materialises a run of adjacent chunks starting at `first` as one
    /// batch and admits each chunk's rows.
    fn admit_run(
        &mut self,
        first: usize,
        shapes: Vec<RowShape>,
    ) -> Result<(), StreamError<S::Error>> {
        let rows = materialise(&mut self.source, &mut self.work, shapes)?;
        let mut rows = rows.into_iter();
        for (offset, len) in rows_per_chunk(rows.len(), self.config.chunk_rows).enumerate() {
            self.work.chunks_rebuilt += 1;
            self.cache
                .admit(first + offset, rows.by_ref().take(len).collect());
        }
        Ok(())
    }

    /// Evicts chunks outside the protected window until the cache is within
    /// budget. The window runs from the checkpoint due at or before
    /// `viewport_top − buffer` to `viewport_bottom + buffer` (DESIGN.md §3.6):
    /// what a scroll in either direction would otherwise have to replay.
    ///
    /// It is the row where a checkpoint is *due*, not the nearest one still
    /// kept: kept checkpoints are capped, so none may stand before a viewport
    /// far behind the frontier, and reaching back to the nearest kept one (row
    /// 0 when there is none) would protect every row laid out. A rebuild from
    /// further back records the due checkpoint again on its way.
    pub(super) fn evict(&mut self) {
        let chunk_rows = self.config.chunk_rows;
        let top = self.viewport.start.saturating_sub(self.config.buffer_rows);
        let bottom = self.viewport.end.saturating_add(self.config.buffer_rows);
        let first = (top - top % self.config.checkpoint_every) / chunk_rows;
        let last = bottom.saturating_sub(1) / chunk_rows;
        self.cache.evict_outside(first..last + 1);
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
        let handles = self.order.handles(start..row).to_vec();
        let links = links_exactly(&mut self.source, &mut self.work, &handles)?;
        self.work.rows_replayed += links.len();
        // The replayed rows are the ones the cache still holds before `row`;
        // only the cursor they leave behind is wanted, so nothing is read —
        // except the shapes of the chunk cut back, whose fingerprint resumes.
        let replayed = cursor.extend(&links, &self.source, &mut self.order);
        let chunk_rows = self.config.chunk_rows;
        if !row.is_multiple_of(chunk_rows) {
            let cut = row / chunk_rows;
            self.cache
                .renote(cut, &replayed[cut * chunk_rows - start..]);
        }
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
