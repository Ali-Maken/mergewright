//! [`ClashBar`]: where the clash screen stands, and whose sides it names
//! (ticket ease-pass/03b).

use graft_term::SanitizedText;

#[cfg(doc)]
use super::HintContext;
use crate::conflict_resolver::SideRoles;

/// The clash screen's state, for [`HintContext::Clash`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClashBar {
    /// Where the cursor's clash and file stand.
    pub at: ClashAt,
    /// The sides of the operation the clashes belong to: a merge's
    /// `yours`/`theirs`, a rebase's `upstream`/`yours`, and so on.
    pub roles: SideRoles,
}

impl ClashBar {
    /// Whether the clashes are a merge's: finished by `c finish`, its files
    /// done by their last pick. Every other operation's file is finished by
    /// `⏎` and the operation goes on by `c continue`.
    #[must_use]
    pub fn is_merge(&self) -> bool {
        self.roles == SideRoles::Merge
    }
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
        /// The next file with an open clash, its whole path; the bar draws
        /// its last segment ([`bar_name`](super::bar_name)).
        next: SanitizedText,
    },
    /// Every clash is solved.
    AllSolved,
}
