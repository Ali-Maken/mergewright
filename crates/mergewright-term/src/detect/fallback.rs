//! The fallback chains of DESIGN.md §4.1 step 3: cell size, theme, and the
//! yes/no/silent answer to a probed capability.

use mergewright_core::capabilities::{
    CellSize, CellSizeSource, ProbeOutcome, Rgb, Supported, Theme, ThemeHint, ThemeSource,
};

use super::{ASSUMED_CELL_SIZE, TerminalHost, WindowPixels, probe, supported};

/// The cell-size fallback chain of DESIGN.md §4.1 step 3, in order: what the
/// terminal reported, what the kernel knows, and finally a flagged guess.
pub(super) fn cell_size(replies: &probe::ProbeReplies, host: &mut dyn TerminalHost) -> CellSize {
    measured_cell_size(replies, host).unwrap_or(CellSize {
        width: ASSUMED_CELL_SIZE.0,
        height: ASSUMED_CELL_SIZE.1,
        source: CellSizeSource::Assumed,
    })
}

/// The two measured rungs of the chain: the terminal's reply, then the
/// kernel. `None` when neither measured anything, which leaves the caller to
/// decide what to assume — the default at startup, the last value on resize.
pub(super) fn measured_cell_size(
    replies: &probe::ProbeReplies,
    host: &mut dyn TerminalHost,
) -> Option<CellSize> {
    if let Some((width, height)) = replies.cell_size {
        return Some(CellSize {
            width,
            height,
            source: CellSizeSource::Reported,
        });
    }
    let (width, height) = host.window_pixels().and_then(WindowPixels::cell_size)?;
    Some(CellSize {
        width,
        height,
        source: CellSizeSource::WindowSize,
    })
}

/// The light/dark hint from the reported background, or dark flagged as the
/// assumption it is (DESIGN.md §4.1 step 3).
pub(super) fn theme(background: Option<Rgb>) -> ThemeHint {
    background.map_or(
        ThemeHint {
            theme: Theme::Dark,
            source: ThemeSource::Assumed,
        },
        reported_theme,
    )
}

/// The hint a reported background gives.
pub(super) fn reported_theme(background: Rgb) -> ThemeHint {
    ThemeHint {
        theme: if is_light(background) {
            Theme::Light
        } else {
            Theme::Dark
        },
        source: ThemeSource::Background(background),
    }
}

/// Whether a background reads as light. Rec. 709 luma against a mid threshold:
/// this only ever picks one of two themes, so the extra accuracy of a full
/// sRGB-linearised luminance would change nothing but the arithmetic.
fn is_light(color: Rgb) -> bool {
    let luma = 0.2126 * f32::from(color.red)
        + 0.7152 * f32::from(color.green)
        + 0.0722 * f32::from(color.blue);
    luma > 127.5
}

/// As [`supported`], but silence only counts as an answer when the round was
/// fenced: an unfenced round proves nothing about anything.
pub(super) fn answered(outcome: &ProbeOutcome, answer: Option<bool>) -> Supported {
    match (outcome, answer) {
        (_, Some(state)) => supported(Some(state)),
        (ProbeOutcome::Answered, None) => Supported::No,
        (_, None) => Supported::Silent,
    }
}
