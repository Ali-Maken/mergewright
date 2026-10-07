//! His colours over the built-ins: where they reach, and which floors they
//! miss (m8-polish-and-ship/16).

use graft_core::capabilities::{ColorDepth, Rgb};

use super::super::contrast::{ContrastMiss, ContrastRule, contrast};
use super::super::{Theme, ThemeKind, Token, TokenColor, depth};
use super::{contrast_misses, table};
use crate::test_support::terminal;

const EVERY_THEME: [ThemeKind; 3] = [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast];

fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}

fn built_in(kind: ThemeKind, token: Token) -> Rgb {
    table(kind, &[])[token.index()]
}

#[test]
fn from_name_inverts_name_and_knows_no_lanes() {
    for token in Token::ALL {
        assert_eq!(Token::from_name(token.name()), Some(token));
    }
    assert_eq!(Token::from_name("lane-3"), None);
    assert_eq!(Token::from_name("selction"), None);
}

#[test]
fn no_overrides_is_the_built_in_theme_at_every_depth() {
    for kind in EVERY_THEME {
        for depth in [
            ColorDepth::TrueColor,
            ColorDepth::Palette256,
            ColorDepth::Palette16,
        ] {
            let caps = terminal(depth, false);
            assert_eq!(
                Theme::with_overrides(kind, &caps, &[]),
                Theme::new(kind, &caps)
            );
        }
    }
}

#[test]
fn an_overridden_selection_is_the_cursor_rows_fill() {
    let fill = rgb(0x3A, 0x3F, 0x4B);
    let theme = Theme::with_overrides(
        ThemeKind::Dark,
        &terminal(ColorDepth::TrueColor, false),
        &[(Token::Selection, fill)],
    );
    assert_eq!(theme.token(Token::Selection), TokenColor::Rgb(fill));
}

#[test]
fn on_selection_lifts_against_the_override_not_the_built_in() {
    // A light fill on the dark theme: dark text is what reads on it, and the
    // built-in dark selection would have lifted toward white instead.
    let fill = rgb(0xE0, 0xE0, 0xE0);
    let theme = Theme::with_overrides(
        ThemeKind::Dark,
        &terminal(ColorDepth::TrueColor, false),
        &[(Token::Selection, fill)],
    );
    let TokenColor::Rgb(text) = theme.on_selection(theme.token(Token::Text)) else {
        panic!("truecolor gives an Rgb");
    };
    let ratio = contrast(text, fill);
    assert!(ratio >= 4.5, "text on the override is {ratio:.2}:1");
}

#[test]
fn an_overridden_text_muted_reaches_the_pixel_marker() {
    let muted = rgb(0x80, 0x70, 0x60);
    let theme = Theme::with_overrides(
        ThemeKind::Dark,
        &terminal(ColorDepth::TrueColor, false),
        &[(Token::TextMuted, muted)],
    );
    assert_eq!(theme.raster_muted(), muted);
}

#[test]
fn an_overridden_conflict_token_at_256_is_cube_nearest_not_the_pin() {
    let ours = rgb(0xFF, 0x00, 0x00);
    let caps = terminal(ColorDepth::Palette256, false);
    let theme = Theme::with_overrides(ThemeKind::Dark, &caps, &[(Token::Ours, ours)]);
    assert_eq!(
        theme.token(Token::Ours),
        TokenColor::Indexed(depth::nearest_256_cube(ours))
    );
    // Untouched conflict tokens keep their pins, and `ours-tint` is not
    // re-derived from the new `ours`.
    let plain = Theme::new(ThemeKind::Dark, &caps);
    assert_eq!(theme.token(Token::Theirs), plain.token(Token::Theirs));
    assert_eq!(theme.token(Token::OursTint), plain.token(Token::OursTint));
}

#[test]
fn lanes_are_not_a_themes_to_change() {
    let caps = terminal(ColorDepth::TrueColor, false);
    let theme = Theme::with_overrides(ThemeKind::Dark, &caps, &[(Token::Accent, rgb(1, 2, 3))]);
    let plain = Theme::new(ThemeKind::Dark, &caps);
    for lane in 0..12 {
        assert_eq!(theme.lane(lane), plain.lane(lane));
    }
}

#[test]
fn no_color_ignores_overrides() {
    for depth in [ColorDepth::TrueColor, ColorDepth::Palette256] {
        let caps = terminal(depth, true);
        let theme = Theme::with_overrides(
            ThemeKind::Dark,
            &caps,
            &[(Token::Selection, rgb(0xFF, 0, 0))],
        );
        for token in Token::ALL {
            assert_eq!(
                theme.token(token),
                Theme::new(ThemeKind::Dark, &caps).token(token)
            );
        }
    }
}

#[test]
fn built_in_themes_miss_no_floor() {
    for kind in EVERY_THEME {
        assert_eq!(contrast_misses(kind, &[]), Vec::new(), "{kind:?}");
    }
}

#[test]
fn selection_on_bg_misses_one_floor_and_says_so() {
    let bg = built_in(ThemeKind::Dark, Token::Bg);
    let misses = contrast_misses(ThemeKind::Dark, &[(Token::Selection, bg)]);
    let words: Vec<String> = misses.iter().map(ContrastMiss::words).collect();
    assert_eq!(words, ["selection on bg 1.0:1 — under 3.0"]);
}

#[test]
fn a_ratio_is_floored_so_a_miss_never_reads_as_its_floor() {
    let miss = ContrastMiss {
        rule: ContrastRule {
            fg: Token::Selection,
            bg: Token::Bg,
            min: 3.0,
        },
        ratio: 2.96,
    };
    assert_eq!(miss.words(), "selection on bg 2.9:1 — under 3.0");
}
