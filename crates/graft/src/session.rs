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

use graft_core::capabilities::{Capabilities, ThemeSource};
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::status_screen::RenderingView;
use graft_ui::theme::{Theme, ThemeKind};

use crate::config::Loaded;

/// What `main` decided before touching the terminal.
pub struct Startup {
    /// The repository to open, from the command line or the current directory.
    pub repo_path: std::path::PathBuf,
    /// The config file, and anything odd about it.
    pub config: Loaded,
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
            tier_forced: forced_tier.is_some(),
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

        Self {
            theme: Theme::new(kind, &capabilities),
            rendering,
            notes: config
                .warnings
                .iter()
                .map(|warning| sanitize_line(&warning.to_string()))
                .collect(),
            capabilities,
        }
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
mod tests {
    use graft_core::capabilities::RenderingTier;

    use super::*;
    use crate::config::{Config, Graph, RendererChoice, ThemeChoice, Ui};

    fn loaded(theme: ThemeChoice, renderer: RendererChoice) -> Loaded {
        Loaded {
            config: Config {
                ui: Ui {
                    theme,
                    ..Ui::default()
                },
                graph: Graph { renderer },
            },
            ..Loaded::default()
        }
    }

    #[test]
    fn detection_decides_when_the_config_says_nothing() {
        let session = Session::settle(&Loaded::default(), for_tests::capabilities());

        assert_eq!(session.rendering.tier, RenderingTier::Unicode);
        assert!(!session.rendering.tier_forced);
        assert_eq!(session.rendering.theme, ThemeKind::Dark);
        assert!(!session.rendering.theme_forced);
        assert!(!session.rendering.theme_unknown);
    }

    #[test]
    fn the_config_file_wins_over_detection() {
        // Upwards too: the terminal never claimed it could draw pixels.
        let config = loaded(ThemeChoice::Light, RendererChoice::Pixel);

        let session = Session::settle(&config, for_tests::capabilities());

        assert_eq!(session.rendering.tier, RenderingTier::Pixel);
        assert!(session.rendering.tier_forced);
        assert_eq!(session.rendering.theme, ThemeKind::Light);
        assert!(session.rendering.theme_forced);
    }

    #[test]
    fn a_terminal_that_said_nothing_about_its_background_says_so() {
        let mut capabilities = for_tests::capabilities();
        capabilities.theme.source = ThemeSource::Assumed;

        let session = Session::settle(&Loaded::default(), capabilities);

        assert_eq!(session.rendering.theme, ThemeKind::Dark);
        assert!(session.rendering.theme_unknown);
    }

    #[test]
    fn config_warnings_become_notes_and_are_sanitised() {
        let config = Loaded {
            warnings: vec![crate::config::ConfigWarning::UnknownKey {
                path: "ui.\u{1b}[31mtheme".to_owned(),
            }],
            ..Loaded::default()
        };

        let session = Session::settle(&config, for_tests::capabilities());

        assert_eq!(session.notes.len(), 1);
        assert!(
            !session.notes[0].as_str().contains('\u{1b}'),
            "{:?}",
            session.notes[0]
        );
    }
}
