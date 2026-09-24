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

use graft::session::Clock;
use graft::update::{AppEvent, AppState, Command, Status, update};
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers, MouseButton};
use graft_git::state::InProgress;

use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame, frame_at};

/// The instant [`about_to_conflict_over`]'s first commit is stamped at, Unix
/// seconds; one minute per commit after it.
///
/// Without this the fixture was built at wall-clock time, so every commit
/// object got a different id on every run. The graph breaks a date tie with
/// the commit id (DESIGN.md §3.2), which is why `main` and `feature` swapped
/// lanes between regenerations of the banner captures
/// (m5-history-and-ghost-preview/21). After
/// `graft_fixtures::git::FIXED_DATE`, so these commits never appear to
/// predate the corpus history they are added to.
const CONFLICT_EPOCH: i64 = 1_609_459_200;

/// The instant the banner captures' relative dates are drawn against: a
/// minute after the newest commit any of them makes, so an age reads the
/// same forever instead of drifting with real time.
const CONFLICT_NOW: i64 = CONFLICT_EPOCH + 120 * 60;

/// Two branches that both rewrote the same two lines: merging `feature` into
/// `main` stops with `shared.rs` and `notes.md` unmerged.
///
/// Two files, not one, so `CONFLICTS (2)` and "2 files still conflicted" are
/// read off a real count rather than a hard-coded singular.
fn about_to_conflict() -> graft_fixtures::Scratch {
    about_to_conflict_over(0, 0)
}

/// The same repository with `extra` more files conflicted on top of
/// `shared.rs` and `notes.md`, named so they sort after both and leave
/// `notes.md` git's first unmerged path either way, over `history` commits
/// of ordinary work before the branch point.
///
/// The captures use both. A screen with four rows of content and thirteen
/// blanks cannot show what the banner's two rows cost, which is the
/// question the 80x24 capture is taken to answer (m5/20 item 6, the user's
/// own words): `history` is what fills the graph, `extra` what fills the
/// changes pane. Both are 0 for the behavioural tests, which pay a git
/// process per commit and are asserting counts, not filling screens.
fn about_to_conflict_over(extra: usize, history: usize) -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    let more: Vec<String> = (0..extra).map(|n| format!("src/step_{n:02}.rs")).collect();
    if extra > 0 {
        std::fs::create_dir_all(path.join("src")).expect("the scratch repo is writable");
    }

    let mut minute = 0;
    for n in 0..history {
        write(path, "changelog.md", &format!("release {n}\n"));
        minute += 1;
        commit(path, &format!("Ship release {n}"), minute);
    }

    write(path, "shared.rs", "fn main() {}\n");
    write(path, "notes.md", "the plan\n");
    for name in &more {
        write(path, name, "fn step() {}\n");
    }
    minute += 1;
    commit(path, "the shared files", minute);

    git(path, &["switch", "--quiet", "-c", "feature"]);
    write(path, "shared.rs", "fn main() { theirs() }\n");
    write(path, "notes.md", "their plan\n");
    for name in &more {
        write(path, name, "fn step() { theirs() }\n");
    }
    commit(path, "the feature's take", minute + 1);

    git(path, &["switch", "--quiet", "main"]);
    write(path, "shared.rs", "fn main() { ours() }\n");
    write(path, "notes.md", "our plan\n");
    for name in &more {
        write(path, name, "fn step() { ours() }\n");
    }
    commit(path, "main's take", minute + 2);
    scratch
}

fn write(repo: &Path, name: &str, body: &str) {
    std::fs::write(repo.join(name), body).expect("the scratch repo is writable");
}

/// One fixture commit, stamped `minute` minutes after [`CONFLICT_EPOCH`] so
/// both its id and its place in date order are the same on every run.
fn commit(repo: &Path, subject: &str, minute: i64) {
    git(repo, &["add", "--all"]);
    let when = format!("@{} +0000", CONFLICT_EPOCH + minute * 60);
    graft_fixtures::git::run_at(repo, &["commit", "-m", subject], &when)
        .unwrap_or_else(|why| panic!("committing {subject:?}: {why}"));
}

/// Opened, with the first rows and the refs on hand.
fn opened(app: &App, state: AppState) -> AppState {
    app.pump(state, "the first rows and the refs", |state| {
        state.refs.is_some()
            && state.working_tree.is_some()
            // The footer's commit-graph line waits on `git --version`.
            && state.system_git.is_some()
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

/// A merge that stopped, on the banner: the stop lands in the resolver
/// (m6-conflicts/18), and `Esc` once its opening read is in is the way back
/// to the banner these tests are about.
pub(crate) fn stopped_on_the_conflict(app: &App, state: AppState) -> AppState {
    let state = landed_in_the_resolver(app, state);
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none(), "Esc closes the landed resolver");
    assert!(state.banner_focus, "and hands the keyboard to the banner");
    state
}

/// Cursor onto `feature`, `m`, through the preview and the confirm box, and
/// wait for the merge to stop and the resolver it lands in to read its
/// first file. The state it hands back is a real conflict.
pub(crate) fn landed_in_the_resolver(app: &App, state: AppState) -> AppState {
    let state = merge_stopped(app, state);
    app.pump(state, "the landed resolver's first file", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.file.is_some() || open.whole_file.is_some())
    })
}

