//! What the theme system has to keep true.
//!
//! Three of these tests are measurements rather than examples — lane
//! separation under simulated colour blindness, chrome contrast against every
//! surface, and lane distinctness after the 256-colour mapping. They exist
//! because the interesting failures here are not crashes: a palette tweak that
//! drops a lane pair below the distinguishable line, or a "nicer" muted grey
//! that falls under the contrast floor, would otherwise ship looking fine.

use graft_core::capabilities::{ColorDepth, Rgb, Theme as TerminalTheme, ThemeHint, ThemeSource};

use crate::test_support::terminal;

use super::depth::{Oklab, linearize, nearest_16, nearest_256, oklab, oklab_from_linear};
use super::lanes::LANE_HUES;
use super::{LANE_COUNT, Theme, ThemeKind, Token, TokenColor};

const EVERY_THEME: [ThemeKind; 3] = [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast];

fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}

// ---------------------------------------------------------------- tokens

#[test]
fn every_token_prompt_md_names_is_present_exactly_once() {
    let names: Vec<&str> = Token::ALL.iter().map(|token| token.name()).collect();

    assert_eq!(
        names,
        [
            "bg",
            "surface",
            "surface-raised",
            "border",
            "text",
            "text-muted",
            "accent",
            "added",
            "removed",
            "warning",
            "info",
        ],
        "the token set is PROMPT.md §5's list, in its order"
    );
}

#[test]
fn token_index_matches_position_in_all() {
    for (position, token) in Token::ALL.iter().enumerate() {
        assert_eq!(
            token.index(),
            position,
            "{} indexes a different theme-table slot than its position in Token::ALL, \
             so every theme table is transposed",
            token.name()
        );
    }
}

#[test]
fn there_are_twelve_lanes() {
    assert_eq!(LANE_HUES.len(), LANE_COUNT);
    assert_eq!(LANE_COUNT, 12, "PROMPT.md §5 names lane-1 through lane-12");
}

/// The resolved value of every token, in every theme, at every colour depth.
#[test]
fn resolved_values() {
    let mut rendered = String::new();

    for kind in EVERY_THEME {
        rendered.push_str(&format!("theme {}\n", kind.name()));
        rendered.push_str("  token           truecolor   256   16\n");

        let themes: Vec<Theme> = [
            ColorDepth::TrueColor,
            ColorDepth::Palette256,
            ColorDepth::Palette16,
        ]
        .into_iter()
        .map(|depth| Theme::new(kind, &terminal(depth, false)))
        .collect();

        for token in Token::ALL {
            rendered.push_str(&row(token.name(), |theme| theme.token(token), &themes));
        }
        for lane in 0..LANE_COUNT {
            rendered.push_str(&row(
                &format!("lane-{}", lane + 1),
                |theme| theme.lane(lane),
                &themes,
            ));
        }
        rendered.push('\n');
    }

    insta::assert_snapshot!(rendered);
}

fn row(name: &str, color: impl Fn(&Theme) -> TokenColor, themes: &[Theme]) -> String {
    let cells: Vec<String> = themes.iter().map(|theme| show(color(theme))).collect();
    format!(
        "  {:<15} {:<11} {:<5} {}\n",
        name, cells[0], cells[1], cells[2]
    )
}

fn show(color: TokenColor) -> String {
    match color {
        TokenColor::Default => "default".to_owned(),
        TokenColor::Rgb(hue) => format!("#{:02X}{:02X}{:02X}", hue.red, hue.green, hue.blue),
        TokenColor::Indexed(index) => index.to_string(),
    }
}

// ------------------------------------------------------------- selection

#[test]
fn an_override_beats_the_terminals_hint() {
    let measured_light = ThemeHint {
        theme: TerminalTheme::Light,
        source: ThemeSource::Background(rgb(0xFF, 0xFF, 0xFF)),
    };

    assert_eq!(
        ThemeKind::select(Some(ThemeKind::Dark), measured_light),
        ThemeKind::Dark
    );
    assert_eq!(
        ThemeKind::select(Some(ThemeKind::HighContrast), measured_light),
        ThemeKind::HighContrast
    );
}

