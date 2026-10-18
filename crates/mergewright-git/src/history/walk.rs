//! The walk itself: [`Order`], the batch types it hands back, and [`Walk`],
//! which drives one of the three [`super::simple`]/[`super::topological`]
//! sequences a chunk at a time.

use std::sync::Arc;

use mergewright_core::commit::Commit;
use mergewright_core::commit_handle::{CommitHandle, Link, Parents};
use mergewright_core::oid::Oid;

use super::node::RepoNodes;
use super::read::{WalkError, handles, hydrate};
use super::simple::{Dated, Follow};
use super::sorted::Sorted;
use super::table::CommitTable;
use super::topological::Topological;
use crate::repo::RepoHandle;
use crate::shallow::ShallowList;

/// How many commits a chunk holds.
///
/// Small enough that the first one is on screen long before the walk is
/// finished, large enough that the per-chunk overhead — a cancellation
/// check, a channel send, a redraw — is amortised over real work.
pub const CHUNK: usize = 500;

/// The order a walk emits commits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// Every commit after all of its children: the order the graph is drawn
    /// in. Needs a commit-graph file to stream; see [`Walk::order_in_force`]
    /// for what happens when there is not one.
    Topological,
    /// Most recent commit first. Always available, and not necessarily a
    /// topological order: a commit can be dated before a commit it is
    /// descended from.
    CommitDate,
    /// Most recent first, following only first parents — the trunk of the
    /// branches the walk starts from.
    FirstParent,
}

/// One batch of commits, and whether there are more.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    /// The commits, in the walk's order.
    pub commits: Vec<Commit>,
    /// `true` when the walk has emitted everything it will ever emit. A
    /// chunk can be short without being the last one — a cancellation cuts
    /// it short too — so this is the only thing that says the walk is over.
    pub done: bool,
    /// `true` when `stop` asked the walk to give up part-way through this
    /// chunk. The commits in it are real, but incomplete.
    pub stopped: bool,
}

/// One batch of links a walk hands back: [`Chunk`] without the commits read
/// whole, each commit and its parents as handles into the walk's
/// [`CommitTable`].
///
/// Same contract as [`Chunk`]. Exactly one of `links.len() == limit`, `done`
/// and `stopped` explains why the batch ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkChunk {
    /// The links, in the walk's order; parents as walked, untrimmed.
    pub links: Vec<Link>,
    /// As [`Chunk::done`]: the walk has emitted its last commit.
    pub done: bool,
    /// As [`Chunk::stopped`]: `stop` fired before the batch was full; the
    /// next call carries on.
    pub stopped: bool,
}

/// A walk in progress.
///
/// Holds its position, so calling [`next_chunk`](Self::next_chunk) again
/// carries on where the last one stopped. Borrows the repository rather than
/// owning it: the handle outlives the walk, and the walk outlives no job.
pub struct Walk<'a> {
    nodes: RepoNodes<'a>,
    repo: &'a gix::Repository,
    sequence: Sequence,
    requested: Order,
    in_force: Order,
    commit_graph: bool,
    reads: u64,
    table: Arc<CommitTable>,
    /// Read once when the walk is built, held for its life: rows hydrate
    /// through it as nodes do.
    shallow: Arc<ShallowList>,
}

/// The three orderings, behind one call.
enum Sequence {
    Topological(Topological),
    /// Topological with no commit-graph: read the range, then emit.
    Sorted(Sorted),
    Dated(Dated),
}

impl<'a> Walk<'a> {
    /// A walk over everything the repository's refs reach.
    ///
    /// `table` is the repository's one id table: [`next_links`](Self::next_links)
    /// hands out handles from it.
    ///
    /// # Errors
    /// [`WalkError`] if the refs cannot be read, or a commit they name
    /// cannot be.
    pub fn all_refs(
        handle: &'a RepoHandle,
        order: Order,
        table: Arc<CommitTable>,
    ) -> Result<Self, WalkError> {
        let tips = handle.commit_tips(&*table)?;
        Walk::from_tips(handle, order, &tips, table)
    }

