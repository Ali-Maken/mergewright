//! Kinds that only move refs: checkout, branch, tag, delete, a branch from a stash, and the guided re-push.

#[allow(clippy::wildcard_imports)]
use super::*;

pub(in crate::preview) fn checkout(
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
        // A new local branch at the remote's tip, then `HEAD` on it.
        CheckoutTarget::Track { remote, local } => {
            let name = branch_ref(local);
            plan.ref_moves = vec![RefMove {
                name: name.clone(),
                from: None,
                to: Some(Target::Real(remote.oid)),
            }];
            plan.head = Some(HeadMove::ToBranch(name));
            Some(remote.oid)
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
pub(in crate::preview) fn head_tip(facts: &RepoFacts) -> Option<Oid> {
    if let Some(oid) = facts.refs.get("HEAD").copied().flatten() {
        return Some(oid);
    }
    match &facts.head {
        HeadRef::Branch(name) => facts.refs.get(name).copied().flatten(),
        HeadRef::Detached(oid) => Some(*oid),
        HeadRef::Unborn(_) => None,
    }
}

pub(in crate::preview) fn branch_create(
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
        // At `HEAD`'s own commit `switch -c` carries his edits as they are,
        // and nothing is set aside: the box says so, rather than the empty
        // diff's "nothing changes" (m7-power-tools/10b).
        if creates_at_head(params, facts) && !dirty_tracked(facts).is_empty() {
            plan.worktree = WorktreeEffect::EditsStay;
        } else if let Some(from) = head_tip(facts) {
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

pub(in crate::preview) fn tag_create(params: &TagCreateParams) -> PreviewPlan {
    PreviewPlan {
        ref_moves: vec![RefMove {
            name: tag_ref(&params.name),
            from: None,
            to: Some(Target::Real(params.at)),
        }],
        ..blank()
    }
}

pub(in crate::preview) fn ref_delete(params: &RefDeleteParams) -> PreviewPlan {
    PreviewPlan {
        ref_moves: vec![RefMove {
            name: params.name.clone(),
            from: Some(params.expected),
            to: None,
        }],
        ..blank()
    }
}

pub(in crate::preview) fn branch_from_stash(
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

/// A push that puts the remote branch back where it was (ADR-0038 §4).
///
/// Only a re-push is drawn here: an ordinary push has no picture to show
/// beyond its own command line, which the M4 box already has. The
/// description says plainly that the remote moves, and `undo` says plainly
/// that `u` will not bring it back — the two things ADR-0038 §4 requires
/// the box to be unmistakable about.
pub(in crate::preview) fn push(
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
