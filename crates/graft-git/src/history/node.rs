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

use graft_core::oid::Oid;

/// A commit reduced to the four things an ordering depends on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Node {
    /// The commit's id.
    pub(crate) id: Oid,
    /// The parents, first parent first.
    pub(crate) parents: Vec<Oid>,
    /// The committer date, in seconds since the Unix epoch.
    pub(crate) time: i64,
    /// The commit-graph generation number: one more than the largest
    /// generation among its parents, and 1 for a root commit.
    ///
    /// [`Node::UNKNOWN_GENERATION`] when the commit is not in the
    /// commit-graph file — which is also the right value for the ordering to
    /// use, because a commit whose generation is unknown could be a child of
    /// anything and so must be explored before anything can be confirmed.
    pub(crate) generation: u64,
}

impl Node {
    /// The generation of a commit the commit-graph does not cover.
    pub(crate) const UNKNOWN_GENERATION: u64 = u64::MAX;
}

/// Where a walk gets its [`Node`]s.
///
/// A trait, not a concrete type, so the orderings can be tested against a
/// hand-built graph with no repository behind it. The fixture corpus is
/// written with strictly increasing timestamps, which makes date order and
/// topological order agree on every shape in it; only a graph with skewed
/// dates can tell the two algorithms apart.
pub(crate) trait Nodes {
    /// The node for `id`.
    ///
    /// # Errors
    /// [`NodeError`] if the commit is missing, unreadable, or holds a field
    /// git's own rules reject.
    fn node(&self, id: Oid) -> Result<Node, NodeError>;
}

/// Why a commit could not be reduced to a [`Node`].
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Key {
    pub(crate) time: i64,
    pub(crate) id: Oid,
}

impl Key {
    pub(crate) fn of(node: &Node) -> Self {
        Key {
            time: node.time,
            id: node.id,
        }
    }
}

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
/// produce.
#[cfg(test)]
pub(crate) struct MapNodes(pub(crate) HashMap<Oid, Node>);

#[cfg(test)]
impl Nodes for MapNodes {
    fn node(&self, id: Oid) -> Result<Node, NodeError> {
        self.0.get(&id).cloned().ok_or(NodeError::Missing { id })
    }
}

/// The repository's own nodes: the commit-graph file when there is one, the
/// object database when there is not.
pub(crate) struct RepoNodes<'a> {
    repo: &'a gix::Repository,
    graph: Option<gix::commitgraph::Graph>,
}

impl<'a> RepoNodes<'a> {
    pub(crate) fn new(repo: &'a gix::Repository, graph: Option<gix::commitgraph::Graph>) -> Self {
        RepoNodes { repo, graph }
    }

    /// Whether a commit-graph file is backing these reads.
    pub(crate) fn has_commit_graph(&self) -> bool {
        self.graph.is_some()
    }

    /// The node straight out of the commit-graph file, or [`None`] if this
    /// commit is not in it.
    ///
    /// Every `?` here also means "not in it": a position the file says is out
    /// of range, or a timestamp too large for an `i64`, means the file is
    /// corrupt or from a future format. Both fall through to the object
    /// database, which is the truth the file is only a cache of — and a real
    /// read failure there is reported rather than hidden.
    fn from_commit_graph(graph: &gix::commitgraph::Graph, id: Oid) -> Option<Node> {
        let oid = gix::hash::oid::try_from_bytes(id.as_bytes()).ok()?;
        let commit = graph.commit_by_id(oid)?;
        let mut parents = Vec::with_capacity(1);
        for parent in commit.iter_parents() {
            // A parent position the file itself calls out of range means the
            // commit-graph is corrupt. Falling back to the object database
            // is the honest answer: the objects are the truth, the graph is
            // only a cache of them.
            let position = parent.ok()?;
            parents.push(Oid::from_bytes(graph.id_at(position).as_bytes()).ok()?);
        }
        Some(Node {
            id,
            parents,
            time: i64::try_from(commit.committer_timestamp()).ok()?,
            generation: u64::from(commit.generation()),
        })
    }

    fn read_object(&self, id: Oid) -> Result<Node, NodeError> {
        let commit = find_commit(self.repo, id)?;
        let mut parents = Vec::with_capacity(1);
        for parent in commit.parent_ids() {
            parents.push(
                Oid::from_bytes(parent.as_bytes())
                    .map_err(|source| NodeError::Malformed { id, source })?,
            );
        }
        Ok(Node {
            id,
            parents,
            time: commit_time(&commit, id)?,
            generation: Node::UNKNOWN_GENERATION,
        })
    }
}

impl Nodes for RepoNodes<'_> {
    fn node(&self, id: Oid) -> Result<Node, NodeError> {
        if let Some(graph) = &self.graph
            && let Some(node) = Self::from_commit_graph(graph, id)
        {
            return Ok(node);
        }
        self.read_object(id)
    }
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
