//! One laid-out commit: its lane, its edges, and any Mistake Radar marker.

use graft_core::commit::Commit;
use graft_core::mistake_radar::MistakeRadarKind;
use graft_core::oid::Oid;

use crate::identity::BranchIdentity;

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
    /// The branch drawn in each column on this row, indexed by column;
    /// `None` where no edge touches the column. Covers the row's own lane,
    /// every lane passing straight through, every lane closing into the node
    /// and every merge lane opening from it — one identity per column is
    /// unambiguous because a column freed on a row is never reused on that
    /// same row. A reserved lane whose tip has not been reached yet has no
    /// edge above its tip, so it is `None` until then. Trailing `None`s are
    /// trimmed.
    ///
    /// Captured when the row is laid out, so a renderer colours any row it
    /// holds — a cached chunk, a viewport that starts mid-history — without
    /// the cursor's state at that moment (m1-the-graph/08).
    pub lanes: Vec<Option<BranchIdentity>>,
    /// A Mistake Radar classification. Always [`None`] on a `Row` `graph`
    /// produces: `graph` lays out the graph and nothing more, and never
    /// calls into `ops`, which is what fills this in as a later pass.
    pub marker: Option<MistakeRadarKind>,
}
