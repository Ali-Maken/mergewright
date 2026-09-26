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
//! - `plan` — what a request plans to: the range, the refusals, the line of
//!   dependent branches ([`dependents`]).
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

mod plan;
mod progress;
pub(crate) mod saved;
mod todo;

pub use plan::{dependents, plan};
pub use progress::progress;
pub use todo::{default_message, squash_groups, validate};

pub(crate) use plan::{line_of, line_picks, messages_of, rebase_rows};
pub(crate) use todo::{GitTodo, git_todo};

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
}

#[cfg(test)]
mod tests;
