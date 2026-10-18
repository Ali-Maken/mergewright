//! Working out what the terminal can do, once, before the first frame.
//!
//! [`detect`] implements DESIGN.md §4.1–§4.4: read the cheap environment
//! hints, send one batch of active queries with a deadline, fall back
//! *per query* on silence, handle tmux, and pick a rendering tier. It returns
//! what it found and nothing else — config's `graph.renderer` override is
//! combined with this in the app layer, so that "what the terminal said" and
//! "what the user asked for" never blur into one value.
//!
//! Two rules run through all of it, both paid for in the graph spike
//! (ADR-0012):
//!
//! - **Environment hints are never trusted alone** (PROMPT.md §1.1). `$TERM`
//!   says what terminfo entry to use, not what the terminal in front of the
//!   user is: it survives `ssh`, `tmux`, `sudo` and a shell config copied
//!   between machines. Every capability that can be asked about is asked
//!   about. iTerm2 inline images are the single exception, because iTerm2 has
//!   no query for them.
//! - **A fallback value is marked as one.** A cell size nobody measured and a
//!   theme nobody reported are still returned — Mergewright has to draw something —
//!   but they carry their provenance in the type, so the diagnostics view can
//!   show `8×16 (assumed)` rather than a number that looks measured.
//!
//! The caller owns raw mode: [`detect`] writes queries and reads replies on a
//! terminal that is already raw; replies come in through the input mux, so
//! keys typed meanwhile still reach the event loop.
//!
//! Mid-session, [`remeasure_cell_size`] and [`requery_theme`] re-ask the two
//! answers that can change under a running Mergewright (DESIGN.md §4.5). The tier
//! is never re-decided.

mod environment;
mod fallback;
mod probe;
mod redetect;
pub(crate) mod reply;
mod tier;
mod tmux;
mod tty;

use std::io;
use std::time::{Duration, Instant};

use mergewright_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ProbeOutcome, RenderingTier, Supported, Theme,
    ThemeHint, ThemeSource, TmuxStatus,
};

pub use environment::Environment;
use fallback::{answered, cell_size, theme};
pub use redetect::{remeasure_cell_size, requery_theme};
use tier::{Graphics, tier};
use tmux::{TmuxProbe, through_tmux};
pub use tty::TtyHost;

/// How long the terminal gets to answer locally (DESIGN.md §4.1).
pub const LOCAL_PROBE_BUDGET: Duration = Duration::from_millis(200);

/// How long it gets over SSH — one extra network round trip (DESIGN.md §4.4).
pub const SSH_PROBE_BUDGET: Duration = Duration::from_millis(500);

/// The cell size used when nothing reports one. Roughly a small terminal font,
/// and always returned as [`CellSizeSource::Assumed`].
pub const ASSUMED_CELL_SIZE: (u32, u32) = (8, 16);

/// The terminal's own pixel dimensions, as the kernel reports them
/// (`TIOCGWINSZ`). The second link in the cell-size fallback chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowPixels {
    /// The grid's width in character cells.
    pub columns: u16,
    /// The grid's height in character cells.
    pub rows: u16,
    /// The window's width in pixels. Zero on the many terminals that leave
    /// this field alone.
    pub width: u16,
    /// The window's height in pixels. Zero as often as `width` is.
    pub height: u16,
}

impl WindowPixels {
    /// Pixels per cell, or `None` when the terminal left the pixel fields at
    /// zero — which many do, and which is not an error.
    fn cell_size(self) -> Option<(u32, u32)> {
        if self.columns == 0 || self.rows == 0 {
            return None;
        }
        let width = u32::from(self.width / self.columns);
        let height = u32::from(self.height / self.rows);
        // Zero pixel fields, or a window reporting fewer pixels than it has
        // cells — both leave nothing to divide a cell by, and a zero-pixel
        // cell would be a division by zero waiting in the pixel renderer.
        if width == 0 || height == 0 {
            return None;
        }
        Some((width, height))
    }
}

/// Everything detection needs that it cannot compute: the terminal it talks
/// to, and the two syscalls around it.
///
/// One seam, not three, because there is one thing behind it — the terminal
/// this process is attached to — and a test that has to stitch three fakes
/// together to describe one terminal is describing the seam wrong.
/// [`TtyHost`] is the real implementation; tests script a fake.
pub trait TerminalHost {
    /// Write a query batch and flush it. The terminal cannot answer what it
    /// has not been sent, so an unflushed write is a guaranteed timeout.
    fn write_probe(&mut self, bytes: &[u8]) -> io::Result<()>;

    /// Read whatever has arrived, waiting no longer than `deadline`.
    ///
    /// Returns `Ok(0)` when the deadline passes or the terminal closes: both
    /// mean "nothing more is coming", which is all the caller does with it.
    fn read(&mut self, buffer: &mut [u8], deadline: Instant) -> io::Result<usize>;

    /// The kernel's window size. `None` when there is no window size to be
    /// had — not an error: it is one rung of the fallback chain, and the rung
    /// below it always answers.
    fn window_pixels(&mut self) -> Option<WindowPixels>;

    /// What `tmux -V` prints, when Mergewright is inside tmux and tmux can be run.
    /// `None` becomes [`TmuxStatus::VersionUnknown`], which is treated as too
    /// old rather than hoped about.
    fn tmux_version(&mut self) -> Option<String>;
}

