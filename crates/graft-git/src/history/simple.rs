//! The two orderings that need no commit-graph: commit date, and first
//! parent only.
//!
//! Both are the same loop — take the most recent commit not yet emitted,
//! emit it, discover what it points at — and differ only in which parents
//! they follow. Neither ever has to look ahead, so both emit their first
//! commit after a single read. That is what makes commit date ADR-0008's
//! first paint when no commit-graph file exists: it is not the order the
//! graph will finally be drawn in, but it is on screen immediately and it is
//! honest about being provisional.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use graft_core::oid::Oid;

use super::node::{Key, Node, NodeError, Nodes};

/// Which parents the walk follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Follow {
    /// Every parent: the full history behind the tips.
    AllParents,
    /// Only the first parent of each commit — the branch's own trunk, with
    /// everything that was merged into it left out.
    FirstParentOnly,
}

/// A commit on the frontier, with the node it was read from.
///
/// The node travels with the key so that popping a commit never has to read
/// it a second time: a date walk reads each commit exactly once, which is
/// what makes it the cheapest first paint available.
struct Queued {
    key: Key,
    node: Node,
}

impl Ord for Queued {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Queued {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for Queued {}

/// A date-ordered walk.
pub(crate) struct Dated {
    frontier: BinaryHeap<Queued>,
    seen: HashSet<Oid>,
    follow: Follow,
}

impl Dated {
    /// Seeds the walk with the commits it starts from.
    pub(crate) fn new(
        nodes: &impl Nodes,
        tips: &[Oid],
        follow: Follow,
        reads: &mut u64,
    ) -> Result<Self, NodeError> {
        let mut walk = Dated {
            frontier: BinaryHeap::new(),
            seen: HashSet::new(),
            follow,
        };
        for &tip in tips {
            walk.push(nodes, tip, reads)?;
        }
        Ok(walk)
    }

    /// The next commit: the most recent one on the frontier, with an exact
    /// date tie broken by the smaller id.
    pub(crate) fn next(
        &mut self,
        nodes: &impl Nodes,
        reads: &mut u64,
    ) -> Result<Option<Oid>, NodeError> {
        let Some(top) = self.frontier.pop() else {
            return Ok(None);
        };
        let parents: &[Oid] = match self.follow {
            Follow::AllParents => &top.node.parents,
            Follow::FirstParentOnly => &top.node.parents[..top.node.parents.len().min(1)],
        };
        for &parent in parents {
            self.push(nodes, parent, reads)?;
        }
        Ok(Some(top.key.id))
    }

    fn push(&mut self, nodes: &impl Nodes, id: Oid, reads: &mut u64) -> Result<(), NodeError> {
        if !self.seen.insert(id) {
            return Ok(());
        }
        let node = nodes.node(id)?;
        *reads += 1;
        self.frontier.push(Queued {
            key: Key::of(&node),
            node,
        });
        Ok(())
    }
}
