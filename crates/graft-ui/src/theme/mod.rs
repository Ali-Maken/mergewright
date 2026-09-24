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
use graft_graph::identity::BranchIdentity;

use crate::syntax::SyntaxRole;

/// The number of chrome tokens PROMPT.md §5 names, lanes excluded, plus
/// [`Token::Selection`] (m5-history-and-ghost-preview/44): PROMPT.md §5 does
/// not name it, but `surface-raised` cannot keep doing both jobs (see
/// [`Token::Selection`]'s doc), so the count grew by one to hold the split.
///
/// Plus the six conflict-resolver roles (ADR-0044 §1): three hues and their
/// three tints.
pub const TOKEN_COUNT: usize = 18;

/// How much of a hue a conflict tint keeps over `bg` (ADR-0044 §1: "medium").
/// One α for all three tints.
pub const TINT_ALPHA: f32 = 0.30;

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
        }
    }

    /// Whether this token is a conflict tint: a background, dropped where
    /// tints are ([`Theme::tint`]).
    #[must_use]
    pub const fn is_tint(self) -> bool {
        matches!(self, Token::OursTint | Token::TheirsTint | Token::OpenTint)
    }

    /// Whether this is one of the six conflict tokens (ADR-0044 §1), which
    /// reduce through [`pinned`] and the cube-only search, not [`reduce`].
    const fn is_conflict(self) -> bool {
        matches!(self, Token::Ours | Token::Theirs | Token::Open) || self.is_tint()
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
            resolved_chrome[token.index()] = if token.is_conflict() {
                reduce_conflict(kind, token, chrome[token.index()], depth)
            } else {
                reduce(chrome[token.index()], depth)
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
        }
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

    /// The colour for one chrome token.
    pub fn token(&self, token: Token) -> TokenColor {
        self.chrome[token.index()]
    }

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

    /// Lane hues for a pixel renderer, indexed like [`Theme::branch`]'s slots.
    ///
    /// An image is not limited to the terminal's ANSI palette, so these are
    /// the theme's truecolor hues at any depth — except where colour is off
    /// (`NO_COLOR`, or a terminal with none): then every lane draws in the
    /// one muted hue, and shape carries lane identity as it does in text.
    #[must_use]
    pub fn raster_lanes(&self) -> Vec<Rgb> {
        match self.depth {
            ColorDepth::None => vec![self.raster_muted()],
            ColorDepth::TrueColor | ColorDepth::Palette256 | ColorDepth::Palette16 => {
                lanes::LANE_HUES.to_vec()
            }
        }
    }

    /// The muted hue (`text-muted`) a pixel renderer draws a `+N` marker in
    /// when no lane joins it: the theme's truecolor value, never reduced.
    #[must_use]
    pub fn raster_muted(&self) -> Rgb {
        self.kind.chrome()[Token::TextMuted.index()]
    }

    /// The colour a branch's lane draws in, from its identity alone — never
    /// from the column it happens to occupy (PROMPT.md §1.2), so a branch
    /// keeps its colour across scrolls, refreshes and column reuse.
    #[must_use]
    pub fn branch(&self, identity: BranchIdentity) -> TokenColor {
        self.lanes[identity.slot(LANE_COUNT)]
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
fn reduce_conflict(kind: ThemeKind, token: Token, hue: Rgb, depth: ColorDepth) -> TokenColor {
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
