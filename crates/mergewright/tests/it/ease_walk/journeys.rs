//! The note's nine journeys, walked in the real app and counted against the
//! `journeys` fence through [`super::counted`]: one key over fails. Each
//! walk starts where the note says (target under the cursor; moving there
//! is navigation, not counted, ux-principles §1.3) and asserts where it
//! lands. Typed text counts 1.

use mergewright_core::event::KeyCode;
use mergewright_git::state::InProgress;

use super::commit::{landed, opened as commit_opened, settled};
use super::counted;
use super::fixture::{self, mock, with_edits};
use super::flow::{
    at_clash, committed, head_parents, pick, previewed, reviewed, stop_after_y, stopped, unmerged,
};
use super::screen::{cursor_on, cursor_on_head, drawn, key_at_rest, line_with, open, typed};
use crate::app_harness::git;

/// `c`, typed subject, `⏎`: HEAD one commit ahead, the cursor on it.
#[test]
fn commit_all_subject() {
    let scratch = mock();
    with_edits(scratch.path());
    let repo = scratch.path();
    let (app, state) = open(repo, 120);
    let before = git(repo, &["rev-parse", "HEAD"]);
    let state = commit_opened(&app, &state);
    let state = typed(&app, &state, "fix toc");
    let state = landed(&app, app.key(&state, KeyCode::Enter));
    counted("commit-all-subject", &["c", "type", "⏎"]);
    assert_eq!(
        git(repo, &["rev-parse", "HEAD~1"]),
        before,
        "one commit ahead"
    );
    assert_eq!(super::screen::head_subject(repo), "fix toc");
    assert!(cursor_on_head(&state), "the cursor on the new commit");
    app.close();
}

/// `c`, subject, `⇥`, body, `esc`, `⏎`: the commit carries the body.
#[test]
fn commit_with_body() {
    let scratch = mock();
    with_edits(scratch.path());
    let repo = scratch.path();
    let (app, state) = open(repo, 120);
    let state = commit_opened(&app, &state);
    let state = typed(&app, &state, "fix toc");
    let state = app.key(&state, KeyCode::Tab);
    let state = typed(&app, &state, "the anchors moved");
    let state = app.key(&state, KeyCode::Escape);
    assert!(
        state.commit_editor.is_some(),
        "esc in the body goes to the message"
    );
    let state = landed(&app, app.key(&state, KeyCode::Enter));
    counted("commit-with-body", &["c", "type", "⇥", "type", "esc", "⏎"]);
    let message = git(repo, &["log", "-1", "--format=%B"]);
    assert_eq!(message.trim_end(), "fix toc\n\nthe anchors moved");
    assert!(cursor_on_head(&state));
    app.close();
}

/// `c`, subject, `⇥ ⇥` to the files, `space` leaves the first out, `⏎`.
#[test]
fn commit_leave_first() {
    let scratch = mock();
    with_edits(scratch.path());
    let repo = scratch.path();
    let (app, state) = open(repo, 120);
    let state = commit_opened(&app, &state);
    let state = typed(&app, &state, "two of three");
    let state = app.key(&app.key(&state, KeyCode::Tab), KeyCode::Tab);
    let state = settled(&app, app.key(&state, KeyCode::Char(' ')));
    let state = landed(&app, app.key(&state, KeyCode::Enter));
    counted("commit-leave-first", &["c", "type", "⇥", "⇥", "space", "⏎"]);
    let files = git(repo, &["show", "--name-only", "--format=", "HEAD"]);
    assert_eq!(files.lines().count(), 2, "one of three left out: {files}");
    assert!(!files.contains(".env"));
    assert!(cursor_on_head(&state));
    app.close();
}

/// `m y o y y` on `integration`: a two-parent HEAD, the cursor on it.
#[test]
fn merge_one_clash() {
    let (scratch, app, state) = super::flow::opened();
    let state = previewed(&app, state, fixture::INTEGRATION);
    let state = stop_after_y(&app, &state);
    crate::clash_stops::on_the_first_clash(&state);
    let state = pick(&app, &state, 'o');
    let state = reviewed(&app, &state);
    let state = committed(&app, &state);
    counted("merge-one-clash", &["m", "y", "o", "y", "y"]);
    assert_eq!(head_parents(scratch.path()), 2, "a merge commit");
    assert!(cursor_on_head(&state), "the cursor on the merge");
    app.close();
}

