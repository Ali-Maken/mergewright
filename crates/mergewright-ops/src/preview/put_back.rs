//! The put-back forecast (ADR-0045 §8, m6-conflicts/84): when a rebase,
//! a merge or a switch sets the user's edits aside, will putting them back
//! clash?
//!
//! Three steps, mirroring what `apply` will do at step 6:
//! 1. `git stash create` of the tree now → `S'` (on a copy of the index);
//! 2. `landed`, the commit `HEAD` ends on: a checkout's target, or for a
//!    rebase the chained forecast's result after his last row (drops
//!    skipped, 05b); where the chain did not reach it, one
//!    `merge-tree HEAD <upstream>` made a commit stands in — for the
//!    replayed branch, which is never built (m5 hard choice 5); for a
//!    cherry-pick, each picked commit replayed from its own parent
//!    (m7-power-tools/11a); for a merge, the source it fast-forwards to or
//!    the merged tree from their merge base (m8-polish-and-ship/19d);
//! 3. `git merge-tree --merge-base=HEAD <landed> <S'>`: the three-way merge
//!    `stash apply` does, whose base is `S'^1`, which is `HEAD`.
//!
//! **Every branch passed to `merge-tree` is a commit.** git 2.40 is the
//! oldest `merge-tree --merge-base` Mergewright forecasts with; whether it accepts
//! a bare tree there was not confirmed (only 2.54 was tried), so the rebase's
//! landed tree is made a commit (`commit-tree`) rather than relying on it.
//!
//! Nothing here fails the preview: a read that goes wrong is
//! `NotChecked(<why>)`, a hole the box names. Only a stop is an error.

use mergewright_core::oid::Oid;
use mergewright_git::history_reads::{self, MergeStep};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};
use mergewright_git::version::GitVersion;

use crate::operation::{
    CheckoutTarget, MergeParams, Operation, RecoverParams, RecoverTarget, RepoFacts,
};
use crate::snapshot::{self, SnapshotError};

use super::{ConflictForecast, PreviewError, forecast_one, kinds};

/// The oldest git whose `merge-tree` takes `--merge-base`, which step 3
/// needs. Mergewright itself runs on 2.38 (PROMPT.md §1); below this the put-back
/// is not checked, and says so.
const MERGE_BASE_MIN: GitVersion = GitVersion {
    major: 2,
    minor: 40,
    patch: 0,
};

/// Why a rebase's put-back is not forecast: the replay itself may stop, and
/// the tree the edits would go back onto is then nobody's guess.
const MAY_STOP: &str = "the rebase may stop first";

/// The same hole for a cherry-pick: a pick that stops, or one whose commit
/// could not be replayed here, leaves no tree to put the edits back onto.
const PICK_MAY_STOP: &str = "the cherry-pick may stop first";

/// The same hole for a merge whose own forecast is not clean
/// (m8-polish-and-ship/19d).
const MERGE_MAY_STOP: &str = "the merge may stop first";

/// The same hole for a rewrite whose replay may stop.
const REWRITE_MAY_STOP: &str = "the rewrite may stop first";

/// A rewrite that only reorders, rewords or squashes (`R`): the line it
/// lands on is not built before `y`, so the put-back is not forecast.
const REORDER_NOT_FORECAST: &str = "the rewritten line is not built before y";

