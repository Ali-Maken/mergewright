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

mod commit_editor;
pub(crate) mod credential;
mod detail;
mod entries;
pub(crate) mod explain;
pub(crate) mod finish;
pub(crate) mod frame;
mod hit;
mod journal;
mod map;
mod name_prompt;
pub(crate) mod node;
pub(crate) mod picker;
mod preview;
pub(crate) mod radar;
pub(crate) mod resolver;
pub(crate) mod rewrite_editor;
pub(crate) mod rows;
mod stash;
pub(crate) mod status;
pub(crate) mod strip;
pub(crate) mod tour;

pub mod strips;

#[cfg(test)]
pub(crate) mod tests;

pub use commit_editor::commit_screen_view;
pub(crate) use commit_editor::mask::mask_diff;
pub use credential::credential_view;
pub use detail::detail_view;
pub(crate) use entries::uncommitted_shown;
pub(crate) use explain::build_explain_view;
pub use frame::{draws_pixels, tier_in_force, view, view_with_graphics};
pub(crate) use hit::badge_index_at;
pub use journal::journal_view;
pub use map::map_view;
pub use name_prompt::name_prompt_view;
pub use rows::{graph_view, max_lanes};
pub use status::status_view;
pub use strip::strip_marks;

/// `2m ago`, from the same cascade the graph's date column uses, so two
/// views of the same moment never disagree. The one age formatter for the
/// stash list and the journal (m8-polish-and-ship/20): "future" and "now"
/// are whole answers — "now ago" is not English.
pub(crate) fn ago(at_seconds: i64, now_seconds: i64) -> String {
    let relative = graft_ui::graph_screen::columns::relative_date(at_seconds, now_seconds);
    if relative == "future" || relative == "now" {
        relative
    } else {
        format!("{relative} ago")
    }
}

// Only `update::tests` calls these outside `rows` itself.
#[cfg(test)]
pub(crate) use rows::divider_shown;
#[cfg(test)]
pub(crate) use rows::order_notice;
