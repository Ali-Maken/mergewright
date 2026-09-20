//! A real conflict, through the real app (m5-history-and-ghost-preview/09,
//! pieces 1–4).
//!
//! `update::banner`'s unit tests settle what each key *means* over scripted
//! state. Nothing there runs `git`, so nothing there can prove that a merge
//! launched with `m` actually stops, that `in_progress` and the unmerged
//! paths come back off disk, that the banner the graph draws names the file
//! git is stuck on, or that `a` really puts the working tree back. That is
//! what these do: real `git`, real job pool, real keys, and the repository
//! read back afterwards rather than the status line.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::update::{AppEvent, AppState, Command, Status, update};
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_git::state::InProgress;

use crate::app_harness::{App, git, git_as_graft};

/// Two branches that both rewrote the same two lines: merging `feature` into
/// `main` stops with `shared.rs` and `notes.md` unmerged.
///
/// Two files, not one, so `CONFLICTS (2)` and "2 files still conflicted" are
/// read off a real count rather than a hard-coded singular.
fn about_to_conflict() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    write(path, "shared.rs", "fn main() {}\n");
    write(path, "notes.md", "the plan\n");
    commit(path, "the shared files");

    git(path, &["switch", "--quiet", "-c", "feature"]);
    write(path, "shared.rs", "fn main() { theirs() }\n");
    write(path, "notes.md", "their plan\n");
    commit(path, "the feature's take");

    git(path, &["switch", "--quiet", "main"]);
    write(path, "shared.rs", "fn main() { ours() }\n");
    write(path, "notes.md", "our plan\n");
    commit(path, "main's take");
    scratch
}

fn write(repo: &Path, name: &str, body: &str) {
    std::fs::write(repo.join(name), body).expect("the scratch repo is writable");
}

fn commit(repo: &Path, subject: &str) {
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-m", subject]);
}

/// Opened, with the first rows and the refs on hand.
fn opened(app: &App, state: AppState) -> AppState {
    app.pump(state, "the first rows and the refs", |state| {
        state.refs.is_some()
            && state.working_tree.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    })
}

/// Is the cursor's row carrying `feature`'s badge?
fn on_feature(state: &AppState) -> bool {
    let Some(refs) = state.refs.as_deref() else {
        return false;
    };
    let Some(row) = state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
    else {
        return false;
    };
    refs.on(row.commit.id).iter().any(
        |label| matches!(label, graft::refs::RefLabel::Branch { name, .. } if name == b"feature"),
    )
}