/// Forecast step 6 of `op`, whose own forecast is `replay`.
///
/// `rebase_landed` is a rebase's landed commit from its own chained
/// forecast (`kinds::rebase`): his rows replayed one by one, the dropped
/// ones skipped (05b). When it is there it is `landed`, exactly; the one
/// `merge-tree HEAD <upstream>` of step 2 stands in only when the chain
/// did not reach it.
///
/// # Errors
/// [`PreviewError::Stopped`] only. Every other failure is a
/// [`ConflictForecast::NotChecked`] naming it.
pub(super) fn forecast(
    repo: &RepoHandle,
    git: &GitVersion,
    op: &Operation,
    facts: &RepoFacts,
    replay: &ConflictForecast,
    rebase_landed: Option<Oid>,
    stop: &dyn Stop,
) -> Result<ConflictForecast, PreviewError> {
    if *git < MERGE_BASE_MIN {
        return Ok(not_checked("git 2.40+ needed to forecast the put-back"));
    }
    let (head, landing) = match landing_of(op, facts, replay, rebase_landed) {
        Ok(found) => found,
        Err(hole) => return Ok(hole),
    };
    stopped(stop)?;
    // The index copy lives in the git dir for the moment it takes, beside
    // the index it copies, and is removed on every path (`TempIndex`).
    let edits = match snapshot::stash_create_on_copy(repo, repo.git_dir(), stop) {
        Ok(Some(edits)) => edits,
        // git found nothing to stash since the facts were read: nothing is
        // put back, so nothing can clash.
        Ok(None) => return Ok(ConflictForecast::Clean),
        Err(SnapshotError::Stopped) => return Err(PreviewError::Stopped),
        Err(why) => return Ok(not_checked(&why.to_string())),
    };
    let landed = match landing {
        Landing::At(target) => target,
        Landing::Replayed(onto) => match replayed(repo, head, onto, stop)? {
            Ok(landed) => landed,
            Err(hole) => return Ok(hole),
        },
        Landing::Picked(commits) => match picked(repo, git, head, &commits, stop)? {
            Ok(landed) => landed,
            Err(hole) => return Ok(hole),
        },
        Landing::Merged(params) => match merge_landed(repo, git, head, &params, stop) {
            Ok(MergeLanded::At(landed)) => landed,
            Ok(MergeLanded::MayStop) => return Ok(not_checked(MERGE_MAY_STOP)),
            Ok(MergeLanded::NotChecked(why)) => return Ok(not_checked(&why)),
            Err(RunError::Stopped) => return Err(PreviewError::Stopped),
            Err(why) => return Ok(not_checked(&why.to_string())),
        },
    };
    stopped(stop)?;
    match forecast_one(repo, git, Some(head), landed, edits, stop) {
        Ok(forecast) => Ok(forecast),
        Err(PreviewError::Stopped) => Err(PreviewError::Stopped),
        Err(why) => Ok(not_checked(&why.to_string())),
    }
}

/// `HEAD` before `op`, and where it lands before the put-back runs; `Err`
/// is the hole to answer with.
fn landing_of(
    op: &Operation,
    facts: &RepoFacts,
    replay: &ConflictForecast,
    rebase_landed: Option<Oid>,
) -> Result<(Oid, Landing), ConflictForecast> {
    match op {
        Operation::Checkout(params) => {
            let target = match &params.to {
                CheckoutTarget::Branch(name) => facts.refs.get(name).copied().flatten(),
                CheckoutTarget::Detached(oid) => Some(*oid),
                CheckoutTarget::Track { remote, .. } => Some(remote.oid),
            };
            match (kinds::head_tip(facts), target) {
                (Some(head), Some(target)) => Ok((head, Landing::At(target))),
                _ => Err(not_checked("where HEAD lands could not be read")),
            }
        }
        Operation::Rebase(params) => {
            if *replay != ConflictForecast::Clean {
                return Err(not_checked(MAY_STOP));
            }
            match rebase_landed {
                Some(landed) => Ok((params.branch.oid, Landing::At(landed))),
                None => Ok((params.branch.oid, Landing::Replayed(params.onto_tip))),
            }
        }
        // A new branch switched to at `at`: `HEAD` lands there, as a
        // checkout's does (m7-power-tools/10b).
        Operation::BranchCreate(params) => match kinds::head_tip(facts) {
            Some(head) => Ok((head, Landing::At(params.at))),
            None => Err(not_checked("where HEAD lands could not be read")),
        },
        // A branch put back while checked out: a `reset --hard` to `to`.
        Operation::Recover(RecoverParams {
            target: RecoverTarget::MoveRef { to, expected, .. },
            ..
        }) => Ok((*expected, Landing::At(*to))),
        // A cherry-pick (m7-power-tools/11a): `HEAD` lands on the picked
        // commits replayed onto it, each from its own first parent.
        Operation::CherryPick(params) => {
            if *replay != ConflictForecast::Clean {
                return Err(not_checked(PICK_MAY_STOP));
            }
            Ok((params.head.oid, Landing::Picked(params.commits.clone())))
        }
        // A put-in (m7-power-tools/09b Q4): his branch's new tip holds
        // exactly `HEAD` plus what went into the commits — the commit
        // Mergewright wrote from the index for `a` (`made`, parent `HEAD`), or
        // absorb's `index_base` (`HEAD` plus every placed hunk) — once the
        // replay is clean. An `R`-editor rewrite writes neither.
        Operation::Rewrite(params) => {
            if *replay != ConflictForecast::Clean {
                return Err(not_checked(REWRITE_MAY_STOP));
            }
            match (params.index_base, params.made.as_slice()) {
                (Some(base), _) | (None, &[base]) => Ok((params.head.oid, Landing::At(base))),
                _ => Err(not_checked(REORDER_NOT_FORECAST)),
            }
        }
        // A merge (m8-polish-and-ship/19d): `HEAD` lands on the source
        // itself, or on the merged tree, once the merge's own forecast is
        // clean.
        Operation::Merge(params) => {
            if *replay != ConflictForecast::Clean {
                return Err(not_checked(MERGE_MAY_STOP));
            }
            Ok((params.into.oid, Landing::Merged(params.clone())))
        }
        // `sets_aside` is `Some` for a rebase, a rewrite, a cherry-pick, a
        // merge, a checkout, a switching create away from `HEAD` or a
        // checked-out put-back only (a pull has no preview).
        _ => Err(not_checked("nothing is put back after this")),
    }
}

