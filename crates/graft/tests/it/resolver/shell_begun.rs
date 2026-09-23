//! A hand edit lost to a whole-file take in a merge **begun in the shell**,
//! and the way the user gets it back (m6 g04/g08 review, M1).

use super::*;

const EDIT: &str = "my hand edit, not in any commit\n";

/// The user hand-edits `theirs-deleted.txt` (`UD`), then `t` takes theirs:
/// `git rm`, the file is gone. The take is its own journal entry with a
/// backup, so the edit is kept — but the app cannot reach it while the
/// merge is open: `u` is refused until the merge ends, and `a` on the
/// banner refuses a merge Graft did not start. The walk the user has today:
/// `git merge --abort` in the shell, back in the app (`Ctrl+R`), `u`,
/// answer the drift box with `Undo`. The edit's bytes are back on disk.
#[test]
fn a_hand_edit_taken_by_t_comes_back_after_the_merge_ends() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();

    let state = hand_edit_then_t(&app, state, &repo);
    let state = unreachable_while_the_merge_is_open(&app, &state, &repo);

    // The user ends it in the shell and comes back. The banner is modal
    // while focused, so `Esc` gives the panes the keyboard for `Ctrl+R`.
    crate::app_harness::git(&repo, &["merge", "--abort"]);
    let state = app.key(&state, KeyCode::Escape);
    assert!(!state.banner_focus, "the panes have the keyboard");
    let state = app.press(&state, KeyPress::control(KeyCode::Char('r')));
    let state = app.pump(state, "the re-read after the shell abort", |state| {
        state.in_progress.is_none()
    });

    let state = undo_through_the_drift_box(&app, &state);
    assert_eq!(
        on_disk(&repo, "theirs-deleted.txt"),
        EDIT,
        "the hand edit is back, byte for byte"
    );
    // What the undo restores is the index the take found: the file's
    // conflict stages, now with no merge around them (ticket 08, review
    // fixes, follow-up). Pinned as observed, so a change is seen.
    assert!(
        still_unmerged(&repo)
            .lines()
            .any(|line| line == "theirs-deleted.txt"),
        "{:?}",
        still_unmerged(&repo)
    );
    assert!(!repo.join(".git/MERGE_HEAD").exists());

    drop(state);
    app.close();
    drop(scratch);
}

/// Writes [`EDIT`] over `theirs-deleted.txt`, then `t` on its card: the
/// file, edit and all, is gone, and the take is its own entry with a backup.
fn hand_edit_then_t(app: &App, state: AppState, repo: &Path) -> AppState {
    let before = journal_entries(repo).len();
    std::fs::write(repo.join("theirs-deleted.txt"), EDIT).expect("writable scratch");
    let state = resolver_on_whole_file(app, state, "theirs-deleted.txt");
    let state = app.key(&state, KeyCode::Char('t'));
    let state = app.pump(state, "the whole-file take", |state| {
        state.operating.is_none()
    });
    assert!(
        !repo.join("theirs-deleted.txt").exists(),
        "`t` removed the file: {:?}",
        state.status
    );
    let entries = journal_entries(repo);
    assert_eq!(entries.len(), before + 1, "the take is its own entry");
    assert!(
        entries[before].backup_ref.is_some(),
        "with a backup of what it dropped: {:?}",
        entries[before]
    );
    state
}

/// Out of the resolver, `u` is refused while the merge is open, and the
/// banner's `a` will not end a merge Graft did not start. Leaves the banner
/// focused.
fn unreachable_while_the_merge_is_open(app: &App, state: &AppState, repo: &Path) -> AppState {
    let state = app.key(state, KeyCode::Escape);
    assert!(state.resolver.is_none(), "back to the pane");
    let state = app.key(&state, KeyCode::Char('u'));
    assert!(state.operating.is_none(), "`u` launched nothing");
    assert!(
        matches!(
            &state.status,
            Status::Sticky { reason, .. } if reason == "finish or abort the merge first"
        ),
        "{:?}",
        state.status
    );

    let state = on_the_banner(app, state);
    let state = app.key(&state, KeyCode::Char('a'));
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the abort's answer", |state| {
        state.operating.is_none()
    });
    assert!(
        matches!(
            &state.status,
            Status::Sticky { reason, .. } if reason.contains("begun in the shell is finished there")
        ),
        "{:?}",
        state.status
    );
    assert!(
        repo.join(".git/MERGE_HEAD").exists(),
        "the merge is still open"
    );
    state
}

/// `u` on the take: the tree drifted since (the abort), so it asks first;
/// `→` onto `Undo`, `⏎`.
fn undo_through_the_drift_box(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('u'));
    let state = app.pump(state, "the undo's drift box", |state| {
        state.confirm.is_some() || state.operating.is_none()
    });
    let confirm = state.confirm.as_ref().expect("the drift box");
    assert_eq!(
        confirm.view.title.as_str(),
        "Undo \u{201c}resolved theirs-deleted.txt (theirs)\u{201d}?"
    );
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the undo", |state| state.operating.is_none());
    assert!(
        matches!(&state.status, Status::Done(said) if said.starts_with("undo:")),
        "{:?}",
        state.status
    );
    state
}