/// Cursor onto `feature`, `m`, through the preview and the confirm box, and
/// wait for the merge to stop. The state it hands back is a real conflict.
fn stopped_on_the_conflict(app: &App, state: AppState) -> AppState {
    let mut state = opened(app, state);
    let mut guard = 0;
    loop {
        assert!(guard < 20, "never found feature's row");
        if on_feature(&state) {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
        guard += 1;
    }
    let state = app.key(&state, KeyCode::Char('m'));
    assert!(
        state.preview.is_some(),
        "`m` opens a preview: {:?}",
        state.status
    );
    let state = app.pump(state, "the merge preview's plan", |state| {
        state.preview.as_ref().is_some_and(|preview| {
            !matches!(preview.plan, graft::update::detail::Loading::Pending(_))
        })
    });
    // `→` off `Cancel` onto `Confirm` (ADR-0038 §1), then `Enter` runs it.
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the merge", |state| state.operating.is_none());
    // Then the re-read the conflict asked for: the banner cannot name a file
    // before the working tree comes back.
    app.pump(state, "the working tree after the conflict", |state| {
        state
            .in_progress
            .as_ref()
            .is_some_and(|detail| detail.kind == InProgress::Merge)
            && !graft::update::banner::unmerged_paths(state).is_empty()
    })
}

/// The journal on disk, as the app's own reader sees it.
fn journal_entries(repo: &Path) -> Vec<graft_core::journal::JournalEntry> {
    graft_ops::journal::Journal::open(&repo.join(".git/graft/undo-journal.jsonl"))
        .expect("the journal the merge just wrote is readable")
        .entries()
        .to_vec()
}

/// What `update` alone answers for one key — the commands, not just the
/// state. The harness dispatches every command to the job pool, and a
/// hand-over command is the loop's, never a job's, so a test about one reads
/// it here instead of pressing the key.
fn commands_for(state: &AppState, key: KeyPress) -> Vec<Command> {
    update(state, AppEvent::Core(Event::Key(key))).1
}

/// The changes pane, reached from wherever the cursor happens to be — the
/// shared harness helper assumes the cursor is on the top row, and after a
/// merge it is on the row the merge came from.
fn open_changes_pane(app: &App, state: AppState) -> AppState {
    let mut state = state;
    let mut guard = 0;
    while !state.on_working_tree {
        assert!(guard < 40, "`k` never reached the uncommitted-changes row");
        state = app.key(&state, KeyCode::Char('k'));
        guard += 1;
    }
    let state = app.key(&state, KeyCode::Enter);
    app.pump(state, "the changes pane's file list", |state| {
        matches!(state.detail.files, graft::update::detail::Loading::Ready(_))
    })
}

/// `Tab` round the cycle until the banner has the keyboard again.
fn tab_back_to_the_banner(app: &App, state: AppState) -> AppState {
    let mut state = state;
    let mut guard = 0;
    while !state.banner_focus {
        assert!(guard < 6, "Tab never came back round to the banner");
        state = app.key(&state, KeyCode::Tab);
        guard += 1;
    }
    state
}

/// Marks `name` resolved the way the user does: cursor onto its `CONFLICTS`
/// row, `Space`, and wait for the working tree to come back without it.
fn mark_resolved(app: &App, state: AppState, name: &str) -> AppState {
    let state = crate::app_harness::cursor_to(app, state, name);
    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.settle_write(state, &format!("marking {name} resolved"));
    app.pump(state, "the file list after the resolution", |state| {
        !graft::update::banner::unmerged_paths(state)
            .iter()
            .any(|path| path.as_os_str() == name)
            && matches!(state.detail.files, graft::update::detail::Loading::Ready(_))
    })
}

// ---------------------------------------------------------------------------
// Piece 1 and 2: the facts, and the banner built from them.
// ---------------------------------------------------------------------------

#[test]
fn conflict_focuses_the_banner_and_lists_the_files() {
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);

    assert!(
        repo.join(".git/MERGE_HEAD").exists(),
        "git really is standing in a merge"
    );
    assert!(
        state.banner_focus,
        "the thing that stopped takes the keyboard: {:?}",
        state.status
    );
    let banner = graft::update::banner::banner_view(&state).expect("a banner is drawn");
    assert_eq!(banner.kind.as_str(), "MERGING");
    assert!(banner.focused, "and it is drawn focused");
    assert!(!banner.skippable, "a merge has no step to skip");
    assert_eq!(
        banner
            .conflict
            .as_ref()
            .map(graft_term::SanitizedText::as_str),
        Some("notes.md"),
        "the banner names the first conflicted path, in git's own order"
    );
    assert_eq!(banner.more_conflicts, 1, "and says how many more there are");

    let paths = graft::update::banner::unmerged_paths(&state);
    assert_eq!(
        paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>(),
        ["notes.md", "shared.rs"],
        "both files are unmerged"
    );

    // Nothing was closed: a live conflict is one `Pending` entry, exactly
    // the shape group 07 wrote (as-built §1).
    let entries = journal_entries(repo);
    let last = entries.last().expect("the merge wrote an entry");
    assert_eq!(
        last.status,
        graft_core::journal::EntryStatus::Pending,
        "the entry is still open while the conflict is"
    );
    assert!(
        last.head_after.is_none(),
        "and no after-state was written for it"
    );

    drop(state);
    app.close();
}

