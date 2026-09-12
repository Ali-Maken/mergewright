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

use graft_core::capabilities::Rgb;

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
