//! Everything Graft reads from, and runs against, a repository: rev-walks,
//! refs, objects and status through `gix` (with `git2` backing single named
//! gaps only, ADR-0011), the typed runner for the system `git` binary, and the
//! parsers for its porcelain and diff output. Commands are built from typed
//! arguments, never by joining shell strings. Depends only on `core`; it never
//! touches the terminal or the UI.
#![forbid(unsafe_code)]

pub mod branches;
pub mod commit_graph;
pub mod contained;
#[cfg(test)]
mod fixture;
pub mod history;
pub mod mailmap;
pub mod repo;
pub mod run;
pub mod status;
pub mod version;
