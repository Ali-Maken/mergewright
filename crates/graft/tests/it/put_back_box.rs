//! The put-back box over J (m7-power-tools/24-a2, J1): what it says, which
//! keys it holds, where `y` and a refusal leave him, and how it is drawn —
//! over the real job pool and real `git`.
//!
//! The words themselves are `update::recover`'s unit tests; these check the
//! box the running app builds from the real `recover_for` and the real plan.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::update::AppState;
use graft::update::detail::Loading;
use graft_core::event::KeyCode;

use crate::app_harness::{App, git};

/// Every date this file's git writes carry, so the lost commit's sha (and
/// the capture) is the same every run.
const WHEN: i64 = 1_578_000_000;

/// `git` with a fixed identity and a fixed date, reflog records included.
fn git_at(repo: &Path, at: i64, args: &[&str]) {
    let date = format!("@{at} +0000");
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Graft Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Graft Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .output()
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn main_tip(repo: &Path) -> String {
    git(repo, &["rev-parse", "refs/heads/main"])
        .trim()
        .to_owned()
}

/// J1's story: `Add rate limiter` committed, then the shell's
/// `reset --hard HEAD~1`. Returns the lost commit and where `main` was left.
fn lost_by_the_shell(repo: &Path) -> (String, String) {
    std::fs::write(repo.join("limits.txt"), "rate: 10\n").expect("the scratch repo is writable");
    git(repo, &["add", "limits.txt"]);
    git_at(repo, WHEN, &["commit", "-q", "-m", "Add rate limiter"]);
    let lost = main_tip(repo);
    git_at(repo, WHEN + 60, &["reset", "-q", "--hard", "HEAD~1"]);
    let left = main_tip(repo);
    assert_ne!(lost, left);
    (lost, left)
}

/// `J`, every stage answered, `⏎` on the preselected lost row, and the
/// put-back's plan landed: the box is up over J.
fn the_box_up(repo: &Path) -> (App, AppState) {
    let (app, state, _) = the_box_up_from(repo);
    (app, state)
}

/// J as drawn at 187x50 with the fixed clock, box or not.
fn drawn(state: &AppState) -> Vec<String> {
    crate::captures::frame_at(
        state,
        187,
        50,
        Some(graft::session::Clock::Fixed(WHEN + 120)),
    )
}

/// [`the_box_up`], and J's frame just before the `⏎` that opened it.
fn the_box_up_from(repo: &Path) -> (App, AppState, Vec<String>) {
    let (app, state) = App::open(repo);
    let state = app.pump(state, "the launch read", |state| {
        state.journal.reading.is_none()
    });
    let state = app.key(&state, KeyCode::Char('J'));
    let state = app.pump(state, "stages 2 and 3", |state| {
        !state.journal.finding_lost()
    });
    let before = drawn(&state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the put-back's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| matches!(preview.plan, Loading::Ready(_)))
            || state.status != graft::update::Status::default()
    });
    assert!(
        state.preview.is_some(),
        "the box opened: {:?}",
        state.status
    );
    assert!(state.journal.shown, "J is under the box");
    (app, state, before)
}

/// J1's footer and buttons: `y put it back`, no `y confirm` beside it, and
/// `Cancel` / `Put back` with the cursor on `Cancel` (ADR-0038 §1).
#[test]
fn the_box_footer_is_y_put_it_back() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    lost_by_the_shell(repo);
    let (app, state) = the_box_up(repo);

    let view = graft::update::preview::preview_box(&state).expect("the box");
    assert_eq!(
        view.again
            .as_ref()
            .map(|(key, words)| (key.as_str(), words.as_str())),
        Some(("y", "put it back"))
    );
    assert!(!view.confirm_key, "no second `y confirm`");
    assert!(!view.keep, "never `R keep` on a put-back");
    let choices: Vec<&str> = view
        .choices()
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(choices, ["Cancel", "Put back"]);
    assert_eq!(view.selected(), 0, "the cursor starts on Cancel");
    app.close();
}

/// The box is modal over J: `j` moves neither J's cursor nor anything else,
/// and `y` is the box's — it puts `main` back.
#[test]
fn keys_go_to_the_put_back_box_over_j() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    let (lost, left) = lost_by_the_shell(repo);
    let (app, state) = the_box_up(repo);

    let before = drawn(&state);
    let state = app.key(&state, KeyCode::Char('j'));
    assert_eq!(
        drawn(&state),
        before,
        "j went to the box: neither J nor the box moved"
    );
    assert!(state.preview.is_some(), "and the box is still up");
    assert_eq!(main_tip(repo), left, "nothing ran");

    let state = app.key(&state, KeyCode::Char('y'));
    assert!(state.preview.is_none(), "y answered the box");
    let state = app.settle_write(state, "the put-back");
    drop(state);
    app.close();
    assert_eq!(main_tip(repo), lost, "y put main back");
}

