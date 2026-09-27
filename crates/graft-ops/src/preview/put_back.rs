//! The put-back forecast (ADR-0045 §8, m6-conflicts/84): when a rebase or a
//! switch sets the user's edits aside, will putting them back clash?
//!
//! Three steps, mirroring what `apply` will do at step 6:
//! 1. `git stash create` of the tree now → `S'` (on a copy of the index);
//! 2. `landed`, the commit `HEAD` ends on: a checkout's target, or for a
//!    rebase the chained forecast's result after his last row (drops
//!    skipped, 05b); where the chain did not reach it, one
//!    `merge-tree HEAD <upstream>` made a commit stands in — for the
//!    replayed branch, which is never built (m5 hard choice 5);
//! 3. `git merge-tree --merge-base=HEAD <landed> <S'>`: the three-way merge
//!    `stash apply` does, whose base is `S'^1`, which is `HEAD`.
//!
//! **Every branch passed to `merge-tree` is a commit.** git 2.40 is the
//! oldest `merge-tree --merge-base` Graft forecasts with; whether it accepts
//! a bare tree there was not confirmed (only 2.54 was tried), so the rebase's
//! landed tree is made a commit (`commit-tree`) rather than relying on it.
//!
//! Nothing here fails the preview: a read that goes wrong is
//! `NotChecked(<why>)`, a hole the box names. Only a stop is an error.

use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::version::GitVersion;

use crate::operation::{CheckoutTarget, Operation, RecoverParams, RecoverTarget, RepoFacts};
use crate::snapshot::{self, SnapshotError};

use super::{ConflictForecast, PreviewError, forecast_one, kinds};

/// The oldest git whose `merge-tree` takes `--merge-base`, which step 3
/// needs. Graft itself runs on 2.38 (PROMPT.md §1); below this the put-back
/// is not checked, and says so.
const MERGE_BASE_MIN: GitVersion = GitVersion {
    major: 2,
    minor: 40,
    patch: 0,
};

/// Why a rebase's put-back is not forecast: the replay itself may stop, and
/// the tree the edits would go back onto is then nobody's guess.
const MAY_STOP: &str = "the rebase may stop first";

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
    let (head, landing) = match op {
        Operation::Checkout(params) => {
            let target = match &params.to {
                CheckoutTarget::Branch(name) => facts.refs.get(name).copied().flatten(),
                CheckoutTarget::Detached(oid) => Some(*oid),
                CheckoutTarget::Track { remote, .. } => Some(remote.oid),
            };
            match (kinds::head_tip(facts), target) {
                (Some(head), Some(target)) => (head, Landing::At(target)),
                _ => return Ok(not_checked("where HEAD lands could not be read")),
            }
        }
        Operation::Rebase(params) => {
            if *replay != ConflictForecast::Clean {
                return Ok(not_checked(MAY_STOP));
            }
            match rebase_landed {
                Some(landed) => (params.branch.oid, Landing::At(landed)),
                None => (params.branch.oid, Landing::Replayed(params.onto_tip)),
            }
        }
        // A new branch switched to at `at`: `HEAD` lands there, as a
        // checkout's does (m7-power-tools/10b).
        Operation::BranchCreate(params) => match kinds::head_tip(facts) {
            Some(head) => (head, Landing::At(params.at)),
            None => return Ok(not_checked("where HEAD lands could not be read")),
        },
        // A branch put back while checked out: a `reset --hard` to `to`.
        Operation::Recover(RecoverParams {
            target: RecoverTarget::MoveRef { to, expected, .. },
            ..
        }) => (*expected, Landing::At(*to)),
        // `sets_aside` is `Some` for a rebase, a rewrite, a checkout, a
        // switching create away from `HEAD` or a checked-out put-back only.
        _ => return Ok(not_checked("nothing is put back after this")),
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
    };
    stopped(stop)?;
    match forecast_one(repo, git, Some(head), landed, edits, stop) {
        Ok(forecast) => Ok(forecast),
        Err(PreviewError::Stopped) => Err(PreviewError::Stopped),
        Err(why) => Ok(not_checked(&why.explain())),
    }
}

/// Where `HEAD` ends before the put-back runs.
enum Landing {
    /// A checkout: this commit.
    At(Oid),
    /// A rebase onto this commit: approximated by one merge (step 2).
    Replayed(Oid),
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
