//! The lane-assignment algorithm: links in, row shapes out.
//!
//! This is DESIGN.md §3.2 and §3.3 turned into code. A [`LayoutCursor`] holds
//! everything the algorithm needs to carry from one commit to the next — the
//! open lanes, which commit each is waiting for, and the pool of columns
//! freed and ready to be reused — so that laying out one more chunk of
//! commits after the last is exactly the same operation as laying out the
//! first chunk.
//!
//! The cursor is deliberately small — bounded by how many lanes are open at
//! once, never by how much history lies above — because a checkpoint
//! (DESIGN.md §3.6) is nothing more than a clone of it. What does grow with
//! history, the list of commits laid out so far, lives in a single
//! [`EmittedOrder`] shared by the cursor and every checkpoint taken from it.

mod assign_row;
mod cache;
mod checkpoint;
pub mod order;
pub mod source;
pub mod stream;

use std::collections::HashMap;
use std::sync::Arc;

use graft_core::commit::Commit;
use graft_core::commit_handle::{CommitHandle, Link};
use graft_core::oid::Oid;

pub use self::order::EmittedOrder;
pub use self::source::{CommitSource, Pull};
pub use self::stream::{
    Advance, Anchor, Invalidated, Resident, StreamConfig, StreamError, StreamingLayout, Work,
};
use crate::identity::BranchIdentity;
use crate::lane::Lane;
use crate::row::{MergeNames, Row, RowShape};
use crate::seeds::Seeds;

/// Resolves a commit's id to its handle (ADR-0026).
///
/// `LayoutCursor` never needs the rest of [`CommitSource`] — no pull, no
/// read, no links — only this one lookup, so it asks for this instead of the
/// whole trait. Every [`CommitSource`] is one (below), so a real walk's
/// source works here unchanged; [`layout`]'s convenience path, which has no
/// source at all, builds a throwaway one straight from the commits in hand.
pub trait HandleLookup {
    /// The handle for `id`. Every id this is asked for was itself read from
    /// the same walk this resolver was built over, so this cannot fail; an
    /// id this resolver has never seen is a bug and panics.
    fn handle_of(&self, id: Oid) -> CommitHandle;

    /// The id behind `handle`. Handles come only from this resolver's
    /// table, so this cannot fail; a foreign handle is a bug and panics.
    fn id_of(&self, handle: CommitHandle) -> Oid;
}

impl<S: CommitSource> HandleLookup for S {
    fn handle_of(&self, id: Oid) -> CommitHandle {
        CommitSource::handle_of(self, &id)
            .unwrap_or_else(|| panic!("{id:?} has no handle in this source's table"))
    }

    fn id_of(&self, handle: CommitHandle) -> Oid {
        CommitSource::id_of(self, handle)
    }
}

/// A dense id-to-handle table built from a commit list already in hand.
///
/// Backs [`layout`]'s convenience path: assigns handles in first-seen order
/// over every id and parent, exactly as a repository's own table would for
/// the same commits, but privately and only for the lifetime of one call —
/// nothing outside `layout` ever sees these handles.
/// A table's next handle, as a `u32` — no repository (or one call's worth of
/// commits) holds anywhere near 4 billion, so a table this size is itself
/// the bug.
fn handle_index(len: usize) -> u32 {
    u32::try_from(len).unwrap_or_else(|_| panic!("more than u32::MAX commits in one table"))
}

struct LocalHandles {
    by_id: HashMap<Oid, CommitHandle>,
    ids: Vec<Oid>,
}

impl LocalHandles {
    fn of(commits: &[Commit]) -> Self {
        let mut table = LocalHandles {
            by_id: HashMap::new(),
            ids: Vec::new(),
        };
        for commit in commits {
            for &id in std::iter::once(&commit.id).chain(&commit.parents) {
                if !table.by_id.contains_key(&id) {
                    table
                        .by_id
                        .insert(id, CommitHandle(handle_index(table.ids.len())));
                    table.ids.push(id);
                }
            }
        }
        table
    }
}

impl HandleLookup for LocalHandles {
    fn handle_of(&self, id: Oid) -> CommitHandle {
        *self
            .by_id
            .get(&id)
            .unwrap_or_else(|| panic!("{id:?} was never seen in this layout's commits"))
    }

    fn id_of(&self, handle: CommitHandle) -> Oid {
        self.ids[handle.index()]
    }
}

/// The links of `commits`, parents as each commit holds them, by handle.
pub(crate) fn links_of(commits: &[Commit], handles: &impl HandleLookup) -> Vec<Link> {
    commits
        .iter()
        .map(|commit| Link {
            commit: handles.handle_of(commit.id),
            parents: commit
                .parents
                .iter()
                .map(|&parent| handles.handle_of(parent))
                .collect(),
        })
        .collect()
}

/// Lays out a whole commit sequence in one call.
///
/// A convenience over [`LayoutCursor`] for callers that already have every
/// commit in hand and do not need a resumable cursor.
///
/// # Panics
///
/// If `commits` draws the same commit twice; see [`LayoutCursor::extend`].
#[must_use]
pub fn layout(commits: &[Commit], seeds: Seeds) -> Vec<Row> {
    let handles = LocalHandles::of(commits);
    let mut order = EmittedOrder::new();
    let shapes = LayoutCursor::new(seeds, &handles).extend(
        &links_of(commits, &handles),
        &handles,
        &mut order,
    );
    // Every merge is one of `commits`, on hand: the whole history is one
    // batch, so no lane opened above it and `gather` never reads.
    let names = MergeNames::gather(
        &shapes,
        commits,
        |merges| -> Result<Vec<Commit>, std::convert::Infallible> {
            unreachable!("a whole-slice layout has every merge on hand, asked for {merges:?}")
        },
        |handle| handles.id_of(handle),
    )
    .unwrap_or_else(|never| match never {});
    shapes
        .into_iter()
        .zip(commits)
        .map(|(shape, commit)| {
            // Cannot fail: `names` was gathered over these very shapes.
            Row::assemble(shape, commit.clone(), &names, &|handle| {
                handles.id_of(handle)
            })
            .unwrap_or_else(|err| unreachable!("gathered over the same shapes: {err}"))
        })
        .collect()
}

