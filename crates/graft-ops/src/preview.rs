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
mod chain;
mod drops;
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

#[cfg(test)]
use ghost::sha1;
#[cfg(test)]
use identity::{parse_ident, parse_offset};
#[cfg(test)]
use revert_to::not_invertible;

use std::path::PathBuf;

use graft_core::commit::{Identity, Timestamp};
use graft_core::journal::{EntryId, HeadRef, JournalEntry};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history_reads::{self, Forecast};
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::version::GitVersion;

use crate::facts::FactsError;
use crate::operation::{
    FilesEffect, Invocation, Operation, Refusal, RepoFacts, SetAside, Tip, short,
};
use crate::undo::{InversePlan, NotInvertible, Step, plan_revert_to as inverse_plans};

/// How many replayed commits a rebase forecasts one by one. Past it the
/// forecast is [`ConflictForecast::NotChecked`]: fifty `merge-tree` spawns
/// is already more than a preview should cost, and the honest answer to
/// "would this 300-commit rebase conflict" is that nobody knows.
pub const FORECAST_CAP: usize = 50;

/// How many changed paths a worktree effect lists before saying `more`.
const PATHS_CAP: usize = 50;

/// A commit id in a plan: one that exists, or one of this plan's ghosts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// A commit the repository already has.
    Real(Oid),
    /// An index into [`PreviewPlan::ghosts`].
    Ghost(usize),
}

/// A commit the operation would create, drawn but never written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GhostCommit {
    /// A synthetic id: SHA-1 over `"graft-ghost\0" + subject + "\0" + n`
    /// (see [`ghost_oid`]). Deterministic, so two previews of the same
    /// operation draw the same rows and the graph's caches agree; never
    /// written to the object database, and never equal to a real commit's
    /// id for any practical purpose.
    pub oid: Oid,
    /// Parents, first parent first.
    pub parents: Vec<Target>,
    /// The subject git would give it.
    pub subject: String,
    /// Who would author it: the replayed commit's author where git keeps it
    /// (rebase, cherry-pick), the repository's own user otherwise (a merge
    /// commit, a revert).
    pub author: Identity,
    /// The commit being replayed, when this ghost is a replay of one: the
    /// row the eye matches it against.
    pub replays: Option<Oid>,
}

/// A ref the operation would move, create or delete.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefMove {
    /// Full ref name, or `origin/main` for a remote branch a push moves.
    pub name: String,
    /// Where it points now; `None` when it does not exist yet.
    pub from: Option<Oid>,
    /// Where it would point; `None` when the operation deletes it.
    pub to: Option<Target>,
}

/// Where `HEAD` would end up. `None` on a plan means `HEAD` stays as it is —
/// including the ordinary case of a branch moving under a `HEAD` that is
/// still on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeadMove {
    /// Attached to a branch, by full ref name.
    ToBranch(String),
    /// Detached at a commit.
    Detached(Target),
}

/// What the operation would do to the working tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorktreeEffect {
    /// Nothing on disk changes.
    Untouched,
    /// Files are rewritten to match the new state.
    Updated {
        /// The paths, capped; the eye only needs a few.
        paths: Vec<PathBuf>,
        /// There were more than the cap.
        more: bool,
    },
    /// Only the index changes: a stash whose one change is staged, with the
    /// file itself as committed (review 86, m1). Not an empty `Updated`,
    /// which reads as "nothing happens"; the staged paths are named.
    IndexOnly {
        /// The staged paths, capped.
        paths: Vec<PathBuf>,
        /// There were more than the cap.
        more: bool,
    },
    /// Uncommitted changes to these paths are thrown away (a `--hard`
    /// reset). The backup still holds them — §6.3 — and the box says so,
    /// but the word the user must read first is `LOSES`.
    Lost {
        /// The dirty tracked paths that would go.
        paths: Vec<PathBuf>,
    },
    /// The whole tree comes back from the journal's backup (the time
    /// machine): no path list, because it is not a diff.
    RestoredFromBackup,
    /// Nothing on disk or in the index changes, and his changed tracked
    /// files stay exactly as they are: a new branch switched to at `HEAD`'s
    /// own commit (`switch -c`), nothing set aside (m7-power-tools/10b).
    /// Not `Untouched`, which does not tell him his edits come along.
    EditsStay,
    /// Files on disk **will** change, and which ones could not be worked
    /// out: the read the preview needed was not there.
    ///
    /// Deliberately not [`WorktreeEffect::Untouched`] and not an empty
    /// `Updated` — both read as "nothing happens to your files", which is
    /// the one thing this does not mean. A hole is named, the way
    /// [`ConflictForecast::NotChecked`] names one.
    NotShown {
        /// Why the list is missing, in the user's terms.
        why: String,
    },
}

/// What `merge-tree` says about the operation, in the only terms it can.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictForecast {
    /// Every pair git was asked about merged cleanly.
    Clean,
    /// At least one pair conflicted. **`Likely`, not certain**: a rebase or
    /// a cherry-pick replays each commit onto the result of the one before,
    /// and this forecast is per commit against its own predecessor (hard
    /// choice 5). A path here may well resolve itself in the real run, and
    /// a clean forecast can still stop.
    Likely {
        /// The paths, in the order git named them.
        paths: Vec<PathBuf>,
    },
    /// No forecast was made, and why: an old git, or more commits than
    /// [`FORECAST_CAP`].
    NotChecked(String),
    /// A rebase or rewrite, replayed step by step with each result fed into
    /// the next (m7/05, `KNOWN_GAPS` :548): the first step that conflicts.
    /// `step` of `of` is git's own count (every todo row with a commit,
    /// drops included, as `rewrite::progress` counts). Nothing past it is
    /// checked.
    StopsAt {
        /// 1-based.
        step: usize,
        /// Every row git counts.
        of: usize,
        /// The commit replayed at that step.
        commit: Oid,
        /// The conflicting paths, in the order git named them.
        paths: Vec<PathBuf>,
    },
}