    /// A walk over everything `tips` reaches.
    ///
    /// The first-parent view of one branch is this with that branch's tip as
    /// the only tip; giving it every ref would follow the first parent of
    /// each of them at once, which is not what anybody means by "first
    /// parent".
    ///
    /// # Errors
    /// [`WalkError`] if a tip cannot be read.
    pub fn from_tips(
        handle: &'a RepoHandle,
        order: Order,
        tips: &[Oid],
        table: Arc<CommitTable>,
    ) -> Result<Self, WalkError> {
        let repo = handle.gix();
        let shallow = Arc::new(ShallowList::read(handle)?);
        let nodes = RepoNodes::new(repo, Arc::clone(&table), Arc::clone(&shallow));
        let commit_graph = nodes.has_commit_graph();
        let tips = handles(&nodes, tips);

        // ADR-0008: without generation numbers the incremental topological
        // sort cannot confirm anything until it has read everything, so
        // there is no first paint to be had from it. Commit date is served
        // instead, and `order_in_force` says so.
        let in_force = match order {
            Order::Topological if !commit_graph => Order::CommitDate,
            other => other,
        };

        let mut reads = 0;
        let sequence = match in_force {
            Order::Topological => {
                Sequence::Topological(Topological::new(&nodes, &tips, &mut reads)?)
            }
            Order::CommitDate => {
                Sequence::Dated(Dated::new(&nodes, &tips, Follow::AllParents, &mut reads)?)
            }
            Order::FirstParent => Sequence::Dated(Dated::new(
                &nodes,
                &tips,
                Follow::FirstParentOnly,
                &mut reads,
            )?),
        };

        Ok(Walk {
            nodes,
            repo,
            sequence,
            requested: order,
            in_force,
            commit_graph,
            reads,
            table,
            shallow,
        })
    }

    /// The topological walk over everything `tips` reaches, whether or not
    /// there is a commit-graph file.
    ///
    /// ADR-0008's background job. Without generation numbers this walk reads
    /// every commit before it emits the first one, so it is never the walk
    /// for a first paint — [`from_tips`](Self::from_tips) serves commit date
    /// for that — but the order it produces is the one the graph is drawn
    /// in, and the layout promotes to it once it finishes. It reports
    /// [`Order::Topological`] both as requested and in force, so it is never
    /// provisional. With a commit-graph file it is exactly the walk
    /// `from_tips` would build.
    ///
    /// # Errors
    /// [`WalkError`] if a tip cannot be read.
    pub fn full_topological(
        handle: &'a RepoHandle,
        tips: &[Oid],
        table: Arc<CommitTable>,
    ) -> Result<Self, WalkError> {
        let repo = handle.gix();
        let shallow = Arc::new(ShallowList::read(handle)?);
        let nodes = RepoNodes::new(repo, Arc::clone(&table), Arc::clone(&shallow));
        let commit_graph = nodes.has_commit_graph();
        let tips = handles(&nodes, tips);
        let mut reads = 0;
        // With a commit-graph this is the incremental sort `from_tips` runs.
        // Without one that sort would expand the whole range before its first
        // emission anyway; `Sorted` does the same in dense arrays.
        let sequence = if commit_graph {
            Sequence::Topological(Topological::new(&nodes, &tips, &mut reads)?)
        } else {
            Sequence::Sorted(Sorted::new(&tips))
        };
        Ok(Walk {
            nodes,
            repo,
            sequence,
            requested: Order::Topological,
            in_force: Order::Topological,
            commit_graph,
            reads,
            table,
            shallow,
        })
    }

    /// The order that was asked for.
    #[must_use]
    pub fn requested_order(&self) -> Order {
        self.requested
    }

    /// The order the commits are actually coming out in.
    ///
    /// Differs from [`requested_order`](Self::requested_order) in exactly
    /// one case: [`Order::Topological`] was asked for and the repository has
    /// no commit-graph file, so [`Order::CommitDate`] is being served.
    #[must_use]
    pub fn order_in_force(&self) -> Order {
        self.in_force
    }

    /// Whether the order on screen is not the order that was asked for.
    ///
    /// The UI says so when this is `true`: the rows are real, but they will
    /// be re-anchored once the full topological walk finishes (ADR-0008).
    #[must_use]
    pub fn is_provisional(&self) -> bool {
        self.requested != self.in_force
    }

