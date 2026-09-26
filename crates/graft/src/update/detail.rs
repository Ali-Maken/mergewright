//! The detail pane's state: what it is showing, the jobs it is waiting on, and
//! the small caches that make stepping back through files and commits free.
//!
//! Pure like the rest of `update`: job ids come from the caller's counter, and
//! every read is a [`Command`](crate::update::events::Command) whose answer
//! arrives as a [`Product`](crate::update::events::Product). An answer whose
//! id is not the one still pending is dropped: the user has moved on.
//!
//! Split into one file per concern (PROMPT.md 6.2, m2-details-and-diffs
//! review M8): [`state`] is the data and its caches; [`ops`] is
//! `impl DetailState`, everything that changes it; [`control`] mints job
//! ids, routes a job's event back in, and owns the Explain panel's
//! open/close; [`keys`] is the pane's own key handling. Every item below is a
//! plain re-export — nothing outside this module should notice the split.

mod control;
mod cursor;
mod keys;
mod landing;
mod ops;
mod state;

/// Ids `update` hands out itself carry this bit; the job runner's own counter
/// never reaches it, so the two can never name the same job.
pub const LOCAL_JOB_BIT: u64 = 1 << 63;

pub(crate) use control::detail_event;
pub use control::{LocalJobs, close_explain, effective_layout, open_explain, tab_hint_rows};
pub use cursor::{DiffCursor, LineRef, Side, diff_cursor};
pub(crate) use keys::{detail_key, enter_opens};
pub use landing::{LandingStage, LineLanding, show_line};
pub(crate) use landing::{LandingWatch, advance};
/// How many diff lines a read carries before it is cut short; the finish
/// summary's diff reads use the same budget (m6-conflicts/19).
pub(crate) use state::MAX_LINES;
pub use state::{
    Bounded, CompareBase, DetailSpec, DetailState, DiffKey, KeptCursor, KeptLine, Loading,
    PaneSize, toggle_compare,
};

#[cfg(test)]
pub(crate) mod tests;
