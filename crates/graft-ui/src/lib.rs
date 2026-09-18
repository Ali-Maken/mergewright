//! The screens: widgets, layout, theme and the view models they render from.
//! `ui` draws and nothing else — it holds no repository handle and never
//! imports `git` or `ops`. Every screen renders a plain view model that `app`
//! assembles, and repo-derived strings reach it only as `term::SanitizedText`,
//! so escape-sequence injection is a compile error rather than a review catch.
//! Depends on `graph`, `term` and `core`.
#![forbid(unsafe_code)]

pub mod context_menu;
pub mod detail;
pub mod detail_pane;
pub mod diff_words;
pub mod fit;
pub mod graph_column;
pub mod graph_screen;
pub mod map_screen;
pub mod paint;
pub mod status_screen;
pub mod syntax;
pub mod theme;

#[cfg(test)]
mod test_support;
