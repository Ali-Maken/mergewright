//! A lane: one open column in the graph, holding one branch's chain.

use graft_core::commit_handle::CommitHandle;

use crate::identity::BranchIdentity;

/// Who a lane draws as — known while laying out, or deferred to assembly.
///
/// A merge lane is named by its merge's subject (DESIGN.md §3.4), which only
/// a commit read has; the layout works on links and reads nothing, so it
/// records the two handles that name the lane and
/// [`Row::assemble`](crate::row::Row::assemble) resolves them later.
/// Paint-only: the layout never compares identities to decide anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LaneIdentity {
    /// Resolved already: a seed's, a known tip's or a short-sha fallback.
    Known(BranchIdentity),
    /// Named by `merge`'s subject, or `parent`'s short sha when no branch
    /// name survives in it. Resolved at assembly, never during layout.
    Merge {
        /// The merge commit whose subject names the lane.
        merge: CommitHandle,
        /// The merge parent the lane waits for.
        parent: CommitHandle,
    },
}

impl LaneIdentity {
    /// The identity, if it is already [`Known`](Self::Known).
    #[must_use]
    pub fn known(self) -> Option<BranchIdentity> {
        match self {
            LaneIdentity::Known(identity) => Some(identity),
            LaneIdentity::Merge { .. } => None,
        }
    }
}

/// One open column in the graph.
///
/// A lane's column ([`index`](Self::index)) is fixed for its whole life
/// (PROMPT.md §1.2, DESIGN.md §3.1): only [`waiting_for`](Self::waiting_for)
/// changes, as the lane's chain advances toward the commit that continues
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lane {
    /// The column this lane occupies.
    pub index: usize,
    /// The colour this lane draws in, independent of `index`.
    pub identity: LaneIdentity,
    /// The next commit that continues this lane.
    pub waiting_for: CommitHandle,
    /// `true` for lane 0 and every integration-branch lane: never handed to
    /// the free pool, even once nothing is left to draw in it (DESIGN.md
    /// §3.1) — the column stays permanently theirs, rather than being
    /// silently reassigned to an unrelated branch above the point they
    /// vacated it.
    pub reserved: bool,
}
