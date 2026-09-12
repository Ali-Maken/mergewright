//! Reading history out of a repository, in order, a chunk at a time.
//!
//! The layout engine is pure: it takes commits in and produces rows. This is
//! the other half — the thing that produces the commits, in the order the
//! graph is to be drawn in, without reading the whole repository first and
//! without blocking on a walk nobody is waiting for any more.
//!
//! Three orders, one interface ([`Order`]):
//!
//! * [`Order::Topological`] is the one the graph is finally drawn in. It
//!   needs the commit-graph file's generation numbers to emit incrementally;
//!   see [`topological`].
//! * [`Order::CommitDate`] is the natural streaming order and needs nothing
//!   at all. ADR-0008 makes it the first paint when there is no commit-graph
//!   file, with the UI saying so.
//! * [`Order::FirstParent`] follows only each commit's first parent: the
//!   branch's own trunk.
//!
//! Every order breaks a tie the same way — commit date descending, then
//! commit id ascending (DESIGN.md §3.2) — so the same repository draws the
//! same graph twice, which matters because scripted history and
//! `fast-import` produce identical timestamps constantly.
//!
//! A walk is not an iterator. [`Walk::next_chunk`] hands back a bounded
//! batch and keeps its position, so the caller can deliver a chunk to the
//! screen, check whether anyone still wants the rest, and come back for
//! more.

mod node;
mod simple;
mod topological;

use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;

use crate::repo::{RepoHandle, TipsError};
use node::{Nodes, RepoNodes};
use simple::{Dated, Follow};
use topological::Topological;

pub use node::NodeError;

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
}

/// The three orderings, behind one call.
enum Sequence {
    Topological(Topological),
    Dated(Dated),
}

impl<'a> Walk<'a> {
    /// A walk over everything the repository's refs reach.
    ///
    /// # Errors
    /// [`WalkError`] if the refs cannot be read, or a commit they name
    /// cannot be.
    pub fn all_refs(handle: &'a RepoHandle, order: Order) -> Result<Self, WalkError> {
        let tips = handle.commit_tips()?;
        Walk::from_tips(handle, order, &tips)
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
    ) -> Result<Self, WalkError> {
        let repo = handle.gix();
        let graph = repo
            .commit_graph_if_enabled()
            .map_err(|source| WalkError::CommitGraph {
                source: Box::new(source),
            })?;
        let nodes = RepoNodes::new(repo, graph);
        let commit_graph = nodes.has_commit_graph();

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
                Sequence::Topological(Topological::new(&nodes, tips, &mut reads)?)
            }
            Order::CommitDate => {
                Sequence::Dated(Dated::new(&nodes, tips, Follow::AllParents, &mut reads)?)
            }
            Order::FirstParent => Sequence::Dated(Dated::new(
                &nodes,
                tips,
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
        loop {
            if stop() {
                return Ok(Chunk {
                    commits,
                    done: false,
                    stopped: true,
                });
            }
            if commits.len() >= limit {
                return Ok(Chunk {
                    commits,
                    done: false,
                    stopped: false,
                });
            }
            let next = match &mut self.sequence {
                Sequence::Topological(sort) => sort.next(&self.nodes, &mut self.reads)?,
                Sequence::Dated(walk) => walk.next(&self.nodes, &mut self.reads)?,
            };
            match next {
                Some(id) => commits.push(self.hydrate(id)?),
                None => {
                    return Ok(Chunk {
                        commits,
                        done: true,
                        stopped: false,
                    });
                }
            }
        }
    }

    /// Reads the whole commit object for one emitted id.
    ///
    /// Only emitted commits are hydrated. Exploration works on [`node`]s,
    /// which come out of the commit-graph without touching the object
    /// database at all when there is one — so the cost of a walk is roughly
    /// the cost of what it shows, not of what it had to look at.
    fn hydrate(&self, id: Oid) -> Result<Commit, WalkError> {
        let commit = node::find_commit(self.repo, id)?;
        let unreadable = |source: gix::objs::decode::Error| NodeError::Unreadable {
            id,
            source: Box::new(source),
        };

        let parents = commit
            .parent_ids()
            .map(|parent| {
                Oid::from_bytes(parent.as_bytes())
                    .map_err(|source| NodeError::Malformed { id, source })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let author = identity(commit.author().map_err(unreadable)?, id)?;
        let committer = identity(commit.committer().map_err(unreadable)?, id)?;
        let message = commit.message_raw().map_err(unreadable)?.to_vec();

        Ok(Commit {
            id,
            parents,
            author,
            committer,
            message,
        })
    }
}

/// How many merges lie on a commit's first-parent chain.
///
/// ADR-0001's last-resort rule for the default branch is "the branch that is
/// the target of the most first-parent merges", which is this counted once
/// per branch tip. It works on [`node`]s rather than whole commits — no
/// message, no tree, no signature — because the answer only needs each
/// commit's parent count, and on a repository with a commit-graph file that
/// costs no object reads at all.
///
/// # Errors
/// [`WalkError`] if a commit on the chain cannot be read.
pub fn first_parent_merges(handle: &RepoHandle, tip: Oid) -> Result<usize, WalkError> {
    let repo = handle.gix();
    let graph = repo
        .commit_graph_if_enabled()
        .map_err(|source| WalkError::CommitGraph {
            source: Box::new(source),
        })?;
    let nodes = RepoNodes::new(repo, graph);

    let mut merges = 0;
    let mut seen = std::collections::HashSet::new();
    let mut at = Some(tip);
    while let Some(id) = at {
        // A first-parent chain cannot revisit a commit in a well-formed
        // repository, but a corrupt one must not spin here for ever.
        if !seen.insert(id) {
            break;
        }
        let node = nodes.node(id)?;
        if node.parents.len() > 1 {
            merges += 1;
        }
        at = node.parents.first().copied();
    }
    Ok(merges)
}

/// One side of git's author/committer pair, as plain bytes.
fn identity(signature: gix::actor::SignatureRef<'_>, id: Oid) -> Result<Identity, NodeError> {
    let time = signature.time().map_err(|source| NodeError::Unreadable {
        id,
        source: Box::new(source),
    })?;
    Ok(Identity {
        name: signature.name.to_vec(),
        email: signature.email.to_vec(),
        when: Timestamp {
            seconds: time.seconds,
            offset_seconds: time.offset,
        },
    })
}

/// Why a walk could not run.
#[derive(Debug, thiserror::Error)]
pub enum WalkError {
    /// The refs the walk starts from could not be read.
    #[error(transparent)]
    Tips(#[from] TipsError),
    /// A commit could not be read.
    #[error(transparent)]
    Commit(#[from] NodeError),
    /// The commit-graph file exists but could not be opened. Not the same as
    /// there being none: a corrupt one is worth reporting.
    #[error("cannot read the commit-graph file: {source}")]
    CommitGraph {
        /// What `gix` said. Boxed to keep `WalkError` small.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

#[cfg(test)]
mod tests;
