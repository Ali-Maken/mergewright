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

// ------------------------------- finishing it in the app (m6-conflicts/20 B)

/// `git merge feature` run **in the shell** on `main`, stopped on
/// `shared.rs` and `notes.md`, both then resolved in the shell (written and
/// `git add`ed): a merge Graft did not start, nothing left unmerged. The
/// commits are `about_to_conflict`'s, stamped at fixed instants, so the
/// graph draws the same every run; the merge commit the app makes is the
/// fixture's name, never whoever runs the test.
pub(super) fn resolved_shell_merge() -> graft_fixtures::Scratch {
    use crate::app_harness::git;
    let scratch = crate::conflict_interim::about_to_conflict();
    let repo = scratch.path();
    git(repo, &["config", "user.name", "Graft Fixture"]);
    git(repo, &["config", "user.email", "fixture@example.invalid"]);
    git_stops(repo, &["merge", "feature"]);
    std::fs::write(repo.join("shared.rs"), "fn main() { both() }\n").expect("writable");
    std::fs::write(repo.join("notes.md"), "the agreed plan\n").expect("writable");
    git(repo, &["add", "shared.rs", "notes.md"]);
    assert!(still_unmerged(repo).is_empty(), "resolved in the shell");
    assert!(repo.join(".git/MERGE_HEAD").exists(), "and still merging");
    scratch
}

/// The app launched into [`resolved_shell_merge`]: the working tree, the
/// journal, the refs and the first rows read — J3's landing.
pub(super) fn launched_into(repo: &Path) -> (App, AppState) {
    let (app, state) = App::open(repo);
    let state = crate::conflict_interim::launched(&app, state);
    let state = app.pump(state, "the refs and the first rows", |state| {
        state.refs.is_some()
            && state.in_progress.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    });
    assert!(
        state.live_entries.is_empty(),
        "no entry of Graft's is live: begun in the shell"
    );
    (app, state)
}

/// `c` on the resolved banner, and the summary's read landed.
fn c_on_the_banner(app: &App, state: AppState) -> AppState {
    let state = on_the_banner(app, state);
    let state = app.key(&state, KeyCode::Char('c'));
    assert!(
        state.finish.is_some(),
        "`c` opens the summary: {:?}",
        state.status
    );
    app.pump(state, "the finish summary", |state| {
        state.finish.as_ref().is_some_and(|open| open.job.is_none())
    })
}

/// `⏎` in the summary, the finish it sends, and the landing it owes.
fn enter_to_the_landing(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Enter);
    assert!(
        state.operating.is_some(),
        "`⏎` commits: {:?}",
        state.finish.as_ref().map(|open| &open.notice)
    );
    let state = app.pump(state, "the finish and its landing", |state| {
        state.operating.is_none() && state.in_progress.is_none() && !state.land_on_head
    });
    settled_frame(app, state)
}

/// Pumps until nothing on the frame is still loading, so a capture never
/// catches a `reading…` or `sorting…` that a moment later is gone (as
/// `conflict_interim::landing_shots::settled`).
fn settled_frame(app: &App, state: AppState) -> AppState {
    app.pump(state, "a frame with nothing loading", |state| {
        crate::captures::frame(state, 120, 40)
            .iter()
            .all(|row| !row.contains("reading\u{2026}") && !row.contains("sorting\u{2026}"))
    })
}

/// The commit under the graph's cursor.
fn cursor_commit(state: &AppState) -> Option<String> {
    state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
        .map(|row| row.commit.id.to_string())
}

/// The frame the user judges, at 120x40 against the fixture's fixed clock.
/// The merge commit is made now, so its id and its date are new every run:
/// each is zeroed where it shows, width kept.
fn shot(app: &App, state: &AppState, repo: &Path, merge: &str) -> (Vec<String>, String) {
    let sized = app.resize(state, 120, 40);
    let frame = crate::captures::frame_at(
        &sized,
        120,
        40,
        Some(graft::session::Clock::Fixed(
            crate::conflict_interim::CONFLICT_NOW,
        )),
    );
    let mut body = format!("{}\n", frame.join("\n"));
    if body.contains(&merge[..7]) {
        body = crate::captures::without_invented_id(&body, merge);
    }
    // The detail pane's `COMMIT` line draws the author date this way.
    let date = crate::app_harness::git(
        repo,
        &[
            "log",
            "-1",
            "--date=format:%Y-%m-%d %H:%M %z",
            "--format=%ad",
            merge,
        ],
    );
    let date = date.trim();
    body = body.replace(date, &date.replace(|c: char| c.is_ascii_digit(), "0"));
    (frame, body)
}

