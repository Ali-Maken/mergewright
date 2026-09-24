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
use super::{LANE_COUNT, TINT_ALPHA, Theme, ThemeKind, Token, TokenColor, mix};

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
            "selection",
            "border",
            "text",
            "text-muted",
            "accent",
            "added",
            "removed",
            "warning",
            "info",
            "ours",
            "theirs",
            "open",
            "ours-tint",
            "theirs-tint",
            "open-tint",
        ],
        "the token set is PROMPT.md §5's list, in its order \
         (`selection` added m5-history-and-ghost-preview/44)"
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

#[test]
fn color_depth_reports_what_the_terminal_can_show() {
    let truecolor = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false));
    let sixteen = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::Palette16, false));

    assert_eq!(truecolor.color_depth(), ColorDepth::TrueColor);
    assert_eq!(sixteen.color_depth(), ColorDepth::Palette16);

    // `NO_COLOR` overrides the terminal's own depth, same as every token
    // (`no_color_empties_every_token_in_every_theme`): a widget asking
    // `color_depth()` for its own separator logic sees the reduction too.
    let no_color = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::Palette16, true));
    assert_eq!(no_color.color_depth(), ColorDepth::None);
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

/// m5-history-and-ghost-preview/44. `selection` is the "you are here" fill —
/// the cursor row in the graph, the map, the stash list, the journal, and
/// the files pane. It replaced `surface-raised` there because `surface-raised`
/// is tuned for a *quiet* step (a raised panel's background) and measured
/// only ≈1.1:1–1.3:1 against `bg`: far under the WCAG 1.4.11 floor for a
/// non-text indicator (3:1), in both themes, and the reason the user could
/// not find his own cursor row twice.
///
/// `text` drawn on the selected row must also stay readable (WCAG 2.1 AA,
/// 4.5:1) — "a selection you can see but cannot read on is not a fix". In
/// the dark theme these two floors are **not simultaneously reachable**: `bg`
/// (#14161B, relative luminance 0.00802) and `text` (#D7DBE0, luminance
/// 0.70492) are only 13.01:1 apart, and splitting that into a ≥3:1 segment
/// below `selection` and a ≥4.5:1 segment above it needs ≥13.5:1 of total
/// room — proven by the arithmetic, not a tuning miss. `selection` here
/// (#5A6478) was chosen to clear the reported defect's floor exactly (3:1
/// against `bg`) and get `text` as close to 4.5:1 as that leaves room for
/// (4.28:1). The light and high-contrast themes have enough room for both
/// floors at once, and clear them.
#[test]
fn selection_clears_the_reported_defect_and_text_stays_close_to_readable() {
    // The floor this replaces: WCAG 1.4.11, a non-text UI indicator against
    // its background.
    const BG_FLOOR: f32 = 3.0;
    // The dark theme cannot reach this at the same time as `BG_FLOOR` (see
    // the derivation above); its achieved ratio is asserted explicitly
    // instead of silently lowering the floor for every theme.
    const TEXT_FLOOR: f32 = 4.5;

    for kind in EVERY_THEME {
        let theme = Theme::new(kind, &terminal(ColorDepth::TrueColor, false));
        let selection = hue(&theme, Token::Selection);
        let bg = hue(&theme, Token::Bg);
        let text = hue(&theme, Token::Text);

        let bg_ratio = contrast(selection, bg);
        assert!(
            bg_ratio >= BG_FLOOR,
            "selection on bg is {bg_ratio:.2}:1 in theme {} (floor {BG_FLOOR}); \
             this is the ratio the user could not see his cursor row at",
            kind.name()
        );

        let text_ratio = contrast(text, selection);
        let expected_text_ratio = match kind {
            // Proven unreachable together with `BG_FLOOR` above; the real,
            // measured number is asserted so a further regression still
            // fails loudly even though it is already short of `TEXT_FLOOR`.
            ThemeKind::Dark => 4.28,
            ThemeKind::Light => 4.71,
            ThemeKind::HighContrast => 5.49,
        };
        assert!(
            (text_ratio - expected_text_ratio).abs() < 0.05,
            "text on selection is {text_ratio:.2}:1 in theme {}, expected ≈{expected_text_ratio:.2}:1; \
             a token elsewhere moved",
            kind.name()
        );
        if kind != ThemeKind::Dark {
            assert!(
                text_ratio >= TEXT_FLOOR,
                "text on selection is {text_ratio:.2}:1 in theme {} (floor {TEXT_FLOOR})",
                kind.name()
            );
        }
    }
}

