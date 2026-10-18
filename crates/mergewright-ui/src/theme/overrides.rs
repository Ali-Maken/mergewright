//! `[theme.<kind>] <token> = "#RRGGBB"`: his colours written over a built-in
//! table, before anything is reduced (m8-polish-and-ship/16).
//!
//! Two readers of the same pre-reduction table: [`Theme::with_overrides`]
//! builds a theme from it, [`contrast_misses`] holds it to
//! [`CONTRAST_RULES`]. The check is on the file's colours, not on a terminal's
//! reduction of them — he edits one file for every terminal he runs in.

use mergewright_core::capabilities::{Capabilities, ColorDepth, Rgb};

use super::chrome::ChromeHues;
use super::conflict::reduce_conflict;
use super::contrast::{CONTRAST_RULES, ContrastMiss, contrast};
use super::{LANE_COUNT, TOKEN_COUNT, Theme, ThemeKind, Token, TokenColor, lanes, reduce};

impl Theme {
    /// [`Theme::new`] with his `[theme.<kind>]` colours written over the
    /// built-in table first, so each is reduced to this terminal like any
    /// other (m8-polish-and-ship/16).
    ///
    /// Each override stands alone: overriding `ours` does not re-derive
    /// `ours-tint`. An overridden conflict token skips its palette pin (the
    /// pin was chosen for the built-in hue) and takes the nearest cube entry.
    /// Lanes are not tokens and cannot be overridden. Under `NO_COLOR` every
    /// token is [`TokenColor::Default`] whatever was overridden.
    pub fn with_overrides(
        kind: ThemeKind,
        capabilities: &Capabilities,
        overrides: &[(Token, Rgb)],
    ) -> Theme {
        let depth = if capabilities.no_color {
            ColorDepth::None
        } else {
            capabilities.color_depth
        };

        let rgb = table(kind, overrides);
        let mut resolved_chrome = [TokenColor::Default; TOKEN_COUNT];
        for token in Token::ALL {
            let hue = rgb[token.index()];
            resolved_chrome[token.index()] = if token.is_conflict() {
                let pinnable = !overrides.iter().any(|(named, _)| *named == token);
                reduce_conflict(kind, token, hue, depth, pinnable)
            } else {
                reduce(hue, depth)
            };
        }

        let mut resolved_lanes = [TokenColor::Default; LANE_COUNT];
        for (slot, hue) in resolved_lanes.iter_mut().zip(lanes::LANE_HUES) {
            *slot = reduce(hue, depth);
        }

        Theme {
            kind,
            depth,
            chrome: resolved_chrome,
            lanes: resolved_lanes,
            rgb,
        }
    }
}

impl Token {
    /// The token a config file names, by [`Token::name`]'s spelling; `None`
    /// for anything else — a typo, or `lane-N` (lanes are not tokens).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Token> {
        Token::ALL.into_iter().find(|token| token.name() == name)
    }
}

/// `kind`'s built-in colours with `overrides` written over them, in order:
/// a token named twice takes its last colour.
pub(super) fn table(kind: ThemeKind, overrides: &[(Token, Rgb)]) -> ChromeHues {
    let mut hues = kind.chrome();
    for &(token, hue) in overrides {
        hues[token.index()] = hue;
    }
    hues
}

/// Every floor `kind` with `overrides` misses, in [`CONTRAST_RULES`] order.
///
/// Pure and terminal-free: the ratio is measured on the colours as written.
/// The built-in tables clear every floor (`theme::contrast` tests), so with
/// no overrides this is empty.
#[must_use]
pub fn contrast_misses(kind: ThemeKind, overrides: &[(Token, Rgb)]) -> Vec<ContrastMiss> {
    let hues = table(kind, overrides);
    CONTRAST_RULES
        .iter()
        .map(|rule| ContrastMiss {
            rule: *rule,
            ratio: contrast(hues[rule.fg.index()], hues[rule.bg.index()]),
        })
        .filter(|miss| miss.ratio < miss.rule.min)
        .collect()
}

#[cfg(test)]
mod tests;
