//! Lane-assignment internals for one row: which lane a commit lands in,
//! which edges it opens and closes, and where each column ends up.
//!
//! Split out of [`super::LayoutCursor`]'s own file (m1-the-graph/75b) so the
//! cursor's public, checkpoint-facing API and this row-by-row bookkeeping
//! read as separate concerns; the type and its invariants are unchanged.

use mergewright_core::commit_handle::{CommitHandle, Link};

use crate::identity::BranchIdentity;
use crate::lane::{Lane, LaneIdentity};
use crate::row::RowShape;
use crate::seeds::BranchSeed;

use super::{EmittedOrder, HandleLookup, LayoutCursor};

impl LayoutCursor {
    /// Registers a reserved lane at `column`, waiting for `seed.tip`.
    pub(super) fn reserve_lane(
        &mut self,
        column: usize,
        seed: BranchSeed,
        handles: &impl HandleLookup,
    ) {
        while self.active.len() <= column {
            self.active.push(None);
        }
        let waiting_for = handles.handle_of(seed.tip);
        self.active[column] = Some(Lane {
            index: column,
            identity: LaneIdentity::Known(seed.identity),
            waiting_for,
            reserved: true,
        });
        self.register_waiting(column, waiting_for);
        self.unreached.push(column);
    }

    /// Assigns one commit to a lane, records its edges, and advances every
    /// lane its parents continue or open.
    pub(super) fn assign_row(
        &mut self,
        link: &Link,
        handles: &impl HandleLookup,
        order: &EmittedOrder,
    ) -> RowShape {
        let found = self.waiting.remove(&link.commit).unwrap_or_default();
        let mut parents = link.parents.iter();
        let first_parent = parents.next();

        // What is drawn above this row, before any lane closes or opens.
        let mut lanes: Vec<Option<LaneIdentity>> = self
            .active
            .iter()
            .enumerate()
            .map(|(column, entry)| {
                entry
                    .as_ref()
                    .filter(|_| !self.unreached.contains(&column))
                    .map(|entry| entry.identity)
            })
            .collect();

        let lane = self.lane_of_row(&found, link.commit, first_parent, handles);
        self.unreached.retain(|&column| column != lane);
        // Read before anything closes: a root that is also a fresh tip opens
        // and closes its lane on this one row.
        let own = self.active[lane].as_ref().map(|entry| entry.identity);
        set_column(&mut lanes, lane, own);

        // Every other lane waiting for this commit closes here. Only a lane
        // drawn above this row bends in: a reserved lane whose tip is this
        // very commit was never drawn (it is not in `lanes`), so it closes
        // without an edge — else the row names a column with nothing above
        // it (m1-the-graph/68).
        let mut closing = Vec::new();
        let mut incoming = Vec::new();
        for candidate in found.into_iter().filter(|&candidate| candidate != lane) {
            self.close_lane(candidate, &mut closing);
            if self.unreached.contains(&candidate) {
                self.unreached.retain(|&column| column != candidate);
            } else {
                incoming.push(candidate);
            }
        }
        incoming.sort_unstable();

        let mut outgoing = Vec::new();
        match first_parent {
            Some(parent) => {
                self.continue_lane(lane, parent, order, &mut outgoing, &mut closing);
            }
            None => self.close_lane(lane, &mut closing),
        }
        for parent in parents {
            let (column, identity) = self.open_merge_lane(
                link.commit,
                parent,
                handles,
                order,
                &mut outgoing,
                &mut closing,
            );
            set_column(&mut lanes, column, Some(identity));
        }
        while lanes.last().is_some_and(Option::is_none) {
            lanes.pop();
        }

        // Row granularity, not the chunk granularity DESIGN.md §3.3's
        // pseudocode sketches: promoting only after a whole chunk would make
        // the layout depend on the chunk size, which contradicts §3.6's
        // "checkpoints are pure functions of the commit sequence up to that
        // point" — a checkpoint taken mid-chunk would freeze a different
        // free pool than one taken between chunks, for the same commits.
        self.free_pool.extend(closing);

        RowShape {
            commit: link.commit,
            lane,
            incoming,
            outgoing,
            lanes,
        }
    }

    /// Advances `lane` to wait for `parent`, or closes it immediately if
    /// `parent` was already emitted. Either way the edge is recorded: an
    /// edge that closes into an already-drawn commit is still a real edge.
    fn continue_lane(
        &mut self,
        lane: usize,
        target: CommitHandle,
        order: &EmittedOrder,
        outgoing: &mut Vec<(usize, CommitHandle)>,
        closing: &mut Vec<usize>,
    ) {
        outgoing.push((lane, target));
        if self.drawn_already(target, order) {
            self.close_lane(lane, closing);
            return;
        }
        if let Some(entry) = &mut self.active[lane] {
            entry.waiting_for = target;
        }
        self.register_waiting(lane, target);
    }

