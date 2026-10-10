//! The one id table of an open repository: every commit Graft holds, as a
//! [`CommitHandle`].
//!
//! ADR-0026. With a commit-graph file the file *is* the table: a commit's
//! handle is its global position in the graph (split chains included), `id_of`
//! is one read of the mapped file and `handle_of` gix's fanout search, and
//! nothing is copied onto the heap. Commits the graph does not cover — fetched
//! since it was written, or every commit when there is no graph — go to the
//! overflow interner and get handles from `graph_count` up.
//!
//! The overflow stores what it must and no more: the raw id bytes at the
//! repository's hash length, packed end to end, plus a map from each id's
//! first eight bytes to its handle. Eight bytes of a cryptographic hash do not
//! collide in practice, but an id someone constructed can, so every hit is
//! checked against the stored bytes and a genuine collision goes to a small
//! map keyed by the whole id.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{PoisonError, RwLock};

use graft_core::commit_handle::CommitHandle;
use graft_core::oid::Oid;

use crate::repo::RepoHandle;
use crate::shallow::{ShallowError, ShallowList};

/// The id table one open repository shares between its walk, its views and
/// its search. `Send + Sync`; hold it as `Arc<CommitTable>`.
pub struct CommitTable {
    /// The commit-graph file, when the repository has one `gix` will use.
    graph: Option<gix::commitgraph::Graph>,
    /// How many commits the graph covers: handles below this are positions.
    graph_count: u32,
    /// Commits outside the graph. A lock because interning happens on
    /// whichever job thread meets a new commit first.
    overflow: RwLock<Overflow>,
}

impl CommitTable {
    /// The table for `repo`, bound to its commit-graph file if it has one.
    ///
    /// Uses the commit-graph exactly when the walk does (`core.commitGraph`
    /// respected), so a handle means the same thing to both. None in a
    /// shallow repository, git's own rule (`commit-graph.c`
    /// `commit_graph_compatible`): the file records parents the clone cut.
    ///
    /// # Errors
    /// [`TableError::CommitGraph`] if the file is there but cannot be opened;
    /// [`TableError::Shallow`] if `<common_dir>/shallow` cannot be read. A
    /// missing file is not an error: the table is all overflow.
    pub fn open(repo: &RepoHandle) -> Result<Self, TableError> {
        CommitTable::open_with(repo, &ShallowList::read(repo)?)
    }

    /// [`open`](Self::open) for a caller that has read the shallow list.
    pub(crate) fn open_with(repo: &RepoHandle, shallow: &ShallowList) -> Result<Self, TableError> {
        let gix = repo.gix();
        let hash_len = gix.object_hash().len_in_bytes();
        if !shallow.is_empty() {
            return Ok(CommitTable::with_graph(None, hash_len));
        }
        let graph = gix
            .commit_graph_if_enabled()
            .map_err(|source| TableError::CommitGraph {
                path: gix.common_dir().join("objects").join("info"),
                source: Box::new(source),
            })?;
        Ok(CommitTable::with_graph(graph, hash_len))
    }

    /// A table with no commit-graph behind it, for ids of `hash_len` bytes:
    /// every id it is given lands in the overflow interner.
    ///
    /// For callers that hold ids and no repository — the Mistake Radar's
    /// context, built from a list of commits rather than from a walk
    /// (`graft::jobs::radar::RadarContext::of_ids`).
    #[must_use]
    pub fn without_a_commit_graph(hash_len: usize) -> Self {
        CommitTable::with_graph(None, hash_len)
    }

    /// A table over `graph`, with an overflow for ids of `hash_len` bytes.
    pub(crate) fn with_graph(graph: Option<gix::commitgraph::Graph>, hash_len: usize) -> Self {
        let graph_count = graph
            .as_ref()
            .map_or(0, gix::commitgraph::Graph::num_commits);
        CommitTable {
            graph,
            graph_count,
            overflow: RwLock::new(Overflow::new(hash_len)),
        }
    }

    /// Whether handles below [`graph_count`](Self::graph_count) come from a
    /// commit-graph file.
    #[must_use]
    pub fn has_commit_graph(&self) -> bool {
        self.graph.is_some()
    }

    /// The commit-graph file behind the low handles, if there is one: the walk
    /// reads parents, dates and generations straight out of it.
    pub(crate) fn graph(&self) -> Option<&gix::commitgraph::Graph> {
        self.graph.as_ref()
    }

