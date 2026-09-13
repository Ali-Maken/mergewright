//! The terminal's capabilities and byte-level protocols. Today: detecting
//! what the host emulator supports and choosing a rendering tier
//! (`detect`), and sanitising repo-derived strings into `SanitizedText` so a
//! malicious repository cannot inject escape sequences into the screen
//! (`sanitize`), and owning stdin: decoding keys and routing terminal
//! replies to whoever asked (`input`). Image encoders turn a `core::RasterFrame` into
//! protocol bytes (DESIGN.md §8.2): Kitty today (`kitty`); iTerm2 and Sixel
//! follow. Depends only on `core`.
// `deny`, not `forbid`, and this is the only crate where that is true.
// Reading the terminal's window size and waiting on its file descriptor are
// two libc calls with no safe equivalent in the standard library; ADR-0013
// records what they do, why they are sound, and what would break them. Both
// sites carry a narrowly-scoped `#[allow(unsafe_code, reason = "...")]`, and
// `forbid` would make those allowances a compile error rather than a
// documented exception. Nothing else in the workspace may use `unsafe` at
// all.
#![deny(unsafe_code)]

pub mod detect;
pub mod input;
pub mod kitty;
pub mod sanitize;

pub use sanitize::{SanitizedText, sanitize, sanitize_line, sanitize_path, sanitize_path_bytes};
