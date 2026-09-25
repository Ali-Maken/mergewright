//! A merge **begun in the shell**, in four groups:
//!
//! - a hand edit lost to a whole-file take, and the way the user gets it
//!   back (m6 g04/g08 review, M1);
//! - finishing it from Graft: `c ⏎` from J3's banner landing, a hook's
//!   refusal keeping the summary (ticket 20);
//! - `u` on that finish: the merge put back, the landing on the banner, and
//!   `c ⏎` finishing it again (tickets 20, 41);
//! - the commit editor's words naming the banner, and their fit at 80x24
//!   (tickets 39, 40).

use super::*;

const EDIT: &str = "my hand edit, not in any commit\n";

/// The user hand-edits `theirs-deleted.txt` (`UD`), then `t` takes theirs:
/// `git rm`, the file is gone. The take is its own journal entry with a
/// backup, so the edit is kept. This walk ends the merge in the shell
/// first: `git merge --abort`, back in the app (`Ctrl+R`), `u`, answer the
/// drift box with `Undo`. The edit's bytes are back on disk. (Since
/// m6-conflicts/57 `u` reaches the take while the merge is still open —
/// `u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge` — and this
/// route, the one a user who left for the shell takes, still holds.)
#[test]
fn a_hand_edit_taken_by_t_comes_back_after_the_merge_ends() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();

    let state = hand_edit_then_t(&app, state, &repo);
    let state = out_of_the_resolver_onto_the_banner(&app, &state, &repo);

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

/// `Esc` out of the resolver, then the banner. Leaves the banner focused
/// and the merge open.
///
/// Until m6-conflicts/57 this walk also pressed `u` here and saw it refused
/// (`finish or abort the merge first`). In a merge begun in the shell `u`
/// now reaches the take (keymap §2.6a's one exception), so the refusal is
/// gone; `u_still_refuses_in_a_merge_begun_in_graft` holds it for a merge
/// Graft began.
fn out_of_the_resolver_onto_the_banner(app: &App, state: &AppState, repo: &Path) -> AppState {
    let state = app.key(state, KeyCode::Escape);
    assert!(state.resolver.is_none(), "back to the pane");
    let state = on_the_banner(app, state);
    assert!(
        repo.join(".git/MERGE_HEAD").exists(),
        "the merge is still open"
    );
    state
}

// ------------------- `u` inside a merge begun in the shell (m6-conflicts/57)

/// The undo row's words after `u` on the take: the take's own description,
/// as its entry reads, under `undo:`.
const TAKE_UNDONE: &str = "undo: resolved theirs-deleted.txt (theirs)";

/// `u`, pressed wherever `state` has the keyboard, run to its refresh.
fn u_to_its_refresh(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('u'));
    assert!(
        state.operating.is_some(),
        "`u` was sent: {:?}",
        state.status
    );
    app.pump(state, "the undo and its refresh", |state| {
        state.operating.is_none() && state.status_read.is_none()
    })
}

/// The take is undone and the merge still stands: the hand edit back byte
/// for byte, the file unmerged again, `MERGE_HEAD` still there, one `Undo`
/// entry reversing the take.
fn the_take_is_back_inside_the_merge(repo: &Path, before: usize) {
    assert_eq!(
        on_disk(repo, "theirs-deleted.txt"),
        EDIT,
        "the hand edit is back, byte for byte"
    );
    assert!(
        still_unmerged(repo)
            .lines()
            .any(|line| line == "theirs-deleted.txt"),
        "the file is conflicted again: {:?}",
        still_unmerged(repo)
    );
    assert!(
        repo.join(".git/MERGE_HEAD").exists(),
        "the merge is still open"
    );
    let entries = journal_entries(repo);
    assert_eq!(entries.len(), before + 2, "the take, then its undo");
    assert_eq!(
        entries[before + 1].kind,
        graft_core::journal::OperationKind::Undo
    );
    assert_eq!(
        entries[before + 1].reverses.as_ref(),
        Some(&entries[before].id)
    );
}

