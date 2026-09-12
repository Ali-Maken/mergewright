//! The terminal's capabilities and byte-level protocols: detecting what the
//! host emulator supports, choosing a rendering tier, encoding a `RasterFrame`
//! into the Kitty, iTerm2 or Sixel image protocols, and sanitising
//! repo-derived strings into `SanitizedText` so a malicious repository cannot
//! inject escape sequences into the screen. Depends only on `core`.

pub mod detect;