#[test]
fn banner_is_modal_when_focused() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = stopped_on_the_conflict(&app, state);

    let before = graft::update::banner::unmerged_paths(&state);
    for key in ['u', 'j', 'S', 'd', 'i'] {
        let after = app.key(&state, KeyCode::Char(key));
        assert!(
            after.operating.is_none(),
            "`{key}` started a second thing on top of the conflict: {:?}",
            after.status
        );
        assert!(
            after.confirm.is_none() && after.preview.is_none(),
            "`{key}` opened something over the banner"
        );
        assert!(
            after.banner_focus,
            "`{key}` should not have taken the keyboard off the banner"
        );
        assert_eq!(
            graft::update::banner::unmerged_paths(&after),
            before,
            "`{key}` changed the conflict"
        );
    }

    // `Ctrl+C` is the one key the banner never claims (§2.1): it reaches
    // whatever the app does with it, which is not "nothing".
    let control_c = KeyPress {
        code: KeyCode::Char('c'),
        modifiers: Modifiers::CONTROL,
    };
    let (after, _) = update(&state, AppEvent::Core(Event::Key(control_c)));
    assert!(
        after.should_quit() || after.confirm.is_some() || !matches!(after.status, Status::Ready),
        "Ctrl+C passed the banner and was answered: {:?}",
        after.status
    );

    // `Tab` is the door out of the mode.
    let left = app.key(&state, KeyCode::Tab);
    assert!(!left.banner_focus, "Tab gives the keyboard back");
    assert!(
        graft::update::banner::banner_view(&left).is_some(),
        "and the banner is still drawn — the operation has not gone anywhere"
    );

    drop(state);
    app.close();
}

// ---------------------------------------------------------------------------
// Piece 3: the three verbs.
// ---------------------------------------------------------------------------

#[test]
fn continue_refuses_while_unmerged_then_finishes() {
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);

    // `c` with both files unmerged: words, no job.
    let refused = app.key(&state, KeyCode::Char('c'));
    assert!(
        refused.operating.is_none(),
        "the refusal ran nothing: {:?}",
        refused.status
    );
    match &refused.status {
        Status::Sticky { reason, keys } => {
            assert_eq!(reason, "2 files still conflicted");
            assert_eq!(
                keys.as_deref(),
                Some("M mergetool  e edit  space marks resolved")
            );
        }
        other => panic!("`c` did not refuse in words: {other:?}"),
    }
    assert!(
        repo.join(".git/MERGE_HEAD").exists(),
        "and the merge is exactly where it was"
    );

    // Resolve both on disk and mark them resolved the way the user does:
    // `Space` on the `CONFLICTS` row, which stages the file (piece 4).
    write(repo, "shared.rs", "fn main() { both() }\n");
    write(repo, "notes.md", "the agreed plan\n");
    let state = app.key(&refused, KeyCode::Tab);
    assert!(!state.banner_focus, "Tab hands the keyboard to the panes");
    let state = open_changes_pane(&app, state);
    let state = mark_resolved(&app, state, "notes.md");
    let state = mark_resolved(&app, state, "shared.rs");
    assert!(
        graft::update::banner::unmerged_paths(&state).is_empty(),
        "both files are marked resolved"
    );

    // Back into the banner the same way the user left it, and `c` runs.
    // The banner lets go only when git does.
    let state = tab_back_to_the_banner(&app, state);
    let state = app.key(&state, KeyCode::Char('c'));
    let state = app.pump(state, "the continue", |state| state.operating.is_none());
    let state = app.pump(state, "the working tree after the merge", |state| {
        state.in_progress.is_none()
    });
    assert!(
        !state.banner_focus,
        "the keyboard comes back when the operation ends"
    );
    assert!(
        graft::update::banner::banner_view(&state).is_none(),
        "and the banner is gone with it"
    );
    assert!(
        !repo.join(".git/MERGE_HEAD").exists(),
        "git finished the merge"
    );
    assert_eq!(
        git(repo, &["rev-list", "--count", "--merges", "HEAD"]).trim(),
        "1",
        "the merge commit is on the branch"
    );

    drop(state);
    app.close();
}

#[test]
fn abort_always_offered_and_restores() {
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let before_tree = git(repo, &["rev-parse", "HEAD^{tree}"]);
    let before_head = git(repo, &["rev-parse", "HEAD"]);
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);

    // Offered after a failed `c`, which is the state a user most often
    // reaches it from.
    let state = app.key(&state, KeyCode::Char('c'));
    assert!(matches!(state.status, Status::Sticky { .. }), "`c` refused");

    let state = app.key(&state, KeyCode::Char('a'));
    let pending = state.confirm.as_ref().expect("`a` asks first");
    assert_eq!(pending.view.title.as_str(), "Abort the merge?");
    assert_eq!(pending.view.command.as_str(), "git merge --abort");
    let labels: Vec<&str> = pending
        .view
        .choices
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(labels, ["Cancel", "Abort"], "Cancel first (ADR-0038 §1)");

    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the abort", |state| state.operating.is_none());
    let state = app.pump(state, "the working tree after the abort", |state| {
        state.in_progress.is_none()
    });

    assert!(!repo.join(".git/MERGE_HEAD").exists(), "the merge is over");
    assert_eq!(
        git(repo, &["rev-parse", "HEAD"]),
        before_head,
        "the branch is back where it started"
    );
    assert_eq!(
        git(repo, &["rev-parse", "HEAD^{tree}"]),
        before_tree,
        "and so is the tree, byte for byte"
    );
    assert!(!state.banner_focus, "and the banner has let go");

    drop(state);
    app.close();
}

