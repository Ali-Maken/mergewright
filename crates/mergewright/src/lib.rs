//! The Mergewright application — ARCHITECTURE.md's `app` layer, and the only crate
//! that may see every other one. It owns the event loop, `AppState`, the pure
//! `update` function that maps an `Event` to a new state plus a list of
//! `Command`s, the job runner that executes those commands off the UI thread,
//! config loading, and the wiring that assembles view models for `ui`.
//!
//! `app` is a library with a thin binary (`main.rs`) on top so that its own
//! tests, and the manual `terminal_probe` example, can drive the same code the
//! binary runs.
#![forbid(unsafe_code)]

pub mod cli;
pub mod config;
pub mod crash;
pub(crate) mod empty_state;
pub mod event_loop;
pub(crate) mod first_launch;
pub mod history;
pub mod jobs;
pub mod layout;
pub mod map;
pub(crate) mod recent;
pub mod refs;
pub mod repos;
pub mod repository;
pub mod search;
pub mod session;
#[cfg(test)]
pub(crate) mod spelled;
pub mod terminal;
pub(crate) mod trace;
pub mod update;
pub mod view;
pub mod working_tree;