#[test]
fn a_measured_background_picks_the_matching_theme() {
    let dark = ThemeHint {
        theme: TerminalTheme::Dark,
        source: ThemeSource::Background(rgb(0x10, 0x10, 0x14)),
    };
    let light = ThemeHint {
        theme: TerminalTheme::Light,
        source: ThemeSource::Background(rgb(0xFA, 0xFA, 0xF8)),
    };

    assert_eq!(ThemeKind::select(None, dark), ThemeKind::Dark);
    assert_eq!(ThemeKind::select(None, light), ThemeKind::Light);
}

#[test]
fn an_unknown_hint_is_dark_whatever_it_claims() {
    // `Assumed` means OSC 11 said nothing usable. `theme` is then a default,
    // not a reading — so it must not steer the choice, even set to Light.
    for claimed in [TerminalTheme::Dark, TerminalTheme::Light] {
        let unknown = ThemeHint {
            theme: claimed,
            source: ThemeSource::Assumed,
        };

        assert_eq!(ThemeKind::select(None, unknown), ThemeKind::Dark);
    }
}

// ------------------------------------------------------------ lane rules

#[test]
fn no_theme_can_change_a_lane_hue() {
    let terminal = terminal(ColorDepth::TrueColor, false);
    let reference = Theme::new(ThemeKind::Dark, &terminal);

    for kind in EVERY_THEME {
        let theme = Theme::new(kind, &terminal);

        for lane in 0..LANE_COUNT {
            assert_eq!(
                theme.lane(lane),
                reference.lane(lane),
                "theme {} changed lane {lane}; PROMPT.md §5 makes lanes theme-independent",
                kind.name()
            );
        }
    }
}

#[test]
fn lanes_past_the_palette_reuse_it_from_the_start() {
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false));

    assert_eq!(theme.lane(LANE_COUNT), theme.lane(0));
    assert_eq!(theme.lane(LANE_COUNT * 3 + 5), theme.lane(5));
}

#[test]
fn adjacent_lanes_stay_apart_under_every_vision_model() {
    // The floor the palette was selected against (lanes.rs). Adjacent lanes
    // are drawn side by side, so they get the strict threshold.
    const ADJACENT_FLOOR: f32 = 0.18;
    // Any two lanes anywhere, including the wrap from lane-12 back to lane-1.
    const ANY_PAIR_FLOOR: f32 = 0.055;

    let mut worst_adjacent = f32::INFINITY;
    let mut worst_pair = f32::INFINITY;

    for (first, &left) in LANE_HUES.iter().enumerate() {
        for (second, &right) in LANE_HUES.iter().enumerate().skip(first + 1) {
            let distance = worst_case_distance(left, right);
            worst_pair = worst_pair.min(distance);

            if second == first + 1 {
                worst_adjacent = worst_adjacent.min(distance);
            }
        }
    }

    assert!(
        worst_adjacent >= ADJACENT_FLOOR,
        "adjacent lanes fell to ΔE {worst_adjacent:.4} under the worst vision model \
         (floor {ADJACENT_FLOOR})"
    );
    assert!(
        worst_pair >= ANY_PAIR_FLOOR,
        "two lanes fell to ΔE {worst_pair:.4} under the worst vision model \
         (floor {ANY_PAIR_FLOOR})"
    );
}

