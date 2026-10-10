//! The cheap half of a commit: what a walk needs to *order* history, without
//! reading the part it needs to *draw* it.
//!
//! A rev-walk touches far more commits than it emits — it explores ancestors
//! to work out what is safe to emit next — so the cost of the walk is the
//! cost of one of these, not the cost of a whole commit object. When a
//! commit-graph file is present, all four fields come out of a fixed-width
//! record with no zlib and no parsing; when it is not, the object has to be
//! read. That difference is the whole of ADR-0008.

use std::cmp::Ordering;
#[cfg(test)]
use std::collections::HashMap;
use std::sync::Arc;

use graft_core::commit_handle::{CommitHandle, Parents};
use graft_core::oid::Oid;

use super::table::CommitTable;
use crate::shallow::ShallowList;

/// A commit reduced to the three things an ordering depends on, besides its
/// id: which commit it is travels as a [`CommitHandle`] beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Node {
    /// The parents, first parent first.
    pub(crate) parents: Parents,
    /// The committer date, in seconds since the Unix epoch.
    pub(crate) time: i64,
    /// The commit-graph generation number: one more than the largest
    /// generation among its parents, and 1 for a root commit.
    ///
    /// [`Node::UNKNOWN_GENERATION`] when the commit is not in the
    /// commit-graph file — which is also the right value for the ordering to
    /// use, because a commit whose generation is unknown could be a child of
    /// anything and so must be explored before anything can be confirmed.
    /// It is git's `GENERATION_NUMBER_INFINITY`.
    pub(crate) generation: u32,
}

impl Node {
    /// The generation of a commit the commit-graph does not cover.
    pub(crate) const UNKNOWN_GENERATION: u32 = u32::MAX;
}

/// Where a walk gets its [`Node`]s, and how handles and ids convert.
///
/// A trait, not a concrete type, so the orderings can be tested against a
/// hand-built graph with no repository behind it. The fixture corpus is
/// written with strictly increasing timestamps, which makes date order and
/// topological order agree on every shape in it; only a graph with skewed
/// dates can tell the two algorithms apart.
pub(crate) trait Nodes {
    /// The handle for `id`, giving it one if it has none yet.
    fn handle(&self, id: &Oid) -> CommitHandle;

    /// The id a handle names.
    fn id(&self, handle: CommitHandle) -> Oid;

    /// The node for `handle`.
    ///
    /// # Errors
    /// [`NodeError`] if the commit is missing, unreadable, or holds a field
    /// git's own rules reject.
    fn node(&self, handle: CommitHandle) -> Result<Node, NodeError>;

    /// Whether reading `handle`'s node again is as cheap as holding it: a
    /// record in the mapped commit-graph file. A walk keeps only the nodes
    /// for which this is `false` — commits it had to parse out of an object.
    fn rereadable(&self, handle: CommitHandle) -> bool;
}

/// Why a commit could not be reduced to a `Node`.
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    /// A commit a ref or another commit points at is not in the repository.
    #[error("commit {id} is not in the repository")]
    Missing {
        /// The commit that could not be found.
        id: Oid,
    },
    /// The commit is there but could not be read or decoded.
    #[error("cannot read commit {id}: {source}")]
    Unreadable {
        /// The commit being read.
        id: Oid,
        /// What `gix` said. Boxed to keep `NodeError` small.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// The commit was read, but holds an object id git's own rules reject.
    #[error("commit {id} is malformed: {source}")]
    Malformed {
        /// The commit being read.
        id: Oid,
        /// Which invariant the value broke.
        source: graft_core::error::Error,
    },
}

/// What one commit is ordered by: its date, then its id.
///
/// Sorts ascending, so a max-heap pops the most recent commit first and
/// breaks an exact date tie by the *smallest* id. That is the tie-break
/// DESIGN.md §3.2 names, and it is what makes two runs over the same
/// repository produce the same graph — commit dates tie constantly in
/// fast-import repositories, in rebases and in scripted history.
///
/// The handle rides along so the winner needs no lookup; it never takes part
/// in the comparison (handle order is not id order: commit-graph layers and
/// the overflow are each sorted differently).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Key {
    pub(crate) time: i64,
    pub(crate) id: Oid,
    pub(crate) handle: CommitHandle,
}

