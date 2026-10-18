//! The operation shape: one enum, four properties (M3 brief point 1).
//!
//! Every write Mergewright makes is an [`Operation`] with, per kind, its
//! preconditions ([`Operation::check`]), its typed `git` invocations
//! ([`Operation::plan`]), what it changes ([`Operation::touches`] — which
//! decides the snapshot) and its inverse ([`crate::undo::inverse`], computed
//! from the journal entry, not from the operation). Everything here is pure:
//! the facts come in as [`RepoFacts`], nothing spawns `git`, nothing reads a
//! file, so every rule is testable without a repository.
//!
//! **Seam rule:** `ops` decides, `app` wires. `app` never builds a
//! `GitInvocation` for a write, never touches the journal file, never chooses
//! a snapshot; it turns a key into an `Operation` and hands it over.

mod check;
mod choice;
mod commit;
mod conflict;
mod explain;
mod finish_merge;
mod history;
mod queries;
mod recover;
mod refusal;
mod remote;
mod rewrite;
mod stage;
mod stash;
mod types;
mod words;

use std::collections::BTreeMap;
use std::path::PathBuf;

use mergewright_core::error::RefNameRule;
use mergewright_core::journal::{HeadRef, OperationKind, PutBackWhy, Touches};
use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::remote::{Remote, TrackedUpstream};
use mergewright_git::run::GitArg;
use mergewright_git::stash::StashEntry;
use mergewright_git::state::InProgress;
use mergewright_git::status::{UnmergedHow, WorkingTreeStatus};
use mergewright_git::version::GitVersion;

use crate::keys::{KeyNames, OpKey, clause, continue_or_abort, then};
use crate::rewrite::EditStop;

pub use choice::{FileChoice, OutWhy, first_choice, looks_like_secrets};
pub use conflict::{OneSided, ResolveWholeParams, RestartKind, RestartParams, Side};
pub(crate) use conflict::{check_restart, shape_of_stages};
pub(crate) use finish_merge::stage_solved_args;
pub use finish_merge::{AbortMergeParams, FinishMergeParams, MergeEnd};
pub use history::{
    BranchCreateParams, BranchFromStashParams, CheckoutParams, CheckoutTarget, FastForward,
    MergeParams, OnEdits, PickParams, RebaseParams, RefDeleteParams, ResetMode, ResetParams,
    ResetTarget, RevertParams, Source, TagCreateParams, Tip,
};
pub(crate) use history::{
    branch_ref, creates_at_head, merge_subject, rebase_invocation, short, short_ref, source_arg,
    tag_ref,
};
pub(crate) use recover::pins as recover_pins;
pub use recover::{FilesEffect, NoPutBack, RecoverParams, RecoverTarget};
pub use refusal::Refusal;
pub use remote::{FetchParams, Lease, PullParams, PushParams, RemoteNames};
use types::amends_at_edit_stop;
pub use types::*;
pub use words::RefusalWords;
use words::{commits_them, dashed, in_the_way_rungs, in_the_way_words, name_paths, ref_words};
pub(crate) use words::{opens_in_progress, runs_in_progress};

