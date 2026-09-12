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
//!   theme nobody reported are still returned — Graft has to draw something —
//!   but they carry their provenance in the type, so the diagnostics view can
//!   show `8×16 (assumed)` rather than a number that looks measured.
//!
//! The caller owns raw mode: [`detect`] writes queries and reads replies on a
//! terminal that is already raw and not yet owned by the event loop. In M0
//! that means startup-only, and keystrokes typed during the probe window are
//! consumed by the reply reader rather than delivered — the known typeahead
//! gap ADR-0012 records, which M1's input mux closes.

mod probe;
mod reply;
mod tty;

use std::env;
use std::io::{self, IsTerminal};
use std::time::{Duration, Instant};

use graft_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, PixelProtocol, ProbeOutcome, RenderingTier,
    Rgb, Supported, Theme, ThemeHint, ThemeSource, TmuxStatus,
};

pub use tty::TtyHost;

/// How long the terminal gets to answer locally (DESIGN.md §4.1).
pub const LOCAL_PROBE_BUDGET: Duration = Duration::from_millis(200);

/// How long it gets over SSH — one extra network round trip (DESIGN.md §4.4).
pub const SSH_PROBE_BUDGET: Duration = Duration::from_millis(500);

/// The cell size used when nothing reports one. Roughly a small terminal font,
/// and always returned as [`CellSizeSource::Assumed`].
pub const ASSUMED_CELL_SIZE: (u32, u32) = (8, 16);

/// The first tmux that can forward escape sequences to the outer terminal at
/// all (DESIGN.md §4.3). Older versions get the Unicode tier, deliberately.
const MIN_PASSTHROUGH_TMUX: (u16, u16) = (3, 3);

/// The first iTerm2 whose inline-image support Graft relies on.
const MIN_ITERM2_MAJOR: u16 = 3;

/// The terminal's own pixel dimensions, as the kernel reports them
/// (`TIOCGWINSZ`). The second link in the cell-size fallback chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowPixels {
    pub columns: u16,
    pub rows: u16,
    pub width: u16,
    pub height: u16,
}

impl WindowPixels {
    /// Pixels per cell, or `None` when the terminal left the pixel fields at
    /// zero — which many do, and which is not an error.
    fn cell_size(self) -> Option<(u32, u32)> {
        if self.columns == 0 || self.rows == 0 || self.width == 0 || self.height == 0 {
            return None;
        }
        Some((
            u32::from(self.width / self.columns),
            u32::from(self.height / self.rows),
        ))
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

    /// What `tmux -V` prints, when Graft is inside tmux and tmux can be run.
    /// `None` becomes [`TmuxStatus::VersionUnknown`], which is treated as too
    /// old rather than hoped about.
    fn tmux_version(&mut self) -> Option<String>;
}

/// The cheap hints, read once (DESIGN.md §4.1 step 1).
///
/// A plain value rather than direct `env::var` calls inside detection: it
/// keeps the whole decision testable without touching process environment —
/// which in Rust 2024 is `unsafe` and, in a test binary with threads, racy.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Environment {
    pub term: Option<String>,
    pub term_program: Option<String>,
    pub term_program_version: Option<String>,
    pub colorterm: Option<String>,
    pub kitty_window_id: Option<String>,
    /// `$TMUX`. Its value is tmux's socket path; only its presence matters.
    pub tmux: Option<String>,
    /// `$SSH_TTY` or `$SSH_CONNECTION` is set.
    pub over_ssh: bool,
    /// `$NO_COLOR` is set to a non-empty value.
    pub no_color: bool,
    /// Graft's own output is a terminal. Not an environment variable, but the
    /// same kind of fact: cheap, read once, and decisive.
    pub stdout_is_tty: bool,
}

impl Environment {
    /// Read the hints from this process.
    pub fn from_process() -> Self {
        Self {
            term: var("TERM"),
            term_program: var("TERM_PROGRAM"),
            term_program_version: var("TERM_PROGRAM_VERSION"),
            colorterm: var("COLORTERM"),
            kitty_window_id: var("KITTY_WINDOW_ID"),
            tmux: var("TMUX"),
            over_ssh: var("SSH_TTY").is_some() || var("SSH_CONNECTION").is_some(),
            no_color: var("NO_COLOR").is_some(),
            stdout_is_tty: io::stdout().is_terminal(),
        }
    }

