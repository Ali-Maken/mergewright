//! One probe round: the queries Graft sends, and the read that fences them.
//!
//! All six queries go out in a single write (DESIGN.md §4.1 step 2), so the
//! terminal pays one round trip rather than six. `DA1` goes last and acts as
//! the fence: every terminal answers it, so its reply is the signal that
//! everything else which was ever going to arrive has arrived. That is what
//! turns silence into a *fact* — "this terminal does not implement that query"
//! — instead of "the deadline beat it" (ADR-0012).

use std::io;
use std::time::Instant;

use graft_core::capabilities::Rgb;

use super::TerminalHost;
use super::reply::{self, replies};

/// A dummy 1×1 RGB image the terminal is asked about but never asked to draw.
/// Only Kitty-protocol terminals answer at all.
const KITTY_GRAPHICS_QUERY: &[u8] = b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\";
/// The graphics query's id; its reply is matched by it.
pub(super) const KITTY_GRAPHICS_ID: u32 = 31;
/// A dummy 1×1 RGBA image sent zlib-deflated (`o=z`): one transparent pixel,
/// deflated then base64. `OK` means the terminal inflates `o=z` uploads.
pub(super) const KITTY_DEFLATE_QUERY: &[u8] =
    b"\x1b_Gi=32,s=1,v=1,a=q,t=d,f=32,o=z;eAFjYGBgAAAABAAB\x1b\\";
/// The deflate query's id, distinct from every other probe and image id.
pub(super) const KITTY_DEFLATE_ID: u32 = 32;
/// Kitty keyboard protocol: report the current enhancement flags.
const KITTY_KEYBOARD_QUERY: &[u8] = b"\x1b[?u";
/// Report the cell size in pixels.
const CELL_SIZE_QUERY: &[u8] = b"\x1b[16t";
/// `DECRQM` for DEC mode 2026, synchronized output.
const SYNCHRONIZED_OUTPUT_QUERY: &[u8] = b"\x1b[?2026$p";
/// Report the background colour.
const BACKGROUND_QUERY: &[u8] = b"\x1b]11;?\x1b\\";
/// Primary device attributes — the fence, and the Sixel answer.
const DEVICE_ATTRIBUTES_QUERY: &[u8] = b"\x1b[c";

/// Everything one round heard back. A field left at its default means the
/// query went unanswered; whether that is meaningful depends on whether the
/// round was [`ProbeReplies::fenced`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProbeReplies {
    pub kitty_graphics: bool,
    /// The deflate query came back `OK`. An error reply leaves it false.
    pub kitty_deflate: bool,
    /// The `DA1` parameters. `Some` exactly when the fence came back.
    pub device_attributes: Option<Vec<u16>>,
    /// `(width, height)` in pixels.
    pub cell_size: Option<(u32, u32)>,
    pub kitty_keyboard: bool,
    pub synchronized_output: Option<bool>,
    pub background: Option<Rgb>,
}

impl ProbeReplies {
    /// The `DA1` fence came back, so silence elsewhere is an answer.
    pub(crate) fn fenced(&self) -> bool {
        self.device_attributes.is_some()
    }

    /// Sixel, as `DA1` reports it: parameter 4.
    pub(crate) fn sixel(&self) -> Option<bool> {
        Some(self.device_attributes.as_ref()?.contains(&4))
    }
}

/// The full query batch of DESIGN.md §4.1 step 2, fenced by `DA1`.
pub(crate) fn full_round() -> Vec<u8> {
    [
        KITTY_GRAPHICS_QUERY,
        KITTY_DEFLATE_QUERY,
        KITTY_KEYBOARD_QUERY,
        CELL_SIZE_QUERY,
        SYNCHRONIZED_OUTPUT_QUERY,
        BACKGROUND_QUERY,
        DEVICE_ATTRIBUTES_QUERY,
    ]
    .concat()
}