/// `about_to_conflict` plus `alpha.md`, which conflicts too but that
/// `rerere` resolves and stages on the merge: git's `Conflicts:` list names
/// it, so it heads the resolver's list as done, and `notes.md` — git's first
/// unmerged path — sits on row 1, not 0.
fn about_to_conflict_behind_a_done_file() -> graft_fixtures::Scratch {
    let scratch = about_to_conflict();
    let path = scratch.path();
    git(path, &["config", "rerere.enabled", "true"]);
    git(path, &["config", "rerere.autoUpdate", "true"]);
    git(path, &["switch", "--quiet", "feature"]);
    write(path, "alpha.md", "their alpha\n");
    commit(path, "the feature's alpha", 10);
    git(path, &["switch", "--quiet", "main"]);
    write(path, "alpha.md", "our alpha\n");
    commit(path, "main's alpha", 11);
    // Teach rerere `alpha.md`'s resolution: the merge stops (its failure is
    // the point, so its status is not checked), `rerere` records the
    // resolved file, and the merge is abandoned.
    let merge = std::process::Command::new("git")
        .args(["merge", "--quiet", "feature"])
        .current_dir(path)
        .output()
        .expect("git runs");
    assert!(!merge.status.success(), "the teaching merge conflicts");
    write(path, "alpha.md", "the agreed alpha\n");
    git(path, &["rerere"]);
    git(path, &["merge", "--abort"]);
    scratch
}

/// m6-conflicts/29 (review-18 Critical 1): after the stop, once the file's
/// read and the conflict set are in, the landed resolver's cursor is on
/// block 0 and the file list's cursor on the first conflicted file's row —
/// row 1 here, behind a done file, so a cursor left at 0 fails.
#[test]
fn the_landing_puts_both_cursors_on_the_first_conflicted_file() {
    let scratch = about_to_conflict_behind_a_done_file();
    let (app, state) = App::open(scratch.path());
    let state = landed_in_the_resolver(&app, state);
    let state = app.pump(state, "the conflict set", |state| {
        graft::update::resolver::listed_files(state).len() == 3
    });
    let first = graft::update::banner::first_conflicted(&state).expect("something is unmerged");
    let open = state.resolver.as_ref().expect("the stop landed");
    assert_eq!(open.path, first);
    assert_eq!(open.cursor_block, Some(0), "the cursor is on block 0");
    let listed: Vec<_> = graft::update::resolver::listed_files(&state)
        .into_iter()
        .map(|file| file.path)
        .collect();
    let at = listed
        .iter()
        .position(|path| path == &first)
        .expect("the first conflicted file is listed");
    assert_ne!(at, 0, "the fixture puts it off row 0: {listed:?}");
    assert_eq!(
        graft::update::resolver::file_cursor(&state),
        at,
        "{listed:?}"
    );
    drop(state);
    app.close();
}

