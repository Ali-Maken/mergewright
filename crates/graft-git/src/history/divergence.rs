//! How far a branch and its upstream have drifted apart: the `↑2 ↓1` a ref
//! badge shows.
//!
//! Counted the way `git rev-list --left-right --count local...upstream`
//! counts: commits only `local` reaches are ahead, commits only `upstream`
//! reaches are behind. The walk paints each commit with the side(s) that
//! reach it, newest first, and stops as soon as every commit still queued is
//! reached by both sides — everything older is shared and changes neither
//! count. So the cost is the size of the two sides' difference plus a little,
//! not the size of history.

use std::collections::{BinaryHeap, HashMap};

use graft_core::oid::Oid;

use super::node::{Node, NodeError, Nodes};
use super::{WalkError, open_nodes};
use crate::repo::RepoHandle;

/// How many commits a branch has that its upstream does not, and the other
/// way round.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct AheadBehind {
    /// Commits only the branch reaches: what a push would send.
    pub ahead: usize,
    /// Commits only the upstream reaches: what a pull would bring in.
    pub behind: usize,
}

impl AheadBehind {
    /// Neither side has a commit the other lacks.
    #[must_use]
    pub fn in_sync(self) -> bool {
        self.ahead == 0 && self.behind == 0
    }
}

/// Counts how far `local` and `upstream` have drifted apart.
///
/// Reads through the commit-graph file when there is one. Commit dates are
/// trusted to put a child after its parents, as `git`'s own walk trusts
/// them; a history with badly skewed clocks can miscount by the skewed
/// commits.
///
/// # Errors
/// [`WalkError`] if the commit-graph file or a commit on either side cannot
/// be read.
pub fn ahead_behind(
    handle: &RepoHandle,
    local: Oid,
    upstream: Oid,
) -> Result<AheadBehind, WalkError> {
    let nodes = open_nodes(handle.gix())?;
    Ok(count(&nodes, local, upstream)?)
}

/// Reached by the branch.
const LOCAL: u8 = 1;
/// Reached by the upstream.
const UPSTREAM: u8 = 2;
/// Reached by both: shared history, counted by neither side.
const BOTH: u8 = LOCAL | UPSTREAM;

/// The counting itself, over any [`Nodes`] so it can be tested against a
/// hand-built graph.
pub(crate) fn count(
    nodes: &impl Nodes,
    local: Oid,
    upstream: Oid,
) -> Result<AheadBehind, NodeError> {
    let mut counts = AheadBehind::default();
    if local == upstream {
        return Ok(counts);
    }
    let mut paint = Paint::default();
    paint.reach(nodes, local, LOCAL)?;
    paint.reach(nodes, upstream, UPSTREAM)?;

    while paint.unshared > 0 {
        let Some((_, id)) = paint.queue.pop() else {
            break;
        };
        let (flags, parents) = {
            let entry = paint.seen.get_mut(&id).ok_or(NodeError::Missing { id })?;
            entry.queued = false;
            (entry.flags, entry.node.parents.clone())
        };
        match flags {
            BOTH => {}
            LOCAL => {
                counts.ahead += 1;
                paint.unshared -= 1;
            }
            _ => {
                counts.behind += 1;
                paint.unshared -= 1;
            }
        }
        for parent in parents {
            paint.reach(nodes, parent, flags)?;
        }
    }
    Ok(counts)
}

/// One commit the walk has met.
struct Seen {
    node: Node,
    flags: u8,
    queued: bool,
}

/// The walk's state: what each commit is reached by, and what is left to
/// visit, newest first.
#[derive(Default)]
struct Paint {
    seen: HashMap<Oid, Seen>,
    queue: BinaryHeap<(i64, Oid)>,
    /// Queued commits not yet known to be reached by both sides. The walk is
    /// over when this reaches zero.
    unshared: usize,
}