/// `m y` on `ops/ci` (no clash): lands on the merge commit.
#[test]
fn merge_clean() {
    let (scratch, app, state) = super::flow::opened();
    let state = previewed(&app, state, fixture::CI_MORE);
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the merge");
    let state = app.pump(state, "the landing", |state| !state.land_on_head);
    let state = crate::captures::at_rest(&app, state);
    counted("merge-clean", &["m", "y"]);
    assert_eq!(head_parents(scratch.path()), 2);
    assert!(cursor_on_head(&state));
    app.close();
}

/// `m y o o n o y y` on `pay-307`: three clashes across two files.
#[test]
fn merge_three_clashes() {
    let (scratch, app, state) = super::flow::opened();
    let state = previewed(&app, state, fixture::PAY_307);
    let mut state = stop_after_y(&app, &state);
    for key in ['o', 'o', 'n', 'o'] {
        state = pick(&app, &state, key);
    }
    let state = reviewed(&app, &state);
    let state = committed(&app, &state);
    counted(
        "merge-three-clashes",
        &["m", "y", "o", "o", "n", "o", "y", "y"],
    );
    assert_eq!(head_parents(scratch.path()), 2);
    assert!(unmerged(scratch.path()).is_empty());
    assert!(cursor_on_head(&state));
    app.close();
}

/// `u` after a pick: the clash open again, the cursor on it.
#[test]
fn undo_a_pick() {
    let (scratch, app, state) = stopped("integration");
    let state = pick(&app, &state, 'o');
    let state = pick(&app, &state, 'u');
    counted("undo-a-pick", &["u"]);
    assert_eq!(at_clash(&state), ("report.md".to_owned(), Some(0)));
    assert!(
        std::fs::read_to_string(scratch.path().join("report.md"))
            .expect("readable")
            .contains("<<<<<<<")
    );
    app.close();
}

/// `esc ⏎` on the clash screen of a merge: paused on the banner, back on
/// the first open clash.
#[test]
fn pause_and_resume_on_the_merge() {
    let (_scratch, app, state) = stopped("integration");
    let paused = app.key(&state, KeyCode::Escape);
    assert!(paused.banner_focus && paused.resolver.is_none());
    let back = resumed(&app, &paused);
    counted("pause-and-resume", &["esc", "⏎"]);
    crate::clash_stops::on_the_first_clash(&back);
    app.close();
}

/// The same on the rebase that clashes (`ali/tidy` onto `integration`).
#[test]
fn pause_and_resume_on_the_rebase() {
    let scratch = mock();
    git(scratch.path(), &["switch", "--quiet", "ali/tidy"]);
    let (app, state) = open(scratch.path(), 120);
    let state = crate::clash_stops::stop_by(&app, state, b"integration", 'r', InProgress::Rebase);
    let paused = app.key(&state, KeyCode::Escape);
    assert!(paused.banner_focus && paused.resolver.is_none());
    let back = resumed(&app, &paused);
    counted("pause-and-resume", &["esc", "⏎"]);
    crate::clash_stops::on_the_first_clash(&back);
    app.close();
}

fn resumed(
    app: &crate::app_harness::App,
    paused: &mergewright::update::AppState,
) -> mergewright::update::AppState {
    app.pump(
        app.key(paused, KeyCode::Enter),
        "the first open clash",
        |state| {
            state
                .resolver
                .as_ref()
                .is_some_and(|open| open.file.is_some() && open.set_job.is_none())
        },
    )
}

/// `space` on the folded PR merge opens it: `▾`, its commits drawn.
#[test]
fn open_folded_merge() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 200);
    let state = cursor_on(&app, state, fixture::PR_MERGE);
    assert!(
        !drawn(&state)
            .iter()
            .any(|line| line.contains(fixture::PR_COMMITS[0]))
    );
    let state = key_at_rest(&app, &state, ' ');
    counted("open-folded-merge", &["space"]);
    let frame = drawn(&state);
    assert!(line_with(&frame, fixture::PR_MERGE).contains('▾'));
    for subject in fixture::PR_COMMITS {
        line_with(&frame, subject);
    }
    app.close();
}
