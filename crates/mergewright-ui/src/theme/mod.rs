//! Colour tokens, the three themes, and what survives on this terminal.
//!
//! Nothing in Mergewright names a colour. Widgets ask for a [`Token`] — `text`,
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
mod conflict;
mod contrast;
mod depth;
mod lanes;
mod overrides;

#[cfg(test)]
mod colourless_tests;
#[cfg(test)]
mod tests;

use mergewright_core::capabilities::{
    Capabilities, ColorDepth, Rgb, Theme as TerminalTheme, ThemeHint, ThemeSource,
};

use crate::syntax::SyntaxRole;

pub use conflict::{TINT_ALPHA, mix};
pub use contrast::{CONTRAST_RULES, ContrastMiss, ContrastRule, contrast};
pub use overrides::contrast_misses;

/// The number of chrome tokens PROMPT.md §5 names, lanes excluded, plus
/// [`Token::Selection`] (m5-history-and-ghost-preview/44): PROMPT.md §5 does
/// not name it, but `surface-raised` cannot keep doing both jobs (see
/// [`Token::Selection`]'s doc), so the count grew by one to hold the split.
///
/// Plus the six conflict-resolver roles (ADR-0044 §1): three hues and their
/// three tints.
///
/// Plus the graph's four role colours (ease pass, ADR-0063): a line's
/// colour says how it relates to the cursor row's branch.
pub const TOKEN_COUNT: usize = 22;

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
    /// Something floating over a pane: the palette, a dialog.
    SurfaceRaised,
    /// "You are here": the cursor row in a list or the graph. A step deliberately
    /// larger than `surface-raised`'s — m5-history-and-ghost-preview/44:
    /// `surface-raised` used to draw this too, at a ~6% lightness step that
    /// measured 1.1:1–1.3:1 against `bg`, under the WCAG 1.4.11 non-text floor
    /// of 3:1. A raised panel wants a quiet step; the row that answers "where
    /// am I" needs to shout, and one token cannot do both without one of its
    /// two jobs failing. `surface-raised` keeps its own (smaller) contrast
    /// floor; this token's floor is asserted separately in `theme::tests`.
    Selection,
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
    /// The resolver's left pane (git's stage 2): its title, rule words and
    /// separator glyphs. Named by position, not owner (ADR-0044 §1).
    Ours,
    /// The resolver's right pane (git's stage 3).
    Theirs,
    /// An unresolved block: `? open`, its separators, the list's open counts.
    Open,
    /// Background of left-block rows and of result rows taken from the left.
    /// A background, never a text colour; dropped at 16 colours, under
    /// `NO_COLOR` and in high contrast ([`Theme::tint`]).
    OursTint,
    /// The same for the right.
    TheirsTint,
    /// Background of the result placeholder rows of an open block.
    OpenTint,
    /// The graph's role colours (ease pass, ADR-0063 G1): the cursor row's
    /// own branch — its line, its name, the route's first stop.
    RoleThis,
    /// A branch the cursor row's branch is built on (its route, trunk aside).
    RoleUnder,
    /// The trunk: the line branches merge into.
    RoleTrunk,
    /// Every other line: the grey the rest of the graph sinks into. Not a
    /// text colour and held to no contrast floor — being quiet is its job.
    RoleOther,
}

impl Token {
    /// Every token, in PROMPT.md §5's order.
    pub const ALL: [Token; TOKEN_COUNT] = [
        Token::Bg,
        Token::Surface,
        Token::SurfaceRaised,
        Token::Selection,
        Token::Border,
        Token::Text,
        Token::TextMuted,
        Token::Accent,
        Token::Added,
        Token::Removed,
        Token::Warning,
        Token::Info,
        Token::Ours,
        Token::Theirs,
        Token::Open,
        Token::OursTint,
        Token::TheirsTint,
        Token::OpenTint,
        Token::RoleThis,
        Token::RoleUnder,
        Token::RoleTrunk,
        Token::RoleOther,
    ];

