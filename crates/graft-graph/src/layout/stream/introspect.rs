//! Read-only queries over a [`StreamingLayout`]: progress, row/id lookups,
//! anchoring and introspection for tests and benchmarks.

use std::ops::Range;

use graft_core::oid::Oid;

use crate::layout::source::CommitSource;

use super::{Anchor, Resident, StreamingLayout, Work};

impl<S: CommitSource> StreamingLayout<S> {
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
        self.order.row_of(self.source.handle_of(&id)?)
    }

    /// The commit on `row`, if the order knows it.
    #[must_use]
    pub fn id_at(&self, row: usize) -> Option<Oid> {
        self.order
            .handle_at(row)
            .map(|handle| self.source.id_of(handle))
    }

    /// Where the viewport goes so that `anchor`'s commit sits at the same
    /// offset it had, if that commit is laid out. `None` means keep
    /// advancing: the commit is not on any row yet.
    #[must_use]
    pub fn reanchor(&self, anchor: &Anchor, height: usize) -> Option<Range<usize>> {
        let handle = self.source.handle_of(&anchor.commit)?;
        let row = self.order.row_of(handle)?;
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
            passed_through: self.cache.passed_through(),
        }
    }

    /// The rows the held checkpoints stand at, ascending.
    #[must_use]
    pub fn checkpoint_rows(&self) -> Vec<usize> {
        self.checkpoints.rows()
    }

    /// Rows between kept checkpoints: `checkpoint_every`, doubled each time
    /// the cap was reached (m1-the-graph/60).
    #[must_use]
    pub fn checkpoint_stride(&self) -> usize {
        self.checkpoints.stride()
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
}
