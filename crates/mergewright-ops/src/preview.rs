//! Ghost Preview planning (PROMPT.md §1.5, §1.7): what an operation would
//! do, drawn before it runs.
//!
//! A [`PreviewPlan`] is a picture, not a promise. It says where refs land,
//! which commits would be created (as **ghosts** — synthetic ids that are
//! never written to the object database), what the working tree would look
//! like afterwards, and whether git is likely to stop on a conflict. The
//! plan and the operation are the same [`Operation`] value, pinned to the
//! tips it was drawn against (M5 design note, hard choice 3), so what the
//! user confirms is what runs: if a tip moved in between, `check` refuses
//! rather than quietly re-planning.
//!
//! Two rules bind everything here:
//!
//! - **`commands` is never written by hand.** It is `op.plan(&facts)`'s own
//!   lines, so the box and the `Command Log` cannot drift from what runs
//!   (`KNOWN_GAPS` row 229).
//! - **A forecast says "may", never "will".** A rebase or rewrite replays
//!   several commits, each onto the result of the last; the forecast follows
//!   that chain (m7/05, `chain`): each clean step's tree becomes an
//!   unreferenced scratch commit, the next step's "ours", so no ref, index
//!   or file is written. The first conflict is [`ConflictForecast::StopsAt`]
//!   and nothing past it is checked. Capped at [`FORECAST_CAP`]; git older
//!   than 2.40 leaves it `NotChecked`, never a clean bill.

mod as_run;
mod aside;
mod chain;
mod drops;
mod forks;
mod ghost;
mod identity;
mod kinds;
mod landing;
mod put_back;
mod recover;
mod revert_to;
mod rewrite;
mod stash;

pub use as_run::plan_as_run;
pub use drops::{AlreadyIn, DROP_CHECK_CAP, DropSuggestion, suggest_drops};
pub use ghost::ghost_oid;
pub use revert_to::plan_revert_to;

use as_run::{checked, fold, refuse_in_the_way};
use identity::identity_now;
pub(crate) use landing::is_ancestor;
use landing::{Landing, Replay, dirty_tracked, forecast_one, worktree};

mod effects;
mod error;
mod merge_look;
use effects::PATHS_CAP;
pub use effects::{
    ConflictForecast, FORECAST_CAP, Fork, ForkFate, GhostCommit, HeadMove, LEFT_BEHIND_CAP,
    LeftBehind, PreviewPlan, RefMove, Target, UndoNote, WhyStays, WorktreeEffect,
};
pub use error::PreviewError;
pub(crate) use merge_look::conflicts_from;
pub use merge_look::{INCOMING_CAP, MergeLook};

#[cfg(test)]
use ghost::sha1;
#[cfg(test)]
use identity::{parse_ident, parse_offset};
#[cfg(test)]
use revert_to::not_invertible;

use std::path::PathBuf;

use mergewright_core::commit::{Identity, Timestamp};
use mergewright_core::journal::{EntryId, HeadRef, JournalEntry};
use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::history_reads::{self, Forecast};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};
use mergewright_git::version::GitVersion;

use crate::facts::FactsError;
use crate::operation::{
    FilesEffect, Invocation, Operation, Refusal, RepoFacts, SetAside, Tip, short,
};
use crate::undo::{InversePlan, NotInvertible, Step, plan_revert_to as inverse_plans};

/// Draw what `op` would do.
///
/// `facts` is the same [`RepoFacts`] the operation would be checked against,
/// read once by the caller; the reads here are the ones facts cannot hold —
/// merge bases, commit ranges, `merge-tree` forecasts, changed paths.
///
/// # Errors
/// [`PreviewError`]: a `git` read failed, the repository could not be read,
/// the caller stopped, `op.check(facts)` refused it (checked first, exactly
/// as `apply` checks before writing), or the operation would plainly be
/// refused and there is nothing worth drawing ([`Refusal::NotFastForward`],
/// [`Refusal::NotOnBranch`]). Files git doesn't keep, standing where the
/// write puts its own, are read once, right after `check` and before any
/// forecast, by [`crate::in_the_way::blocking`] — the read `apply` repeats
/// at `y` — and refused as [`Refusal::InTheWay`], or for a stash apply or
/// pop as [`Refusal::StashInTheWay`] (ADR-0049).
///
/// A stash apply or pop is the one exception to "exactly as `check`": its
/// `TouchesChangedFiles` becomes [`Refusal::StashInTheWay`] naming the
/// unsaved or untracked paths, the same refusal in his words (m6-conflicts/86).
pub fn plan(
    repo: &RepoHandle,
    git: &GitVersion,
    op: &Operation,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    planned(repo, git, op, facts, stop).map(|(drawn, _)| drawn)
}