/// **J6b: `c ⏎`, two keys from the resolved banner** (the `b-finish` count
/// of 2). git: a merge commit whose second parent is `feature`, no
/// `MERGE_HEAD`, one `FinishMerge` entry. App: B3's first landing — the
/// graph focused, the cursor on the merge commit, `merged feature into
/// main ✓`, the summary closed.
#[test]
fn c_enter_finishes_a_shell_begun_merge() {
    let scratch = resolved_shell_merge();
    let repo = scratch.path().to_owned();
    let before = journal_entries(&repo).len();
    let (app, state) = launched_into(&repo);
    let state = on_the_banner(&app, state);

    let state = app.key(&state, KeyCode::Char('c'));
    let state = app.pump(state, "the finish summary", |state| {
        state.finish.as_ref().is_some_and(|open| open.job.is_none())
    });
    let state = enter_to_the_landing(&app, &state);

    let feature = crate::app_harness::git(&repo, &["rev-parse", "feature"]);
    let second = crate::app_harness::git(&repo, &["rev-parse", "HEAD^2"]);
    assert_eq!(second.trim(), feature.trim(), "HEAD^2 is feature's tip");
    assert!(!repo.join(".git/MERGE_HEAD").exists(), "git finished it");
    let entries = journal_entries(&repo);
    assert_eq!(entries.len(), before + 1, "one new entry");
    assert_eq!(
        entries[before].kind,
        graft_core::journal::OperationKind::FinishMerge
    );

    assert!(state.finish.is_none(), "the summary closed");
    assert!(state.graph_has_focus(), "the graph has the keyboard");
    let head = crate::app_harness::git(&repo, &["rev-parse", "HEAD"]);
    assert_eq!(cursor_commit(&state).as_deref(), Some(head.trim()));
    assert_eq!(
        state.status,
        Status::Done("merged feature into main".to_owned())
    );

    let (frame, body) = shot(&app, &state, &repo, head.trim());
    assert!(!frame[0].contains("MERGING"), "{body}");
    // The status is the graph's footer row: the detail pane the landing
    // opens sits below it, so it is not the frame's last row.
    assert!(
        frame
            .iter()
            .any(|row| row.trim_end() == " merged feature into main ✓"),
        "{body}"
    );
    crate::captures::check_shot("m6/fix-finish-shell-merge-120x40.txt", &body);
    drop(state);
    app.close();
}

/// **`u` after a shell-begun finish puts the merge back.** git: `MERGE_HEAD`
/// is back, naming `feature`, and `main` is on its pre-merge tip. App: B3's
/// `u` landing — `undo: finish merge of feature`, `in_progress` a merge
/// again after the refresh, the header `MERGING` and the banner drawn, the
/// focus where `u` found it, and no stop line (`stopped` stays unset,
/// `KNOWN_GAPS.md` row M6/12).
#[test]
fn u_after_a_shell_begun_finish_puts_the_merge_back() {
    let scratch = resolved_shell_merge();
    let repo = scratch.path().to_owned();
    let pre_merge = crate::app_harness::git(&repo, &["rev-parse", "main"]);
    let (app, state) = launched_into(&repo);
    let state = c_on_the_banner(&app, state);
    let state = enter_to_the_landing(&app, &state);
    let merge = crate::app_harness::git(&repo, &["rev-parse", "HEAD"]);
    let focused_graph = state.graph_has_focus();
    let banner_focus = state.banner_focus;

    let state = app.key(&state, KeyCode::Char('u'));
    assert!(state.operating.is_some(), "`u` ran: {:?}", state.status);
    let state = app.pump(state, "the undo and its refresh", |state| {
        state.operating.is_none() && state.in_progress.is_some()
    });
    let state = settled_frame(&app, state);

    let merge_head = std::fs::read_to_string(repo.join(".git/MERGE_HEAD")).expect("MERGE_HEAD");
    let feature = crate::app_harness::git(&repo, &["rev-parse", "feature"]);
    assert_eq!(merge_head.trim(), feature.trim(), "MERGE_HEAD is back");
    let main = crate::app_harness::git(&repo, &["rev-parse", "main"]);
    assert_eq!(main, pre_merge, "main is on its pre-merge tip");

    assert_eq!(
        state.status,
        Status::Done("undo: finish merge of feature".to_owned())
    );
    assert_eq!(
        state.in_progress.as_ref().map(|detail| detail.kind),
        Some(graft_git::state::InProgress::Merge)
    );
    assert!(
        graft::update::banner::banner_view(&state).is_some(),
        "the banner is drawn"
    );
    assert_eq!(state.graph_has_focus(), focused_graph, "focus unchanged");
    assert_eq!(state.banner_focus, banner_focus, "focus unchanged");
    assert!(state.stopped.is_none(), "no stop line: KNOWN_GAPS M6/12");

    let (frame, body) = shot(&app, &state, &repo, merge.trim());
    assert!(frame[0].trim_end().ends_with("MERGING"), "{body}");
    assert!(body.contains("undo: finish merge of feature"), "{body}");
    crate::captures::check_shot("m6/fix-undo-shell-merge-120x40.txt", &body);
    drop(state);
    app.close();
}

