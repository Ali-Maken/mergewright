//! The six conflict-resolver tokens (ADR-0044 §1): which of them a terminal
//! drops, the palette entries they are pinned to, and how they reduce.
//!
//! Split from [`super`] (m6-conflicts/64, review M2) so the theme's own
//! module stays within PROMPT.md §6.2's size. The hues themselves live in
//! [`super::chrome`]'s tables with every other token.

use graft_core::capabilities::{ColorDepth, Rgb};

use super::{Theme, ThemeKind, Token, TokenColor, depth, reduce};

/// How much of a hue a conflict tint keeps over `bg` (ADR-0044 §1: "medium").
/// One α for all three tints.
pub const TINT_ALPHA: f32 = 0.30;

impl Token {
    /// Whether this token is a conflict tint: a background, dropped where
    /// tints are ([`Theme::tint`]).
    #[must_use]
    pub const fn is_tint(self) -> bool {
        matches!(self, Token::OursTint | Token::TheirsTint | Token::OpenTint)
    }

    /// Whether this is one of the six conflict tokens (ADR-0044 §1), which
    /// reduce through [`pinned`] and the cube-only search, not [`reduce`].
    pub(super) const fn is_conflict(self) -> bool {
        matches!(self, Token::Ours | Token::Theirs | Token::Open) || self.is_tint()
    }
}

impl Theme {
    /// A tint token's background, or `None` where tints are dropped: 16
    /// colours, `NO_COLOR`, and the high-contrast theme (ADR-0044 §1). Widgets
    /// ask this, never the depth.
    ///
    /// A non-tint token → `None`, with a `debug_assert` naming the misuse.
    #[must_use]
    pub fn tint(&self, token: Token) -> Option<TokenColor> {
        debug_assert!(
            token.is_tint(),
            "Theme::tint asked for {}, which is not a tint",
            token.name()
        );
        if !token.is_tint() || tints_dropped(self.kind, self.depth) {
            return None;
        }
        Some(self.token(token))
    }
}

/// `fg` over `bg` at `alpha` (0 = bg, 1 = fg), per channel, rounded. Pure.
#[must_use]
pub fn mix(fg: Rgb, bg: Rgb, alpha: f32) -> Rgb {
    let channel = |fg: u8, bg: u8| {
        let mixed = f32::from(fg) * alpha + f32::from(bg) * (1.0 - alpha);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to the u8 range first, so the cast cannot truncate or wrap"
        )]
        let rounded = mixed.round().clamp(0.0, 255.0) as u8;
        rounded
    };
    Rgb {
        red: channel(fg.red, bg.red),
        green: channel(fg.green, bg.green),
        blue: channel(fg.blue, bg.blue),
    }
}

/// Where conflict tints paint nothing (ADR-0044 §1): ANSI backgrounds at 16
/// colours are full-strength blocks, `NO_COLOR` has no colour, and the
/// high-contrast theme keeps its black ground.
const fn tints_dropped(kind: ThemeKind, depth: ColorDepth) -> bool {
    matches!(kind, ThemeKind::HighContrast)
        || matches!(depth, ColorDepth::Palette16 | ColorDepth::None)
}

/// A conflict token's fixed palette entry, where ADR-0044 §1 pins one: the
/// nearest-colour search would put a dark tint on the grey ramp and lose it.
const fn pinned(kind: ThemeKind, token: Token, depth: ColorDepth) -> Option<u8> {
    let hues_pinned = matches!(kind, ThemeKind::Dark | ThemeKind::HighContrast);
    let tints_pinned = matches!(kind, ThemeKind::Dark);
    match (depth, token) {
        (ColorDepth::Palette256, Token::Ours) if hues_pinned => Some(74),
        (ColorDepth::Palette256, Token::Theirs) if hues_pinned => Some(175),
        (ColorDepth::Palette256, Token::Open) if hues_pinned => Some(214),
        (ColorDepth::Palette256, Token::OursTint) if tints_pinned => Some(23),
        (ColorDepth::Palette256, Token::TheirsTint) if tints_pinned => Some(53),
        (ColorDepth::Palette256, Token::OpenTint) if tints_pinned => Some(58),
        (ColorDepth::Palette16, Token::Ours) if hues_pinned => Some(6),
        (ColorDepth::Palette16, Token::Theirs) if hues_pinned => Some(5),
        (ColorDepth::Palette16, Token::Open) if hues_pinned => Some(3),
        _ => None,
    }
}

/// One conflict token reduced to this depth: dropped tints paint nothing,
/// then a pin, then the nearest cube entry (never the grey ramp) at 256
/// colours or the nearest system colour at 16.
pub(super) fn reduce_conflict(
    kind: ThemeKind,
    token: Token,
    hue: Rgb,
    depth: ColorDepth,
) -> TokenColor {
    if token.is_tint() && tints_dropped(kind, depth) {
        return TokenColor::Default;
    }
    if let Some(index) = pinned(kind, token, depth) {
        return TokenColor::Indexed(index);
    }
    match depth {
        ColorDepth::Palette256 => TokenColor::Indexed(depth::nearest_256_cube(hue)),
        ColorDepth::TrueColor | ColorDepth::Palette16 | ColorDepth::None => reduce(hue, depth),
    }
}
