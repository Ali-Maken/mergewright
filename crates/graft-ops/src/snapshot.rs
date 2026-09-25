//! The backup snapshot: both trees, one commit, one ref (ADR-0009).
//!
//! `git stash create` silently drops untracked files, so PROMPT.md §3 rules
//! it out. What Graft takes instead is two trees:
//!
//! - the **index tree**, the staged state exactly as it stands;
//! - the **worktree tree**, every tracked file's working-tree content plus
//!   every untracked file (design note change 1 — §6.3's "index tree plus
//!   untracked blobs" loses an unstaged edit to a tracked file, which is the
//!   commonest thing a discard destroys).
//!
//! Both come from a **copy** of the real index at
//! `<git-path graft>/tmp-index-*`. The copy carries git's stat cache, so
//! `add -A` re-hashes only the files whose stat data moved: 15 ms against
//! 236 ms for a fresh `read-tree` on a 5,000-file tree (git 2.54). Nothing
//! here ever writes the real `.git/index` — a read that rewrote the index
//! would fight the user's own git (M2 review round 2).
//!
//! The two trees are joined by `mktree` into `{ index/, worktree/ }` and
//! committed under `refs/graft/backup/<worktree>/<ulid>` (ADR-0010's
//! namespacing, its Correction's `<ulid>`), so both stay reachable and a
//! restore reads each by name.
//!
//! Taking a snapshot lives here. Its other concerns are submodules, each
//! public item re-exported so every `snapshot::` path reads as before:
//! `stage_tree` (an unmerged index as one tree, and back), `chain` (the
//! backup chain under a live entry), `index_paths` (which paths two states
//! differ on) and `plumbing` (the temporary index and the `git` they share).

mod chain;
mod index_paths;
mod plumbing;
mod stage_tree;

pub use chain::{LinkKind, backup_chain, chain_backup, current_step_chain, recorded_index_state};
pub use index_paths::index_paths_changed;
pub(crate) use index_paths::{index_paths, tracked_paths_now};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_core::journal::{EntryId, IndexState, Touches};
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{RunError, Stop};

use plumbing::{TempIndex, add_all, commit_tree, mktree, try_write_tree, update_ref, write_tree};
use stage_tree::{stage_tree, tree_stage_entries};

/// How much of the repository a snapshot has to cover before one operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotScope {
    /// Nothing to back up: the operation moves refs only.
    None,
    /// Both trees (ADR-0009: unconditional for anything touching the index
    /// or the working tree).
    Full,
}

/// One snapshot: the trees and the commit that holds them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// The staged state. A half-finished merge has no tree `write-tree` will
    /// give, so it is captured as the **stage tree** instead
    /// ([`IndexState::Unmerged`]) — never as an absence (m6-conflicts/08).
    pub index: IndexState,
    /// Tracked working-tree content plus untracked files.
    pub worktree_tree: Oid,
    /// `{ index/ | unmerged/ → the index state, worktree/ → worktree_tree }`,
    /// committed.
    pub commit: Oid,
}

/// One `(mode, oid, stage)` an index holds for a path.
///
/// A merged path has exactly one, at stage 0. A conflicted one has up to
/// three: 1 the merge base, 2 ours, 3 theirs — and fewer when a side has no
/// version (`UD`, `AU`, …).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StageEntry {
    /// The file mode, as git prints it (`100644`, `100755`, `120000`).
    pub mode: u32,
    /// The blob.
    pub oid: Oid,
    /// 0, 1, 2 or 3.
    pub stage: u8,
}

