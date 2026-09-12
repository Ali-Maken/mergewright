//! What the host terminal can do, as plain data.
//!
//! [`graft_term::detect`] produces a [`Capabilities`]; `graph`'s renderers and
//! `ui`'s diagnostics view consume it. Those three crates are siblings, so the
//! shape has to sit below all of them — here (DESIGN.md §8.2). Nothing in this
//! module probes, decides or formats anything: it only records what detection
//! found, including *how* it found each value, so a fallback is never mistaken
//! for a measurement (ADR-0007).
//!
//! [`graft_term::detect`]: https://docs.rs/graft-term

use std::time::Duration;

/// How Graft draws the commit graph (CONTEXT.md: "rendering tier").
///
/// Decided once per session, before the first frame (DESIGN.md §4.2). Config
/// may override it; that is the app layer's job, not detection's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderingTier {
    /// Real pixels through an image protocol — see [`Capabilities::pixel_protocol`].
    Pixel,
    /// Box-drawing characters. The everyday tier in editor terminals, which
    /// have no image protocol.
    Unicode,
    /// ASCII only: no colour assumptions, no box drawing.
    Ascii,
}

/// The image protocol the pixel tier would use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PixelProtocol {
    /// The Kitty graphics protocol (Kitty, Ghostty, WezTerm).
    Kitty,
    /// iTerm2 inline images (OSC 1337).
    Iterm2,
    /// Sixel.
    Sixel,
}

/// The answer to one yes/no capability query, with the silence kept distinct
/// from a spoken "no".
///
/// Silence and a reported "no" mean the same thing to a renderer and very
/// different things to whoever is debugging why a tier was chosen, so the two
/// are never collapsed here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Supported {
    /// The terminal answered, and the capability is there.
    Yes,
    /// The terminal answered, and said it does not have the capability.
    No,
    /// Nothing came back before the probe deadline. Treated as unsupported —
    /// every query that can go silent is an enhancement, never a correctness
    /// requirement (DESIGN.md §4.1 step 3).
    Silent,
}

/// One character cell, in pixels, and where the number came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CellSize {
    /// Cell width in pixels.
    pub width: u32,
    /// Cell height in pixels.
    pub height: u32,
    /// Whether anything measured the two numbers above.
    pub source: CellSizeSource,
}

/// Where a [`CellSize`] came from — the difference between a measurement and a
/// guess, which the diagnostics view shows rather than hiding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellSizeSource {
    /// The terminal answered `CSI 16 t`. Exact.
    Reported,
    /// Derived from the kernel's window size (`TIOCGWINSZ`): pixel dimensions
    /// divided by the character grid. Exact when the terminal fills those
    /// fields honestly, which most do and some round.
    WindowSize,
    /// Nothing answered and the kernel reported no pixel size, so
    /// `term`'s assumed default stands in. Approximate: never treat it as
    /// measured (DESIGN.md §4.1 step 3).
    Assumed,
}

/// An 8-bit-per-channel colour. Only ever a terminal's *reported* background
/// here, used for the light/dark hint and never as a paint colour (ADR-0003).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb {
    /// Red, 0-255.
    pub red: u8,
    /// Green, 0-255.
    pub green: u8,
    /// Blue, 0-255.
    pub blue: u8,
}

/// Which way the terminal's colours run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Theme {
    /// Light text on a dark background. Graft's default, and what an
    /// unreported background is assumed to be.
    Dark,
    /// Dark text on a light background.
    Light,
}

/// The light/dark hint, always usable, always carrying whether it was measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ThemeHint {
    /// Usable straight away: [`ThemeSource::Assumed`] already resolves to
    /// [`Theme::Dark`] (DESIGN.md §4.1 step 3).
    pub theme: Theme,
    /// Whether the terminal reported the background this was read from.
    pub source: ThemeSource,
}

/// Where a [`ThemeHint`] came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeSource {
    /// The terminal answered `OSC 11` with this background colour.
    Background(Rgb),
    /// `OSC 11` stayed silent or answered something unparseable, so dark
    /// stands in. This is the "unknown" state: the value is a default, and a
    /// manual `ui.theme` override is the fix.
    Assumed,
}

/// How much colour the terminal has. Independent of [`Capabilities::no_color`]:
/// this is what the terminal *can* do, that is what the user *asked* for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorDepth {
    /// 24-bit colour: any RGB value is drawable as asked.
    TrueColor,
    /// The 256-colour palette. Graft's colours are matched to the nearest
    /// entry.
    Palette256,
    /// The 16 ANSI colours, whose actual values the user's terminal theme
    /// chooses. The floor for anything colourful.
    Palette16,
    /// No colour at all: `TERM=dumb`, or output that is not a terminal.
    None,
}

