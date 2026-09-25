//! m6-conflicts/68 on a real repository: an abort reads `aborted: … ✓`, and
//! what `u` says right after it. Real `git`, real job pool, real keys; the
//! repository is read back rather than trusted from the line.
//!
//! Colour is not in a character grid: **unverified** (CLAUDE.md rule 5).

use graft::session::Clock;
use graft::update::{AppState, Status};
use graft_core::event::KeyCode;

use super::{CONFLICT_NOW, git, killed_mid_rebase, landed};
use crate::app_harness::App;
use crate::captures::{check_shot, frame_at};

const ABORTED: &str = "aborted: rebase feature onto main";

/// Relaunch into `killed_mid_rebase`'s stop, `a y`, and wait for the abort
/// and the landing it owes.
fn aborted_after_relaunch(app: &App, state: AppState) -> AppState {
    let state = landed(app, state);
    assert!(state.banner_focus, "{:?}", state.status);
    let state = app.key(&state, KeyCode::Char('a'));
    assert!(state.confirm.is_some(), "`a` asks first");
    let state = app.key(&state, KeyCode::Char('y'));
    assert!(state.operating.is_some(), "`y` sent the abort");
    let state = app.pump(state, "the abort and its landing", |state| {
        state.operating.is_none()
            && state.in_progress.is_none()
            && !state.land_on_head
            && matches!(state.status, Status::Done { .. })
    });
    app.pump(state, "a settled frame", |state| {
        crate::captures::frame(state, 120, 40)
            .iter()
            .all(|row| !row.contains("reading\u{2026}") && !row.contains("sorting\u{2026}"))
    })
}

#[test]
fn a_relaunched_rebase_abort_reads_aborted() {
    let scratch = killed_mid_rebase();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = aborted_after_relaunch(&app, state);

    assert!(
        !repo.join(".git/rebase-merge").exists() && !repo.join(".git/rebase-apply").exists(),
        "git has no rebase left"
    );
    let status = git(repo, &["status"]);
    assert!(!status.contains("rebase"), "{status}");
    assert_eq!(state.status, Status::done(ABORTED));

    let sized = app.resize(&state, 120, 40);
    let frame = frame_at(&sized, 120, 40, Some(Clock::Fixed(CONFLICT_NOW)));
    let body = frame.join("\n");
    assert!(!frame[0].contains("REBASING"), "{body}");
    // The graph's footer: with the changes pane open below the graph it is
    // not the frame's last row.
    assert!(
        frame
            .iter()
            .any(|row| row.contains(&format!("{ABORTED} \u{2713}"))),
        "{body}"
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
    check_shot(
        "m6/fix-abort-reads-aborted-120x40.txt",
        &format!("{body}\n"),
    );
    drop(state);
    app.close();
}

/// **Pin, not a fix** (the scout did not read `undo::land_after_undo`):
/// `u` right after the abort. What it reads is recorded in the ticket's
/// NOTES.
#[test]
fn abort_then_u_words() {
    let scratch = killed_mid_rebase();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = aborted_after_relaunch(&app, state);
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.pump(state, "the undo's answer", |state| {
        state.operating.is_none() && !matches!(state.status, Status::Busy(_))
    });
    // Pinned as observed: the relaunched entry has no backup, so there is
    // nothing for `u` to take back, and it does not word the abort as done.
    assert_eq!(state.status, Status::done("nothing to undo"));
    assert!(state.in_progress.is_none(), "the rebase stays aborted");
    assert!(
        !repo.join(".git/rebase-merge").exists(),
        "git has no rebase back"
    );
    drop(state);
    app.close();
}