/// Work out what this terminal can do.
///
/// Runs one probe round (two inside tmux), each bounded by the deadline in
/// DESIGN.md §4.4, and always returns a usable [`Capabilities`]: a terminal
/// that answers nothing gets fallbacks that say they are fallbacks, and a
/// terminal that cannot be reached at all is reported in
/// [`Capabilities::probe`] rather than quietly becoming an ASCII session
/// nobody can explain.
///
/// The caller must have raw mode on, and must not be reading the terminal
/// from anywhere else while this runs.
pub fn detect(environment: &Environment, host: &mut dyn TerminalHost) -> Capabilities {
    if !environment.is_probeable() {
        return unprobed(environment);
    }

    let deadline = Instant::now() + environment.probe_budget();
    let (replies, first_round) = match probe::run(host, &probe::full_round(), deadline) {
        Ok(replies) if replies.fenced() => (replies, ProbeOutcome::Answered),
        Ok(replies) => (replies, ProbeOutcome::TimedOut),
        // Detection carries on with fallbacks: the user still gets a graph,
        // and the failure is on the struct for the diagnostics view to show.
        Err(error) => (
            probe::ProbeReplies::default(),
            ProbeOutcome::Failed(error.to_string()),
        ),
    };

    let tmux_probe = match environment.tmux {
        None => TmuxProbe {
            status: TmuxStatus::Outside,
            graphics: Graphics::from(&replies),
            failure: None,
        },
        Some(_) => through_tmux(environment, host),
    };
    let TmuxProbe {
        status: tmux,
        graphics,
        failure,
    } = tmux_probe;
    // What the diagnostics view reports about the probe, folding both rounds
    // into one answer. A failure in either is a fact about this terminal and
    // outranks everything, so it is never dropped. Otherwise a fenced tmux
    // re-probe counts as an answer even when round one timed out: that round
    // is the one that decided the tier, and the screen must not read "pixel,
    // assumed (terminal silent)" about a tier the outer terminal really did
    // ack (M0 review round 2, Minor 2).
    let outcome = if let Some(error) = failure {
        ProbeOutcome::Failed(error)
    } else if matches!(first_round, ProbeOutcome::Failed(_))
        || !matches!(tmux, TmuxStatus::Passthrough { .. })
    {
        first_round.clone()
    } else {
        ProbeOutcome::Answered
    };

    let iterm2_images = environment.is_iterm2() && graphics.reaches_the_terminal;
    let (tier, pixel_protocol) = tier(environment, &graphics, iterm2_images);

    Capabilities {
        tier,
        pixel_protocol,
        kitty_graphics: graphics.kitty,
        kitty_deflate: graphics.kitty_deflate,
        answers_dsr: replies.device_status,
        sixel: graphics.sixel,
        iterm2_images,
        // Judged against the *first* round, not the merged outcome. These two
        // queries are sent once, in round one; the tmux re-probe re-asks only
        // the graphics queries, so its failure says nothing about them. A
        // fenced first round means silence here really was a "no", and a
        // second-round failure must not turn that back into "we never heard".
        kitty_keyboard: answered(&first_round, replies.kitty_keyboard.then_some(true)),
        synchronized_output: answered(&first_round, replies.synchronized_output),
        cell_size: cell_size(&replies, host),
        theme: theme(replies.background),
        color_depth: environment.color_depth(),
        no_color: environment.no_color,
        is_tty: environment.stdout_is_tty,
        over_ssh: environment.over_ssh,
        tmux,
        probe: outcome,
        probe_budget: environment.probe_budget(),
    }
}

/// A yes/no query's answer, keeping "nothing came back" distinct from "no".
fn supported(answer: Option<bool>) -> Supported {
    match answer {
        Some(true) => Supported::Yes,
        Some(false) => Supported::No,
        None => Supported::Silent,
    }
}

/// The capabilities of a terminal Mergewright never asked: output that is not a
/// terminal, or `TERM=dumb`. Both are ASCII by DESIGN.md §4.2, and writing
/// probe bytes into either would only corrupt what is on the other end.
fn unprobed(environment: &Environment) -> Capabilities {
    Capabilities {
        tier: RenderingTier::Ascii,
        pixel_protocol: None,
        kitty_graphics: Supported::Silent,
        kitty_deflate: false,
        answers_dsr: false,
        sixel: Supported::Silent,
        iterm2_images: false,
        kitty_keyboard: Supported::Silent,
        synchronized_output: Supported::Silent,
        cell_size: CellSize {
            width: ASSUMED_CELL_SIZE.0,
            height: ASSUMED_CELL_SIZE.1,
            source: CellSizeSource::Assumed,
        },
        theme: ThemeHint {
            theme: Theme::Dark,
            source: ThemeSource::Assumed,
        },
        color_depth: environment.color_depth(),
        no_color: environment.no_color,
        is_tty: environment.stdout_is_tty,
        over_ssh: environment.over_ssh,
        // Inside tmux with the output piped, the version was never asked for.
        // "Outside" would be a plain untruth, and this struct is what the
        // diagnostics view shows.
        tmux: match environment.tmux {
            Some(_) => TmuxStatus::VersionUnknown,
            None => TmuxStatus::Outside,
        },
        probe: ProbeOutcome::Skipped,
        probe_budget: environment.probe_budget(),
    }
}

#[cfg(test)]
mod tests;
