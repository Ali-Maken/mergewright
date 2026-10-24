//! ease-pass/30 D4: the renderer line says why pixels failed, fitted to the
//! row.

use super::*;
use crate::update::Status;
use mergewright_term::sanitize_line;

fn dropped(why: &str) -> AppState {
    AppState {
        renderer_override: Some(RenderingTier::Unicode),
        pixels_failed: Some(Box::new(sanitize_line(why))),
        ..AppState::default()
    }
}

fn line(state: &AppState, tier: RenderingTier, columns: u16) -> Option<String> {
    footer_notice(state, tier, columns).map(|text| text.as_str().to_owned())
}

#[test]
fn the_renderer_line_says_why_pixels_failed() {
    assert_eq!(
        line(&dropped("EINVAL:bad"), RenderingTier::Unicode, 100).as_deref(),
        Some("renderer: unicode \u{b7} pixels failed: EINVAL:bad (F3 cycles)")
    );
    // After one more `F3`: the same reason, the tier it is at.
    let mut ascii = dropped("EINVAL:bad");
    ascii.renderer_override = Some(RenderingTier::Ascii);
    assert_eq!(
        line(&ascii, RenderingTier::Ascii, 100).as_deref(),
        Some("renderer: ascii \u{b7} pixels failed: EINVAL:bad (F3 cycles)")
    );
    // No failure: `F3`'s own line, as before.
    let plain = AppState {
        renderer_override: Some(RenderingTier::Unicode),
        ..AppState::default()
    };
    assert_eq!(
        line(&plain, RenderingTier::Unicode, 100).as_deref(),
        Some("renderer: unicode (F3 cycles)")
    );
}

#[test]
fn the_renderer_line_fits_the_row() {
    let long = "x".repeat(120);
    let state = dropped(&long);
    let text = line(&state, RenderingTier::Unicode, 80).expect("a notice");
    let cells = 80 - 1 - usize::from(mergewright_ui::graph_screen::RIGHT_MARGIN);
    assert!(text.chars().count() <= cells, "{text:?}");
    assert!(
        text.starts_with("renderer: unicode \u{b7} pixels failed: x"),
        "{text:?}"
    );
    assert!(
        text.contains("\u{2026} (F3 cycles)"),
        "cut with the mark: {text:?}"
    );
    // Too narrow for a reason: today's line.
    assert_eq!(
        line(&state, RenderingTier::Unicode, 30).as_deref(),
        Some("renderer: unicode (F3 cycles)")
    );
}

#[test]
fn a_busy_status_still_wins_the_row() {
    let mut state = dropped("EINVAL:bad");
    state.status = Status::Busy("fetching".to_owned());
    let text = line(&state, RenderingTier::Unicode, 100).expect("a notice");
    assert!(text.starts_with("fetching"), "{text:?}");
    assert!(!text.contains("pixels failed"), "{text:?}");
}