/// **`u` on the banner restores a whole-file take in a merge begun in the
/// shell** (keymap §2.6a's one exception; `b-undo-a-take-in-the-shell`, 1
/// key). The take is an entry of its own there, and ops' gate lets its undo
/// through because its after-state is this same merge. The banner keeps the
/// keyboard, and the status reads `undo: <the take> ✓`.
#[test]
fn u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = crate::conflict_interim::launched(&app, state);
    assert!(state.live_entries.is_empty(), "begun in the shell");
    let before = journal_entries(&repo).len();

    let state = hand_edit_then_t(&app, state, &repo);
    let state = out_of_the_resolver_onto_the_banner(&app, &state, &repo);
    let state = u_to_its_refresh(&app, &state);

    the_take_is_back_inside_the_merge(&repo, before);
    assert!(state.banner_focus, "the banner keeps the keyboard");
    assert_eq!(state.status, Status::done(TAKE_UNDONE));
    drop(state);
    app.close();
    drop(scratch);
}

/// **`u` in the graph undoes a step of a shell merge**, as on the banner:
/// the global key is the same undo, and the graph keeps the keyboard.
#[test]
fn u_in_the_graph_undoes_a_step_of_a_shell_merge() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = crate::conflict_interim::launched(&app, state);
    let before = journal_entries(&repo).len();

    let state = hand_edit_then_t(&app, state, &repo);
    let state = out_of_the_resolver_onto_the_banner(&app, &state, &repo);
    // `Tab` from the banner is the next step of the cycle: the graph.
    let state = app.key(&state, KeyCode::Tab);
    assert!(state.graph_has_focus(), "the graph has the keyboard");
    let state = u_to_its_refresh(&app, &state);

    the_take_is_back_inside_the_merge(&repo, before);
    assert!(state.graph_has_focus(), "the graph keeps the keyboard");
    assert!(!state.banner_focus);
    assert_eq!(state.status, Status::done(TAKE_UNDONE));
    drop(state);
    app.close();
    drop(scratch);
}

// ------------------------- aborting it in the app, and `u` (m6-conflicts/57)

/// `git merge feature` run **in the shell** on `main` and left stopped on
/// `shared.rs` and `notes.md`: a merge Graft did not start, two files
/// conflicted. `about_to_conflict`'s commits, stamped at fixed instants, so
/// the graph draws the same every run.
fn stopped_shell_merge() -> graft_fixtures::Scratch {
    let scratch = crate::conflict_interim::about_to_conflict();
    git_stops(scratch.path(), &["merge", "feature"]);
    scratch
}

/// The frame at 120x40 against the fixture's fixed clock.
fn frame_120(app: &App, state: &AppState) -> Vec<String> {
    let sized = app.resize(state, 120, 40);
    crate::captures::frame_at(
        &sized,
        120,
        40,
        Some(graft::session::Clock::Fixed(
            crate::conflict_interim::CONFLICT_NOW,
        )),
    )
}

/// The one row holding `words`, trimmed; panics with the frame if none does.
fn row_with<'a>(frame: &'a [String], words: &str) -> &'a str {
    frame.iter().find(|row| row.contains(words)).map_or_else(
        || panic!("no row holds {words:?}:\n{}", frame.join("\n")),
        |row| row.trim(),
    )
}

/// What the undo of the abort must put back, read before the abort.
struct MergeFiles {
    head: String,
    merge_head: String,
    merge_msg: Vec<u8>,
    unmerged: String,
}

fn merge_files(repo: &Path) -> MergeFiles {
    MergeFiles {
        head: crate::app_harness::git(repo, &["rev-parse", "HEAD"]),
        merge_head: std::fs::read_to_string(repo.join(".git/MERGE_HEAD")).expect("MERGE_HEAD"),
        merge_msg: std::fs::read(repo.join(".git/MERGE_MSG")).expect("MERGE_MSG"),
        unmerged: still_unmerged(repo),
    }
}

/// The words the abort box says on a merge begun in the shell.
const ABORT_LINES: [&str; 2] = [
    "main goes back to before the merge",
    "u puts the merge back",
];

/// The `u` landing's row after the abort is taken back (ticket 41's landing).
const ABORT_UNDONE_WORDS: &str = "undo: abort merge of feature ✓ · c finishes the merge";