/// The graphics half of the batch, for the tmux passthrough re-probe: only
/// image support depends on reaching the *outer* terminal. Cell size, theme
/// and keyboard were already answered by tmux itself, which is the layer that
/// actually serves them (DESIGN.md §4.3).
pub(crate) fn graphics_round() -> Vec<u8> {
    [
        KITTY_GRAPHICS_QUERY,
        KITTY_DEFLATE_QUERY,
        DEVICE_ATTRIBUTES_QUERY,
    ]
    .concat()
}

/// The cell-size query alone, fenced: what a resize re-asks (DESIGN.md §4.5).
pub(crate) fn cell_size_round() -> Vec<u8> {
    [CELL_SIZE_QUERY, DEVICE_ATTRIBUTES_QUERY].concat()
}

/// The background query alone, fenced: what focus regain re-asks
/// (DESIGN.md §4.5).
pub(crate) fn background_round() -> Vec<u8> {
    [BACKGROUND_QUERY, DEVICE_ATTRIBUTES_QUERY].concat()
}

/// Wrap a query batch in tmux's passthrough sequence so the outer terminal
/// sees it (see [`crate::tmux`]).
pub(crate) fn tmux_passthrough(payload: &[u8]) -> Vec<u8> {
    let mut wrapped = Vec::new();
    crate::tmux::tmux_passthrough(payload, &mut wrapped);
    wrapped
}

/// Send `queries` and read replies until the `DA1` fence arrives or `deadline`
/// passes.
///
/// Errors are returned rather than absorbed: a terminal Graft cannot write to
/// or read from is a fact the diagnostics view shows, not a silent downgrade
/// to ASCII that nobody can explain.
pub(crate) fn run(
    host: &mut dyn TerminalHost,
    queries: &[u8],
    deadline: Instant,
) -> io::Result<ProbeReplies> {
    host.write_probe(queries)?;

    let mut buffer = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        let parsed = parse(&buffer);
        if parsed.fenced() {
            return Ok(parsed);
        }
        // A read that returns nothing means the deadline passed or the
        // terminal closed: either way no more replies are coming.
        match host.read(&mut chunk, deadline)? {
            0 => return Ok(parsed),
            read => buffer.extend_from_slice(&chunk[..read]),
        }
    }
}

/// Pick the replies Graft asked for out of everything the terminal sent.
fn parse(buffer: &[u8]) -> ProbeReplies {
    let mut found = ProbeReplies::default();
    for item in replies(buffer) {
        // Each query is recognised by its own reply shape, so one pass over
        // the buffer fills every field. Anything unrecognised — typeahead, a
        // reply to a query Graft never sent — falls through untouched.
        if let Some(ok) = reply::kitty_graphics_reply(item, KITTY_DEFLATE_ID) {
            found.kitty_deflate = ok;
        } else if reply::kitty_graphics_reply(item, KITTY_GRAPHICS_ID).is_some() {
            found.kitty_graphics = true;
        } else if reply::kitty_keyboard(item) {
            found.kitty_keyboard = true;
        } else if let Some(parameters) = reply::device_attributes(item) {
            found.device_attributes = Some(parameters);
        } else if let Some(size) = reply::cell_size(item) {
            found.cell_size = Some(size);
        } else if let Some(state) = reply::synchronized_output(item) {
            found.synchronized_output = Some(state);
        } else if let Some(color) = reply::background_color(item) {
            found.background = Some(color);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fence_is_the_last_query_sent() {
        let round = full_round();
        assert!(round.ends_with(DEVICE_ATTRIBUTES_QUERY));
        assert!(round.starts_with(KITTY_GRAPHICS_QUERY));
    }

    #[test]
    fn a_buffer_with_no_fence_is_not_fenced() {
        let parsed = parse(b"\x1b[6;37;17t");
        assert!(!parsed.fenced());
        assert_eq!(parsed.cell_size, Some((17, 37)));
        assert_eq!(parsed.sixel(), None);
    }

    #[test]
    fn sixel_comes_from_device_attributes_parameter_four() {
        assert_eq!(parse(b"\x1b[?63;4;22c").sixel(), Some(true));
        assert_eq!(parse(b"\x1b[?62;22c").sixel(), Some(false));
    }
}
