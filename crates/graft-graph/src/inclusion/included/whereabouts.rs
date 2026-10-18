//! Where one commit stands in a view: drawn, folded behind a merge, hidden
//! by a choice, or not reached yet (ease-pass 06c). What a seek to a commit
//! the default fold hides needs, to open the merge instead of missing.

use graft_core::oid::Oid;

use crate::layout::source::CommitSource;

use super::Included;

/// Where a commit stands in one view, as far as the walk has got.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Whereabouts {
    /// A row of this view.
    Shown,
    /// Hidden by the default fold behind this shown merge.
    Folded {
        /// The shown merge that folds it: opening it is the way to it, or to
        /// the next merge in on the way.
        behind: Oid,
    },
    /// Walked, hidden by a view he chose (hidden stashes, stash entries,
    /// first parents only).
    Filtered,
    /// Not decided in this view yet: the walk has not reached it, or a
    /// change of view has it to decide again.
    NotWalked,
}

impl<S: CommitSource> Included<S> {
    /// Where `id` stands in this view; never advances the walk. A hidden
    /// commit behind a merge that is itself hidden is behind the shown merge
    /// at the end of that chain — the one a row stands for.
    #[must_use]
    pub fn whereabouts(&self, id: Oid) -> Whereabouts {
        let Some(handle) = self.source.handle_of(&id) else {
            return Whereabouts::NotWalked;
        };
        // Walked but past the decided prefix (a pull's limit, or a
        // `reinclude` deciding again): `shown` and the fold say nothing yet.
        // That tail is short except just after a view change.
        if !self.walked.contains(handle) || self.sequence[self.decided..].contains(&handle) {
            return Whereabouts::NotWalked;
        }
        if self.shown.contains(handle) {
            return Whereabouts::Shown;
        }
        let Some(fold) = self.fold.as_ref() else {
            return Whereabouts::Filtered;
        };
        let Some(mut merge) = fold.behind(handle) else {
            return Whereabouts::Filtered;
        };
        // Each step is to a merge admitted before it, so the chain ends.
        while let Some(outer) = fold.behind(merge) {
            merge = outer;
        }
        Whereabouts::Folded {
            behind: self.source.id_of(merge),
        }
    }
}
