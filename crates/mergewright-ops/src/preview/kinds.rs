//! One planner per operation kind: what the picture shows for a merge, a
//! rebase, a cherry-pick, a revert, a reset, a checkout, a branch, a tag, a
//! ref delete, a branch-from-stash and a push.
//!
//! Each returns a whole [`PreviewPlan`], built on [`blank`], and each is
//! reached only through [`super::plan`] — which fills `description`,
//! `commands` and `against` afterwards, so no kind writes a command line by
//! hand. The shared parts (the plan types, the forecast [`fold`], ghost oids
//! and the time machine) stay in the parent module.

use mergewright_core::journal::HeadRef;
use mergewright_core::oid::Oid;
use mergewright_git::history_reads;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{RunError, Stop};
use mergewright_git::version::GitVersion;

use crate::operation::{
    BranchCreateParams, BranchFromStashParams, CheckoutParams, CheckoutTarget, FastForward,
    MergeParams, PickParams, PushParams, RebaseParams, RefDeleteParams, Refusal, RepoFacts,
    ResetMode, ResetParams, ResetTarget, RevertParams, TagCreateParams, branch_ref,
    creates_at_head, merge_subject, short, short_ref, tag_ref,
};

use crate::facts::FactsError;
use crate::rewrite::{BranchLine, line_picks};

use super::chain::{self, Step};
use super::merge_look;
use super::rewrite;
use super::{
    ConflictForecast, FORECAST_CAP, GhostCommit, HeadMove, Landing, PreviewError, PreviewPlan,
    RefMove, Replay, Target, UndoNote, WorktreeEffect, blank, conflicts_from, dirty_tracked, fold,
    forecast_one, ghost_oid, identity_now, is_ancestor, worktree,
};

mod rebase;
mod refs;

pub(super) use rebase::rebase;
pub(super) use refs::{
    branch_create, branch_from_stash, checkout, head_tip, push, ref_delete, tag_create,
};

// ── merge ───────────────────────────────────────────────────────────────

pub(super) fn merge(
    repo: &RepoHandle,
    git: &GitVersion,
    params: &MergeParams,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let into = params.into.oid;
    let source = params.source_tip;
    let base = history_reads::merge_base(repo, into, source)?;
    let mut plan = blank();
    if base == Some(source) {
        // Already contains it: git would say "Already up to date." and do
        // nothing at all, which is worth saying before the keypress.
        plan.undo = UndoNote::NotUndoable("nothing would change".to_owned());
        return Ok(plan);
    }
    if base == Some(into) && params.ff != FastForward::Never {
        plan.ref_moves = vec![RefMove {
            name: params.into.name.clone(),
            from: Some(into),
            to: Some(Target::Real(source)),
        }];
        // A fast-forward lands `HEAD` on `source` itself, so the tree it
        // ends at is real. `Joining` would give the same answer here — the
        // fork point *is* `into` — which is the rule agreeing with itself.
        plan.worktree = worktree(
            repo,
            &Landing::At {
                from: into,
                to: source,
            },
        )?;
        // The same box as any merge, flagged so its words say nothing is
        // combined; the forecast stays `Clean`, as a fast-forward joins
        // nothing.
        plan.merge = Some(merge_look::look(repo, git, into, source, true, stop)?);
        return Ok(plan);
    }
    if params.ff == FastForward::Only {
        return Err(PreviewError::Refused(Refusal::NotFastForward {
            branch: short_ref(&params.into.name).to_owned(),
        }));
    }
    plan.ghosts = vec![GhostCommit {
        oid: ghost_oid(&merge_subject(params), 0),
        parents: vec![Target::Real(into), Target::Real(source)],
        subject: merge_subject(params),
        author: identity_now(repo, stop)?,
        replays: None,
    }];
    plan.ref_moves = vec![RefMove {
        name: params.into.name.clone(),
        from: Some(into),
        to: Some(Target::Ghost(0)),
    }];
    // The merge commit's tree is a ghost, so what lands on disk is what
    // `source` gained since the two diverged — never `into` against
    // `source`, which would also name `HEAD`'s own files, already on disk.
    plan.worktree = worktree(
        repo,
        &Landing::Joining {
            ours: into,
            theirs: source,
        },
    )?;
    // One read for the box and the forecast both (ease-pass 10): the
    // conflicts are derived from the look, never forecast a second time.
    let look = merge_look::look(repo, git, into, source, false, stop)?;
    plan.conflicts = conflicts_from(&look.check);
    plan.merge = Some(look);
    Ok(plan)
}

// ── cherry-pick ─────────────────────────────────────────────────────────

