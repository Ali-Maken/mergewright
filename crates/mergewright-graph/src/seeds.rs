//! What lane 0 and the reserved integration lanes start out pointing at.
//!
//! Resolving which branch is the default branch and which are integration
//! branches is `mergewright-git::branches::Branches`' job (ADR-0001); this is the
//! shape layout consumes to turn that resolution into reserved lanes before
//! the walk starts.

use mergewright_core::oid::Oid;

use crate::identity::BranchIdentity;

/// One branch's tip and identity, as layout needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BranchSeed {
    /// The commit the branch currently points at.
    pub tip: Oid,
    /// The colour this branch draws in.
    pub identity: BranchIdentity,
    /// The local branch this ref is the remote copy of: its configured
    /// upstream, else the same short name on a remote. `None` for local and
    /// remote-only branches.
    pub follows: Option<BranchIdentity>,
}

impl BranchSeed {
    /// A local or remote-only branch: it is no other ref's copy.
    #[must_use]
    pub fn new(tip: Oid, identity: BranchIdentity) -> Self {
        Self {
            tip,
            identity,
            follows: None,
        }
    }
}

/// The branches a [`LayoutCursor`](crate::layout::LayoutCursor) is seeded
/// with, before it sees a single commit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seeds {
    /// Lane 0: the repository's default branch, straight for its whole life
    /// and winning every tie (PROMPT.md §1.2). [`None`] in a repository with
    /// no branch at all.
    pub default_branch: Option<BranchSeed>,
    /// Long-lived branches with a lane reserved adjacent to lane 0, never
    /// competing for it. In the order their lanes are assigned: the first
    /// entry gets column 1, the next column 2, and so on.
    pub integration_branches: Vec<BranchSeed>,
    /// Ordinary branch tips, for colour stability only: an open lane whose
    /// tip is here draws in its registered identity instead of falling back
    /// to a short-SHA identity. Omitting a tip degrades gracefully — it is
    /// not an error — because only colour stability, never correctness,
    /// depends on this list.
    pub known_branches: Vec<BranchSeed>,
}
