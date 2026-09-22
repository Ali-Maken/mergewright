//! The promise's assertion: **the ghost equals what git did.** One function,
//! used by every `ghost-equals-git` scenario, that takes the plan the
//! preview drew and judges it against the repository after `Confirm` — by
//! running git, never by asking the app what happened.

use graft_ops::preview::{GhostCommit, HeadMove, PreviewPlan, Target, WorktreeEffect};

use super::fixture::Fixture;
use super::truth::{Claim, Truth};
use crate::repo_truth::{check, run_git, run_git_with};

/// Every [`RefMove`](graft_ops::preview::RefMove) with a real target equals
/// `for-each-ref`; every ghost target's ref now points at a commit whose
/// subject is the ghost's, whose parents are the ghost's parents' real
/// counterparts, and — for a replay — whose patch-id equals the replayed
/// commit's; `HEAD` went where the plan said; the paths the plan said would
/// change are among those git changed.
pub fn ghost_equals_git(
    claim: &Claim,
    before: &Truth,
    after: &Truth,
    fixture: &Fixture,
) -> Result<(), String> {
    let plan = &claim.plan.plan;
    check(!plan.ref_moves.is_empty(), || {
        "the plan moves no ref, so there is nothing to judge".to_owned()
    })?;
    for ref_move in &plan.ref_moves {
        let now = after.repo.refs.get(&ref_move.name).map(String::as_str);
        match &ref_move.to {
            None => check(now.is_none(), || {
                format!("{} was to be deleted, is {now:?}", ref_move.name)
            })?,
            Some(Target::Real(oid)) => check(now == Some(oid.to_string().as_str()), || {
                format!("{} was to be {oid}, is {now:?}", ref_move.name)
            })?,
            Some(Target::Ghost(index)) => {
                let real = now.ok_or_else(|| format!("{} is gone", ref_move.name))?;
                ghost_is(plan, *index, real, fixture)?;
            }
        }
        if let (Some(from), Some(was)) = (ref_move.from, before.repo.refs.get(&ref_move.name)) {
            check(from.to_string() == *was, || {
                format!("{} was drawn from {from}, was really {was}", ref_move.name)
            })?;
        }
    }
    match &plan.head {
        None => check(after.head_symbolic == before.head_symbolic, || {
            format!(
                "HEAD was to stay, moved {:?} → {:?}",
                before.head_symbolic, after.head_symbolic
            )
        })?,
        Some(HeadMove::ToBranch(name)) => {
            check(after.head_symbolic.as_deref() == Some(name), || {
                format!("HEAD was to be on {name}, is {:?}", after.head_symbolic)
            })?;
        }
        Some(HeadMove::Detached(target)) => {
            check(after.head_symbolic.is_none(), || {
                format!("HEAD was to detach, is on {:?}", after.head_symbolic)
            })?;
            let head = after
                .repo
                .head
                .as_deref()
                .ok_or_else(|| "no HEAD after".to_owned())?;
            match target {
                Target::Real(oid) => check(head == oid.to_string(), || {
                    format!("HEAD was to detach at {oid}, is at {head}")
                })?,
                Target::Ghost(index) => ghost_is(plan, *index, head, fixture)?,
            }
        }
    }
    worktree_effect(plan, before, after)
}

/// The commit `real` is what ghost `index` promised, recursively through
/// its ghost parents.
fn ghost_is(plan: &PreviewPlan, index: usize, real: &str, fixture: &Fixture) -> Result<(), String> {
    let ghost: &GhostCommit = plan
        .ghosts
        .get(index)
        .ok_or_else(|| format!("the plan has no ghost {index}"))?;
    let subject = fixture.git(&["log", "-1", "--format=%s", real])?;
    check(subject.trim() == ghost.subject, || {
        format!(
            "ghost {index}: subject `{}`, git wrote `{}`",
            ghost.subject,
            subject.trim()
        )
    })?;
    let parents: Vec<String> = fixture
        .git(&["log", "-1", "--format=%P", real])?
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    check(parents.len() == ghost.parents.len(), || {
        format!(
            "ghost {index}: {} parents drawn, git wrote {}",
            ghost.parents.len(),
            parents.len()
        )
    })?;
    for (slot, (drawn, written)) in ghost.parents.iter().zip(&parents).enumerate() {
        match drawn {
            Target::Real(oid) => check(oid.to_string() == *written, || {
                format!("ghost {index}: parent {slot} drawn {oid}, git wrote {written}")
            })?,
            Target::Ghost(parent) => ghost_is(plan, *parent, written, fixture)?,
        }
    }
    if let Some(replays) = ghost.replays {
        // A revert's ghost is matched against the commit it undoes: its
        // patch is that commit's, reversed.
        let reversed = plan
            .commands
            .first()
            .is_some_and(|command| command.starts_with("git revert "));
        let drawn = patch_id(fixture, &replays.to_string(), reversed)?;
        let written = patch_id(fixture, real, false)?;
        check(drawn == written, || {
            format!("ghost {index}: replays {replays}, but the patch-ids differ")
        })?;
    }
    Ok(())
}

/// `git patch-id --stable` of one commit's diff, or of its reverse.
pub fn patch_id(fixture: &Fixture, commit: &str, reversed: bool) -> Result<String, String> {
    // `-R` is ignored on a one-commit `diff-tree`; the reverse is the two
    // trees named the other way round.
    let parent = format!("{commit}^");
    let args: Vec<&str> = if reversed {
        vec!["diff-tree", "-p", commit, &parent]
    } else {
        vec!["diff-tree", "-p", "--root", commit]
    };
    let diff = run_git(fixture.repo(), &args)?;
    let out = run_git_with(
        fixture.repo(),
        &["patch-id", "--stable"],
        Some(diff.as_bytes()),
    )?;
    Ok(out.split_whitespace().next().unwrap_or_default().to_owned())
}

/// What the plan said about the working tree holds on disk.
fn worktree_effect(plan: &PreviewPlan, before: &Truth, after: &Truth) -> Result<(), String> {
    let changed: Vec<String> = before
        .repo
        .files
        .keys()
        .chain(after.repo.files.keys())
        .filter(|path| before.repo.files.get(*path) != after.repo.files.get(*path))
        .map(|path| path.display().to_string())
        .collect();
    match &plan.worktree {
        WorktreeEffect::Untouched => check(changed.is_empty(), || {
            format!("the tree was to stay, changed {changed:?}")
        }),
        WorktreeEffect::Updated { paths, .. } | WorktreeEffect::Lost { paths } => {
            for path in paths {
                let path = path.display().to_string();
                check(changed.contains(&path), || {
                    format!("{path} was drawn as changing, did not (changed: {changed:?})")
                })?;
            }
            Ok(())
        }
        WorktreeEffect::RestoredFromBackup | WorktreeEffect::NotShown { .. } => Ok(()),
    }
}
