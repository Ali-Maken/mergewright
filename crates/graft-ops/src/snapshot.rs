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
//! restore reads each by name. A before-snapshot adds `raw/` beside them when
//! it keeps bytes git would not give back (ADR-0052).
//!
//! Taking a snapshot lives here. Its other concerns are submodules, each
//! public item re-exported so every `snapshot::` path reads as before:
//! `stage_tree` (an unmerged index as one tree, and back), `chain` (the
//! backup chain under a live entry), `index_paths` (which paths two states
//! differ on), `raw` (the bytes git would not give back, ADR-0052) and
//! `plumbing` (the temporary index and the `git` they share).

mod chain;
mod commit;
mod index_paths;
mod plumbing;
mod raw;
mod stage_tree;

pub(crate) use chain::chain_backup;
pub use chain::{LinkKind, backup_chain, current_step_chain, recorded_index_state};
pub(crate) use commit::{commit_trees, move_backup_ref, scratch_commit, stash_create_on_copy};
pub use index_paths::index_paths_changed;
pub(crate) use index_paths::{index_paths, tracked_paths_now};
pub use plumbing::{BACKUP_AUTHOR_EMAIL, BACKUP_AUTHOR_NAME};
pub use raw::raw_record;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_core::journal::{EntryId, IndexState, Touches};
use graft_core::oid::Oid;
use graft_core::on_disk::Generation;
use graft_git::repo::RepoHandle;
use graft_git::run::{RunError, Stop};

use commit::commit_joined;
pub(crate) use plumbing::{TempIndex, c_quoted, path_of};
use plumbing::{add_all, mktree, try_write_tree, update_ref, write_tree};
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
    /// The bytes on disk of the uncommitted files git's checkout of
    /// `worktree_tree` would not give back ([`raw_record`], ADR-0052);
    /// `None` when there are none.
    pub worktree_raw: Option<Oid>,
    /// `{ index/ | unmerged/ → the index state, raw/ → worktree_raw when
    /// kept, worktree/ → worktree_tree }`, committed.
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
    /// An in-process read `raw_record` makes instead of a `git` spawn failed
    /// (m7-power-tools/30r): an object or the config would not read. Never
    /// holds `ReadError::Stopped`; a stop is [`SnapshotError::Stopped`].
    #[error("{0}")]
    Read(graft_git::snapshot_reads::ReadError),
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
    /// git could not check out `path` while the raw record compared bytes,
    /// named by the file (m8-polish-and-ship/20). The cause is git's own
    /// words in `error`: a failing smudge filter names itself there, and so
    /// does anything else — a missing blob, a full disk — so the words never
    /// guess (m8/21b-3; was `Filter`, whose words blamed a filter always).
    #[error("{path}: git could not check it out: {error}")]
    CheckOut {
        /// The file, relative to the worktree root, lossy.
        path: String,
        /// git's own failure.
        error: RunError,
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

/// `<prefix>backup/<worktree>/<ulid>` — shared across worktrees by git,
/// namespaced per worktree by us (ADR-0010). `<prefix>` is the repository's
/// names in force: `refs/graft/` or `refs/tributary/` (ADR-0057, Amendment).
pub fn backup_ref_name(names: Generation, worktree: &str, id: &EntryId) -> String {
    format!("{}backup/{worktree}/{id}", names.ref_prefix())
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

/// [`index_state`], then `then` run over the **same** temporary copy of the
/// index before it is removed: a read that needs the index as it stands
/// (`apply --cached --check`, the stash preview's `--index` rule) without
/// touching the real one. `then` gets the state and the copy's path.
pub(crate) fn index_state_then<T>(
    repo: &RepoHandle,
    scratch: &Path,
    stop: &dyn Stop,
    then: impl FnOnce(&IndexState, &Path) -> T,
) -> Result<(IndexState, T), SnapshotError> {
    let temp = TempIndex::copied_from(repo, scratch)?;
    let state = index_state_of(repo, temp.path(), scratch, stop)?;
    let after = then(&state, temp.path());
    Ok((state, after))
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
///
/// `head` is `HEAD`'s commit (`None` while unborn): the files that differ
/// from it are the ones [`raw_record`] may keep raw, joined as `raw/` beside
/// `worktree/` only when it keeps any, so a snapshot with nothing to keep is
/// laid out exactly as every backup commit since M3.
// Eight: the backup's name (`id`, `worktree`), its message (`label`), what it
// keeps alive (`parents`) and what counts as uncommitted (`head`) are each the
// caller's to say, and a struct for one caller would only rename them.
#[allow(
    clippy::too_many_arguments,
    reason = "each argument is the caller's to say; a struct for one caller would only rename them"
)]
pub fn take(
    repo: &RepoHandle,
    scratch: &Path,
    id: &EntryId,
    worktree: &str,
    label: &str,
    parents: &[Oid],
    head: Option<Oid>,
    stop: &dyn Stop,
) -> Result<Snapshot, SnapshotError> {
    let (index, worktree_tree) = both_trees(repo, scratch, stop)?;
    // `take`'s spawns are not journaled (it logs none of its own), so
    // `raw_record`'s list is dropped.
    let mut commands = Vec::new();
    let worktree_raw = raw_record(repo, scratch, stop, head, worktree_tree, &mut commands)?;
    let joined = mktree(repo, Some(index), Some(worktree_tree), worktree_raw, stop)?;
    let commit = commit_joined(repo, joined, id, label, parents, stop)?;
    let name = backup_ref_name(repo.on_disk_names(), worktree, id);
    update_ref(repo, &name, commit, stop)?;
    Ok(Snapshot {
        index,
        worktree_tree,
        worktree_raw,
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

#[cfg(test)]
mod tests;
