//! One laid-out commit: its lane, its edges, and any Mistake Radar marker.

use graft_core::commit::Commit;
use graft_core::mistake_radar::MistakeRadarKind;
use graft_core::oid::Oid;

/// One commit, positioned in the graph.
///
/// Hashable so a row cache can keep a fingerprint of rows it evicted and
/// check that a rebuild reproduced them (DESIGN.md §3.6).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Row {
    /// The commit this row draws.
    pub commit: Commit,
    /// This commit's own column: the lane it continues, or a freshly opened
    /// one if none was waiting for it.
    pub lane: usize,
    /// Lanes that converge on this row and close here — bending in, rather
    /// than continuing straight down as `lane` does. Sorted ascending: order
    /// carries no information of its own, so a fixed order keeps two runs
    /// over the same commits byte-identical.
    pub incoming: Vec<usize>,
    /// Lanes leaving this row, one per parent, in parent order: `outgoing[0]`
    /// is always the first-parent edge.
    pub outgoing: Vec<(usize, Oid)>,
    /// A Mistake Radar classification. Always [`None`] on a `Row` `graph`
    /// produces: `graph` lays out the graph and nothing more, and never
    /// calls into `ops`, which is what fills this in as a later pass.
    pub marker: Option<MistakeRadarKind>,
}