#[test]
fn skip_on_a_merge_says_nothing_to_skip() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = stopped_on_the_conflict(&app, state);

    let after = app.key(&state, KeyCode::Char('s'));
    assert!(after.confirm.is_none(), "no box: there is nothing to ask");
    assert!(after.operating.is_none(), "and nothing ran");
    match &after.status {
        Status::Sticky { reason, keys } => {
            assert_eq!(reason, "nothing to skip in a merge");
            assert_eq!(keys.as_deref(), Some("c continue  a abort"));
        }
        other => panic!("`s` said nothing useful: {other:?}"),
    }

    drop(state);
    app.close();
}

// ---------------------------------------------------------------------------
// Piece 4: the hand-offs.
// ---------------------------------------------------------------------------

#[test]
fn mergetool_absent_refuses_in_words() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = stopped_on_the_conflict(&app, state);
    assert!(
        state.mergetool.is_none(),
        "the fixture configures no merge.tool"
    );

    let commands = commands_for(&state, KeyPress::plain(KeyCode::Char('M')));
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::OpenMergetool { .. })),
        "nothing was handed over: {commands:?}"
    );
    let after = app.key(&state, KeyCode::Char('M'));
    match &after.status {
        Status::Sticky { reason, keys } => {
            assert_eq!(reason, "no merge.tool configured");
            assert!(
                keys.as_deref().is_some_and(|keys| keys.contains("e edit")),
                "and it names the way out: {keys:?}"
            );
        }
        other => panic!("`M` with no tool said nothing useful: {other:?}"),
    }

    drop(state);
    app.close();
}

#[test]
fn mergetool_present_asks_the_loop_for_the_first_conflict() {
    let scratch = about_to_conflict();
    let repo = scratch.path();
    git(repo, &["config", "merge.tool", "meld"]);
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);
    assert_eq!(
        state.mergetool.as_deref(),
        Some("meld"),
        "the configured tool is read off disk with the rest of the facts"
    );

    let commands = commands_for(&state, KeyPress::plain(KeyCode::Char('M')));
    let asked: Vec<&Path> = commands
        .iter()
        .filter_map(|command| match command {
            Command::OpenMergetool { path } => Some(path.as_path()),
            _ => None,
        })
        .collect();
    assert_eq!(
        asked,
        [Path::new("notes.md")],
        "one hand-over, for the first conflicted file: {commands:?}"
    );

    drop(state);
    app.close();
}

#[test]
fn e_opens_the_editor_at_the_first_marker() {
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);

    let commands = commands_for(&state, KeyPress::plain(KeyCode::Char('e')));
    let asked: Vec<&Path> = commands
        .iter()
        .filter_map(|command| match command {
            Command::OpenConflictEditor { path } => Some(path.as_path()),
            _ => None,
        })
        .collect();
    assert_eq!(
        asked,
        [Path::new("notes.md")],
        "`e` sends the first conflicted file to the loop: {commands:?}"
    );

    // The line the loop then opens it at: the marker git wrote, read from
    // the real file. `update` cannot read it itself — it is pure — so this
    // is where the two halves are checked against each other.
    let line = graft_git::history_reads::first_marker_line(&repo.join("notes.md"))
        .expect("the conflicted file is readable");
    assert_eq!(
        line,
        Some(1),
        "git puts `<<<<<<< ` on the first line of a one-line conflict"
    );

    drop(state);
    app.close();
}