pub(super) fn cherry_pick(
    repo: &RepoHandle,
    git: &GitVersion,
    params: &PickParams,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let head = params.head.oid;
    let mut plan = blank();
    let mut conflicts = ConflictForecast::Clean;
    let mut replays: Vec<Replay> = Vec::new();
    for commit in &params.commits {
        // One `rev-list <commit> ^<head>` capped at one gives the commit
        // itself — subject, author and parents in a single spawn — or
        // nothing when `head` already has it. Both answer
        // `check_commit` (m7-power-tools/11a): already on the branch, or a
        // merge, refuses, and the preview never opens.
        let range = history_reads::commits_between(repo, head, *commit, 1, stop)?;
        let found = range.commits.last();
        let parents = found.map_or(0, |summary| summary.parents.len());
        params
            .check_commit(*commit, found.is_none(), parents)
            .map_err(PreviewError::Refused)?;
        let Some(summary) = found else {
            unreachable!("check_commit refuses a commit `head` already has");
        };
        let index = plan.ghosts.len();
        let parent = match index {
            0 => Target::Real(head),
            earlier => Target::Ghost(earlier - 1),
        };
        plan.ghosts.push(GhostCommit {
            oid: ghost_oid(&summary.subject, index),
            parents: vec![parent],
            subject: summary.subject.clone(),
            author: summary.author.clone(),
            replays: Some(summary.id),
        });
        let base = summary.parents.first().copied();
        replays.push(Replay {
            parent: base,
            commit: *commit,
        });
        conflicts = fold(
            conflicts,
            forecast_one(repo, git, base, head, *commit, stop)?,
        );
    }
    plan.ref_moves = vec![RefMove {
        name: params.head.name.clone(),
        from: Some(head),
        to: Some(Target::Ghost(plan.ghosts.len() - 1)),
    }];
    // Only the picked patches come in: each against its own first parent,
    // which is what the patch *is*. `head` against the commit would name
    // everything on the other branch too, picked or not.
    plan.worktree = worktree(repo, &Landing::Replaying(replays))?;
    plan.conflicts = conflicts;
    Ok(plan)
}

// ── revert ──────────────────────────────────────────────────────────────

pub(super) fn revert(
    repo: &RepoHandle,
    git: &GitVersion,
    params: &RevertParams,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let head = params.head.oid;
    let subject = format!("Revert \"{}\"", params.subject);
    let mut plan = blank();
    plan.ghosts = vec![GhostCommit {
        oid: ghost_oid(&subject, 0),
        parents: vec![Target::Real(head)],
        subject,
        // `git revert` authors the new commit as the user, keeping none of
        // the reverted commit's authorship — only its subject, quoted.
        author: identity_now(repo, stop)?,
        replays: Some(params.commit),
    }];
    plan.ref_moves = vec![RefMove {
        name: params.head.name.clone(),
        from: Some(head),
        to: Some(Target::Ghost(0)),
    }];
    // What a revert rewrites is the reverted commit's **own** change —
    // `parent..commit` — not the history since it. The old pair
    // (`commit..head`) was empty whenever the commit being reverted was
    // `HEAD`, which is the ordinary case, so the box said no file changed
    // while git rewrote several (m5/07 review, M3).
    plan.worktree = worktree(
        repo,
        &Landing::Replaying(vec![Replay {
            parent: params.parent,
            commit: params.commit,
        }]),
    )?;
    // The reverse patch as a three-way merge: base is the commit itself,
    // theirs is its parent — going backwards along that one edge.
    plan.conflicts = match params.parent {
        Some(parent) => forecast_one(repo, git, Some(params.commit), head, parent, stop)?,
        None => {
            ConflictForecast::NotChecked("the reverted commit's parent was not recorded".to_owned())
        }
    };
    Ok(plan)
}

// ── reset ───────────────────────────────────────────────────────────────

pub(super) fn reset(
    repo: &RepoHandle,
    params: &ResetParams,
    facts: &RepoFacts,
) -> Result<PreviewPlan, PreviewError> {
    let mut plan = blank();
    match &params.target {
        ResetTarget::Head { branch, mode } => {
            plan.ref_moves = vec![RefMove {
                name: branch.name.clone(),
                from: Some(branch.oid),
                to: Some(Target::Real(params.to)),
            }];
            plan.worktree = match mode {
                ResetMode::Soft | ResetMode::Mixed => WorktreeEffect::Untouched,
                ResetMode::Hard => hard_reset_effect(repo, branch.oid, params.to, facts)?,
            };
        }
        // Another branch moved by compare-and-swap: `HEAD`, the index and
        // the working tree are not in it at all.
        ResetTarget::Branch(tip) => {
            plan.ref_moves = vec![RefMove {
                name: tip.name.clone(),
                from: Some(tip.oid),
                to: Some(Target::Real(params.to)),
            }];
        }
    }
    Ok(plan)
}

/// A `--hard` reset with dirty tracked files says `LOSES` and names them;
/// with a clean tree it is an ordinary update. The two cannot both be said
/// — one [`WorktreeEffect`] per plan — and the one that must be read is the
/// loss.
fn hard_reset_effect(
    repo: &RepoHandle,
    from: Oid,
    to: Oid,
    facts: &RepoFacts,
) -> Result<WorktreeEffect, PreviewError> {
    let dirty = dirty_tracked(facts);
    if dirty.is_empty() {
        // The tree lands on `to`, which already exists.
        worktree(repo, &Landing::At { from, to })
    } else {
        Ok(WorktreeEffect::Lost { paths: dirty })
    }
}