/// Where `HEAD` ends before the put-back runs.
enum Landing {
    /// A checkout: this commit.
    At(Oid),
    /// A rebase onto this commit: approximated by one merge (step 2).
    Replayed(Oid),
    /// A cherry-pick of these commits, in order: each replayed exactly,
    /// from its own first parent, as git does.
    Picked(Vec<Oid>),
    /// A merge of this source into `HEAD` ([`merge_landed`]).
    Merged(MergeParams),
}

/// Where a merge leaves `HEAD`, for the put-back forecast.
pub(crate) enum MergeLanded {
    /// The commit the merge lands on: the source (fast-forward) or a
    /// scratch commit of the merged tree.
    At(Oid),
    /// The merge itself would stop on a conflict, or git would refuse it:
    /// `put_back` is `NotChecked("the merge may stop first")`.
    MayStop,
    /// A read that went wrong, in its own words.
    NotChecked(String),
}

/// The commit a merge of `params.source_tip` into `head` lands on.
///
/// Already in (`source` is an ancestor): `head`, which git leaves where it
/// is. A fast-forward, or `--no-ff` over one: `source`, whose tree is the
/// one git ends on either way. Otherwise the three-way merge from their
/// merge base (`merge_tree_step`), its tree made a commit on both, as the
/// rebase landing makes its own. No merge base: git refuses unrelated
/// histories, so the put-back never runs — [`MergeLanded::MayStop`].
///
/// # Errors
/// [`RunError`] from `merge-base` or `merge-tree`: git missing or stopped.
/// A scratch commit that fails is [`MergeLanded::NotChecked`].
pub(crate) fn merge_landed(
    repo: &RepoHandle,
    git: &GitVersion,
    head: Oid,
    params: &MergeParams,
    stop: &dyn Stop,
) -> Result<MergeLanded, RunError> {
    let source = params.source_tip;
    if stop.should_stop() {
        return Err(RunError::Stopped);
    }
    let Some(base) = history_reads::merge_base(repo, head, source)? else {
        return Ok(MergeLanded::MayStop);
    };
    if base == source {
        return Ok(MergeLanded::At(head));
    }
    if base == head {
        return Ok(MergeLanded::At(source));
    }
    let tree = match history_reads::merge_tree_step(repo, git, base, head, source, stop)? {
        MergeStep::Clean { tree } => tree,
        MergeStep::Conflicts { .. } => return Ok(MergeLanded::MayStop),
        MergeStep::NotCheckable(why) => return Ok(MergeLanded::NotChecked(why)),
    };
    match snapshot::scratch_commit(repo, tree, &[head, source], stop) {
        Ok(landed) => Ok(MergeLanded::At(landed)),
        Err(SnapshotError::Stopped) => Err(RunError::Stopped),
        Err(why) => Ok(MergeLanded::NotChecked(why.to_string())),
    }
}

