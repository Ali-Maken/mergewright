//! The screens: widgets, layout, theme and the view models they render from.
//! `ui` draws and nothing else — it holds no repository handle and never
//! imports `git` or `ops`. Every screen renders a plain view model that `app`
//! assembles, and repo-derived strings reach it only as `term::SanitizedText`,
//! so escape-sequence injection is a compile error rather than a review catch.
//! Depends on `graph`, `term` and `core`.

pub mod theme;