    /// How many commits the commit-graph covers; 0 without one.
    #[must_use]
    pub fn graph_count(&self) -> u32 {
        self.graph_count
    }

    /// The handle for `id`, giving it one if it has none yet.
    ///
    /// A commit in the graph gets its position and nothing is stored. Any
    /// other id is appended to the overflow once; interning it again returns
    /// the same handle.
    ///
    /// # Panics
    /// If the repository holds 2³² commits or more, which no handle can name.
    pub fn intern(&self, id: &Oid) -> CommitHandle {
        if let Some(handle) = self.handle_of(id) {
            return handle;
        }
        let mut overflow = self
            .overflow
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        // Another thread may have appended it between the two locks.
        if let Some(index) = overflow.index_of(id) {
            return self.overflow_handle(index);
        }
        let index = overflow.append(id);
        self.overflow_handle(index)
    }

    /// The handle `id` already has, if any. Never appends.
    #[must_use]
    pub fn handle_of(&self, id: &Oid) -> Option<CommitHandle> {
        if let Some(graph) = &self.graph
            && let Some(position) =
                graph.lookup(gix::hash::oid::from_bytes_unchecked(id.as_bytes()))
        {
            return Some(CommitHandle(position.0));
        }
        let overflow = self.overflow.read().unwrap_or_else(PoisonError::into_inner);
        overflow
            .index_of(id)
            .map(|index| self.overflow_handle(index))
    }

    /// The id a handle names.
    ///
    /// Infallible because handles come only from this table.
    ///
    /// # Panics
    /// If `handle` was not handed out by this table — a handle from another
    /// repository's table, or a made-up one. That is a bug in the caller, not
    /// a condition to recover from.
    #[must_use]
    pub fn id_of(&self, handle: CommitHandle) -> Oid {
        if let Some(graph) = &self.graph
            && handle.0 < self.graph_count
        {
            let id = graph.id_at(gix::commitgraph::Position(handle.0));
            return match Oid::from_bytes(id.as_bytes()) {
                Ok(id) => id,
                // gix reads ids at the repository's hash length, 20 or 32.
                Err(error) => panic!("the commit-graph holds an id Tributary cannot: {error}"),
            };
        }
        let overflow = self.overflow.read().unwrap_or_else(PoisonError::into_inner);
        overflow.id_at((handle.0 - self.graph_count) as usize)
    }

    /// Every commit with a handle: the graph's plus the overflow's.
    #[must_use]
    pub fn len(&self) -> usize {
        self.graph_count as usize + self.overflow_len()
    }

    /// Whether no commit has a handle.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Handles given to commits the graph does not cover.
    #[must_use]
    pub fn overflow_len(&self) -> usize {
        self.overflow
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// Heap bytes the table holds. The mapped commit-graph is not heap and is
    /// not counted.
    #[must_use]
    pub fn approximate_bytes(&self) -> usize {
        self.overflow
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .approximate_bytes()
    }

    /// The handle for the overflow entry at `index`.
    fn overflow_handle(&self, index: usize) -> CommitHandle {
        match u32::try_from(index)
            .ok()
            .and_then(|index| self.graph_count.checked_add(index))
        {
            Some(handle) => CommitHandle(handle),
            None => panic!("a repository of 2^32 commits or more has no handle for the last"),
        }
    }
}

impl crate::repo::KnownCommits for CommitTable {
    /// A commit's own table knows it without appending: the graph's fanout
    /// search or the overflow's prefix lookup, never a header read
    /// (m1-the-graph/58).
    fn is_known_commit(&self, id: Oid) -> bool {
        self.handle_of(&id).is_some()
    }

    /// With a commit-graph the table holds every commit the graph was
    /// written for; without one it holds nothing yet at ref time.
    fn covers_the_graph(&self) -> bool {
        self.has_commit_graph()
    }
}

impl std::fmt::Debug for CommitTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommitTable")
            .field("graph_count", &self.graph_count)
            .field("overflow_len", &self.overflow_len())
            .finish_non_exhaustive()
    }
}