/// Cursor onto `feature`, `m`, through the preview and the confirm box, and
/// wait for the merge to stop and the working tree to come back.
fn merge_stopped(app: &App, state: AppState) -> AppState {
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
    let state = app.pump(state, "the changes pane's file list", |state| {
        matches!(state.detail.files, graft::update::detail::Loading::Ready(_))
    });
    // And the selected file's diff. The body draws `reading\u{2026}` while
    // **either** the list or the diff is still in flight
    // (`view::detail::body_view`), and the list landing first is what put
    // `reading\u{2026}` into all three committed captures: a photograph of a
    // half-loaded screen, which is weak evidence for everything else on it
    // (m5/20 item 5).
    app.pump(state, "the conflicted file's diff", |state| {
        !matches!(
            state.detail.diff,
            graft::update::detail::Loading::Pending(_)
        )
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
fn esc_from_the_landing_shows_the_banner_and_lists_the_files() {
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
        "Esc from the landed resolver hands the banner the keyboard: {:?}",
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
        "the first conflicted path, in git's own order \u{2014} never drawn on the \
         banner, but it is what decides whether `M` and `e` have something \
         to open (`view::rows::banner_context`)"
    );

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
    // `is_quit` and quits, exactly as it would with no banner at all.
    //
    // The assertion used to be `should_quit() || confirm.is_some() ||
    // status != Ready`, and after a conflict the status is already `Sticky`
    // — so the third arm was true whatever `Ctrl+C` did, and a banner that
    // swallowed it left this test green (m5-09 review, Major 4). Naming the
    // outcome is what makes it able to fail.
    let control_c = KeyPress {
        code: KeyCode::Char('c'),
        modifiers: Modifiers::CONTROL,
    };
    let (after, _) = update(&state, AppEvent::Core(Event::Key(control_c)));
    assert!(
        after.should_quit(),
        "Ctrl+C must quit through the banner: {:?}",
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

/// m5/21 item 2: the body beside a `CONFLICTS` row drew **nothing**. The
/// read asked for was the worktree against the index (`git diff-files`), and
/// for an unmerged path the index holds three stages, so git answers with a
/// *combined* diff (`diff --cc`) that the unified parser does not read — 0
/// hunks, a blank pane beside a file the pane has just named.
///
/// The conflicted file, with its markers, against `HEAD` is a plain two-way
/// diff, and it is the question the user is actually asking.
#[test]
fn an_unmerged_file_draws_its_conflict_in_the_body() {
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);

    let state = app.key(&state, KeyCode::Tab);
    let state = open_changes_pane(&app, state);
    let state = crate::app_harness::cursor_to(&app, state, "notes.md");
    let state = app.pump(state, "the conflicted file's diff", |state| {
        matches!(state.detail.diff, graft::update::detail::Loading::Ready(_))
    });

    match &state.detail.diff {
        graft::update::detail::Loading::Ready(diff) => {
            assert!(
                !diff.hunks.is_empty(),
                "a conflicted file's diff came back with no hunks: {:?}",
                diff.status
            );
        }
        other => panic!("the conflicted file's diff never arrived: {other:?}"),
    }

    // And it reaches the drawn frame: the markers git left in the file are
    // what the body shows.
    let rows = frame(&state, 120, 30);
    assert!(
        rows.iter().any(|row| row.contains("<<<<<<<")),
        "the conflict markers are not on the frame:\n{}",
        rows.join("\n")
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
    // m6-conflicts/12: a stop is state, not a status. The bottom row says
    // whose operation it is before the keys that resolve and end it.
    assert_eq!(state.status, Status::Ready);
    stop_line_reads(
        &state,
        "merge still in progress",
        &["\u{23ce} resolve", "c continue", "a abort"],
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

/// `Shape::StoppedRebase` as a killed Graft leaves it: git's rebase state
/// on disk and the journal's `Pending` line for the rebase that stopped,
/// with no outcome after it. `head_before` is the branch the rebase started
/// on, which is what `graft_ops`' launch read matches a live entry by.
fn killed_mid_rebase() -> graft_fixtures::Scratch {
    use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind};
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::StoppedRebase);
    let path = scratch.path();
    let mut journal =
        graft_ops::journal::Journal::open(&path.join(".git/graft/undo-journal.jsonl"))
            .expect("the journal file can be created");
    journal
        .append_pending(&JournalEntry {
            id: EntryId::new(CONFLICT_EPOCH * 1000, [7; 10]),
            at_unix_ms: CONFLICT_EPOCH * 1000,
            kind: OperationKind::Rebase,
            description: "rebase feature onto main".to_owned(),
            status: EntryStatus::Pending,
            touches: graft_core::journal::Touches::default(),
            head_before: HeadRef::Branch("refs/heads/feature".to_owned()),
            head_after: None,
            refs_before: std::collections::BTreeMap::new(),
            refs_after: std::collections::BTreeMap::new(),
            index_before: None,
            index_after: None,
            worktree_before: None,
            worktree_after: None,
            backup_ref: None,
            pins: Vec::new(),
            commands: Vec::new(),
            reverses: None,
        })
        .expect("the pending line is written");
    scratch
}

/// The graph screen's bottom row at 120x40 (the changes pane opens below
/// it) names the stop, then the keys it offers.
fn stop_line_reads(state: &AppState, reason: &str, keys: &[&str]) {
    let drawn = frame(state, 120, 40);
    let bottom = drawn
        .iter()
        .find(|line| line.contains(reason))
        .unwrap_or_else(|| panic!("the stop is named:\n{}", drawn.join("\n")));
    let at = bottom.find(reason).expect("the row was found by it");
    for key in keys {
        let key_at = bottom
            .find(key)
            .unwrap_or_else(|| panic!("{key} is offered: {bottom:?}"));
        assert!(key_at > at, "the reason leads, then {key}: {bottom:?}");
    }
}

/// Both launch reads, and the changes pane's list the takeover asked for.
fn landed(app: &App, state: AppState) -> AppState {
    let state = launched(app, state);
    app.pump(state, "the changes pane the relaunch opened", |state| {
        matches!(state.detail.files, graft::update::detail::Loading::Ready(_))
    })
}

/// The row under the changes pane's cursor, and its section heading.
fn cursor_row(state: &AppState) -> (String, Option<graft_ui::detail::WorkingTreeSection>) {
    let graft::update::detail::Loading::Ready(files) = &state.detail.files else {
        panic!("the file list has not arrived: {:?}", state.detail.files);
    };
    (
        files.rows[files.cursor].name.as_str().to_owned(),
        files.selected_section(),
    )
}

/// **Decision 2a.** A relaunch into a stopped rebase lands one `⏎` from the
/// resolver: the banner has the keyboard, the changes pane is open on
/// CONFLICTS with the cursor on git's first conflicted path, and the line
/// names `⏎ resolve` beside the two keys that end the rebase. Nothing is
/// opened or written by the launch itself — `⏎` is the user's.
#[test]
fn relaunch_lands_on_the_conflict() {
    let scratch = killed_mid_rebase();
    let repo = scratch.path();

    let (app, state) = App::open(repo);
    let state = landed(&app, state);

    assert_eq!(state.live_entries.len(), 1, "the pending rebase is live");
    assert!(state.banner_focus, "{:?}", state.status);
    assert_eq!(state.status, Status::Ready);
    stop_line_reads(
        &state,
        "rebase still in progress",
        &["\u{23ce} resolve", "c continue", "a abort"],
    );
    assert!(
        state.on_changes_pane(),
        "the pane is open on the working tree"
    );
    let first =
        graft::update::banner::first_conflicted(&state).expect("the rebase has conflicted files");
    // git's order is by path, so the fixture's first is not `both.txt`
    // (the ticket's guess) but the `UA` file that sorts ahead of it.
    assert_eq!(first, Path::new("added-by-them.txt"));
    let (row, section) = cursor_row(&state);
    assert_eq!(
        section,
        Some(graft_ui::detail::WorkingTreeSection::Unmerged),
        "the cursor is under CONFLICTS"
    );
    assert_eq!(Path::new(&row), first, "on git's first conflicted path");
    assert!(state.resolver.is_none(), "the launch opened nothing itself");

    let commands = commands_for(&state, KeyPress::plain(KeyCode::Enter));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(_, path) if *path == first)),
        "one \u{23ce} opens the resolver on it: {commands:?}"
    );
    drop(state);
    app.close();
}

/// The same relaunch with every file resolved and staged before the kill:
/// nothing to resolve, so no `⏎ resolve` on the line, and the pane lands on
/// STAGED — what `c` would commit.
#[test]
fn relaunch_with_nothing_conflicted() {
    let scratch = killed_mid_rebase();
    let repo = scratch.path();
    git(repo, &["add", "-A"]);

    let (app, state) = App::open(repo);
    let state = landed(&app, state);

    assert!(state.banner_focus, "{:?}", state.status);
    assert_eq!(state.status, Status::Ready);
    stop_line_reads(
        &state,
        "rebase still in progress",
        &["c continue", "a abort"],
    );
    let drawn = frame(&state, 120, 40);
    let bottom = drawn
        .iter()
        .find(|line| line.contains("still in progress"))
        .expect("the stop row is drawn");
    assert!(!bottom.contains("resolve"), "nothing to resolve: {bottom}");
    assert!(state.on_changes_pane());
    assert_eq!(
        cursor_row(&state).1,
        Some(graft_ui::detail::WorkingTreeSection::Staged),
        "the cursor is under STAGED"
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
    // What the line reads instead: the operation's own state, the stop
    // line `banner::stop_reason` words (m6-conflicts/12). A state, not an
    // error (CLAUDE.md, "a status field must never read as an error").
    let drawn = frame(&state, 120, 40);
    assert!(
        drawn
            .iter()
            .any(|line| line.contains("merge stopped \u{b7} ")),
        "the live merge names itself instead:\n{}",
        drawn.join("\n")
    );

    drop(state);
    app.close();
}

// ---------------------------------------------------------------------------
// The banner on the drawn frame, at every width (m5/16, review C1).
// ---------------------------------------------------------------------------

/// The state the three captures are taken in: git stopped on the conflict,
/// the changes pane open on it, and the keyboard back on the banner.
fn stopped_with_the_pane_open(app: &App, state: AppState) -> AppState {
    let state = stopped_on_the_conflict(app, state);
    let state = app.key(&state, KeyCode::Tab);
    let state = open_changes_pane(app, state);
    tab_back_to_the_banner(app, state)
}

/// **The Critical.** A focused banner is modal — it owns the keyboard — so
/// it must be **on the frame**, with its keys, at every width. Asserted on
/// the drawn buffer, not on `banner_view`: the view model was right all
/// along, and the banner was still nowhere at 80 columns, because below 100
/// the layout goes to tabs, the pane takes the screen and the graph header
/// went with it (`docs/shots/m5-ui-banner-conflict-80x24.txt` on `main`
/// showed no banner at all, with `c`/`s`/`a` live and invisible).
#[test]
fn the_banner_and_its_keys_are_on_the_frame_at_every_width() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = stopped_with_the_pane_open(&app, state);
    assert!(state.banner_focus, "the banner has the keyboard");

    for (columns, rows) in [(80u16, 24u16), (120, 40), (160, 48)] {
        let drawn = frame(&state, columns, rows);
        let at = format!("at {columns}x{rows}:\n{}", drawn.join("\n"));
        assert!(
            drawn.iter().any(|line| line.contains("MERGING")),
            "the banner names the operation {at}"
        );
        let banner = drawn
            .iter()
            .find(|line| line.contains("MERGING"))
            .unwrap_or_else(|| panic!("no banner {at}"));
        assert!(
            !banner.contains("notes.md") && !banner.contains("conflict"),
            "the banner names the state; the pane names the files {banner:?} {at}"
        );
        assert!(
            drawn.iter().any(|line| line.contains("CONFLICTS (2)")),
            "and the pane does name them {at}"
        );
        assert!(
            drawn.iter().any(|line| line.contains("notes.md")),
            "including the one git stopped on {at}"
        );
        let hints = drawn
            .iter()
            .find(|line| line.contains("c continue"))
            .unwrap_or_else(|| panic!("no line teaches `c continue` {at}"));
        assert!(hints.contains("a abort"), "{hints:?} {at}");
        assert!(hints.contains("⇥ graph"), "the way out {hints:?} {at}");
        assert!(
            hints.contains("M mergetool") && hints.contains("e edit"),
            "two files are conflicted, so both apply {hints:?} {at}"
        );
    }
    drop(state);
    app.close();
}

/// One wording, at every width \u2014 read off the **drawn** frame, which is
/// where the user read the defect.
///
/// The three captures of one repository in one state spelled the banner
/// two ways: `conflict in notes.md` at 120, `conflict notes.md` at 80 and
/// 160 (m5/20 item 3). The cause was a `width >= 100` tier inside
/// `banner_line`, fed the banner's own region \u2014 120 cells on a 120-column
/// screen with the detail pane below it, about 92 on a 160-column one with
/// the pane beside it. So the *wider* terminal got the terser spelling.
/// The tier is gone and the file with it, and the wording is now one
/// string with no width in it at all.
///
/// Anchored to the frame rather than to `banner_line`, which now takes no
/// width and would agree with itself for free.
#[test]
fn the_banner_reads_the_same_at_every_width() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = stopped_with_the_pane_open(&app, state);

    let mut wordings = Vec::new();
    for (columns, rows) in [(80u16, 24u16), (120, 40), (160, 48)] {
        let drawn = frame(&state, columns, rows);
        let line = drawn
            .iter()
            .find(|line| line.contains("MERGING"))
            .unwrap_or_else(|| panic!("no banner at {columns}x{rows}:\n{}", drawn.join("\n")))
            .clone();
        let at = line.find("MERGING").expect("just matched");
        // The banner's own run of text: up to the gap that separates it
        // from whatever else shares the row (the pane, at 160).
        let tail = &line[at..];
        let wording = tail
            .split("  ")
            .next()
            .unwrap_or(tail)
            .trim_end()
            .to_owned();
        wordings.push((columns, wording));
    }
    let first = wordings[0].1.clone();
    assert_eq!(first, "MERGING", "{wordings:?}");
    for (columns, wording) in &wordings {
        assert_eq!(
            wording, &first,
            "the banner is worded differently at {columns} columns: {wordings:?}"
        );
    }
    drop(state);
    app.close();
}