#[test]
fn space_on_a_conflicted_row_refuses_while_markers_remain() {
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);

    let state = app.key(&state, KeyCode::Tab);
    let state = open_changes_pane(&app, state);
    let rows = crate::app_harness::rows(&state);
    assert_eq!(
        rows,
        ["CONFLICTS", "notes.md", "shared.rs"],
        "the conflicts are listed first, once each: a conflicted file is one \
         thing to do, not an unstaged change as well"
    );

    // The file on disk still has git's markers in it. Marking it resolved
    // would commit them, so `graft_ops` refuses and the app says which file
    // and what to press.
    let state = crate::app_harness::cursor_to(&app, state, "notes.md");
    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.pump(state, "the refused stage", |state| {
        state.operating.is_none()
    });
    match &state.status {
        Status::Sticky { reason, keys } => {
            assert_eq!(reason, "notes.md still has conflict markers");
            assert_eq!(keys.as_deref(), Some("e edit  M mergetool"));
        }
        other => panic!("staging a file full of markers was not refused: {other:?}"),
    }
    // `diff --cached` always lists an unmerged path during a merge, so the
    // index is read the only way that answers the question: is this path
    // still unmerged?
    assert!(
        !git(repo, &["ls-files", "--unmerged", "--", "notes.md"])
            .trim()
            .is_empty(),
        "the file is still unmerged: nothing was marked resolved"
    );

    drop(state);
    app.close();
}

// ---------------------------------------------------------------------------
// Piece 5: live is not interrupted. A live conflict and an entry a previous
// run tore in half are the **same bytes** — a `Pending` line with no outcome
// — and only git's own state tells them apart (m5/09 seam A).
// ---------------------------------------------------------------------------

/// Both launch reads have landed: the working tree (which fills
/// `in_progress`) and the journal (which fills `live_entries`). Either can
/// land second, and the takeover needs both.
fn launched(app: &App, state: AppState) -> AppState {
    app.pump(state, "the launch reads", |state| {
        state.working_tree.is_some()
            && matches!(
                state.journal.entries,
                graft::update::detail::Loading::Ready(_)
            )
            && state.journal.reading.is_none()
    })
}

/// The journal read that follows a write has landed, so `live_entries` is
/// about the conflict that just happened rather than the state before it.
fn journal_caught_up(app: &App, state: AppState) -> AppState {
    app.pump(state, "the journal read the conflict asked for", |state| {
        state.journal.reading.is_none()
            && matches!(
                state.journal.entries,
                graft::update::detail::Loading::Ready(_)
            )
    })
}

/// Killed mid-merge and started again: git's state is still there, so the
/// banner takes the keyboard on its own and the line says whose operation it
/// is (PROMPT.md §9 — `kill -9` mid-rebase is answered by a key, not by
/// reading docs).
#[test]
fn relaunch_into_a_live_conflict() {
    let scratch = about_to_conflict();
    let repo = scratch.path();

    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);
    let pending = journal_entries(repo)
        .last()
        .expect("the merge wrote an entry")
        .id
        .clone();
    drop(state);
    app.close();

    // A second Graft over the same worktree, the way a relaunch arrives.
    let (app, state) = App::open(repo);
    let state = launched(&app, state);

    assert_eq!(
        state.live_entries,
        vec![pending],
        "the launch read names the entry git is still part-way through, \
         and closes nothing"
    );
    assert!(
        state.banner_focus,
        "the banner takes the keyboard at launch: {:?}",
        state.status
    );
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: "merge still in progress from a previous run".to_owned(),
            keys: Some("c continue  a abort".to_owned()),
        },
        "and the line says whose operation it is, with the two keys that end it"
    );
    // Still open on disk: the launch read recovered nothing (as-built §1).
    assert_eq!(
        journal_entries(repo)
            .last()
            .expect("the entry is still there")
            .status,
        graft_core::journal::EntryStatus::Pending,
    );

    drop(state);
    app.close();
}

