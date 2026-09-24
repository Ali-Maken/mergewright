//! The captures m6-conflicts/12 ends with: where the app lands the user
//! after each event of a live operation, drawn from real repositories over
//! the real job pool. Each file is checked against what the code draws
//! (`GRAFT_UPDATE_SHOTS=1` rewrites it). 187x50 is the user's own terminal.
//!
//! Colour — the focused banner's reverse video — is not in a character
//! grid: it ships **unverified** (CLAUDE.md rule 5).

use graft::session::Clock;
use graft::update::{AppState, Status};
use graft_core::event::KeyCode;

use super::{
    CONFLICT_NOW, about_to_conflict, git, killed_mid_rebase, landed, landed_in_the_resolver,
    stopped_on_the_conflict,
};
use crate::app_harness::App;
use crate::captures::{check_shot, frame_at, without_invented_id};

/// The frame at `columns`x`rows`, resized first so the layout is the one a
/// terminal that size gets.
fn drawn(app: &App, state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    let sized = app.resize(state, columns, rows);
    frame_at(&sized, columns, rows, Some(Clock::Fixed(CONFLICT_NOW)))
}

/// Pumps until nothing on the frame is still loading, so the shutter never
/// catches a `reading…` or `sorting…` that a second later is gone.
fn settled(app: &App, state: AppState, what: &str) -> AppState {
    app.pump(state, what, |state| {
        crate::captures::frame(state, 120, 40)
            .iter()
            .all(|row| !row.contains("reading\u{2026}") && !row.contains("sorting\u{2026}"))
    })
}

/// The graph screen's footer: the row the stop line and the hints share.
/// With the changes pane open below the graph it is not the frame's last.
fn footer<'a>(frame: &'a [String], marker: &str) -> &'a str {
    frame
        .iter()
        .rev()
        .find(|row| row.contains(marker))
        .unwrap_or_else(|| panic!("no row holds {marker:?}:\n{}", frame.join("\n")))
}

/// m6-conflicts/18: the stop lands in the resolver on the first
/// conflicted path, the header still `MERGING`.
#[test]
fn capture_a_merge_that_stops() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = settled(
        &app,
        landed_in_the_resolver(&app, state),
        "the landed resolver",
    );
    let first = state
        .resolver
        .as_ref()
        .expect("the stop opened the resolver")
        .path
        .display()
        .to_string();
    assert_eq!(state.status, Status::Ready, "a stop is not a status");

    for (columns, rows) in [(80u16, 24u16), (120, 40), (187, 50)] {
        let frame = drawn(&app, &state, columns, rows);
        let body = frame.join("\n");
        assert!(frame[0].trim_end().ends_with("MERGING"), "{body}");
        assert!(
            frame
                .iter()
                .any(|row| row.contains(&format!("\u{25b8} {first}"))),
            "the first file is marked: {body}"
        );
        let bar = frame.last().expect("a frame has rows");
        assert!(bar.trim_start().starts_with("o yours"), "{bar}");
        check_shot(
            &format!("m6/fix-stop-lands-in-resolver-{columns}x{rows}.txt"),
            &format!("{body}\n"),
        );
    }
    drop(state);
    app.close();
}

/// m6-conflicts/29 (review-18 Minor 1): `Esc` from the landing, on a real
/// two-file merge, shows the banner's keys and the plural count — the
/// asserts `capture_a_merge_that_stops` carried before the landing moved.
#[test]
fn esc_from_the_landing_shows_the_banner_row_on_a_real_merge() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = settled(
        &app,
        stopped_on_the_conflict(&app, state),
        "the banner after Esc",
    );
    for (columns, rows) in [(80u16, 24u16), (120, 40), (187, 50)] {
        let frame = drawn(&app, &state, columns, rows);
        let body = frame.join("\n");
        let row = footer(&frame, "\u{23ce} resolve");
        assert!(
            row.contains("\u{23ce} resolve  c continue  a abort"),
            "{columns}x{rows}: {body}"
        );
        assert!(!body.contains("esc dismiss"), "{columns}x{rows}: {body}");
        if columns >= 120 {
            assert!(
                row.contains("merge stopped \u{b7} 2 files conflicted \u{b7}"),
                "{columns}x{rows}: {row}"
            );
        }
    }
    drop(state);
    app.close();
}

