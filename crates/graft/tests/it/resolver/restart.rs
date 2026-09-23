//! `d`: starting a file over, and how the resolver's writes sit in the journal (m6-conflicts/05 scope 3; m6-05 review, Major 5).

use super::*;

// ---------------------------------------------------------------- d: restart

/// `d` raises a confirm box; answering it runs `checkout --merge` and the
/// file comes back with all its markers (m6-conflicts/05 scope 3).
#[test]
#[allow(
    non_snake_case,
    reason = "the key is `d`, and the test is named for it"
)]
#[allow(
    clippy::too_many_lines,
    reason = "one acceptance box walked end to end, in its own order"
)]
fn d_starts_over_behind_a_box() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on(&app, state, "both.txt");
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(&app, state);
    assert_eq!(
        open(&state).file.as_ref().expect("a document").open(),
        0,
        "both.txt's one block is taken"
    );

    let state = app.key(&state, KeyCode::Char('d'));
    let confirm = state.confirm.as_ref().expect("`d` raised a box");
    assert_eq!(confirm.view.title.as_str(), "Start both.txt over?");
    assert!(
        confirm
            .view
            .message
            .iter()
            .any(|line| line.as_str().contains("comes back with all its markers")),
        "{:?}",
        confirm.view.message
    );
    let message: Vec<&str> = confirm
        .view
        .message
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(
        message,
        vec![
            "the conflict comes back with all its markers",
            "takes and edits in this file are lost · not undoable while the merge is open",
        ]
    );
    let choices: Vec<&str> = confirm
        .view
        .choices
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(choices, vec!["Cancel", "Start over"]);
    assert_eq!(
        confirm.view.command.as_str(),
        "git checkout --merge -- both.txt"
    );

    // `→` off `Cancel` onto `Start over`, then `Enter` runs it.
    let state = app.key(&state, KeyCode::Right);
    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Enter));
    let [Command::Operate(_, operation)] = commands.as_slice() else {
        panic!("`Start over` queued {commands:?}")
    };
    assert!(
        matches!(
            operation.as_ref(),
            Operation::RestartConflict(graft_ops::operation::RestartParams { path, .. })
                if path == Path::new("both.txt")
        ),
        "{operation:?}"
    );
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the restart", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.job.is_none())
    });

    assert!(state.confirm.is_none(), "the box is gone");
    assert_eq!(open(&state).path, Path::new("both.txt"), "the same file");
    assert_eq!(
        open(&state).file.as_ref().expect("a document").open(),
        1,
        "the take is gone: the block is open again"
    );
    assert!(
        on_disk(&repo, "both.txt").contains("<<<<<<<"),
        "the markers are back on disk"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// `Esc` (or `←` onto `Cancel`, then `Enter`) answers the box for free: no
/// write, the take stands.
#[test]
#[allow(
    non_snake_case,
    reason = "the key is `d`, and the test is named for it"
)]
fn d_cancel_is_free() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on(&app, state, "both.txt");
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(&app, state);

    let state = app.key(&state, KeyCode::Char('d'));
    assert!(state.confirm.is_some());
    let state = app.key(&state, KeyCode::Escape);

    assert!(state.confirm.is_none(), "`Esc` answers the box");
    assert!(state.operating.is_none(), "no write was launched");
    assert_eq!(
        open(&state).file.as_ref().expect("a document").open(),
        0,
        "the take still stands"
    );
    assert_eq!(
        on_disk(&repo, "both.txt"),
        "same on both sides\nthe line, as we changed it\nand a tail\n",
        "nothing on disk moved"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// In a merge **begun in the shell** a whole-file `o` and a `d` restart add
/// no journal entry: they are steps of a live operation
/// (`apply::resume::inside_live_operation`), and with no entry of Graft's
/// to chain onto there is nothing to fold into — so, too, **no backup is
/// taken** of what either step drops (the ops trade-off m6 g03 review M2
/// raised). The fold itself, into an entry Graft did write, is
/// [`whole_file_and_restart_fold_into_live_entry`] (m6-05 review, Major 5).
#[test]
fn no_entry_added_in_a_shell_begun_merge() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    // The fixture built the merge with raw git, never through the app, so
    // there is no journal entry for the merge itself here — only whether a
    // whole-file resolve or a restart adds one of its own is this test's,
    // and that is `before` either way.
    let before = journal_entries(&repo).len();

    // A whole-file `o`.
    let state = resolver_on_whole_file(&app, state, "theirs-deleted.txt");
    let state = app.key(&state, KeyCode::Char('o'));
    let state = app.pump(state, "the whole-file resolve", |state| {
        state.operating.is_none()
    });
    assert_eq!(
        journal_entries(&repo).len(),
        before,
        "no new entry for the whole-file resolve"
    );

    // The resolve advanced to whatever conflicted file is next in the
    // list, keeping the resolver's modal keyboard — `Esc` gives the pane
    // back, already open on the files list (so `cursor_to` alone finds
    // `both.txt`; `on_the_row`/`changes_pane` assume a fresh graph-top
    // start, which this is not).
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none(), "closed, back to the pane");
    let state = app.pump(state, "the files list after the resolve", |state| {
        matches!(state.detail.files, graft::update::detail::Loading::Ready(_))
    });

    // A `d` restart on a block take.
    let state = cursor_to(&app, state, "both.txt");
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the conflict file", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.file.is_some())
    });
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(&app, state);
    let state = app.key(&state, KeyCode::Char('d'));
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the restart", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.job.is_none())
    });
    assert_eq!(
        journal_entries(&repo).len(),
        before,
        "no new entry for the restart either"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// A merge begun **through the app** that stopped on `shared.rs` (both
/// modified) and `gone.txt` (deleted by them): `feature` rewrote the one and
/// deleted the other, `main` rewrote both.
fn app_merge_with_a_whole_file_conflict() -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let write = |name: &str, body: &str| {
        std::fs::write(repo.join(name), body).expect("writable scratch");
    };
    write("shared.rs", "fn main() {}\n");
    write("gone.txt", "the plan\n");
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-m", "the shared files"]);
    git(repo, &["switch", "--quiet", "-c", "feature"]);
    write("shared.rs", "fn main() { theirs() }\n");
    git(repo, &["rm", "--quiet", "gone.txt"]);
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-m", "the feature's take"]);
    git(repo, &["switch", "--quiet", "main"]);
    write("shared.rs", "fn main() { ours() }\n");
    write("gone.txt", "our plan\n");
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-m", "main's take"]);
    scratch
}

