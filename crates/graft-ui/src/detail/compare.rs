//! The compare view's types: which two points, how far apart, and what changed.

#[allow(clippy::wildcard_imports)]
use super::*;

/// A compare's header (m7-14): the two sides by name and what the read
/// found. Names are the row's first local branch, else its first remote,
/// else its short sha — sanitised like every string here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompareHeader {
    /// The pinned side (`x`), `B` in the header's words.
    pub base: SanitizedText,
    /// The cursor's side, `C` in the header's words.
    pub head: SanitizedText,
    /// What is known so far.
    pub state: CompareState,
}

/// Where a compare's read stands, and what it found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompareState {
    /// The span or the file list is still coming.
    Reading,
    /// The cursor is on the base itself.
    IsBase,
    /// `head` has commits of its own since it left `base`.
    Since {
        /// Commits on `head` since the split.
        commits: usize,
        /// The file list's totals; `None` until it lands.
        tally: Option<FileTally>,
    },
    /// `head` is already in `base`: nothing of its own.
    AlreadyIn,
    /// No common ancestor: the diff is plain two-dot.
    NoSharedHistory,
    /// The read failed, and why.
    Failed(SanitizedText),
}

/// A compare's file totals: binary files count to `files` only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileTally {
    /// Files changed.
    pub files: usize,
    /// Lines added.
    pub added: usize,
    /// Lines removed.
    pub removed: usize,
}

/// What a compare's file-list read learnt about the two sides (m7-14):
/// carried on [`FileListView::compare`] so the header can word it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareSummary {
    /// Listed from the split; `commits` on the head side since it.
    Since {
        /// Where the two split: the diff's old side, for anything that
        /// reads it (a blame of an old-side line starts here).
        split: graft_core::oid::Oid,
        /// Commits reachable from head and not from the split.
        commits: usize,
    },
    /// No shared history: listed two-dot, `base..head`.
    NoSharedHistory,
}