    /// Whether the repository has a commit-graph file `gix` will use.
    ///
    /// `false` also when the file exists but `core.commitGraph` is off,
    /// because that is the same thing from the walk's point of view.
    #[must_use]
    pub fn has_commit_graph(&self) -> bool {
        self.commit_graph
    }

    /// The id table this walk's handles come from.
    #[must_use]
    pub fn table(&self) -> &Arc<CommitTable> {
        &self.table
    }

    /// How many commits the walk has read so far.
    ///
    /// The measure the 150 ms first paint is argued in: an incremental sort
    /// emits its first commit after a handful of these, and a whole-range
    /// sort only after one per commit in the repository.
    #[must_use]
    pub fn explorations(&self) -> u64 {
        self.reads
    }

    /// The next batch of at most `limit` commits.
    ///
    /// `stop` is checked before every commit. It is a predicate rather than
    /// a cancellation token because `git` may not depend on the app's job
    /// runner — the runner passes its own flag in from the other side.
    ///
    /// # Errors
    /// [`WalkError`] if a commit cannot be read or decoded.
    pub fn next_chunk(
        &mut self,
        limit: usize,
        stop: &dyn Fn() -> bool,
    ) -> Result<Chunk, WalkError> {
        let mut commits = Vec::new();
        let table = Arc::clone(&self.table);
        let shallow = Arc::clone(&self.shallow);
        let paused = self.advance(limit, stop, |repo, commit, _parents| {
            commits.push(hydrate(repo, table.id_of(commit), &shallow)?);
            Ok(())
        })?;
        Ok(Chunk {
            commits,
            done: paused == Paused::Done,
            stopped: paused == Paused::Stopped,
        })
    }

    /// The next batch of at most `limit` links, without reading the commits
    /// whole: each commit and its parents as walked (untrimmed), interned
    /// through [`table`](Self::table).
    ///
    /// The same commits as [`next_chunk`](Self::next_chunk) in the same
    /// order, and `limit` and `stop` behave as they do there. For a caller
    /// that needs only the shape, like ADR-0008's promotion: it skips the
    /// object read and decode `next_chunk` does per commit, and the parents
    /// come from the walk's own exploration, so no commit is looked up twice.
    ///
    /// # Errors
    /// [`WalkError`] if a commit cannot be read while walking.
    pub fn next_links(
        &mut self,
        limit: usize,
        stop: &dyn Fn() -> bool,
    ) -> Result<LinkChunk, WalkError> {
        let mut links = Vec::new();
        let paused = self.advance(limit, stop, |_, commit, parents| {
            links.push(Link { commit, parents });
            Ok(())
        })?;
        Ok(LinkChunk {
            links,
            done: paused == Paused::Done,
            stopped: paused == Paused::Stopped,
        })
    }

    /// Hands up to `limit` commits in walk order to `emit`, and says why it
    /// paused.
    fn advance(
        &mut self,
        limit: usize,
        stop: &dyn Fn() -> bool,
        mut emit: impl FnMut(&'a gix::Repository, CommitHandle, Parents) -> Result<(), WalkError>,
    ) -> Result<Paused, WalkError> {
        let mut emitted = 0;
        loop {
            if stop() {
                return Ok(Paused::Stopped);
            }
            if emitted >= limit {
                return Ok(Paused::Full);
            }
            let next = match &mut self.sequence {
                Sequence::Topological(sort) => sort.next(&self.nodes, &mut self.reads)?,
                Sequence::Sorted(sort) => sort.next(&self.nodes, &mut self.reads)?,
                Sequence::Dated(walk) => walk.next(&self.nodes, &mut self.reads)?,
            };
            match next {
                Some((id, parents)) => {
                    emit(self.repo, id, parents)?;
                    emitted += 1;
                }
                None => return Ok(Paused::Done),
            }
        }
    }
}

/// Why [`Walk::advance`] handed control back.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Paused {
    /// The batch is full; more may follow.
    Full,
    /// `stop` asked the walk to give up.
    Stopped,
    /// The walk has emitted everything.
    Done,
}
