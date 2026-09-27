//! One planner per operation kind: what the picture shows for a merge, a
//! rebase, a cherry-pick, a revert, a reset, a checkout, a branch, a tag, a
//! ref delete, a branch-from-stash and a push.
//!
//! Each returns a whole [`PreviewPlan`], built on [`blank`], and each is
//! reached only through [`super::plan`] — which fills `description`,
//! `commands` and `against` afterwards, so no kind writes a command line by
//! hand. The shared parts (the plan types, the forecast [`fold`], ghost oids
//! and the time machine) stay in the parent module.

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_git::history_reads;
use graft_git::repo::RepoHandle;
use graft_git::run::{RunError, Stop};
use graft_git::version::GitVersion;

use crate::operation::{
    BranchCreateParams, BranchFromStashParams, CheckoutParams, CheckoutTarget, FastForward,
    MergeParams, PickParams, PushParams, RebaseParams, RefDeleteParams, Refusal, RepoFacts,
    ResetMode, ResetParams, ResetTarget, RevertParams, TagCreateParams, branch_ref, merge_subject,
    short, short_ref, tag_ref,
};

use crate::facts::FactsError;
use crate::rewrite::{BranchLine, line_picks};

use super::chain::{self, Step};
use super::rewrite;
use super::{
    ConflictForecast, FORECAST_CAP, GhostCommit, HeadMove, Landing, PreviewError, PreviewPlan,
    RefMove, Replay, Target, UndoNote, WorktreeEffect, blank, dirty_tracked, fold, forecast_one,
    ghost_oid, identity_now, is_ancestor, worktree,
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
    plan.conflicts = forecast_one(repo, git, None, into, source, stop)?;
    Ok(plan)
}

// ── rebase ──────────────────────────────────────────────────────────────

/// `r`: his branch and each branch of `params.line` built on it, replayed
/// onto `onto` as the engine runs it (05b) — one ghost per pick, his rows in
/// `params.drops` left out, one [`RefMove`] per branch bottom first, and the
/// chained forecast over the same todo.
///
/// Also answers his branch's landed commit, from the chain, for the
/// put-back forecast: `None` when the chain did not reach it (a stop, a
/// hole, past the cap) and on the one-branch shortcuts that replay nothing.
pub(super) fn rebase(
    repo: &RepoHandle,
    git: &GitVersion,
    params: &RebaseParams,
    stop: &dyn Stop,
) -> Result<(PreviewPlan, Option<Oid>), PreviewError> {
    let onto = params.onto_tip;
    // Not `commits_between`: the ghosts must be the commits git will write,
    // and `git rebase` replays a narrower list than `onto..branch` — merges
    // and changes already on `onto` are dropped (m5/43). See
    // `replayed_commits`. The engine's rows (`rewrite::rebase_rows`) are
    // the same `rev-list`.
    let range = history_reads::replayed_commits(repo, onto, params.branch.oid, FORECAST_CAP, stop)?;
    let mut plan = blank();
    if range.commits.is_empty() && params.line.dependents().is_empty() {
        if params.branch.oid == onto {
            plan.undo = UndoNote::NotUndoable("nothing would be replayed".to_owned());
            return Ok((plan, None));
        }
        // Nothing to replay, but the branch still moves: git checks `onto`
        // out and leaves the branch there — a fast-forward when the branch
        // is behind, and a plain move when everything it had was a merge or
        // was already applied. Proved by running `git rebase` on both
        // shapes (m5/43). Drawing "nothing would be replayed" here would
        // hide a ref move and a rewritten tree.
        plan.ref_moves = vec![RefMove {
            name: params.branch.name.clone(),
            from: Some(params.branch.oid),
            to: Some(Target::Real(onto)),
        }];
        plan.worktree = worktree(
            repo,
            &Landing::At {
                from: params.branch.oid,
                to: onto,
            },
        )?;
        return Ok((plan, None));
    }
    let followers = followers_of(repo, &params.line, stop)?;
    line_ghosts(&mut plan, params, &range.commits, &followers);
    // The replayed branch's tree is a ghost. What lands on disk is what
    // `onto` gained since the two diverged: the branch's own commits are
    // replayed on top and their files are already here.
    plan.worktree = worktree(
        repo,
        &Landing::Joining {
            ours: params.branch.oid,
            theirs: onto,
        },
    )?;
    let steps = todo_steps(params, &range.commits, &followers);
    let landed = if range.capped {
        plan.conflicts = ConflictForecast::NotChecked(format!("more than {FORECAST_CAP} commits"));
        None
    } else {
        let chained = chain::chain(repo, git, onto, &steps, stop)?;
        plan.conflicts = chained.forecast;
        // His branch's landed commit: the result after his last row, or
        // `onto` when he has none. `None` past a stop or a hole.
        match range.commits.len() {
            0 => Some(onto),
            his => chained.results.get(his - 1).copied().flatten(),
        }
    };
    let rewritten: Vec<Oid> = steps.iter().map(|step| step.commit).collect();
    plan.published = rewrite::published(repo, &params.line, &rewritten, onto, stop)?;
    Ok((plan, landed))
}

