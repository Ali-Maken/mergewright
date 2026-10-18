//! Tags a graph row carries after its subject until the next write starts
//! (m7-power-tools/09): `✎ changed` on each commit a put-in changed.
//! Journal-screen (24) sets its own through [`RowTags::set`].
//!
//! Cleared in one place, `operate::claim_write`, so every write — undo,
//! redo and a stage included — ends them.

use mergewright_core::oid::Oid;
use mergewright_ui::graph_screen::RowTag;

/// Which commits carry a tag, and which.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowTags {
    at: Vec<(Oid, RowTag)>,
}

impl RowTags {
    /// Tag `at`, replacing any tag it had.
    pub(crate) fn set(&mut self, at: Oid, tag: RowTag) {
        self.at.retain(|(id, _)| *id != at);
        self.at.push((at, tag));
    }

    /// The tag on `id`, if any.
    #[must_use]
    pub(crate) fn on(&self, id: Oid) -> Option<&RowTag> {
        self.at
            .iter()
            .find_map(|(at, tag)| (*at == id).then_some(tag))
    }

    /// No row carries a tag any more.
    pub(crate) fn clear(&mut self) {
        self.at.clear();
    }
}