/// One write to the repository, as the user asked for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Paths or a patch into the index.
    Stage(Target),
    /// Paths or a patch out of the index.
    Unstage(Target),
    /// Paths or a patch dropped from the working tree (and the index).
    Discard(Target),
    /// A new commit from the index.
    Commit(CommitParams),
    /// `HEAD` rewritten from the index with a new message.
    Amend(CommitParams),
    /// `git stash push`.
    StashPush(StashParams),
    /// `git stash apply --index stash@{index}`; `commit` pins what the list
    /// showed, so a list that moved underneath is refused, never guessed at.
    StashApply {
        /// Position in the stash list, `0` newest.
        index: usize,
        /// The stash commit the list showed at that position.
        commit: Oid,
    },
    /// `git stash pop --index stash@{index}`.
    StashPop {
        /// Position in the stash list, `0` newest.
        index: usize,
        /// The stash commit the list showed at that position.
        commit: Oid,
    },
    /// `git stash drop stash@{index}`.
    StashDrop {
        /// Position in the stash list, `0` newest.
        index: usize,
        /// The stash commit the list showed at that position.
        commit: Oid,
    },
    /// `git fetch`: moves remote-tracking refs, nothing the user typed.
    /// Journaled, never undone.
    Fetch(FetchParams),
    /// A fast-forward pull: fetch the upstream's remote, then `merge
    /// --ff-only` onto its tracking ref. Undoable: the branch and the tree
    /// go back, the tracking ref stays where the fetch left it.
    Pull(PullParams),
    /// `git push`. Journaled, never undone — the remote keeps what it got.
    Push(PushParams),
    /// `git merge` into `HEAD`'s branch. One entry from start to finish: a
    /// conflict leaves it `Pending` until `resume` closes it (m5/07).
    Merge(MergeParams),
    /// `git rebase` of `HEAD`'s branch, through the rewrite engine: git's
    /// `rebase -i` on a todo of picks, the branches built on his following
    /// (ADR-0047 §6). One entry across every stop.
    Rebase(RebaseParams),
    /// A rewrite of `HEAD`'s branch — squash, fixup, reword, reorder, drop,
    /// edit, staged changes put into an older commit — planned by
    /// [`crate::rewrite::plan`] and run as git's `rebase -i` on the todo it
    /// planned, the branches built on his following (ADR-0047). One entry
    /// across every stop.
    Rewrite(crate::rewrite::RewriteParams),
    /// `git cherry-pick` onto `HEAD`.
    CherryPick(PickParams),
    /// `git revert` onto `HEAD`.
    Revert(RevertParams),
    /// `git reset` of `HEAD`'s branch, or `update-ref` of another branch.
    Reset(ResetParams),
    /// `git switch` to a branch or a detached commit.
    Checkout(CheckoutParams),
    /// `git branch <name> <at>`, optionally switched to.
    BranchCreate(BranchCreateParams),
    /// A branch deleted by compare-and-swap.
    BranchDelete(RefDeleteParams),
    /// `git tag`.
    TagCreate(TagCreateParams),
    /// A tag deleted by compare-and-swap.
    TagDelete(RefDeleteParams),
    /// `git stash branch`.
    BranchFromStash(BranchFromStashParams),
    /// One conflicted path resolved whole, to one side: the shapes a block
    /// take cannot reach — no markers, or binary (m6-conflicts/03).
    ResolveWholeFile(ResolveWholeParams),
    /// One conflicted path put back the way git left it: `git checkout
    /// --merge`, the resolver's "start this file over" (m6-conflicts/03).
    RestartConflict(RestartParams),
    /// A merge begun in the shell, finished: `git merge --continue`, or
    /// `git commit -F -` with an edited message. Its own entry, whose `u`
    /// puts the merge back in progress (m6-conflicts/20). `apply::resume`
    /// sends it when no entry of Mergewright's is live; a merge Mergewright began is
    /// closed on its own entry instead.
    FinishMerge(FinishMergeParams),
    /// A merge begun in the shell, aborted: `git merge --abort`. Its own
    /// entry with a full backup, whose `u` puts the merge back as it stood
    /// (m6-conflicts/57). `apply::resume(Abort)` sends it when no entry of
    /// Mergewright's is live; a merge Mergewright began is aborted on its own entry.
    AbortMerge(AbortMergeParams),
    /// A stash that clashed on its way back, dropped once the last clashed
    /// file is done (ADR-0045 §3): `stash drop stash@{n}`, `n` looked up by
    /// the commit in the facts `apply` reads, never assumed. Refused
    /// `NoSuchStash` when it is gone. Journaled as `StashDrop` with the stash
    /// pinned, so `u` stores it back.
    DropPutBack {
        /// The stash's commit.
        stash: Oid,
        /// Why it was put back: a set-aside's copy is named as such, so the
        /// done line and the journal row give one name for one drop.
        why: PutBackWhy,
    },
    /// A put-back from the journal's list (ADR-0046 §3, m7-power-tools/23):
    /// a branch moved back, a deleted branch made again, or an edit an undo
    /// replaced restored from that undo's backup. `u` undoes it.
    Recover(RecoverParams),
}

#[cfg(test)]
mod tests;