impl Key {
    pub(crate) fn of(nodes: &impl Nodes, handle: CommitHandle, node: &Node) -> Self {
        Key {
            time: node.time,
            id: nodes.id(handle),
            handle,
        }
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Key {}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time
            .cmp(&other.time)
            .then_with(|| other.id.cmp(&self.id))
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A graph held in memory, for tests that need dates the corpus cannot
/// produce: `id → (parents, time, generation)`.
///
/// A node with [`Node::UNKNOWN_GENERATION`] stands for a commit outside the
/// commit-graph, so it is not [`rereadable`](Nodes::rereadable) and the walk
/// has to keep it, as it would a parsed object.
#[cfg(test)]
pub(crate) struct MapNodes {
    nodes: HashMap<Oid, (Vec<Oid>, i64, u32)>,
    table: CommitTable,
}

#[cfg(test)]
impl MapNodes {
    pub(crate) fn new(nodes: HashMap<Oid, (Vec<Oid>, i64, u32)>) -> Self {
        MapNodes {
            nodes,
            table: CommitTable::with_graph(None, Oid::SHA1_BYTES),
        }
    }
}

#[cfg(test)]
impl Nodes for MapNodes {
    fn handle(&self, id: &Oid) -> CommitHandle {
        self.table.intern(id)
    }

    fn id(&self, handle: CommitHandle) -> Oid {
        self.table.id_of(handle)
    }

    fn node(&self, handle: CommitHandle) -> Result<Node, NodeError> {
        let id = self.id(handle);
        let (parents, time, generation) = self.nodes.get(&id).ok_or(NodeError::Missing { id })?;
        Ok(Node {
            parents: parents.iter().map(|parent| self.handle(parent)).collect(),
            time: *time,
            generation: *generation,
        })
    }

    fn rereadable(&self, handle: CommitHandle) -> bool {
        self.nodes
            .get(&self.id(handle))
            .is_some_and(|&(_, _, generation)| generation != Node::UNKNOWN_GENERATION)
    }
}

/// The repository's own nodes: the commit-graph file for every commit it
/// covers, the object database only for the ones it does not.
///
/// A commit `shallow` lists has no parents, whichever source it came from:
/// git's rule for a shallow clone's boundary (m7-power-tools/32). Its
/// parent was never fetched, so it is never interned either.
pub(crate) struct RepoNodes<'a> {
    repo: &'a gix::Repository,
    table: Arc<CommitTable>,
    shallow: Arc<ShallowList>,
}

impl<'a> RepoNodes<'a> {
    pub(crate) fn new(
        repo: &'a gix::Repository,
        table: Arc<CommitTable>,
        shallow: Arc<ShallowList>,
    ) -> Self {
        RepoNodes {
            repo,
            table,
            shallow,
        }
    }

    /// Whether `handle` is a shallow boundary. One `is_empty` branch on a
    /// complete repository; the id is only looked up in a shallow one.
    fn is_boundary(&self, handle: CommitHandle) -> bool {
        !self.shallow.is_empty() && self.shallow.contains(&self.id(handle))
    }

    /// Whether a commit-graph file is backing these reads.
    pub(crate) fn has_commit_graph(&self) -> bool {
        self.table.has_commit_graph()
    }

    /// The commit-graph position a handle is, if it is one.
    fn position(&self, handle: CommitHandle) -> Option<(&gix::commitgraph::Graph, u32)> {
        let graph = self.table.graph()?;
        (handle.0 < self.table.graph_count()).then_some((graph, handle.0))
    }
}

impl Nodes for RepoNodes<'_> {
    fn handle(&self, id: &Oid) -> CommitHandle {
        self.table.intern(id)
    }

    fn id(&self, handle: CommitHandle) -> Oid {
        self.table.id_of(handle)
    }

