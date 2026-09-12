//! A lane: one open column in the graph, holding one branch's chain.

use graft_core::oid::Oid;

use crate::identity::BranchIdentity;

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
    pub identity: BranchIdentity,
    /// The next commit that continues this lane.
    pub waiting_for: Oid,
    /// `true` for lane 0 and every integration-branch lane: never handed to
    /// the free pool, even once nothing is left to draw in it (DESIGN.md
    /// §3.1) — the column stays permanently theirs, rather than being
    /// silently reassigned to an unrelated branch above the point they
    /// vacated it.
    pub reserved: bool,
}
