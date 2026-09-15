//! Real-terminal check of the Kitty image commands Graft sends
//! (m1-the-graph/24d, ADR-0012). Run it *inside* the terminal under test —
//! a pty capture proves nothing about what the terminal does with the bytes.
//!
//! ```text
//! kitty_check <report> <capture>
//! ```
//!
//! Drives `graft_term::kitty::KittyImages` — the encoder the app uses —
//! through the pixel tier's lifecycle, every command at `q=1`:
//!
//! 1. first frame: transmit two strips and place them;
//! 2. scroll: the same uploads re-cropped (no transmit);
//! 3. resize: rows erased one by one (the app's resize clear since
//!    m1-the-graph/25), then a frame that only re-places — no transmit;
//! 4. status screen: `hide` every placement;
//! 5. F3 away from pixels frees every upload, F3 back transmits afresh under
//!    new ids and places (m1-the-graph/33);
//! 6. controls: a missing id must ENOENT; a full `ESC[2J` then placements
//!    only shows whether the clear freed the uploads;
//! 7. exit: `free_all`.
//!
//! After each step a DA1 query (`CSI c`) fences the replies: everything the
//! terminal says about images before the DA1 answer belongs to that step.
//! With `q=1` the terminal answers only failures, so zero replies is a pass —
//! and the DA1 answer proves the terminal was listening. The strips are
//! synthetic lane stripes, not a rasterised history: this checks the
//! protocol, not the picture (the user judges that by eye).
//!
//! Writes a plain-text report to `<report>`, every byte sent to `<capture>`,
//! and a decoded, human-readable summary of those bytes to `<capture>.txt`.
//!
//! Split in two (m1-the-graph/79, PROMPT.md 6.2): `driver` builds the
//! command bytes and drives the check's steps; `terminal` sends them, reads
//! and fences the terminal's replies, and decodes a capture for humans.

mod driver;
mod terminal;

use std::io;

fn main() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    let (Some(report_path), Some(capture_path)) = (args.next(), args.next()) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: kitty_check <report> <capture>",
        ));
    };

    crossterm::terminal::enable_raw_mode()?;
    let outcome = driver::check();
    let restored = crossterm::terminal::disable_raw_mode();
    let (report, capture) = outcome?;
    restored?;

    std::fs::write(&report_path, &report)?;
    std::fs::write(&capture_path, &capture)?;
    std::fs::write(format!("{capture_path}.txt"), terminal::decode(&capture))?;
    Ok(())
}
