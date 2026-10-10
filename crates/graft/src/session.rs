//! What Graft settled before the first frame, and does not change while it
//! runs.
//!
//! Two values, split by when they exist. [`Startup`] is what `main` works out
//! with the terminal still the user's — the repository to open, and the config
//! file, whose failure has to be printable on a normal terminal. [`Session`] is
//! what is true once the terminal has been taken over and the capability probe
//! has answered: the tier, the theme, and where each of those came from.
//!
//! Neither belongs in [`AppState`](crate::update::AppState). The state is what
//! events change; these are the facts events are interpreted *against*, they
//! are settled once, and keeping them out of the state is what keeps `update`
//! a function of the things that actually vary.

use graft_core::capabilities::{Capabilities, CellSize, ProbeOutcome, ThemeHint, ThemeSource};
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::status_screen::{RenderingView, TierSource};
use graft_ui::theme::{Theme, ThemeKind};

use crate::config::{Loaded, ThemeOverrides};

/// What `main` decided before touching the terminal.
pub struct Startup {
    /// The repository to open, from the command line or the current directory.
    pub repo_path: std::path::PathBuf,
    /// The config file, and anything odd about it.
    pub config: Loaded,
    /// What a harness wrapping `run` asks for (the graph gallery,
    /// m1-the-graph/35): a standing footer line, keys handed back as
    /// [`crate::event_loop::Exit::HandedBack`]. Default for the shipped
    /// binary.
    pub harness: crate::update::Harness,
}

/// A terminal answer that changed mid-session (DESIGN.md §4.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalChange {
    /// The cell size after a resize, provenance included.
    CellSize(CellSize),
    /// The background's light/dark hint after focus regain.
    Theme(ThemeHint),
}

/// **The one place `SystemTime::now()` is read for anything a capture can
/// show** (m1-the-graph/53; widened m5-history-and-ghost-preview/25).
///
/// It started as a rule about the date path — every relative date on
/// screen is `then` fitted against [`Clock::now_seconds`], never the wall
/// clock read directly. That scope left a hole, and captures fell into it:
/// the journal timestamp `jobs::operate` and `jobs::undo` stamp each entry
/// with is not a date on the date path, so it read the wall clock
/// directly, and any capture of a repository the app writes **during** the
/// capture drew an instant that changed on every run. Pinning that
/// render-side instead would draw a real entry as a literal `future`
/// (`graft_ui::journal::columns`).
///
/// So the rule is the whole rule now: **any instant that can reach a
/// frame — drawn, or written into the repository and then drawn — comes
/// through a `Clock`.** Timing that never reaches the screen (a debounce
/// window, a deadline) is not covered and does not need to be.
///
/// The shipped binary passes two clocks: the drawn clock is `Wall`, or
/// `Fixed` when `TRIB_NOW` pins it ([`launch_clock`]); the journal's stamps
/// are always `Wall`. `Fixed` is also what a caller that
/// cannot tolerate drift asks for instead: a snapshot test, the gallery
/// viewing a corpus stop at `graft_fixtures::CORPUS_NOW` so its capture
/// never ages, or a capture that writes to the repository it is
/// photographing (`JobRunner::with_clock`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Clock {
    /// Read the real system clock.
    #[default]
    Wall,
    /// Always this instant, Unix seconds.
    Fixed(i64),
}

impl Clock {
    /// The instant to draw every relative date against.
    ///
    /// `duration_since` only fails when the system clock reads before 1970,
    /// in which case every commit is at least that old and `relative_date`
    /// reads it the same as any far-past commit.
    #[must_use]
    pub fn now_seconds(self) -> i64 {
        match self {
            Clock::Wall => std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| {
                    i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
                }),
            Clock::Fixed(seconds) => seconds,
        }
    }

    /// The same instant in milliseconds, for the one thing a second is too
    /// coarse for: debouncing a terminal's focus reports, which arrive in
    /// pairs and in bursts (m3-working-tree-undo-and-stash/06). Read at
    /// millisecond resolution rather than scaled up from
    /// [`Clock::now_seconds`], which would round two reports 600 ms apart
    /// into the same instant.
    #[must_use]
    pub fn now_millis(self) -> i64 {
        match self {
            Clock::Wall => std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| {
                    i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
                }),
            Clock::Fixed(seconds) => seconds.saturating_mul(1_000),
        }
    }
}

/// Set to whole seconds since 1970, pins the session clock for the whole
/// run — the **drawn clock only**: every age on screen and the practice
/// repo's dates (m8/19d-b). Journal stamps stay on the wall clock: a test
/// variable must never write false history into a real repository
/// (m8/19e §R1). For reproducible pty runs and captures; the pty harness
/// sets it by default.
pub(crate) const NOW_VAR: &str = "TRIB_NOW";