    fn node(&self, handle: CommitHandle) -> Result<Node, NodeError> {
        let boundary = self.is_boundary(handle);
        if let Some((graph, position)) = self.position(handle) {
            let mut node = node_from_graph(graph, gix::commitgraph::Position(position))?;
            // `CommitTable::open` binds no graph in a shallow repository, as
            // git does; a table opened before the clone became shallow still
            // must not hand out a parent that was cut.
            if boundary {
                node.parents = Parents::None;
            }
            return Ok(node);
        }
        node_from_object(self.repo, &self.id(handle), &self.table, boundary)
    }

    fn rereadable(&self, handle: CommitHandle) -> bool {
        self.position(handle).is_some()
    }
}

/// A commit's node straight out of the commit-graph file: parents as
/// positions (which are handles), date and generation. No object is read.
///
/// # Errors
/// [`NodeError::Unreadable`] if the file names a parent position out of range
/// or a date past `i64`: the file is corrupt, and saying so beats guessing.
pub(crate) fn node_from_graph(
    graph: &gix::commitgraph::Graph,
    pos: gix::commitgraph::Position,
) -> Result<Node, NodeError> {
    let commit = graph.commit_at(pos);
    let corrupt = |source: Box<dyn std::error::Error + Send + Sync>| NodeError::Unreadable {
        id: graph_id(graph, pos),
        source,
    };
    let time =
        i64::try_from(commit.committer_timestamp()).map_err(|source| corrupt(Box::new(source)))?;
    let generation = commit.generation();
    let parents = commit
        .iter_parents()
        .map(|parent| parent.map(|position| CommitHandle(position.0)))
        .collect::<Result<Parents, _>>()
        .map_err(|source| corrupt(source.to_string().into()))?;
    Ok(Node {
        parents,
        time,
        generation,
    })
}

/// The id at a commit-graph position.
///
/// # Panics
/// If the file holds an id of a length Graft cannot hold, as
/// [`CommitTable::id_of`] does for the same file.
fn graph_id(graph: &gix::commitgraph::Graph, pos: gix::commitgraph::Position) -> Oid {
    match Oid::from_bytes(graph.id_at(pos).as_bytes()) {
        Ok(id) => id,
        Err(error) => panic!("the commit-graph holds an id Tributary cannot: {error}"),
    }
}

/// A commit outside the commit-graph, header-parsed from the object
/// database: parents interned through `table`, generation unknown (explored
/// first, git's rule). A `boundary` (listed in `shallow`) has none: its
/// object still names parents the clone never fetched.
///
/// # Errors
/// [`NodeError`] if the object is missing, unreadable or malformed.
pub(crate) fn node_from_object(
    repo: &gix::Repository,
    id: &Oid,
    table: &CommitTable,
    boundary: bool,
) -> Result<Node, NodeError> {
    let id = *id;
    let commit = find_commit(repo, id)?;
    let mut parents = Vec::with_capacity(1);
    for parent in commit.parent_ids().filter(|_| !boundary) {
        let parent = Oid::from_bytes(parent.as_bytes())
            .map_err(|source| NodeError::Malformed { id, source })?;
        parents.push(table.intern(&parent));
    }
    Ok(Node {
        parents: parents.into_iter().collect(),
        time: commit_time(&commit, id)?,
        generation: Node::UNKNOWN_GENERATION,
    })
}

/// Finds a commit object, telling "not in this repository" apart from "could
/// not be read".
pub(crate) fn find_commit(repo: &gix::Repository, id: Oid) -> Result<gix::Commit<'_>, NodeError> {
    let oid =
        gix::hash::oid::try_from_bytes(id.as_bytes()).map_err(|source| NodeError::Unreadable {
            id,
            source: Box::new(source),
        })?;
    repo.find_commit(oid).map_err(|source| match source {
        gix::object::find::existing::with_conversion::Error::Find(
            gix::object::find::existing::Error::NotFound { .. },
        ) => NodeError::Missing { id },
        other => NodeError::Unreadable {
            id,
            source: Box::new(other),
        },
    })
}

/// A commit's committer date in seconds since the Unix epoch.
pub(crate) fn commit_time(commit: &gix::Commit<'_>, id: Oid) -> Result<i64, NodeError> {
    commit
        .time()
        .map(|time| time.seconds)
        .map_err(|source| NodeError::Unreadable {
            id,
            source: Box::new(source),
        })
}