#[test]
fn capture_after_esc_on_the_banner() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = stopped_on_the_conflict(&app, state);
    let state = app.key(&state, KeyCode::Escape);
    assert!(!state.banner_focus);
    assert!(state.stopped.is_some(), "Esc leaves the stop standing");

    for (columns, rows) in [(120u16, 40u16), (187, 50)] {
        let frame = drawn(&app, &state, columns, rows);
        let row = footer(&frame, "banner");
        assert!(row.trim_start().starts_with("\u{21e5} banner"), "{row}");
        check_shot(
            &format!("m6/fix-landing-after-esc-{columns}x{rows}.txt"),
            &format!("{}\n", frame.join("\n")),
        );
    }
    drop(state);
    app.close();
}

#[test]
fn capture_a_launch_into_a_shell_merge() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::StoppedMerge);
    let (app, state) = App::open(scratch.path());
    let state = settled(&app, landed(&app, state), "the landing's pane");
    assert!(state.banner_focus, "the launch lands on the banner");

    for (columns, rows) in [(120u16, 40u16), (187, 50)] {
        let frame = drawn(&app, &state, columns, rows);
        let row = footer(&frame, "a abort");
        // With the pane open the footer is the graph column's width, too
        // narrow for the whole wording: `begun outside Graft` is dropped
        // whole, and the count is drawn whole — at 187 it once read `1…`
        // for 13 (m6-conflicts/24, review m6-amend-11-12 M1).
        assert!(
            row.trim_start()
                .starts_with("merge in progress \u{b7} 13 files conflicted \u{b7}"),
            "{row}"
        );
        assert!(
            frame.iter().any(|line| line.contains("CONFLICTS")),
            "the pane is on CONFLICTS:\n{}",
            frame.join("\n")
        );
        check_shot(
            &format!("m6/fix-landing-b-launch-{columns}x{rows}.txt"),
            &format!("{}\n", frame.join("\n")),
        );
    }
    drop(state);
    app.close();
}

#[test]
fn capture_after_continue_on_a_rebase() {
    let scratch = killed_mid_rebase();
    let repo = scratch.path();
    git(repo, &["add", "-A"]);
    // The replayed commits are committed by whoever runs the test unless
    // the repository says otherwise: the fixture's own name keeps a real
    // person's out of a capture that may be published.
    git(repo, &["config", "user.name", "Graft Fixture"]);
    git(repo, &["config", "user.email", "fixture@example.invalid"]);
    let (app, state) = App::open(repo);
    let state = landed(&app, state);
    assert!(state.banner_focus, "{:?}", state.status);

    let state = app.key(&state, KeyCode::Char('c'));
    let state = app.pump(state, "the continue and the landing it owes", |state| {
        state.operating.is_none()
            && state.in_progress.is_none()
            && !state.land_on_head
            && matches!(state.status, Status::Done { .. })
            && state
                .working_tree
                .as_ref()
                .is_some_and(|tree| tree.entries.is_empty())
    });
    let state = settled(&app, state, "the finished rebase's frame");
    assert!(state.stopped.is_none(), "the operation ended");
    assert!(
        state.graph_has_focus(),
        "the graph has the keyboard after a finish"
    );
    let head = git(repo, &["rev-parse", "HEAD"]);
    let selected = state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
        .map(|row| row.commit.id.to_string());
    assert_eq!(
        selected.as_deref(),
        Some(head.trim()),
        "the cursor is on HEAD"
    );

    // The replayed commits are made now, so their ids are new every run:
    // each one the frame shows is zeroed, width kept.
    let replayed = git(repo, &["rev-list", "main..HEAD"]);
    for (columns, rows) in [(120u16, 40u16), (187, 50)] {
        let frame = drawn(&app, &state, columns, rows);
        let row = footer(&frame, "\u{2713}");
        assert!(row.trim_end().ends_with('\u{2713}'), "{row}");
        let mut body = format!("{}\n", frame.join("\n"));
        for id in replayed.lines() {
            if body.contains(&id[..7]) {
                body = without_invented_id(&body, id);
            }
        }
        check_shot(
            &format!("m6/fix-landing-after-continue-rebase-{columns}x{rows}.txt"),
            &body,
        );
    }
    drop(state);
    app.close();
}
