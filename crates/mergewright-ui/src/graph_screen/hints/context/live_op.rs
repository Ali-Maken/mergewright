//! What an operation that is live gives the graph's bar: how one press
//! reaches the banner, and what `c` does there.

#[cfg(doc)]
use super::HintContext;

/// Which one press reaches the banner from the graph
/// ([`HintContext::OperationLive`], m8/20d): the pane cycle forward
/// (`app.next_pane`) or backward (`app.prev_pane`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WayToBanner {
    /// One `⇥`.
    Forward,
    /// One `⇧⇥`.
    Back,
}

/// What `c` does in the graph or the changes pane while an operation is
/// live (m6-conflicts/52): the banner's `c` where that acts, else words
/// that refuse — so its hint says `finish`, `continue`, or nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveC {
    /// A merge: `c finish` opens the finish summary.
    Finish,
    /// A rebase, cherry-pick or revert Mergewright began: `c continue`.
    Continue,
    /// A rewrite Mergewright began, stopped on an `edit` row (m7-power-tools/07b):
    /// `c continue`, and `a amend it` leads — `a` amends the stopped commit
    /// with what is staged, keeping its message.
    ContinueAmends,
    /// A bisect, or a rebase, cherry-pick or revert the shell began: `c`
    /// only refuses in words, so no `c` hint.
    Refuses,
}