    /// Opens a new lane for a merge parent (the second and any later parent
    /// of a merge commit — handled the same way whether there are one or
    /// several of them, so an octopus merge needs no special case).
    ///
    /// The merged branch's identity is recovered from the merge message
    /// (DESIGN.md §3.4) — at assembly, through [`LaneIdentity::Merge`]: the
    /// message needs a commit read and the layout reads nothing. A
    /// `known_branches` lookup is deliberately not
    /// consulted here the way it is in [`open_new_lane`](Self::open_new_lane):
    /// a merge parent is a historical ancestor at merge time, essentially
    /// never equal to any branch's *current* tip, so the message (or its
    /// short-SHA fallback) is the only signal that can fire.
    ///
    /// Returns the column opened and the identity it draws in, which stay
    /// observable to the row even if the lane closes again at once.
    fn open_merge_lane(
        &mut self,
        merge: CommitHandle,
        target: CommitHandle,
        handles: &impl HandleLookup,
        order: &EmittedOrder,
        outgoing: &mut Vec<(usize, CommitHandle)>,
        closing: &mut Vec<usize>,
    ) -> (usize, LaneIdentity) {
        let lane = self.allocate_column();
        // A live branch at the parent beats a name recovered from the merge
        // subject (ADR-0064); otherwise the name is recovered at assembly.
        let identity = match self.known_identities.get(&handles.id_of(target)) {
            Some(tip) => LaneIdentity::Known(tip.lane_name()),
            None => LaneIdentity::Merge {
                merge,
                parent: target,
            },
        };
        self.active[lane] = Some(Lane {
            index: lane,
            identity,
            waiting_for: target,
            reserved: false,
        });
        outgoing.push((lane, target));
        if self.drawn_already(target, order) {
            self.close_lane(lane, closing);
        } else {
            self.register_waiting(lane, target);
        }
        (lane, identity)
    }

    /// Whether `handle` sits on a row above the one being laid out.
    ///
    /// A replay from a checkpoint sees an `order` that already runs past the
    /// cursor's own position; a commit recorded *below* the cursor is one the
    /// replay has yet to reach, exactly as it was for the original run.
    fn drawn_already(&self, handle: CommitHandle, order: &EmittedOrder) -> bool {
        order
            .row_of(handle)
            .is_some_and(|row| row < self.rows_emitted)
    }

    /// Opens a lane for a commit nothing was waiting for: a fresh tip.
    ///
    /// `waiting_for` is set from `first_parent` when there is one; a root
    /// commit that is also a fresh tip has no parent to wait for; either way
    /// the caller overwrites or closes the lane immediately afterward, so
    /// this never has to invent a value that would otherwise be observed.
    fn open_new_lane(
        &mut self,
        commit: CommitHandle,
        first_parent: Option<CommitHandle>,
        handles: &impl HandleLookup,
    ) -> usize {
        let lane = self.allocate_column();
        let id = handles.id_of(commit);
        let identity = self
            .known_identities
            .get(&id)
            .map_or_else(|| BranchIdentity::of_short_sha(id), |seed| seed.identity);
        let waiting_for = first_parent.unwrap_or(commit);
        self.active[lane] = Some(Lane {
            index: lane,
            identity: LaneIdentity::Known(identity),
            waiting_for,
            reserved: false,
        });
        lane
    }

    /// The lane `commit` lands in: the winner among the lanes waiting for it,
    /// which takes the name of `commit`'s branch if it is another branch's
    /// tip (ADR-0064); or a fresh one if nothing waited.
    fn lane_of_row(
        &mut self,
        found: &[usize],
        commit: CommitHandle,
        first_parent: Option<CommitHandle>,
        handles: &impl HandleLookup,
    ) -> usize {
        let Some(lane) = winning_lane(found) else {
            return self.open_new_lane(commit, first_parent, handles);
        };
        let tip = self.known_identities.get(&handles.id_of(commit));
        if let Some(entry) = &mut self.active[lane]
            && let Some(identity) = renamed_at_tip(entry, tip)
        {
            entry.identity = identity;
        }
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
    fn register_waiting(&mut self, lane: usize, target: CommitHandle) {
        self.waiting.entry(target).or_default().push(lane);
    }
}

/// Records `identity` as what `column` draws on a row, growing the row's
/// column list as far as needed.
fn set_column(
    lanes: &mut Vec<Option<LaneIdentity>>,
    column: usize,
    identity: Option<LaneIdentity>,
) {
    if lanes.len() <= column {
        lanes.resize(column + 1, None);
    }
    lanes[column] = identity;
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

/// The name a lane carries from this row down when it reaches `tip`, or
/// `None` to keep its own (ADR-0064).
///
/// `None` when the lane is reserved, there is no tip here, the tip's
/// identity is the lane's, or the tip is the remote copy of the lane's
/// branch. A [`LaneIdentity::Merge`] lane is renamed by any tip: its name is
/// only recovered at assembly.
pub(crate) fn renamed_at_tip(lane: &Lane, tip: Option<&BranchSeed>) -> Option<LaneIdentity> {
    let tip = tip?;
    if lane.reserved {
        return None;
    }
    if let LaneIdentity::Known(own) = lane.identity
        && (own == tip.identity || tip.follows == Some(own))
    {
        return None;
    }
    Some(LaneIdentity::Known(tip.lane_name()))
}
