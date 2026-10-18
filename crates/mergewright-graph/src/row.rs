//! One laid-out commit: its lane, its edges, and any Mistake Radar marker.

use std::collections::HashMap;

use mergewright_core::commit::Commit;
use mergewright_core::commit_handle::CommitHandle;
use mergewright_core::mistake_radar::MistakeRadarKind;
use mergewright_core::oid::Oid;

use crate::identity::BranchIdentity;
use crate::lane::LaneIdentity;
use crate::merge_name::recover_branch_name;

/// One commit, positioned in the graph.
///
/// Hashable so a row cache can keep a fingerprint of rows it evicted and
/// check that a rebuild reproduced them (DESIGN.md §3.6).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Row {
    /// The commit as the repository has it: `commit.parents` is every real
    /// parent under every inclusion (fold, opened merges, first-parent
    /// view, hidden stashes). Anything that reasons about the commit, or
    /// builds a git write, reads this (ease-pass 06 D1).
    pub commit: Commit,
    /// The parents this view draws an edge to, in order — the same ids as
    /// [`Row::outgoing`]: the real parents trimmed by the fold, the
    /// first-parent view and hidden stashes. Drawing only.
    pub drawn_parents: Vec<Oid>,
    /// This commit's own column: the lane it continues, or a freshly opened
    /// one if none was waiting for it.
    pub lane: usize,
    /// Lanes that converge on this row and close here — bending in, rather
    /// than continuing straight down as `lane` does. Sorted ascending: order
    /// carries no information of its own, so a fixed order keeps two runs
    /// over the same commits byte-identical.
    pub incoming: Vec<usize>,
    /// Lanes leaving this row, one per parent, in parent order: `outgoing[0]`
    /// is always the first-parent edge.
    pub outgoing: Vec<(usize, Oid)>,
    /// The branch drawn in each column on this row, indexed by column;
    /// `None` where no edge touches the column. Covers the row's own lane,
    /// every lane passing straight through, every lane closing into the node
    /// and every merge lane opening from it — one identity per column is
    /// unambiguous because a column freed on a row is never reused on that
    /// same row. A reserved lane whose tip has not been reached yet has no
    /// edge above its tip, so it is `None` until then. Trailing `None`s are
    /// trimmed.
    ///
    /// Captured when the row is laid out, so a renderer colours any row it
    /// holds — a cached chunk, a viewport that starts mid-history — without
    /// the cursor's state at that moment (m1-the-graph/08).
    pub lanes: Vec<Option<BranchIdentity>>,
    /// A Mistake Radar classification. Always [`None`] on a `Row` `graph`
    /// produces: `graph` lays out the graph and nothing more, and never
    /// calls into `ops`, which is what fills this in as a later pass.
    pub marker: Option<MistakeRadarKind>,
}

/// One commit laid out, before its commit is read: what
/// [`LayoutCursor::extend`](crate::layout::LayoutCursor::extend) produces
/// from links (ADR-0026 item 4). [`Row::assemble`] pairs it with the commit.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RowShape {
    /// The commit this row draws.
    pub commit: CommitHandle,
    /// As [`Row::lane`].
    pub lane: usize,
    /// As [`Row::incoming`].
    pub incoming: Vec<usize>,
    /// As [`Row::outgoing`], parents by handle: the parents the view shows.
    pub outgoing: Vec<(usize, CommitHandle)>,
    /// As [`Row::lanes`], merge lanes not yet named.
    pub lanes: Vec<Option<LaneIdentity>>,
}

/// The names of the merge lanes a batch of shapes draws, gathered once per
/// batch so [`Row::assemble`] reads nothing.
#[derive(Clone, Debug, Default)]
pub struct MergeNames {
    /// Keyed by merge and parent: an unnamed octopus merge falls back to a
    /// different short sha per parent.
    names: HashMap<(CommitHandle, CommitHandle), BranchIdentity>,
    reads: usize,
}