    /// How long the terminal gets to answer (DESIGN.md §4.4).
    fn probe_budget(&self) -> Duration {
        if self.over_ssh {
            SSH_PROBE_BUDGET
        } else {
            LOCAL_PROBE_BUDGET
        }
    }

    /// A terminal worth asking questions. `TERM=dumb` answers nothing by
    /// definition, and a pipe would just be sent escape sequences that end up
    /// in the user's file.
    fn is_probeable(&self) -> bool {
        self.stdout_is_tty && !self.is_dumb()
    }

    fn is_dumb(&self) -> bool {
        self.term.as_deref() == Some("dumb")
    }

    /// The iTerm2 hint: the one capability allowed to rest on environment
    /// alone, because iTerm2's inline images have no query (DESIGN.md §4.1
    /// step 4).
    fn is_iterm2(&self) -> bool {
        if self.term_program.as_deref() != Some("iTerm.app") {
            return false;
        }
        match self.term_program_version.as_deref().and_then(version) {
            Some((major, _)) => major >= MIN_ITERM2_MAJOR,
            // iTerm2 without a version string is old enough to predate the
            // variable. Take the Unicode tier rather than guess.
            None => false,
        }
    }

    /// Whether 24-bit colour is available. `COLORTERM` is the standard signal;
    /// the rest of the list is terminals that set neither `COLORTERM` nor a
    /// `-direct` terminfo entry but do render truecolor.
    fn has_truecolor(&self) -> bool {
        if matches!(self.colorterm.as_deref(), Some("truecolor" | "24bit")) {
            return true;
        }
        if self.kitty_window_id.is_some() {
            return true;
        }
        let term = self.term.as_deref().unwrap_or_default();
        if term.ends_with("-direct") || term.contains("kitty") || term.contains("ghostty") {
            return true;
        }
        matches!(
            self.term_program.as_deref(),
            Some("iTerm.app" | "WezTerm" | "ghostty" | "vscode" | "zed" | "Hyper")
        )
    }

    fn color_depth(&self) -> ColorDepth {
        if !self.stdout_is_tty || self.is_dumb() {
            return ColorDepth::None;
        }
        if self.has_truecolor() {
            return ColorDepth::TrueColor;
        }
        let term = self.term.as_deref().unwrap_or_default();
        if term.contains("256") {
            ColorDepth::Palette256
        } else {
            ColorDepth::Palette16
        }
    }
}