/// **`a` on the banner aborts a merge begun in the shell** (keymap §2.6a,
/// `b-abort-shell-merge`: `a → ⏎`, 3 keys), and `u` puts it back.
///
/// git: no `MERGE_HEAD` after the abort, `HEAD` unchanged, one
/// `AbortMerge` entry `abort merge of feature`; after `u`, `MERGE_HEAD`,
/// `MERGE_MSG` byte for byte and both files' stages are back. App: the box
/// says `main goes back to before the merge` / `u puts the merge back` with
/// the cursor on `[ Cancel ]`; the abort lands on the graph with
/// `abort merge of feature ✓`; `J` lists the entry; `u` lands on the banner
/// with `undo: abort merge of feature ✓ · c finishes the merge`. The three
/// frames are the capture the user judges.
#[test]
fn a_on_the_banner_aborts_a_shell_merge() {
    let scratch = stopped_shell_merge();
    let repo = scratch.path().to_owned();
    let before = journal_entries(&repo).len();
    let merge = merge_files(&repo);
    let (app, state) = launched_into(&repo);
    assert!(state.banner_focus, "the launch landed on the banner");
    let state = settled_frame(&app, state);

    let (state, boxed) = a_shows_the_abort_box(&app, &state);
    let (state, aborted) = right_enter_aborts(&app, &state, &repo, before, &merge.head);
    let state = j_lists_the_abort(&app, &state);
    let (state, undone) = u_puts_the_merge_back(&app, &state, &repo, &merge);

    let body = format!(
        "{ABORT_CAPTION}=== 1. a: the abort box ===\n{}\n=== 2. → ⏎: aborted ===\n{}\n=== 3. u: the merge back ===\n{}\n",
        boxed.join("\n"),
        aborted.join("\n"),
        undone.join("\n")
    );
    crate::captures::check_shot("m6/fix-shell-merge-abort-and-undo-120x40.txt", &body);
    drop(state);
    app.close();
    drop(scratch);
}

/// Frame 1, `a` (one key): the box, its two lines, the cursor on
/// `[ Cancel ]`.
fn a_shows_the_abort_box(app: &App, state: &AppState) -> (AppState, Vec<String>) {
    let state = app.key(state, KeyCode::Char('a'));
    let confirm = state.confirm.as_ref().expect("the abort box");
    assert_eq!(confirm.view.selected(), 0, "the cursor starts on Cancel");
    let lines: Vec<&str> = confirm
        .view
        .message
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(lines, ABORT_LINES);
    let boxed = frame_120(app, &state);
    for line in ABORT_LINES {
        row_with(&boxed, line);
    }
    row_with(&boxed, "[ Cancel ]");
    (state, boxed)
}