/// The state carried from one row to the next while laying out a graph.
///
/// Everything here is a point lookup or an append — never an iteration over
/// a whole map — which is what keeps the algorithm's own bookkeeping free of
/// the hashing-order nondeterminism that would otherwise make two runs over
/// the same commits disagree.
///
/// Cloning a cursor is how a checkpoint is taken (DESIGN.md §3.6). The clone
/// costs what the open lanes cost: the seeds' identity table is shared, not
/// copied, and nothing here remembers the rows already emitted beyond
/// counting them.
#[derive(Clone, Debug, Default)]
pub struct LayoutCursor {
    /// Every open lane, indexed by column. `None` is a column nothing
    /// currently occupies: either never allocated past this point, or
    /// closed and not yet (or never, if reserved) returned to the pool.
    active: Vec<Option<Lane>>,
    /// For each commit still to come, the lanes waiting for it to continue
    /// them.
    waiting: HashMap<CommitHandle, Vec<usize>>,
    /// Columns closed on an earlier row and free to be reused, starting from
    /// the row after the one that freed them.
    free_pool: Vec<usize>,
    /// How many rows this cursor has laid out: the row the next commit lands
    /// on. Together with an [`EmittedOrder`] it tells whether a lane's
    /// target was already drawn (a late-arriving parent under
    /// `Order::CommitDate`, or clock-skewed history) so the lane can close
    /// at once instead of waiting for a commit that will never come.
    rows_emitted: usize,
    /// Known tips, for colour stability when an ordinary lane opens fresh.
    /// Fixed once the cursor is built, so clones share one table. Keyed by
    /// id, not handle: a known tip need not be in the resolver's table, and
    /// a fresh tip's id is resolved anyway for its short-sha fallback.
    known_identities: Arc<HashMap<Oid, BranchIdentity>>,
    /// Reserved columns whose tip no row has landed on yet. Such a lane is
    /// open, but nothing is drawn above its tip, so rows leave it out of
    /// [`Row::lanes`]. At most one entry per reserved lane.
    unreached: Vec<usize>,
}

impl LayoutCursor {
    /// A cursor seeded with lane 0 and the reserved integration lanes,
    /// ready to lay out the first commit.
    #[must_use]
    pub fn new(seeds: Seeds, handles: &impl HandleLookup) -> Self {
        let mut cursor = LayoutCursor::default();

        if let Some(seed) = seeds.default_branch {
            cursor.reserve_lane(0, seed, handles);
        }
        for (offset, seed) in seeds.integration_branches.into_iter().enumerate() {
            cursor.reserve_lane(1 + offset, seed, handles);
        }
        cursor.known_identities = Arc::new(
            seeds
                .known_branches
                .into_iter()
                .map(|seed| (seed.tip, seed.identity))
                .collect(),
        );

        cursor
    }

    /// Lays out the next batch of links, continuing from wherever the last
    /// call left off, and records each in `order` at the row it lands on.
    /// Reads nothing: merge lanes carry a deferred
    /// [`crate::lane::LaneIdentity::Merge`], named at [`Row::assemble`].
    ///
    /// `commits` must be in the same walk order the cursor has already seen
    /// — the order DESIGN.md §3.2 assumes throughout: a parent is laid out
    /// no earlier than its child. `order` must be the one every earlier call
    /// on this cursor (or on the cursor it was cloned from) recorded into.
    ///
    /// # Panics
    ///
    /// If a commit is laid out twice, or if `order` already holds a
    /// different commit at the row being laid out — see
    /// [`EmittedOrder::note`].
    #[must_use]
    pub fn extend(
        &mut self,
        links: &[Link],
        handles: &impl HandleLookup,
        order: &mut EmittedOrder,
    ) -> Vec<RowShape> {
        order.reserve(links.len());
        links
            .iter()
            .map(|link| {
                order
                    .note(link.commit)
                    .unwrap_or_else(|err| panic!("{err}"));
                let row = self.assign_row(link, handles, order);
                self.rows_emitted += 1;
                row
            })
            .collect()
    }

    /// How many rows this cursor has laid out so far.
    #[must_use]
    pub fn rows_emitted(&self) -> usize {
        self.rows_emitted
    }

    /// Roughly how much memory a checkpoint made from this cursor costs,
    /// counting reserved capacity rather than only what is in use. The
    /// shared identity table is not counted: it exists once however many
    /// checkpoints there are.
    #[must_use]
    pub fn approximate_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.active.capacity() * std::mem::size_of::<Option<Lane>>()
            + self.free_pool.capacity() * std::mem::size_of::<usize>()
            + self.unreached.capacity() * std::mem::size_of::<usize>()
            + self.waiting.capacity()
                * (std::mem::size_of::<CommitHandle>() + std::mem::size_of::<Vec<usize>>())
            + self
                .waiting
                .values()
                .map(|lanes| lanes.capacity() * std::mem::size_of::<usize>())
                .sum::<usize>()
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
}

#[cfg(test)]
pub(crate) mod tests;
