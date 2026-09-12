//! Detection from inside tmux (DESIGN.md §4.3).
//!
//! tmux sits between Graft and the terminal that actually draws, so a
//! graphics query answered in round one says only that *tmux* is there. This
//! module re-asks the graphics queries wrapped in tmux's passthrough
//! sequence, so the outer terminal is the one that answers.
//!
//! Nothing here hopes. A tmux too old to forward anything, a tmux with
//! `allow-passthrough` off, and an outer terminal with no image protocol all
//! look identical from inside — no reply — and all three mean the same thing:
//! no pixel tier.

use std::time::Instant;

use graft_core::capabilities::TmuxStatus;

use super::environment::{Environment, version};
use super::{Graphics, TerminalHost, probe};

/// The first tmux that can forward escape sequences to the outer terminal at
/// all (DESIGN.md §4.3). Older versions get the Unicode tier, deliberately.
const MIN_PASSTHROUGH_TMUX: (u16, u16) = (3, 3);

/// What the tmux path established.
pub(super) struct TmuxProbe {
    pub(super) status: TmuxStatus,
    pub(super) graphics: Graphics,
    /// The re-probe's I/O error, when the terminal could not be written to or
    /// read from. Reported rather than folded into "tmux said nothing": one
    /// is a broken terminal, the other is a working tmux with passthrough off.
    pub(super) failure: Option<String>,
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
pub(super) fn through_tmux(environment: &Environment, host: &mut dyn TerminalHost) -> TmuxProbe {
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