/// [`plan`], and for a rewrite the sections of the forks that follow
/// (ADR-0059), which `plan_as_run` puts into the operation `y` runs.
fn planned(
    repo: &RepoHandle,
    git: &GitVersion,
    op: &Operation,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<(PreviewPlan, Vec<crate::rewrite::ForkSection>), PreviewError> {
    if stop.should_stop() {
        return Err(PreviewError::Stopped);
    }
    checked(op, facts)?;
    // ADR-0049: a file git doesn't keep, where the write puts one of its
    // own, is refused before any forecast — and again by `apply` at `y`.
    refuse_in_the_way(repo, op, facts, stop)?;
    let (mut drawn, landed, sections) = kind_plan(repo, git, op, facts, stop)?;
    drawn.description = op.describe();
    drawn.commands = op
        .plan(facts)
        .iter()
        .map(Invocation::command_line)
        .collect();
    drawn.against = op.tips();
    drawn.set_aside = op
        .sets_aside(facts)
        .filter(|_| !up_to_date_merge(op, &drawn))
        .map(|aside| aside::shown_aside(op, facts, aside));
    // Only with edits aside: a clean tree spawns nothing more.
    drawn.put_back = match drawn.set_aside {
        Some(_) => Some(put_back::forecast(
            repo,
            git,
            op,
            facts,
            &drawn.conflicts,
            landed,
            stop,
        )?),
        None => None,
    };
    Ok((drawn, sections))
}

/// A merge that moves no ref: git would answer "Already up to date." and do
/// nothing, so the box is the nothing-to-do one and nothing is set aside
/// for it or put back (19d-a2, ADR-0045 §8: `put_back` follows `set_aside`).
fn up_to_date_merge(op: &Operation, drawn: &PreviewPlan) -> bool {
    matches!(op, Operation::Merge(_)) && drawn.ref_moves.is_empty()
}

/// The kind's own picture, before the fields every kind fills the same way;
/// a rebase's landed commit, from its own chained forecast — what the
/// put-back is measured against (05b: drops skipped, the line replayed);
/// and a rewrite's follower sections (ADR-0059).
fn kind_plan(
    repo: &RepoHandle,
    git: &GitVersion,
    op: &Operation,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<(PreviewPlan, Option<Oid>, Vec<crate::rewrite::ForkSection>), PreviewError> {
    let mut landed = None;
    let mut sections = Vec::new();
    let drawn = match op {
        Operation::StashApply { index, commit } | Operation::StashPop { index, commit } => {
            let (worktree, conflicts) = stash::forecast_stash(repo, git, *index, *commit, stop)?;
            PreviewPlan {
                worktree,
                conflicts,
                ..blank()
            }
        }
        Operation::Merge(params) => kinds::merge(repo, git, params, stop)?,
        Operation::Rebase(params) => {
            let (drawn, his) = kinds::rebase(repo, git, params, stop)?;
            landed = his;
            drawn
        }
        Operation::Rewrite(params) => {
            let (drawn, forks) = rewrite::rewrite(repo, git, params, stop)?;
            sections = forks;
            drawn
        }
        Operation::CherryPick(params) => kinds::cherry_pick(repo, git, params, stop)?,
        Operation::Revert(params) => kinds::revert(repo, git, params, stop)?,
        Operation::Reset(params) => kinds::reset(repo, params, facts)?,
        Operation::Checkout(params) => kinds::checkout(repo, params, facts)?,
        Operation::BranchCreate(params) => kinds::branch_create(repo, params, facts)?,
        Operation::TagCreate(params) => kinds::tag_create(params),
        Operation::BranchDelete(params) | Operation::TagDelete(params) => kinds::ref_delete(params),
        Operation::BranchFromStash(params) => kinds::branch_from_stash(repo, params, facts, stop)?,
        Operation::Push(params) => kinds::push(repo, params, facts, stop)?,
        Operation::Recover(params) => recover::recover(repo, params, facts, stop)?,
        other => return Err(PreviewError::Refused(Refusal::NotPreviewable(other.kind()))),
    };
    Ok((drawn, landed, sections))
}

/// A plan with the fields every kind fills the same way, ready to be
/// overwritten by the kind's own. `description`, `commands` and `against`
/// are filled by [`plan`] itself, from the operation, so no kind can write
/// a command line by hand.
fn blank() -> PreviewPlan {
    PreviewPlan {
        description: String::new(),
        commands: Vec::new(),
        ghosts: Vec::new(),
        ref_moves: Vec::new(),
        head: None,
        worktree: WorktreeEffect::Untouched,
        conflicts: ConflictForecast::Clean,
        undo: UndoNote::Undoable,
        against: Vec::new(),
        set_aside: None,
        put_back: None,
        files: None,
        already_in: None,
        published: Vec::new(),
        left_behind: Vec::new(),
        left_behind_more: 0,
        forks: Vec::new(),
        forks_more: 0,
        messages: Vec::new(),
        merge: None,
    }
}

/// The blank plan every kind starts from: nothing moves, nothing is
/// written, `Clean`, undoable. For a caller building a plan by hand (a
/// test's fixture) without naming every field.
impl Default for PreviewPlan {
    fn default() -> Self {
        blank()
    }
}

#[cfg(test)]
mod tests;