/// The hint bar counts from **its own row**, not from the screen.
///
/// At 160x48 the detail pane sits beside the graph and takes about 42% of
/// the screen, so the bar has roughly 92 cells \u2014 and the old `width_tier`
/// read 92 as "narrow" and capped the list at three, while a 120-column
/// screen with the pane below it kept the full 120 cells and drew six. The
/// bigger terminal taught fewer keys (ADR-0039, 2026-09-21 amendment).
///
/// Anchored to the drawn bar at a real screen size with a real pane open.
/// Ticket 19's property test sweeps `hints_for`'s own `columns` argument,
/// and `hints_for` was always monotone in that; the defect was in what the
/// bar's room actually was.
#[test]
fn the_hint_bar_fills_the_row_it_has_at_a_hundred_and_sixty_columns() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    // The banner is modal and draws its own short list, so this asks the
    // graph's own bar: give the keyboard back first.
    let state = stopped_with_the_pane_open(&app, state);
    let state = app.key(&state, KeyCode::Tab);

    let drawn = frame(&state, 160, 48);
    let bar = drawn
        .iter()
        .rev()
        .find(|line| line.contains(" more"))
        .unwrap_or_else(|| panic!("no hint bar at 160x48:\n{}", drawn.join("\n")))
        .clone();
    let hints = bar
        .split("  ")
        .filter(|part| !part.trim().is_empty())
        .count();
    assert!(
        hints > 3,
        "160 columns with a pane open leaves the bar ~92 cells, which fits \
         more than three hints: {bar:?}"
    );
    drop(state);
    app.close();
}

