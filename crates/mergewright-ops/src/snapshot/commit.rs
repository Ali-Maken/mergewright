//! The commit writers: a backup commit over joined trees, a scratch commit
//! for a forecast, and the ref that moves to the after-snapshot.

use std::path::Path;

use mergewright_core::journal::{EntryId, IndexState};
use mergewright_core::oid::Oid;
use mergewright_core::on_disk::MESSAGE_TAG;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::Stop;

use super::SnapshotError;
use super::TempIndex;
use super::plumbing::{self, commit_tree, mktree, update_ref};
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
///
/// It never holds a `raw/` tree: only [`take`](super::take)'s before-snapshot keeps one
/// (ADR-0052, "only before").
pub(crate) fn commit_trees(
    repo: &RepoHandle,
    index: Option<IndexState>,
    worktree_tree: Option<Oid>,
    id: &EntryId,
    label: &str,
    parents: &[Oid],
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let joined = mktree(repo, index, worktree_tree, None, stop)?;
    commit_joined(repo, joined, id, label, parents, stop)
}

/// A backup commit over a joined tree, with the message every backup carries.
pub(super) fn commit_joined(
    repo: &RepoHandle,
    joined: Oid,
    id: &EntryId,
    label: &str,
    parents: &[Oid],
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    commit_tree(
        repo,
        joined,
        parents,
        &format!("{MESSAGE_TAG} {id} {label}"),
        stop,
    )
}

/// `git stash create` over a temporary copy of the index, as
/// [`index_state`](super::index_state) copies it: the stash commit of the tree now, or `None`
/// when git found nothing to stash. Objects only — `stash create` refreshes
/// the index it reads, and that is the copy, so `.git/index` is not
/// rewritten (ADR-0045 §8, the put-back forecast).
pub(crate) fn stash_create_on_copy(
    repo: &RepoHandle,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<Option<Oid>, SnapshotError> {
    let temp = TempIndex::copied_from(repo, scratch)?;
    let output = mergewright_git::run::GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", temp.path())
        .arg(mergewright_git::run::GitArg::Flag("stash"))
        .arg(mergewright_git::run::GitArg::Flag("create"))
        .run(stop)
        .map_err(plumbing::stopped)?;
    let printed = String::from_utf8_lossy(&output.stdout);
    match printed.trim() {
        "" => Ok(None),
        line => plumbing::oid_from(line.as_bytes(), "stash create").map(Some),
    }
}

/// A commit over `tree` with `parents`, made with Mergewright's own identity and
/// referenced by nothing: a loose object the next `gc` prunes. The put-back
/// forecast needs its landed tree as a commit, because `merge-tree` on the
/// oldest git it runs on (2.40) takes commits for its branches.
pub(crate) fn scratch_commit(
    repo: &RepoHandle,
    tree: Oid,
    parents: &[Oid],
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    commit_tree(
        repo,
        tree,
        parents,
        &format!("{MESSAGE_TAG}: put-back forecast"),
        stop,
    )
}

/// Point an existing backup ref at another commit (the after-snapshot).
pub(crate) fn move_backup_ref(
    repo: &RepoHandle,
    name: &str,
    to: Oid,
    stop: &dyn Stop,
) -> Result<(), SnapshotError> {
    update_ref(repo, name, to, stop)
}
