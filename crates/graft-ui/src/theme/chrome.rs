//! The chrome colours: every non-lane token, once per theme.
//!
//! Chrome is everything that isn't a branch line — panes, borders, text,
//! and the four semantic colours (`added`, `removed`, `warning`, `info`).
//! PROMPT.md §5 wants it calm and in the background, so the surfaces sit close
//! together and only the semantic tokens carry saturation.
//!
//! # Contrast
//!
//! Every token that ever carries text or a glyph reaches at least WCAG 2.1
//! 4.5:1 against `bg`, `surface` and `surface-raised` in all three themes;
//! [`tests`](super::tests) asserts it, so a future colour tweak can't quietly
//! drop below the line. `border` is exempt: it draws rules, not text, and
//! deliberately stays faint (about 1.5:1) in the two normal themes. The
//! high-contrast theme pulls it to full white.
//!
//! # `bg`
//!
//! `bg` is the theme's *nominal* background. ADR-0003's translucency problem
//! applies to whoever paints it: on a translucent terminal, filling the screen
//! with `bg` destroys the user's blur. Deciding when to paint it and when to
//! leave the terminal's own background showing belongs to the widgets
//! (ticket 09), not here; this module only says what the colour would be.

use graft_core::capabilities::Rgb;

use super::{TOKEN_COUNT, Token};

/// The chrome colours of one theme, indexed by [`Token::index`].
pub(super) type ChromeHues = [Rgb; TOKEN_COUNT];

/// Dark: the default, and what an unknown terminal gets (DESIGN.md §4.1).
///
/// Dark B (ADR-0058 §1): the ground he shares with his other tools, `text`
/// brighter. `selection` #5C667B measures 3.14:1 against `bg` (floor 3:1,
/// WCAG 1.4.11), `text` 5.14:1 on it and `text-muted` 6.95:1 on `bg` (floors
/// 4.5:1) — every floor in `CONTRAST_RULES`, asserted in `contrast::tests`.
pub(super) const DARK: ChromeHues = roles(
    conflict(
        build(
            rgb(0x14, 0x16, 0x1B), // bg
            rgb(0x1B, 0x1E, 0x24), // surface
            rgb(0x23, 0x27, 0x30), // surface-raised
            rgb(0x5C, 0x66, 0x7B), // selection
            rgb(0x33, 0x39, 0x44), // border
            rgb(0xF0, 0xF2, 0xF5), // text
            rgb(0x9A, 0xA1, 0xAC), // text-muted
            rgb(0x4F, 0xA3, 0xE3), // accent
            rgb(0x5D, 0xB8, 0x7A), // added
            rgb(0xE0, 0x6C, 0x75), // removed
            rgb(0xE5, 0xA9, 0x4E), // warning
            rgb(0x6F, 0xB3, 0xD2), // info
        ),
        rgb(0x56, 0xB4, 0xE9), // ours: Okabe–Ito sky blue
        rgb(0xCC, 0x79, 0xA7), // theirs: Okabe–Ito reddish purple
        rgb(0xE6, 0x9F, 0x00), // open
        rgb(0x28, 0x45, 0x59), // ours-tint = mix(ours, bg, 0.30)
        rgb(0x4B, 0x34, 0x45), // theirs-tint
        rgb(0x53, 0x3F, 0x13), // open-tint
    ),
    // The main mock's role colours (ease pass, ADR-0063).
    rgb(0x7E, 0xD4, 0x9B), // role-this: 10.2:1 on bg
    rgb(0xE9, 0xB6, 0x5C), // role-under: 9.8:1
    rgb(0x9F, 0xB7, 0xFF), // role-trunk: 9.2:1
    rgb(0x4D, 0x56, 0x54), // role-other: 2.4:1, quiet on purpose
);

/// Light: the same relationships inverted, not the dark hues brightened.
///
/// `selection` (m5-history-and-ghost-preview/44): #828CA0 measures 3.27:1
/// against `bg` and 4.71:1 for `text` drawn on it — both floors clear here.
pub(super) const LIGHT: ChromeHues = roles(
    conflict(
        build(
            rgb(0xFB, 0xFB, 0xFC), // bg
            rgb(0xF2, 0xF3, 0xF5), // surface
            rgb(0xE8, 0xEA, 0xED), // surface-raised
            rgb(0x82, 0x8C, 0xA0), // selection
            rgb(0xC9, 0xCD, 0xD4), // border
            rgb(0x1E, 0x22, 0x2A), // text
            rgb(0x5F, 0x67, 0x73), // text-muted
            rgb(0x15, 0x63, 0x9F), // accent
            rgb(0x15, 0x70, 0x3C), // added
            rgb(0xB4, 0x23, 0x2B), // removed
            rgb(0x8A, 0x5A, 0x00), // warning
            rgb(0x1C, 0x6B, 0x8C), // info
        ),
        rgb(0x00, 0x72, 0xB2), // ours
        rgb(0xA8, 0x46, 0x7F), // theirs
        rgb(0xB3, 0x6B, 0x00), // open
        rgb(0xB0, 0xD2, 0xE6), // ours-tint = mix(ours, bg, 0.30)
        rgb(0xE2, 0xC5, 0xD7), // theirs-tint
        rgb(0xE5, 0xD0, 0xB0), // open-tint
    ),
    // The dark roles' hues darkened to clear 3:1 on the light ground.
    rgb(0x1E, 0x7F, 0x45), // role-this: 4.9:1 on bg
    rgb(0x9A, 0x62, 0x00), // role-under: 4.9:1
    rgb(0x35, 0x50, 0xB8), // role-trunk: 6.8:1
    rgb(0xB4, 0xBA, 0xC2), // role-other: 1.9:1, quiet on purpose
);

