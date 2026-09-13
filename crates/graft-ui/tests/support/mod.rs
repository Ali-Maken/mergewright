//! Shared by every test binary in this crate; each uses only part of it.
#![allow(
    dead_code,
    reason = "each test binary uses a different part of the support code"
)]

pub mod corpus;
pub mod screen;

use graft_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, ProbeOutcome, RenderingTier, Supported,
    Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
};
use graft_ui::theme::{Theme, ThemeKind};

/// A theme on an ordinary Unicode terminal with this colour depth.
pub fn theme(kind: ThemeKind, color_depth: ColorDepth) -> Theme {
    themed(kind, color_depth, false)
}

/// A theme on an ordinary Unicode terminal with this colour depth, and
/// `NO_COLOR` set or not.
pub fn themed(kind: ThemeKind, color_depth: ColorDepth, no_color: bool) -> Theme {
    Theme::new(
        kind,
        &Capabilities {
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
            probe_budget: std::time::Duration::from_millis(250),
        },
    )
}
