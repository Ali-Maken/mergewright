//! The cheap half of detection: what this process can know for free.
//!
//! DESIGN.md §4.1 step 1. Everything here is an environment variable or a
//! one-syscall fact, read once into an [`Environment`] and never read again.
//! None of it is trusted on its own — `$TERM` survives `ssh`, `tmux`, `sudo`
//! and a copied shell config — so these answers only choose *how* to ask the
//! terminal and what to fall back to when it says nothing. The single
//! exception is iTerm2's inline images, which have no query to ask.

use std::env;
use std::io::{self, IsTerminal};
use std::time::Duration;

use graft_core::capabilities::ColorDepth;

use super::{LOCAL_PROBE_BUDGET, SSH_PROBE_BUDGET};

/// The first iTerm2 whose inline-image support Graft relies on.
const MIN_ITERM2_MAJOR: u16 = 3;

/// The cheap hints, read once (DESIGN.md §4.1 step 1).
///
/// A plain value rather than direct `env::var` calls inside detection: it
/// keeps the whole decision testable without touching process environment —
/// which in Rust 2024 is `unsafe` and, in a test binary with threads, racy.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Environment {
    /// `$TERM`: which terminfo entry to use. Says nothing reliable about the
    /// terminal actually in front of the user.
    pub term: Option<String>,
    /// `$TERM_PROGRAM`: the emulator's own name for itself, where it sets one.
    pub term_program: Option<String>,
    /// `$TERM_PROGRAM_VERSION`, alongside the name above.
    pub term_program_version: Option<String>,
    /// `$COLORTERM`: `truecolor` or `24bit` is the standard 24-bit signal.
    pub colorterm: Option<String>,
    /// `$KITTY_WINDOW_ID`. Only its presence matters: Kitty sets it, and
    /// Kitty has truecolor.
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
    pub(super) fn probe_budget(&self) -> Duration {
        if self.over_ssh {
            SSH_PROBE_BUDGET
        } else {
            LOCAL_PROBE_BUDGET
        }
    }

    /// A terminal worth asking questions. `TERM=dumb` answers nothing by
    /// definition, and a pipe would just be sent escape sequences that end up
    /// in the user's file.
    pub(super) fn is_probeable(&self) -> bool {
        self.stdout_is_tty && !self.is_dumb()
    }

    fn is_dumb(&self) -> bool {
        self.term.as_deref() == Some("dumb")
    }

    /// The iTerm2 hint: the one capability allowed to rest on environment
    /// alone, because iTerm2's inline images have no query (DESIGN.md §4.1
    /// step 4).
    pub(super) fn is_iterm2(&self) -> bool {
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
    pub(super) fn has_truecolor(&self) -> bool {
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

    pub(super) fn color_depth(&self) -> ColorDepth {
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

/// The leading `major.minor` of a version string, ignoring any prefix and any
/// suffix: `3.4`, `tmux 3.5a`, `next-3.6` and `3.5.0beta1` all parse.
///
/// Lives here because both version strings detection reads are environment
/// strings first: iTerm2's `TERM_PROGRAM_VERSION`, and tmux's, which recent
/// releases also publish there rather than making Graft run `tmux -V`.
pub(super) fn version(text: &str) -> Option<(u16, u16)> {
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
