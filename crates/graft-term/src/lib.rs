//! The terminal's capabilities and byte-level protocols. Today: detecting
//! what the host emulator supports and choosing a rendering tier
//! (`detect`), and sanitising repo-derived strings into `SanitizedText` so a
//! malicious repository cannot inject escape sequences into the screen
//! (`sanitize`). The image encoder that turns a `core::RasterFrame` into
//! Kitty, iTerm2 or Sixel bytes belongs here too (DESIGN.md §8.2) and lands
//! in M1. Depends only on `core`.

pub mod detect;
pub mod sanitize;

pub use sanitize::{SanitizedText, sanitize, sanitize_line, sanitize_path, sanitize_path_bytes};