/// The ids the commit-graph does not cover, in the order they were interned.
struct Overflow {
    /// Bytes per id: 20 for SHA-1, 32 for SHA-256.
    hash_len: usize,
    /// Every id's raw bytes, end to end, [`IDS_PER_PAGE`] ids a page: entry
    /// `i` is at `(i % IDS_PER_PAGE) * hash_len` in page `i / IDS_PER_PAGE`.
    ///
    /// Not one `Vec`: growing that to a million ids by doubling copies it
    /// over and over, and on macOS the freed copies stay in the footprint
    /// (m1-the-graph/59: ~32 MiB at 1M). A page is allocated at its full size
    /// once and never moves.
    pages: Vec<Vec<u8>>,
    /// Ids stored.
    len: usize,
    /// Entry index by the id's first eight bytes. The first id with a prefix
    /// lives here; a later one sharing it goes to `collided`.
    by_prefix: HashMap<u64, u32>,
    /// Entry index by whole id, for ids whose prefix was already taken.
    collided: HashMap<Oid, u32>,
}

impl Overflow {
    fn new(hash_len: usize) -> Self {
        Overflow {
            hash_len,
            pages: Vec::new(),
            len: 0,
            by_prefix: HashMap::new(),
            collided: HashMap::new(),
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    /// The entry holding `id`, if any.
    fn index_of(&self, id: &Oid) -> Option<usize> {
        let index = *self.by_prefix.get(&prefix(id))? as usize;
        if self.stored(index) == id.as_bytes() {
            return Some(index);
        }
        self.collided.get(id).map(|&index| index as usize)
    }

    /// Appends `id`, which must not be present, and returns its entry.
    fn append(&mut self, id: &Oid) -> usize {
        debug_assert_eq!(id.as_bytes().len(), self.hash_len, "an id of another hash");
        let index = self.len();
        let Ok(stored) = u32::try_from(index) else {
            panic!("a repository of 2^32 commits or more has no handle for the last");
        };
        if index.is_multiple_of(IDS_PER_PAGE) {
            self.pages
                .push(Vec::with_capacity(IDS_PER_PAGE * self.hash_len));
        }
        let Some(page) = self.pages.last_mut() else {
            unreachable!("a page was pushed for the first id of every page");
        };
        page.extend_from_slice(id.as_bytes());
        self.len += 1;
        match self.by_prefix.entry(prefix(id)) {
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(stored);
            }
            std::collections::hash_map::Entry::Occupied(_) => {
                self.collided.insert(*id, stored);
            }
        }
        index
    }

    fn id_at(&self, index: usize) -> Oid {
        assert!(index < self.len(), "a commit handle from another table");
        match Oid::from_bytes(self.stored(index)) {
            Ok(id) => id,
            // `append` stores whole ids, so every entry is 20 or 32 bytes.
            Err(error) => panic!("the overflow holds a malformed id: {error}"),
        }
    }

    fn stored(&self, index: usize) -> &[u8] {
        let start = (index % IDS_PER_PAGE) * self.hash_len;
        &self.pages[index / IDS_PER_PAGE][start..start + self.hash_len]
    }

    /// Heap bytes, capacity included. A `HashMap` entry costs its key and value
    /// plus one control byte, over about 8/7 of its capacity in buckets.
    fn approximate_bytes(&self) -> usize {
        fn map<K, V>(map: &HashMap<K, V>) -> usize {
            let bucket = std::mem::size_of::<(K, V)>() + 1;
            map.capacity() * 8 / 7 * bucket
        }
        let pages: usize = self.pages.iter().map(Vec::capacity).sum();
        pages + map(&self.by_prefix) + map(&self.collided)
    }
}

/// Ids in one overflow page: 2¹⁶, 1.25 MiB of SHA-1 ids.
const IDS_PER_PAGE: usize = 1 << 16;

/// An id's first eight bytes, as the key the overflow is indexed by.
fn prefix(id: &Oid) -> u64 {
    let mut head = [0; 8];
    head.copy_from_slice(&id.as_bytes()[..8]);
    u64::from_le_bytes(head)
}

/// Why the table could not be opened.
#[derive(Debug, thiserror::Error)]
pub enum TableError {
    /// The commit-graph file is there but `gix` could not open it.
    #[error("cannot open the commit-graph in {}: {source}", path.display())]
    CommitGraph {
        /// The directory the commit-graph file or chain lives in.
        path: PathBuf,
        /// What `gix` said. Boxed to keep `TableError` small.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// `<common_dir>/shallow` exists but could not be read, so whether the
    /// graph may be used is unknown (m7/32).
    #[error(transparent)]
    Shallow(#[from] ShallowError),
}

#[cfg(test)]
mod tests;