/// **A hook refusing keeps the summary** on a shell-begun merge, as on one
/// Graft began (`finish.rs::a_hook_refusal_keeps_the_summary_open`): the
/// hook's words on the summary, the typed subject kept, git still merging,
/// and the `FinishMerge` entry `Aborted`, not in force.
#[test]
fn a_hook_refusal_keeps_the_summary_on_a_shell_begun_merge() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = resolved_shell_merge();
    let repo = scratch.path().to_owned();
    let hook = repo.join(".git/hooks/commit-msg");
    std::fs::create_dir_all(hook.parent().expect("a hooks dir")).expect("writable");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho 'commit-msg: a ticket number is required' >&2\nexit 1\n",
    )
    .expect("writable");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let before = journal_entries(&repo).len();
    let (app, state) = launched_into(&repo);
    let state = c_on_the_banner(&app, state);
    let state = app.key(&state, KeyCode::Tab);
    let state = " (kept)"
        .chars()
        .fold(state, |state, typed| app.key(&state, KeyCode::Char(typed)));
    let subject = state
        .finish
        .as_ref()
        .map(|open| open.subject.text().to_owned())
        .expect("open");

    let state = app.key(&state, KeyCode::Enter);
    assert!(state.operating.is_some(), "`⏎` commits");
    let state = app.pump(state, "the refused finish", |state| {
        state.operating.is_none()
    });
    let open = state
        .finish
        .as_ref()
        .unwrap_or_else(|| panic!("the summary stays open: {:?}", state.status));
    let (words, problem) = open.notice.clone().expect("the refusal is on the summary");
    assert!(problem, "a refusal reads as a problem");
    assert!(
        words.contains("a ticket number is required"),
        "the hook's own words: {words:?}"
    );
    assert_eq!(open.subject.text(), subject, "the message is kept");
    assert!(
        repo.join(".git/MERGE_HEAD").exists(),
        "git is still merging"
    );
    let entries = journal_entries(&repo);
    assert_eq!(entries.len(), before + 1, "the finish was journaled");
    assert_eq!(
        entries[before].kind,
        graft_core::journal::OperationKind::FinishMerge
    );
    assert!(
        matches!(
            entries[before].status,
            graft_core::journal::EntryStatus::Aborted { .. }
        ),
        "{:?}",
        entries[before].status
    );
    drop(state);
    app.close();
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

// ------------------------ the editor's words reach the banner (m6-conflicts/39)

/// The words ticket 39 gives a merge begun in the shell.
const SHELL_WORDS: &str = "merge begun in the shell — esc, ⇥ banner, then c finishes it";

/// **The keys the commit editor names reach the summary** (ticket 39): from
/// J3's state, the editor opened from the changes pane says `esc, ⇥ banner,
/// then c`, and exactly those keys open the summary, returning to the banner.
#[test]
fn the_keys_the_commit_editor_names_reach_the_summary() {
    let scratch = resolved_shell_merge();
    let (app, state) = launched_into(scratch.path());
    let open = super::editor_words::editor_open(&app, state);
    assert_eq!(super::editor_words::editor_words(&open), SHELL_WORDS);

    let (banner, tabs) = super::editor_words::esc_then_tabs_to_the_banner(&app, &open);
    assert_eq!(tabs, 2, "Files → Diff → banner");
    let state = app.key(&banner, KeyCode::Char('c'));
    assert_eq!(
        state.finish.as_ref().map(|finish| finish.return_to),
        Some(graft::update::finish::ReturnTo::Banner),
        "`c` opens the summary: {:?}",
        state.status
    );
    drop(state);
    app.close();
}
