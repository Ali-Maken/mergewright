//! The rewrite engine (ADR-0047): every rewrite of history — squash, fixup,
//! reword, reorder, drop, edit, staged changes put into an older commit, and
//! every `r` rebase — runs git's own `rebase -i` on a todo Graft wrote, with
//! the branches built on his following on one line.
//!
//! Why git's sequencer and not a replay loop of Graft's: stops, abort,
//! kill -9 recovery, M6's banner and M6's resolver all read git's
//! `rebase-merge/` already. One engine keeps them all true.
//!
//! The pieces:
//! - `todo` — pure: squash groups, the default squashed message, validation,
//!   autosquash marking, and the todo text git is handed.
//! - `plan` — what a request plans to: the range and the refusals.
//! - `absorb` — absorb's base, map over it, and rows.
//! - `unpushed` — "your commits", shared by absorb and `a`: down to the
//!   first pushed commit, the base, a merge or the root (at most 10 on the
//!   default or only branch with no base below `HEAD`).
//! - `dependents` — the line of branches built on his ([`dependents`]) and
//!   the refusals over it.
//! - `reads` — the raw history reads both plan from (reads only).
//! - `saved` — the plan saved under `<git-path graft>/rewrite/<entry-id>/`
//!   before the `Pending` line, and the message helper git calls.
//! - `progress` — where a stopped rewrite is, for the banner.
//!
//! Running it is `apply`'s (`apply::engine`): an [`crate::operation::Operation::Rewrite`]
//! or a [`crate::operation::Operation::Rebase`] goes through the same run,
//! and `apply::resume` takes a stopped one on with the same helper.

use std::collections::BTreeMap;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;

use crate::facts::FactsError;
use crate::operation::{Refusal, Tip};

mod absorb;
mod dependents;
mod plan;
mod progress;
mod reads;
pub(crate) mod saved;
mod todo;
mod unpushed;

pub use absorb::absorb_plan_map;
pub use dependents::dependents;
pub use plan::plan;
pub use progress::{edit_stop, progress};
pub use todo::{default_message, squash_groups, validate};
pub use unpushed::ABSORB_FALLBACK_DEPTH;

pub(crate) use dependents::{left_behind, line_of};
pub(crate) use reads::{line_picks, messages_of, rebase_rows};
pub(crate) use todo::{GitTodo, git_todo, git_words};

/// What git does with one row of the todo: git's own letters, never `exec`
/// (ADR-0047 §1), so git's step count is the user's own rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TodoAction {
    /// Keep the commit as it is (replayed if its parent moved).
    Pick,
    /// Keep the commit, with the message in [`RewriteParams::messages`].
    Reword,
    /// Keep the commit and stop on it, so it can be amended.
    Edit,
    /// Meld into the kept row below it (older), messages combined.
    Squash,
    /// Meld into the kept row below it (older), its message dropped.
    Fixup,
    /// Leave the commit out.
    Drop,
}

impl TodoAction {
    /// git's word for it in a todo file.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Pick => "pick",
            Self::Reword => "reword",
            Self::Edit => "edit",
            Self::Squash => "squash",
            Self::Fixup => "fixup",
            Self::Drop => "drop",
        }
    }

    /// Squash or fixup: melds into the row below.
    #[must_use]
    pub fn melds(self) -> bool {
        matches!(self, Self::Squash | Self::Fixup)
    }

    /// Pick, reword or edit: a row that keeps a commit of its own, and so
    /// can be melded into.
    #[must_use]
    pub fn keeps(self) -> bool {
        matches!(self, Self::Pick | Self::Reword | Self::Edit)
    }

    /// git's word read back from a todo or `done` file; `None` for anything
    /// else (`update-ref`, `exec`, `label`, comments).
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        Some(match word {
            "pick" | "p" => Self::Pick,
            "reword" | "r" => Self::Reword,
            "edit" | "e" => Self::Edit,
            "squash" | "s" => Self::Squash,
            "fixup" | "f" => Self::Fixup,
            "drop" | "d" => Self::Drop,
            _ => return None,
        })
    }
}

