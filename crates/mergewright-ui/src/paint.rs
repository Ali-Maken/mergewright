//! Turning a themed colour into the one a widget paints with.
//!
//! `theme` deliberately holds no `ratatui` type — it answers in
//! [`TokenColor`], which is Mergewright's own vocabulary — so the single conversion
//! lives here, next to the widgets that need it. One place to get the
//! `NO_COLOR` case right, rather than one per screen.

use mergewright_core::capabilities::Rgb;
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

/// `style` as drawn on the `selection` fill: its foreground lifted to stay
/// legible there ([`Theme::on_selection`], ADR-0058 §3). For every screen
/// that paints a cursor row; the rest of the style is kept.
///
/// The foreground is read back from the style, so a widget can build its
/// styles as on any row and lift them in one place. A style with no
/// foreground — or one in a named ANSI colour, which no theme hands out —
/// draws `text`.
#[must_use]
pub fn on_selection(theme: &Theme, style: Style) -> Style {
    style.fg(lifted(theme, style.fg))
}

/// [`on_selection`] over every cell of `area` already drawn: for a row whose
/// pieces come from several helpers, lifted once after they have all drawn.
pub fn lift_on_selection(
    buf: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    theme: &Theme,
) {
    let area = area.intersection(buf.area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.fg = lifted(theme, Some(cell.fg));
        }
    }
}

/// One foreground as drawn on the fill. `None` (no foreground set) and named
/// ANSI colours — which no theme hands out — read as the terminal default,
/// which [`Theme::on_selection`] turns into `text`.
fn lifted(theme: &Theme, fg: Option<Color>) -> Color {
    let fg = match fg {
        Some(Color::Rgb(red, green, blue)) => TokenColor::Rgb(Rgb { red, green, blue }),
        Some(Color::Indexed(index)) => TokenColor::Indexed(index),
        _ => TokenColor::Default,
    };
    color(theme.on_selection(fg))
}

#[cfg(test)]
mod tests {
    use mergewright_core::capabilities::ColorDepth;

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