impl Paint {
    /// Marks `id` as reached by `flags`, queueing it if that is news.
    fn reach(&mut self, nodes: &impl Nodes, id: Oid, flags: u8) -> Result<(), NodeError> {
        if let Some(entry) = self.seen.get_mut(&id) {
            let grown = entry.flags | flags;
            if grown == entry.flags {
                return Ok(());
            }
            if entry.queued {
                if grown == BOTH {
                    self.unshared -= 1;
                }
            } else {
                // Only reachable with skewed dates: a parent visited before
                // a child that also reaches it. Visit it again so what it
                // passes on to its own parents is right.
                entry.queued = true;
                self.queue.push((entry.node.time, id));
                if grown != BOTH {
                    self.unshared += 1;
                }
            }
            entry.flags = grown;
            return Ok(());
        }
        let node = nodes.node(id)?;
        self.queue.push((node.time, id));
        if flags != BOTH {
            self.unshared += 1;
        }
        self.seen.insert(
            id,
            Seen {
                node,
                flags,
                queued: true,
            },
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::history::node::MapNodes;

    fn oid(n: u8) -> Oid {
        Oid::from_bytes(&[n; Oid::SHA1_BYTES]).expect("20 bytes is a sha1")
    }

    /// A graph from `(commit, parents)` pairs; commit `n` is dated `n`, so
    /// higher numbers are newer.
    fn graph(edges: &[(u8, &[u8])]) -> MapNodes {
        MapNodes(
            edges
                .iter()
                .map(|&(id, parents)| {
                    (
                        oid(id),
                        Node {
                            id: oid(id),
                            parents: parents.iter().map(|&p| oid(p)).collect(),
                            time: i64::from(id),
                            generation: Node::UNKNOWN_GENERATION,
                        },
                    )
                })
                .collect::<HashMap<_, _>>(),
        )
    }

    fn counted(nodes: &MapNodes, local: u8, upstream: u8) -> AheadBehind {
        count(nodes, oid(local), oid(upstream)).expect("every commit is in the graph")
    }

    #[test]
    fn a_branch_level_with_its_upstream_is_in_sync() {
        let nodes = graph(&[(1, &[]), (2, &[1])]);
        assert!(counted(&nodes, 2, 2).in_sync());
    }

    #[test]
    fn ahead_only() {
        let nodes = graph(&[(1, &[]), (2, &[1]), (3, &[2]), (4, &[3])]);
        assert_eq!(
            counted(&nodes, 4, 2),
            AheadBehind {
                ahead: 2,
                behind: 0
            }
        );
    }

    #[test]
    fn behind_only() {
        let nodes = graph(&[(1, &[]), (2, &[1]), (3, &[2])]);
        assert_eq!(
            counted(&nodes, 1, 3),
            AheadBehind {
                ahead: 0,
                behind: 2
            }
        );
    }

    #[test]
    fn diverged() {
        // 1 ─ 2 ─ 4 ─ 6   local
        //      ╲
        //       3 ─ 5     upstream
        let nodes = graph(&[
            (1, &[]),
            (2, &[1]),
            (3, &[2]),
            (4, &[2]),
            (5, &[3]),
            (6, &[4]),
        ]);
        assert_eq!(
            counted(&nodes, 6, 5),
            AheadBehind {
                ahead: 2,
                behind: 2
            }
        );
    }

    #[test]
    fn a_merge_of_the_upstream_counts_only_what_is_new() {
        // local merged upstream's 3, then added 5; upstream is at 3.
        let nodes = graph(&[(1, &[]), (2, &[1]), (3, &[1]), (4, &[2, 3]), (5, &[4])]);
        assert_eq!(
            counted(&nodes, 5, 3),
            AheadBehind {
                ahead: 3,
                behind: 0
            }
        );
    }

    #[test]
    fn unrelated_histories_count_everything() {
        let nodes = graph(&[(1, &[]), (2, &[1]), (3, &[]), (4, &[3]), (5, &[4])]);
        assert_eq!(
            counted(&nodes, 2, 5),
            AheadBehind {
                ahead: 2,
                behind: 3
            }
        );
    }
}