/// Whether Graft is inside tmux and, if so, whether the outer terminal can be
/// reached through it (DESIGN.md §4.3). Decided once at startup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TmuxStatus {
    /// No `$TMUX`: Graft is talking to the terminal directly.
    Outside,
    /// Inside tmux, and tmux answered the passthrough-wrapped probe: the outer
    /// terminal is reachable, so a pixel tier is on the table.
    Passthrough {
        /// The `major.minor[suffix]` string tmux reported.
        version: String,
    },
    /// Inside tmux, passthrough usable in principle, but the wrapped probe
    /// went unanswered — `allow-passthrough` is off, or the outer terminal has
    /// no image protocol. Unicode tier, deliberately.
    PassthroughSilent {
        /// The `major.minor[suffix]` string tmux reported.
        version: String,
    },
    /// Inside tmux older than the version that can pass escape sequences
    /// through at all. Unicode tier, deliberately.
    TooOld {
        /// The `major.minor[suffix]` string tmux reported.
        version: String,
    },
    /// Inside tmux, but its version could not be established (`tmux -V` did
    /// not run, or its output was not a version). Treated as too old.
    VersionUnknown,
}

/// What became of the probe round (DESIGN.md §4.1 step 2).
///
/// Read this before reading any [`Supported::Silent`] below it: silence means
/// "this terminal does not answer that query" only when the round itself
/// finished.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The queries went out and the `DA1` fence came back, so every reply that
    /// was going to arrive had arrived.
    Answered,
    /// The queries went out and the deadline passed with no `DA1` fence.
    /// Everything below falls back.
    TimedOut,
    /// No queries were sent: output is not a terminal, or `TERM=dumb`.
    Skipped,
    /// The terminal could not be written to or read from. The message is the
    /// I/O error; detection carried on with fallbacks rather than refusing to
    /// start, and this is what the diagnostics view shows instead of a tier
    /// nobody can explain.
    Failed(String),
}

/// Everything detection established about the terminal, in one value.
///
/// Every field that can be a fallback says so in its own type — there is no
/// separate "is this real?" flag to forget to check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capabilities {
    /// The tier as *detected*. Config may override it (DESIGN.md §4.2, last
    /// row); that combination happens in the app layer, not here.
    pub tier: RenderingTier,
    /// Which protocol [`RenderingTier::Pixel`] would use. `None` on every
    /// other tier.
    pub pixel_protocol: Option<PixelProtocol>,
    /// Answered the Kitty graphics query (`a=q`), fenced by `DA1`.
    pub kitty_graphics: Supported,
    /// `DA1` listed parameter 4.
    pub sixel: Supported,
    /// iTerm2 inline images. Environment hint only — iTerm2's inline-image
    /// support has no query protocol, and this is the single place
    /// PROMPT.md §1.1 allows an env hint to stand alone (DESIGN.md §4.1 step
    /// 4).
    pub iterm2_images: bool,
    /// The Kitty keyboard protocol, which is what makes `Ctrl+Shift+Z`
    /// distinguishable from `Ctrl+Z` (DESIGN.md §2.2).
    pub kitty_keyboard: Supported,
    /// DEC mode 2026. Diagnostic only: Graft brackets frames either way,
    /// because a terminal without the mode ignores the sequence harmlessly.
    pub synchronized_output: Supported,
    /// One character cell in pixels, for the pixel tier to draw against.
    pub cell_size: CellSize,
    /// Light or dark, and whether the terminal said so.
    pub theme: ThemeHint,
    /// How much colour the terminal can show, whatever the user asked for.
    pub color_depth: ColorDepth,
    /// `NO_COLOR` is set to a non-empty value. Colour off in the chrome; the
    /// tier is unaffected, because `NO_COLOR` is about colour, not about
    /// protocol capability (DESIGN.md §4.6).
    pub no_color: bool,
    /// Graft's output is a terminal at all.
    pub is_tty: bool,
    /// `$SSH_TTY`/`$SSH_CONNECTION` is set. Only effect: a longer probe
    /// deadline (DESIGN.md §4.4).
    pub over_ssh: bool,
    /// Whether Graft is inside tmux, and whether the outer terminal could be
    /// reached through it.
    pub tmux: TmuxStatus,
    /// What became of the probe round. Anything but
    /// [`ProbeOutcome::Answered`] means every field above that could not be
    /// answered from the environment is a fallback.
    pub probe: ProbeOutcome,
    /// The deadline detection gave the terminal to answer.
    pub probe_budget: Duration,
}