/// Same test, against `surface-raised` — the token this replaced. It must
/// fail: that failure is the defect the user reported twice.
///
/// Run standalone (`cargo test -p graft-ui selection_fails_against_the_old_token -- --nocapture`)
/// against `Token::SurfaceRaised` before `Token::Selection` existed: it
/// printed `bg is 1.21:1 in theme dark` and `bg is 1.17:1 in theme light`
/// (see NOTES in m5-history-and-ghost-preview/44 for the run).
#[test]
fn selection_fails_against_the_old_token() {
    const BG_FLOOR: f32 = 3.0;

    for kind in [ThemeKind::Dark, ThemeKind::Light] {
        let theme = Theme::new(kind, &terminal(ColorDepth::TrueColor, false));
        let raised = hue(&theme, Token::SurfaceRaised);
        let bg = hue(&theme, Token::Bg);

        let ratio = contrast(raised, bg);
        assert!(
            ratio < BG_FLOOR,
            "surface-raised on bg is {ratio:.2}:1 in theme {} — if this ever \
             clears {BG_FLOOR}, `selection` no longer needs to exist as a \
             separate token and this test (not `selection`'s) should be deleted",
            kind.name()
        );
    }
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
            // The tints are backgrounds, dropped at 16 colours (ADR-0044 §1:
            // 1.5–2.5:1 against bg), not colours reduced here.
            if token.is_tint() {
                continue;
            }
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
            .map_or(0, |(index, _)| {
                u8::try_from(index).expect("CUBE_LEVELS has six entries")
            })
    };

    16 + 36 * level(color.red) + 6 * level(color.green) + level(color.blue)
}

// ---------------------------------------------------------------- pixels

#[test]
fn raster_lanes_are_truecolor_hues_at_any_depth_and_one_muted_hue_without_colour() {
    for depth in [
        ColorDepth::TrueColor,
        ColorDepth::Palette256,
        ColorDepth::Palette16,
    ] {
        let theme = Theme::new(ThemeKind::Dark, &terminal(depth, false));
        assert_eq!(theme.raster_lanes(), LANE_HUES.to_vec(), "{depth:?}");
    }
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, true));
    assert_eq!(theme.raster_lanes(), vec![theme.raster_muted()]);
    assert_eq!(theme.raster_muted(), rgb(0x8A, 0x91, 0x9C));
}

// ------------------------------------------------------ conflict colours

const HUES: [Token; 3] = [Token::Ours, Token::Theirs, Token::Open];
const TINTS: [(Token, Token); 3] = [
    (Token::OursTint, Token::Ours),
    (Token::TheirsTint, Token::Theirs),
    (Token::OpenTint, Token::Open),
];

/// ADR-0044 §1 and ticket 54 decision 1: the six conflict tokens at every
/// depth, in the dark and high-contrast themes. A tint where tints are dropped
/// is the no-colour variant, and `tint` answers `None` there.
#[test]
fn conflict_tokens_resolve_per_depth() {
    use TokenColor::{Default as No, Indexed as I, Rgb as C};
    let hues = [
        (Token::Ours, rgb(0x56, 0xB4, 0xE9), 74, 6),
        (Token::Theirs, rgb(0xCC, 0x79, 0xA7), 175, 5),
        (Token::Open, rgb(0xE6, 0x9F, 0x00), 214, 3),
    ];
    let tints = [
        (Token::OursTint, rgb(0x28, 0x45, 0x59), 23),
        (Token::TheirsTint, rgb(0x4B, 0x34, 0x45), 53),
        (Token::OpenTint, rgb(0x53, 0x3F, 0x13), 58),
    ];

    for kind in [ThemeKind::Dark, ThemeKind::HighContrast] {
        let themes = depths(kind);
        let n = kind.name();

        for (token, truecolor, pin256, pin16) in hues {
            let want = [C(truecolor), I(pin256), I(pin16), No];
            let got = themes.each_ref().map(|theme| theme.token(token));
            assert_eq!(got, want, "{n} {}", token.name());
        }

        for (token, truecolor, pin256) in tints {
            let dark = kind == ThemeKind::Dark;
            let want = if dark {
                [C(truecolor), I(pin256), No, No]
            } else {
                [No; 4]
            };
            let want_tint = want.map(|color| (dark && color != No).then_some(color));
            let got = themes.each_ref().map(|theme| theme.token(token));
            let got_tint = themes.each_ref().map(|theme| theme.tint(token));
            assert_eq!(got, want, "{n} {}", token.name());
            assert_eq!(got_tint, want_tint, "{n} {}", token.name());
        }
    }
}

/// Truecolor, 256, 16, then `NO_COLOR` on a truecolor terminal.
fn depths(kind: ThemeKind) -> [Theme; 4] {
    [
        (ColorDepth::TrueColor, false),
        (ColorDepth::Palette256, false),
        (ColorDepth::Palette16, false),
        (ColorDepth::TrueColor, true),
    ]
    .map(|(depth, no_color)| Theme::new(kind, &terminal(depth, no_color)))
}

