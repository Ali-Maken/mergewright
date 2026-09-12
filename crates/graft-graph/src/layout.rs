//! The lane-assignment algorithm: commits in, rows out.
//!
//! This is DESIGN.md §3.2 and §3.3 turned into code. A [`LayoutCursor`] holds
//! everything the algorithm needs to carry from one commit to the next — the
//! open lanes, which commit each is waiting for, and the pool of columns
//! freed and ready to be reused — so that laying out one more chunk of
//! commits after the last is exactly the same operation as laying out the
//! first chunk.

use std::collections::{HashMap, HashSet};

use graft_core::commit::Commit;
use graft_core::oid::Oid;

use crate::identity::BranchIdentity;
use crate::lane::Lane;
use crate::merge_name::recover_branch_name;
use crate::row::Row;
use crate::seeds::{BranchSeed, Seeds};

/// Lays out a whole commit sequence in one call.
///
/// A convenience over [`LayoutCursor`] for callers that already have every
/// commit in hand and do not need a resumable cursor.
#[must_use]
pub fn layout(commits: &[Commit], seeds: Seeds) -> Vec<Row> {
    LayoutCursor::new(seeds).extend(commits)
}

/// The state carried from one row to the next while laying out a graph.
///
/// Everything here is a point lookup or an append — never an iteration over
/// a whole map — which is what keeps the algorithm's own bookkeeping free of
/// the hashing-order nondeterminism that would otherwise make two runs over
/// the same commits disagree.
#[derive(Debug, Default)]
pub struct LayoutCursor {
    /// Every open lane, indexed by column. `None` is a column nothing
    /// currently occupies: either never allocated past this point, or
    /// closed and not yet (or never, if reserved) returned to the pool.
    active: Vec<Option<Lane>>,
    /// For each commit still to come, the lanes waiting for it to continue
    /// them.
    waiting: HashMap<Oid, Vec<usize>>,
    /// Columns closed on an earlier row and free to be reused, starting from
    /// the row after the one that freed them.
    free_pool: Vec<usize>,
    /// Every commit already emitted as a row, so a lane whose target was
    /// already drawn (a late-arriving parent under `Order::CommitDate`, or
    /// clock-skewed history) can be closed immediately instead of left
    /// waiting for a commit that will never come.
    already_emitted: HashSet<Oid>,
    /// Known tips, for colour stability when an ordinary lane opens fresh.
    known_identities: HashMap<Oid, BranchIdentity>,
}

impl LayoutCursor {
    /// A cursor seeded with lane 0 and the reserved integration lanes,
    /// ready to lay out the first commit.
    #[must_use]
    pub fn new(seeds: Seeds) -> Self {
        let mut cursor = LayoutCursor::default();

        if let Some(seed) = seeds.default_branch {
            cursor.reserve_lane(0, seed);
        }
        for (offset, seed) in seeds.integration_branches.into_iter().enumerate() {
            cursor.reserve_lane(1 + offset, seed);
        }
        for seed in seeds.known_branches {
            cursor.known_identities.insert(seed.tip, seed.identity);
        }

        cursor
    }

    /// Lays out the next batch of commits, continuing from wherever the last
    /// call left off.
    ///
    /// `commits` must be in the same walk order the cursor has already seen
    /// — the order DESIGN.md §3.2 assumes throughout: a parent is laid out
    /// no earlier than its child.
    #[must_use]
    pub fn extend(&mut self, commits: &[Commit]) -> Vec<Row> {
        commits
            .iter()
            .map(|commit| self.assign_row(commit))
            .collect()
    }

    /// Every column's current lane, `None` where nothing is open.
    ///
    /// `Row` carries a lane *index* and nothing else (DESIGN.md §3.1's data
    /// model), so this is how a caller reaches the identity a currently open
    /// lane draws in — e.g. to colour the row it just received. It reports
    /// only the *current* state: once a column is reused for a different
    /// lane, its earlier occupant's identity is no longer reachable here, so
    /// a caller that needs a row's colour after the fact (a cache, a
    /// scrollback) must capture it at the time the row was produced.
    #[must_use]
    pub fn active_lanes(&self) -> &[Option<Lane>] {
        &self.active
    }

    /// Registers a reserved lane at `column`, waiting for `seed.tip`.
    fn reserve_lane(&mut self, column: usize, seed: BranchSeed) {
        while self.active.len() <= column {
            self.active.push(None);
        }
        self.active[column] = Some(Lane {
            index: column,
            identity: seed.identity,
            waiting_for: seed.tip,
            reserved: true,
        });
        self.register_waiting(column, seed.tip);
    }

    /// Assigns one commit to a lane, records its edges, and advances every
    /// lane its parents continue or open.
    fn assign_row(&mut self, commit: &Commit) -> Row {
        let found = self.waiting.remove(&commit.id).unwrap_or_default();
        let mut parents = commit.parents.iter().copied();
        let first_parent = parents.next();

        let lane = winning_lane(&found).unwrap_or_else(|| self.open_new_lane(commit, first_parent));

        let mut closing = Vec::new();
        let mut incoming: Vec<usize> = found
            .into_iter()
            .filter(|&candidate| candidate != lane)
            .collect();
        for &candidate in &incoming {
            self.close_lane(candidate, &mut closing);
        }
        incoming.sort_unstable();

        let mut outgoing = Vec::new();
        match first_parent {
            Some(parent) => self.continue_lane(lane, parent, &mut outgoing, &mut closing),
            None => self.close_lane(lane, &mut closing),
        }
        for parent in parents {
            self.open_merge_lane(commit, parent, &mut outgoing, &mut closing);
        }

        self.already_emitted.insert(commit.id);
        // Row granularity, not the chunk granularity DESIGN.md §3.3's
        // pseudocode sketches: promoting only after a whole chunk would make
        // the layout depend on the chunk size, which contradicts §3.6's
        // "checkpoints are pure functions of the commit sequence up to that
        // point" — a checkpoint taken mid-chunk would freeze a different
        // free pool than one taken between chunks, for the same commits.
        self.free_pool.extend(closing);

        Row {
            commit: commit.clone(),
            lane,
            incoming,
            outgoing,
            marker: None,
        }
    }