/// Frame 2, `→ ⏎` (two keys, three with `a`: the note's
/// `b-abort-shell-merge`, walked by `m5_walk`): git aborted, `HEAD` unmoved, one `AbortMerge`
/// entry, the graph focused with `abort merge of feature ✓`.
fn right_enter_aborts(
    app: &App,
    state: &AppState,
    repo: &Path,
    before: usize,
    head: &str,
) -> (AppState, Vec<String>) {
    let state = app.key(state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    assert!(state.operating.is_some(), "`⏎` aborts: {:?}", state.status);
    let state = app.pump(state, "the abort and its landing", |state| {
        state.operating.is_none()
            && state.in_progress.is_none()
            && state.status_read.is_none()
            && !state.land_on_head
    });
    let state = settled_frame(app, state);
    assert!(!repo.join(".git/MERGE_HEAD").exists(), "git aborted it");
    assert_eq!(
        crate::app_harness::git(repo, &["rev-parse", "HEAD"]),
        head,
        "HEAD is where it was"
    );
    let entries = journal_entries(repo);
    assert_eq!(entries.len(), before + 1, "one new entry");
    assert_eq!(
        entries[before].kind,
        graft_core::journal::OperationKind::AbortMerge
    );
    assert_eq!(entries[before].description, "abort merge of feature");
    assert!(state.graph_has_focus(), "the graph has the keyboard");
    assert_eq!(state.status, Status::done("abort merge of feature"));
    let aborted = frame_120(app, &state);
    assert!(!aborted[0].contains("MERGING"), "{}", aborted.join("\n"));
    assert_eq!(
        row_with(&aborted, "abort merge of feature"),
        "abort merge of feature ✓"
    );
    (state, aborted)
}

/// `J` lists the abort, a row of its own in the journal's words; `Esc`
/// closes the browser.
fn j_lists_the_abort(app: &App, state: &AppState) -> AppState {
    let journal = app.key(state, KeyCode::Char('J'));
    let journal = app.pump(journal, "the journal browser's read", |state| {
        state.journal.shown && state.journal.reading.is_none()
    });
    row_with(&frame_120(app, &journal), "abort merge of feature");
    let state = app.key(&journal, KeyCode::Escape);
    assert!(!state.journal.shown, "Esc closes the browser");
    state
}

/// Frame 3, `u`: `MERGE_HEAD`, `MERGE_MSG` byte for byte, both files' stages and
/// `HEAD` as before the abort; the banner has the keyboard, with ticket
/// 41's landing words.
fn u_puts_the_merge_back(
    app: &App,
    state: &AppState,
    repo: &Path,
    merge: &MergeFiles,
) -> (AppState, Vec<String>) {
    let state = app.key(state, KeyCode::Char('u'));
    assert!(state.operating.is_some(), "`u` ran: {:?}", state.status);
    let state = app.pump(state, "the undo and its landing", |state| {
        state.operating.is_none() && state.in_progress.is_some() && state.status_read.is_none()
    });
    let state = settled_frame(app, state);
    assert_eq!(
        std::fs::read_to_string(repo.join(".git/MERGE_HEAD")).expect("MERGE_HEAD is back"),
        merge.merge_head
    );
    assert_eq!(
        std::fs::read(repo.join(".git/MERGE_MSG")).expect("MERGE_MSG is back"),
        merge.merge_msg,
        "MERGE_MSG byte for byte"
    );
    assert_eq!(
        still_unmerged(repo),
        merge.unmerged,
        "both files conflicted"
    );
    assert_eq!(
        crate::app_harness::git(repo, &["rev-parse", "HEAD"]),
        merge.head
    );
    assert!(state.banner_focus, "the banner has the keyboard");
    assert_eq!(
        state.status,
        Status::done_with_keys("undo: abort merge of feature", "c finishes the merge")
    );
    let undone = frame_120(app, &state);
    let body = undone.join("\n");
    assert!(undone[0].trim_end().ends_with("MERGING"), "{body}");
    // The ticket's capture line asked for `CONFLICTS (2)`. Ticket 41's landing
    // (kept unchanged, as the ticket says) focuses the banner and leaves the
    // detail pane on the commit, so the two conflicts show as the graph's
    // working-tree row. Pinned as drawn; the question is in the ticket's NOTES.
    assert!(body.contains("Uncommitted changes (2 unmerged)"), "{body}");
    assert_eq!(row_with(&undone, "undo: abort merge"), ABORT_UNDONE_WORDS);
    (state, undone)
}

/// The capture's header: what the user judges in each frame.
const ABORT_CAPTION: &str = "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# a_on_the_banner_aborts_a_shell_merge in
# crates/graft/tests/it/resolver/shell_begun.rs (asserted). `git merge
# feature` run in the shell on `main`, stopped on two files; Graft launched
# on it, the banner focused (m6-conflicts/57).
#
# Judge by eye:
# - Frame 1 (`a`): the abort box over the banner reads
#   `main goes back to before the merge` and `u puts the merge back`, the
#   cursor on `[ Cancel ]`.
# - Frame 2 (`→ ⏎`): no `MERGING` in the header; the footer row reads
#   `abort merge of feature ✓`.
# - Frame 3 (`u`): `MERGING` again, the graph's top row
#   `Uncommitted changes (2 unmerged)`; the footer row reads
#   `undo: abort merge of feature ✓ · c finishes the merge`.
# - Colour and the focus highlight are unverified (CLAUDE.md rule 5).
#
";

/// **An entry from before the merge still refuses, in words** — ops' gate
/// answering through the app. The abort above, then `git merge feature`
/// again in the shell: `u` would take back the abort, which is older than
/// the merge now standing. Ops refuses (`NotInvertible::OperationLive`),
/// nothing is written, and the line is `u`'s own refusal.
#[test]
fn u_on_an_entry_from_before_a_shell_merge_refuses_in_words() {
    let scratch = stopped_shell_merge();
    let repo = scratch.path().to_owned();
    let (app, state) = launched_into(&repo);
    let state = on_the_banner(&app, state);
    let state = app.key(&state, KeyCode::Char('a'));
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the abort", |state| {
        state.operating.is_none() && state.in_progress.is_none() && state.status_read.is_none()
    });
    assert!(!repo.join(".git/MERGE_HEAD").exists(), "aborted");

    git_stops(&repo, &["merge", "feature"]);
    let state = app.press(&state, KeyPress::control(KeyCode::Char('r')));
    let state = app.pump(state, "the re-read of the new merge", |state| {
        state.in_progress.is_some()
            && state.status_read.is_none()
            && state.journal.reading.is_none()
    });
    let before = journal_entries(&repo);

    let state = app.key(&state, KeyCode::Char('u'));
    assert!(state.operating.is_some(), "`u` is sent: {:?}", state.status);
    let state = app.pump(state, "ops' answer", |state| state.operating.is_none());
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: "finish or abort the merge first".to_owned(),
            keys: Some("⇥ banner".to_owned()),
        }
    );
    assert_eq!(journal_entries(&repo), before, "nothing written");
    assert!(
        repo.join(".git/MERGE_HEAD").exists(),
        "the new merge stands"
    );
    drop(state);
    app.close();
    drop(scratch);
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
    shot_at(app, state, repo, merge, (120, 40))
}