/// The pick's landed commit: each commit's own change replayed onto the
/// result so far (`merge-tree --merge-base=<its parent>`, the chained
/// rebase forecast's step), each tree made a commit. `Ok(Err(_))` is the
/// hole to answer with.
fn picked(
    repo: &RepoHandle,
    git: &GitVersion,
    head: Oid,
    commits: &[Oid],
    stop: &dyn Stop,
) -> Result<Result<Oid, ConflictForecast>, PreviewError> {
    let mut landed = head;
    for commit in commits {
        stopped(stop)?;
        // `<commit> ^<landed>` capped at one is the commit itself, with its
        // parents; empty when the result already holds it.
        let range = match history_reads::commits_between(repo, landed, *commit, 1, stop) {
            Ok(range) => range,
            Err(RunError::Stopped) => return Err(PreviewError::Stopped),
            Err(why) => return Ok(Err(not_checked(&why.to_string()))),
        };
        let Some(base) = range
            .commits
            .last()
            .and_then(|it| it.parents.first().copied())
        else {
            return Ok(Err(not_checked(PICK_MAY_STOP)));
        };
        let merged = match history_reads::merge_tree_step(repo, git, base, landed, *commit, stop) {
            Ok(merged) => merged,
            Err(RunError::Stopped) => return Err(PreviewError::Stopped),
            Err(why) => return Ok(Err(not_checked(&why.to_string()))),
        };
        let tree = match merged {
            MergeStep::Clean { tree } => tree,
            MergeStep::Conflicts { .. } => return Ok(Err(not_checked(PICK_MAY_STOP))),
            MergeStep::NotCheckable(why) => return Ok(Err(not_checked(&why))),
        };
        landed = match snapshot::scratch_commit(repo, tree, &[landed], stop) {
            Ok(landed) => landed,
            Err(SnapshotError::Stopped) => return Err(PreviewError::Stopped),
            Err(why) => return Ok(Err(not_checked(&why.to_string()))),
        };
    }
    Ok(Ok(landed))
}

/// The rebase's landed commit: `merge-tree --write-tree <head> <onto>`, its
/// tree made a commit on `onto`. `Ok(Err(_))` is the hole to answer with.
fn replayed(
    repo: &RepoHandle,
    head: Oid,
    onto: Oid,
    stop: &dyn Stop,
) -> Result<Result<Oid, ConflictForecast>, PreviewError> {
    stopped(stop)?;
    let merged = GitInvocation::new(repo)
        .args([
            GitArg::Flag("merge-tree"),
            GitArg::Flag("--write-tree"),
            GitArg::Flag("--no-messages"),
            GitArg::Oid(head),
            GitArg::Oid(onto),
        ])
        .run_raw(stop);
    let merged = match merged {
        Ok(merged) => merged,
        Err(RunError::Stopped) => return Err(PreviewError::Stopped),
        Err(why) => return Ok(Err(not_checked(&why.to_string()))),
    };
    let tree = match merged.code {
        Some(0) => String::from_utf8_lossy(&merged.output.stdout)
            .lines()
            .next()
            .and_then(|line| Oid::from_hex(line.trim()).ok()),
        // A conflict here is the replay's own: the same hole as a replay
        // forecast that is not clean.
        Some(1) => return Ok(Err(not_checked(MAY_STOP))),
        _ => None,
    };
    let Some(tree) = tree else {
        let said = String::from_utf8_lossy(&merged.output.stderr);
        let said = said.lines().next().unwrap_or("merge-tree gave no tree");
        return Ok(Err(not_checked(said)));
    };
    stopped(stop)?;
    match snapshot::scratch_commit(repo, tree, &[onto], stop) {
        Ok(landed) => Ok(Ok(landed)),
        Err(SnapshotError::Stopped) => Err(PreviewError::Stopped),
        Err(why) => Ok(Err(not_checked(&why.to_string()))),
    }
}

fn stopped(stop: &dyn Stop) -> Result<(), PreviewError> {
    if stop.should_stop() {
        return Err(PreviewError::Stopped);
    }
    Ok(())
}

fn not_checked(why: &str) -> ConflictForecast {
    ConflictForecast::NotChecked(why.to_owned())
}