/// High contrast: black ground, white rules, saturated semantics. Reachable
/// only through an explicit override — no terminal background implies it.
///
/// `selection` (m5-history-and-ghost-preview/44): #696969 measures 3.83:1
/// against `bg` and 5.49:1 for `text` drawn on it — both floors clear here.
pub(super) const HIGH_CONTRAST: ChromeHues = roles(
    conflict(
        build(
            rgb(0x00, 0x00, 0x00), // bg
            rgb(0x00, 0x00, 0x00), // surface
            rgb(0x1A, 0x1A, 0x1A), // surface-raised
            rgb(0x69, 0x69, 0x69), // selection
            rgb(0xFF, 0xFF, 0xFF), // border
            rgb(0xFF, 0xFF, 0xFF), // text
            rgb(0xD0, 0xD0, 0xD0), // text-muted
            rgb(0xFF, 0xD4, 0x00), // accent
            rgb(0x00, 0xE6, 0x76), // added
            rgb(0xFF, 0x52, 0x52), // removed
            rgb(0xFF, 0x9E, 0x1F), // warning
            rgb(0x40, 0xC4, 0xFF), // info
        ),
        rgb(0x56, 0xB4, 0xE9), // ours: the dark hues clear 4.5:1 on black
        rgb(0xCC, 0x79, 0xA7), // theirs
        rgb(0xE6, 0x9F, 0x00), // open
        rgb(0x00, 0x00, 0x00), // ours-tint: bg; never drawn, `Theme::tint` is None here
        rgb(0x00, 0x00, 0x00), // theirs-tint
        rgb(0x00, 0x00, 0x00), // open-tint
    ),
    rgb(0x00, 0xE6, 0x76), // role-this: 12.6:1 on bg
    rgb(0xFF, 0xB3, 0x00), // role-under: 11.7:1
    rgb(0x82, 0xA0, 0xFF), // role-trunk: 8.4:1
    rgb(0x6E, 0x6E, 0x6E), // role-other: 4.1:1
);

/// Spells the token order out once, so a theme table can't silently transpose
/// two colours: every argument is named, and [`Token::index`] is the only
/// thing that decides where each lands.
#[expect(
    clippy::too_many_arguments,
    reason = "one argument per token is the point: it names each colour at every call site"
)]
const fn build(
    bg: Rgb,
    surface: Rgb,
    surface_raised: Rgb,
    selection: Rgb,
    border: Rgb,
    text: Rgb,
    text_muted: Rgb,
    accent: Rgb,
    added: Rgb,
    removed: Rgb,
    warning: Rgb,
    info: Rgb,
) -> ChromeHues {
    let mut hues = [bg; TOKEN_COUNT];
    hues[Token::Bg.index()] = bg;
    hues[Token::Surface.index()] = surface;
    hues[Token::SurfaceRaised.index()] = surface_raised;
    hues[Token::Selection.index()] = selection;
    hues[Token::Border.index()] = border;
    hues[Token::Text.index()] = text;
    hues[Token::TextMuted.index()] = text_muted;
    hues[Token::Accent.index()] = accent;
    hues[Token::Added.index()] = added;
    hues[Token::Removed.index()] = removed;
    hues[Token::Warning.index()] = warning;
    hues[Token::Info.index()] = info;
    hues
}

/// Sets the six conflict slots (ADR-0044 §1) on a table [`build`] made, one
/// named argument per colour. Tints are literals; `theme::tests` ties each to
/// `theme::mix` so the table and the rule cannot drift.
const fn conflict(
    mut hues: ChromeHues,
    ours: Rgb,
    theirs: Rgb,
    open: Rgb,
    ours_tint: Rgb,
    theirs_tint: Rgb,
    open_tint: Rgb,
) -> ChromeHues {
    hues[Token::Ours.index()] = ours;
    hues[Token::Theirs.index()] = theirs;
    hues[Token::Open.index()] = open;
    hues[Token::OursTint.index()] = ours_tint;
    hues[Token::TheirsTint.index()] = theirs_tint;
    hues[Token::OpenTint.index()] = open_tint;
    hues
}

/// Sets the four graph role slots (ease pass, ADR-0063) on a table, one
/// named argument per colour.
const fn roles(mut hues: ChromeHues, this: Rgb, under: Rgb, trunk: Rgb, other: Rgb) -> ChromeHues {
    hues[Token::RoleThis.index()] = this;
    hues[Token::RoleUnder.index()] = under;
    hues[Token::RoleTrunk.index()] = trunk;
    hues[Token::RoleOther.index()] = other;
    hues
}

const fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}
