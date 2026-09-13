//! Asking again mid-session: the two re-queries of DESIGN.md §4.5.
//!
//! The tier is decided once and never revisited here. What does change under
//! a running Graft is the size of a cell (a font-size change, a window dragged
//! to a display with a different scale) and the terminal's background (a
//! theme switched while Graft sat in the background). Each gets one fenced
//! query of its own, never the full startup round.
//!
//! Both functions take the value currently in force and never make it worse
//! than it was: silence keeps the last theme as it was, and keeps the last
//! cell size's numbers while saying they are no longer measured. A size that
//! was right before a resize is a far better guess than the startup default,
//! but it is still a guess, and the diagnostics view must say so.

use std::io;
use std::time::{Duration, Instant};

use graft_core::capabilities::{CellSize, CellSizeSource, ThemeHint};

use super::{TerminalHost, measured_cell_size, probe, reported_theme};

/// Re-ask the cell size after a resize: `CSI 16 t`, then `TIOCGWINSZ`, then
/// `previous`'s numbers marked [`CellSizeSource::Assumed`].
///
/// # Errors
///
/// Only when the query cannot be written or the reply cannot be read — a
/// terminal Graft can no longer talk to. Silence is not an error.
pub fn remeasure_cell_size(
    host: &mut dyn TerminalHost,
    previous: CellSize,
    budget: Duration,
) -> io::Result<CellSize> {
    let replies = ask(host, &probe::cell_size_round(), budget)?;
    Ok(measured_cell_size(&replies, host).unwrap_or(CellSize {
        source: CellSizeSource::Assumed,
        ..previous
    }))
}

/// Re-ask the background on focus regain (`OSC 11`). A terminal that says
/// nothing leaves `previous` exactly as it was: silence now says nothing about
/// a background that was reported before.
///
/// # Errors
///
/// As [`remeasure_cell_size`]: only a terminal that cannot be written to or
/// read from.
pub fn requery_theme(
    host: &mut dyn TerminalHost,
    previous: ThemeHint,
    budget: Duration,
) -> io::Result<ThemeHint> {
    let replies = ask(host, &probe::background_round(), budget)?;
    Ok(replies.background.map_or(previous, reported_theme))
}

/// Send one fenced round and collect its replies.
///
/// Anything already waiting is thrown away first: it can only be a reply to
/// an earlier round that came back after that round's deadline, and read now
/// it would be taken for this round's answer — its fence ending the read
/// before the real reply arrives.
fn ask(
    host: &mut dyn TerminalHost,
    queries: &[u8],
    budget: Duration,
) -> io::Result<probe::ProbeReplies> {
    let mut stale = [0u8; 512];
    while host.read(&mut stale, Instant::now())? > 0 {}
    probe::run(host, queries, Instant::now() + budget)
}