    /// Advances `lane` to wait for `parent`, or closes it immediately if
    /// `parent` was already emitted. Either way the edge is recorded: an
    /// edge that closes into an already-drawn commit is still a real edge.
    fn continue_lane(
        &mut self,
        lane: usize,
        parent: Oid,
        outgoing: &mut Vec<(usize, Oid)>,
        closing: &mut Vec<usize>,
    ) {
        outgoing.push((lane, parent));
        if self.already_emitted.contains(&parent) {
            self.close_lane(lane, closing);
            return;
        }
        if let Some(entry) = &mut self.active[lane] {
            entry.waiting_for = parent;
        }
        self.register_waiting(lane, parent);
    }

    /// Opens a new lane for a merge parent (the second and any later parent
    /// of a merge commit — handled the same way whether there are one or
    /// several of them, so an octopus merge needs no special case).
    ///
    /// The merged branch's identity is recovered from the merge message
    /// (DESIGN.md §3.4); a `known_branches` lookup is deliberately not
    /// consulted here the way it is in [`open_new_lane`](Self::open_new_lane):
    /// a merge parent is a historical ancestor at merge time, essentially
    /// never equal to any branch's *current* tip, so the message (or its
    /// short-SHA fallback) is the only signal that can fire.
    fn open_merge_lane(
        &mut self,
        commit: &Commit,
        parent: Oid,
        outgoing: &mut Vec<(usize, Oid)>,
        closing: &mut Vec<usize>,
    ) {
        let lane = self.allocate_column();
        let identity = match recover_branch_name(commit.subject()) {
            Some(name) => BranchIdentity::of_name(&name),
            None => BranchIdentity::of_short_sha(parent),
        };
        self.active[lane] = Some(Lane {
            index: lane,
            identity,
            waiting_for: parent,
            reserved: false,
        });
        outgoing.push((lane, parent));
        if self.already_emitted.contains(&parent) {
            self.close_lane(lane, closing);
            return;
        }
        self.register_waiting(lane, parent);
    }

    /// Opens a lane for a commit nothing was waiting for: a fresh tip.
    ///
    /// `waiting_for` is set from `first_parent` when there is one; a root
    /// commit that is also a fresh tip has no parent to wait for; either way
    /// the caller overwrites or closes the lane immediately afterward, so
    /// this never has to invent a value that would otherwise be observed.
    fn open_new_lane(&mut self, commit: &Commit, first_parent: Option<Oid>) -> usize {
        let lane = self.allocate_column();
        let identity = self
            .known_identities
            .get(&commit.id)
            .copied()
            .unwrap_or_else(|| BranchIdentity::of_short_sha(commit.id));
        self.active[lane] = Some(Lane {
            index: lane,
            identity,
            waiting_for: first_parent.unwrap_or(commit.id),
            reserved: false,
        });
        lane
    }

    /// Takes the lowest-indexed free column, or opens a new one past every
    /// column allocated so far.
    fn allocate_column(&mut self) -> usize {
        if let Some(position) = self
            .free_pool
            .iter()
            .enumerate()
            .min_by_key(|&(_, &column)| column)
            .map(|(position, _)| position)
        {
            return self.free_pool.remove(position);
        }
        let lane = self.active.len();
        self.active.push(None);
        lane
    }

    /// Closes `lane`: clears it from `active`, and — unless it is reserved —
    /// queues its column to be reused starting on the *next* row.
    ///
    /// One guard covers every way a lane closes: reaching a root commit,
    /// losing a convergence to a lower-indexed lane, and a target that
    /// turned out to already be drawn. A reserved lane never enters the
    /// pool through any of them — DESIGN.md §3.1's "never handed to the free
    /// pool" is unconditional, not scoped to the root-commit case alone: an
    /// integration branch's chain reconverging with the default branch's at
    /// their historical fork point must leave that column permanently
    /// retired, exactly like reaching a root.
    fn close_lane(&mut self, lane: usize, closing: &mut Vec<usize>) {
        let reserved = self.active[lane]
            .as_ref()
            .is_some_and(|entry| entry.reserved);
        self.active[lane] = None;
        if !reserved {
            closing.push(lane);
        }
    }

    /// Records that `lane` is waiting for `target` to continue it.
    fn register_waiting(&mut self, lane: usize, target: Oid) {
        self.waiting.entry(target).or_default().push(lane);
    }
}

/// Which lane a row belongs to, among the lanes waiting for it: lane 0 if it
/// is one of them, else the lowest-indexed lane (DESIGN.md §3.2 step 3).
fn winning_lane(incoming: &[usize]) -> Option<usize> {
    if incoming.contains(&0) {
        Some(0)
    } else {
        incoming.iter().copied().min()
    }
}

#[cfg(test)]
mod tests;