/// The dangerous version of this change closes nothing and calls every
/// `Pending` entry live. This is the other side: the **same** journal bytes
/// with git's merge state gone. M3's recovery path must be exactly what it
/// was — the entry is closed, its backup sealed, and the user told.
#[test]
fn interrupted_entry_without_live_state_is_still_closed() {
    let scratch = about_to_conflict();
    let repo = scratch.path();

    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);
    drop(state);
    app.close();

    // The merge ended outside Graft, so the journal's `Pending` line is now
    // torn rather than live. Nothing about the journal file changed.
    git(repo, &["merge", "--abort"]);
    assert!(
        !repo.join(".git/MERGE_HEAD").exists(),
        "git's merge state is gone"
    );

    let (app, state) = App::open(repo);
    let state = launched(&app, state);

    assert!(
        state.live_entries.is_empty(),
        "nothing is live, so nothing is spared recovery"
    );
    let last = journal_entries(repo)
        .last()
        .expect("the entry is still there")
        .clone();
    match &last.status {
        graft_core::journal::EntryStatus::Aborted { reason } => assert!(
            reason.contains("interrupted"),
            "recovery closes it as interrupted: {reason}"
        ),
        other => panic!("the M3 recovery path must still close it: {other:?}"),
    }
    let notice =
        graft::update::undo::interrupted_notice(&state).expect("and the user is told about it");
    assert!(notice.contains("an operation was interrupted"), "{notice}");

    let rows = graft::view::journal_view(&state, 2_000_000_000).rows;
    let row = rows.first().expect("the torn entry is the newest row");
    assert_eq!(row.mark, graft_ui::journal_screen::RowMark::Interrupted);
    assert!(
        row.summary.as_str().starts_with("interrupted"),
        "{}",
        row.summary.as_str()
    );

    drop(state);
    app.close();
}

/// The browser's row for the operation standing right there. Not
/// `interrupted` — nothing was torn — and not `← u`: there is no outcome to
/// take back until the merge finishes or is aborted.
#[test]
fn journal_row_for_live_entry_says_in_progress() {
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);
    let state = journal_caught_up(&app, state);

    assert_eq!(state.live_entries.len(), 1, "one live entry, the merge");

    let rows = graft::view::journal_view(&state, 2_000_000_000).rows;
    let row = rows.first().expect("the merge is the newest row");
    assert!(
        row.summary
            .as_str()
            .starts_with("in progress \u{2014} c continue  a abort"),
        "the row offers the keys that end it: {}",
        row.summary.as_str()
    );
    assert!(
        !row.summary.as_str().contains("interrupted"),
        "a merge standing right there was not interrupted: {}",
        row.summary.as_str()
    );
    assert_eq!(
        row.mark,
        graft_ui::journal_screen::RowMark::None,
        "never `interrupted`, never `\u{2190} u`"
    );
    assert_eq!(
        row.failure, None,
        "nothing failed: the operation has not finished happening"
    );

    drop(state);
    app.close();
}

/// The line the three captures were waiting on. The footer used to claim an
/// operation was interrupted about the merge the banner two rows above was
/// naming, and that claim won over the banner's own key hints.
#[test]
fn the_footer_does_not_call_a_live_merge_interrupted() {
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);
    let state = journal_caught_up(&app, state);

    assert_eq!(
        graft::update::undo::interrupted_notice(&state),
        None,
        "the notice is about entries a previous run left part-written, \
         and this merge is live"
    );

    for (columns, rows) in [(80u16, 24u16), (120, 40), (160, 48)] {
        let drawn = frame(&state, columns, rows);
        assert!(
            !drawn
                .iter()
                .any(|line| line.contains("an operation was interrupted")),
            "the footer must not say it either, at {columns}x{rows}:\n{}",
            drawn.join("\n")
        );
    }
    // What the line reads instead: the operation's own state, in the words
    // `update::operate` put there when git stopped. A state, not an error
    // (CLAUDE.md, "a status field must never read as an error").
    let drawn = frame(&state, 120, 40);
    assert!(
        drawn
            .iter()
            .any(|line| line.contains("merge feature into main")),
        "the live merge names itself instead:\n{}",
        drawn.join("\n")
    );

    drop(state);
    app.close();
}

// ---------------------------------------------------------------------------
// Piece 9: the captures. Not an assertion — the user judges these by eye
// (CLAUDE.md rule 6).
// ---------------------------------------------------------------------------

/// Every row of the frame `view::view` draws for `state` at this size.
fn frame(state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    let session = graft::session::Session::settle(
        &graft::config::Loaded::default(),
        crate::hint_bar::capabilities(),
    );
    let mut state = state.clone();
    state.size = graft::update::Size { columns, rows };
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(columns, rows))
        .expect("test backend opens");
    terminal
        .draw(|frame| graft::view::view(&session, &state, frame))
        .expect("drawing into a test backend cannot fail");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// Writes one capture into `docs/shots/` and says where it went.
