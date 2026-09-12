//! Truecolor down to 256 or 16 colours, by perceptual distance.
//!
//! PROMPT.md §5 and DESIGN.md §4.6 both rule out naive per-channel rounding:
//! rounding each of R, G and B to the nearest cube level moves a colour by
//! whatever the cube spacing happens to be in that corner of the space, which
//! is a different perceptual jump in each corner and can push two lanes onto
//! the same entry while a better one sits unused. So each colour is converted
//! to OKLab (Björn Ottosson, 2020) and matched against the palette by
//! Euclidean distance there, where equal distances mean roughly equal
//! perceived difference.
//!
//! # Which entries are candidates
//!
//! At 256 colours the search covers **16–255 only**: the 6×6×6 cube and the
//! 24-step grey ramp, whose values are fixed by the xterm palette. Entries
//! 0–15 are left out on purpose — every terminal lets the user redefine them,
//! so "nearest to index 1" means nearest to *a* red, not to the red that will
//! appear. Skipping them costs almost nothing (240 candidates remain) and
//! removes the one way this mapping could produce a colour Graft never chose.
//!
//! At 16 colours there is no such choice: 0–15 is the whole palette, so the
//! search uses the canonical xterm defaults and inherits their uncertainty.
//! It also inherits their coarseness — the palette has one mid-grey and a
//! handful of hues, and twelve lane hues do not survive the trip (see
//! KNOWN_GAPS.md; making lanes distinguishable at 16 colours is M1's job).

use graft_core::capabilities::Rgb;

/// How much more a chroma error costs than a lightness error of the same size,
/// when matching a colour to a palette entry.
///
/// Plain Euclidean OKLab distance is the right metric between two colours that
/// are *close*. These palettes are not close: the nearest 16-colour entry to
/// Graft's dark background is ΔE 0.19 away, ten times a just-noticeable
/// difference, and at that range lightness and hue errors stop being
/// comparable. Weighting them equally picked navy for `#14161B` and maroon for
/// the dark theme's `#333944` border — each a hair nearer than black, and each
/// visibly the wrong colour, because getting the *lightness* right does not
/// make a blue pane read as charcoal.
///
/// Doubling the chroma axes says what actually matters when a colour has to
/// travel this far: keep its identity — neutral stays neutral, a hue stays
/// that hue — and let lightness give. Two is the smallest round weight that
/// holds every near-neutral chrome token in all three themes on the grey axis
/// at 16 colours ([`tests`](super::tests) asserts it). At 256 colours, where
/// the cube is dense enough that the question barely arises, it moves six of
/// this module's forty-three colours by one cube step each, every one of them
/// towards the truer hue.
const CHROMA_WEIGHT: f32 = 2.0;

/// A colour in OKLab. Only ever an intermediate: distances are compared here,
/// results are always sRGB or a palette index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Oklab {
    lightness: f32,
    green_red: f32,
    blue_yellow: f32,
}

impl Oklab {
    /// Perceptual distance: OKLab is near enough uniform that this number
    /// means the same thing anywhere in the space, which is what makes a
    /// minimum-separation assertion in [`tests`](super::tests) meaningful.
    /// Nothing in the running program compares colours this way — only that
    /// assertion does — so it is compiled only for tests.
    #[cfg(test)]
    pub(super) fn distance(self, other: Self) -> f32 {
        self.squared_distance(other).sqrt()
    }

    /// Orders candidates identically to [`Oklab::distance`] and saves the
    /// square root in the inner loop.
    fn squared_distance(self, other: Self) -> f32 {
        let lightness = self.lightness - other.lightness;
        let green_red = (self.green_red - other.green_red) * CHROMA_WEIGHT;
        let blue_yellow = (self.blue_yellow - other.blue_yellow) * CHROMA_WEIGHT;
        lightness * lightness + green_red * green_red + blue_yellow * blue_yellow
    }

    /// How far this colour sits from the grey axis.
    #[cfg(test)]
    pub(super) fn chroma(self) -> f32 {
        self.green_red.hypot(self.blue_yellow)
    }
}

/// sRGB (8-bit, gamma-encoded) to OKLab, via the linear-light sRGB the
/// transform is defined over.
pub(super) fn oklab(color: Rgb) -> Oklab {
    oklab_from_linear(
        linearize(color.red),
        linearize(color.green),
        linearize(color.blue),
    )
}