/// The banner's line is **outside** the pane in the tab layout: the pane's
/// first row is the tab strip's, and the banner is above it. Drawn inside,
/// it would travel with whichever tab is shown — which is how it came to be
/// invisible with the changes pane open.
#[test]
fn below_a_hundred_columns_the_banner_draws_above_the_tab_line() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = stopped_with_the_pane_open(&app, state);

    let drawn = frame(&state, 80, 24);
    assert!(
        drawn[0].contains("MERGING"),
        "the top row is the banner's: {drawn:?}"
    );
    assert!(
        drawn[1].contains("[Detail]"),
        "the tab line sits under it: {drawn:?}"
    );
    assert!(
        drawn[23].contains("c continue"),
        "and the banner's keys are the bottom row: {drawn:?}"
    );
    drop(state);
    app.close();
}

/// What the banner costs below 100 columns, stated as a number because the
/// user judges it by eye: **two** rows while it has the keyboard, **one**
/// while it is only drawn, **none** with no operation live.
#[test]
fn the_tab_layout_banner_costs_one_row_drawn_and_two_focused() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let focused = stopped_with_the_pane_open(&app, state);

    let mut unfocused = focused.clone();
    unfocused.banner_focus = false;
    let mut no_operation = unfocused.clone();
    no_operation.in_progress = None;

    let rows_of = |state: &AppState| {
        frame(state, 80, 24)
            .iter()
            .filter(|line| line.contains("unmerged") || line.contains("CONFLICTS"))
            .count()
    };
    // The pane's own top row, wherever it starts.
    let pane_top = |state: &AppState| {
        frame(state, 80, 24)
            .iter()
            .position(|line| line.contains("UNCOMMITTED"))
            .expect("the pane draws its title")
    };
    assert_eq!(pane_top(&no_operation), 1, "tab strip, then the pane");
    assert_eq!(pane_top(&unfocused), 2, "one row: the banner");
    assert_eq!(pane_top(&focused), 2, "the second row is taken off the end");
    assert!(
        rows_of(&focused) > 0 && rows_of(&no_operation) > 0,
        "the pane still draws its conflicts either way"
    );

    // The row is taken off the *end*, so the pane is a row shorter while
    // focused - the half of the claim the top row cannot show (m5/14-22
    // review, Minor 8). Read at 12 rows, where the diff outlives the pane:
    // at 24 it stops before the bottom either way, so the lost row is blank
    // and a frame proves nothing.
    let pane_bottom = |state: &AppState| {
        frame(state, 80, 12)
            .iter()
            .rposition(|line| !line.trim().is_empty() && !line.contains("c continue"))
            .expect("the pane draws something")
    };
    assert_eq!(
        pane_bottom(&focused),
        pane_bottom(&unfocused) - 1,
        "the focused banner's second row comes out of the pane, not the terminal"
    );
    drop(focused);
    app.close();
}