/// [`shot`] at another size.
fn shot_at(
    app: &App,
    state: &AppState,
    repo: &Path,
    merge: &str,
    (columns, rows): (u16, u16),
) -> (Vec<String>, String) {
    let sized = app.resize(state, columns, rows);
    let frame = crate::captures::frame_at(
        &sized,
        columns,
        rows,
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
    // J3 lands on the banner (keymap §2.6a), so the count of 2 starts
    // here: `on_the_banner` below presses no `Tab` (review Minor 9).
    assert!(state.banner_focus, "the launch landed on the banner");
    let state = on_the_banner(&app, state);
    assert!(state.banner_focus);

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
    assert_eq!(state.status, Status::done("merged feature into main"));

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

/// The words the `u` landing's footer row reads (ticket 41): 55 columns.
const UNDO_LANDING_WORDS: &str = "undo: finish merge of feature ✓ · c finishes the merge";

/// Where `u` on a shell-begun finish lands, with what the tests below need
/// to check it against.
struct UndoLanding {
    scratch: graft_fixtures::Scratch,
    app: App,
    state: AppState,
    /// `main` before the merge was finished.
    pre_merge: String,
    /// The merge commit `u` took back.
    merge: String,
}

/// Launch into J3, `c ⏎` to finish, then `u`, run to its refresh.
fn u_landing() -> UndoLanding {
    let scratch = resolved_shell_merge();
    let repo = scratch.path().to_owned();
    let pre_merge = crate::app_harness::git(&repo, &["rev-parse", "main"]);
    let (app, state) = launched_into(&repo);
    let state = c_on_the_banner(&app, state);
    let state = enter_to_the_landing(&app, &state);
    assert!(state.graph_has_focus(), "the finish landed on the graph");
    let merge = crate::app_harness::git(&repo, &["rev-parse", "HEAD"]);

    let state = app.key(&state, KeyCode::Char('u'));
    assert!(state.operating.is_some(), "`u` ran: {:?}", state.status);
    let state = app.pump(state, "the undo and its refresh", |state| {
        state.operating.is_none() && state.in_progress.is_some() && state.status_read.is_none()
    });
    let state = settled_frame(&app, state);
    UndoLanding {
        scratch,
        app,
        state,
        pre_merge,
        merge,
    }
}

/// The one drawn row naming the undo, checked whole: the words, and one
/// tick — the renderer's, after `what` and before the key (ticket 41).
fn the_undo_row(frame: &[String], body: &str) {
    let row = frame
        .iter()
        .find(|row| row.contains("undo: finish merge of feature"))
        .unwrap_or_else(|| panic!("no row names the undo:\n{body}"));
    assert_eq!(row.trim(), UNDO_LANDING_WORDS, "{body}");
    assert_eq!(row.matches('✓').count(), 1, "one tick: {row:?}");
}

/// **`u` after a shell-begun finish puts the merge back.** git: `MERGE_HEAD`
/// is back, naming `feature`, and `main` is on its pre-merge tip. App: the
/// `u` landing (ticket 41) — `in_progress` a merge again after the refresh,
/// the header `MERGING`, **the banner focused**, as J3's launch into the
/// same state is, and the footer row `undo: finish merge of feature ✓ · c
/// finishes the merge`; no stop line (`stopped` stays unset, an undo is not
/// a stop). Captured at 120x40 and 80x24.
#[test]
fn u_after_a_shell_begun_finish_puts_the_merge_back() {
    let UndoLanding {
        scratch,
        app,
        state,
        pre_merge,
        merge,
    } = u_landing();
    let repo = scratch.path();

    let merge_head = std::fs::read_to_string(repo.join(".git/MERGE_HEAD")).expect("MERGE_HEAD");
    let feature = crate::app_harness::git(repo, &["rev-parse", "feature"]);
    assert_eq!(merge_head.trim(), feature.trim(), "MERGE_HEAD is back");
    let main = crate::app_harness::git(repo, &["rev-parse", "main"]);
    assert_eq!(main, pre_merge, "main is on its pre-merge tip");

    assert_eq!(
        state.status,
        Status::done_with_keys("undo: finish merge of feature", "c finishes the merge")
    );
    assert_eq!(
        state.in_progress.as_ref().map(|detail| detail.kind),
        Some(graft_git::state::InProgress::Merge)
    );
    assert!(
        graft::update::banner::banner_view(&state).is_some(),
        "the banner is drawn"
    );
    assert!(state.banner_focus, "the banner has the keyboard");
    assert!(state.stopped.is_none(), "an undo is not a stop");
    assert!(state.commit_editor.is_none(), "no commit editor");

    let (frame, body) = shot(&app, &state, repo, merge.trim());
    assert!(frame[0].trim_end().ends_with("MERGING"), "{body}");
    the_undo_row(&frame, &body);
    crate::captures::check_shot("m6/fix-undo-shell-merge-120x40.txt", &body);

    let (frame, body) = shot_at(&app, &state, repo, merge.trim(), (80, 24));
    // The narrow tier's tab strip is row 0; the header is under it.
    assert!(frame[1].trim_end().ends_with("MERGING"), "{body}");
    the_undo_row(&frame, &body);
    assert!(
        !body.contains(&format!("{}…", &UNDO_LANDING_WORDS[..20])),
        "{body}"
    );
    crate::captures::check_shot("m6/fix-undo-shell-merge-80x24.txt", &body);
    drop(state);
    app.close();
}

/// **`c ⏎` from the `u` landing finishes the merge again: 2 keys**
/// (`b-finish-again-after-undo`, ticket 41). `c` opens the finish summary,
/// not the commit editor; `⏎` commits a merge whose parents are the
/// pre-merge tip and `feature`, journaled as a `FinishMerge`.
#[test]
fn c_enter_after_u_finishes_the_merge_again() {
    let UndoLanding {
        scratch,
        app,
        state,
        pre_merge,
        ..
    } = u_landing();
    let repo = scratch.path();
    let merge_head = std::fs::read_to_string(repo.join(".git/MERGE_HEAD")).expect("MERGE_HEAD");
    let mut keys = 0;

    let state = app.key(&state, KeyCode::Char('c'));
    keys += 1;
    assert!(state.finish.is_some(), "`c` opened the summary");
    assert!(state.commit_editor.is_none(), "not the commit editor");
    let state = app.pump(state, "the finish summary", |state| {
        state.finish.as_ref().is_some_and(|open| open.job.is_none())
    });
    let state = app.key(&state, KeyCode::Enter);
    keys += 1;
    assert!(state.operating.is_some(), "`⏎` commits");
    let state = app.pump(state, "the finish and its landing", |state| {
        state.operating.is_none() && state.in_progress.is_none() && !state.land_on_head
    });

    assert_eq!(keys, 2, "the note's count");
    assert!(!repo.join(".git/MERGE_HEAD").exists(), "git finished it");
    let first = crate::app_harness::git(repo, &["rev-parse", "HEAD^1"]);
    let second = crate::app_harness::git(repo, &["rev-parse", "HEAD^2"]);
    assert_eq!(first, pre_merge, "first parent: the pre-merge tip");
    assert_eq!(
        second.trim(),
        merge_head.trim(),
        "second parent: MERGE_HEAD"
    );
    let newest = journal_entries(repo)
        .into_iter()
        .rev()
        .find(|entry| entry.status == graft_core::journal::EntryStatus::Committed)
        .expect("an entry in force");
    assert_eq!(
        newest.kind,
        graft_core::journal::OperationKind::FinishMerge,
        "{newest:?}"
    );
    drop(state);
    app.close();
}

/// **`U` on the banner at the `u` landing redoes the finish.** Ticket 41
/// pinned a refusal here. Since m6-conflicts/57 the merge standing after
/// that `u` counts as begun in the shell (no entry of Graft's is live), so
/// keymap §2.6a's exception applies: the focused banner sends `U`, and ops'
/// gate lets the redo through because the `Undo` entry it reverses left
/// this very merge. The merge commit is back, `MERGE_HEAD` gone.
#[test]
fn capital_u_on_the_banner_at_the_landing_redoes_the_finish() {
    let UndoLanding {
        scratch,
        app,
        state,
        merge,
        ..
    } = u_landing();
    let repo = scratch.path();
    assert!(state.banner_focus);
    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('U')));
    assert!(
        matches!(commands.as_slice(), [Command::Redo(_, false)]),
        "{commands:?}"
    );
    let state = app.key(&state, KeyCode::Char('U'));
    let state = app.pump(state, "the redo and its refresh", |state| {
        state.operating.is_none() && state.in_progress.is_none() && state.status_read.is_none()
    });
    assert!(
        !repo.join(".git/MERGE_HEAD").exists(),
        "the merge is finished"
    );
    assert_eq!(crate::app_harness::git(repo, &["rev-parse", "HEAD"]), merge);
    assert!(
        matches!(&state.status, Status::Done { what, .. } if what.contains("finish merge of feature")),
        "{:?}",
        state.status
    );
    drop(state);
    app.close();
    drop(scratch);
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
        matches!(&state.status, Status::Done { what: said, .. } if said.starts_with("undo:")),
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

/// Asserts the editor's frame at `columns`x`rows` carries `words` whole:
/// one row holds the exact sentence, no row names `git commit`, no row is
/// cut with `…` (m6-conflicts/40).
pub(super) fn the_words_fit(state: &AppState, words: &str, columns: u16, rows: u16) -> Vec<String> {
    let frame = crate::captures::frame(state, columns, rows);
    let body = frame.join("\n");
    assert!(
        frame.iter().any(|row| row.contains(words)),
        "no row holds `{words}` whole at {columns}x{rows}:\n{body}"
    );
    assert!(!body.contains("git commit"), "{body}");
    assert!(
        !frame.iter().any(|row| row.trim_end().ends_with('…')),
        "a row is cut at {columns}x{rows}:\n{body}"
    );
    frame
}

/// **The 80x24 check (ticket 39, then 40):** over J3 the whole sentence
/// shows at the narrow breakpoint, on its own row above the options row.
#[test]
fn the_shell_merge_words_fit_at_80_by_24() {
    let scratch = resolved_shell_merge();
    let (app, state) = launched_into(scratch.path());
    let open = super::editor_words::editor_open(&app, state);
    the_words_fit(&open, SHELL_WORDS, 80, 24);
    app.close();
}

/// Regenerates `docs/shots/m6/fix-editor-names-the-banner-{120x40,80x24}.txt`
/// (m6-conflicts/40): the commit editor over J3, its refusal on its own row.
#[test]
fn capture_fix_editor_names_the_banner() {
    let scratch = resolved_shell_merge();
    let (app, state) = launched_into(scratch.path());
    let open = settled_frame(&app, super::editor_words::editor_open(&app, state));
    let mut captures = graft_fixtures::Captures::new();
    for (columns, rows) in [(120_u16, 40_u16), (80, 24)] {
        let sized = app.resize(&open, columns, rows);
        let frame = crate::captures::frame_at(
            &sized,
            columns,
            rows,
            Some(graft::session::Clock::Fixed(
                crate::conflict_interim::CONFLICT_NOW,
            )),
        );
        the_words_fit(&sized, SHELL_WORDS, columns, rows);
        let header = format!(
            "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_fix_editor_names_the_banner in
# crates/graft/tests/it/resolver/shell_begun.rs (asserted). J3: a merge
# begun in the shell, resolved; the commit editor opened from the changes
# pane with `c` (m6-conflicts/40).
#
# Judge by eye:
# - The row directly above the options row reads, from column 3, exactly
#   `{SHELL_WORDS}` — whole at {columns} columns.
# - The options row ends at `[ Commit ]`, nothing after it.
# - `git commit` and `…` appear nowhere in the editor's rows.
# - The title row reads `COMMIT ── ⎇ main ── 2 files staged`.
# - The reason and the button are dimmed; colour is unverified (CLAUDE.md
#   rule 5), so the words carry it here.
#
=== {columns}x{rows} ===
"
        );
        captures.check(
            format!(
                "{}/../../docs/shots/m6/fix-editor-names-the-banner-{columns}x{rows}.txt",
                env!("CARGO_MANIFEST_DIR")
            ),
            format!("{header}{}\n", frame.join("\n")),
        );
    }
    app.close();
    captures.finish();
}

/// Regenerates `docs/shots/m6/fix-c-mid-merge-answers-in-words-{187x50,80x24}.txt`
/// (m6-conflicts/48): J3, the changes pane focused, `c` pressed — no editor,
/// the footer answers in words.
#[test]
fn capture_fix_c_mid_merge_answers_in_words() {
    let scratch = resolved_shell_merge();
    let (app, state) = launched_into(scratch.path());
    let pane = settled_frame(&app, super::editor_words::pane_focused(&app, state));
    let refused = app.key(&pane, KeyCode::Char('c'));
    assert!(refused.commit_editor.is_none(), "`c` opened the editor");
    assert_eq!(refused.status, super::editor_words::shell_merge_refusal());
    let mut captures = graft_fixtures::Captures::new();
    for (columns, rows) in [(187_u16, 50_u16), (80, 24)] {
        let sized = app.resize(&refused, columns, rows);
        let frame = crate::captures::frame_at(
            &sized,
            columns,
            rows,
            Some(graft::session::Clock::Fixed(
                crate::conflict_interim::CONFLICT_NOW,
            )),
        );
        let body = frame.join("\n");
        assert!(body.contains("MERGING"), "{body}");
        assert!(
            frame
                .iter()
                .any(|row| row.contains("merge begun in the shell")
                    && row.contains("⇥ banner, then c finishes it")),
            "no footer row holds the reason and the keys whole at {columns}x{rows}:\n{body}"
        );
        assert!(!body.contains("COMMIT ──"), "an editor is drawn:\n{body}");
        let header = format!(
            "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_fix_c_mid_merge_answers_in_words in
# crates/graft/tests/it/resolver/shell_begun.rs (asserted). J3: a merge
# begun in the shell, resolved; the changes pane focused, `c` pressed
# (m6-conflicts/48).
#
# Judge by eye:
# - No commit editor is drawn.
# - The header reads `MERGING`.
# - The footer holds the reason `merge begun in the shell` and the keys
#   `⇥ banner, then c finishes it`, the keys whole at {columns} columns.
# - Colour is unverified (CLAUDE.md rule 5).
#
=== {columns}x{rows} ===
"
        );
        captures.check(
            format!(
                "{}/../../docs/shots/m6/fix-c-mid-merge-answers-in-words-{columns}x{rows}.txt",
                env!("CARGO_MANIFEST_DIR")
            ),
            format!("{header}{}\n", frame.join("\n")),
        );
    }
    app.close();
    captures.finish();
}
