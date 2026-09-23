//! The backup chain (m6-conflicts/08, Decision 2): a commit hung off a live
//! entry's backup ref before each folded step, and the walk that reads the
//! recorded index states back out of it, newest first.

use graft_core::journal::{IndexState, JournalEntry};
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, Stop};

use crate::apply::Context;

use super::plumbing::{read_ref, run_git, split_nul};
use super::stage_tree::ls_tree_entry;
use super::{SnapshotError, both_trees, commit_trees, move_backup_ref};

/// Decision 2's chained commit: the repository as it stands **now**, hung off
/// the entry's backup ref, with no journal line of its own.
///
/// A step taken inside a live operation folds into that operation's one entry
/// (M5 hard choice 2), but git's own `add` drops stages 1–3 for the path and
/// nothing else keeps them. So every folded step and every `resume` takes one
/// of these first, parented on the ref's current commit: the ref stays a
/// single chain — pre-op → before step 1 → … → after — and `d`
/// ([`crate::operation::Operation::RestartConflict`]) reads the stages back
/// out of it instead of re-running the merge.
///
/// `Ok(None)` when the entry names no backup ref: there is nothing to chain
/// onto, which is exactly the case of an operation begun outside Graft.
///
/// # Errors
/// [`SnapshotError`] — the stop, or a `git` that would not run. Nothing is
/// written when it fails: the ref moves last.
pub fn chain_backup(ctx: &Context<'_>, entry: &JournalEntry) -> Result<Option<Oid>, SnapshotError> {
    let Some(name) = entry.backup_ref.as_deref() else {
        return Ok(None);
    };
    if ctx.stop.should_stop() {
        return Err(SnapshotError::Stopped);
    }
    let (index, worktree_tree) = both_trees(ctx.repo, ctx.scratch(), ctx.stop)?;
    let parents: Vec<Oid> = read_ref(ctx.repo, name, ctx.stop)?.into_iter().collect();
    let label = format!("step within: {}", entry.description);
    let commit = commit_trees(
        ctx.repo,
        Some(index),
        Some(worktree_tree),
        &entry.id,
        &label,
        &parents,
        ctx.stop,
    )?;
    move_backup_ref(ctx.repo, name, commit, ctx.stop)?;
    Ok(Some(commit))
}

/// The index state a backup commit records, or `None` when the commit is not
/// one of Graft's backups.
///
/// A backup commit's tree is `{ index/ | unmerged/, worktree/ }`
/// ([`commit_trees`]), so the **name** of the index subtree is what says which
/// shape it holds — the commit is readable on its own, without the journal
/// line that named it. A commit with neither subtree is somebody's own
/// history, which is how [`backup_chain`] knows it has walked off the end of
/// the chain.
///
/// # Errors
/// [`SnapshotError`] when `ls-tree` will not run or prints a record with no
/// object id in it.
pub fn recorded_index_state(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Option<IndexState>, SnapshotError> {
    let output = run_git(
        repo,
        vec![
            GitArg::Flag("ls-tree"),
            GitArg::Flag("-z"),
            GitArg::Oid(commit),
        ],
        None,
        stop,
    )?;
    for record in split_nul(&output.stdout) {
        // The name decides the shape, so the entry is read as a plain tree
        // record whatever it turns out to be.
        let (name, entry) = ls_tree_entry(&record, false)?;
        match name.as_os_str().as_encoded_bytes() {
            b"index" => return Ok(Some(IndexState::Merged(entry.oid))),
            b"unmerged" => {
                return Ok(Some(IndexState::Unmerged {
                    unmerged: entry.oid,
                }));
            }
            _ => {}
        }
    }
    Ok(None)
}

/// A backup ref's chain of recorded index states, **newest first**.
///
/// The ref's own commit, then its first parent, and so on — the single linear
/// chain [`chain_backup`] keeps: pre-operation → before step 1 → … The walk
/// stops at the first commit that is not a backup commit
/// ([`recorded_index_state`] gives `None`), which is where the pre-operation
/// snapshot's parents hand over to the user's own history. An absent ref is
/// an empty chain, not a failure.
///
/// This is what `d` reads after `⏎`: the stages git dropped when it staged
/// the path are in one of these, and nothing here re-runs the merge.
///
/// # Errors
/// [`SnapshotError`] — the stop, or a `git` that would not run.
pub fn backup_chain(
    repo: &RepoHandle,
    name: &str,
    stop: &dyn Stop,
) -> Result<Vec<IndexState>, SnapshotError> {
    let mut chain = Vec::new();
    let mut at = read_ref(repo, name, stop)?;
    while let Some(commit) = at {
        let Some(state) = recorded_index_state(repo, commit, stop)? else {
            break;
        };
        chain.push(state);
        at = first_parent(repo, commit, stop)?;
    }
    Ok(chain)
}

/// A commit's first parent, or `None` when it is a root.
///
/// `rev-list --parents -n 1` prints `<commit> <parent>…`, so one spawn
/// answers it without `rev-parse <c>^1` failing loudly on a root commit.
fn first_parent(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Option<Oid>, SnapshotError> {
    let output = run_git(
        repo,
        vec![
            GitArg::Flag("rev-list"),
            GitArg::Flag("--parents"),
            GitArg::Flag("-n"),
            GitArg::Flag("1"),
            GitArg::Oid(commit),
        ],
        None,
        stop,
    )?;
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(parent) = text.split_whitespace().nth(1) else {
        return Ok(None);
    };
    Oid::from_hex(parent)
        .map(Some)
        .map_err(|_| SnapshotError::NotAnOid {
            what: "rev-list --parents",
            printed: parent.to_owned(),
        })
}
