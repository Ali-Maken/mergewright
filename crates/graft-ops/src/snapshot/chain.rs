//! The backup chain (m6-conflicts/08, Decision 2): a commit hung off a live
//! entry's backup ref before each folded step, and the walk that reads the
//! recorded index states back out of it, newest first.

use graft_core::journal::{IndexState, JournalEntry};
use graft_core::oid::Oid;
use graft_core::on_disk::OnDiskNames;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, Stop};

use crate::apply::Context;

use super::plumbing::{read_ref, run_git, split_nul};
use super::stage_tree::ls_tree_entry;
use super::{SnapshotError, both_trees, commit_trees, move_backup_ref};

/// Why a link of the backup chain was written (m6-conflicts/22). Recorded in
/// the backup commit's message as its last line, `Tributary-Link: step` or
/// `Tributary-Link: resume` (`Graft-Link:` before the rename, still read), so
/// the chain is readable without the journal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    /// Before a step folded into the live operation (a resolver write).
    /// Also what a link written before m6-conflicts/22, with no
    /// `Graft-Link:` line, reads as.
    Step,
    /// Before `resume` hands the operation back to git (`--continue`,
    /// `--skip`, `--abort`, `commit`). For a multi-step operation it marks
    /// where the previous step ended: everything newer is the current step's.
    Resume,
}

impl LinkKind {
    /// The line a new link is written with. Always the new key, whatever
    /// names the repository is using: the reader takes both, forever, so
    /// nothing depends on which one a commit carries (ADR-0057, Amendment).
    pub(super) fn trailer(self) -> String {
        let key = OnDiskNames::New.link_trailer();
        match self {
            LinkKind::Step => format!("{key} step"),
            LinkKind::Resume => format!("{key} resume"),
        }
    }
}

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
/// `link` says why it was written, and goes into the commit's message
/// ([`LinkKind`]).
///
/// `Ok(None)` when the entry names no backup ref: there is nothing to chain
/// onto, which is exactly the case of an operation begun outside Tributary.
///
/// # Errors
/// [`SnapshotError`] — the stop, or a `git` that would not run. Nothing is
/// written when it fails: the ref moves last.
pub(crate) fn chain_backup(
    ctx: &Context<'_>,
    entry: &JournalEntry,
    link: LinkKind,
) -> Result<Option<Oid>, SnapshotError> {
    let Some(name) = entry.backup_ref.as_deref() else {
        return Ok(None);
    };
    if ctx.stop.should_stop() {
        return Err(SnapshotError::Stopped);
    }
    let (index, worktree_tree) = both_trees(ctx.repo, ctx.scratch(), ctx.stop)?;
    let parents: Vec<Oid> = read_ref(ctx.repo, name, ctx.stop)?.into_iter().collect();
    let label = format!("step within: {}\n\n{}\n", entry.description, link.trailer());
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
/// (`commit_trees`), so the **name** of the index subtree is what says which
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

/// A backup ref's chain of recorded index states, **newest first**, each with
/// the [`LinkKind`] its commit was written as.
///
/// The ref's own commit, then its first parent, and so on — the single linear
/// chain `chain_backup` keeps: pre-operation → before step 1 → … The walk
/// stops at the first commit that is not a backup commit
/// ([`recorded_index_state`] gives `None`), which is where the pre-operation
/// snapshot's parents hand over to the user's own history. An absent ref is
/// an empty chain, not a failure. The pre-operation snapshot and any link
/// written before m6-conflicts/22 carry no `Graft-Link:` line and read as
/// [`LinkKind::Step`].
///
/// This is what `d` reads after `⏎`: the stages git dropped when it staged
/// the path are in one of these, and nothing here re-runs the merge.
///
/// # Errors
/// [`SnapshotError`] — the stop, a `git` that would not run, or a
/// `Graft-Link:` line naming a kind this build does not know.
pub fn backup_chain(
    repo: &RepoHandle,
    name: &str,
    stop: &dyn Stop,
) -> Result<Vec<(IndexState, LinkKind)>, SnapshotError> {
    walk(repo, name, Bound::Whole, stop)
}

/// The recorded index states of the **current step** only, newest first:
/// [`backup_chain`] cut at its first [`LinkKind::Resume`] link, which is left
/// out. A `Resume` link is written as the previous step is handed back to
/// git, so it and every older link belong to a step that is over
/// (m6-conflicts/22). A merge has one step and no `Resume` link before its
/// finish, so for it this is the whole chain.
///
/// Every reader that asks "what did *this* step conflict on" goes through
/// here — the conflict set and `d`'s stage source — so a finished step's
/// conflict can neither be listed nor restored into the current one (review
/// m6-amend-10-22 M1).
///
/// # Errors
/// As [`backup_chain`].
pub fn current_step_chain(
    repo: &RepoHandle,
    name: &str,
    stop: &dyn Stop,
) -> Result<Vec<IndexState>, SnapshotError> {
    let chain = walk(repo, name, Bound::CurrentStep, stop)?;
    Ok(chain.into_iter().map(|(state, _)| state).collect())
}

/// How far [`walk`] goes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bound {
    /// To the pre-operation snapshot.
    Whole,
    /// To the first `Resume` link, not included.
    CurrentStep,
}

