//! The detail pane's regions, breakpoints and states.
//!
//! Draws a [`DetailView`](crate::detail::DetailView) (`crate::detail`,
//! view-model only) beside, below or in place of the graph, per DESIGN.md
//! §1.1-1.3's breakpoints. [`diff_body`] (`diff-widget`,
//! m2-details-and-diffs/08) is the real unified diff renderer: gutter,
//! soft/word backgrounds, syntax fg, a pinned hunk heading, `␍`, and a
//! truncation footer.
//!
//! Deviations from the ticket's literal signatures, both reported as spec
//! gaps rather than silently worked around:
//! - [`DetailPane`] also carries `theme` (07's gap): every other widget in
//!   this crate is handed a `&Theme` the same way, and there is no other
//!   colour source at render time.
//! - [`diff_body`] takes a `truncated: Option<(usize, usize)>` the ticket's
//!   signature didn't list: the truncation footer it must draw is
//!   [`crate::detail::DiffView`]'s field, not the hunks', and there is no
//!   other way to hand it over under the same argument list.
//!
//! Split into one file per region (PROMPT.md 6.2, m2-details-and-diffs
//! review M8): [`layout`] is the pane's own geometry; [`line_render`] is the
//! row-drawing helpers [`unified`] and [`side_by_side`] both build on, which
//! is what keeps the two diff renderers matching on the same fixture by
//! construction; [`header`], [`files`] and [`body`] are the three regions
//! [`DetailPane`] composes. Every item below is a plain re-export — nothing
//! outside this crate should notice the file moved.

mod body;
mod files;
mod header;
mod layout;
pub(crate) mod line_render;
mod side_by_side;
mod unified;

pub use body::DetailPane;
pub(crate) use files::status_letter;
pub use layout::{DetailAreas, DetailLayout, detail_areas, split_area};
pub use line_render::DiffFocus;
pub use side_by_side::diff_body_side_by_side;
pub use unified::diff_body;

#[cfg(test)]
pub(crate) mod tests;
