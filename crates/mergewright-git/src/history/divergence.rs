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

use std::sync::Arc;

use mergewright_core::commit_handle::CommitHandle;
use mergewright_core::oid::Oid;

use super::node::{Node, NodeError, Nodes, RepoNodes};
use super::{CommitTable, WalkError};
use crate::repo::RepoHandle;
use crate::shallow::ShallowList;

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
    // Checked before opening the commit graph, not only inside `count`: most
    // branches sit on their upstream, and opening it for each of 25k such
    // branches cost ~0.6 s of a 50k-ref read in debug (m1-the-graph/30d).
    if local == upstream {
        return Ok(AheadBehind::default());
    }
    let shallow = Arc::new(ShallowList::read(handle)?);
    let table =
        CommitTable::open_with(handle, &shallow).map_err(|source| WalkError::CommitGraph {
            source: Box::new(source),
        })?;
    let nodes = RepoNodes::new(handle.gix(), Arc::new(table), shallow);
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
    paint.reach(nodes, nodes.handle(&local), LOCAL)?;
    paint.reach(nodes, nodes.handle(&upstream), UPSTREAM)?;

    // The oldest date of a commit popped as one side's only. A shared
    // commit still queued at that date may be its child (commits made in
    // one second share a date; only the id orders them), so the walk goes
    // on until the queue is older than it: the mark that child passes down
    // can still make it shared (m8-polish-and-ship/14).
    let mut one_sided_floor = i64::MAX;
    loop {
        let settled = paint.unshared == 0
            && paint
                .queue
                .peek()
                .is_none_or(|&(time, _, _)| time < one_sided_floor);
        if settled {
            break;
        }
        let Some((time, id, handle)) = paint.queue.pop() else {
            break;
        };
        let (flags, parents) = {
            let entry = paint
                .seen
                .get_mut(&handle)
                .ok_or(NodeError::Missing { id })?;
            entry.queued = false;
            (entry.flags, entry.node.parents.clone())
        };
        if flags != BOTH {
            paint.unshared -= 1;
            one_sided_floor = one_sided_floor.min(time);
        }
        for parent in parents.iter() {
            paint.reach(nodes, parent, flags)?;
        }
    }
    // Counted once the marks have settled, not at each pop: a commit popped
    // as one side's only can turn out shared later (m8-polish-and-ship/14).
    for entry in paint.seen.values() {
        match entry.flags {
            LOCAL => counts.ahead += 1,
            UPSTREAM => counts.behind += 1,
            _ => {}
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
    seen: HashMap<CommitHandle, Seen>,
    /// Date, then id (the tie-break), then the handle the id has.
    queue: BinaryHeap<(i64, Oid, CommitHandle)>,
    /// Queued commits not yet known to be reached by both sides. The walk is
    /// over when this reaches zero.
    unshared: usize,
}

impl Paint {
    /// Marks `id` as reached by `flags`, queueing it if that is news.
    fn reach(
        &mut self,
        nodes: &impl Nodes,
        handle: CommitHandle,
        flags: u8,
    ) -> Result<(), NodeError> {
        if let Some(entry) = self.seen.get_mut(&handle) {
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
                self.queue.push((entry.node.time, nodes.id(handle), handle));
                if grown != BOTH {
                    self.unshared += 1;
                }
            }
            entry.flags = grown;
            return Ok(());
        }
        let node = nodes.node(handle)?;
        self.queue.push((node.time, nodes.id(handle), handle));
        if flags != BOTH {
            self.unshared += 1;
        }
        self.seen.insert(
            handle,
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
        MapNodes::new(
            edges
                .iter()
                .map(|&(id, parents)| {
                    (
                        oid(id),
                        (
                            parents.iter().map(|&p| oid(p)).collect(),
                            i64::from(id),
                            Node::UNKNOWN_GENERATION,
                        ),
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

    /// [`graph`] with every commit at the same date, as a scripted series
    /// of commits made inside one second has: only the id breaks ties.
    fn flat(edges: &[(u8, &[u8])]) -> MapNodes {
        MapNodes::new(
            edges
                .iter()
                .map(|&(id, parents)| {
                    (
                        oid(id),
                        (
                            parents.iter().map(|&p| oid(p)).collect(),
                            0,
                            Node::UNKNOWN_GENERATION,
                        ),
                    )
                })
                .collect::<HashMap<_, _>>(),
        )
    }

    /// The answer by brute force: every commit each side reaches.
    fn truth(edges: &[(u8, &[u8])], local: u8, upstream: u8) -> AheadBehind {
        let parents: HashMap<u8, &[u8]> = edges.iter().copied().collect();
        let reach = |tip: u8| {
            let mut seen = std::collections::HashSet::new();
            let mut stack = vec![tip];
            while let Some(c) = stack.pop() {
                if seen.insert(c) {
                    stack.extend(parents[&c].iter().copied());
                }
            }
            seen
        };
        let (l, u) = (reach(local), reach(upstream));
        AheadBehind {
            ahead: l.difference(&u).count(),
            behind: u.difference(&l).count(),
        }
    }

    /// Every pair of commits in `edges`, counted on one date, against the
    /// brute-force answer.
    fn assert_flat_matches_truth(edges: &[(u8, &[u8])]) {
        let nodes = flat(edges);
        for &(a, _) in edges {
            for &(b, _) in edges {
                assert_eq!(
                    counted(&nodes, a, b),
                    truth(edges, a, b),
                    "{a}...{b} on one date"
                );
            }
        }
    }

    #[test]
    fn one_date_a_parent_with_the_higher_id_is_not_counted_twice() {
        // `main ↑6 ↓1` where rev-list said `1 1` (m8/14): on one date 200
        // pops before its child 9, as local's only, before 9 passes
        // upstream's mark down to it.
        //   1 ─ 200 ─ 9   local
        //         ╲
        //          3      upstream
        let edges: &[(u8, &[u8])] = &[(1, &[]), (200, &[1]), (9, &[200]), (3, &[200])];
        assert_eq!(
            counted(&flat(edges), 9, 3),
            AheadBehind {
                ahead: 1,
                behind: 1
            }
        );
    }

    #[test]
    fn one_date_chains_and_forks_match_brute_force() {
        assert_flat_matches_truth(&[
            (10, &[]),
            (250, &[10]),
            (7, &[250]),
            (180, &[7]),
            (3, &[180]),
            (90, &[180]),
            (240, &[90]),
            (5, &[3]),
        ]);
    }

    #[test]
    fn one_date_merges_match_brute_force() {
        // A merge of the upstream into local, and a criss-cross.
        assert_flat_matches_truth(&[
            (100, &[]),
            (2, &[100]),
            (200, &[100]),
            (30, &[2, 200]),
            (230, &[200, 2]),
            (4, &[30]),
            (240, &[230, 4]),
            (6, &[240]),
        ]);
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
