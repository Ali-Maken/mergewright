//! `Theme::colourless` (`#`, ease-pass G14): what `NO_COLOR` resolves to.

use mergewright_core::capabilities::ColorDepth;

use crate::test_support::terminal;

use super::{LANE_COUNT, Theme, ThemeKind, Token};

#[test]
fn colourless_is_what_no_color_resolves_to_in_every_theme() {
    for kind in [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast] {
        let coloured = Theme::new(kind, &terminal(ColorDepth::TrueColor, false));
        let no_color = Theme::new(kind, &terminal(ColorDepth::TrueColor, true));
        let off = coloured.colourless();
        assert_eq!(off.color_depth(), ColorDepth::None);
        for token in Token::ALL {
            assert_eq!(off.token(token), no_color.token(token), "{}", token.name());
        }
        for lane in 0..LANE_COUNT {
            assert_eq!(off.lane(lane), no_color.lane(lane), "lane {lane}");
        }
        assert_eq!(off.kind(), kind);
    }
}
