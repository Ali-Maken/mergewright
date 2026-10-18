//! `Esc` then `G` on a history longer than one row request, over the real
//! job pool (m6-conflicts/85 A). The walk rests between asks; `Esc` with
//! nothing open used to cancel it — `stopped at N rows` — and the `G` after
//! drew a blank graph pane (review m6-usability-2 A).
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use mergewright::session::Clock;
use mergewright::update::AppState;
use mergewright_core::event::KeyCode;

use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame_at, settled_frame};

/// The frame at 120×40, dated against the corpus clock.
fn at(state: &AppState) -> Vec<String> {
    frame_at(
        state,
        120,
        40,
        Some(Clock::Fixed(mergewright_fixtures::CORPUS_NOW)),
    )
}

#[test]
fn esc_then_g_on_a_long_history_draws_its_last_rows() {
    // 200 commits: the first screen's request covers fewer.
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Linear);
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = app.resize(&state, 120, 40);
    let state = app.pump(state, "the working tree", |state| {
        state.working_tree.is_some()
    });
    let rested = settled_frame(&app, state);
    let before = at(&rested);
    // Every row laid out, but not every row on hand: the last screen is
    // past the window the first request brought, so `G` has to ask for it.
    assert!(before[0].contains("200 commits"), "{}", before[0]);
    let held = rested
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .map(|window| window.first + window.rows.len());
    assert!(
        held.is_some_and(|end| end < 200),
        "the first request holds fewer rows than history has: {held:?}"
    );

    let escaped = app.key(&rested, KeyCode::Escape);
    let after_esc = at(&escaped);
    assert_eq!(
        after_esc[0], before[0],
        "`Esc` with nothing open changes nothing"
    );
    assert!(escaped.awaiting.is_some(), "the walk is not cancelled");

    let bottom = app.key(&escaped, KeyCode::Char('G'));
    let bottom = app.pump(bottom, "the last rows", |state| {
        state
            .history
            .as_ref()
            .is_some_and(|history| history.jump.is_none() && history.selected().is_some())
    });
    let bottom = settled_frame(&app, bottom);
    let frame = at(&bottom);
    let text = frame.join("\n");
    assert!(frame[0].contains("200 commits"), "{}", frame[0]);
    assert!(!text.contains("stopped"), "{text}");
    let root = git(path, &["rev-list", "--max-parents=0", "HEAD"]);
    let root = root.trim().get(..7).expect("a full object id");
    assert!(text.contains(root), "the root commit is drawn:\n{text}");
    let history = bottom.history.as_ref().expect("a walk");
    assert_eq!(history.cursor, 199, "the cursor is on the last row");
    app.close();

    check_shot(
        "m6/esc-then-g-120x40.txt",
        &format!(
            "# m6-conflicts/85 A: `esc G` on a 200-commit history (fixture `linear`), 120x40, \
             after the first screen settled. Colour unverified (character grid only).\n\
             # What right looks like: the header reads `200 commits`, not `stopped`; the graph \
             pane's last rows are commits (sha, subject), not blank; the bar is the row's own keys.\n\
             {text}\n"
        ),
    );
}