/// One chain of ghosts on `onto`: his kept rows (`params.drops` left out),
/// his branch's [`RefMove`], then each follower's rows and its own move.
fn line_ghosts(
    plan: &mut PreviewPlan,
    params: &RebaseParams,
    his: &[history_reads::CommitSummary],
    followers: &Followers,
) {
    let mut below = Target::Real(params.onto_tip);
    let kept = his
        .iter()
        .filter(|commit| !params.drops.contains(&commit.id));
    for commit in kept {
        below = push_ghost(plan, commit, below);
    }
    plan.ref_moves.push(RefMove {
        name: params.branch.name.clone(),
        from: Some(params.branch.oid),
        to: Some(below),
    });
    let mut drawn = 0;
    for (name, from, end) in &followers.ends {
        for commit in &followers.commits[drawn..*end] {
            below = push_ghost(plan, commit, below);
        }
        drawn = *end;
        plan.ref_moves.push(RefMove {
            name: name.clone(),
            from: Some(*from),
            to: Some(below),
        });
    }
}

/// The todo git runs, as the chained forecast's steps: his rows (a dropped
/// one counted, never replayed), then each follower's picks.
fn todo_steps(
    params: &RebaseParams,
    his: &[history_reads::CommitSummary],
    followers: &Followers,
) -> Vec<Step> {
    let step = |commit: &history_reads::CommitSummary, replays: bool| Step {
        commit: commit.id,
        parent: commit.parents.first().copied(),
        replays,
    };
    his.iter()
        .map(|commit| step(commit, !params.drops.contains(&commit.id)))
        .chain(followers.commits.iter().map(|commit| step(commit, true)))
        .collect()
}

/// A ghost for `commit` on `below`, next in the chain; the target standing
/// for it.
fn push_ghost(
    plan: &mut PreviewPlan,
    commit: &history_reads::CommitSummary,
    below: Target,
) -> Target {
    let index = plan.ghosts.len();
    plan.ghosts.push(GhostCommit {
        oid: ghost_oid(&commit.subject, index),
        parents: vec![below],
        subject: commit.subject.clone(),
        author: commit.author.clone(),
        replays: Some(commit.id),
    });
    Target::Ghost(index)
}

/// Each follower's picks (the commits between the branch below it and its
/// tip), oldest first, all of them in one list, and where each branch of
/// the line ends in it: `(name, from, picks so far)`.
struct Followers {
    commits: Vec<history_reads::CommitSummary>,
    ends: Vec<(String, Oid, usize)>,
}

