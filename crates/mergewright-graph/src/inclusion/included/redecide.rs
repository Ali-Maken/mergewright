//! Deciding again from a point in the walk: what [`Included::reinclude`]
//! does for a changed inclusion and [`Included::repair`] does for what
//! folding by default got wrong in the provisional order.

use std::collections::HashMap;

use mergewright_core::commit_handle::{CommitHandle, HandleSet};

use crate::layout::source::CommitSource;
use crate::layout::stream::{Advance, Invalidated, StreamError, StreamingLayout};

use super::super::{Frontier, REPLAY_BATCH};
use super::Included;

impl<S: CommitSource> Included<S> {
    /// Mends what the fold got wrong in walk order, returning the first
    /// row to lay out again from, or `None` when nothing was wrong:
    /// commits a late child showed are forced shown and decided again
    /// ([`FoldWalk::take_late`](crate::inclusion::FoldWalk::take_late));
    /// merges whose dropped edge turned out to reach a shown commit draw
    /// it, served again from the merge down without deciding anything again
    /// ([`FoldWalk::take_rejoined`](crate::inclusion::FoldWalk::take_rejoined)).
    ///
    /// # Errors
    ///
    /// If re-reading the commits before the first forced one fails.
    pub fn repair(&mut self) -> Result<Option<usize>, S::Error> {
        let Some((late, rejoined)) = self
            .fold
            .as_mut()
            .map(|fold| (fold.take_late(), fold.take_rejoined()))
        else {
            return Ok(None);
        };
        if late.is_empty() && rejoined.is_empty() {
            return Ok(None);
        }
        let mut named: Vec<CommitHandle> = late.clone();
        named.extend(rejoined.iter().map(|&(merge, _)| merge));
        // Only admitted commits are reported, and every admitted commit is
        // in the decided prefix, so this finds one; were it ever not to,
        // nothing decided would name it and nothing would need laying out.
        let Some(position) = self.first_position(&named) else {
            return Ok(None);
        };
        if late.is_empty() {
            if let Some(fold) = self.fold.as_mut() {
                for (merge, parent) in rejoined {
                    fold.rejoin(merge, parent);
                }
            }
            self.served = position;
            return Ok(Some(self.row_at(position)));
        }
        for &commit in &late {
            self.forced.insert(self.source.id_of(commit));
        }
        self.redecide(position, &rejoined).map(Some)
    }

    /// Decides everything from `sequence[position]` on again, rebuilding
    /// the frontier and the fold from scratch over what comes before, and
    /// returns the row to lay out again from.
    ///
    /// Moved up first to the merge of any rejoined edge the replay could not
    /// draw the way the layout drew it: one still `pending` (not drawn yet;
    /// the replay would draw it), or one drawn whose parent is decided again
    /// (the replay would not).
    pub(super) fn redecide(
        &mut self,
        position: usize,
        pending: &[(CommitHandle, CommitHandle)],
    ) -> Result<usize, S::Error> {
        let position = self.before_rejoins(position, pending);
        let row = self.row_at(position);

        // Everything before `position` decides as it did (none of it is
        // named by the change), so replaying it rebuilds the frontier and
        // the fold the new inclusion would have had there. Structure only —
        // parents by handle, through `links` — since the outcome is thrown
        // away here; only their state is kept.
        self.frontier = Frontier::new(&self.seeds);
        self.restart_fold();
        for &handle in &self.sequence[position..self.decided] {
            self.shown.remove(handle);
        }
        self.decided = position;
        self.served = position;
        let mut start = 0;
        while start < position {
            let end = (start + REPLAY_BATCH).min(position);
            let links = self.source.links(&self.sequence[start..end])?;
            self.resolve_marks();
            for link in links {
                // The verdict was made before and is already in `shown`.
                let _verdict = self.decide(link);
                // Edges the layout above `position` already draws again.
                if let Some(fold) = self.fold.as_mut() {
                    for (merge, parent) in fold.take_rejoined() {
                        fold.rejoin(merge, parent);
                    }
                }
            }
            start = end;
        }
        Ok(row)
    }

    /// `position`, moved up to the merge of every rejoin a replay from it
    /// would draw differently ([`Included::redecide`]), until none is left
    /// below it.
    fn before_rejoins(
        &mut self,
        mut position: usize,
        pending: &[(CommitHandle, CommitHandle)],
    ) -> usize {
        let Some((pending, drawn)) = self.fold.as_mut().map(|fold| {
            let mut pending = pending.to_vec();
            pending.extend(fold.take_rejoined());
            (pending, fold.rejoins().to_vec())
        }) else {
            return position;
        };
        let positions = self.positions_of(
            drawn
                .iter()
                .chain(&pending)
                .flat_map(|&(merge, parent)| [merge, parent]),
        );
        let at = |handle: CommitHandle| positions.get(&handle).copied().unwrap_or(usize::MAX);
        for &(merge, _) in &pending {
            position = position.min(at(merge));
        }
        loop {
            let moved = drawn
                .iter()
                .filter(|&&(_, parent)| at(parent) >= position)
                .map(|&(merge, _)| at(merge))
                .fold(position, usize::min);
            if moved == position {
                return position;
            }
            position = moved;
        }
    }

    /// The first position in the decided sequence holding any of `handles`.
    fn first_position(&self, handles: &[CommitHandle]) -> Option<usize> {
        let mut wanted = HandleSet::new();
        for &handle in handles {
            wanted.insert(handle);
        }
        self.sequence[..self.decided]
            .iter()
            .position(|&handle| wanted.contains(handle))
    }

    /// Where each of `handles` sits in the decided sequence, for those that
    /// do.
    fn positions_of(
        &self,
        handles: impl IntoIterator<Item = CommitHandle>,
    ) -> HashMap<CommitHandle, usize> {
        let wanted: HashMap<CommitHandle, usize> = handles
            .into_iter()
            .map(|handle| (handle, usize::MAX))
            .collect();
        if wanted.is_empty() {
            return wanted;
        }
        self.sequence[..self.decided]
            .iter()
            .enumerate()
            .filter(|(_, handle)| wanted.contains_key(handle))
            .map(|(position, &handle)| (handle, position))
            .collect()
    }

    /// The row the commit at `sequence[position]` is (or would be) drawn
    /// at: how many before it are shown.
    fn row_at(&self, position: usize) -> usize {
        self.sequence[..position]
            .iter()
            .filter(|&&handle| self.shown.contains(handle))
            .count()
    }
}

/// Mends what folding by default got wrong in walk order
/// ([`Included::repair`]) and throws away the rows that change. `None` when
/// nothing was wrong.
///
/// # Errors
///
/// If re-reading history to replay the kept state fails.
pub fn repair<S: CommitSource>(
    layout: &mut StreamingLayout<Included<S>>,
) -> Result<Option<Invalidated>, StreamError<S::Error>> {
    match layout.source_mut().repair().map_err(StreamError::Source)? {
        Some(row) => layout.truncate(row).map(Some),
        None => Ok(None),
    }
}

/// [`StreamingLayout::advance`], then [`repair`] and advance again until
/// nothing is left to mend: what a folded view lays out with. The
/// [`Advance`] is the last one's.
///
/// # Errors
///
/// As [`StreamingLayout::advance`] and [`repair`].
pub fn advance_folded<S: CommitSource>(
    layout: &mut StreamingLayout<Included<S>>,
    budget: usize,
    stop: &dyn Fn() -> bool,
) -> Result<Advance, StreamError<S::Error>> {
    loop {
        let advanced = layout.advance(budget, stop)?;
        if advanced.stopped || repair(layout)?.is_none() {
            return Ok(advanced);
        }
    }
}
