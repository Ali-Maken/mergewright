//! A history row's node, decided once for both tiers (ease-pass 06r, review
//! M1 and M2): the Unicode tier's [`NodeKind`] and the pixel tier's [`Ring`]
//! are two spellings of one [`Node`], so the tiers cannot disagree.
//!
//! Merge-ness is the app's answer, [`parents::is_merge`]: a folded merge and
//! a merge in the first-parent view are merges; no stash entry is. The
//! newest entry draws as the stash; an older one draws plain. The drawing
//! crates' `NodeKind::of` and `Ring::of` are stash-blind (they count git's
//! parents), so they never decide a history row's node.

use mergewright_graph::Row;
use mergewright_graph::raster::Ring;
use mergewright_ui::graph_column::NodeKind;

use crate::history::parents;
use crate::refs::RefBadges;

/// The shape a history row's node draws as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Node {
    /// `HEAD`'s commit. Wins over the rest: where you are is the node a
    /// person looks for first.
    Head,
    /// The newest stash entry (`refs/stash`).
    Stash,
    /// A merge to the app ([`parents::is_merge`]).
    Merge,
    /// A commit with no parents.
    Root,
    /// Everything else, an older stash entry included.
    Commit,
}

impl Node {
    /// `row`'s node. With no refs read yet no row is `HEAD` or a stash
    /// entry, and git's parents decide.
    pub(crate) fn of(row: &Row, refs: Option<&RefBadges>) -> Node {
        let id = Some(row.commit.id);
        if refs.and_then(RefBadges::head) == id {
            Node::Head
        } else if refs.and_then(RefBadges::stash) == id {
            Node::Stash
        } else if parents::is_merge(row, refs) {
            Node::Merge
        } else if parents::of(row, refs).is_empty() {
            Node::Root
        } else {
            Node::Commit
        }
    }

    /// The Unicode tier's spelling.
    pub(crate) fn kind(self) -> NodeKind {
        match self {
            Node::Head => NodeKind::Head,
            Node::Stash => NodeKind::Stash,
            Node::Merge => NodeKind::Merge,
            Node::Root => NodeKind::Root,
            Node::Commit => NodeKind::Commit,
        }
    }

    /// The pixel tier's spelling.
    pub(crate) fn ring(self) -> Ring {
        match self {
            Node::Head => Ring::Head,
            Node::Stash => Ring::Stash,
            Node::Merge => Ring::Merge,
            Node::Root => Ring::Root,
            Node::Commit => Ring::Commit,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
