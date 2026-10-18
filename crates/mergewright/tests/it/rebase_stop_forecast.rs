//! The rebase preview's chained stop forecast, drawn (m7-power-tools/05):
//! real `git`, the real job pool, `r` on `main`'s row, the real
//! `mergewright_ops::preview::plan`, the real `view::view`.

use mergewright::session::Clock;
use mergewright_core::event::KeyCode;

use crate::app_harness::{App, git};
use crate::preview::{FORK_EPOCH, dated_commit, frame_at, on_main, opened};

/// `main` and `feature` fork after `shared.txt` is added; each then changes
/// it, `feature` in the third of its five commits. Rebasing `feature` onto
/// `main` picks two cleanly and stops at `feature edits shared`.
fn stops_at_step_three() -> mergewright_fixtures::Scratch {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let path = scratch.path();
    dated_commit(path, "shared.txt", "base\n", "add shared", 1);
    git(path, &["branch", "feature"]);
    dated_commit(path, "shared.txt", "main's line\n", "main edits shared", 2);
    git(path, &["switch", "feature"]);
    dated_commit(path, "work.txt", "one\n", "start the feature", 3);
    dated_commit(path, "more.txt", "one\n", "more of the feature", 4);
    dated_commit(
        path,
        "shared.txt",
        "feature's line\n",
        "feature edits shared",
        5,
    );
    dated_commit(path, "more.txt", "two\n", "grow the feature", 6);
    dated_commit(path, "work.txt", "two\n", "finish the feature", 7);
    scratch
}

/// `docs/shots/m7/rebase-stop-forecast-80x24.txt`. What right looks like:
/// the box's forecast row reads `may stop at step 3 of 5 (pick <sha>)
/// · not checked past it` — at 80 columns the paths and the subject are
/// cut, the step never — and `feature` is named whole. Colour unverified
/// (rule 5).
#[test]
fn capture_rebase_stop_forecast() {
    let scratch = stops_at_step_three();
    let (app, state) = App::open(scratch.path());
    let state = app.resize(&state, 80, 24);
    let mut state = opened(&app, state);
    let mut guard = 0;
    while !on_main(&state) {
        assert!(guard < 20, "never found main's row");
        state = app.key(&state, KeyCode::Char('j'));
        guard += 1;
    }
    let state = app.key(&state, KeyCode::Char('r'));
    let state = app.pump(state, "the preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    // A minute after the newest commit (minute 7): `FORK_NOW` is minute 6,
    // which would draw that commit as `future`.
    let frame = frame_at(&state, 80, 24, Some(Clock::Fixed(FORK_EPOCH + 8 * 60)));
    let shot = frame.join("\n");
    let stop: Vec<&String> = frame
        .iter()
        .filter(|row| row.contains("may stop at step 3 of 5"))
        .collect();
    assert_eq!(stop.len(), 1, "{shot}");
    assert!(stop[0].contains("not checked past it"), "{shot}");
    let body = format!("{shot}\n");
    crate::captures::check_shot("m7/rebase-stop-forecast-80x24.txt", &body);
    app.close();
}
