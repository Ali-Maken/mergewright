//! Colour tokens, the three themes, and what survives on this terminal.
//!
//! Nothing in Graft names a colour. Widgets ask for a [`Token`] — `text`,
//! `border`, `added` — or for a lane's hue, and a [`Theme`] answers with a
//! [`TokenColor`] that the terminal in front of the user can actually show.
//! Everything between those two points lives in here: which theme the
//! terminal's background implies, what each token is worth in that theme, what
//! `NO_COLOR` does to it, and how a truecolor hue becomes a palette entry when
//! the terminal has only 256 or 16 of them.
//!
//! ```text
//! let kind = ThemeKind::select(config.theme, capabilities.theme);
//! let theme = Theme::new(kind, &capabilities);
//! let border = theme.token(Token::Border);   // Rgb, Indexed, or Default
//! let lane = theme.lane(row.lane);           // never theme-dependent
//! ```
//!
//! # The two halves, and why they don't mix
//!
//! Chrome ([`chrome`]) is themed; lanes ([`lanes`]) are not. PROMPT.md §5
//! forbids a theme from making two adjacent lanes indistinguishable, so a
//! theme has no lane colours to change — the twelve hues are a constant, and
//! [`Theme::lane`] reads them no matter which theme is active. The only thing
//! a terminal can do to a lane hue is reduce its depth, which happens to every
//! colour equally.
//!
//! # What this module does not do
//!
//! It hands out colours; it does not draw. There is no `ratatui` type here on
//! purpose: converting a [`TokenColor`] into whatever the renderer wants is
//! the widgets' job (ticket 09). Runtime theme switching and re-querying
//! `OSC 11` on focus (DESIGN.md §4.5) are M1.

mod chrome;
mod depth;
mod lanes;

#[cfg(test)]
mod tests;

use graft_core::capabilities::{
    Capabilities, ColorDepth, Rgb, Theme as TerminalTheme, ThemeHint, ThemeSource,
};

/// The number of chrome tokens PROMPT.md §5 names, lanes excluded.
const TOKEN_COUNT: usize = 11;

/// The number of lane hues. Lanes beyond this reuse them ([`Theme::lane`]).
pub const LANE_COUNT: usize = 12;

/// One named colour of the interface, excluding lanes.
///
/// Exactly PROMPT.md §5's list. Lanes are deliberately not variants here: they
/// are not themed, and giving them a `Token` would invite a theme table to
/// define one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    /// The window's ground. See [`chrome`] on why drawing it is a separate
    /// decision from knowing it.
    Bg,
    /// A pane's fill, a step up from `bg`.
    Surface,
    /// Something floating over a pane: the palette, a dialog, a selected row.
    SurfaceRaised,
    /// Pane rules. Thin, faint, and not a text colour.
    Border,
    /// Body text.
    Text,
    /// Text that is present but secondary: timestamps, hints, counts.
    TextMuted,
    /// Focus. The focused pane's border and its title (DESIGN.md §1).
    Accent,
    /// Added lines, and additions anywhere else.
    Added,
    /// Removed lines, and deletions anywhere else.
    Removed,
    /// Something that needs attention but isn't an error: a Mistake Radar
    /// marker, a dirty working tree, an in-progress operation.
    Warning,
    /// Neutral emphasis: a note, a count, a state that is merely unusual.
    Info,
}

impl Token {
    /// Every token, in PROMPT.md §5's order.
    pub const ALL: [Token; TOKEN_COUNT] = [
        Token::Bg,
        Token::Surface,
        Token::SurfaceRaised,
        Token::Border,
        Token::Text,
        Token::TextMuted,
        Token::Accent,
        Token::Added,
        Token::Removed,
        Token::Warning,
        Token::Info,
    ];

    /// The token's name as PROMPT.md §5 and the config file spell it.
    pub const fn name(self) -> &'static str {
        match self {
            Token::Bg => "bg",
            Token::Surface => "surface",
            Token::SurfaceRaised => "surface-raised",
            Token::Border => "border",
            Token::Text => "text",
            Token::TextMuted => "text-muted",
            Token::Accent => "accent",
            Token::Added => "added",
            Token::Removed => "removed",
            Token::Warning => "warning",
            Token::Info => "info",
        }
    }

    /// Where this token sits in a theme's colour table. Declaration order;
    /// [`tests`] pins it against [`Token::ALL`].
    pub(super) const fn index(self) -> usize {
        self as usize
    }
}

/// Which of the three shipped themes.
///
/// [`ThemeKind::HighContrast`] is reachable only through an explicit override:
/// no terminal background implies it, and guessing that a user wants maximum
/// contrast from their wallpaper would be wrong more often than right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeKind {
    Dark,
    Light,
    HighContrast,
}

