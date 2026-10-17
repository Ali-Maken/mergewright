//! Reading history for the screen: the walk, laid out as it arrives.
//!
//! The walk lives in `graft-git` and the layout engine in `graft-graph`, and
//! neither may see the other (ARCHITECTURE.md). This is where they meet:
//! [`WalkSource`] puts a `Walk` behind the layout's `CommitSource` seam,
//! [`seeds_for`] turns ADR-0001's branch resolution into reserved lanes, and
//! [`stream`] drives a `StreamingLayout` for as long as the screen shows it,
//! answering each [`RowRequest`] with just the rows it names — laying out
//! further only when a request reaches past the frontier. The screen holds one
//! [`Window`] of rows, never the whole history (DESIGN.md §3.6).
//!
//! Nothing here knows about jobs or threads beyond a `stop` check, a
//! request channel and a `report` callback; `crate::jobs` supplies both.
//!
//! Split along its concerns: [`state`] is what a walk's history holds and
//! reports, [`stream`] opens a walk, [`views`] holds the layouts it is shown
//! through (and promotes them), [`serve`](mod@serve) answers requests against them,
//! and [`scroll`] moves [`History`] on screen.

mod fold;
pub mod parents;
mod scroll;
mod seeds;
mod serve;
mod source;
mod state;
mod stream;
mod views;

#[cfg(test)]
mod idle_tests;
#[cfg(test)]
mod promotion_tests;
#[cfg(test)]
mod tests;

pub use fold::{Inside, RowFold};
pub use scroll::Motion;
pub use seeds::{preview_seeds, seeds_for};
pub use serve::serve;
pub use source::WalkSource;
pub use state::{
    Ended, History, HistoryUpdate, Phase, Promote, Promotion, RowRequest, View, WalkFacts, Wanted,
    Window,
};
pub use stream::{stream, topological_order};
pub use views::{Open, Views};

#[cfg(test)]
use serve::answer;
