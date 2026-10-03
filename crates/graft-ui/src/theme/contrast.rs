//! Contrast: the WCAG ratio, the floors every theme is held to, and the lift
//! that keeps text legible on the `selection` fill (ADR-0058 §3).
//!
//! Pure `Rgb` arithmetic first ([`contrast`], [`Theme::legible_on`]), so the
//! numbers are testable on their own; [`Theme::on_selection`] is the one
//! entry a widget calls, holding a [`TokenColor`] at whatever depth the
//! terminal has.

use graft_core::capabilities::{ColorDepth, Rgb};

use super::{Theme, Token, TokenColor, depth, mix};

/// The text floor: WCAG 2.1 AA for normal text.
const LEGIBLE: f32 = 4.5;
/// How far each step of the lift moves toward white or black.
const STEP: f32 = 0.05;
/// Steps of [`STEP`] until the lift reaches its target outright.
const MAX_STEPS: u8 = 20;

const WHITE: Rgb = Rgb {
    red: 0xFF,
    green: 0xFF,
    blue: 0xFF,
};
const BLACK: Rgb = Rgb {
    red: 0,
    green: 0,
    blue: 0,
};

/// WCAG 2.x contrast ratio, `(L1 + 0.05) / (L2 + 0.05)` over relative
/// luminance, lighter first. Symmetric: the order of `fg` and `bg` does not
/// matter. 1.0 for equal colours, 21.0 for black on white.
#[must_use]
pub fn contrast(fg: Rgb, bg: Rgb) -> f32 {
    let (fg, bg) = (luminance(fg), luminance(bg));
    let (lighter, darker) = if fg > bg { (fg, bg) } else { (bg, fg) };
    (lighter + 0.05) / (darker + 0.05)
}

fn luminance(color: Rgb) -> f32 {
    0.2126 * depth::linearize(color.red)
        + 0.7152 * depth::linearize(color.green)
        + 0.0722 * depth::linearize(color.blue)
}

/// One floor a theme must clear: `fg` drawn on `bg` at `min`:1 or better.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContrastRule {
    /// The foreground (or, for `selection`, the indicator).
    pub fg: Token,
    /// What it sits on.
    pub bg: Token,
    /// The ratio it must reach.
    pub min: f32,
}

/// The four floors every shipped theme is tested against
/// (`docs/design-notes/m8-polish-and-ship.md`, "Dark pick: B"): text on the
/// ground and on the cursor row at WCAG AA, the cursor row against the ground
/// at WCAG 1.4.11's non-text floor, and muted text on the ground at AA
/// (raised from 3.0, `KNOWN_GAPS` row 541).
pub const CONTRAST_RULES: &[ContrastRule] = &[
    ContrastRule {
        fg: Token::Text,
        bg: Token::Bg,
        min: 4.5,
    },
    ContrastRule {
        fg: Token::Text,
        bg: Token::Selection,
        min: 4.5,
    },
    ContrastRule {
        fg: Token::Selection,
        bg: Token::Bg,
        min: 3.0,
    },
    ContrastRule {
        fg: Token::TextMuted,
        bg: Token::Bg,
        min: 4.5,
    },
];

/// The colour the lift heads for on `bg`: white where white contrasts more,
/// black otherwise — white on a dark fill, black on a light one, and
/// whichever works on high contrast's grey (ADR-0058 §3).
fn lift_target(bg: Rgb) -> Rgb {
    if contrast(WHITE, bg) >= contrast(BLACK, bg) {
        WHITE
    } else {
        BLACK
    }
}

/// `fg` mixed toward `target` in [`STEP`]s, the first that `clears`; `fg`
/// itself when it already does, `target` when twenty steps do not.
fn lift(fg: Rgb, target: Rgb, clears: impl Fn(Rgb) -> bool) -> Rgb {
    (0..=MAX_STEPS)
        .map(|step| mix(target, fg, f32::from(step) * STEP))
        .find(|candidate| clears(*candidate))
        .unwrap_or(target)
}

impl Theme {
    /// `fg` as drawn on `bg`: unchanged when it already reaches 4.5:1, else
    /// mixed toward white (dark `bg`) or black (light `bg`) in 5% steps until
    /// it does. A lane hue keeps its family — the channel order is kept by
    /// mixing toward a grey — and becomes readable.
    #[must_use]
    pub fn legible_on(&self, fg: Rgb, bg: Rgb) -> Rgb {
        lift(fg, lift_target(bg), |candidate| {
            contrast(candidate, bg) >= LEGIBLE
        })
    }

    /// `fg` as drawn on this theme's `selection` fill, at this terminal's
    /// depth — what every widget calls for text it draws on the cursor row.
    ///
    /// - Truecolor: [`Theme::legible_on`] against `selection`.
    /// - 256 colours: the same lift, but each step is reduced to the palette
    ///   and judged as the terminal will show it, against the reduced
    ///   `selection`; the first reduced pair at 4.5:1 wins.
    /// - 16 colours, `NO_COLOR`, or a `fg` of [`TokenColor::Default`]:
    ///   `text` — there is no fill to be legible on, or no hue to keep, and
    ///   the caller adds bold (ADR-0058 §3).
    #[must_use]
    pub fn on_selection(&self, fg: TokenColor) -> TokenColor {
        let selection = self.kind.chrome()[Token::Selection.index()];
        match (self.depth, fg) {
            (ColorDepth::TrueColor, TokenColor::Rgb(fg)) => {
                TokenColor::Rgb(self.legible_on(fg, selection))
            }
            (ColorDepth::Palette256, TokenColor::Indexed(index)) => {
                let fill = depth::palette_entry(depth::nearest_256(selection));
                let lifted = lift(
                    depth::palette_entry(index),
                    lift_target(fill),
                    |candidate| {
                        contrast(depth::palette_entry(depth::nearest_256(candidate)), fill)
                            >= LEGIBLE
                    },
                );
                TokenColor::Indexed(depth::nearest_256(lifted))
            }
            // A truecolor theme never hands out an index, nor a 256-colour
            // one an `Rgb`; and below 256 colours the fill is not drawn.
            // Every other pairing is `text`, which is legible everywhere a
            // fill is drawn (`CONTRAST_RULES`) and is the 16-colour rule.
            _ => self.token(Token::Text),
        }
    }
}

#[cfg(test)]
mod tests;
