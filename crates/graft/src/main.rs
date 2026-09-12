//! The Graft binary — ARCHITECTURE.md's `app` layer, and the only crate that
//! may see every other one. It owns the event loop, `AppState`, the pure
//! `update` function that maps an `Event` to a new state plus a list of
//! `Command`s, the job runner that executes those commands off the UI thread,
//! config loading, and the wiring that assembles view models for `ui`.

fn main() {}