/// ADR-0039's doorway still rides every other context at every width. The
/// focused banner is the one place it is not drawn, and deliberately: it is
/// modal, so `.` opens nothing while it has the keyboard, and the doorway's
/// promise is that what was dropped is still reachable.
#[test]
fn the_doorway_survives_everywhere_the_banner_does_not_own_the_keyboard() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = stopped_with_the_pane_open(&app, state);

    for (columns, rows) in [(80u16, 24u16), (120, 40), (160, 48)] {
        let drawn = frame(&state, columns, rows);
        assert!(
            !drawn.iter().any(|line| line.contains(". more")),
            "the modal banner must not offer a key it swallows, at {columns}x{rows}:\n{}",
            drawn.join("\n")
        );
    }

    // `Tab` hands the keyboard back, and with it the doorway. Present, not
    // last: the keyboard goes to the *graph*, whose bar has a tail of bare
    // keys, and since m5/28 the doorway leads that tail rather than
    // trailing the row (ADR-0039, one rule for both layouts). Before m5/31
    // this bar was the changes pane's — which the graph having focus makes
    // wrong, since `changes_pane_write_key` answers none of its keys then.
    let left = app.key(&state, KeyCode::Tab);
    for (columns, rows) in [(120u16, 40u16), (160, 48)] {
        let drawn = frame(&left, columns, rows);
        assert!(
            drawn.iter().any(|line| line.contains(". more")),
            "the doorway is back at {columns}x{rows}:\n{}",
            drawn.join("\n")
        );
    }
    drop(left);
    app.close();
}

// ---------------------------------------------------------------------------
// Piece 9: the captures. Not an assertion — the user judges these by eye
// (CLAUDE.md rule 6).
// ---------------------------------------------------------------------------

/// The width-specific bullets in [`capture_header`]: only one side of the
/// narrow tier's tab strip is on screen under 100 columns, and what a
/// stopped merge costs is not the same on the two of them.
fn cost_bullets(conflicts: usize, graph_tab: bool) -> String {
    if graph_tab {
        "#  - **What it costs.** Under 100 columns only one side of the tab\n\
         #    strip is on screen and this frame stands on the graph, with 30\n\
         #    commits of ordinary work behind it: history runs from the first\n\
         #    row to the last with nothing blank. The banner shares the header\n\
         #    row with the repository path, and its keys take over the hint bar\n\
         #    that was already there \u{2014} so on this tab a stopped merge costs\n\
         #    **no** rows of history at all. The row of its own is the `Detail`\n\
         #    tab's case, where the `Graph  [Detail]` strip needs a line of its\n\
         #    own anyway; that tab at 24 rows is a six-row file list over one\n\
         #    file's conflict, which shows a screen rather than a cost.\n"
    } else {
        "#  - At this width the pane and the graph are both on screen, so the\n\
         #    banner shares the header row rather than taking one of its own.\n\
         #  - The changes pane opens with `CONFLICTS ({conflicts})`, above `STAGED`\n\
         #    and `UNSTAGED`, and each conflicted file appears **once**: there\n\
         #    is one thing to do to it, not two.\n\
         #  - **No `reading\u{2026}`.** The capture waits for the selected file's\n\
         #    diff as well as the file list; the earlier pass photographed a\n\
         #    half-loaded pane.\n"
    }
    .replace("{conflicts}", &conflicts.to_string())
}

/// Where this capture came from: the passes that changed it, and the
/// fixture facts a reader needs before comparing it with the last one.
/// Its own function because [`capture_header`] is at its line limit.
fn provenance(conflicts: usize) -> String {
    "# Re-captured for m5/23, which pinned this fixture's clock: every\n\
# commit is stamped a fixed minute after 2021-01-01 and the frame is\n\
# drawn against a fixed instant, so the object ids, the ages and the\n\
# lane order of `main` and `feature` are the same on every run. They\n\
# all differ from the last pass, once, as a consequence.\n\
#\n\
# Captured before that for m5/21, which made the working-tree row\n\
# count unmerged paths and gave the changes pane a body for a\n\
# conflicted file (it is read against `HEAD` now; against the\n\
# index git answers with a combined diff and the body drew\n\
# nothing).\n\
#\n\
# Captured first for m5/20, which dropped the `\u{26a0}` and the filename from\n\
# the banner, settled the pane before the shutter, put 30 commits of\n\
# ordinary work and {conflicts} conflicted files behind the frame so the\n\
# banner's row cost lands on content rather than on blank space, and\n\
# made the hint bar count from its own row rather than from width\n\
# tiers (so the bars here are longer than on the last pass). What is\n\
# **not** proven here: colour (rule 5), and that the banner row reads\n\
# well by eye \u{2014}\n\
# that is yours to judge (rule 6).\n"
        .to_owned()
        .replace("{conflicts}", &conflicts.to_string())
}

