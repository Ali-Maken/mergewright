//! The Map view (`M`, DESIGN.md §5.1): one line per local branch, plotted as
//! a density sparkline against a shared time axis (`map-view-screen`,
//! m2-details-and-diffs/19).
//!
//! Draws whatever [`crate::graft::view::map::map_view`] (a later crate)
//! builds from [`crate::graft::map::MapData`] — this module owns no I/O and
//! no state, only the widget.
//!
//! Text glyphs at every tier in M2 (design note, Proposed changes): the
//! pixel tier's real density curve is out of scope here (M8 polish gap).
//! Mouse is also out of scope (M2 gap row) — every interaction here is a
//! key, `update::map::map_key`'s.
//!
//! **Undocumented choices, not fully pinned by the design note:**
//! - The header's day/month wording switches at 60 days (`span_words`):
//!   below it reads `last N days`, at or above `last N months` (rounded to
//!   the nearest month, at least one).
//! - The ASCII tier's tag mark (`┴` in Unicode) draws `+`, mirroring
//!   `graph_column`'s ASCII junction glyph — the design note gives no ASCII
//!   form for it.
//! - When a Radar mark and a Tag mark land in the same bucket (both name
//!   the same bucket index), the later one in `MapRowView::marks` wins the
//!   glyph; unspecified by the design note, and rare (a tag and a finding on
//!   the same commit).

use graft_core::mistake_radar::Severity;
use graft_graph::identity::BranchIdentity;
use graft_term::SanitizedText;

use crate::theme::Theme;

mod render;

/// A mark on one bucket of one row, once sanitised for drawing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapMarkView {
    /// A tag's short name.
    Tag(SanitizedText),
    /// The loudest Mistake Radar finding in the bucket.
    Radar(Severity),
}

/// One row: a branch's density sparkline, ready to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapRowView {
    /// The branch's name, sanitised.
    pub name: SanitizedText,
    /// Lane colour: the same identity the main graph's lanes use.
    pub identity: BranchIdentity,
    /// Commits per bucket; [`MapView::buckets`] long.
    pub counts: Vec<u32>,
    /// `HEAD` is on this branch: prefixed `▸`, suffixed ` ●`.
    pub is_current: bool,
    /// `Some((row, bucket))`: this branch merged into the row at `row`, its
    /// own line ending at `bucket` — where the elbow (`●╮`) draws.
    pub merged_into: Option<(usize, usize)>,
    /// Tag and Radar marks, by bucket.
    pub marks: Vec<(usize, MapMarkView)>,
    /// The tip is in the newest bucket and this row is not `merged_into`
    /// anything: draws ` (active)`. Never set on the current row (it draws
    /// ` ●` instead).
    pub active: bool,
    /// This row's own walk did not hit its cap; `false` colours nothing here
    /// — only [`MapView::complete`] draws the footer's ` · partial`.
    pub complete: bool,
}

/// What the Map view is doing right now, in place of its rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapStatus {
    /// Still walking branches: `done` of `total` have a row so far.
    Loading {
        /// Rows built so far.
        done: u32,
        /// Rows the build expects to visit.
        total: u32,
    },
    /// No local branches at all.
    Empty,
    /// The build failed; the line to show, already sanitised.
    Failed(SanitizedText),
    /// Rows are ready to draw.
    Ready,
}

/// Everything the Map view draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapView {
    /// The branch named in the header: current if there is one, else the
    /// default branch.
    pub title_branch: SanitizedText,
    /// The shared time span's width, in days, for the header's wording.
    pub span_days: u32,
    /// Rows, in the order they draw (current first, then default, then the
    /// rest by tip date).
    pub rows: Vec<MapRowView>,
    /// The largest bucket count over every row: what a bar's glyphs scale
    /// against.
    pub max_count: u32,
    /// Columns every row's bar has.
    pub buckets: usize,
    /// The row under the cursor.
    pub selected: usize,
    /// Local branches beyond what `rows` holds; the footer's `+N more`.
    pub more: usize,
    /// `false` if any row's walk hit its cap; the footer's ` · partial`.
    pub complete: bool,
    /// What to draw instead of, or alongside, the rows.
    pub status: MapStatus,
    /// Draw bars and marks with the ASCII glyph table rather than Unicode
    /// block density (`graph.renderer`, mirroring [`GraphView::ascii`]).
    ///
    /// [`GraphView::ascii`]: crate::graph_screen::GraphView::ascii
    pub ascii: bool,
    /// A write is running and `Esc` stops it before the Map sees the key
    /// (graft's `update::remote::esc_stops_write`): the footer reads `esc
    /// cancels` … `M back`, never `M/esc back` (m8/59 Major A).
    pub esc_cancels: bool,
}

/// The Map view, ready to render.
#[derive(Clone, Copy, Debug)]
pub struct MapScreen<'a> {
    /// What to draw.
    pub view: &'a MapView,
    /// The theme to draw it in.
    pub theme: &'a Theme,
    /// His keymap: the footer's keys are spelled from it.
    pub keys: &'a crate::keymap::Keymap,
}

#[cfg(test)]
mod tests;
