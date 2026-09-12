//! What kind of unusual history shape a row sits on.
//!
//! PROMPT.md §1.4 lists seven patterns the Mistake Radar looks for. Five of
//! them are a classification of one row, decided once and attached to
//! `graft-graph`'s `Row::marker` as `graph` lays it out: `graph` never
//! computes them itself (it has no access to `ops`, which owns the detection
//! logic), but the type they produce has to live somewhere both `graph` and
//! `ops` can see without either depending on the other (ADR-0007) — here.
//!
//! The other two rows in that table are not a per-row classification and so
//! are not variants of this type: duplicate commits are a bounded, off-thread
//! pass over the visible window that attaches a cross-reference rather than a
//! marker on layout itself (DESIGN.md §3.4), and risky state is a banner over
//! the whole view, not a property of any one row.

/// A row's classification under the Mistake Radar, once `ops` has computed
/// it.
///
/// Always [`None`] on a `Row` `graph` produces: `graph` lays out the graph
/// and nothing more, and never calls into `ops`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MistakeRadarKind {
    /// A merge whose first parent is not the branch that was checked out —
    /// history recorded backward relative to what actually happened.
    BackwardMerge,
    /// A merge that looks like a rebase or fast-forward was intended, but a
    /// real merge commit was made instead.
    FoxtrotMerge,
    /// A merge that only exists because `git pull` was run with a local
    /// branch that had diverged, rather than the update being rebased in.
    AccidentalPullMerge,
    /// A merge whose parents are not actually related the way a merge
    /// implies — the graph equivalent of a merge commit with unrelated
    /// histories forced together.
    EvilMerge,
    /// A remote-tracking ref whose commit no longer appears on the branch it
    /// tracks: evidence of a force-push, though not something that can be
    /// checked with certainty from history alone.
    ForcePushedRemote,
}
