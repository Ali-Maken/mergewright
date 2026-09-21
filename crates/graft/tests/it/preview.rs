//! Ghost Preview from a real repository, through the real app
//! (m5-history-and-ghost-preview/08).
//!
//! `update::preview`'s unit tests settle what each key *means* over scripted
//! state. Nothing there opens a repository, so nothing there can prove that
//! `r` on a real fork reaches `graft_ops::preview::plan`, that the plan comes
//! back with ghosts in it, or that the graph pane actually **draws** them.
//! That is what these do: real `git`, real job pool, real keys, real frame,
//! read off the drawn buffer.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::config::Loaded;
use graft::session::{Clock, Session};
use graft::update::AppState;
use graft_core::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::app_harness::{App, git, git_as_graft};
use crate::hint_bar::capabilities;

/// [`forked`]'s first commit, Unix seconds; one minute per commit after it.
///
/// Fixed on purpose. `git` dates at one-second resolution, so five commits
/// made in a burst all carry the same second; the graph breaks a date tie
/// with the commit id (DESIGN.md §3.2, `graft_git::history::node::Key`), and
/// a wall clock puts a different id on every run — which is why this
/// fixture's rows, and the captures taken from them, came out in a different
/// order each time (m5-history-and-ghost-preview/13). The instant is after
/// `graft_fixtures::git::FIXED_DATE`, so these commits never appear to
/// predate the corpus history they are added to.
const FORK_EPOCH: i64 = 1_609_459_200;

/// The instant the fixture's relative dates are drawn against: a minute
/// after its newest commit, so a capture of it reads the same age forever
/// instead of drifting with real time (`session::Clock::Fixed`).
const FORK_NOW: i64 = FORK_EPOCH + 6 * 60;

/// A fork: `main` with three commits, `feature` branched off the first and
/// carrying two of its own, `feature` checked out. Rebasing `feature` onto
/// `main` replays two commits, which is a picture with ghosts in it.
fn forked() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    dated_commit(path, "base.txt", "base\n", "add the base", 1);
    git(path, &["branch", "feature"]);
    dated_commit(path, "main-one.txt", "one\n", "main moves on", 2);
    dated_commit(path, "main-two.txt", "two\n", "main moves again", 3);
    git(path, &["switch", "feature"]);
    dated_commit(path, "work-one.txt", "one\n", "start the feature", 4);
    dated_commit(path, "work-two.txt", "two\n", "finish the feature", 5);
    scratch
}

/// One commit of [`forked`], stamped `minute` minutes after [`FORK_EPOCH`]
/// so both its id and its place in date order are the same on every run.
fn dated_commit(repo: &Path, name: &str, body: &str, subject: &str, minute: i64) {
    std::fs::write(repo.join(name), body).expect("the scratch repo is writable");
    git(repo, &["add", name]);
    let when = format!("@{} +0000", FORK_EPOCH + minute * 60);
    graft_fixtures::git::run_at(repo, &["commit", "-m", subject], &when)
        .unwrap_or_else(|why| panic!("committing {subject:?}: {why}"));
}

fn commit(repo: &Path, name: &str, body: &str, subject: &str) {
    std::fs::write(repo.join(name), body).expect("the scratch repo is writable");
    git(repo, &["add", name]);
    git_as_graft(repo, &["commit", "-m", subject]);
}

/// Opened, with the first rows and the refs on hand — everything `r` reads.
fn opened(app: &App, state: AppState) -> AppState {
    app.pump(state, "the first rows and the refs", |state| {
        state.refs.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    })
}