    /// The token's name as PROMPT.md §5 and the config file spell it.
    pub const fn name(self) -> &'static str {
        match self {
            Token::Bg => "bg",
            Token::Surface => "surface",
            Token::SurfaceRaised => "surface-raised",
            Token::Selection => "selection",
            Token::Border => "border",
            Token::Text => "text",
            Token::TextMuted => "text-muted",
            Token::Accent => "accent",
            Token::Added => "added",
            Token::Removed => "removed",
            Token::Warning => "warning",
            Token::Info => "info",
            Token::Ours => "ours",
            Token::Theirs => "theirs",
            Token::Open => "open",
            Token::OursTint => "ours-tint",
            Token::TheirsTint => "theirs-tint",
            Token::OpenTint => "open-tint",
            Token::RoleThis => "role-this",
            Token::RoleUnder => "role-under",
            Token::RoleTrunk => "role-trunk",
            Token::RoleOther => "role-other",
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
    /// For a dark terminal background. The default.
    Dark,
    /// For a light terminal background.
    Light,
    /// Maximum contrast, minimum colour. Override only.
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
    depth: ColorDepth,
    chrome: [TokenColor; TOKEN_COUNT],
    lanes: [TokenColor; LANE_COUNT],
    /// The colours `chrome` was reduced from: the built-in table with his
    /// overrides written over it. What reads a truecolor hue — the
    /// `selection` lift, the pixel `+N` marker — reads it here, never the
    /// built-in table, so an override reaches them too.
    rgb: [Rgb; TOKEN_COUNT],
}

impl Theme {
    /// Resolve `kind` against what the terminal can do.
    ///
    /// Two things in [`Capabilities`] matter here and nothing else does:
    /// `no_color` (what the user asked for) and `color_depth` (what the
    /// terminal can do). Either one being absent of colour empties every
    /// token, including the lanes — DESIGN.md §4.6 puts lane identity on
    /// shape and dash patterns in that case, not on a colour Mergewright was told
    /// not to use.
    pub fn new(kind: ThemeKind, capabilities: &Capabilities) -> Theme {
        Theme::with_overrides(kind, capabilities, &[])
    }

    /// Which theme this is. For the diagnostics view and for config round-trips.
    pub fn kind(&self) -> ThemeKind {
        self.kind
    }

    /// How much colour this terminal has, or [`ColorDepth::None`] if
    /// `NO_COLOR` was set — the same reduction [`Theme::new`] applied to
    /// every token. Widgets read this to reach for a non-colour separator
    /// (DESIGN.md §4.6) where a background step alone would flatten at
    /// [`ColorDepth::Palette16`] (KNOWN_GAPS.md M0).
    pub fn color_depth(&self) -> ColorDepth {
        self.depth
    }

    /// This theme with every colour taken out: what `NO_COLOR` resolves to
    /// (every token and lane [`TokenColor::Default`], depth
    /// [`ColorDepth::None`]), for `#` (ease-pass G14). Bold, dim and the
    /// glyphs are the widgets' and stay.
    #[must_use]
    pub fn colourless(&self) -> Theme {
        Theme {
            kind: self.kind,
            depth: ColorDepth::None,
            chrome: [TokenColor::Default; TOKEN_COUNT],
            lanes: [TokenColor::Default; LANE_COUNT],
            rgb: self.rgb,
        }
    }

    /// The colour for one chrome token.
    pub fn token(&self, token: Token) -> TokenColor {
        self.chrome[token.index()]
    }

    /// The colour for one [`SyntaxRole`], from the same tokens as everything
    /// else — a diff's syntax colouring is chrome, not a second palette.
    ///
    /// At [`ColorDepth::Palette16`] and under `NO_COLOR` (`ColorDepth::None`)
    /// every role collapses to `Token::Text`: the sign column already carries
    /// added/removed, and a 16-colour terminal doesn't have enough distinct
    /// hues left over to also tell nine syntax roles apart (m1-the-graph/12c
    /// rule).
    #[must_use]
    pub fn syntax(&self, role: SyntaxRole) -> TokenColor {
        if matches!(self.depth, ColorDepth::Palette16 | ColorDepth::None) {
            return self.token(Token::Text);
        }

        self.token(match role {
            SyntaxRole::Plain | SyntaxRole::Function => Token::Text,
            SyntaxRole::Comment | SyntaxRole::Punctuation | SyntaxRole::Attribute => {
                Token::TextMuted
            }
            SyntaxRole::Keyword => Token::Accent,
            SyntaxRole::String | SyntaxRole::Number => Token::Info,
            SyntaxRole::Type => Token::Warning,
        })
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
