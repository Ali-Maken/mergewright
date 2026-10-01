//! What a preview says an operation would do: the plan and each effect it names.

use super::{
    DropSuggestion, FactsError, FilesEffect, Identity, Oid, PathBuf, RefName, Refusal, RunError,
    SetAside, Tip,
};

/// How many replayed commits a rebase forecasts one by one. Past it the
/// forecast is [`ConflictForecast::NotChecked`]: fifty `merge-tree` spawns
/// is already more than a preview should cost, and the honest answer to
/// "would this 300-commit rebase conflict" is that nobody knows.
pub const FORECAST_CAP: usize = 50;

/// How many changed paths a worktree effect lists before saying `more`.
pub(super) const PATHS_CAP: usize = 50;

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

/// How many left-behind branches a plan names; the rest are counted
/// ([`PreviewPlan::left_behind_more`]). Each named one costs a
/// `merge-base`.
pub const LEFT_BEHIND_CAP: usize = 8;

/// A local branch a rewrite or rebase leaves where it is: off the line, it
/// holds a commit the operation replaces (m7/37).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeftBehind {
    /// The branch, in full (`refs/heads/feature-b`).
    pub branch: RefName,
    /// The old commit it stays on: where it meets the line's old top.
    pub on: Oid,
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
    /// Local branches off the line that hold a commit this rewrites or
    /// replays, at most [`LEFT_BEHIND_CAP`]: only the line follows
    /// (ADR-0047 §4), and these stay on the old commits (m7/37). Empty for
    /// every kind but a rebase or rewrite.
    pub left_behind: Vec<LeftBehind>,
    /// How many more such branches there are past the named ones.
    pub left_behind_more: usize,
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