/// What [`NOW_VAR`]'s value asks for: unset → `Wall`; whole seconds →
/// `Fixed`; anything else → the words of the refusal (unsanitised; the
/// caller sanitises). Pure.
pub(crate) fn clock_from(var: Option<&std::ffi::OsStr>) -> Result<Clock, String> {
    let Some(raw) = var else {
        return Ok(Clock::Wall);
    };
    let refuse = || {
        format!(
            "{NOW_VAR} must be whole seconds since 1970, got {}",
            raw.to_string_lossy()
        )
    };
    let text = raw.to_str().ok_or_else(refuse)?;
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(refuse());
    }
    let seconds = text.parse::<i64>().map_err(|_too_big| refuse())?;
    Ok(Clock::Fixed(seconds))
}

/// The session clock this process runs on, read once at launch from
/// [`NOW_VAR`] (`clock_from`): the drawn clock only, never the journal's
/// stamps (m8/19e §R1). `Err` carries sanitised words for stderr; the
/// binary refuses to launch on it (exit 2).
///
/// # Errors
/// The variable is set to anything but whole seconds.
pub fn launch_clock() -> Result<Clock, String> {
    clock_from(std::env::var_os(NOW_VAR).as_deref())
        .map_err(|words| sanitize_line(&words).as_str().to_owned())
}

/// What Graft is drawing with, for as long as this run lasts.
pub struct Session {
    /// Everything detection found. Kept whole: the tier and theme below are
    /// conclusions drawn from it, and the diagnostics screen shows the
    /// evidence they were drawn from.
    pub capabilities: Capabilities,
    /// The resolved theme, ready for widgets to paint from.
    pub theme: Theme,
    /// The tier and theme as the status screen shows them, provenance
    /// included.
    pub rendering: RenderingView,
    /// Anything worth telling the user that was known before the first frame —
    /// today, the config file's warnings. Already sanitised: the text quotes
    /// keys the user typed.
    pub notes: Vec<SanitizedText>,
    /// His `[theme.<kind>]` colours, fixed for the run: every rebuild of
    /// [`Session::theme`] goes through [`Session::retheme`], which reads them.
    overrides: ThemeOverrides,
    /// The theme he chose from the palette (`Theme: light`, m8/11 decision
    /// 6): it sticks for the run, over the terminal's evidence and over a
    /// forced `ui.theme`. `None` until he chooses.
    chosen: Option<ThemeKind>,
}

impl Session {
    /// Combine what was detected with what the config file asked for.
    ///
    /// Config wins, in both directions (DESIGN.md §4.2): detection can prove
    /// that a terminal answered a query, never that it cannot draw, so forcing
    /// `pixel` on a silent terminal is allowed and forcing `ascii` on a
    /// capable one is respected.
    #[must_use]
    pub fn settle(config: &Loaded, capabilities: Capabilities) -> Self {
        let forced_tier = config.config.graph.renderer.overridden();
        let forced_theme = config.config.ui.theme.overridden();
        let kind = ThemeKind::select(forced_theme, capabilities.theme);

        let rendering = RenderingView {
            tier: forced_tier.unwrap_or(capabilities.tier),
            tier_source: tier_source(forced_tier.is_some(), &capabilities.probe),
            cell_size: capabilities.cell_size,
            theme: kind,
            theme_forced: forced_theme.is_some(),
            // `Assumed` is the terminal saying nothing, not a measurement of a
            // dark background (ADR-0003), and the fix is a line of config —
            // which the user can only know to write if the screen says so.
            theme_unknown: matches!(capabilities.theme.source, ThemeSource::Assumed),
            color_depth: capabilities.color_depth,
            no_color: capabilities.no_color,
        };

        // Config warnings are not notes here: with the keymap's they are the
        // repository screen's `config_problems` (`config::keymap_from`,
        // ADR-0055), said once on the bottom row and listed on `F2`.
        let mut notes: Vec<SanitizedText> = Vec::new();
        // The tier row says the probe failed; only here is there room to say
        // what it failed with. An I/O error on the terminal is not something
        // to drop on the floor because Graft managed to carry on without it.
        if let ProbeOutcome::Failed(error) = &capabilities.probe {
            notes.push(sanitize_line(&format!(
                "the terminal capability probe failed: {error}"
            )));
        }

        let overrides = config.config.theme.clone();
        Self {
            theme: Theme::with_overrides(kind, &capabilities, overrides.for_kind(kind)),
            rendering,
            notes,
            capabilities,
            overrides,
            chosen: None,
        }
    }