/// OKLab from linear-light sRGB, for callers that have already left the
/// gamma-encoded world — the colour-vision-deficiency simulation in
/// [`tests`](super::tests) is the one such caller.
pub(super) fn oklab_from_linear(red: f32, green: f32, blue: f32) -> Oklab {
    // Ottosson's linear-sRGB → LMS matrix, then the cube root that makes the
    // space perceptually uniform, then LMS → Lab.
    let long = 0.412_221_47 * red + 0.536_332_55 * green + 0.051_445_995 * blue;
    let medium = 0.211_903_5 * red + 0.680_699_5 * green + 0.107_396_96 * blue;
    let short = 0.088_302_46 * red + 0.281_718_85 * green + 0.629_978_7 * blue;

    let long = long.cbrt();
    let medium = medium.cbrt();
    let short = short.cbrt();

    Oklab {
        lightness: 0.210_454_26 * long + 0.793_617_8 * medium - 0.004_072_047 * short,
        green_red: 1.977_998_5 * long - 2.428_592_2 * medium + 0.450_593_7 * short,
        blue_yellow: 0.025_904_037 * long + 0.782_771_77 * medium - 0.808_675_77 * short,
    }
}

/// Undo sRGB's transfer function: the matrix above is defined over
/// linear-light values, and applying it to gamma-encoded ones is the classic
/// way to get colours that are subtly wrong everywhere.
pub(super) fn linearize(channel: u8) -> f32 {
    let channel = f32::from(channel) / 255.0;
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// The nearest entry in the 240 fixed entries of the xterm 256-colour palette.
pub(super) fn nearest_256(color: Rgb) -> u8 {
    nearest(color, 16..=255, palette_256_entry)
}

/// The nearest of the 16 ANSI system colours, at their xterm default values.
pub(super) fn nearest_16(color: Rgb) -> u8 {
    nearest(color, 0..=15, |index| SYSTEM_16[usize::from(index)])
}

fn nearest(color: Rgb, candidates: std::ops::RangeInclusive<u8>, entry: impl Fn(u8) -> Rgb) -> u8 {
    let target = oklab(color);
    let mut best = *candidates.start();
    let mut best_distance = f32::INFINITY;

    for index in candidates {
        let distance = target.squared_distance(oklab(entry(index)));
        if distance < best_distance {
            best_distance = distance;
            best = index;
        }
    }

    best
}

/// The colour xterm shows for a palette index in 16–255.
///
/// 16–231 are a 6×6×6 RGB cube over the levels below; 232–255 are a grey ramp
/// from 8 to 238 in steps of 10. Indices under 16 have no fixed value and are
/// not accepted here.
fn palette_256_entry(index: u8) -> Rgb {
    /// The six cube levels. Not evenly spaced: the first step is 95, the rest
    /// are 40, which is exactly why per-channel rounding misbehaves near black.
    const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

    debug_assert!(index >= 16, "indices under 16 are terminal-configurable");

    if index >= 232 {
        let grey = 8 + (index - 232) * 10;
        return Rgb {
            red: grey,
            green: grey,
            blue: grey,
        };
    }

    let offset = usize::from(index - 16);
    Rgb {
        red: CUBE_LEVELS[offset / 36],
        green: CUBE_LEVELS[(offset / 6) % 6],
        blue: CUBE_LEVELS[offset % 6],
    }
}

/// xterm's default values for the sixteen system colours. The user's terminal
/// may show something else entirely for any of these; they are the best
/// available stand-in, not a promise.
const SYSTEM_16: [Rgb; 16] = [
    rgb(0x00, 0x00, 0x00), // 0  black
    rgb(0x80, 0x00, 0x00), // 1  red
    rgb(0x00, 0x80, 0x00), // 2  green
    rgb(0x80, 0x80, 0x00), // 3  yellow
    rgb(0x00, 0x00, 0x80), // 4  blue
    rgb(0x80, 0x00, 0x80), // 5  magenta
    rgb(0x00, 0x80, 0x80), // 6  cyan
    rgb(0xC0, 0xC0, 0xC0), // 7  white
    rgb(0x80, 0x80, 0x80), // 8  bright black
    rgb(0xFF, 0x00, 0x00), // 9  bright red
    rgb(0x00, 0xFF, 0x00), // 10 bright green
    rgb(0xFF, 0xFF, 0x00), // 11 bright yellow
    rgb(0x00, 0x00, 0xFF), // 12 bright blue
    rgb(0xFF, 0x00, 0xFF), // 13 bright magenta
    rgb(0x00, 0xFF, 0xFF), // 14 bright cyan
    rgb(0xFF, 0xFF, 0xFF), // 15 bright white
];

const fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}
