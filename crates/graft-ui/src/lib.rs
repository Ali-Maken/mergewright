//! The screens: widgets, layout, theme and the view models they render from.
//! `ui` draws and nothing else — it holds no repository handle and never
//! imports `git` or `ops`. Every screen renders a plain view model that `app`
//! assembles, and repo-derived strings reach it only as `term::SanitizedText`,
//! so escape-sequence injection is a compile error rather than a review catch.
//! Depends on `graph`, `term` and `core`.
#![forbid(unsafe_code)]

pub mod blame;
pub mod commit_editor;
pub mod confirm;
pub mod conflict_resolver;
pub mod context_menu;
pub mod credential_prompt;
pub mod detail;
pub mod detail_pane;
pub mod diff_words;
pub mod finish_summary;
pub mod fit;
mod frame_box;
pub mod graph_column;
pub mod graph_screen;
pub mod journal_screen;
pub mod map_screen;
pub mod name_prompt;
pub mod overview_strip;
pub mod paint;
pub mod preview_box;
pub mod stash_screen;
pub mod status_screen;
pub mod syntax;
pub mod text_input;
pub mod theme;

#[cfg(test)]
mod test_support;