/// The caption on one banner capture.
///
/// `conflicts` is the real count, so the caption can never claim a number
/// the frame does not show. `graph_tab` says which side of the narrow
/// tier's tab strip this frame is standing on, because only one of them is
/// on screen under 100 columns and the two are not the same picture.
fn capture_header(
    label: &str,
    columns: u16,
    rows: u16,
    conflicts: usize,
    graph_tab: bool,
) -> String {
    let cost = cost_bullets(conflicts, graph_tab);
    let provenance = provenance(conflicts);
    format!(
        "# m5-ui-banner-conflict-{label}.txt: {columns}x{rows}, drawn by `view::view`\n\
             # from a real AppState \u{2014} a real repository whose `main` and `feature`\n\
             # both rewrote the same {conflicts} files, opened over the real job pool, the\n\
             # merge launched with `m` through the preview and the confirm box, and\n\
             # git really stopped on the conflict (crates/graft/tests/it/\n\
             # conflict_interim.rs, capture_banner_for_docs_shots, asserted). The\n\
             # app was resized to {columns}x{rows} before the frame was drawn, so the\n\
             # layout it chose is the layout for this screen and not for another\n\
             # one. Not a pty capture: the terminal is ratatui's TestBackend, so\n\
             # this is a character grid with no colour attributes in it \u{2014}\n\
             # CLAUDE.md rule 5, colour ships unverified. The scratch repository's\n\
             # temporary path is set to `/code/api` **before** the frame is drawn,\n\
             # so the header is laid out against the width a real checkout takes;\n\
             # nothing else is touched.\n\
             #\n\
             # What right looks like:\n\
             #  - A banner above the panes reading `MERGING` \u{2014} the state the\n\
             #    repository is in, said once, at the top, and **nothing else**. No\n\
             #    filename: the changes pane lists every conflicted file, and\n\
             #    saying it twice on a 24-row screen spends a row on nothing.\n\
             #    Worded the\n\
             #    same at all three widths.\n\
             #  - No `\u{26a0}`. A stopped merge is a state to finish, not a failure;\n\
             #    the glyph keeps its force for the Mistake Radar's real findings.\n\
             {cost}\
             #  - The footer teaches the banner's own keys \u{2014} `c continue`,\n\
             #    `a abort`, `M mergetool`, `e edit`, `\u{21e5} graph` \u{2014} and nothing else.\n\
             #    No `<> resize` and no `. more`: the banner is modal and swallows\n\
             #    both.\n\
             #  - The banner is drawn focused (colour, so unverified here), and the\n\
             #    footer says nothing false about the merge: no `an operation was\n\
             #    interrupted` line.\n\
             #  - **The working-tree row counts the conflict**: it reads\n\
             #    `Uncommitted changes ({conflicts} unmerged)`, not the `()` it read on\n\
             #    the last pass. An unmerged path is in none of staged,\n\
             #    unstaged and untracked, so the row drew and then counted\n\
             #    nothing (m5/21). Unmerged leads the count when other\n\
             #    buckets are there too.\n\
             #  - Nothing on the screen reads as an error.\n\
             #\n\
             {provenance}\
             #\n\n"
    )
}

/// Writes the banner captures for `docs/shots/`.
///
/// Regenerate on purpose: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --test it conflict_interim::capture_banner_for_docs_shots`.
#[test]
fn capture_banner_for_docs_shots() {
    // Enough behind the frame that the banner's two rows land on content:
    // 30 commits of ordinary work for the graph and 16 conflicted files for
    // the changes pane. The capture is taken to show what those rows cost,
    // and it cannot show a cost against blank space (m5/20 item 6).
    let conflicts = 16;
    let scratch = about_to_conflict_over(conflicts - 2, 30);
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);

    // Two screens, not one. Under 100 columns the tab strip shows one side
    // at a time, and the narrow capture stands on the **graph**: the
    // changes pane's file list is capped at 40% of the pane, so the Detail
    // tab at 24 rows is six rows of list over a body showing one file's
    // conflict — a fine screen, but not one that can show what the banner's
    // rows cost, which is what item 6 was filed about. The graph fills.
    //
    // Cloned before the pane is opened, because the banner keeps the
    // keyboard through all of it (`detail.focus` is what the tab strip
    // reads, and it is still `Graph` here).
    let on_the_graph = state.clone();
    // The focused banner is modal: `Tab` out of it first, or the `k`s that
    // walk to the changes row are the banner's to swallow.
    let state = app.key(&state, KeyCode::Tab);
    let state = open_changes_pane(&app, state);
    let state = tab_back_to_the_banner(&app, state);
    let home = repo.display().to_string();

    for (label, columns, rows, graph_tab) in [
        ("80x24", 80u16, 24u16, true),
        ("120x40", 120, 40, false),
        ("160x48", 160, 48, false),
    ] {
        let shot = if graph_tab { &on_the_graph } else { &state };
        // The path is shortened **before** drawing, not after. The scratch
        // repository lives under a ~50-character temporary path, and the
        // header lays itself out against it: replacing the text afterwards
        // produced a capture that showed `/code/api` beside a banner cut to
        // `MERG…`, and the reader had no way to know the width had been
        // spent on a path they could not see. Shortening the state makes
        // the capture the frame a user with `~/code/api` actually gets
        // (m5/16). Done to a copy, per shot, so the state the app is still
        // driving keeps the path git is reachable by.
        let mut shot = shot.clone();
        if let Some(facts) = shot.repository.as_mut() {
            facts.path = std::path::PathBuf::from("/code/api");
        }
        // Resized before drawing: the layout, the graph's row budget and
        // the hint bar's room all come off `state.size`, so a state sized
        // for one screen photographed at another is a picture of neither.
        let sized = app.resize(&shot, columns, rows);
        let drawn = frame_at(&sized, columns, rows, Some(Clock::Fixed(CONFLICT_NOW)));
        let body = drawn.join("\n");
        assert!(
            body.contains("MERGING"),
            "every banner capture is of a stopped merge: {label}"
        );
        if graph_tab {
            assert!(
                drawn.iter().all(|line| !line.trim().is_empty()),
                "a frame with blank rows in it cannot show what a row costs \
                 (m5/20 item 6): {label}\n{body}"
            );
            assert!(
                drawn[0].contains("/code/api") && drawn[0].contains("MERGING"),
                "on the graph tab the banner shares the header row: {label}\n{body}"
            );
        } else {
            assert!(
                body.contains(&format!("CONFLICTS ({conflicts})")),
                "the wide captures show the changes pane's own count: {label}"
            );
            assert!(
                !body.contains("reading\u{2026}"),
                "the shutter waits for the pane to settle (m5/20 item 5): {label}"
            );
        }
        let mut out = capture_header(label, columns, rows, conflicts, graph_tab);
        for line in &drawn {
            out.push_str(&line.replace(&home, "/code/api"));
            out.push('\n');
        }
        check_shot(&format!("m5-ui-banner-conflict-{label}.txt"), &out);
    }
    drop(state);
    drop(on_the_graph);
    app.close();
}