/// Presses `j` until the cursor's row carries `main`'s badge, then `r`, then
/// waits for the plan.
fn preview_of_a_rebase(app: &App, state: AppState) -> AppState {
    let mut state = opened(app, state);
    let mut guard = 0;
    loop {
        assert!(
            guard < 20,
            "never found main's row: {:?}",
            rows_drawn(&state)
        );
        if on_main(&state) {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
        guard += 1;
    }
    let state = app.key(&state, KeyCode::Char('r'));
    assert!(
        state.preview.is_some(),
        "`r` opens a preview: {:?}",
        state.status
    );
    app.pump(state, "the preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    })
}

/// Is the cursor on a row carrying the `main` branch badge?
fn on_main(state: &AppState) -> bool {
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
    refs.on(row.commit.id)
        .iter()
        .any(|label| matches!(label, graft::refs::RefLabel::Branch { name, .. } if name == b"main"))
}

fn rows_drawn(state: &AppState) -> Vec<String> {
    state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .map(|window| {
            window
                .rows
                .iter()
                .map(|row| String::from_utf8_lossy(row.commit.subject()).into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// Every row of the frame `view::view` draws for `state` at this size.
fn frame(state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    frame_at(state, columns, rows, None)
}

/// [`frame`], with the instant its relative dates are drawn against. `None`
/// is the wall clock; a capture that must not age passes
/// `Some(Clock::Fixed(…))`.
fn frame_at(state: &AppState, columns: u16, rows: u16, clock: Option<Clock>) -> Vec<String> {
    let session = Session::settle(&Loaded::default(), capabilities());
    let mut state = state.clone();
    state.size = graft::update::Size { columns, rows };
    if let Some(clock) = clock {
        state.harness.clock = clock;
    }
    let mut terminal = Terminal::new(TestBackend::new(columns, rows)).expect("test backend opens");
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

/// `r` on `main`'s row draws a rebase of `feature` onto it: ghosts for the
/// two replayed commits, and a box that names them.
#[test]
fn r_on_a_fork_draws_ghosts_for_the_commits_it_would_replay() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = preview_of_a_rebase(&app, state);

    let preview = state.preview.as_ref().expect("a preview is open");
    assert_eq!(
        preview.ghost_index.len(),
        2,
        "two commits are replayed, so two ghosts"
    );
    assert!(
        preview.undrawable.is_none(),
        "everything it points at is on screen or was read as an anchor"
    );
    app.close();
}

/// The ghosts reach the screen. The M5 group 04 review's Critical 1 was a
/// box that never drew and three layers of tests that never noticed, so this
/// one reads the real buffer at the real size.
#[test]
fn the_preview_box_and_its_ghost_rows_are_actually_drawn() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = preview_of_a_rebase(&app, state);

    let drawn = frame(&state, 100, 30);
    let text = drawn.join("\n");
    assert!(
        text.contains("Preview: rebase"),
        "the box's title is on screen:\n{text}"
    );
    assert!(
        text.contains("[ Cancel ]") || text.contains("Cancel"),
        "and its buttons are:\n{text}"
    );
    assert!(
        text.contains("git rebase"),
        "and the plan's own command line:\n{text}"
    );
    // A ghost row draws `—` where a real row draws a short sha.
    let ghost_rows = drawn
        .iter()
        .filter(|line| line.contains('\u{2014}'))
        .count();
    assert!(ghost_rows >= 2, "two ghost rows are drawn:\n{text}");
    app.close();
}

/// `Esc` closes the picture and leaves the repository exactly as it was.
/// Nothing runs until `Confirm` (PROMPT.md §1.7).
#[test]
fn esc_closes_the_preview_and_the_repository_never_moved() {
    let scratch = forked();
    let before = git(scratch.path(), &["rev-parse", "HEAD"]);
    let (app, state) = App::open(scratch.path());
    let state = preview_of_a_rebase(&app, state);
    let state = app.key(&state, KeyCode::Escape);

    assert!(state.preview.is_none(), "Esc closes it");
    assert_eq!(
        git(scratch.path(), &["rev-parse", "HEAD"]),
        before,
        "and a drawn preview changed nothing"
    );
    app.close();
}

/// The captures under `docs/shots/`, from this same real path — the **user**
/// judges the ghost rows and the docked box by eye (CLAUDE.md rule 6).
///
/// Run by hand: `cargo test -p graft --test it
/// preview::capture_preview_for_docs_shots -- --ignored --nocapture`.
#[test]
#[ignore = "writes docs/shots/m5-ui-preview-rebase-*.txt; run by hand to regenerate"]
fn capture_preview_for_docs_shots() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = preview_of_a_rebase(&app, state);
    // The scratch repository lives under a different temporary directory
    // every run, and the header row draws its path: replaced so that
    // regenerating the captures shows only what actually changed.
    let home = scratch.path().display().to_string();

    for (label, columns, rows) in [("80x24", 80u16, 24u16), ("160x48", 160, 48)] {
        let drawn = frame(&state, columns, rows);
        let mut out = format!(
            "# m5-ui-preview-rebase-{label}.txt: {columns}x{rows}, drawn by `view::view`\n\
             # from a real AppState — a real repository (`main` three commits, `feature`\n\
             # forked off the first with two of its own, `feature` checked out), opened\n\
             # over the real job pool, cursor moved to `main`'s row and `r` pressed\n\
             # through `update`, then the real `graft_ops::preview::plan` run\n\
             # (crates/graft/tests/it/preview.rs, capture_preview_for_docs_shots,\n\
             # #[ignore]d). Not a pty capture: the terminal is ratatui's TestBackend, so\n\
             # this is a character grid with no colour attributes in it — CLAUDE.md rule\n\
             # 5, colour ships unverified. The scratch repository's temporary path is\n\
             # replaced by `/code/api`; nothing else is touched.\n\
             #\n\
             # What right looks like:\n\
             #  - Two ghost rows for the commits the rebase would replay: a hollow node\n\
             #    glyph, dashed edges, and `\u{2014}` in place of a short sha and an age,\n\
             #    because those commits do not exist yet.\n\
             #  - `feature`'s badge has left its old row and sits on the newest ghost —\n\
             #    the picture shows where the branch *would* be, not where it is.\n\
             #  - The box is docked to the bottom of the graph, above the footer row, and\n\
             #    it is a box: every interior row bordered, nothing drawn in a hole.\n\
             #  - The box names what happens in the plan's own words, shows the real\n\
             #    `git rebase …` line it would run, and offers `[ Cancel ]` first with\n\
             #    the cursor on it (ADR-0038 \u{a7}1: the safe choice is where the cursor\n\
             #    rests).\n\
             #  - The rows behind the box are the picture, not the live graph, and the\n\
             #    box covers rows the graph never drew — no real row is hidden by it.\n\
             #\n"
        );
        out.push('\n');
        for line in &drawn {
            out.push_str(&line.replace(&home, "/code/api"));
            out.push('\n');
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots")
            .join(format!("m5-ui-preview-rebase-{label}.txt"));
        std::fs::write(&path, &out).expect("docs/shots is writable");
        println!("wrote {} ({} bytes)", path.display(), out.len());
    }
    app.close();
}

// ---------------------------------------------------------------------------
// The journal's time machine (piece 8): `Enter` on an older row.
//
// Every other test in this file stops at the picture. This one confirms it,
// which is the first and only time `graft_ops::undo::revert_to` runs at all:
// the whole write path — the key, `PreviewSubject::RevertTo`, the plan from
// `plan_revert_to`, `Command::RevertTo`, `jobs::undo::run_revert_to` and
// `Product::RevertedTo` — was built and typechecked but never exercised
// (this ticket's own handoff note). So it is read back from git, not from
// the status line.
// ---------------------------------------------------------------------------

/// The journal, as the app's own reader sees it.
fn journal_entries(repo: &Path) -> Vec<graft_core::journal::JournalEntry> {
    graft_ops::journal::Journal::open(&repo.join(".git/graft/undo-journal.jsonl"))
        .expect("the journal the writes just made is readable")
        .entries()
        .to_vec()
}

/// Stages `name` from inside the changes pane and waits for the write and
/// the refreshed file list that follows it.
fn stage(app: &App, state: AppState, name: &str) -> AppState {
    let state = crate::app_harness::cursor_to(app, state, name);
    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.settle_write(state, &format!("staging {name}"));
    app.pump(state, "the file list after the stage", |state| {
        matches!(state.detail.files, graft::update::detail::Loading::Ready(_))
    })
}

/// The three files the stages below journal, oldest first.
const STAGED: [&str; 3] = ["one.txt", "two.txt", "three.txt"];

/// A repository with one commit and three untracked files, ready to stage.
fn three_to_stage() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    commit(repo, "base.txt", "base\n", "add the base");
    for name in STAGED {
        std::fs::write(repo.join(name), "x\n").expect("the scratch repo is writable");
    }
    scratch
}

/// Stages all three through the changes pane: three journal entries, one
/// per stage, in the order they were made.
fn stage_all_three(app: &App, state: AppState) -> AppState {
    let mut state = crate::app_harness::changes_pane(app, state);
    for name in STAGED {
        state = stage(app, state, name);
    }
    state
}

/// `Esc` out of the pane, `J`, then down to the oldest of the three rows —
/// the one the time machine aims at (the browser lists newest first).
fn journal_at_the_oldest_row(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Escape);
    let state = app.key(&state, KeyCode::Char('J'));
    let state = app.pump(state, "the journal read", |state| {
        state.journal.shown
            && matches!(
                state.journal.entries,
                graft::update::detail::Loading::Ready(_)
            )
    });
    let state = app.key(&state, KeyCode::Char('j'));
    app.key(&state, KeyCode::Char('j'))
}

/// Waits for the plan the preview asked for, and insists it landed.
fn planned_preview(app: &App, state: AppState) -> AppState {
    let state = app.pump(state, "the revert plan", |state| {
        state.preview.as_ref().is_some_and(|preview| {
            !matches!(preview.plan, graft::update::detail::Loading::Pending(_))
        })
    });
    let plan = &state.preview.as_ref().expect("still open").plan;
    assert!(
        matches!(plan, graft::update::detail::Loading::Ready(_)),
        "the revert plan came back: {plan:?}"
    );
    state
}

/// Three stages, then `J`, then `Enter` on the oldest of them, then
/// `Confirm`: the index goes all the way back to empty, three `Undo`
/// entries are journaled, and the status line says how far it went.
#[test]
fn journal_enter_on_an_older_row_previews_and_confirm_really_reverts() {
    let scratch = three_to_stage();
    let repo = scratch.path();
    assert_eq!(
        git(repo, &["diff", "--cached", "--name-only"]).trim(),
        "",
        "the index starts empty"
    );

    let (app, state) = App::open(repo);
    let state = stage_all_three(&app, state);
    let staged = git(repo, &["diff", "--cached", "--name-only"]);
    assert_eq!(
        staged.split_whitespace().count(),
        3,
        "three stages landed: {staged}"
    );
    let before = journal_entries(repo);
    assert_eq!(before.len(), 3, "one entry per stage: {before:?}");
    let oldest = before[0].id.clone();

    let state = journal_at_the_oldest_row(&app, &state);

    // `Enter` previews. Nothing has run: the repository is untouched and no
    // write is in flight.
    let state = app.key(&state, KeyCode::Enter);
    let preview = state.preview.as_ref().expect("Enter opened a preview");
    assert_eq!(
        preview.subject,
        graft::update::preview::PreviewSubject::RevertTo(oldest),
        "the preview is aimed at the row under the cursor"
    );
    assert!(!state.journal.shown, "the preview has the screen");
    assert!(state.operating.is_none(), "Enter ran nothing");
    let state = planned_preview(&app, state);
    assert_eq!(
        journal_entries(repo).len(),
        3,
        "planning is a read: the journal did not grow"
    );
    assert_eq!(
        git(repo, &["diff", "--cached", "--name-only"]),
        staged,
        "planning is a read: the index did not move"
    );

    // `→` off `Cancel` onto `Confirm`, then `Enter`. Now it runs.
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    assert!(state.preview.is_none(), "the picture has been answered");
    let state = app.settle_write(state, "the revert");
    let said = match &state.status {
        graft::update::Status::Done(line) => line.clone(),
        other => panic!("the revert did not report Done: {other:?}"),
    };
    drop(state);
    app.close();

    three_stages_undone(repo, &said);
}

/// What the repository must look like once the time machine has run: the
/// index empty, the files still on disk, and one `Undo` entry per step.
fn three_stages_undone(repo: &Path, said: &str) {
    assert_eq!(
        git(repo, &["diff", "--cached", "--name-only"]).trim(),
        "",
        "the index is back where it was before the first stage"
    );
    for name in STAGED {
        assert!(
            repo.join(name).exists(),
            "{name} is unstaged, not deleted — the files are still there"
        );
    }
    let after = journal_entries(repo);
    assert_eq!(
        after.len(),
        6,
        "one `Undo` entry per step, never one summary entry (DESIGN \u{a7}6.6): {after:?}"
    );
    assert!(
        after[3..]
            .iter()
            .all(|entry| entry.kind == graft_core::journal::OperationKind::Undo),
        "the three new entries are undos: {:?}",
        &after[3..]
    );
    assert!(
        said.contains("3 steps"),
        "the line says how far it went: {said}"
    );
}

/// The time machine's own capture — the picture the journal's `Enter` draws
/// before anything runs. The **user** judges it (CLAUDE.md rule 6).
///
/// Run by hand: `cargo test -p graft --test it
/// preview::capture_time_machine_for_docs_shots -- --ignored --nocapture`.
#[test]
#[ignore = "writes docs/shots/m5-ui-preview-time-machine-*.txt; run by hand to regenerate"]
fn capture_time_machine_for_docs_shots() {
    let scratch = three_to_stage();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stage_all_three(&app, state);
    let state = journal_at_the_oldest_row(&app, &state);
    let state = app.key(&state, KeyCode::Enter);
    let state = planned_preview(&app, state);
    let home = repo.display().to_string();

    for (label, columns, rows) in [("80x24", 80u16, 24u16), ("160x48", 160, 48)] {
        let drawn = frame(&state, columns, rows);
        let mut out = format!(
            "# m5-ui-preview-time-machine-{label}.txt: {columns}x{rows}, drawn by\n\
             # `view::view` from a real AppState. A real repository with one commit and\n\
             # three files staged one at a time through the changes pane (three journal\n\
             # entries), then `J`, `j`, `j`, `Enter` on the oldest of the three, through\n\
             # `update` over the real job pool, then the real\n\
             # `graft_ops::preview::plan_revert_to` run\n\
             # (crates/graft/tests/it/preview.rs, capture_time_machine_for_docs_shots,\n\
             # #[ignore]d). Not a pty capture: ratatui's TestBackend, a character grid\n\
             # with no colour attributes \u{2014} CLAUDE.md rule 5, colour ships\n\
             # unverified. The scratch repository's temporary path is replaced by\n\
             # `/code/api`; nothing else is touched.\n\
             #\n\
             # What right looks like:\n\
             #  - The journal browser is gone and the preview box is in its place. `Esc`\n\
             #    puts the browser back on the same row.\n\
             #  - The box says what reverting to before that entry would do, in the\n\
             #    plan's own words \u{2014} how many steps, and what the index and working\n\
             #    tree would be. **No `git \u{2026}` line:** a revert is `graft`'s own\n\
             #    restore, step by step, not a git command anyone could type, and\n\
             #    `plan_revert_to` returns no commands (review 08 Minor 6 \u{2014} this\n\
             #    header used to ask the reader to look for one).\n\
             #  - `[ Cancel ]` is first and the cursor rests on it.\n\
             #  - Nothing has run: this picture is a read.\n\
             #\n"
        );
        out.push('\n');
        for line in &drawn {
            out.push_str(&line.replace(&home, "/code/api"));
            out.push('\n');
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots")
            .join(format!("m5-ui-preview-time-machine-{label}.txt"));
        std::fs::write(&path, &out).expect("docs/shots is writable");
        println!("wrote {} ({} bytes)", path.display(), out.len());
    }
    app.close();
}

// ---------------------------------------------------------------------------
// The `.` menu's history verbs (piece 6).
//
// `update::context_menu::verbs`' own tests settle what each verb *means* over
// scripted refs. Nothing there opens a repository, so nothing there can prove
// that `.` on a real branch row actually lists them, that the widget draws
// them, or that choosing one reaches `graft_ops::preview::plan`. That is what
// these do.
// ---------------------------------------------------------------------------

/// `.` on `main`'s row, with the menu's plan on hand.
/// Presses `j` until the cursor sits on `main`'s row — where the history
/// verbs have something to name.
fn on_mains_row(app: &App, state: AppState) -> AppState {
    let mut state = opened(app, state);
    let mut guard = 0;
    loop {
        assert!(
            guard < 20,
            "never found main's row: {:?}",
            rows_drawn(&state)
        );
        if on_main(&state) {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
        guard += 1;
    }
    state
}

fn menu_on_mains_row(app: &App, state: AppState) -> AppState {
    let state = on_mains_row(app, state);
    let state = app.key(&state, KeyCode::Char('.'));
    assert!(
        state.context_menu.is_some(),
        "`.` opens the menu: {:?}",
        state.status
    );
    state
}

/// The labels the menu lists, in order.
fn menu_labels(state: &AppState) -> Vec<String> {
    state
        .context_menu
        .as_ref()
        .expect("a menu is open")
        .menu
        .items
        .iter()
        .map(|item| item.label.as_str().to_owned())
        .collect()
}

/// The menu is **opaque**: on every row it draws, everything right of the
/// label is blank, out to the end of the line.
///
/// The widget pads each of its own rows, so the popup's own columns were
/// never the problem — the popup is only as wide as its longest label, and
/// the graph rows it floats over kept drawing their date and sha columns
/// beside each verb. In a character grid that reads as if the verb were
/// bound to that sha (`Copy SHA (y)  …  now  6d3bd01`). The user found it by
/// eye in `docs/shots/m5-ui-context-menu-verbs-80x24.txt`; this is the
/// assertion the capture could not make (m5-history-and-ghost-preview/12).
///
/// `frame` trims each line's trailing blanks, so an opaque row is exactly
/// `" {label}"` and a bleeding one is longer. Both capture widths, because
/// the bleed is wider and louder at 160 columns than at 80.
#[test]
fn the_context_menu_is_opaque_on_every_row_it_draws() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = menu_on_mains_row(&app, state);
    let labels = menu_labels(&state);
    assert!(labels.len() > 8, "the fixture's menu is long: {labels:?}");

    for (columns, rows) in [(80_u16, 24_u16), (160, 48)] {
        let drawn = frame(&state, columns, rows);
        for label in &labels {
            let wanted = format!(" {label}");
            let line = drawn
                .iter()
                .find(|line| line.starts_with(&wanted))
                .unwrap_or_else(|| {
                    panic!(
                        "{wanted:?} is drawn at {columns}x{rows}:\n{}",
                        drawn.join("\n")
                    )
                });
            assert_eq!(
                line,
                &wanted,
                "at {columns}x{rows} the menu row {label:?} carries what was \
                 underneath it to the right of its label:\n{}",
                drawn.join("\n")
            );
        }
    }
    app.close();
}

/// A trailing `(m)` on a menu label promises that `m` is live **right now**.
/// So pressing it with the menu open closes the menu and runs the verb, in
/// one keystroke, landing on exactly the picture `m` draws when no menu was
/// open (`docs/design/02-keymap.md` §2.1: the `.` menu claims `j`/`k`/`Enter`
/// /`Esc` and nothing else).
///
/// Compared frame-to-frame rather than field-to-field: "as if the menu had
/// not been open" is a claim about what the user ends up looking at.
#[test]
fn a_verb_key_the_menu_advertises_closes_it_and_runs_the_verb() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let row = on_mains_row(&app, state);

    let menu = app.key(&row, KeyCode::Char('.'));
    assert!(
        menu.context_menu.is_some(),
        "`.` opens the menu: {:?}",
        menu.status
    );
    let labels = menu_labels(&menu);
    assert!(
        labels.iter().any(|label| label.ends_with("(m)")),
        "the menu advertises `m`: {labels:?}"
    );

    let through_menu = app.key(&menu, KeyCode::Char('m'));
    assert!(
        through_menu.context_menu.is_none(),
        "`m` closes the menu on its way through"
    );
    assert!(
        through_menu.preview.is_some(),
        "`m` ran the merge it advertised instead of being swallowed: {:?}",
        through_menu.status
    );
    let through_menu = app.pump(through_menu, "the merge plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });

    let direct = app.key(&row, KeyCode::Char('m'));
    let direct = app.pump(direct, "the merge plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });

    assert_eq!(
        frame(&through_menu, 80, 24),
        frame(&direct, 80, 24),
        "`.` then `m` must land where a bare `m` lands"
    );
    app.close();
}

/// `m` is not alone: every letter the menu advertises has to be live, not
/// just the one the defect was found with. Each of these closes the menu and
/// then does its own job, so the screen it leaves is **not** the screen that
/// merely closing the menu leaves.
///
/// `f`/`p`/`P` are left out on purpose: pressing them really starts a
/// network job, which is not a fast-tier test. `y` is left out because this
/// harness cannot run it — `CopyToClipboard` is terminal output the event
/// loop performs, not a job the test runner can answer. That all four are
/// handed on is pinned by
/// `graft::update::context_menu::tests::context_menu_key_closes_and_hands_a_verb_key_on`,
/// and that the menu's own items run the same code `f`/`p`/`P` run is pinned
/// by `graft::update::context_menu::tests::the_menu_runs_what_the_dropped_keys_run`.
#[test]
fn every_verb_the_menu_advertises_is_live_while_it_is_open() {
    for code in [
        KeyCode::Char('x'),
        KeyCode::Char('m'),
        KeyCode::Char('r'),
        KeyCode::Char('u'),
    ] {
        let scratch = forked();
        let (app, state) = App::open(scratch.path());
        let menu = menu_on_mains_row(&app, state);

        let merely_closed = app.key(&menu, KeyCode::Escape);
        let pressed = app.key(&menu, code);
        assert!(
            pressed.context_menu.is_none(),
            "{code:?} closes the menu on its way through"
        );
        assert_ne!(
            frame(&pressed, 80, 24),
            frame(&merely_closed, 80, 24),
            "{code:?} was swallowed: the menu closed and nothing else happened"
        );
        app.close();
    }
}

/// `Esc` is one of the four keys the menu does claim: it closes the menu and
/// stops there, running nothing (`docs/design/02-keymap.md` §2.1).
#[test]
fn esc_closes_the_menu_and_runs_nothing() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let menu = menu_on_mains_row(&app, state);
    let closed = app.key(&menu, KeyCode::Escape);
    assert!(closed.context_menu.is_none(), "`Esc` closes the menu");
    assert!(
        closed.preview.is_none(),
        "`Esc` is not a verb: {:?}",
        closed.status
    );
    app.close();
}

/// Below 40x12 the Ghost Preview box gives up its frame and draws one
/// warning line instead. That line still has to be **opaque**: the caller
/// tints the whole rectangle, so anything the widget does not write itself
/// shows through as the graph row that was underneath, wearing the overlay's
/// background (m5-history-and-ghost-preview/13).
///
/// `frame` trims trailing blanks, so an opaque warning row is exactly the
/// message and a leaky one is longer.
#[test]
fn the_preview_warning_row_is_opaque_when_the_box_cannot_fit() {
    const WARNING: &str = "preview needs 40x12 · esc closes";
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = preview_of_a_rebase(&app, state);

    for (columns, rows) in [(38_u16, 20_u16), (39, 8), (80, 11), (80, 3), (80, 2)] {
        let drawn = frame(&state, columns, rows);
        let line = drawn
            .iter()
            .find(|line| line.contains(WARNING))
            .unwrap_or_else(|| {
                panic!(
                    "the warning is drawn at {columns}x{rows}:\n{}",
                    drawn.join("\n")
                )
            });
        assert_eq!(
            line,
            WARNING,
            "at {columns}x{rows} the warning row carries what was under it:\n{}",
            drawn.join("\n")
        );
    }
    app.close();
}

/// `.` on a real branch row lists the history verbs, named against the real
/// branches, and the widget really draws them.
#[test]
fn the_context_menu_lists_its_history_verbs_and_draws_them() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = menu_on_mains_row(&app, state);

    let labels = menu_labels(&state);
    for wanted in [
        "Checkout main",
        "Merge main into feature (m)",
        "Rebase feature onto main (r)",
        "Reset feature to here, mixed",
        "Reset feature to here, hard",
        "Delete branch main",
    ] {
        assert!(
            labels.iter().any(|label| label == wanted),
            "{wanted:?} missing from {labels:?}"
        );
    }

    // Drawn, not merely listed: the rows come off the real buffer.
    let text = frame(&state, 100, 30).join("\n");
    assert!(text.contains("Merge main into feature (m)"), "{text}");
    assert!(text.contains("Rebase feature onto main (r)"), "{text}");
    app.close();
}

/// Choosing a verb opens the real Ghost Preview of exactly that operation,
/// through `graft_ops::preview::plan`, and writes nothing.
#[test]
fn choosing_a_menu_verb_previews_that_operation_and_writes_nothing() {
    let scratch = forked();
    let before = git(
        scratch.path(),
        &["for-each-ref", "--format=%(refname) %(objectname)"],
    );
    let (app, state) = App::open(scratch.path());
    let state = menu_on_mains_row(&app, state);

    let wanted = "Rebase feature onto main (r)";
    let at = menu_labels(&state)
        .iter()
        .position(|label| label == wanted)
        .unwrap_or_else(|| panic!("{wanted:?} missing from {:?}", menu_labels(&state)));
    let expected = {
        let open = state.context_menu.as_ref().expect("a menu is open");
        let graft_ui::context_menu::ContextAction::History(index) = open.menu.items[at].action
        else {
            panic!("{wanted:?} is a history verb")
        };
        open.verbs[index].op.clone()
    };

    let mut state = state;
    for _ in 0..at {
        state = app.key(&state, KeyCode::Char('j'));
    }
    let state = app.key(&state, KeyCode::Enter);

    assert!(state.context_menu.is_none(), "the menu closed");
    let preview = state.preview.as_ref().expect("a preview opened");
    assert_eq!(
        preview.subject,
        graft::update::preview::PreviewSubject::Operation {
            op: Box::new(expected),
            opened_by: ".",
        },
        "the operation previewed is the one the label named"
    );

    // The plan comes back from the real `graft_ops::preview::plan`, with the
    // two commits it would replay in it.
    let state = app.pump(state, "the menu verb's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let text = frame(&state, 100, 30).join("\n");
    assert!(text.contains("Preview:"), "{text}");
    assert_eq!(
        git(
            scratch.path(),
            &["for-each-ref", "--format=%(refname) %(objectname)"]
        ),
        before,
        "planning is a read: nothing moved"
    );
    app.close();
}

/// The capture under `docs/shots/`, from this same real path, **asserted**
/// against what is committed there — the **user** judges the menu by eye
/// (CLAUDE.md rule 6), and this keeps what he judged from changing behind
/// his back.
///
/// It can assert now because the frame is reproducible: [`forked`] stamps a
/// fixed date on every commit it makes and [`FORK_NOW`] fixes the instant
/// the ages are drawn against. Before that it was not, and a snapshot of a
/// frame that wobbles is worse than none
/// (m5-history-and-ghost-preview/13).
///
/// To regenerate after a deliberate change: `GRAFT_UPDATE_SHOTS=1 cargo test
/// -p graft --test it preview::capture_context_menu_for_docs_shots`, then
/// look at the diff before committing it.
#[test]
fn capture_context_menu_for_docs_shots() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = menu_on_mains_row(&app, state);
    let home = scratch.path().display().to_string();
    let updating = std::env::var_os("GRAFT_UPDATE_SHOTS").is_some();

    for (label, columns, rows) in [("80x24", 80u16, 24u16), ("160x48", 160, 48)] {
        let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(FORK_NOW)));
        let mut out = context_menu_caption(label, columns, rows);
        out.push('\n');
        for line in &drawn {
            out.push_str(&line.replace(&home, "/code/api"));
            out.push('\n');
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots")
            .join(format!("m5-ui-context-menu-verbs-{label}.txt"));
        if updating {
            std::fs::write(&path, &out).expect("docs/shots is writable");
            println!("wrote {} ({} bytes)", path.display(), out.len());
            continue;
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|why| {
            panic!(
                "{} is missing ({why}); regenerate with GRAFT_UPDATE_SHOTS=1",
                path.display()
            )
        });
        assert_eq!(
            out, committed,
            "the {label} capture has drifted from what is committed. Read the \
             new frame, and if it is right regenerate with GRAFT_UPDATE_SHOTS=1:\n{out}"
        );
    }
    app.close();
}

/// The "what right looks like" block the M5 contact sheet puts this capture
/// under (ADR-0037).
///
/// Every line here is a claim the user checks by eye, so every line has to be
/// true of **this** width. The block was byte-identical in both files until
/// m5-history-and-ghost-preview/12, which left the 160-column capture
/// claiming to be the 80-column one.
fn context_menu_caption(label: &str, columns: u16, rows: u16) -> String {
    let width_line = if columns <= 80 {
        format!(
            "#  - At {columns} columns \u{2014} the narrow end \u{2014} the menu still fits\n\
             #    inside the graph pane rather than running off it.\n"
        )
    } else {
        format!(
            "#  - At {columns} columns the graph's date and sha columns sit far to the\n\
             #    right of the menu, and they stop dead at the menu's own rows: the\n\
             #    popup clears the whole band it stands on, so no verb has a stray\n\
             #    sha beside it.\n"
        )
    };
    format!(
        "# m5-ui-context-menu-verbs-{label}.txt: {columns}x{rows}, drawn by `view::view`\n\
             # from a real AppState — a real repository (`main` three commits, `feature`\n\
             # forked off the first with two of its own, `feature` checked out), opened\n\
             # over the real job pool, cursor moved to `main`'s row and `.` pressed\n\
             # through `update`\n\
             # (crates/graft/tests/it/preview.rs, capture_context_menu_for_docs_shots).\n\
             # This file is a golden snapshot: that test asserts the frame still\n\
             # matches it, so drift fails the suite. Regenerate on purpose with\n\
             # GRAFT_UPDATE_SHOTS=1. Not a pty capture: the terminal is ratatui's\n\
             # TestBackend, so this is a character grid with no colour attributes in\n\
             # it — CLAUDE.md rule 5, colour ships unverified. The fixture's commits\n\
             # carry fixed dates and the ages are drawn against a fixed clock, so the\n\
             # frame is the same on every run and does not age. The scratch\n\
             # repository's temporary path is replaced by `/code/api`; nothing else\n\
             # is touched.\n\
             #\n\
             # What right looks like:\n\
             #  - The menu is anchored under `main`'s row and is wide enough for its\n\
             #    longest label, with the cursor on the first item.\n\
             #  - The row's own actions come first (`Copy SHA (y)`, `Compare with this\n\
             #    (x)`, `Open details (\u{23ce})`), then the history verbs, then the ambient\n\
             #    ones (`Fetch (f)` \u{2026} `Undo (u)`). The menu lists every verb valid for\n\
             #    this row at every width \u{2014} it does not filter against the hint bar,\n\
             #    because a verb you can only discover at 160 columns is the failure\n\
             #    ADR-0039 exists to prevent.\n\
             #  - The menu is opaque. On each of its rows, everything right of the\n\
             #    label is blank out to the edge of the pane: no date or sha from the\n\
             #    commit row underneath survives beside a verb, where it would read as\n\
             #    if the verb were bound to that sha.\n\
             #  - The history verbs name the real branches, not placeholders: `Checkout\n\
             #    main`, `Merge main into feature (m)`, `Rebase feature onto main (r)`,\n\
             #    `Cherry-pick <sha7>`, `Revert <sha7>`, `Reset feature to here,\n\
             #    mixed`/`, hard`, `Delete branch main`. `feature` is the branch\n\
             #    checked out, `main` is the row under the cursor.\n\
             #  - No verb names the branch you are standing on where git would refuse\n\
             #    it: there is no `Checkout feature` and no `Delete branch feature`.\n\
             #  - A trailing `(key)` means one thing everywhere in the menu: that\n\
             #    letter is live right now, so next time you can skip the menu. Only\n\
             #    `Merge` and `Rebase` carry one, because only `m` and `r` exist. A\n\
             #    qualifier that is not a key never wears that shape \u{2014} hence `Reset\n\
             #    \u{2026}, mixed`, not `(mixed)` (`docs/design/02-keymap.md` \u{a7}2.1).\n\
             #  - Choosing any of them opens a picture, not a write (PROMPT.md \u{a7}1.7).\n\
             {width_line}\
             #\n"
    )
}

// ---------------------------------------------------------------------------
// Scrolled a screen or more into history (ticket 17 F1).

/// A history taller than the screen with a branch left behind at the bottom
/// of it: the state every earlier test and capture missed.
///
/// `old-release` is thirty rows down, so reaching it scrolls the viewport and
/// `history.top` stops being `0` — the one value where the preview's slice
/// start saturates and drawing the picture from the slice's first row looks
/// identical to drawing it from the user's own first row.
///
/// `redesign` is what puts the distance there: twenty-five commits newer than
/// `topic`'s, the way a colleague's branch sits above yours in a real
/// repository. Rebasing `topic` onto `old-release` then replays five
/// commits, and every ghost lands beside the row the cursor is on.
fn left_behind() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    dated_commit(path, "base.txt", "base\n", "add the base", 1);
    git(path, &["switch", "-c", "old-release"]);
    dated_commit(path, "release.txt", "1.0\n", "cut release 1.0", 2);
    git(path, &["switch", "main"]);
    for (n, minute) in (3..=5).enumerate() {
        dated_commit(
            path,
            &format!("main-{n}.txt"),
            "x\n",
            &format!("main moves on ({n})"),
            minute,
        );
    }
    git(path, &["switch", "-c", "topic"]);
    dated_commit(path, "topic-one.txt", "one\n", "start the topic", 6);
    dated_commit(path, "topic-two.txt", "two\n", "finish the topic", 7);
    git(path, &["switch", "-c", "redesign", "main"]);
    for n in 0..25 {
        dated_commit(
            path,
            &format!("redesign-{n:02}.txt"),
            "x\n",
            &format!("redesign step {n:02}"),
            8 + i64::from(n),
        );
    }
    git(path, &["switch", "topic"]);
    scratch
}

/// Is the cursor on a row carrying `name`'s branch badge?
fn on_branch(state: &AppState, name: &[u8]) -> bool {
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
    refs.on(row.commit.id)
        .iter()
        .any(|label| matches!(label, graft::refs::RefLabel::Branch { name: it, .. } if it == name))
}

/// Scrolls to `old-release`'s row — well below the first screen — and
/// presses `r`.
fn preview_scrolled_into_history(app: &App, state: AppState) -> AppState {
    let mut state = opened(app, state);
    // A screen the fixture is taller than, without the fixture having to be
    // a hundred real commits: the harness opens at 120x40.
    state = app.resize(&state, 80, 24);
    let mut guard = 0;
    loop {
        assert!(guard < 200, "never reached old-release's row");
        if on_branch(&state, b"old-release") {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
        state = app.pump(state, "the row under the cursor", |state| {
            state
                .history
                .as_ref()
                .and_then(graft::history::History::selected)
                .is_some()
        });
        guard += 1;
    }
    let top = state.history.as_ref().expect("a history").top;
    assert!(
        top > 0,
        "the whole point of this fixture is a scrolled viewport"
    );
    let state = app.key(&state, KeyCode::Char('r'));
    assert!(
        state.preview.is_some(),
        "`r` opens a preview: {:?}",
        state.status
    );
    app.pump(state, "the preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    })
}

/// Ticket 17 F1, against a real repository and the real drawn frame: the
/// rows the preview draws are the user's own screen.
///
/// The unit test in `view::tests` pins the same rule over scripted state;
/// this one proves it survives the real walk, the real plan job and the
/// real slice.
#[test]
fn a_preview_scrolled_into_history_draws_the_rows_the_user_was_on() {
    let scratch = left_behind();
    let (app, state) = App::open(scratch.path());
    let state = preview_scrolled_into_history(&app, state);

    let drawn = frame(&state, 80, 24).join("\n");
    assert!(
        drawn.contains("cut release 1.0"),
        "the row the cursor is on is on screen:\n{drawn}"
    );
    assert!(
        !drawn.contains("redesign step 24"),
        "and the newest screen, a screen above the user's, is not:\n{drawn}"
    );
    assert!(
        drawn.contains("start the topic"),
        "the replayed commits are drawn as ghosts beside it:\n{drawn}"
    );
    app.close();
}

/// The capture the milestone never had: the preview **scrolled into
/// history**, which is where the graph spends most of its life on a real
/// repository. The user judges it by eye (CLAUDE.md rule 6).
///
/// Run by hand: `cargo test -p graft --test it
/// preview::capture_scrolled_preview_for_docs_shots -- --ignored --nocapture`.
#[test]
#[ignore = "writes docs/shots/m5-ui-preview-scrolled-*.txt; run by hand to regenerate"]
fn capture_scrolled_preview_for_docs_shots() {
    let scratch = left_behind();
    let (app, state) = App::open(scratch.path());
    let state = preview_scrolled_into_history(&app, state);
    let top = state.history.as_ref().expect("a history").top;
    let home = scratch.path().display().to_string();

    for (label, columns, rows) in [("80x24", 80u16, 24u16), ("160x48", 160, 48)] {
        let drawn = frame(&state, columns, rows);
        let mut out = format!(
            "# m5-ui-preview-scrolled-{label}.txt: {columns}x{rows}, drawn by `view::view`\n\
             # from a real AppState. A real repository: `old-release` cut at the base,\n\
             # three commits on `main`, `topic` forked off its tip with two of its own,\n\
             # and `redesign` — twenty-five commits newer than all of them — pushing\n\
             # `old-release` thirty rows down the graph. Opened over the real job pool,\n\
             # the cursor walked down to `old-release`'s row with `j` (so the viewport\n\
             # really is scrolled: `history.top` is {top}, not 0), then `r` pressed\n\
             # through `update` and the real `graft_ops::preview::plan` run\n\
             # (crates/graft/tests/it/preview.rs, capture_scrolled_preview_for_docs_shots,\n\
             # #[ignore]d). Not a pty capture: ratatui's TestBackend, a character grid\n\
             # with no colour attributes in it — CLAUDE.md rule 5, colour ships\n\
             # unverified. The scratch repository's temporary path is replaced by\n\
             # `/code/api`; nothing else is touched.\n\
             #\n\
             # What right looks like:\n\
             #  - The rows on screen are the rows the user was looking at. `cut release\n\
             #    1.0` — the row the cursor was on when `r` was pressed — is visible,\n\
             #    and the newest commits (`redesign step 24` and its neighbours, a\n\
             #    screen or more above) are **not**. Before ticket 17 this capture\n\
             #    showed exactly those rows instead, and the user's own were off-screen.\n\
             #  - The five ghost rows sit right above `cut release 1.0`: hollow node\n\
             #    glyphs, dashed edges, `\u{2014}` in place of a short sha and an age.\n\
             #    They are `main`'s three commits and `topic`'s two, replayed.\n\
             #  - `topic`'s badge has left its own row and sits on the newest ghost.\n\
             #  - The highlighted row is `cut release 1.0`: the commit the key was\n\
             #    pressed on, not whichever row happens to sit where the box's choice\n\
             #    cursor points. (Highlight is a colour, so the grid cannot show it —\n\
             #    unverified by construction, rule 5.)\n\
             #  - The box is docked above the footer, every interior row bordered, and\n\
             #    `[ Cancel ]` comes first with the cursor on it (ADR-0038 \u{a7}1).\n\
             #\n"
        );
        out.push('\n');
        for line in &drawn {
            out.push_str(&line.replace(&home, "/code/api"));
            out.push('\n');
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots")
            .join(format!("m5-ui-preview-scrolled-{label}.txt"));
        std::fs::write(&path, &out).expect("docs/shots is writable");
        println!("wrote {} ({} bytes)", path.display(), out.len());
    }
    app.close();
}