/// Why a snapshot could not be taken.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    /// A `git` command failed.
    #[error("{0}")]
    Run(#[from] RunError),
    /// The index could not be copied, or the temporary copy could not be
    /// removed.
    #[error("snapshot scratch: {0}")]
    Io(#[from] std::io::Error),
    /// `git` printed something that is not an object id — or a record whose
    /// shape the parser could not read one out of.
    #[error("git printed {what} where an object id belongs: {printed}")]
    NotAnOid {
        /// Which command.
        what: &'static str,
        /// What it printed, first line, lossy.
        printed: String,
    },
    /// A backup commit's `Graft-Link:` line names a kind this build does
    /// not know (m6-conflicts/22) — written by a later Graft, so guessing
    /// would bound a conflict set wrongly.
    #[error("backup commit {commit} has an unknown link kind: {printed}")]
    UnknownLink {
        /// The backup commit.
        commit: Oid,
        /// The line as written, lossy.
        printed: String,
    },
    /// The caller asked to stop.
    #[error("stopped")]
    Stopped,
}

/// Hard choice 3: today, anything touching the index or the working tree
/// takes the full snapshot, and nothing else takes one.
///
/// ADR-0009 stands — detection can miss, a snapshot cannot — so this is
/// deliberately not clever. It is one function so that narrowing it later
/// (index-only ops to an index tree alone) is one edit with numbers behind
/// it, measured on the heavy fixture, never a silent drift.
pub fn scope(touches: &Touches) -> SnapshotScope {
    if touches.index || touches.worktree {
        SnapshotScope::Full
    } else {
        SnapshotScope::None
    }
}

/// `refs/graft/backup/<worktree>/<ulid>` — shared across worktrees by git,
/// namespaced per worktree by us (ADR-0010).
pub fn backup_ref_name(worktree: &str, id: &EntryId) -> String {
    format!("refs/graft/backup/{worktree}/{id}")
}

/// The staged state, read without touching the real index.
///
/// A merged index is the plain tree `write-tree` gives. An unmerged one —
/// what a stopped merge or a conflicted stash pop leaves — has no such tree
/// at all, so it is written as the **stage tree**: every index entry moved to
/// `<stage>/<path>` at stage 0 on a second temporary index, which `write-tree`
/// then accepts (m6-conflicts/08 decision 1).
///
/// No blob is written by either path. Every stage's blob is already an object
/// — ours is `HEAD`'s, theirs is `MERGE_HEAD`'s or the stash commit's, the
/// base is the merge base's — and the tree only makes them reachable from the
/// backup ref.
pub fn index_state(
    repo: &RepoHandle,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<IndexState, SnapshotError> {
    let temp = TempIndex::copied_from(repo, scratch)?;
    index_state_of(repo, temp.path(), scratch, stop)
}

/// [`index_state`] over an index copy the caller already holds.
fn index_state_of(
    repo: &RepoHandle,
    index: &Path,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<IndexState, SnapshotError> {
    match try_write_tree(repo, index, stop)? {
        Some(tree) => Ok(IndexState::Merged(tree)),
        None => Ok(IndexState::Unmerged {
            unmerged: stage_tree(repo, index, scratch, stop)?,
        }),
    }
}

/// Tracked working-tree content plus untracked files, as a tree.
pub fn worktree_tree(
    repo: &RepoHandle,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let temp = TempIndex::copied_from(repo, scratch)?;
    add_all(repo, temp.path(), stop)?;
    write_tree(repo, temp.path(), stop)
}

/// Both trees, the joined tree, the commit and the backup ref.
///
/// `parents` are the commits the snapshot must keep reachable — the pins of
/// an operation that is about to make a stash commit unreachable — plus, for
/// an after-snapshot, the before-commit.
///
/// One index copy serves both trees: `write-tree` gives the index tree, then
/// `add -A` over the same copy and a second `write-tree` gives the worktree
/// tree. Two copies would pay the stat walk twice for nothing.
pub fn take(
    repo: &RepoHandle,
    scratch: &Path,
    id: &EntryId,
    worktree: &str,
    label: &str,
    parents: &[Oid],
    stop: &dyn Stop,
) -> Result<Snapshot, SnapshotError> {
    let (index, worktree_tree) = both_trees(repo, scratch, stop)?;
    let commit = commit_trees(
        repo,
        Some(index),
        Some(worktree_tree),
        id,
        label,
        parents,
        stop,
    )?;
    let name = backup_ref_name(worktree, id);
    update_ref(repo, &name, commit, stop)?;
    Ok(Snapshot {
        index,
        worktree_tree,
        commit,
    })
}

/// Both trees from one index copy: `write-tree` (or the stage tree) for the
/// index side, then `add -A` over the same copy and a second `write-tree` for
/// the working tree. Two copies would pay the stat walk twice for nothing.
///
/// The index side is read **before** `add -A`, which resolves every conflict
/// in the copy: after it, the stages are gone.
fn both_trees(
    repo: &RepoHandle,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<(IndexState, Oid), SnapshotError> {
    let temp = TempIndex::copied_from(repo, scratch)?;
    let index = index_state_of(repo, temp.path(), scratch, stop)?;
    add_all(repo, temp.path(), stop)?;
    let worktree_tree = write_tree(repo, temp.path(), stop)?;
    Ok((index, worktree_tree))
}

/// The `(mode, oid, stage)` triples one path has in a recorded index state;
/// empty when the state does not hold the path at all.
///
/// # Errors
/// [`SnapshotError`] when `ls-tree` will not run or prints a record with no
/// object id in it.
pub fn stage_entries(
    repo: &RepoHandle,
    index: IndexState,
    path: &Path,
    stop: &dyn Stop,
) -> Result<Vec<StageEntry>, SnapshotError> {
    let held = stage_entries_for(
        repo,
        index,
        std::slice::from_ref(&path.to_path_buf()),
        stop,
        None,
    )?;
    Ok(held.into_values().next().unwrap_or_default())
}

/// The same for several paths at once: one `ls-tree`, not one per path,
/// its command line pushed to `log` when there is one.
pub(crate) fn stage_entries_for(
    repo: &RepoHandle,
    index: IndexState,
    paths: &[PathBuf],
    stop: &dyn Stop,
    log: Option<&mut Vec<String>>,
) -> Result<BTreeMap<PathBuf, Vec<StageEntry>>, SnapshotError> {
    tree_stage_entries(repo, index, Some(paths), stop, log)
}

/// Every path a recorded index state holds at a stage above 0 — the paths
/// that state records as conflicted — in path order. Empty for a merged
/// state, which records resolutions, not conflicts (m6-conflicts/09).
pub(crate) fn conflicted_paths(
    repo: &RepoHandle,
    index: IndexState,
    stop: &dyn Stop,
) -> Result<Vec<PathBuf>, SnapshotError> {
    if !index.is_unmerged() {
        return Ok(Vec::new());
    }
    Ok(tree_stage_entries(repo, index, None, stop, None)?
        .into_iter()
        .filter(|(_, entries)| entries.iter().any(|entry| entry.stage >= 1))
        .map(|(path, _)| path)
        .collect())
}

/// Join the trees into one backup commit, without touching any ref.
///
/// `take` uses it for the before-snapshot; the protocol uses it again for the
/// after-snapshot, whose parents are the before-commit and every commit the
/// entry's after-fields name.
///
/// Either side may be absent: an after-state read that failed for one should
/// still keep the other reachable. A commit over neither is still worth
/// making — its parents are what keeps the entry's recorded commits alive.
///
/// The index side lands under `index/` when it is merged and `unmerged/` when
/// it is the stage tree, **never both**. A merged commit's layout is therefore
/// byte-identical to every backup ref written since M3, and the shape is
/// readable from the commit alone rather than only from the journal.
pub fn commit_trees(
    repo: &RepoHandle,
    index: Option<IndexState>,
    worktree_tree: Option<Oid>,
    id: &EntryId,
    label: &str,
    parents: &[Oid],
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let joined = mktree(repo, index, worktree_tree, stop)?;
    commit_tree(repo, joined, parents, &format!("graft {id} {label}"), stop)
}

/// `git stash create` over a temporary copy of the index, as
/// [`index_state`] copies it: the stash commit of the tree now, or `None`
/// when git found nothing to stash. Objects only — `stash create` refreshes
/// the index it reads, and that is the copy, so `.git/index` is not
/// rewritten (ADR-0045 §8, the put-back forecast).
pub(crate) fn stash_create_on_copy(
    repo: &RepoHandle,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<Option<Oid>, SnapshotError> {
    let temp = TempIndex::copied_from(repo, scratch)?;
    let output = graft_git::run::GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", temp.path())
        .arg(graft_git::run::GitArg::Flag("stash"))
        .arg(graft_git::run::GitArg::Flag("create"))
        .run(stop)
        .map_err(plumbing::stopped)?;
    let printed = String::from_utf8_lossy(&output.stdout);
    match printed.trim() {
        "" => Ok(None),
        line => plumbing::oid_from(line.as_bytes(), "stash create").map(Some),
    }
}

/// A commit over `tree` with `parents`, made with Graft's own identity and
/// referenced by nothing: a loose object the next `gc` prunes. The put-back
/// forecast needs its landed tree as a commit, because `merge-tree` on the
/// oldest git it runs on (2.40) takes commits for its branches.
pub(crate) fn scratch_commit(
    repo: &RepoHandle,
    tree: Oid,
    parents: &[Oid],
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    commit_tree(repo, tree, parents, "graft: put-back forecast", stop)
}

/// Point an existing backup ref at another commit (the after-snapshot).
pub fn move_backup_ref(
    repo: &RepoHandle,
    name: &str,
    to: Oid,
    stop: &dyn Stop,
) -> Result<(), SnapshotError> {
    update_ref(repo, name, to, stop)
}

#[cfg(test)]
mod tests;