/// An environment variable, treating empty as unset — which is what every
/// shell means by `FOO=`, and what `NO_COLOR`'s own specification says.
fn var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
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
    let (replies, outcome) = match probe::run(host, &probe::full_round(), deadline) {
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
    // A second-round failure is a fact about this terminal, so it replaces a
    // first round that went fine rather than disappearing behind it.
    let outcome = match failure {
        Some(error) => ProbeOutcome::Failed(error),
        None => outcome,
    };

    let iterm2_images = environment.is_iterm2() && graphics.reaches_the_terminal;
    let (tier, pixel_protocol) = tier(environment, &graphics, iterm2_images);

    Capabilities {
        tier,
        pixel_protocol,
        kitty_graphics: graphics.kitty,
        sixel: graphics.sixel,
        iterm2_images,
        kitty_keyboard: answered(&outcome, replies.kitty_keyboard.then_some(true)),
        synchronized_output: answered(&outcome, replies.synchronized_output),
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

/// What the terminal can draw, once tmux has had its say.
struct Graphics {
    kitty: Supported,
    sixel: Supported,
    /// Whether image-protocol bytes reach a terminal that can act on them at
    /// all. False inside a tmux that will not pass them through, which is the
    /// whole point of §4.3: no image tier is attempted on a hope.
    reaches_the_terminal: bool,
}

impl Graphics {
    /// Graphics as a direct probe round found them.
    fn from(replies: &probe::ProbeReplies) -> Self {
        Self {
            kitty: supported(replies.fenced().then_some(replies.kitty_graphics)),
            sixel: supported(replies.sixel()),
            reaches_the_terminal: true,
        }
    }

    /// Graphics ruled out because nothing gets through to the terminal.
    fn blocked() -> Self {
        Self {
            kitty: Supported::Silent,
            sixel: Supported::Silent,
            reaches_the_terminal: false,
        }
    }
}

/// What the tmux path established.
struct TmuxProbe {
    status: TmuxStatus,
    graphics: Graphics,
    /// The re-probe's I/O error, when the terminal could not be written to or
    /// read from. Reported rather than folded into "tmux said nothing": one
    /// is a broken terminal, the other is a working tmux with passthrough off.
    failure: Option<String>,
}

impl TmuxProbe {
    /// No image protocol is reachable, and nothing went wrong finding that out.
    fn blocked(status: TmuxStatus) -> Self {
        Self {
            status,
            graphics: Graphics::blocked(),
            failure: None,
        }
    }
}

/// The tmux path (DESIGN.md §4.3): check the version, then re-send the
/// graphics queries wrapped in tmux's passthrough sequence so the *outer*
/// terminal answers them.
///
/// Nothing here hopes. A version too old, a tmux that refuses to forward
/// (`allow-passthrough` off), an outer terminal with no image protocol: all
/// three look the same from inside — no reply — and all three mean the same
/// thing, the Unicode tier.
fn through_tmux(environment: &Environment, host: &mut dyn TerminalHost) -> TmuxProbe {
    let Some(reported) = tmux_version(environment, host) else {
        return TmuxProbe::blocked(TmuxStatus::VersionUnknown);
    };
    let Some(numbers) = version(&reported) else {
        return TmuxProbe::blocked(TmuxStatus::VersionUnknown);
    };
    if numbers < MIN_PASSTHROUGH_TMUX {
        return TmuxProbe::blocked(TmuxStatus::TooOld { version: reported });
    }

    // A fresh budget: the first round may already have spent its own.
    let deadline = Instant::now() + environment.probe_budget();
    let wrapped = probe::tmux_passthrough(&probe::graphics_round());
    let replies = match probe::run(host, &wrapped, deadline) {
        Ok(replies) => replies,
        Err(error) => {
            return TmuxProbe {
                status: TmuxStatus::PassthroughSilent { version: reported },
                graphics: Graphics::blocked(),
                failure: Some(error.to_string()),
            };
        }
    };

    // The fence itself is the test. With passthrough off, tmux discards the
    // whole wrapped sequence and no `DA1` answer comes back at all, so a
    // fenced round means the outer terminal really did see the queries.
    if !replies.fenced() {
        return TmuxProbe::blocked(TmuxStatus::PassthroughSilent { version: reported });
    }
    TmuxProbe {
        status: TmuxStatus::Passthrough { version: reported },
        graphics: Graphics::from(&replies),
        failure: None,
    }
}

/// tmux's version as a bare `3.4`-style string. tmux sets `TERM_PROGRAM`
/// itself on recent releases, which saves a subprocess; otherwise ask it.
fn tmux_version(environment: &Environment, host: &mut dyn TerminalHost) -> Option<String> {
    let from_environment = (environment.term_program.as_deref() == Some("tmux"))
        .then(|| environment.term_program_version.clone())
        .flatten();
    let reported = match from_environment {
        Some(version) => version,
        None => host.tmux_version()?,
    };
    let trimmed = reported.trim();
    Some(
        trimmed
            .strip_prefix("tmux ")
            .unwrap_or(trimmed)
            .trim()
            .to_owned(),
    )
}

/// The leading `major.minor` of a version string, ignoring any prefix and any
/// suffix: `3.4`, `tmux 3.5a`, `next-3.6` and `3.5.0beta1` all parse.
fn version(text: &str) -> Option<(u16, u16)> {
    let start = text.find(|character: char| character.is_ascii_digit())?;
    let mut fields = text[start..].split('.');
    let major = fields.next()?.parse().ok()?;
    let minor = fields
        .next()
        .map(|field| field.trim_end_matches(|c: char| !c.is_ascii_digit()))
        .and_then(|digits| digits.parse().ok())
        .unwrap_or(0);
    Some((major, minor))
}

/// The tier table of DESIGN.md §4.2, plus §4.3's tmux rule.
///
/// The protocol preference — Kitty, then iTerm2, then Sixel — is §4.2's.
/// KNOWN_GAPS.md records that M1 revisits this order (the user's is Kitty,
/// then *Unicode*, then iTerm2, then Sixel) and must write an ADR when it
/// settles; until then the approved design stands.
fn tier(
    environment: &Environment,
    graphics: &Graphics,
    iterm2_images: bool,
) -> (RenderingTier, Option<PixelProtocol>) {
    let protocol = if graphics.kitty == Supported::Yes {
        Some(PixelProtocol::Kitty)
    } else if iterm2_images {
        Some(PixelProtocol::Iterm2)
    } else if graphics.sixel == Supported::Yes {
        Some(PixelProtocol::Sixel)
    } else {
        None
    };

    match protocol {
        Some(protocol) => (RenderingTier::Pixel, Some(protocol)),
        // §4.3 is explicit that a tmux without passthrough falls back to the
        // Unicode tier, so it does not drop further on a missing `COLORTERM`:
        // tmux draws box-drawing characters whatever its colour depth.
        None if environment.tmux.is_some() => (RenderingTier::Unicode, None),
        None if environment.has_truecolor() => (RenderingTier::Unicode, None),
        None => (RenderingTier::Ascii, None),
    }
}

/// The cell-size fallback chain of DESIGN.md §4.1 step 3, in order: what the
/// terminal reported, what the kernel knows, and finally a flagged guess.
fn cell_size(replies: &probe::ProbeReplies, host: &mut dyn TerminalHost) -> CellSize {
    if let Some((width, height)) = replies.cell_size {
        return CellSize {
            width,
            height,
            source: CellSizeSource::Reported,
        };
    }
    if let Some((width, height)) = host.window_pixels().and_then(WindowPixels::cell_size) {
        return CellSize {
            width,
            height,
            source: CellSizeSource::WindowSize,
        };
    }
    CellSize {
        width: ASSUMED_CELL_SIZE.0,
        height: ASSUMED_CELL_SIZE.1,
        source: CellSizeSource::Assumed,
    }
}

/// The light/dark hint from the reported background, or dark flagged as the
/// assumption it is (DESIGN.md §4.1 step 3).
fn theme(background: Option<Rgb>) -> ThemeHint {
    match background {
        Some(background) => ThemeHint {
            theme: if is_light(background) {
                Theme::Light
            } else {
                Theme::Dark
            },
            source: ThemeSource::Background(background),
        },
        None => ThemeHint {
            theme: Theme::Dark,
            source: ThemeSource::Assumed,
        },
    }
}

/// Whether a background reads as light. Rec. 709 luma against a mid threshold:
/// this only ever picks one of two themes, so the extra accuracy of a full
/// sRGB-linearised luminance would change nothing but the arithmetic.
fn is_light(color: Rgb) -> bool {
    let luma = 0.2126 * f32::from(color.red)
        + 0.7152 * f32::from(color.green)
        + 0.0722 * f32::from(color.blue);
    luma > 127.5
}

/// A yes/no query's answer, keeping "nothing came back" distinct from "no".
fn supported(answer: Option<bool>) -> Supported {
    match answer {
        Some(true) => Supported::Yes,
        Some(false) => Supported::No,
        None => Supported::Silent,
    }
}

/// As [`supported`], but silence only counts as an answer when the round was
/// fenced: an unfenced round proves nothing about anything.
fn answered(outcome: &ProbeOutcome, answer: Option<bool>) -> Supported {
    match (outcome, answer) {
        (_, Some(state)) => supported(Some(state)),
        (ProbeOutcome::Answered, None) => Supported::No,
        (_, None) => Supported::Silent,
    }
}

/// The capabilities of a terminal Graft never asked: output that is not a
/// terminal, or `TERM=dumb`. Both are ASCII by DESIGN.md §4.2, and writing
/// probe bytes into either would only corrupt what is on the other end.
fn unprobed(environment: &Environment) -> Capabilities {
    Capabilities {
        tier: RenderingTier::Ascii,
        pixel_protocol: None,
        kitty_graphics: Supported::Silent,
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
