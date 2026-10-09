use graft_core::capabilities::{
    CellSizeSource, RenderingTier, Rgb, Theme as TerminalTheme, ThemeHint,
};

#[test]
fn clock_from_reads_whole_seconds_only() {
    use std::ffi::OsStr;
    assert_eq!(clock_from(None), Ok(Clock::Wall));
    assert_eq!(
        clock_from(Some(OsStr::new("1609459200"))),
        Ok(Clock::Fixed(1_609_459_200))
    );
    for bad in ["x", "", "1.5", "-3", "99999999999999999999"] {
        assert_eq!(
            clock_from(Some(OsStr::new(bad))),
            Err(format!(
                "GRAFT_NOW must be whole seconds since 1970, got {bad}"
            )),
            "{bad:?}"
        );
    }
}

use super::*;
use crate::config::{Config, Graph, RendererChoice, ThemeChoice, Ui};

fn loaded(theme: ThemeChoice, renderer: RendererChoice) -> Loaded {
    Loaded {
        config: Config {
            ui: Ui {
                theme,
                ..Ui::default()
            },
            graph: Graph {
                renderer,
                ..Graph::default()
            },
            ..Config::default()
        },
        ..Loaded::default()
    }
}

#[test]
fn detection_decides_when_the_config_says_nothing() {
    let session = Session::settle(&Loaded::default(), for_tests::capabilities());

    assert_eq!(session.rendering.tier, RenderingTier::Unicode);
    assert_eq!(session.rendering.tier_source, TierSource::Detected);
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
    assert_eq!(session.rendering.tier_source, TierSource::Forced);
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
fn a_terminal_that_never_answered_the_probe_does_not_claim_detection() {
    // The M0 review's Major 2: the tier was worked out from environment
    // hints alone, and the screen must not call that a measurement.
    let mut capabilities = for_tests::capabilities();
    capabilities.probe = ProbeOutcome::TimedOut;

    let session = Session::settle(&Loaded::default(), capabilities);

    assert_eq!(session.rendering.tier_source, TierSource::Silent);
}

#[test]
fn a_probe_that_failed_says_so_and_reports_the_error() {
    let mut capabilities = for_tests::capabilities();
    capabilities.probe = ProbeOutcome::Failed("broken pipe".to_owned());

    let session = Session::settle(&Loaded::default(), capabilities);

    assert_eq!(session.rendering.tier_source, TierSource::ProbeFailed);
    assert!(
        session
            .notes
            .iter()
            .any(|note| note.as_str().contains("broken pipe")),
        "{:?}",
        session.notes
    );
}

#[test]
fn a_terminal_that_was_never_asked_says_that_instead() {
    let mut capabilities = for_tests::capabilities();
    capabilities.probe = ProbeOutcome::Skipped;

    let session = Session::settle(&Loaded::default(), capabilities);

    assert_eq!(session.rendering.tier_source, TierSource::NotProbed);
}

#[test]
fn an_override_is_named_as_one_even_when_the_probe_never_answered() {
    // Both are true; the override is what is actually in force, so that is
    // what the row says.
    let mut capabilities = for_tests::capabilities();
    capabilities.probe = ProbeOutcome::TimedOut;

    let session = Session::settle(
        &loaded(ThemeChoice::Dark, RendererChoice::Ascii),
        capabilities,
    );

    assert_eq!(session.rendering.tier_source, TierSource::Forced);
}

#[test]
fn a_new_cell_size_changes_the_session_but_not_the_tier() {
    let mut session = for_tests::session();
    let tier = session.rendering.tier;
    let size = CellSize {
        width: 20,
        height: 40,
        source: CellSizeSource::WindowSize,
    };

    assert!(session.apply(TerminalChange::CellSize(size)));
    assert!(
        !session.apply(TerminalChange::CellSize(size)),
        "no change twice"
    );

    assert_eq!(session.rendering.cell_size, size);
    assert_eq!(session.capabilities.cell_size, size);
    assert_eq!(session.rendering.tier, tier);
}

#[test]
fn a_light_background_on_focus_regain_switches_the_theme() {
    let mut session = for_tests::session();
    let light = ThemeHint {
        theme: TerminalTheme::Light,
        source: ThemeSource::Background(Rgb {
            red: 250,
            green: 250,
            blue: 250,
        }),
    };

    assert!(session.apply(TerminalChange::Theme(light)));

    assert_eq!(session.rendering.theme, ThemeKind::Light);
    assert!(!session.rendering.theme_unknown);
}

#[test]
fn a_forced_theme_survives_a_re_detected_background() {
    let mut session = Session::settle(
        &loaded(ThemeChoice::Dark, RendererChoice::Auto),
        for_tests::capabilities(),
    );
    let light = ThemeHint {
        theme: TerminalTheme::Light,
        source: ThemeSource::Background(Rgb {
            red: 250,
            green: 250,
            blue: 250,
        }),
    };

    session.apply(TerminalChange::Theme(light));

    assert_eq!(session.rendering.theme, ThemeKind::Dark);
}

#[test]
fn config_warnings_are_not_session_notes() {
    // m8-polish-and-ship/09b: they are `AppState::config_problems` now,
    // drawn by `view::status::notes` (sanitised there) and the bottom row.
    let config = Loaded {
        warnings: vec![crate::config::ConfigWarning::UnknownKey {
            path: "ui.\u{1b}[31mtheme".to_owned(),
        }],
        ..Loaded::default()
    };

    let session = Session::settle(&config, for_tests::capabilities());

    assert!(session.notes.is_empty(), "{:?}", session.notes);
}
