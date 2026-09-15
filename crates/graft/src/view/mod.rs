//! The end of the loop: `view(state)` (ARCHITECTURE.md).
//!
//! One pure function from what Graft knows to a frame. It reads and draws; it
//! never changes anything, so what is on screen is always a direct function of
//! the state.
//!
//! This is also the boundary the sanitiser guards. Everything here that came
//! out of a repository — a path, a branch name, a failure git wrote — crosses
//! into `ui` as [`SanitizedText`], which only `term` can construct, so a branch
//! called `ma<ESC>[31min` reaches the terminal as the picture of an escape and
//! never as the escape (DESIGN.md §8.2).
//!
//! Split along its concerns: [`frame`] picks the tier and screen, [`rows`]
//! builds the graph view model, [`entries`] builds its rows, and [`status`]
//! builds the status screen's view model.

mod entries;
mod frame;
mod rows;
mod status;

pub mod strips;

#[cfg(test)]
pub(crate) mod tests;

pub(crate) use entries::uncommitted_shown;
pub use frame::{draws_pixels, tier_in_force, view, view_with_graphics};
pub use rows::graph_view;
pub use status::status_view;

// Only `update::tests` calls this outside `rows` itself.
#[cfg(test)]
pub(crate) use rows::order_notice;