/// One row of a rewrite: what git does with one commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodoLine {
    /// The action.
    pub action: TodoAction,
    /// The commit, full id.
    pub commit: Oid,
    /// Its subject line, for the rows on screen.
    pub subject: String,
}

/// His branch, then each branch built on it, bottom first. One line, never a
/// fork (ADR-0047 §4). Every branch in it moves, and the entry records all of
/// them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BranchLine {
    /// Bottom first: `branches[0]` is the branch `HEAD` is on.
    pub branches: Vec<Tip>,
}

impl BranchLine {
    /// The branch the run is started on: the top of the line (ADR-0047 §3).
    #[must_use]
    pub fn top(&self) -> Option<&Tip> {
        self.branches.last()
    }

    /// Branches built on his: every one but the first.
    #[must_use]
    pub fn dependents(&self) -> &[Tip] {
        self.branches.split_first().map_or(&[], |(_, rest)| rest)
    }
}

/// A rewrite, planned: what `y` runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteParams {
    /// The branch `HEAD` is on and its tip when planned; `check` refuses
    /// [`Refusal::Moved`] when it moved since.
    pub head: Tip,
    /// Where the first kept row is replayed.
    pub onto: Oid,
    /// His rows, oldest first, as git runs them. The app shows them reversed.
    /// The dependents' commits are not rows: they follow as picks.
    pub todo: Vec<TodoLine>,
    /// Every branch that moves, his first.
    pub line: BranchLine,
    /// Todo index (a reword row, or the kept row of a squash group) → the
    /// whole message. Written before `y`; git's editor never opens.
    pub messages: BTreeMap<usize, String>,
    /// Commit objects Graft wrote from the index (`a` into an older commit),
    /// with no ref. Pinned by the entry until it closes.
    pub made: Vec<Oid>,
    /// Absorb only: `HEAD`'s tree plus every placed hunk, as a commit with
    /// no ref (parent `HEAD`) — the tree the rewrite leaves `HEAD` at when it
    /// goes through. The set-aside then puts the edits back **with the
    /// index**, measured from this commit, so what stays is still staged
    /// (the index-only kind included) and an edit made on past a placed
    /// hunk merges cleanly (06b). Pinned by the entry until it closes.
    /// `None` for every other rewrite, whose put-back applies `S` itself
    /// with `--index` (ADR-0045 amendment 2026-09-26 b, which covers
    /// absorb's as one case).
    pub index_base: Option<Oid>,
}

/// What the user asked to rewrite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RewriteRequest {
    /// `R` on a commit: his branch from `HEAD` down to where it left its
    /// base, and further down to `from` if that is deeper.
    Open {
        /// The row `R` was pressed on.
        from: Oid,
    },
    /// `a` on an older commit: the staged changes put into `target`.
    AmendInto {
        /// The commit that gains them.
        target: Oid,
    },
    /// Absorb: each staged hunk put into the commit on his branch that last
    /// changed its lines ([`crate::absorb::absorb_map`]); a hunk that fits
    /// none stays staged.
    Absorb,
    /// `R` in a rebase preview: the editor on that same rebase, as `y`
    /// would run it (`line` and `drops` filled by
    /// [`crate::preview::plan_as_run`]): every row the rebase replays, the
    /// suggested drops marked `drop`, onto `onto_tip`. Unedited, it runs the
    /// rebase's own todo.
    FromRebase(crate::operation::RebaseParams),
}

/// Why [`plan`] has no plan.
#[derive(Debug, thiserror::Error)]
pub enum RewriteError {
    /// Refused before anything was written, in words.
    #[error("{}", .0.explain())]
    Refused(Refusal),
    /// A read failed.
    #[error("{0}")]
    Facts(FactsError),
    /// The caller stopped.
    #[error("stopped")]
    Stopped,
}