/// OKLab distance between two colours under the worst of normal vision,
/// protanopia, deuteranopia and tritanopia.
///
/// The three deficiencies are simulated with Machado, Oliveira and Fernandes
/// (2009) severity-1.0 matrices, applied in linear-light sRGB and clamped back
/// into gamut, since a display cannot emit the negative light they can ask
/// for. A simulation is
/// not a user: it says two colours are *numerically* separable, not that
/// someone can tell them apart on a terminal. M1's visual pass is what
/// confirms that (KNOWN_GAPS.md).
fn worst_case_distance(left: Rgb, right: Rgb) -> f32 {
    const VISION: [[f32; 9]; 4] = [
        // Normal vision: identity.
        [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        // Protanopia.
        [
            0.152_286, 1.052_583, -0.204_868, 0.114_503, 0.786_281, 0.099_216, -0.003_882,
            -0.048_116, 1.051_998,
        ],
        // Deuteranopia.
        [
            0.367_322, 0.860_646, -0.227_968, 0.280_085, 0.672_501, 0.047_413, -0.011_820,
            0.042_940, 0.968_881,
        ],
        // Tritanopia.
        [
            1.255_528, -0.076_749, -0.178_779, -0.078_411, 0.930_809, 0.147_602, 0.004_733,
            0.691_367, 0.303_900,
        ],
    ];

    VISION
        .iter()
        .map(|matrix| as_seen(left, matrix).distance(as_seen(right, matrix)))
        .fold(f32::INFINITY, f32::min)
}

fn as_seen(color: Rgb, matrix: &[f32; 9]) -> Oklab {
    let channels = [
        linearize(color.red),
        linearize(color.green),
        linearize(color.blue),
    ];
    let mix = |row: usize| {
        (0..3)
            .map(|column| matrix[row * 3 + column] * channels[column])
            .sum::<f32>()
            .clamp(0.0, 1.0)
    };

    oklab_from_linear(mix(0), mix(1), mix(2))
}

#[test]
fn the_twelve_lanes_survive_the_256_colour_mapping() {
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::Palette256, false));

    let mut seen = Vec::new();
    for lane in 0..LANE_COUNT {
        let TokenColor::Indexed(index) = theme.lane(lane) else {
            panic!("a 256-colour terminal must give lane {lane} a palette index");
        };

        assert!(
            index >= 16,
            "lane {lane} landed on redefinable index {index}"
        );
        assert!(
            !seen.contains(&index),
            "lanes collapsed onto palette index {index} at 256 colours"
        );
        seen.push(index);
    }
}

// ------------------------------------------------------------- no colour

#[test]
fn no_color_empties_every_token_in_every_theme() {
    for kind in EVERY_THEME {
        // `NO_COLOR` is about colour, not capability: the terminal here is a
        // full truecolor one (DESIGN.md §4.6).
        let theme = Theme::new(kind, &terminal(ColorDepth::TrueColor, true));

        for token in Token::ALL {
            assert_eq!(
                theme.token(token),
                TokenColor::Default,
                "NO_COLOR left {} coloured in theme {}",
                token.name(),
                kind.name()
            );
        }

        for lane in 0..LANE_COUNT {
            assert_eq!(
                theme.lane(lane),
                TokenColor::Default,
                "NO_COLOR left lane {lane} coloured in theme {}",
                kind.name()
            );
        }
    }
}

#[test]
fn a_colourless_terminal_is_empty_even_without_no_color() {
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::None, false));

    assert_eq!(theme.token(Token::Text), TokenColor::Default);
    assert_eq!(theme.lane(0), TokenColor::Default);
}

// -------------------------------------------------------------- contrast

#[test]
fn readable_tokens_clear_the_contrast_floor_on_every_surface() {
    // WCAG 2.1 AA for body text. `border` is excluded: it draws rules, not
    // text, and PROMPT.md §5 asks for thin, quiet borders (chrome.rs).
    const FLOOR: f32 = 4.5;
    const SURFACES: [Token; 3] = [Token::Bg, Token::Surface, Token::SurfaceRaised];
    const READABLE: [Token; 6] = [
        Token::Text,
        Token::TextMuted,
        Token::Accent,
        Token::Added,
        Token::Removed,
        Token::Warning,
    ];

    for kind in EVERY_THEME {
        let theme = Theme::new(kind, &terminal(ColorDepth::TrueColor, false));

        for surface in SURFACES {
            for token in READABLE {
                let ratio = contrast(hue(&theme, token), hue(&theme, surface));

                assert!(
                    ratio >= FLOOR,
                    "{} on {} is {ratio:.2}:1 in theme {} (floor {FLOOR})",
                    token.name(),
                    surface.name(),
                    kind.name()
                );
            }
        }
    }
}

fn hue(theme: &Theme, token: Token) -> Rgb {
    match theme.token(token) {
        TokenColor::Rgb(hue) => hue,
        other => panic!(
            "a truecolor terminal must give {} an Rgb, got {other:?}",
            token.name()
        ),
    }
}

/// WCAG 2.1 contrast ratio, from relative luminance.
fn contrast(left: Rgb, right: Rgb) -> f32 {
    let luminance = |color: Rgb| {
        0.2126 * linearize(color.red)
            + 0.7152 * linearize(color.green)
            + 0.0722 * linearize(color.blue)
    };

    let (lighter, darker) = {
        let (left, right) = (luminance(left), luminance(right));
        if left > right {
            (left, right)
        } else {
            (right, left)
        }
    };

    (lighter + 0.05) / (darker + 0.05)
}

