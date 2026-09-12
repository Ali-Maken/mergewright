//! Fixtures shared by the crate's tests.
//!
//! [`Capabilities`] has no `Default` on purpose — an unprobed terminal is not
//! a neutral one — so every test that needs a [`Theme`] would otherwise repeat
//! a sixteen-field literal, and a new field would mean editing all of them.

use std::time::Duration;

use graft_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, ProbeOutcome, RenderingTier, Supported,
    Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
};

/// A terminal with the given colour depth and `NO_COLOR` setting. Every other
/// field is irrelevant to the tests that use this and is filled with something
/// inert.
pub(crate) fn terminal(color_depth: ColorDepth, no_color: bool) -> Capabilities {
    Capabilities {
        tier: RenderingTier::Unicode,
        pixel_protocol: None,
        kitty_graphics: Supported::No,
        sixel: Supported::No,
        iterm2_images: false,
        kitty_keyboard: Supported::No,
        synchronized_output: Supported::No,
        cell_size: CellSize {
            width: 8,
            height: 16,
            source: CellSizeSource::Assumed,
        },
        theme: ThemeHint {
            theme: TerminalTheme::Dark,
            source: ThemeSource::Assumed,
        },
        color_depth,
        no_color,
        is_tty: true,
        over_ssh: false,
        tmux: TmuxStatus::Outside,
        probe: ProbeOutcome::Answered,
        probe_budget: Duration::from_millis(250),
    }
}