    /// Rebuild the theme as `kind`, with that kind's overrides — the only way
    /// the theme changes mid-run (a re-detected background; the palette's
    /// theme rows). `true` when the theme drawn changed.
    pub(crate) fn retheme(&mut self, kind: ThemeKind) -> bool {
        let theme = Theme::with_overrides(kind, &self.capabilities, self.overrides.for_kind(kind));
        self.rendering.theme = kind;
        if theme == self.theme {
            return false;
        }
        self.theme = theme;
        true
    }

    /// He chose `kind` from the palette: draw in it for the rest of the run,
    /// whatever the terminal says later. `true` when the theme drawn
    /// changed.
    pub(crate) fn choose(&mut self, kind: ThemeKind) -> bool {
        self.chosen = Some(kind);
        self.retheme(kind)
    }

    /// Take in a re-detected answer; `true` when anything drawn from it
    /// changed, so the caller knows to redraw (and the pixel renderer to
    /// re-rasterise). The tier is never touched: it is decided once.
    pub fn apply(&mut self, change: TerminalChange) -> bool {
        match change {
            TerminalChange::CellSize(size) => {
                if self.capabilities.cell_size == size {
                    return false;
                }
                self.capabilities.cell_size = size;
                self.rendering.cell_size = size;
            }
            TerminalChange::Theme(hint) => {
                if self.capabilities.theme == hint {
                    return false;
                }
                self.capabilities.theme = hint;
                // His palette choice, then a `ui.theme` override, still
                // win; only the evidence changes.
                let forced = self
                    .chosen
                    .or_else(|| self.rendering.theme_forced.then_some(self.rendering.theme));
                // The hint changed, so something drawn from it did, whether
                // or not the theme itself did (`theme_unknown` below).
                self.retheme(ThemeKind::select(forced, hint));
                self.rendering.theme_unknown = matches!(hint.source, ThemeSource::Assumed);
            }
        }
        true
    }
}

/// Where the tier on screen came from (DESIGN.md §4.1's provenance rule).
///
/// Config first, because an override means detection's answer is not what is
/// in force whatever the probe did. Otherwise the probe's own outcome decides:
/// only a round that came back fenced licenses the word "detected" — everything
/// else produced a tier from environment hints alone.
fn tier_source(forced: bool, probe: &ProbeOutcome) -> TierSource {
    if forced {
        return TierSource::Forced;
    }
    match probe {
        ProbeOutcome::Answered => TierSource::Detected,
        ProbeOutcome::TimedOut => TierSource::Silent,
        ProbeOutcome::Failed(_) => TierSource::ProbeFailed,
        ProbeOutcome::Skipped => TierSource::NotProbed,
    }
}

#[cfg(test)]
pub(crate) mod for_tests {
    //! A terminal for tests that need a [`Session`] and do not care what it
    //! detected. [`Capabilities`] has no `Default` on purpose — an unprobed
    //! terminal is not a neutral one — so the literal lives here once instead
    //! of in every test module.

    use std::time::Duration;

    use graft_core::capabilities::{
        Capabilities, CellSize, CellSizeSource, ColorDepth, ProbeOutcome, RenderingTier, Supported,
        Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
    };

    use super::Session;
    use crate::config::Loaded;

    /// A plain truecolour terminal that answered every probe.
    pub(crate) fn capabilities() -> Capabilities {
        Capabilities {
            tier: RenderingTier::Unicode,
            pixel_protocol: None,
            kitty_graphics: Supported::No,
            kitty_deflate: false,
            answers_dsr: false,
            sixel: Supported::No,
            iterm2_images: false,
            kitty_keyboard: Supported::No,
            synchronized_output: Supported::Yes,
            cell_size: CellSize {
                width: 9,
                height: 18,
                source: CellSizeSource::Reported,
            },
            theme: ThemeHint {
                theme: TerminalTheme::Dark,
                source: ThemeSource::Background(graft_core::capabilities::Rgb {
                    red: 0x10,
                    green: 0x10,
                    blue: 0x14,
                }),
            },
            color_depth: ColorDepth::TrueColor,
            no_color: false,
            is_tty: true,
            over_ssh: false,
            tmux: TmuxStatus::Outside,
            probe: ProbeOutcome::Answered,
            probe_budget: Duration::from_millis(250),
        }
    }

    /// A session on that terminal, with no config file.
    pub(crate) fn session() -> Session {
        Session::settle(&Loaded::default(), capabilities())
    }
}

#[cfg(test)]
mod theme_tests;

#[cfg(test)]
mod tests;