fn followers_of(
    repo: &RepoHandle,
    line: &BranchLine,
    stop: &dyn Stop,
) -> Result<Followers, PreviewError> {
    let picks = line_picks(repo, line, stop).map_err(|why| match why {
        FactsError::Stopped => PreviewError::Stopped,
        other => PreviewError::Facts(other),
    })?;
    let ids: Vec<Oid> = picks
        .iter()
        .flat_map(|(_, rows)| rows.iter().map(|row| row.commit))
        .collect();
    let mut info = chain::summaries(repo, &ids, stop)?;
    let mut commits = Vec::with_capacity(ids.len());
    let mut ends = Vec::with_capacity(picks.len());
    for ((name, rows), tip) in picks.iter().zip(line.dependents()) {
        for row in rows {
            let Some(commit) = info.remove(&row.commit) else {
                return Err(PreviewError::Read(RunError::Unparsable(format!(
                    "rev-list gave nothing for {}",
                    short(row.commit)
                ))));
            };
            commits.push(commit);
        }
        ends.push((name.clone(), tip.oid, commits.len()));
    }
    Ok(Followers { commits, ends })
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
        // The picked commit is by definition not on `head`, so one
        // `rev-list <commit> ^<head>` capped at one gives the commit itself
        // — subject, author and parents in a single spawn.
        let range = history_reads::commits_between(repo, head, *commit, 1, stop)?;
        let Some(summary) = range.commits.last() else {
            // Already on this branch: git would make an empty commit or
            // refuse, and either way there is nothing to draw.
            plan.undo = UndoNote::NotUndoable(format!("{} is already here", short(*commit)));
            return Ok(plan);
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

// ── checkout, branch, tag, stash branch ─────────────────────────────────

pub(super) fn checkout(
    repo: &RepoHandle,
    params: &CheckoutParams,
    facts: &RepoFacts,
) -> Result<PreviewPlan, PreviewError> {
    let from = head_tip(facts);
    let mut plan = blank();
    let to = match &params.to {
        CheckoutTarget::Branch(name) => {
            plan.head = Some(HeadMove::ToBranch(name.clone()));
            facts.refs.get(name).copied().flatten()
        }
        CheckoutTarget::Detached(oid) => {
            plan.head = Some(HeadMove::Detached(Target::Real(*oid)));
            Some(*oid)
        }
    };
    plan.worktree = match (from, to) {
        (Some(from), Some(to)) => worktree(repo, &Landing::At { from, to })?,
        // An unborn `HEAD`, or a branch the facts could not resolve: there
        // is no pair to diff, and saying "nothing changes" would be a
        // guess. The ref move and the `HEAD` move still say where it goes.
        _ => WorktreeEffect::Untouched,
    };
    Ok(plan)
}

/// What `HEAD` resolves to now, as the facts saw it.
///
/// A scope that asked for `"HEAD"` has the answer already resolved; falling
/// back to the branch's own entry covers a scope that named the branch
/// instead. A scope that named neither has no tip to give, and saying
/// "nothing changes" would be a guess.
pub(super) fn head_tip(facts: &RepoFacts) -> Option<Oid> {
    if let Some(oid) = facts.refs.get("HEAD").copied().flatten() {
        return Some(oid);
    }
    match &facts.head {
        HeadRef::Branch(name) => facts.refs.get(name).copied().flatten(),
        HeadRef::Detached(oid) => Some(*oid),
        HeadRef::Unborn(_) => None,
    }
}

pub(super) fn branch_create(
    repo: &RepoHandle,
    params: &BranchCreateParams,
    facts: &RepoFacts,
) -> Result<PreviewPlan, PreviewError> {
    let mut plan = blank();
    plan.ref_moves = vec![RefMove {
        name: branch_ref(&params.name),
        from: None,
        to: Some(Target::Real(params.at)),
    }];
    if params.checkout {
        plan.head = Some(HeadMove::ToBranch(branch_ref(&params.name)));
        if let Some(from) = head_tip(facts) {
            plan.worktree = worktree(
                repo,
                &Landing::At {
                    from,
                    to: params.at,
                },
            )?;
        }
    }
    Ok(plan)
}

pub(super) fn tag_create(params: &TagCreateParams) -> PreviewPlan {
    PreviewPlan {
        ref_moves: vec![RefMove {
            name: tag_ref(&params.name),
            from: None,
            to: Some(Target::Real(params.at)),
        }],
        ..blank()
    }
}

pub(super) fn ref_delete(params: &RefDeleteParams) -> PreviewPlan {
    PreviewPlan {
        ref_moves: vec![RefMove {
            name: params.name.clone(),
            from: Some(params.expected),
            to: None,
        }],
        ..blank()
    }
}

pub(super) fn branch_from_stash(
    repo: &RepoHandle,
    params: &BranchFromStashParams,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    // `git stash branch` creates the branch at the stash's **base** — the
    // commit the stash was taken on, its first parent — and applies there,
    // which is why it cannot conflict. The stash commit is not reachable
    // from `HEAD`, so one capped `rev-list` reaches it and its parents.
    let head = head_tip(facts).ok_or_else(|| {
        PreviewError::Read(RunError::Unparsable(
            "HEAD is on no commit, so there is nothing the stash is not on".to_owned(),
        ))
    })?;
    let range = history_reads::commits_between(repo, head, params.commit, 1, stop)?;
    let base = range
        .commits
        .last()
        .and_then(|summary| summary.parents.first().copied())
        .ok_or_else(|| {
            PreviewError::Read(RunError::Unparsable(format!(
                "the stash commit {} has no base commit to branch from",
                short(params.commit)
            )))
        })?;
    let name = branch_ref(&params.name);
    Ok(PreviewPlan {
        ref_moves: vec![RefMove {
            name: name.clone(),
            from: None,
            to: Some(Target::Real(base)),
        }],
        head: Some(HeadMove::ToBranch(name)),
        // `stash branch` checks the new branch out at `base` and applies
        // the stash there, so the tree lands on the stash commit's own
        // tree — a real one. From `HEAD`, not from `base`: leaving `HEAD`
        // is part of what rewrites the user's files.
        worktree: worktree(
            repo,
            &Landing::At {
                from: head,
                to: params.commit,
            },
        )?,
        ..blank()
    })
}

// ── the guided re-push ──────────────────────────────────────────────────

/// A push that puts the remote branch back where it was (ADR-0038 §4).
///
/// Only a re-push is drawn here: an ordinary push has no picture to show
/// beyond its own command line, which the M4 box already has. The
/// description says plainly that the remote moves, and `undo` says plainly
/// that `u` will not bring it back — the two things ADR-0038 §4 requires
/// the box to be unmistakable about.
pub(super) fn push(
    repo: &RepoHandle,
    params: &PushParams,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let Some(source) = params.source_commit else {
        return Err(PreviewError::Refused(Refusal::NotPreviewable(
            graft_core::journal::OperationKind::Push,
        )));
    };
    // The commit has to be on the branch being pushed. A branch the facts
    // cannot resolve fails the same way: an unverified re-push is not one
    // this box will draw.
    let on_branch = match facts.refs.get(&params.branch).copied().flatten() {
        Some(tip) => is_ancestor(repo, source, tip, stop)?,
        None => false,
    };
    if !on_branch {
        return Err(PreviewError::Refused(Refusal::NotOnBranch {
            commit: source,
            branch: short_ref(&params.branch).to_owned(),
        }));
    }
    let remote_side = format!("{}/{}", params.remote, short_ref(&params.remote_branch));
    Ok(PreviewPlan {
        ref_moves: vec![RefMove {
            name: remote_side.clone(),
            from: params.lease.map(|lease| lease.expected),
            to: Some(Target::Real(source)),
        }],
        undo: UndoNote::NotUndoable(format!(
            "this rewrites {remote_side} for everyone who has already pulled it; u undoes your \
             own branch, never a remote"
        )),
        ..blank()
    })
}
