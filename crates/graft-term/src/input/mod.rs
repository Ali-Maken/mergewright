//! The one reader of the terminal's input, for the whole life of the process.
//!
//! Before this module, detection read stdin itself while probing and the
//! event loop read it afterwards, so a key pressed during the probe window
//! was read by the probe and thrown away (KNOWN_GAPS.md, "The probe eats
//! typeahead"). Now [`InputMux`] owns the file descriptor: it decodes every
//! byte once, hands terminal replies to whoever is waiting for one
//! ([`ReplyWaiter`]), and queues everything else as events, in the order the
//! bytes arrived, for whoever reads them — even if that is not until the
//! probe has finished.

mod decode;
mod gate;
mod mux;

pub use mux::{InputMux, Inputs, ReplyWaiter, SEQUENCE_TIMEOUT};
