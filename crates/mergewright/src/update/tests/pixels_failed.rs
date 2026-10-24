//! ease-pass/30: `AppEvent::PixelsFailed` drops the pixel tier once, as `F3`
//! would, and `F3` is the way back.

use mergewright_term::sanitize_line;

use super::*;

fn why() -> mergewright_term::SanitizedText {
    sanitize_line("EINVAL:bad")
}

fn at_pixels() -> AppState {
    AppState {
        settled_tier: Some(RenderingTier::Pixel),
        ..AppState::default()
    }
}

#[test]
fn pixels_failed_drops_to_unicode_once() {
    let (dropped, commands) = update(&at_pixels(), AppEvent::PixelsFailed(why()));
    assert_eq!(dropped.renderer_override, Some(RenderingTier::Unicode));
    assert_eq!(dropped.pixels_failed.as_deref(), Some(&why()));
    assert!(commands.is_empty(), "{commands:?}");

    // A second one (a late reply) changes nothing, nor does its reason.
    let (again, commands) = update(
        &dropped,
        AppEvent::PixelsFailed(sanitize_line("a second reason")),
    );
    assert_eq!(again, dropped);
    assert!(commands.is_empty(), "{commands:?}");

    // `F3` to ASCII keeps the reason; `F3` again clears it with the override.
    let (ascii, _) = update(&dropped, key(KeyCode::Function(3)));
    assert_eq!(ascii.renderer_override, Some(RenderingTier::Ascii));
    assert_eq!(ascii.pixels_failed.as_deref(), Some(&why()));
    // A failure reported at ASCII (after `F3`) changes nothing either.
    let (late, _) = update(&ascii, AppEvent::PixelsFailed(why()));
    assert_eq!(late, ascii);
    let (top, _) = update(&ascii, key(KeyCode::Function(3)));
    assert_eq!(top.renderer_override, None);
    assert_eq!(top.pixels_failed, None);

    // And pixels again can fail again: each retry is his key.
    let (again, _) = update(&top, AppEvent::PixelsFailed(why()));
    assert_eq!(again.renderer_override, Some(RenderingTier::Unicode));
}

#[test]
fn pixels_failed_off_the_pixel_tier_changes_nothing() {
    for settled in [RenderingTier::Unicode, RenderingTier::Ascii] {
        let state = AppState {
            settled_tier: Some(settled),
            ..AppState::default()
        };
        let (next, commands) = update(&state, AppEvent::PixelsFailed(why()));
        assert_eq!(next, state);
        assert!(commands.is_empty());
    }
}
