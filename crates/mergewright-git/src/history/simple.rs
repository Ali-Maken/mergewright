//! The two orderings that need no commit-graph: commit date, and first
//! parent only.
//!
//! Both are the same loop — take the most recent commit not yet emitted,
//! emit it, discover what it points at — and differ only in which parents
//! they follow. On distinct dates neither ever has to look ahead, so both
//! emit their first commit after a single read. That is what makes commit
//! date ADR-0008's first paint when no commit-graph file exists: it is not
//! the order the graph will finally be drawn in, but it is on screen
//! immediately and it is honest about being provisional.
//!
//! **Exact date ties** are the one place the loop looks ahead
//! (m1-the-graph/67). A child is never dated before its parent unless a clock
//! was wrong, so a commit's unseen children all sit at its own date or later;
//! everything later is already out. The walk therefore takes the whole class
//! of commits at the frontier's newest date — reading down through parents
//! at that same date — and emits the class children first (Kahn's algorithm,
//! the smaller id first among the ready). History written by a script, a
//! rebase or `git am` shares timestamps; without this a parent was drawn
//! above its child. A clock that ran backwards still can put one there: that
//! is the provisional order's known limit, and promotion corrects it.

use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, VecDeque};

use mergewright_core::commit_handle::{CommitHandle, HandleSet, Parents};

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

impl Follow {
    /// How many of a commit's parents the walk follows.
    fn count(self, parents: &Parents) -> usize {
        match self {
            Follow::AllParents => parents.len(),
            Follow::FirstParentOnly => 1,
        }
    }
}

/// The parents of `queued` the walk follows.
fn followed(follow: Follow, queued: &Queued) -> impl Iterator<Item = CommitHandle> + '_ {
    let count = follow.count(&queued.node.parents);
    queued.node.parents.iter().take(count)
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
    seen: HandleSet,
    follow: Follow,
    /// A tie class already put in order, waiting to be handed out.
    ready: VecDeque<(CommitHandle, Parents)>,
}

impl Dated {
    /// Seeds the walk with the commits it starts from.
    pub(crate) fn new(
        nodes: &impl Nodes,
        tips: &[CommitHandle],
        follow: Follow,
        reads: &mut u64,
    ) -> Result<Self, NodeError> {
        let mut walk = Dated {
            frontier: BinaryHeap::new(),
            seen: HandleSet::new(),
            follow,
            ready: VecDeque::new(),
        };
        for &tip in tips {
            if let Some(queued) = walk.read(nodes, tip, reads)? {
                walk.frontier.push(queued);
            }
        }
        Ok(walk)
    }

    /// The next commit and its parents: the most recent one on the frontier,
    /// with an exact date tie broken by the smaller id — but never a commit
    /// before a child at its own date.
    pub(crate) fn next(
        &mut self,
        nodes: &impl Nodes,
        reads: &mut u64,
    ) -> Result<Option<(CommitHandle, Parents)>, NodeError> {
        if let Some(next) = self.ready.pop_front() {
            return Ok(Some(next));
        }
        let Some(top) = self.frontier.pop() else {
            return Ok(None);
        };
        let time = top.key.time;
        let mut class = vec![top];
        while self
            .frontier
            .peek()
            .is_some_and(|queued| queued.key.time == time)
        {
            class.extend(self.frontier.pop());
        }
        // Read down through the class: a parent at the same date joins it
        // (one of its children may still be unseen); any other goes back on
        // the frontier. The class grows while it is walked.
        let mut member = 0;
        while member < class.len() {
            // Indexed rather than iterated: `read` needs `self` while the
            // class is borrowed, and a collected copy would allocate once per
            // commit on the million-commit first paint. Parents are few.
            let count = self.follow.count(&class[member].node.parents);
            for nth in 0..count {
                let Some(parent) = class[member].node.parents.iter().nth(nth) else {
                    break;
                };
                if let Some(queued) = self.read(nodes, parent, reads)? {
                    if queued.key.time == time {
                        class.push(queued);
                    } else {
                        self.frontier.push(queued);
                    }
                }
            }
            member += 1;
        }
        if class.len() == 1 {
            let only = class
                .pop()
                .map(|queued| (queued.key.handle, queued.node.parents));
            return Ok(only);
        }
        self.order_class(class);
        Ok(self.ready.pop_front())
    }

    /// Puts one tie class children first into `ready`: Kahn's algorithm over
    /// the edges inside the class, the smaller id first among the ready.
    fn order_class(&mut self, mut class: Vec<Queued>) {
        // Best first, so a smaller index is the higher priority.
        class.sort_unstable_by(|a, b| b.cmp(a));
        let mut index: Vec<(u32, usize)> = class
            .iter()
            .enumerate()
            .map(|(at, queued)| (queued.key.handle.0, at))
            .collect();
        index.sort_unstable();
        let position = |handle: CommitHandle| {
            index
                .binary_search_by_key(&handle.0, |&(key, _)| key)
                .ok()
                .map(|found| index[found].1)
        };
        let follow = self.follow;
        let mut children = vec![0u32; class.len()];
        for queued in &class {
            for parent in followed(follow, queued).filter_map(position) {
                children[parent] += 1;
            }
        }
        let mut waiting: BinaryHeap<Reverse<usize>> = (0..class.len())
            .filter(|&at| children[at] == 0)
            .map(Reverse)
            .collect();
        let mut out = Vec::with_capacity(class.len());
        while let Some(Reverse(at)) = waiting.pop() {
            for parent in followed(follow, &class[at]).filter_map(position) {
                children[parent] -= 1;
                if children[parent] == 0 {
                    waiting.push(Reverse(parent));
                }
            }
            out.push(at);
        }
        // A class is acyclic (commits cannot be their own ancestors), so
        // every member came out exactly once.
        debug_assert_eq!(out.len(), class.len());
        let mut slots: Vec<Option<Queued>> = class.into_iter().map(Some).collect();
        self.ready.extend(
            out.into_iter()
                .filter_map(|at| slots[at].take())
                .map(|queued| (queued.key.handle, queued.node.parents)),
        );
    }

    /// Reads a commit the walk has not met yet; `None` if it has.
    fn read(
        &mut self,
        nodes: &impl Nodes,
        handle: CommitHandle,
        reads: &mut u64,
    ) -> Result<Option<Queued>, NodeError> {
        if !self.seen.insert(handle) {
            return Ok(None);
        }
        let node = nodes.node(handle)?;
        *reads += 1;
        Ok(Some(Queued {
            key: Key::of(nodes, handle, &node),
            node,
        }))
    }
}
