//! `[theme.<kind>]` as the file says it, and the problems it raises
//! (m8-polish-and-ship/16).

use mergewright_core::capabilities::Rgb;
use mergewright_ui::theme::{Token, TokenColor};

use super::tests::loaded_from_a_file;
use super::*;

fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}

#[test]
fn j10_one_selection_line_loads_as_one_override() {
    let loaded = loaded_from_a_file("[theme.dark]\nselection = \"#3A3F4B\"\n");
    assert_eq!(
        loaded.config.theme.dark,
        [(Token::Selection, rgb(0x3A, 0x3F, 0x4B))]
    );
    assert!(loaded.config.theme.light.is_empty());
    assert_eq!(loaded.warnings, []);
}

#[test]
fn valid_keys_in_every_kind_say_nothing_and_hex_takes_either_case() {
    let loaded = loaded_from_a_file(
        "[theme.dark]\naccent = \"#abcdef\"\ntext = \"#ABCDEF\"\n\
         [theme.light]\nbg = \"#ffffff\"\n\
         [theme.high-contrast]\nborder = \"#808080\"\n\
         [theme.high_contrast]\ninfo = \"#00ffff\"\n",
    );
    assert_eq!(loaded.warnings, []);
    let hue = rgb(0xAB, 0xCD, 0xEF);
    assert_eq!(
        loaded.config.theme.dark,
        [(Token::Accent, hue), (Token::Text, hue)]
    );
    assert_eq!(loaded.config.theme.light, [(Token::Bg, rgb(255, 255, 255))]);
    assert_eq!(loaded.config.theme.high_contrast.len(), 2);
}

#[test]
fn a_lane_colour_is_refused_and_lanes_stay_put() {
    let loaded = loaded_from_a_file("[theme.dark]\nlane-3 = \"#FF0000\"\n");
    assert_eq!(
        loaded.warnings,
        [ConfigWarning::LaneColour {
            path: "theme.dark.lane-3".to_owned()
        }]
    );
    assert_eq!(
        loaded.warnings[0].to_string(),
        "theme.dark.lane-3: lane colours are fixed, not themeable"
    );
    assert!(loaded.config.theme.dark.is_empty());
    let caps = crate::session::for_tests::capabilities();
    let kind = mergewright_ui::theme::ThemeKind::Dark;
    let theme =
        mergewright_ui::theme::Theme::with_overrides(kind, &caps, loaded.config.theme.for_kind(kind));
    assert_eq!(
        theme.lane(3),
        mergewright_ui::theme::Theme::new(kind, &caps).lane(3)
    );
}

#[test]
fn a_colour_that_is_not_hex_is_a_bad_colour() {
    let loaded =
        loaded_from_a_file("[theme.dark]\nselection = \"red\"\naccent = \"#12345\"\ntext = 3\n");
    assert_eq!(
        loaded.warnings,
        [
            ConfigWarning::BadColour {
                path: "theme.dark.accent".to_owned(),
                value: "#12345".to_owned()
            },
            ConfigWarning::BadColour {
                path: "theme.dark.selection".to_owned(),
                value: "red".to_owned()
            },
            ConfigWarning::BadColour {
                path: "theme.dark.text".to_owned(),
                value: "3".to_owned()
            },
        ]
    );
    assert_eq!(
        loaded.warnings[1].to_string(),
        "theme.dark.selection: \"red\" is not a #RRGGBB colour"
    );
    assert!(loaded.config.theme.dark.is_empty());
}

#[test]
fn an_unknown_kind_or_token_is_an_unknown_key() {
    let loaded = loaded_from_a_file(
        "[theme.solarized]\nbg = \"#000000\"\n[theme.dark]\nselction = \"#000000\"\n",
    );
    assert_eq!(
        loaded.warnings,
        [
            ConfigWarning::UnknownKey {
                path: "theme.dark.selction".to_owned()
            },
            ConfigWarning::UnknownKey {
                path: "theme.solarized".to_owned()
            },
        ]
    );
}

#[test]
fn theme_that_is_not_a_table_is_unknown_as_before() {
    let loaded = loaded_from_a_file("theme = \"dark\"\n");
    assert_eq!(
        loaded.warnings,
        [ConfigWarning::UnknownKey {
            path: "theme".to_owned()
        }]
    );
}

#[test]
fn selection_on_the_built_in_bg_is_one_contrast_problem() {
    let bg = built_in_dark_bg();
    let overrides = ThemeOverrides {
        dark: vec![(Token::Selection, bg)],
        ..ThemeOverrides::default()
    };
    assert_eq!(
        theme_problems(&overrides, false),
        ["[theme.dark] selection on bg 1.0:1 — under 3.0"]
    );
}

#[test]
fn a_light_miss_is_named_whatever_theme_runs() {
    // The session's theme is not an input: all three tables are checked.
    let white = rgb(0xFF, 0xFF, 0xFF);
    let overrides = ThemeOverrides {
        light: vec![(Token::Text, white), (Token::Bg, white)],
        ..ThemeOverrides::default()
    };
    let problems = theme_problems(&overrides, false);
    assert!(
        problems
            .iter()
            .all(|problem| problem.starts_with("[theme.light] ")),
        "{problems:?}"
    );
    assert!(
        problems.contains(&"[theme.light] text on bg 1.0:1 — under 4.5".to_owned()),
        "{problems:?}"
    );
}

#[test]
fn no_color_reports_no_contrast_miss() {
    let overrides = ThemeOverrides {
        dark: vec![(Token::Selection, built_in_dark_bg())],
        ..ThemeOverrides::default()
    };
    assert_eq!(theme_problems(&overrides, true), Vec::<String>::new());
}

/// The dark theme's `bg`, read from the theme itself — no literal here.
pub(crate) fn built_in_dark_bg() -> Rgb {
    let theme = mergewright_ui::theme::Theme::new(
        mergewright_ui::theme::ThemeKind::Dark,
        &crate::session::for_tests::capabilities(),
    );
    match theme.token(Token::Bg) {
        TokenColor::Rgb(hue) => hue,
        other => panic!("truecolor bg is an Rgb, got {other:?}"),
    }
}