fn assert_tints_follow_the_mix_rule(kind: ThemeKind) {
    let theme = Theme::new(kind, &terminal(ColorDepth::TrueColor, false));
    let bg = hue(&theme, Token::Bg);
    for (tint, of) in TINTS {
        let want = mix(hue(&theme, of), bg, TINT_ALPHA);
        let got = hue(&theme, tint);
        for (a, b) in [
            (got.red, want.red),
            (got.green, want.green),
            (got.blue, want.blue),
        ] {
            assert!(
                a.abs_diff(b) <= 1,
                "{} in {} is {got:?}, mix gives {want:?}",
                tint.name(),
                kind.name()
            );
        }
    }
}

#[test]
fn dark_tints_equal_the_mix_rule() {
    assert_tints_follow_the_mix_rule(ThemeKind::Dark);
}

#[test]
fn light_tints_equal_the_mix_rule() {
    assert_tints_follow_the_mix_rule(ThemeKind::Light);
}

/// ADR-0044 §1: a tint is a quiet background, 1.5:1–2.5:1 against `bg`, below
/// `selection` (the "you are here" fill), and `text` stays readable on it.
#[test]
fn dark_tints_meet_their_contrast_targets() {
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false));
    let bg = hue(&theme, Token::Bg);
    let selection = contrast(hue(&theme, Token::Selection), bg);
    for (tint, _) in TINTS {
        let fill = hue(&theme, tint);
        let against_bg = contrast(fill, bg);
        let text = contrast(hue(&theme, Token::Text), fill);
        assert!(
            (1.5..=2.5).contains(&against_bg),
            "{} is {against_bg:.2}:1 on bg",
            tint.name()
        );
        assert!(
            against_bg < selection,
            "{} outshouts selection",
            tint.name()
        );
        assert!(text >= 4.5, "text on {} is {text:.2}:1", tint.name());
    }
}

#[test]
fn high_contrast_hues_clear_4_5_on_bg() {
    let theme = Theme::new(
        ThemeKind::HighContrast,
        &terminal(ColorDepth::TrueColor, false),
    );
    let bg = hue(&theme, Token::Bg);
    for token in HUES {
        let ratio = contrast(hue(&theme, token), bg);
        assert!(ratio >= 4.5, "{} is {ratio:.2}:1", token.name());
    }
}

/// ADR-0044 §1: the grey ramp would lose a tint's hue. The high-contrast theme's tints
/// resolve to no colour, which is not on the ramp either.
#[test]
fn no_conflict_token_maps_to_the_grey_ramp() {
    for kind in EVERY_THEME {
        let theme = Theme::new(kind, &terminal(ColorDepth::Palette256, false));
        for token in HUES.into_iter().chain(TINTS.map(|(tint, _)| tint)) {
            if let TokenColor::Indexed(index) = theme.token(token) {
                assert!(
                    (16..=231).contains(&index),
                    "{} in {} is {index}",
                    token.name(),
                    kind.name()
                );
            }
        }
    }
}

#[test]
fn existing_tokens_reduce_as_before() {
    for kind in EVERY_THEME {
        for depth in [ColorDepth::Palette256, ColorDepth::Palette16] {
            let theme = Theme::new(kind, &terminal(depth, false));
            let truecolor = Theme::new(kind, &terminal(ColorDepth::TrueColor, false));
            for token in Token::ALL.into_iter().take(12) {
                let old = match depth {
                    ColorDepth::Palette256 => nearest_256(hue(&truecolor, token)),
                    _ => nearest_16(hue(&truecolor, token)),
                };
                assert_eq!(
                    theme.token(token),
                    TokenColor::Indexed(old),
                    "{} in {} moved",
                    token.name(),
                    kind.name()
                );
            }
        }
    }
}

#[test]
fn tint_is_none_at_sixteen_colours_under_no_color_and_in_high_contrast() {
    for kind in EVERY_THEME {
        for (depth, no_color) in [
            (ColorDepth::TrueColor, false),
            (ColorDepth::Palette256, false),
            (ColorDepth::Palette16, false),
            (ColorDepth::None, false),
            (ColorDepth::TrueColor, true),
        ] {
            let theme = Theme::new(kind, &terminal(depth, no_color));
            let dropped = kind == ThemeKind::HighContrast
                || no_color
                || matches!(depth, ColorDepth::Palette16 | ColorDepth::None);
            for (tint, _) in TINTS {
                assert_eq!(
                    theme.tint(tint).is_none(),
                    dropped,
                    "{} in {} at {depth:?}, NO_COLOR {no_color}",
                    tint.name(),
                    kind.name()
                );
            }
        }
    }
}