impl From<FactsError> for RewriteError {
    fn from(error: FactsError) -> Self {
        match error {
            FactsError::Stopped => Self::Stopped,
            other => Self::Facts(other),
        }
    }
}

/// What the branches built on his came to: one line, or a fork Graft will
/// not guess through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dependents {
    /// His branch, then each branch built on it, bottom first.
    Line(BranchLine),
    /// Two branches both built on his, neither on the other.
    Fork {
        /// One of them.
        a: RefName,
        /// The other.
        b: RefName,
    },
}

/// A kept row and the rows melded into it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SquashGroup {
    /// Todo index of the row that keeps a commit.
    pub kept: usize,
    /// Todo indices of the squash and fixup rows melded into it, newest
    /// first.
    pub melded: Vec<usize>,
}

/// Why a todo cannot run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TodoProblem {
    /// A squash or fixup row with no kept row below it.
    MeldsIntoNothing {
        /// Its todo index.
        row: usize,
    },
    /// Every row dropped: the branch would be its base again.
    EverythingDropped,
}

impl TodoProblem {
    /// The words the editor shows on the row, and `y` refuses with (R1).
    #[must_use]
    pub fn explain(&self) -> String {
        match self {
            Self::MeldsIntoNothing { .. } => "nothing below to meld into".to_owned(),
            Self::EverythingDropped => {
                "every commit dropped · the branch would be its base again".to_owned()
            }
        }
    }
}

/// Where a stopped rewrite is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteProgress {
    /// This step and how many there are, counting only the user's rows.
    pub step: (usize, usize),
    /// The row git is on.
    pub current: Option<TodoLine>,
    /// Still to apply, oldest first.
    pub pending: Vec<TodoLine>,
    /// The branch of the line this step belongs to: name, i, n (1-based).
    /// `Some` only when the line has more than one branch.
    pub branch: Option<(RefName, usize, usize)>,
    /// A plan of Graft's is saved for this rebase. `false` for one begun in
    /// the shell.
    pub begun_in_graft: bool,
    /// The branches of the line above this step's, bottom up, each with the
    /// commits of its own still in `pending` (m7-power-tools/07d). Empty
    /// when `branch` is `None`.
    pub followers: Vec<Follower>,
}

impl RewriteProgress {
    /// `pending` minus the followers' rows: this branch's rows still to
    /// apply, oldest first. The followers' rows are `pending`'s last ones
    /// (git runs the line bottom up).
    #[must_use]
    pub fn pending_here(&self) -> &[TodoLine] {
        let theirs: usize = self.followers.iter().map(|f| f.commits).sum();
        &self.pending[..self.pending.len().saturating_sub(theirs)]
    }
}

/// A branch of the line still to come at a rewrite stop: `feature-b
/// (Sara): 2 commits follow` (m7-power-tools/07d D2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Follower {
    /// The branch's short name (`feature-b`).
    pub branch: String,
    /// Who wrote its tip, as the saved plan had it before the rewrite
    /// (`%an`): whose branch this is.
    pub tip_author: String,
    /// Its pick-like rows still in the todo.
    pub commits: usize,
}

/// A rebase stopped on an `edit` row (m7-power-tools/07a): git handed him
/// the commit to change. Read by [`edit_stop`] from `rebase-merge/amend`,
/// which git writes at an edit stop only (checked against git 2.54.0 by
/// `git_writes_rebase_merge_amend_at_an_edit_stop`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditStop {
    /// The rewritten commit git stopped on (`rebase-merge/amend`).
    pub stopped: Oid,
    /// `HEAD` is that commit or an amend of it: its first parent is the
    /// stopped commit's. `false` once he commits on top of it. P14's
    /// exception (`Operation::check`) allows an amend only while this holds.
    pub head_on_it: bool,
}

#[cfg(test)]
mod tests;