/// `y` closes the box **and** J: the landing is on the graph.
#[test]
fn y_on_the_put_back_box_closes_j() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    lost_by_the_shell(repo);
    let (app, state) = the_box_up(repo);

    let state = app.key(&state, KeyCode::Char('y'));
    assert!(state.preview.is_none());
    assert!(!state.journal.shown, "J closed with the box");
    let state = app.settle_write(state, "the put-back");
    assert!(!state.journal.shown, "and stays closed once it lands");
    app.close();
}

/// A plan refused (here: `main` moved on since the list read it) closes
/// the box and leaves J up, on the same row, with the refusal on the
/// status J's status row draws.
///
/// The refusal is handed to `preview_produced` as the plan job would hand
/// it: making the real plan refuse needs `main` to move between the
/// `recover_for` answer and the plan job, which the pool gives no seam for.
#[test]
fn a_refused_plan_closes_the_box_onto_js_status_row() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    let (_lost, left) = lost_by_the_shell(repo);
    let (app, mut state, j_before) = the_box_up_from(repo);

    let job = state.preview.as_ref().expect("the box").job;
    let now = graft_core::oid::Oid::from_hex(&left).expect("a sha");
    graft::update::preview::preview_produced(
        &mut state,
        job,
        Err(graft::update::preview::PreviewFailure::Refused(
            graft_ops::operation::Refusal::MovedOnSince {
                name: "main".to_owned(),
                now,
            },
        )),
    );
    assert!(state.preview.is_none(), "the box closed");
    assert!(state.journal.shown, "J is still up");
    // J as it was, on the same row — but for its status row, which now
    // carries the refusal (24-b: J draws its own status row above the
    // legend, where he pressed the key).
    let after = drawn(&state);
    let body = after.join("\n");
    let refusal = row_of(&after, "main moved on since")
        .unwrap_or_else(|| panic!("the refusal on J's status row:\n{body}"));
    let legend =
        row_of(&after, "\u{25cf} done in Graft").unwrap_or_else(|| panic!("J's legend:\n{body}"));
    assert_eq!(
        refusal + 1,
        legend,
        "the status row sits above the legend:\n{body}"
    );
    assert_eq!(after.len(), j_before.len());
    for (row, (now, was)) in after.iter().zip(&j_before).enumerate() {
        if row != refusal {
            assert_eq!(now, was, "row {row} is J as it was:\n{body}");
        }
    }
    assert!(
        matches!(&state.status, graft::update::Status::Sticky { .. }),
        "the refusal is on the status J shows: {:?}",
        state.status
    );
    app.close();
}

/// Where each part of a frame is: the row index of the first line holding
/// `needle`.
fn row_of(frame: &[String], needle: &str) -> Option<usize> {
    frame.iter().position(|row| row.contains(needle))
}

/// The box docked over J, as the stash box docks over S: J's title row on
/// top, J's rows between, the box at the bottom. Also
/// `docs/shots/m7/put-back-preview-187x50.txt`.
#[test]
fn the_put_back_box_draws_over_j() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    let (lost, _left) = lost_by_the_shell(repo);
    let (app, state) = the_box_up(repo);
    let frame = drawn(&state);
    let body = format!("{}\n", frame.join("\n"));

    let title = format!(
        "Preview: put main back to {} \"Add rate limiter\"",
        &lost[..7]
    );
    let journal = row_of(&frame, "JOURNAL").expect("J's title row");
    let box_title = row_of(&frame, &title).unwrap_or_else(|| panic!("the box's title:\n{body}"));
    assert!(journal < box_title, "J above the box:\n{body}");
    assert!(
        box_title > journal + 1,
        "J's rows between its title and the box:\n{body}"
    );
    for line in [
        "now       main \u{2192}",
        "after y   main \u{2192}",
        "u undoes it",
    ] {
        assert!(
            row_of(&frame, line).is_some_and(|row| row > box_title),
            "{line:?} in the box:\n{body}"
        );
    }
    assert!(
        frame.iter().any(|row| row.contains("your files:")),
        "the files line:\n{body}"
    );
    assert!(
        frame
            .iter()
            .any(|row| row.trim_start().starts_with("y put it back")
                || row.contains(" y put it back")),
        "the footer:\n{body}"
    );
    assert!(
        !body.contains("J/esc close"),
        "J's bar is not drawn under a box that holds its keys:\n{body}"
    );
    crate::captures::check_shot("m7/put-back-preview-187x50.txt", &body);
    app.close();
}
