//! What a lane is to the branch under the cursor: the branch itself, a
//! branch it was built on, the trunk, or something else. The graph colours
//! by this role, not by branch (design note `ease-pass`, G1): three colours
//! and grey, whichever branch the cursor rests on.
//!
//! Pure. The chain of bases is measured in git by `mergewright-ops`
//! (`branch_status::route`) and handed in here as identities.

use crate::identity::BranchIdentity;
use crate::row::Row;

/// What one branch is to the focused one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LaneRole {
    /// The focused branch itself: the one the cursor row was made on.
    This,
    /// A branch the focused one was built on (its route, trunk excluded).
    Under,
    /// The trunk (`main`).
    Trunk,
    /// Anything else; drawn grey.
    Other,
}

/// The focused branch, the bases it was built on, and the trunk under them
/// all: everything [`Route::role`] needs to give each lane its role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    trunk: BranchIdentity,
    /// Nearest base first, trunk excluded.
    chain: Vec<BranchIdentity>,
    focus: Option<BranchIdentity>,
}

impl Route {
    /// A route whose bases are known. `built_on`: bases nearest first, trunk
    /// excluded. A trunk that slipped into `built_on` is dropped from the
    /// chain, so the trunk always reads as [`LaneRole::Trunk`].
    pub fn new(
        trunk: BranchIdentity,
        built_on: &[BranchIdentity],
        focus: Option<BranchIdentity>,
    ) -> Self {
        let chain = built_on
            .iter()
            .copied()
            .filter(|&base| base != trunk)
            .collect();
        Self {
            trunk,
            chain,
            focus,
        }
    }

    /// Before the route is read: no chain yet, so only the focus and the
    /// trunk stand out.
    pub fn unread(trunk: BranchIdentity, focus: Option<BranchIdentity>) -> Self {
        Self::new(trunk, &[], focus)
    }

    /// The focused branch, if the cursor row was made on a named one.
    pub fn focus(&self) -> Option<BranchIdentity> {
        self.focus
    }

    /// The trunk.
    pub fn trunk(&self) -> BranchIdentity {
        self.trunk
    }

    /// The bases the focused branch was built on, nearest first, trunk
    /// excluded.
    pub fn chain(&self) -> &[BranchIdentity] {
        &self.chain
    }

    /// `branch`'s role: the focus → [`LaneRole::This`] (even when it is the
    /// trunk); else the trunk → [`LaneRole::Trunk`]; else in the chain →
    /// [`LaneRole::Under`]; else [`LaneRole::Other`].
    pub fn role(&self, branch: BranchIdentity) -> LaneRole {
        if self.focus == Some(branch) {
            LaneRole::This
        } else if branch == self.trunk {
            LaneRole::Trunk
        } else if self.chain.contains(&branch) {
            LaneRole::Under
        } else {
            LaneRole::Other
        }
    }
}

/// The branch a row was made on: the identity of the row's own lane,
/// `row.lanes[row.lane]`. `None` when that lane is unnamed or out of range.
pub fn made_on(row: &Row) -> Option<BranchIdentity> {
    row.lanes.get(row.lane).copied().flatten()
}

#[cfg(test)]
mod tests;