impl ThemeKind {
    /// Which theme to use: the override if there is one, otherwise what the
    /// terminal's background says, otherwise dark.
    ///
    /// The hint comes from `OSC 11`, which reports an opaque colour even on a
    /// translucent background — so it is read as light-or-dark and never as a
    /// colour (ADR-0003). A [`ThemeSource::Assumed`] hint is the *unknown*
    /// case, not a measurement of a dark terminal: the terminal said nothing,
    /// or said something unparseable. Dark is what unknown resolves to, and an
    /// override is the fix.
    pub fn select(overridden: Option<ThemeKind>, hint: ThemeHint) -> ThemeKind {
        if let Some(kind) = overridden {
            return kind;
        }

        match hint.source {
            ThemeSource::Background(_) => match hint.theme {
                TerminalTheme::Dark => ThemeKind::Dark,
                TerminalTheme::Light => ThemeKind::Light,
            },
            ThemeSource::Assumed => ThemeKind::Dark,
        }
    }

    /// The theme's name as the config file spells it.
    pub const fn name(self) -> &'static str {
        match self {
            ThemeKind::Dark => "dark",
            ThemeKind::Light => "light",
            ThemeKind::HighContrast => "high-contrast",
        }
    }

    fn chrome(self) -> chrome::ChromeHues {
        match self {
            ThemeKind::Dark => chrome::DARK,
            ThemeKind::Light => chrome::LIGHT,
            ThemeKind::HighContrast => chrome::HIGH_CONTRAST,
        }
    }
}

/// What to paint for one token on this terminal.
///
/// Already reduced: a widget never has to ask what the colour depth was, and
/// never has to decide what `NO_COLOR` means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenColor {
    /// Paint no colour — leave whatever the terminal already uses. `NO_COLOR`
    /// and [`ColorDepth::None`] turn every token into this.
    Default,
    /// A 24-bit colour, as the theme defines it.
    Rgb(Rgb),
    /// An ANSI palette index: 16–255 on a 256-colour terminal, 0–15 on a
    /// 16-colour one.
    Indexed(u8),
}

/// A theme resolved for one terminal: every token already reduced to a colour
/// that terminal can show.
///
/// Built once per session. Holding one means the depth mapping has already run
/// — there is no path that hands a widget a colour the terminal can't display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    kind: ThemeKind,
    chrome: [TokenColor; TOKEN_COUNT],
    lanes: [TokenColor; LANE_COUNT],
}

impl Theme {
    /// Resolve `kind` against what the terminal can do.
    ///
    /// Two things in [`Capabilities`] matter here and nothing else does:
    /// `no_color` (what the user asked for) and `color_depth` (what the
    /// terminal can do). Either one being absent of colour empties every
    /// token, including the lanes — DESIGN.md §4.6 puts lane identity on
    /// shape and dash patterns in that case, not on a colour Graft was told
    /// not to use.
    pub fn new(kind: ThemeKind, capabilities: &Capabilities) -> Theme {
        let depth = if capabilities.no_color {
            ColorDepth::None
        } else {
            capabilities.color_depth
        };

        let chrome = kind.chrome();
        let mut resolved_chrome = [TokenColor::Default; TOKEN_COUNT];
        for token in Token::ALL {
            resolved_chrome[token.index()] = reduce(chrome[token.index()], depth);
        }

        let mut resolved_lanes = [TokenColor::Default; LANE_COUNT];
        for (slot, hue) in resolved_lanes.iter_mut().zip(lanes::LANE_HUES) {
            *slot = reduce(hue, depth);
        }

        Theme {
            kind,
            chrome: resolved_chrome,
            lanes: resolved_lanes,
        }
    }

    /// Which theme this is. For the diagnostics view and for config round-trips.
    pub fn kind(&self) -> ThemeKind {
        self.kind
    }

    /// The colour for one chrome token.
    pub fn token(&self, token: Token) -> TokenColor {
        self.chrome[token.index()]
    }

    /// The colour for a lane, by its index (CONTEXT.md: lane 0 is the default
    /// branch, and is PROMPT.md §5's `lane-1`).
    ///
    /// A repository can have more concurrent lanes than the palette has hues.
    /// Lane `LANE_COUNT` reuses lane 0's hue, and so on: reuse is far apart on
    /// screen, whereas running out of colours would leave a lane with none.
    /// The graph's own shape and dash redundancy (DESIGN.md §4.6) is what
    /// separates two lanes sharing a hue.
    pub fn lane(&self, lane: usize) -> TokenColor {
        self.lanes[lane % LANE_COUNT]
    }
}

/// One truecolor hue, reduced to what this depth can show.
fn reduce(hue: Rgb, depth: ColorDepth) -> TokenColor {
    match depth {
        ColorDepth::TrueColor => TokenColor::Rgb(hue),
        ColorDepth::Palette256 => TokenColor::Indexed(depth::nearest_256(hue)),
        ColorDepth::Palette16 => TokenColor::Indexed(depth::nearest_16(hue)),
        ColorDepth::None => TokenColor::Default,
    }
}
