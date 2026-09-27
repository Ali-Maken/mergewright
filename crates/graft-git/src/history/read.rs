//! Reading a commit whole, and the small free functions that only need a
//! [`node`](super::node): hydrating one from its object, first-parent merge
//! counting for ADR-0001, the default branch's first-parent chain and a
//! ref's own reflog for the Mistake Radar (`radar-context-and-markers`), and
//! [`WalkError`] itself.

use std::sync::Arc;

use gix::bstr::BStr;
use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::commit_handle::{CommitHandle, HandleSet};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;

use super::node::{self, NodeError, Nodes, RepoNodes};
use super::table::CommitTable;
use crate::repo::{RepoHandle, TipsError};
use crate::run::Stop;
use crate::shallow::{ShallowError, ShallowList};

/// Reads one commit whole, by id.
///
/// The streaming layout rebuilds evicted rows by re-reading commits it has
/// already emitted (DESIGN.md §3.6); this is that read. It is the same object
/// read a walk does for the commits it emits, so a commit read back is
/// identical to the one the walk produced.
///
/// # Errors
/// [`WalkError`] if the commit cannot be read or decoded.
pub fn read_commit(handle: &RepoHandle, id: Oid) -> Result<Commit, WalkError> {
    hydrate(handle.gix(), id, &ShallowList::read(handle)?)
}

/// The walk's tips as handles, each interned once.
pub(super) fn handles(nodes: &impl Nodes, tips: &[Oid]) -> Vec<CommitHandle> {
    tips.iter().map(|tip| nodes.handle(tip)).collect()
}

/// Reads the whole commit object for one emitted id.
///
/// Only emitted commits are hydrated. Exploration works on [`node`]s, which
/// come out of the commit-graph without touching the object database at all
/// when there is one — so the cost of a walk is roughly the cost of what it
/// shows, not of what it had to look at.
///
/// A commit `shallow` lists gets no parents, as its node does
/// ([`RepoNodes`]), so a row's parents and its links agree.
pub(super) fn hydrate(
    repo: &gix::Repository,
    id: Oid,
    shallow: &ShallowList,
) -> Result<Commit, WalkError> {
    let commit = node::find_commit(repo, id)?;
    let boundary = !shallow.is_empty() && shallow.contains(&id);
    let unreadable = |source: gix::objs::decode::Error| NodeError::Unreadable {
        id,
        source: Box::new(source),
    };

    let parents = commit
        .parent_ids()
        .filter(|_| !boundary)
        .map(|parent| {
            Oid::from_bytes(parent.as_bytes()).map_err(|source| NodeError::Malformed { id, source })
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
    let shallow = Arc::new(ShallowList::read(handle)?);
    let table =
        CommitTable::open_with(handle, &shallow).map_err(|source| WalkError::CommitGraph {
            source: Box::new(source),
        })?;
    let nodes = RepoNodes::new(handle.gix(), Arc::new(table), shallow);

    let mut merges = 0;
    let mut seen = HandleSet::new();
    let mut at = Some(nodes.handle(&tip));
    while let Some(commit) = at {
        // A first-parent chain cannot revisit a commit in a well-formed
        // repository, but a corrupt one must not spin here for ever.
        if !seen.insert(commit) {
            break;
        }
        let node = nodes.node(commit)?;
        if node.parents.len() > 1 {
            merges += 1;
        }
        at = node.parents.first();
    }
    Ok(merges)
}

/// The default branch's first-parent chain, from `tip` to the root, as
/// handles into `table`.
///
/// Reuses the per-handle node reads [`first_parent_merges`] walks with,
/// rather than hydrating whole commits: the Mistake Radar only needs each
/// commit's identity and parent, never its message or signature. A root
/// commit, or a shallow clone's boundary (parentless, git's rule: see
/// [`crate::shallow`]), ends the chain.
///
/// `stop` is polled once per commit, the same plain flag check
/// [`crate::run::GitInvocation::run`]'s process loop makes — no timer, since
/// this walk never spawns a process to kill.
///
/// # Errors
/// [`WalkError`] if a commit on the chain cannot be read.
pub fn first_parent_chain(
    repo: &RepoHandle,
    table: &Arc<CommitTable>,
    tip: Oid,
    stop: &dyn Stop,
) -> Result<Vec<CommitHandle>, WalkError> {
    let shallow = Arc::new(ShallowList::read(repo)?);
    let nodes = RepoNodes::new(repo.gix(), Arc::clone(table), shallow);
    let mut chain = Vec::new();
    let mut seen = HandleSet::new();
    let mut at = Some(nodes.handle(&tip));
    while let Some(commit) = at {
        if stop.should_stop() {
            break;
        }
        // A first-parent chain cannot revisit a commit in a well-formed
        // repository, but a corrupt one must not spin here for ever.
        if !seen.insert(commit) {
            break;
        }
        chain.push(commit);
        let node = nodes.node(commit)?;
        at = node.parents.first();
    }
    Ok(chain)
}

/// A ref's own reflog: its previous tips, newest first, deduped, capped at
/// `cap` entries.
///
/// A ref with no reflog at all — never checked out, or the repository has
/// `core.logAllRefUpdates` off — is `Ok(vec![])`, not an error: it simply has
/// no history to report, which is a normal state for the Mistake Radar's
/// force-push probe to see.
///
/// # Errors
/// [`ReflogError`] if the ref exists but its reflog cannot be read.
pub fn previous_tips(
    repo: &RepoHandle,
    refname: &RefName,
    cap: usize,
) -> Result<Vec<Oid>, ReflogError> {
    let gix_repo = repo.gix();
    let name = BStr::new(refname.as_bytes());
    let reference = match gix_repo.find_reference(name) {
        Ok(reference) => reference,
        Err(gix::reference::find::existing::Error::NotFound { .. }) => return Ok(vec![]),
        Err(source) => return Err(ReflogError::Unreadable(source.to_string())),
    };
    let mut platform = reference.log_iter();
    let Some(entries) = platform
        .rev()
        .map_err(|source| ReflogError::Unreadable(source.to_string()))?
    else {
        return Ok(vec![]);
    };

    let mut tips = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in entries {
        if tips.len() >= cap {
            break;
        }
        let line = entry.map_err(|source| ReflogError::Unreadable(source.to_string()))?;
        let previous = line.previous_oid;
        if previous.is_null() {
            continue;
        }
        let oid = Oid::from_bytes(previous.as_bytes())
            .map_err(|source| ReflogError::Unreadable(source.to_string()))?;
        if seen.insert(oid) {
            tips.push(oid);
        }
    }
    Ok(tips)
}

/// Why a ref's reflog could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ReflogError {
    /// `gix` could not read the ref or its reflog.
    #[error("{0}")]
    Unreadable(String),
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
    /// `<common_dir>/shallow` exists but could not be read (m7/32): a walk
    /// that ignored it would ask for commits a shallow clone never fetched.
    #[error(transparent)]
    Shallow(#[from] ShallowError),
}
