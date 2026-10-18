//! What a preview says an operation would do: the plan and each effect it names.

use super::{
    DropSuggestion, FilesEffect, Identity, MergeLook, Oid, PathBuf, RefName, SetAside, Tip,
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
    /// (see [`ghost_oid`](crate::preview::ghost_oid)). Deterministic, so two previews of the same
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

/// A local branch forked off a commit a rewrite replaces (ADR-0059 §1): off
/// the line, its tip outside the replayed range, meeting the line's old top
/// at a replaced commit, with no merge in its own commits. Its fate is fixed
/// in the preview and run as previewed (§2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fork {
    /// The branch, in full (`refs/heads/feature-x`).
    pub branch: RefName,
    /// The replaced commit it leaves the line at (the old id): where it meets
    /// the line's old top, [`LeftBehind::on`] for the same branch.
    pub at: Oid,
    /// Its own commits: `at..branch`.
    pub commits: usize,
    /// Whether it follows, and where to, or why it stays.
    pub fate: ForkFate,
}

/// What a rewrite does with a [`Fork`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForkFate {
    /// Its own commits are replayed onto the new `at`, in the same rebase.
    Follows {
        /// What the new `at` is: the ghost of `at`'s row (or of the row
        /// below it, when `at`'s row is dropped).
        onto: Target,
    },
    /// It stays on the old commits, and why.
    Stays(WhyStays),
}

/// Why a [`Fork`] stays where it is (ADR-0059 §2). A forecast hole is never
/// read as clean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WhyStays {
    /// One of its own commits would conflict, replayed onto the new `at`.
    WouldConflict {
        /// That commit.
        commit: Oid,
        /// The conflicting paths, in the order git named them.
        paths: Vec<PathBuf>,
    },
    /// The line's own forecast stops at or below `at`'s row: nothing above
    /// the stop is checked.
    StopsBelow,
    /// A merge lies in its own commits.
    HasMerge {
        /// The first merge found.
        merge: Oid,
    },
    /// It is pushed (ADR-0059 §6.2): its upstream holds a commit of
    /// `onto..F` (its fork point and the replaced commits under it
    /// included), or any remote-tracking ref holds one of its own commits.
    /// Moving it would need a force-push he never asked for.
    Pushed {
        /// The remote-tracking ref that holds it, in full
        /// (`refs/remotes/origin/feature-x`), ADR-0059 §6.2: (a) its
        /// remote-tracking upstream or (b) a remote ref of its own name,
        /// holding a commit of `onto..F`; or (c) any remote ref holding an
        /// own commit. Never a local branch.
        upstream: RefName,
    },
    /// It is checked out in another worktree. Git would move it all the
    /// same and leave that worktree's index and files on the old commit
    /// (ADR-0059 §6.1): the gate is Mergewright's.
    Elsewhere {
        /// That worktree.
        worktree: PathBuf,
    },
    /// It shares commits with another fork that follows: a fork of a fork,
    /// which Mergewright does not move (ADR-0059 §2).
    Nested {
        /// The fork it shares commits with.
        on: RefName,
    },
    /// It is the default branch.
    DefaultBranch,
    /// Past the first [`LEFT_BEHIND_CAP`]: the fate of every fork counted in
    /// [`PreviewPlan::forks_more`], which is never named, so never in
    /// [`PreviewPlan::forks`].
    PastCap,
    /// Its forecast was not made, and why (an old git, more than
    /// [`FORECAST_CAP`] commits, a read that failed).
    NotChecked(String),
    /// A branch on the fork's own commits fails a gate (ADR-0059 §6.1), so
    /// the whole fork stays. `why` is only [`Self::Elsewhere`],
    /// [`Self::Pushed`] or [`Self::DefaultBranch`].
    Carries {
        /// That branch, in full.
        branch: RefName,
        /// The gate it fails.
        why: Box<WhyStays>,
    },
    /// Following would drop a commit the fork is built on (ADR-0059 §6.3):
    /// dropped, or moved after its fork point's row.
    WouldLose {
        /// That commit, as it was.
        commit: Oid,
    },
    /// Following would add a commit the fork was not built on (ADR-0059
    /// §6.3): moved to or under its fork point's row, or folded into it.
    WouldGain {
        /// That commit, as it was.
        commit: Oid,
    },
}

impl WhyStays {
    /// mergewright-ops' clause, no keys: `would conflict in src/a.rs`.
    #[must_use]
    pub fn words(&self) -> String {
        match self {
            Self::WouldConflict { paths, .. } => match paths.as_slice() {
                [] => "would conflict".to_owned(),
                [one] => format!("would conflict in {}", one.display()),
                [first, rest @ ..] => format!(
                    "would conflict in {} and {} more",
                    first.display(),
                    rest.len()
                ),
            },
            Self::StopsBelow => "the rewrite stops below it".to_owned(),
            Self::HasMerge { .. } => "has a merge".to_owned(),
            Self::Pushed { upstream } => format!(
                "pushed to {}",
                String::from_utf8_lossy(upstream.shorthand())
            ),
            Self::Elsewhere { worktree } => format!("checked out in {}", worktree.display()),
            Self::Nested { on } => format!(
                "forked from {}, which moves",
                String::from_utf8_lossy(on.shorthand())
            ),
            Self::DefaultBranch => "the default branch".to_owned(),
            Self::PastCap => format!("past the first {LEFT_BEHIND_CAP}"),
            Self::NotChecked(why) => format!("not checked: {why}"),
            Self::Carries { branch, why } => format!(
                "{} is {}",
                String::from_utf8_lossy(branch.shorthand()),
                why.words()
            ),
            Self::WouldLose { commit } => {
                format!("would lose {}", crate::operation::short(*commit))
            }
            Self::WouldGain { commit } => {
                format!("would gain {}", crate::operation::short(*commit))
            }
        }
    }
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
    /// [`Refusal::Moved`](crate::operation::Refusal::Moved) if any of them has moved by confirm time.
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
    ///
    /// Since ADR-0059 a rewrite's forks that follow are not here: it holds
    /// the forks that stay, and a backup parked at a tip of the line (not a
    /// fork, §1). `r` lists every such branch as before (its forks never
    /// follow yet). Read [`PreviewPlan::forks`] for the fates.
    pub left_behind: Vec<LeftBehind>,
    /// How many more such branches there are past the named ones.
    pub left_behind_more: usize,
    /// A rewrite's forks (ADR-0059), the first [`LEFT_BEHIND_CAP`] named,
    /// each with its fate; in order: the ones that follow, then the ones
    /// that stay, each oldest fork point first. Empty for every other kind,
    /// `r` included.
    pub forks: Vec<Fork>,
    /// Forks past the named ones: each stays ([`WhyStays::PastCap`]).
    pub forks_more: usize,
    /// Each resulting commit whose message Mergewright writes (a reword, or the
    /// kept row of a squash group), keyed by its ghost (m7/05).
    pub messages: Vec<(Target, String)>,
    /// A merge's own reads (ease-pass 10): what comes in, what git
    /// combines, what clashes. `Some` only on a merge that changes something.
    pub merge: Option<MergeLook>,
}