/// **Review M2, confirmed by running it.** On the *Graph* tab below 100
/// columns the operation banner drew **twice**, and so did its keys.
///
/// `split_for_detail` draws the tab banner for `Tab { shown }` whichever
/// side is shown, and the graph screen carries the same banner on its own
/// header row (`view::rows` sets `banner_op` unconditionally) and feeds it
/// to the hint bar. With the Detail tab shown the graph is not drawn and
/// nothing doubles, which is why every existing case missed it:
/// `the_tab_layout_banner_costs_one_row_drawn_and_two_focused` locates the
/// pane by `"UNCOMMITTED"`, i.e. the Detail tab only.
///
/// The fix is the one the review proposed: the tab banner is the *fallback*
/// for a frame with no graph header on it, so it draws only when the graph
/// is not on screen.
#[test]
fn the_banner_and_its_keys_draw_once_on_the_graph_tab() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let mut on_the_graph_tab = stopped_with_the_pane_open(&app, state);
    on_the_graph_tab.detail.focus = graft_ui::detail::DetailFocus::Graph;

    for focused in [false, true] {
        let mut state = on_the_graph_tab.clone();
        state.banner_focus = focused;
        let drawn = frame(&state, 80, 24);
        let at = format!("focused={focused}, 80x24:\n{}", drawn.join("\n"));
        assert!(
            drawn.iter().any(|line| line.contains("[Graph]")),
            "the graph tab is the one shown {at}"
        );
        let banners = drawn.iter().filter(|line| line.contains("MERGING")).count();
        assert_eq!(banners, 1, "the banner draws once {at}");
        let key_rows = drawn
            .iter()
            .filter(|line| line.contains("c continue"))
            .count();
        assert!(key_rows <= 1, "its keys are listed once {at}");
        if focused {
            assert_eq!(key_rows, 1, "and a focused banner still shows them {at}");
        }
    }
    drop(on_the_graph_tab);
    app.close();
}

/// **Review M1, on the screen.** The rows the tab layout spends — the
/// `[Graph] Detail` strip, and the operation banner's row when one is drawn
/// above it — were unknown to `update::geometry` and to
/// `AppState::graph_rows`, so below 100 columns with a merge stopped every
/// click landed on a row above the one the user was pointing at.
///
/// Driven through the real mouse event, not through the geometry: what is
/// being asserted is that the commit the click picks is the commit whose
/// subject is drawn on that screen row. The sweep
/// (`update::tests::the_geometry_mirror_matches_the_rects_the_frame_hands_out`)
/// is the class test; this is the anchor that ties it to pixels.
#[test]
fn a_click_below_a_hundred_columns_picks_the_row_it_landed_on() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let mut state = stopped_with_the_pane_open(&app, state);
    state.detail.focus = graft_ui::detail::DetailFocus::Graph;
    state.size = graft::update::Size {
        columns: 80,
        rows: 24,
    };

    for banner_focus in [false, true] {
        let mut on_screen = state.clone();
        on_screen.banner_focus = banner_focus;
        let drawn = frame(&on_screen, 80, 24);
        let at = format!("banner_focus={banner_focus}:\n{}", drawn.join("\n"));
        for subject in ["main's take", "the shared files", "Add a readme"] {
            let row = drawn
                .iter()
                .position(|line| line.contains(subject))
                .unwrap_or_else(|| panic!("`{subject}` is on the frame {at}"));
            let row = u16::try_from(row).expect("24 rows fit a u16");
            let clicked = app.mouse_down(&on_screen, MouseButton::Left, 2, row);
            let picked = clicked
                .history
                .as_ref()
                .and_then(graft::history::History::selected)
                .map(|found| String::from_utf8_lossy(found.commit.subject()).into_owned())
                .unwrap_or_default();
            assert_eq!(
                picked, subject,
                "a click on screen row {row} picked `{picked}` {at}"
            );
        }
    }
    drop(state);
    app.close();
}

/// The other half of review M1's click: **below 100 columns with the Detail
/// tab shown the graph is not on screen at all**, so no click can name a
/// graph row. Before the guard in `geometry::graph_row_at`, clicking the
/// file `notes.md` moved the hidden graph's selection to another commit -
/// nothing on screen said so, and the pane the user was reading is what
/// changed next.
#[test]
fn a_click_on_the_detail_tab_does_not_move_the_hidden_graph() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let mut state = stopped_with_the_pane_open(&app, state);
    state.size = graft::update::Size {
        columns: 80,
        rows: 24,
    };

    let selected = |state: &AppState| {
        state
            .history
            .as_ref()
            .and_then(graft::history::History::selected)
            .map(|found| String::from_utf8_lossy(found.commit.subject()).into_owned())
            .unwrap_or_default()
    };

    for banner_focus in [false, true] {
        let mut on_screen = state.clone();
        on_screen.banner_focus = banner_focus;
        let drawn = frame(&on_screen, 80, 24);
        let at = format!("banner_focus={banner_focus}:\n{}", drawn.join("\n"));
        let row = drawn
            .iter()
            .position(|line| line.contains("notes.md"))
            .expect("the conflicted file is on the frame");
        let row = u16::try_from(row).expect("24 rows fit a u16");
        let clicked = app.mouse_down(&on_screen, MouseButton::Left, 4, row);
        assert_eq!(
            selected(&clicked),
            selected(&on_screen),
            "a click on a file name selected another commit {at}"
        );
        assert!(
            !clicked.on_working_tree || on_screen.on_working_tree,
            "nor the uncommitted row {at}"
        );
    }
    drop(state);
    app.close();
}

mod landing_shots;
