//! ADR-0049 §4's table: every path an operation writes, uncapped. The
//! replays' rows are [`super::replay`]'s.

use std::collections::BTreeSet;
use std::path::PathBuf;

use mergewright_core::journal::HeadRef;
use mergewright_core::oid::Oid;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, Stop};

use super::reads::{diff_tree, head, listed, names, parents_of, unborn};
use super::replay::replayed_paths;
use crate::facts::FactsError;
use crate::operation::{
    CheckoutTarget, Operation, RecoverParams, RecoverTarget, RepoFacts, ResetMode, ResetTarget,
    branch_ref,
};

/// Every path the operation writes. `Ok(None)` = the kind writes no path
/// the index doesn't track.
///
/// One deviation from the table, a superset: a hard reset (and a put-back
/// of the checked-out branch, which is one) writes every path where the
/// index differs from the target — `diff-index --cached <to>` — not only
/// `HEAD`'s diff to it. A file he `git rm --cached` is in no diff of
/// `HEAD`'s, yet `reset --hard` writes it back over his bytes.
pub(super) fn written_paths(
    repo: &RepoHandle,
    op: &Operation,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<Option<BTreeSet<PathBuf>>, FactsError> {
    let paths = match op {
        Operation::Checkout(params) => from_head(repo, facts, landed(&params.to, facts), stop)?,
        Operation::BranchCreate(params) if params.checkout => {
            from_head(repo, facts, GitArg::Oid(params.at), stop)?
        }
        // Unrelated histories: git refuses the merge (no
        // `--allow-unrelated-histories`), so it writes nothing, and
        // `diff-tree --merge-base` has no base to diff from (ease-pass 10).
        Operation::Merge(params)
            if mergewright_git::history_reads::merge_base(repo, params.into.oid, params.source_tip)
                .map_err(super::reads::run_error)?
                .is_none() =>
        {
            BTreeSet::new()
        }
        Operation::Merge(params) => incoming(repo, facts, params.source_tip, stop)?,
        // A fast-forward, so the incoming side, as for a merge (review 28
        // M1: `HEAD`'s whole diff to the tip also counted his own unpushed
        // commits). Before the fetch: its tip as last fetched. What the
        // fetch brings is guarded by git's own `--no-overwrite-ignore`
        // (ADR-0049 §4).
        Operation::Pull(params) => match params.upstream.tracking_tip {
            Some(tip) => incoming(repo, facts, tip, stop)?,
            None => BTreeSet::new(),
        },
        // A file put-back writes exactly its paths. Tracked ones drop out
        // in `blocking`; what is left is his, untracked or ignored (m7/30o
        // R1: `refused` lets the untracked through, the snapshot keeps them).
        Operation::Recover(RecoverParams {
            target: RecoverTarget::Files { paths, .. },
            ..
        }) => paths.iter().cloned().collect(),
        Operation::Reset(_) | Operation::Recover(_) => match hard_reset_to(op, facts) {
            Some(to) => hard_reset(repo, to, stop)?,
            None => return Ok(None),
        },
        Operation::BranchFromStash(params) => branch_from_stash(repo, facts, params.commit, stop)?,
        Operation::StashApply { commit, .. } | Operation::StashPop { commit, .. } => {
            stash_paths(repo, *commit, &parents_of(repo, *commit, stop)?, stop)?
        }
        Operation::Rebase(_)
        | Operation::Rewrite(_)
        | Operation::CherryPick(_)
        | Operation::Revert(_) => replayed_paths(repo, op, stop)?,
        // Stage, unstage, discard, the conflict kinds, commits and every
        // ref-only kind write only paths the index tracks, or none.
        _ => return Ok(None),
    };
    Ok(Some(paths))
}

/// Where a `reset --hard` lands: a hard reset's target, or a put-back of
/// the checked-out branch. `None` for a soft or mixed reset and a put-back
/// of a branch not checked out (an `update-ref` writes no file). A file
/// put-back never gets here: `written_paths` answers it with its paths.
fn hard_reset_to(op: &Operation, facts: &RepoFacts) -> Option<Oid> {
    match op {
        Operation::Reset(params) => match &params.target {
            ResetTarget::Head {
                mode: ResetMode::Hard,
                ..
            } => Some(params.to),
            _ => None,
        },
        Operation::Recover(params) => match &params.target {
            RecoverTarget::MoveRef { name, to, .. } if matches!(&facts.head, HeadRef::Branch(head) if *head == branch_ref(name)) => {
                Some(*to)
            }
            _ => None,
        },
        _ => None,
    }
}

/// Where a checkout lands: the branch's tip as the facts read it.
fn landed(to: &CheckoutTarget, facts: &RepoFacts) -> GitArg {
    match to {
        CheckoutTarget::Branch(name) => facts
            .refs
            .get(name)
            .copied()
            .flatten()
            .map_or_else(|| GitArg::Ref(name.clone().into_bytes()), GitArg::Oid),
        CheckoutTarget::Detached(oid) => GitArg::Oid(*oid),
        // The new branch starts where the list saw the remote's tip; `check`
        // refused already if it moved.
        CheckoutTarget::Track { remote, .. } => GitArg::Oid(remote.oid),
    }
}

/// `HEAD`'s diff to where it lands; every path of the landed tree when
/// `HEAD` has no commit yet.
fn from_head(
    repo: &RepoHandle,
    facts: &RepoFacts,
    landed: GitArg,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    if unborn(facts) {
        return listed(repo, landed, stop);
    }
    diff_tree(repo, &[], [head(), landed], stop)
}

/// The incoming side: what `tip` changed since its merge base with `HEAD`.
/// A fast-forward's base is `HEAD`, so then this is `HEAD`'s diff; when
/// `HEAD` is ahead, the base is `tip` and nothing comes in.
fn incoming(
    repo: &RepoHandle,
    facts: &RepoFacts,
    tip: Oid,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    if unborn(facts) {
        return listed(repo, GitArg::Oid(tip), stop);
    }
    diff_tree(repo, &["--merge-base"], [head(), GitArg::Oid(tip)], stop)
}

/// Every path where the index differs from `to`: what `reset --hard <to>`
/// writes.
fn hard_reset(
    repo: &RepoHandle,
    to: Oid,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    names(
        repo,
        vec![
            GitArg::Flag("diff-index"),
            GitArg::Flag("--cached"),
            GitArg::Flag("--name-only"),
            GitArg::Flag("--no-renames"),
            GitArg::Flag("-z"),
            GitArg::Oid(to),
        ],
        stop,
    )
}

/// `switch -c <name> stash^1`, then `stash pop --index`.
fn branch_from_stash(
    repo: &RepoHandle,
    facts: &RepoFacts,
    stash: Oid,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    let parents = parents_of(repo, stash, stop)?;
    let mut paths = match parents.first() {
        Some(base) => from_head(repo, facts, GitArg::Oid(*base), stop)?,
        None => BTreeSet::new(),
    };
    paths.extend(stash_paths(repo, stash, &parents, stop)?);
    Ok(paths)
}

/// A stash put back: its worktree change (`stash^1`→`stash`), its staged
/// change (`stash^1`→`stash^2`) and its untracked files (`stash^3`).
fn stash_paths(
    repo: &RepoHandle,
    stash: Oid,
    parents: &[Oid],
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    let Some(&base) = parents.first() else {
        return Ok(BTreeSet::new());
    };
    let mut paths = diff_tree(repo, &[], [GitArg::Oid(base), GitArg::Oid(stash)], stop)?;
    if let Some(&staged) = parents.get(1) {
        paths.extend(diff_tree(
            repo,
            &[],
            [GitArg::Oid(base), GitArg::Oid(staged)],
            stop,
        )?);
    }
    if let Some(&untracked) = parents.get(2) {
        paths.extend(listed(repo, GitArg::Oid(untracked), stop)?);
    }
    Ok(paths)
}