/// The live entry's backup ref, and where it points now.
fn live_backup(repo: &Path) -> (String, String) {
    let entries = journal_entries(repo);
    let live = entries.last().expect("the merge wrote an entry");
    assert_eq!(
        live.status,
        graft_core::journal::EntryStatus::Pending,
        "the merge's entry is still live"
    );
    let name = live
        .backup_ref
        .clone()
        .expect("a live entry has a backup ref");
    let tip = crate::app_harness::git(repo, &["rev-parse", &name])
        .trim()
        .to_owned();
    (name, tip)
}

/// A whole-file `o` and a `d` restart run *inside* the merge's own live
/// operation when Graft began it: neither adds a journal entry, the merge's
/// one `Pending` entry is still what `u` undoes, and each step **chains**
/// that entry's backup — the ref moves to a commit whose first parent is
/// where it was, so what the step dropped is kept (m6-conflicts/05 scope 1,
/// 3; m6-05 review, Major 5).
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "two folded steps walked end to end, each checked against the ref it moved"
)]
fn whole_file_and_restart_fold_into_live_entry() {
    let scratch = app_merge_with_a_whole_file_conflict();
    let repo = scratch.path().to_owned();
    let (app, state) = App::open(&repo);
    let state = crate::conflict_interim::stopped_on_the_conflict(&app, state);
    let entries = journal_entries(&repo).len();
    let (backup, merged) = live_backup(&repo);

    // `⏎` on the banner opens git's first conflicted path: the card.
    assert!(state.banner_focus, "the stop has the keyboard");
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the whole-file card", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.whole_file.is_some())
    });
    assert_eq!(open(&state).path, Path::new("gone.txt"));
    let state = app.key(&state, KeyCode::Char('o'));
    assert!(
        state.operating.is_some(),
        "`o` launched: {:?}",
        state.status
    );
    // The resolve advances to the next conflicted file, `shared.rs`.
    let state = app.pump(state, "the resolve, and the next file", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.file.is_some())
    });
    assert_eq!(open(&state).path, Path::new("shared.rs"));
    assert_eq!(journal_entries(&repo).len(), entries, "no entry for `o`");
    let (same, resolved) = live_backup(&repo);
    assert_eq!(same, backup, "still the merge's own entry");
    assert_ne!(resolved, merged, "the resolve chained the backup");
    assert_eq!(
        crate::app_harness::git(&repo, &["rev-parse", &format!("{resolved}^1")]).trim(),
        merged,
        "onto where it was"
    );

    // A take, then `d`: the restart.
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(&app, state);
    let state = app.key(&state, KeyCode::Char('d'));
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    assert!(
        state.operating.is_some(),
        "`d` launched: {:?}",
        state.status
    );
    let state = app.pump(state, "the restart, and the fresh read", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.job.is_none() && open.file.is_some())
    });
    assert_eq!(
        open(&state).file.as_ref().map(|file| file.open()),
        Some(1),
        "the markers are back"
    );
    assert_eq!(journal_entries(&repo).len(), entries, "no entry for `d`");
    let (same, restarted) = live_backup(&repo);
    assert_eq!(same, backup, "still the merge's own entry");
    assert_eq!(
        crate::app_harness::git(&repo, &["rev-parse", &format!("{restarted}^1")]).trim(),
        resolved,
        "the restart chained onto the resolve's backup"
    );

    drop(state);
    app.close();
    drop(scratch);
}