// ---------------------------------------------------------- depth mapping

#[test]
fn known_colours_map_to_their_exact_palette_entries() {
    // Both palettes contain these exactly, so "nearest" has a right answer.
    assert_eq!(
        nearest_256(rgb(0x00, 0x00, 0x00)),
        16,
        "black is the cube's origin"
    );
    assert_eq!(
        nearest_256(rgb(0xFF, 0xFF, 0xFF)),
        231,
        "white is the cube's far corner"
    );
    assert_eq!(nearest_256(rgb(0xFF, 0x00, 0x00)), 196, "pure red");
    assert_eq!(nearest_256(rgb(0x00, 0x00, 0xFF)), 21, "pure blue");
    assert_eq!(
        nearest_256(rgb(0x80, 0x80, 0x80)),
        244,
        "an exact step of the grey ramp"
    );

    assert_eq!(nearest_16(rgb(0x00, 0x00, 0x00)), 0);
    assert_eq!(nearest_16(rgb(0xFF, 0x00, 0x00)), 9);
    assert_eq!(nearest_16(rgb(0xFF, 0xFF, 0xFF)), 15);
}

#[test]
fn the_mapping_never_lands_on_a_redefinable_index_at_256_colours() {
    for red in [0x00, 0x40, 0x80, 0xC0, 0xFF] {
        for green in [0x00, 0x40, 0x80, 0xC0, 0xFF] {
            for blue in [0x00, 0x40, 0x80, 0xC0, 0xFF] {
                let index = nearest_256(rgb(red, green, blue));

                assert!(
                    index >= 16,
                    "#{red:02X}{green:02X}{blue:02X} mapped to index {index}, \
                     which the user's terminal is free to redefine"
                );
            }
        }
    }
}

#[test]
fn near_neutral_chrome_does_not_pick_up_a_hue_at_sixteen_colours() {
    // The 16-colour palette has four points on the grey axis and twelve
    // saturated ones, all of them far from any chrome colour. A grey that
    // lands on navy because navy's lightness happened to be closer is the
    // failure depth.rs's chroma weighting exists to prevent.
    const NEUTRAL: f32 = 0.03;
    const GREY_AXIS: [u8; 4] = [0, 7, 8, 15];

    for kind in EVERY_THEME {
        let truecolor = Theme::new(kind, &terminal(ColorDepth::TrueColor, false));
        let sixteen = Theme::new(kind, &terminal(ColorDepth::Palette16, false));

        for token in Token::ALL {
            if oklab(hue(&truecolor, token)).chroma() > NEUTRAL {
                continue;
            }

            let TokenColor::Indexed(index) = sixteen.token(token) else {
                panic!("a 16-colour terminal must give {} an index", token.name());
            };

            assert!(
                GREY_AXIS.contains(&index),
                "{} is neutral in theme {} but became palette index {index} at 16 colours",
                token.name(),
                kind.name()
            );
        }
    }
}

#[test]
fn perceptual_distance_beats_per_channel_rounding() {
    // #303030 sits one unit nearer cube level 95 than level 0 on every
    // channel, so rounding each channel independently gives #5F5F5F — a
    // visibly lighter grey. The grey ramp holds #303030 exactly.
    let dark_grey = rgb(0x30, 0x30, 0x30);

    assert_eq!(
        naive_cube_index(dark_grey),
        59,
        "the mistake this test guards against"
    );
    assert_eq!(
        nearest_256(dark_grey),
        236,
        "grey ramp entry 4, which is #303030 exactly"
    );
}

/// Per-channel rounding onto the 6×6×6 cube: what DESIGN.md §4.6 rules out.
fn naive_cube_index(color: Rgb) -> u8 {
    const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

    let level = |channel: u8| {
        CUBE_LEVELS
            .iter()
            .enumerate()
            .min_by_key(|&(_, &level)| i32::from(channel).abs_diff(i32::from(level)))
            .map(|(index, _)| index as u8)
            .unwrap_or(0)
    };

    16 + 36 * level(color.red) + 6 * level(color.green) + level(color.blue)
}
