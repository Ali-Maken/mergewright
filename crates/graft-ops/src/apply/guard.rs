//! `apply`'s gates that need a read: asked again where the write runs,
//! before the set-aside and the `Pending` line. Moved out of `apply.rs`
//! (m8/13e).

use graft_git::run::RunError;

use crate::facts::FactsError;
use crate::operation::{Operation, Refusal, RepoFacts};

use super::{ApplyError, Context};

/// ADR-0038 §4's ancestry rule, enforced where the push actually runs.
///
/// A re-push force-writes the remote branch to a commit the user picked out
/// of their own history, and the rule is that the commit must be **on that
/// branch**. Deciding it needs a read (`merge-base --is-ancestor`), and
/// `check` is pure over `RepoFacts` and cannot spawn — so the decision lived
/// in `preview::push` alone, and anything reaching `apply` without drawing a
/// picture first would have forced any commit at all under the lease
/// (m5/07 review, M5). `apply` can spawn, so it asks the same question in
/// the same terms, before the `Pending` line and before git is told to push.
///
/// Nothing else is re-checked here: every other precondition is a fact
/// `check` already holds.
pub(super) fn guard_repush(
    ctx: &Context<'_>,
    op: &Operation,
    facts: &RepoFacts,
) -> Result<(), ApplyError> {
    let Operation::Push(params) = op else {
        return Ok(());
    };
    let Some(source) = params.source_commit else {
        return Ok(());
    };
    // A branch the facts cannot resolve is refused the same way a commit off
    // the branch is: an unverified re-push is not one that runs.
    let on_branch = match facts.refs.get(&params.branch).copied().flatten() {
        Some(tip) => {
            crate::preview::is_ancestor(ctx.repo, source, tip, ctx.stop).map_err(|error| {
                match error {
                    RunError::Stopped => ApplyError::Stopped,
                    other => ApplyError::Facts(FactsError::Ancestry(other)),
                }
            })?
        }
        None => false,
    };
    if on_branch {
        return Ok(());
    }
    Err(ApplyError::Refused(Refusal::NotOnBranch {
        commit: source,
        branch: crate::operation::short_ref(&params.branch).to_owned(),
    }))
}

/// A cherry-pick's two refusals that need a read (m7-power-tools/11a): a
/// commit the branch already has, and a merge. The preview asks them
/// (`preview::kinds::cherry_pick`); `apply` asks again, in the same terms,
/// before the set-aside and the `Pending` line, as [`guard_repush`] does —
/// so a pick reaching `apply` without a picture refuses the same way, and
/// nothing is written.
pub(super) fn guard_pick(ctx: &Context<'_>, op: &Operation) -> Result<(), ApplyError> {
    let Operation::CherryPick(params) = op else {
        return Ok(());
    };
    for commit in &params.commits {
        // `<commit> ^<head>` capped at one: the commit and its parents, or
        // nothing when `head` already has it.
        let range = graft_git::history_reads::commits_between(
            ctx.repo,
            params.head.oid,
            *commit,
            1,
            ctx.stop,
        )
        .map_err(|error| match error {
            RunError::Stopped => ApplyError::Stopped,
            other => ApplyError::Facts(FactsError::Ancestry(other)),
        })?;
        let found = range.commits.last();
        let parents = found.map_or(0, |summary| summary.parents.len());
        params
            .check_commit(*commit, found.is_none(), parents)
            .map_err(ApplyError::Refused)?;
    }
    Ok(())
}

/// His files git doesn't keep, where the write puts its own (ADR-0049):
/// the preview's question asked again here, before the set-aside and the
/// `Pending` line, because facts can change between the picture and `y`
/// and `check` cannot see trees or the disk. A refusal leaves nothing
/// behind: no stash, no entry.
pub(super) fn guard_in_the_way(
    ctx: &Context<'_>,
    op: &Operation,
    facts: &RepoFacts,
) -> Result<(), ApplyError> {
    let blocking =
        crate::in_the_way::blocking(ctx.repo, op, facts, ctx.stop).map_err(
            |error| match error {
                FactsError::Stopped => ApplyError::Stopped,
                other => ApplyError::Facts(other),
            },
        )?;
    match crate::in_the_way::refused(op, blocking) {
        Some(refused) => Err(ApplyError::Refused(refused)),
        None => Ok(()),
    }
}
