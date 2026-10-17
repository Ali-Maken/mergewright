//! The lane palette: twelve hues that identify branch lines, in every theme.
//!
//! # Why this lives outside the themes
//!
//! PROMPT.md §5 makes the lane palette independent of the UI theme: themes
//! restyle chrome, and "a theme must never be able to make two adjacent lanes
//! indistinguishable". That is enforced here by construction rather than by
//! review — a [`ThemeKind`](super::ThemeKind) has no field for lane colours to
//! override, so the only lane hues that exist are the ones in this file.
//!
//! ADR-0003 adds a second constraint. The pixel tier rasterises the graph with
//! a real alpha channel and never paints a background, so a lane colour has to
//! stay legible over a backdrop Graft cannot see — which rules out the very
//! dark and the very light ends of the scale regardless of theme.
//!
//! # Where the hues come from
//!
//! Every entry is taken verbatim from Paul Tol's colour schemes (SRON
//! technical note SRON/EPS/TN/09-002, "Colour schemes"), the standard
//! published set of colour-vision-deficiency-safe qualitative palettes: the
//! `vibrant`, `bright`, `muted` and `light` schemes. No hue here was invented
//! or hand-tweaked.
//!
//! Tol publishes no single scheme with twelve entries, and his schemes are
//! designed as sets, so taking the union and picking twelve needs its own
//! justification. The selection was made by maximising the *worst-case*
//! pairwise OKLab distance across normal vision and simulated protanopia,
//! deuteranopia and tritanopia (Machado et al. 2009, severity 1.0, clamped
//! back into gamut), over the candidates whose OKLab lightness falls in
//! `0.50..=0.86` — the band ADR-0003 implies. The order was then chosen to
//! maximise the worst *adjacent* pair, because adjacent lanes are the ones
//! drawn side by side.
//!
//! What that buys, measured by [`tests`](super::tests):
//!
//! - worst pair anywhere in the twelve, under the worst of the four vision
//!   models: OKLab ΔE 0.056 — `lane-2` against `lane-12`, which the ordering
//!   puts as far apart on screen as the palette allows;
//! - worst *adjacent* pair under the same worst case: ΔE 0.180, and ΔE 0.157
//!   across the wrap from `lane-12` back to `lane-1`.
//!
//! This is a numeric argument, not a validated one. Tol validated each scheme;
//! nobody has validated this union, and no human has looked at these twelve on
//! a real terminal yet. M1's visual pass does that (KNOWN_GAPS.md).

use graft_core::capabilities::{ColorDepth, Rgb};
use graft_graph::identity::BranchIdentity;

use super::{LANE_COUNT, Theme, Token, TokenColor, conflict, reduce};

/// One hue per lane, in token order: `LANE_HUES[0]` is PROMPT.md §5's
/// `lane-1`, which CONTEXT.md calls lane index 0 — the default branch.
///
/// Twelve is the whole palette; a repository with more concurrent lanes than
/// that reuses hues (see [`Theme::lane`](super::Theme::lane)).
pub(super) const LANE_HUES: [Rgb; 12] = [
    rgb(0x00, 0x77, 0xBB), // lane-1  Tol vibrant blue   — the default branch
    rgb(0xEE, 0x88, 0x66), // lane-2  Tol light orange
    rgb(0xAA, 0x33, 0x77), // lane-3  Tol bright purple
    rgb(0x77, 0xAA, 0xDD), // lane-4  Tol light blue
    rgb(0xDD, 0xCC, 0x77), // lane-5  Tol muted sand
    rgb(0x00, 0x99, 0x88), // lane-6  Tol vibrant teal
    rgb(0xCC, 0xBB, 0x44), // lane-7  Tol bright yellow
    rgb(0xCC, 0x33, 0x11), // lane-8  Tol vibrant red
    rgb(0xFF, 0xAA, 0xBB), // lane-9  Tol light pink
    rgb(0xCC, 0x66, 0x77), // lane-10 Tol muted rose
    rgb(0x88, 0xCC, 0xEE), // lane-11 Tol muted cyan
    rgb(0x99, 0x99, 0x33), // lane-12 Tol muted olive
];

const fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}

/// Lane colours, read off a resolved [`Theme`]: by index, by branch
/// identity, faded, and for a pixel renderer.
impl Theme {
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

    /// A lane's faded hue for cells off the selected thread
    /// (m8-polish-and-ship/17 item 3): the lane's truecolor hue mixed 45%
    /// toward `bg`, reduced to this depth. Terminals that ignore SGR 2 show
    /// no DIM, so at truecolor and 256 colours the fade is the colour itself.
    /// At 16 colours and with no colour there is no in-between hue to pick:
    /// the lane's own colour comes back and the caller keeps DIM
    /// ([`Theme::fades_by_hue`] says which).
    #[must_use]
    pub fn lane_muted(&self, lane: usize) -> TokenColor {
        if !self.fades_by_hue() {
            return self.lane(lane);
        }
        let hue = LANE_HUES[lane % LANE_COUNT];
        let bg = self.rgb[Token::Bg.index()];
        reduce(conflict::mix(hue, bg, 0.55), self.depth)
    }

    /// [`Theme::lane_muted`] for a branch, by identity — the slot
    /// [`Theme::branch`] reads.
    #[must_use]
    pub fn branch_muted(&self, identity: BranchIdentity) -> TokenColor {
        self.lane_muted(identity.slot(LANE_COUNT))
    }

    /// Whether off-thread lanes fade by hue ([`Theme::lane_muted`]) rather
    /// than by DIM: truecolor and 256 colours only.
    #[must_use]
    pub fn fades_by_hue(&self) -> bool {
        matches!(self.depth, ColorDepth::TrueColor | ColorDepth::Palette256)
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
                LANE_HUES.to_vec()
            }
        }
    }

    /// The muted hue (`text-muted`) a pixel renderer draws a `+N` marker in
    /// when no lane joins it: the theme's truecolor value (his override if
    /// he gave one), never reduced.
    #[must_use]
    pub fn raster_muted(&self) -> Rgb {
        self.rgb[Token::TextMuted.index()]
    }

    /// The colour a branch's lane draws in, from its identity alone — never
    /// from the column it happens to occupy (PROMPT.md §1.2), so a branch
    /// keeps its colour across scrolls, refreshes and column reuse.
    #[must_use]
    pub fn branch(&self, identity: BranchIdentity) -> TokenColor {
        self.lanes[identity.slot(LANE_COUNT)]
    }
}
