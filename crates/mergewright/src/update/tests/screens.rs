//! `update` tests: screens. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// The pool announcing a walk the screen asked for.
///
/// `update` names the walk when it builds `Command::ReadHistory`, so a
/// `Started` is only ever taken under the name the screen is already holding
/// — feeding one out of nowhere is a walk nobody asked for
/// (m5-history-and-ghost-preview/05).
pub(super) fn walk_started(state: &AppState, id: JobId) -> (AppState, Vec<Command>) {
    let asked = AppState {
        awaiting: Some(id),
        ..state.clone()
    };
    update(&asked, AppEvent::Started(id, Task::ReadHistory))
}

#[test]
fn mergewright_opens_on_the_graph_and_f2_swaps_to_the_status_screen_and_back() {
    let state = AppState::default();
    assert_eq!(state.screen, Screen::Graph);

    let (status, commands) = update(&state, key(KeyCode::Function(2)));
    assert_eq!(status.screen, Screen::Status);
    assert!(commands.is_empty(), "{commands:?}");

    let (graph, _) = update(&status, key(KeyCode::Function(2)));
    assert_eq!(graph.screen, Screen::Graph);
}

/// `F3` from `settled` and on, `presses` times: the override after each.
fn f3_presses(settled: Option<RenderingTier>, presses: usize) -> Vec<Option<RenderingTier>> {
    let mut state = AppState {
        settled_tier: settled,
        ..AppState::default()
    };
    (0..presses)
        .map(|_| {
            let (next, commands) = update(&state, key(KeyCode::Function(3)));
            assert!(commands.is_empty(), "a view-time flag: {commands:?}");
            state = next;
            state.renderer_override
        })
        .collect()
}

#[test]
fn f3_on_a_unicode_terminal_toggles_unicode_and_ascii() {
    // No session (`settled_tier` None) counts as Unicode, the same way.
    for settled in [Some(RenderingTier::Unicode), None] {
        assert_eq!(
            f3_presses(settled, 3),
            vec![Some(RenderingTier::Ascii), None, Some(RenderingTier::Ascii)],
            "back at the top the override clears: nothing extra to say"
        );
    }
}

#[test]
fn f3_on_an_ascii_only_terminal_does_nothing() {
    assert_eq!(f3_presses(Some(RenderingTier::Ascii), 2), vec![None, None]);
}

#[test]
fn at_the_pixel_tier_history_asks_for_whole_strips() {
    let state = AppState {
        settled_tier: Some(RenderingTier::Pixel),
        ..AppState::default()
    };
    let (started, _) = walk_started(&state, JobId::new(1));
    let history = started.history.expect("a walk started");
    assert_eq!(
        history.strip_rows,
        Some(mergewright_graph::raster::cache::STRIP_ROWS)
    );

    let (unicode, _) = walk_started(&AppState::default(), JobId::new(1));
    assert_eq!(unicode.history.expect("a walk started").strip_rows, None);
}

#[test]
fn f3_cycles_pixels_unicode_ascii_and_back_to_whole_strips() {
    let state = AppState {
        settled_tier: Some(RenderingTier::Pixel),
        ..AppState::default()
    };
    let (started, _) = walk_started(&state, JobId::new(1));
    let strip_rows = |state: &AppState| state.history.as_ref().and_then(|h| h.strip_rows);

    let (unicode, _) = update(&started, key(KeyCode::Function(3)));
    assert_eq!(unicode.tier(), RenderingTier::Unicode);
    assert_eq!(strip_rows(&unicode), None);

    let (ascii, _) = update(&unicode, key(KeyCode::Function(3)));
    assert_eq!(ascii.tier(), RenderingTier::Ascii);
    assert_eq!(strip_rows(&ascii), None);

    let (pixel, _) = update(&ascii, key(KeyCode::Function(3)));
    assert_eq!(pixel.renderer_override, None, "the settled tier again");
    assert_eq!(pixel.tier(), RenderingTier::Pixel);
    assert_eq!(
        strip_rows(&pixel),
        Some(mergewright_graph::raster::cache::STRIP_ROWS),
        "requests pad to whole strips again"
    );
}
