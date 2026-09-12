//! Turning a themed colour into the one a widget paints with.
//!
//! `theme` deliberately holds no `ratatui` type — it answers in
//! [`TokenColor`], which is Graft's own vocabulary — so the single conversion
//! lives here, next to the widgets that need it. One place to get the
//! `NO_COLOR` case right, rather than one per screen.

use graft_core::capabilities::Rgb;
use ratatui::style::{Color, Style};

use crate::theme::{Theme, Token, TokenColor};

/// The colour to paint with.
///
/// [`TokenColor::Default`] becomes [`Color::Reset`] rather than a guess at the
/// terminal's own foreground: `NO_COLOR` and a terminal with no colour at all
/// both arrive here, and both mean "leave this alone".
#[must_use]
pub fn color(token: TokenColor) -> Color {
    match token {
        TokenColor::Default => Color::Reset,
        TokenColor::Rgb(Rgb { red, green, blue }) => Color::Rgb(red, green, blue),
        TokenColor::Indexed(index) => Color::Indexed(index),
    }
}

/// A style that paints text in one of the theme's tokens.
#[must_use]
pub fn foreground(theme: &Theme, token: Token) -> Style {
    Style::new().fg(color(theme.token(token)))
}

#[cfg(test)]
mod tests {
    use graft_core::capabilities::ColorDepth;

    use super::*;
    use crate::test_support::terminal;
    use crate::theme::ThemeKind;

    #[test]
    fn a_true_colour_token_paints_its_own_rgb() {
        let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false));

        assert!(matches!(
            color(theme.token(Token::Text)),
            Color::Rgb(_, _, _)
        ));
    }

    #[test]
    fn a_palette_token_paints_an_index() {
        let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::Palette256, false));

        assert!(matches!(color(theme.token(Token::Text)), Color::Indexed(_)));
    }

    #[test]
    fn no_color_paints_nothing_at_all() {
        // Not "black", not "white": whatever the terminal was already using.
        let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, true));

        for token in Token::ALL {
            assert_eq!(color(theme.token(token)), Color::Reset, "{}", token.name());
        }
    }
}
