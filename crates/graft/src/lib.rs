//! The Graft application — ARCHITECTURE.md's `app` layer, and the only crate
//! that may see every other one. It owns the event loop, `AppState`, the pure
//! `update` function that maps an `Event` to a new state plus a list of
//! `Command`s, the job runner that executes those commands off the UI thread,
//! config loading, and the wiring that assembles view models for `ui`.
//!
//! `app` is a library with a thin binary (`main.rs`) on top so that its own
//! tests, and the manual `terminal_probe` example, can drive the same code the
//! binary runs.

pub mod crash;
pub mod terminal;