impl MergeNames {
    /// Names every distinct `Merge { merge, .. }` in `shapes[..].lanes`.
    /// `commits[i]` is `shapes[i].commit`'s commit (same length, same order);
    /// a merge on hand there costs nothing, the rest go to `read` in one
    /// batch — bounded by the lanes open at the batch's first row.
    ///
    /// # Errors
    ///
    /// Whatever `read` fails with.
    ///
    /// # Panics
    ///
    /// If `commits` and `shapes` differ in length, or `read` returns fewer
    /// commits than it was asked for (a reader checks its own answers).
    pub fn gather<E>(
        shapes: &[RowShape],
        commits: &[Commit],
        mut read: impl FnMut(&[CommitHandle]) -> Result<Vec<Commit>, E>,
        id_of: impl Fn(CommitHandle) -> Oid,
    ) -> Result<Self, E> {
        assert_eq!(shapes.len(), commits.len(), "one commit per shape");
        let on_hand: HashMap<CommitHandle, &Commit> = shapes
            .iter()
            .map(|shape| shape.commit)
            .zip(commits)
            .collect();
        let mut wanted: Vec<(CommitHandle, CommitHandle)> = Vec::new();
        for identity in shapes.iter().flat_map(|shape| shape.lanes.iter().flatten()) {
            if let LaneIdentity::Merge { merge, parent } = *identity
                && !wanted.contains(&(merge, parent))
            {
                wanted.push((merge, parent));
            }
        }

        let mut missing: Vec<CommitHandle> = Vec::new();
        for &(merge, _) in &wanted {
            if !on_hand.contains_key(&merge) && !missing.contains(&merge) {
                missing.push(merge);
            }
        }
        let read_back = if missing.is_empty() {
            Vec::new()
        } else {
            read(&missing)?
        };
        assert_eq!(
            read_back.len(),
            missing.len(),
            "a merge-name read returns one commit per handle"
        );
        let fetched: HashMap<CommitHandle, &Commit> =
            missing.iter().copied().zip(&read_back).collect();

        let names = wanted
            .into_iter()
            .map(|(merge, parent)| {
                let commit = on_hand
                    .get(&merge)
                    .or_else(|| fetched.get(&merge))
                    .unwrap_or_else(|| unreachable!("every merge is on hand or was just read"));
                let identity = match recover_branch_name(commit.subject()) {
                    Some(name) => BranchIdentity::of_name(&name),
                    None => BranchIdentity::of_short_sha(id_of(parent)),
                };
                ((merge, parent), identity)
            })
            .collect();
        Ok(MergeNames {
            names,
            reads: missing.len(),
        })
    }

    /// Commits [`gather`](Self::gather) had to read: merges not on hand.
    #[must_use]
    pub fn reads(&self) -> usize {
        self.reads
    }

    /// The branch `identity` draws as.
    ///
    /// # Errors
    ///
    /// [`AssembleError::MergeNotGathered`] for a merge lane these names were
    /// not gathered over.
    pub fn resolve(&self, identity: LaneIdentity) -> Result<BranchIdentity, AssembleError> {
        match identity {
            LaneIdentity::Known(known) => Ok(known),
            LaneIdentity::Merge { merge, parent } => self
                .names
                .get(&(merge, parent))
                .copied()
                .ok_or(AssembleError::MergeNotGathered { merge }),
        }
    }
}

/// Why a shape could not become a row.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssembleError {
    /// A merge lane's name was not gathered for this batch.
    #[error("merge lane of {merge:?} was not named before assembly")]
    MergeNotGathered {
        /// The merge whose subject names the lane.
        merge: CommitHandle,
    },
}

impl Row {
    /// Two or more of git's parents. Stash-blind: the drawing `of` fns call
    /// this; the app's answer, which excludes stash entries, is
    /// `history::parents::is_merge`.
    #[must_use]
    pub fn is_merge(&self) -> bool {
        self.commit.parents.len() >= 2
    }

    /// `drawn_parents` := the shape's outgoing parents' ids, in order (the
    /// parents the view shows); `commit` kept as read, every real parent
    /// included; `lanes` resolved through `names`.
    ///
    /// # Errors
    ///
    /// If a merge lane on the row is missing from `names`.
    pub fn assemble(
        shape: RowShape,
        commit: Commit,
        names: &MergeNames,
        id_of: &dyn Fn(CommitHandle) -> Oid,
    ) -> Result<Row, AssembleError> {
        let drawn_parents = shape
            .outgoing
            .iter()
            .map(|&(_, parent)| id_of(parent))
            .collect();
        let outgoing = shape
            .outgoing
            .into_iter()
            .map(|(column, parent)| (column, id_of(parent)))
            .collect();
        let lanes = shape
            .lanes
            .into_iter()
            .map(|identity| identity.map(|identity| names.resolve(identity)).transpose())
            .collect::<Result<_, _>>()?;
        Ok(Row {
            commit,
            drawn_parents,
            lane: shape.lane,
            incoming: shape.incoming,
            outgoing,
            lanes,
            marker: None,
        })
    }
}

#[cfg(test)]
mod tests;