/// Whether `u` takes this operation back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UndoNote {
    /// `u` restores what the entry recorded.
    Undoable,
    /// It does not, and why — one line for the box.
    NotUndoable(String),
}

/// Everything the preview box and the ghost rows are drawn from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewPlan {
    /// One line: what this would do, in the operation's own words.
    pub description: String,
    /// The exact `git` lines that would run — `op.plan(&facts)`'s, never
    /// hand-written (`KNOWN_GAPS` row 229).
    pub commands: Vec<String>,
    /// The commits that would be created, parents before children, so an
    /// index in [`Target::Ghost`] always points at a ghost already drawn.
    pub ghosts: Vec<GhostCommit>,
    /// The refs that would move.
    pub ref_moves: Vec<RefMove>,
    /// Where `HEAD` would go, when it goes anywhere.
    pub head: Option<HeadMove>,
    /// What the working tree would look like afterwards.
    pub worktree: WorktreeEffect,
    /// What git is likely to do about conflicts.
    pub conflicts: ConflictForecast,
    /// Whether `u` takes it back.
    pub undo: UndoNote,
    /// The tips this plan was drawn against: `check` refuses
    /// [`Refusal::Moved`] if any of them has moved by confirm time.
    pub against: Vec<Tip>,
    /// What a rebase or checkout would set aside first and put back after
    /// (ADR-0045 §4): `op.sets_aside(facts)`, never a second count — but
    /// for a put-in or absorb, whose staged edits go into commits, `files`
    /// counts only the unstaged ones, the part put back (0 when none).
    pub set_aside: Option<SetAside>,
    /// Whether putting those edits back is likely to clash (ADR-0045 §8):
    /// `Some` exactly when [`PreviewPlan::set_aside`] is. A forecast, like
    /// [`PreviewPlan::conflicts`]: a rebase's is made against one merge
    /// standing for the replayed branch, and a rebase that may stop first
    /// is `NotChecked`.
    pub put_back: Option<ConflictForecast>,
    /// A put-back's `your files:` line (`Operation::Recover`, m7/23);
    /// `None` for every other operation.
    pub files: Option<FilesEffect>,
    /// A rebase's squash-merge check of its bottom branch (m7/05): `Some`
    /// when the bottom branch matched a commit on `onto`, or when the check
    /// was not made; `None` when nothing matched, and for every other kind.
    pub already_in: Option<DropSuggestion>,
    /// Branches of the line whose upstream holds a commit this rewrites:
    /// the next push of each needs force (m7/05).
    pub published: Vec<RefName>,
    /// Each resulting commit whose message Graft writes (a reword, or the
    /// kept row of a squash group), keyed by its ghost (m7/05).
    pub messages: Vec<(Target, String)>,
}

/// Why a preview could not be drawn.
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    /// The repository could not be read.
    #[error("{0}")]
    Facts(FactsError),
    /// A `git` read failed.
    #[error("{0}")]
    Read(RunError),
    /// The operation would be refused, and there is no point drawing it: a
    /// `--ff-only` merge of diverged branches, a re-push of a commit that
    /// is not on the branch.
    #[error("{}", .0.explain())]
    Refused(Refusal),
    /// The caller asked to stop.
    #[error("stopped")]
    Stopped,
}

impl PreviewError {
    /// One line for the box.
    #[must_use]
    pub fn explain(&self) -> String {
        self.to_string()
    }
}

impl From<RunError> for PreviewError {
    fn from(why: RunError) -> Self {
        match why {
            RunError::Stopped => PreviewError::Stopped,
            other => PreviewError::Read(other),
        }
    }
}

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
    if stop.should_stop() {
        return Err(PreviewError::Stopped);
    }
    checked(op, facts)?;
    // ADR-0049: a file git doesn't keep, where the write puts one of its
    // own, is refused before any forecast — and again by `apply` at `y`.
    refuse_in_the_way(repo, op, facts, stop)?;
    // A rebase's landed commit, from its own chained forecast: what the
    // put-back is measured against (05b: drops skipped, the line replayed).
    let mut landed = None;
    let mut drawn = match op {
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
        Operation::Rewrite(params) => rewrite::rewrite(repo, git, params, stop)?,
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
    drawn.description = op.describe();
    drawn.commands = op
        .plan(facts)
        .iter()
        .map(Invocation::command_line)
        .collect();
    drawn.against = op.tips();
    drawn.set_aside = op
        .sets_aside(facts)
        .map(|aside| shown_aside(op, facts, aside));
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
    Ok(drawn)
}

/// The set-aside the preview counts: what is set aside **and put back**.
/// A put-in's and an absorb's staged edits go into commits, so only the
/// unstaged part comes back — the landing's "your unstaged edits" (review
/// m7-09..21c Minor 6); every other kind puts back all it set aside. The
/// set-aside itself stays `Some`, so the put-back is still forecast.
fn shown_aside(op: &Operation, facts: &RepoFacts, aside: SetAside) -> SetAside {
    match op {
        Operation::Rewrite(params) if !params.made.is_empty() || params.index_base.is_some() => {
            SetAside {
                files: facts
                    .status
                    .entries
                    .iter()
                    .filter(|entry| entry.is_unstaged())
                    .count(),
            }
        }
        _ => aside,
    }
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
        messages: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
