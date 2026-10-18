//! The floors as numbers (ADR-0058, design note "Dark pick: B"). Colour on
//! screen is the user's eye; these hold the arithmetic.

use mergewright_core::capabilities::{ColorDepth, Rgb};

use super::super::lanes::LANE_HUES;
use super::super::{Theme, ThemeKind, Token, TokenColor, depth};
use super::{CONTRAST_RULES, contrast};
use crate::test_support::terminal;

const EVERY_THEME: [ThemeKind; 3] = [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast];

fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}

fn truecolor(kind: ThemeKind) -> Theme {
    Theme::new(kind, &terminal(ColorDepth::TrueColor, false))
}

fn hue(theme: &Theme, token: Token) -> Rgb {
    match theme.token(token) {
        TokenColor::Rgb(hue) => hue,
        other => panic!("truecolor gives {} an Rgb, got {other:?}", token.name()),
    }
}

#[test]
fn contrast_matches_wcag_on_three_pairs() {
    let white = rgb(0xFF, 0xFF, 0xFF);
    let black = rgb(0, 0, 0);
    assert!((contrast(white, black) - 21.0).abs() < 0.01);
    assert!((contrast(black, white) - 21.0).abs() < 0.01, "symmetric");
    let grey = contrast(rgb(0x77, 0x77, 0x77), white);
    assert!((grey - 4.48).abs() < 0.01, "#777 on white is {grey:.3}");
    let same = rgb(0x5C, 0x66, 0x7B);
    assert!((contrast(same, same) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn every_theme_clears_every_contrast_rule() {
    for kind in EVERY_THEME {
        let theme = truecolor(kind);
        for rule in CONTRAST_RULES {
            let ratio = contrast(hue(&theme, rule.fg), hue(&theme, rule.bg));
            assert!(
                ratio >= rule.min,
                "{}: {} on {} is {ratio:.2}:1, floor {}",
                kind.name(),
                rule.fg.name(),
                rule.bg.name(),
                rule.min
            );
        }
    }
}

/// Dark B's three numbers, as the note measured them (less 0.01 for
/// rounding): `docs/shots/m8-proto-4a-dark-a-b.txt:21-24`.
#[test]
fn dark_b_measures_what_the_note_measured() {
    let theme = truecolor(ThemeKind::Dark);
    let bg = hue(&theme, Token::Bg);
    let selection = hue(&theme, Token::Selection);
    let pairs = [
        ("selection/bg", contrast(selection, bg), 3.13),
        (
            "text/selection",
            contrast(hue(&theme, Token::Text), selection),
            5.13,
        ),
        (
            "text-muted/bg",
            contrast(hue(&theme, Token::TextMuted), bg),
            6.94,
        ),
    ];
    for (name, ratio, floor) in pairs {
        assert!(ratio >= floor, "{name} is {ratio:.3}:1, wanted ≥ {floor}");
    }
}

fn dominant(color: Rgb) -> usize {
    let channels = [color.red, color.green, color.blue];
    (0..3).max_by_key(|&i| channels[i]).expect("three channels")
}

/// Every lane hue — and so every branch colour, which reads the same twelve
/// (`Theme::branch`) — on each theme's `selection` reaches 4.5:1 after the
/// lift, and its strongest channel is still its strongest: a lifted blue is
/// still blue.
#[test]
fn legible_on_lifts_every_lane_onto_every_selection() {
    for kind in EVERY_THEME {
        let theme = truecolor(kind);
        let selection = hue(&theme, Token::Selection);
        for (lane, hue) in LANE_HUES.iter().copied().enumerate() {
            let lifted = theme.legible_on(hue, selection);
            let ratio = contrast(lifted, selection);
            assert!(
                ratio >= 4.5,
                "{} lane {lane}: {ratio:.2}:1 after the lift",
                kind.name()
            );
            let channels = [lifted.red, lifted.green, lifted.blue];
            let was = dominant(hue);
            assert!(
                channels.iter().all(|&c| channels[was] >= c),
                "{} lane {lane}: {hue:?} → {lifted:?} lost its dominant channel",
                kind.name()
            );
        }
    }
}

#[test]
fn legible_on_leaves_a_legible_colour_alone() {
    let theme = truecolor(ThemeKind::Dark);
    let selection = hue(&theme, Token::Selection);
    let text = hue(&theme, Token::Text);
    assert_eq!(theme.legible_on(text, selection), text);
}

/// The lift heads for black on a light fill: light theme's darkest lane
/// result is darker than the hue it started from.
#[test]
fn legible_on_heads_for_black_on_the_light_fill() {
    let theme = truecolor(ThemeKind::Light);
    let selection = hue(&theme, Token::Selection);
    let lane = LANE_HUES[1];
    let lifted = theme.legible_on(lane, selection);
    assert_ne!(lifted, lane, "lane 1 needed lifting on light selection");
    assert!(lifted.red <= lane.red && lifted.green <= lane.green && lifted.blue <= lane.blue);
}

#[test]
fn on_selection_at_truecolor_is_legible_on() {
    let theme = truecolor(ThemeKind::Dark);
    let selection = hue(&theme, Token::Selection);
    for (lane, hue) in LANE_HUES.iter().enumerate() {
        let TokenColor::Rgb(drawn) = theme.on_selection(theme.lane(lane)) else {
            panic!("truecolor stays Rgb");
        };
        assert_eq!(drawn, theme.legible_on(*hue, selection));
    }
}

/// At 256 colours the lift is judged after reduction: the index drawn, as
/// xterm shows it, against the index `selection` reduces to.
#[test]
fn on_selection_at_256_clears_after_reduction_for_every_lane() {
    for kind in EVERY_THEME {
        let theme = Theme::new(kind, &terminal(ColorDepth::Palette256, false));
        let TokenColor::Indexed(fill) = theme.token(Token::Selection) else {
            panic!("256 colours give an index");
        };
        let fill = depth::palette_entry(fill);
        for lane in 0..LANE_HUES.len() {
            let TokenColor::Indexed(drawn) = theme.on_selection(theme.lane(lane)) else {
                panic!("256 colours stay indexed");
            };
            let ratio = contrast(depth::palette_entry(drawn), fill);
            assert!(
                ratio >= 4.5,
                "{} lane {lane} at 256: index {drawn} is {ratio:.2}:1",
                kind.name()
            );
        }
    }
}

#[test]
fn on_selection_without_a_fill_is_text() {
    for (depth, no_color) in [
        (ColorDepth::Palette16, false),
        (ColorDepth::TrueColor, true),
    ] {
        let theme = Theme::new(ThemeKind::Dark, &terminal(depth, no_color));
        for lane in 0..LANE_HUES.len() {
            assert_eq!(
                theme.on_selection(theme.lane(lane)),
                theme.token(Token::Text),
                "{depth:?} no_color={no_color}"
            );
        }
    }
}
