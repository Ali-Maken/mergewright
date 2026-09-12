//! The chrome colours: the eleven non-lane tokens, once per theme.
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

/// The eleven chrome colours of one theme, indexed by [`Token::index`].
pub(super) type ChromeHues = [Rgb; TOKEN_COUNT];

/// Dark: the default, and what an unknown terminal gets (DESIGN.md §4.1).
pub(super) const DARK: ChromeHues = build(
    rgb(0x14, 0x16, 0x1B), // bg
    rgb(0x1B, 0x1E, 0x24), // surface
    rgb(0x23, 0x27, 0x30), // surface-raised
    rgb(0x33, 0x39, 0x44), // border
    rgb(0xD7, 0xDB, 0xE0), // text
    rgb(0x8A, 0x91, 0x9C), // text-muted
    rgb(0x4F, 0xA3, 0xE3), // accent
    rgb(0x5D, 0xB8, 0x7A), // added
    rgb(0xE0, 0x6C, 0x75), // removed
    rgb(0xE5, 0xA9, 0x4E), // warning
    rgb(0x6F, 0xB3, 0xD2), // info
);

/// Light: the same relationships inverted, not the dark hues brightened.
pub(super) const LIGHT: ChromeHues = build(
    rgb(0xFB, 0xFB, 0xFC), // bg
    rgb(0xF2, 0xF3, 0xF5), // surface
    rgb(0xE8, 0xEA, 0xED), // surface-raised
    rgb(0xC9, 0xCD, 0xD4), // border
    rgb(0x1E, 0x22, 0x2A), // text
    rgb(0x5F, 0x67, 0x73), // text-muted
    rgb(0x15, 0x63, 0x9F), // accent
    rgb(0x15, 0x70, 0x3C), // added
    rgb(0xB4, 0x23, 0x2B), // removed
    rgb(0x8A, 0x5A, 0x00), // warning
    rgb(0x1C, 0x6B, 0x8C), // info
);

/// High contrast: black ground, white rules, saturated semantics. Reachable
/// only through an explicit override — no terminal background implies it.
pub(super) const HIGH_CONTRAST: ChromeHues = build(
    rgb(0x00, 0x00, 0x00), // bg
    rgb(0x00, 0x00, 0x00), // surface
    rgb(0x1A, 0x1A, 0x1A), // surface-raised
    rgb(0xFF, 0xFF, 0xFF), // border
    rgb(0xFF, 0xFF, 0xFF), // text
    rgb(0xD0, 0xD0, 0xD0), // text-muted
    rgb(0xFF, 0xD4, 0x00), // accent
    rgb(0x00, 0xE6, 0x76), // added
    rgb(0xFF, 0x52, 0x52), // removed
    rgb(0xFF, 0x9E, 0x1F), // warning
    rgb(0x40, 0xC4, 0xFF), // info
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

const fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}
