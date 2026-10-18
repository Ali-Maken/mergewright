//! [`ReviewBar`]: where the merge's review stands (ticket ease-pass/12;
//! 12b adds the look).

#[cfg(doc)]
use super::HintContext;

/// The review's bar, for [`HintContext::Review`] (design note ease-pass,
/// M15): the review is modal, its own whole list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewBar {
    /// The review's list of what will land: `y commit the merge · j k move
    /// · ⏎ look at it · u undo last pick · esc back to the clashes (y
    /// brings you here again)` (03 Handoff).
    List,
}
