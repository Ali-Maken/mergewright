//! The history place on real `git` (m7-power-tools/21): what a trace does
//! at the edges — a depth-1 clone, a broken `.git/shallow`. The journeys
//! are `m7_walk::history`.

use graft::update::place::{FileLoad, TraceLoad};
use graft_core::event::KeyCode;
use graft_git::trace::{SHALLOW_END, TraceEnd};

use crate::m7_walk::blame::{blame, on_line_22, open_on_the_file, session};
use crate::m7_walk::history::{history, traced};

/// A `git clone --depth 1` of the session fixture: `main` alone.
fn depth_one() -> (graft_fixtures::Scratch, tempfile::TempDir) {
    let source = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Session);
    let clone = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let url = format!("file://{}", source.path().display());
    crate::app_harness::git(
        clone.path(),
        &["clone", "--quiet", "--depth", "1", &url, "clone"],
    );
    (source, clone)
}

/// `F` on a depth-1 clone: the trace ends at the clone's edge, said in the
/// blame place's words; no panic.
#[test]
fn a_depth_one_clone_shows_the_end_row_alone() {
    let (_source, dir) = depth_one();
    let clone = dir.path().join("clone");
    let (app, state) = open_on_the_file(&clone, 187, 50, false);
    let state = app.key(&state, KeyCode::Char('F'));
    let state = traced(&app, state);
    let place = history(&state);
    let TraceLoad::Ready { rows, end } = &place.trace else {
        panic!("not traced: {:?}", place.trace);
    };
    assert_eq!(end, &TraceEnd::GitStops(SHALLOW_END.to_owned()), "{rows:?}");
    // The reader gives no rows on a depth-1 clone (review 18bcd Minor 6).
    assert!(rows.is_empty(), "no rows: {rows:?}");
    assert_eq!(place.selected, None);
    assert_eq!(place.file, FileLoad::Idle);
    let back = app.key(&state, KeyCode::Escape);
    assert!(!back.trail.is_live(), "Esc goes back");
    app.close();
}

/// A `.git/shallow` that is a directory: git can't read the clone's edge,
/// and the trace fails — drawn as the list's failure, not a panic, not the
/// block fallback.
#[test]
fn a_trace_error_is_drawn_as_a_failure() {
    let scratch = session();
    let repo = scratch.path();
    let (app, state) = on_line_22(repo, 140, 40);
    let state = blame(&app, &state);
    std::fs::create_dir(repo.join(".git").join("shallow"))
        .unwrap_or_else(|why| panic!("a directory for .git/shallow: {why}"));
    let state = app.key(&state, KeyCode::Char('L'));
    let state = traced(&app, state);
    let place = history(&state);
    assert!(
        matches!(&place.trace, TraceLoad::Failed(words) if !words.is_empty()),
        "{:?}",
        place.trace
    );
    let back = app.key(&state, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "Esc: blame again");
    app.close();
}
