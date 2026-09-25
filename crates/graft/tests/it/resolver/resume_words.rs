//! m6-conflicts/68 on a real repository: a skip that finishes the rebase
//! says it skipped. Graft begins the rebase, so the skip closes Graft's own
//! entry; git's log and the journal are read back rather than trusted from
//! the line.

use graft::update::Status;
use graft_core::event::KeyCode;
use graft_core::journal::{EntryStatus, OperationKind};

use super::file_list::{at_step_two, graft_rebase_at_step_one};
use crate::app_harness::git;

#[test]
fn a_final_skip_says_it_skipped() {
    let (scratch, app, state) = graft_rebase_at_step_one();
    let repo = scratch.path();
    let state = at_step_two(&app, state);
    // The commit step 2 is replaying: the one the skip leaves out.
    let skipped = git(repo, &["log", "-1", "--format=%s", "REBASE_HEAD"]);
    let skipped = skipped.trim().to_owned();
    assert!(!skipped.is_empty());

    let mut state = app.key(&state, KeyCode::Escape);
    if !state.banner_focus {
        state = app.key(&state, KeyCode::Tab);
    }
    assert!(state.banner_focus, "{:?}", state.status);
    let state = app.key(&state, KeyCode::Char('s'));
    assert!(state.confirm.is_some(), "`s` asks first");
    let state = app.key(&state, KeyCode::Char('y'));
    assert!(state.operating.is_some(), "`y` sent the skip");
    let state = app.pump(state, "the skip that finishes the rebase", |state| {
        state.operating.is_none()
            && state.in_progress.is_none()
            && matches!(state.status, Status::Done { .. })
    });

    let entries = graft_ops::journal::Journal::open(&repo.join(".git/graft/undo-journal.jsonl"))
        .expect("the journal the rebase wrote is readable")
        .entries()
        .to_vec();
    let rebase = entries
        .iter()
        .rev()
        .find(|entry| entry.kind == OperationKind::Rebase)
        .expect("the rebase is journaled");
    assert_eq!(
        rebase.status,
        EntryStatus::Committed,
        "the skip finished it"
    );
    assert_eq!(
        state.status,
        Status::done(format!("{}, last commit skipped", rebase.description))
    );
    let body = crate::captures::frame(&state, 120, 40).join("\n");
    assert!(
        body.contains(&format!(
            "{}, last commit skipped \u{2713}",
            rebase.description
        )),
        "{body}"
    );
    let log = git(repo, &["log", "--format=%s", "HEAD"]);
    assert!(
        !log.lines().any(|subject| subject == skipped),
        "{skipped:?} was skipped: {log}"
    );
    assert!(
        !repo.join(".git/rebase-merge").exists(),
        "the rebase is over"
    );
    drop(state);
    app.close();
    drop(scratch);
}