fn write_shot(name: &str, body: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/shots")
        .join(name);
    std::fs::write(&path, body).expect("docs/shots is writable");
    println!("wrote {} ({} bytes)", path.display(), body.len());
}

/// The header every banner capture carries: how it was made, what right
/// looks like, and what is still wrong in it and whose that is.
fn capture_header(label: &str, columns: u16, rows: u16) -> String {
    format!(
        "# m5-ui-banner-conflict-{label}.txt: {columns}x{rows}, drawn by `view::view`\n\
             # from a real AppState \u{2014} a real repository whose `main` and `feature`\n\
             # both rewrote the same two files, opened over the real job pool, the\n\
             # merge launched with `m` through the preview and the confirm box, and\n\
             # git really stopped on the conflict (crates/graft/tests/it/\n\
             # conflict_interim.rs, capture_banner_for_docs_shots, #[ignore]d). Not a\n\
             # pty capture: the terminal is ratatui's TestBackend, so this is a\n\
             # character grid with no colour attributes in it \u{2014} CLAUDE.md rule 5,\n\
             # colour ships unverified. The scratch repository's temporary path is\n\
             # replaced by `/code/api`; nothing else is touched.\n\
             #\n\
             # What right looks like:\n\
             #  - A banner above the panes reading `MERGING`, the file git stopped on\n\
             #    (`notes.md`), and `+1 more` for the second one \u{2014} the state the\n\
             #    repository is in, said once, at the top.\n\
             #  - The banner is drawn focused (colour, so unverified here), and the\n\
             #    footer says nothing false about the merge: the line that used to\n\
             #    read `an operation was interrupted` is gone \u{2014} see below.\n\
             #  - The changes pane opens with `CONFLICTS (2)`, above `STAGED` and\n\
             #    `UNSTAGED`, and each conflicted file appears **once**: there is one\n\
             #    thing to do to it, not two.\n\
             #  - Nothing on the screen reads as an error. A stopped merge is a state\n\
             #    to finish, and the words are about what to press next.\n\
             #\n\
             # Re-captured after piece 5 (m5/14). The `an operation was interrupted`\n\
             # line these captures used to carry is gone: the journal now tells a\n\
             # live entry from an interrupted one, and a merge standing right there\n\
             # is neither announced nor drawn as torn.\n\
             #\n\
             # STILL WRONG, and it is **not** piece 5's \u{2014} filed by m5/14, needs\n\
             # `graft-ui`: the hint bar does not teach the banner's keys. Every\n\
             # entry of `graph_screen::hints::banner_list` is still\n\
             # `HintState::Planned`, which is never drawn, and `c continue` /\n\
             # `s skip` / `a abort` are missing from `update::keys::bound_actions`\n\
             # \u{2014} so the pair of tests that would have caught it agree with each\n\
             # other and pass. Group 09 bound the keys in `banner_key` without\n\
             # flipping the table. Until that is flipped, the footer under a focused,\n\
             # modal banner teaches `<> resize  . more` and nothing about `c` or `a`.\n\
             # Judge the banner, the pane and the absence of the interrupted line\n\
             # here; the hint bar is the next ticket's.\n\
             #\n\n"
    )
}

/// Writes the banner captures for `docs/shots/`.
///
/// Run by hand: `cargo test -p graft --test it
/// conflict_interim::capture_banner_for_docs_shots -- --ignored --nocapture`.
#[test]
#[ignore = "writes docs/shots/m5-ui-banner-conflict-*.txt; run by hand to regenerate"]
fn capture_banner_for_docs_shots() {
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);
    // The changes pane open on the conflicts, and the keyboard back on the
    // banner: the screen the user is actually looking at.
    let state = app.key(&state, KeyCode::Tab);
    let state = open_changes_pane(&app, state);
    let state = tab_back_to_the_banner(&app, state);
    let home = repo.display().to_string();

    for (label, columns, rows) in [
        ("80x24", 80u16, 24u16),
        ("120x40", 120, 40),
        ("160x48", 160, 48),
    ] {
        let drawn = frame(&state, columns, rows);
        let mut out = capture_header(label, columns, rows);
        for line in &drawn {
            out.push_str(&line.replace(&home, "/code/api"));
            out.push('\n');
        }
        write_shot(&format!("m5-ui-banner-conflict-{label}.txt"), &out);
    }
    drop(state);
    app.close();
}