/// The one chain walk both public readers share.
fn walk(
    repo: &RepoHandle,
    name: &str,
    bound: Bound,
    stop: &dyn Stop,
) -> Result<Vec<(IndexState, LinkKind)>, SnapshotError> {
    let mut chain = Vec::new();
    let mut at = read_ref(repo, name, stop)?;
    while let Some(commit) = at {
        let Some(state) = recorded_index_state(repo, commit, stop)? else {
            break;
        };
        let (parent, link) = parent_and_link(repo, commit, stop)?;
        if bound == Bound::CurrentStep && link == LinkKind::Resume {
            break;
        }
        chain.push((state, link));
        at = parent;
    }
    Ok(chain)
}

/// A backup commit's first parent (`None` for a root) and its link kind,
/// from one `cat-file commit`.
fn parent_and_link(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<(Option<Oid>, LinkKind), SnapshotError> {
    let output = run_git(
        repo,
        vec![
            GitArg::Flag("cat-file"),
            GitArg::Flag("commit"),
            GitArg::Oid(commit),
        ],
        None,
        stop,
    )?;
    let text = String::from_utf8_lossy(&output.stdout);
    let (headers, message) = text.split_once("\n\n").unwrap_or((&text, ""));
    let parent = match headers
        .lines()
        .find_map(|line| line.strip_prefix("parent "))
    {
        Some(hex) => Some(Oid::from_hex(hex).map_err(|_| SnapshotError::NotAnOid {
            what: "cat-file commit",
            printed: hex.to_owned(),
        })?),
        None => None,
    };
    Ok((parent, link_of(commit, message)?))
}

/// The kind a backup commit's message records: its last non-empty line, when
/// that is a `Tributary-Link:` or `Graft-Link:` line (either key, whatever
/// names are in force: backup commits are immutable); [`LinkKind::Step`]
/// when there is none (a link written before the kind was, or the
/// pre-operation snapshot).
pub(super) fn link_of(commit: Oid, message: &str) -> Result<LinkKind, SnapshotError> {
    let last = message.lines().rev().find(|line| !line.trim().is_empty());
    let Some((line, value)) = last.and_then(|line| {
        OnDiskNames::ALL
            .iter()
            .find_map(|names| line.strip_prefix(names.link_trailer()))
            .map(|value| (line, value))
    }) else {
        return Ok(LinkKind::Step);
    };
    match value.trim() {
        "step" => Ok(LinkKind::Step),
        "resume" => Ok(LinkKind::Resume),
        _ => Err(SnapshotError::UnknownLink {
            commit,
            printed: line.to_owned(),
        }),
    }
}
