//! The backdrop: the one alpha every element of a strip draws at.

use std::collections::BTreeSet;

use super::super::{Emphasis, FADED_ALPHA, OFF_THREAD_ALPHA};
use super::Lit;

/// The one alpha a strip draws every element at: the lowest in play for the
/// frame. It changes only when Focus toggles or a thread comes or goes,
/// never on a cursor move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Backdrop {
    /// No thread on screen: everything at full alpha, no emphasis layer.
    #[default]
    Full,
    /// A thread on screen: everything at [`OFF_THREAD_ALPHA`].
    OffThread,
    /// Focus mode: everything at [`FADED_ALPHA`].
    Faded,
}

impl Backdrop {
    /// The alpha the strip draws at.
    #[must_use]
    pub fn alpha(self) -> f32 {
        match self {
            Backdrop::Full => 1.0,
            Backdrop::OffThread => OFF_THREAD_ALPHA,
            Backdrop::Faded => FADED_ALPHA,
        }
    }

    /// The lowest alpha any of these rows draws at (ADR-0053 Decision 2):
    /// `Faded` if some row has `thread: None` and [`Emphasis::Dimmed`];
    /// else `OffThread` if some row has `thread: Some(_)` (∅ included);
    /// else `Full`. Read off the paint, never off the features that made it,
    /// so a new reason to dim a row cannot be missed here.
    #[must_use]
    pub fn lowest<'a>(lits: impl IntoIterator<Item = Lit<'a>>) -> Backdrop {
        let mut lowest = Backdrop::Full;
        for lit in lits {
            match (lit.thread, lit.emphasis) {
                (None, Emphasis::Dimmed) => return Backdrop::Faded,
                (Some(_), _) => lowest = Backdrop::OffThread,
                (None, Emphasis::Normal) => {}
            }
        }
        lowest
    }

    /// The paint that draws every element at this alpha, through the same
    /// rules as a row's own paint.
    pub(super) fn paint(self) -> Lit<'static> {
        /// No column on the thread: every column off it.
        static NONE_LIT: BTreeSet<usize> = BTreeSet::new();
        match self {
            Backdrop::Full => Lit::default(),
            Backdrop::OffThread => Lit {
                emphasis: Emphasis::Normal,
                thread: Some(&NONE_LIT),
            },
            Backdrop::Faded => Lit {
                emphasis: Emphasis::Dimmed,
                thread: None,
            },
        }
    }
}
