//! Types [`super::refs`] and [`super::peel`] return: ref and tip
//! snapshots, and the errors reading refs can fail with.

use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;

/// Why a walk's starting points could not be found.
#[derive(Debug, thiserror::Error)]
pub enum TipsError {
    /// The ref store could not be read, or a ref could not be peeled.
    #[error("cannot read the refs in {}: {source}", git_dir.display())]
    Read {
        /// The repository whose refs were being read.
        git_dir: PathBuf,
        /// What `gix` said. Boxed to keep `TipsError` small.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A ref peeled to an object id git's own rules reject.
    #[error("a ref in {} names a malformed object: {source}", git_dir.display())]
    Malformed {
        /// The repository whose refs were being read.
        git_dir: PathBuf,
        /// Which invariant the value broke.
        source: graft_core::error::Error,
    },
}

/// Every ref and every commit they reach, read in one pass
/// ([`RepoHandle::ref_snapshot`](crate::repo::RepoHandle::ref_snapshot)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefSnapshot {
    /// What [`RepoHandle::references`](crate::repo::RepoHandle::references) returns: every ref, sorted by name.
    pub refs: Vec<RefEntry>,
    /// What [`RepoHandle::commit_tips`](crate::repo::RepoHandle::commit_tips) returns: sorted, deduplicated, `HEAD`
    /// included.
    pub tips: Vec<Oid>,
    /// What [`RepoHandle::peeled_references`](crate::repo::RepoHandle::peeled_references) returns: every ref that reaches
    /// a commit, with that commit, sorted by name.
    pub peeled: Vec<(RefName, Oid)>,
    /// How the pass learned its tips (m1-the-graph/58b).
    pub stats: TipStats,
}

/// How one ref-store pass learned its tips — what the table answered, what
/// had to be read.
///
/// For the memory record (`budgets --phases`) and the corpus tests; never a
/// decision input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TipStats {
    /// Refs enumerated.
    pub refs: u32,
    /// Tips the [`KnownCommits`](crate::repo::KnownCommits) table answered without a read.
    pub known: u32,
    /// Annotated tag objects met (peeled from `packed-refs` or decoded).
    pub tags: u32,
    /// Tags resolved from a `packed-refs` peeled id: no object read.
    pub peeled_packed: u32,
    /// Objects decoded whole (tags, and targets the table does not know).
    pub objects_read: u32,
    /// Header-only reads.
    pub find_header: u32,
}

/// One ref, as the ref store holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefEntry {
    /// The full name — `refs/heads/main`, `refs/tags/v1.0`.
    pub name: RefName,
    /// What the ref points at.
    pub target: RefTarget,
}

/// What a ref points at: an object, or another ref.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefTarget {
    /// An object id, read as stored and not peeled.
    Object(Oid),
    /// Another ref's full name. Rare outside `HEAD`, but legal anywhere.
    Symbolic(RefName),
}

/// Why the refs could not be read.
#[derive(Debug, thiserror::Error)]
pub enum RefsError {
    /// The ref store could not be read.
    #[error("cannot read the refs in {}: {source}", git_dir.display())]
    Read {
        /// The repository whose refs were being read.
        git_dir: PathBuf,
        /// What `gix` said. Boxed to keep `RefsError` small.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A ref was read, but names something git's own rules reject.
    #[error("a ref in {} is malformed: {source}", git_dir.display())]
    Malformed {
        /// The repository whose refs were being read.
        git_dir: PathBuf,
        /// Which invariant the value broke.
        source: graft_core::error::Error,
    },
}
