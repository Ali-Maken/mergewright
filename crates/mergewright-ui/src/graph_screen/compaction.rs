//! How many lanes the graph screen draws, and which: the cap, whether the
//! user lifted it, and the columns a fold weighs first (DESIGN.md §3.5).

use mergewright_graph::fold::{Fold, Keep};
use mergewright_graph::row::Row;

/// Lane compaction as the screen applies it (DESIGN.md §3.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compaction {
    /// `graph.max_lanes`: the most lanes drawn, the rail included.
    pub max_lanes: usize,
    /// The user pressed the expand key: no cap until he presses it again. A
    /// lane with no commit in view is still not drawn (ease-pass 06b).
    pub expanded: bool,
    /// Columns a fold weighs above the ordinary rank: the selected commit's
    /// own column, reserved integration lanes and the selection's thread
    /// (ADR-0025). A priority, not a veto — one still folds if the budget
    /// cannot hold it.
    pub keep: Keep,
}

impl Compaction {
    /// Compaction at `max_lanes`, collapsed, with nothing kept but lane 0.
    #[must_use]
    pub fn at(max_lanes: usize) -> Self {
        Compaction {
            max_lanes,
            expanded: false,
            keep: Keep::default(),
        }
    }

    /// The fold `rows` draw through: at the cap, or uncapped while expanded.
    pub(super) fn plan<'a>(&self, rows: impl IntoIterator<Item = &'a Row>) -> Fold {
        let rows: Vec<Row> = rows.into_iter().cloned().collect();
        let cap = if self.expanded {
            usize::MAX
        } else {
            self.max_lanes
        };
        Fold::plan(cap, &self.keep, &rows)
    }
}
