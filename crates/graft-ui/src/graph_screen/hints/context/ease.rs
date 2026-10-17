//! The ease pass's bar states (ticket ease-pass/03b): which graph row the
//! cursor is on and where the clash screen stands, with the names and
//! counts their words carry.

use graft_term::SanitizedText;

#[cfg(doc)]
use super::HintContext;
use crate::conflict_resolver::SideRoles;

/// Which graph row the cursor is on, for [`HintContext::GraphRow`]: each
/// kind says its own next steps (design note ease-pass, G13 and M1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowBar {
    /// His own branch's row: `⏎ open · P push · c commit`.
    Own,
    /// A teammate's or the trunk's row: `m merge <b> into yours · ⏎ open ·
    /// C switch to <b> · x compare`, `<b>` its [`bar_name`].
    Elsewhere {
        /// The branch the row's badge names, whole; the bar draws its last
        /// path segment.
        branch: SanitizedText,
    },
    /// A folded merge: `space show the N commits inside · ⏎ open · x compare`.
    FoldedMerge {
        /// How many commits the fold hides.
        inside: u32,
    },
    /// A merge opened by `space`: `space hide the commits inside`.
    Opened,
    /// Just after a commit (C13): `u undo the commit, your edits come back ·
    /// P push · c commit again`.
    JustCommitted,
    /// Just after a merge (M18): `u undo the merge · P push · space show
    /// the N commits inside`.
    JustMerged {
        /// How many commits the merge brought in.
        inside: u32,
    },
}

/// The clash screen's state, for [`HintContext::Clash`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClashBar {
    /// Where the cursor's clash and file stand.
    pub at: ClashAt,
    /// Which operation the clashes belong to.
    pub op: ClashOp,
}

/// Where the clash screen's cursor stands (design note ease-pass, M11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClashAt {
    /// The cursor's clash is open: the picks.
    Open,
    /// A pick was made and the file still has an open clash.
    Picked,
    /// The file is solved and another file still clashes.
    FileSolved {
        /// The next file with an open clash, as the list names it.
        next: SanitizedText,
    },
    /// Every clash is solved.
    AllSolved,
}

/// Which operation the clashes belong to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClashOp {
    /// A merge: finished by `c` (P later points it at the review).
    Merge,
    /// A rebase, cherry-pick, revert or stash put-back: a file is finished
    /// by `⏎` and the operation goes on by `c`. Carries the side roles, so
    /// `o`/`t` name the sides as today's resolver does — `take upstream` in
    /// a rebase, never `yours` for the side that is not
    /// (`conflict_resolver::names`, `SideRoles::ours`).
    Other(SideRoles),
}

/// The bar's form of a branch name: its last path segment
/// (`sara/fix-toc` → `fix-toc`, Q2 a). Already sanitized.
#[must_use]
pub fn bar_name(full: &SanitizedText) -> &str {
    let name = full.as_str();
    name.rsplit('/').next().unwrap_or(name)
}
