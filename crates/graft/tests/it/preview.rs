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

/// The band the menu clears runs to the edge of the **graph** pane and
/// stops there: with the detail pane open, every column of the detail pane
/// is exactly what it was before the menu opened.
///
/// `primary_pane_area` (`view::frame`) is what decides which half of the
/// split the band runs to, and nothing exercised it: the opacity test above
/// runs with the detail pane closed at both sizes, and so does the golden
/// snapshot. Swapping `split_area(…).0` for `.1` — clearing *across* the
/// detail pane — passed the whole suite (M5 groups 12/13 review, M3).
#[test]
fn the_menus_band_stops_at_the_detail_pane_instead_of_clearing_it() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = on_mains_row(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the detail pane's file list", |state| {
        matches!(state.detail.files, graft::update::detail::Loading::Ready(_))
    });

    // 160x48 lays the detail pane out to the right (`DetailLayout::Right`),
    // which is the split this is about.
    let width = 160_usize;
    let closed = frame(&state, 160, 48);
    let split = char_index_of(&closed[0], "COMMIT ")
        .unwrap_or_else(|| panic!("the detail pane draws its header:\n{}", closed.join("\n")));
    assert!(
        closed
            .iter()
            .filter_map(|line| char_index_of(line, "parents "))
            .chain(
                closed
                    .iter()
                    .filter_map(|line| char_index_of(line, "A main-two.txt"))
            )
            .all(|at| at == split),
        "the detail pane's own rows all start at its left edge, column {split}:\n{}",
        closed.join("\n")
    );

    let menu = app.key(&state, KeyCode::Char('.'));
    assert!(menu.context_menu.is_some(), "`.` opens the menu");
    let open = frame(&menu, 160, 48);

    for label in menu_labels(&menu) {
        let wanted = format!(" {label}");
        let row = open
            .iter()
            .position(|line| line.starts_with(&wanted))
            .unwrap_or_else(|| panic!("{wanted:?} is drawn:\n{}", open.join("\n")));
        let drawn = cells(&open[row], width);
        let before = cells(&closed[row], width);
        let left: String = drawn[..split].iter().collect();
        assert_eq!(
            left.trim_end(),
            wanted,
            "the menu row {label:?} carries what was under it out to the split:\n{}",
            open.join("\n")
        );
        assert_eq!(
            &drawn[split..],
            &before[split..],
            "the menu's band cleared into the detail pane on row {row} ({label:?}):\n{}",
            open.join("\n")
        );
    }
    app.close();
}

/// The column `needle` starts at, counting cells rather than bytes: these
/// frames are full of box drawing and `…`.
fn char_index_of(line: &str, needle: &str) -> Option<usize> {
    line.find(needle).map(|byte| line[..byte].chars().count())
}

/// One frame line as `width` cells. [`frame`] trims trailing blanks, so the
/// line has to be padded back out before two of them can be compared
/// column by column.
fn cells(line: &str, width: usize) -> Vec<char> {
    let mut cells: Vec<char> = line.chars().collect();
    cells.resize(width, ' ');
    cells
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
/// then does **its own** job — named here per key, not as "the screen
/// changed".
///
/// It used to assert only that the frame differed from the one `Esc` leaves.
/// That is "something happened", and for `u` on this fixture the something
/// was a line on the status bar: a future generic "key not bound" status
/// would have kept it green (M5 groups 12/13 review, m3).
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
        let tip = main_tip(scratch.path());
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
        // The effect this key, and no other, is supposed to have.
        the_effect_of(code, &pressed, &tip);
        app.close();
    }
}

/// What each letter the menu advertises is supposed to *do*, named per key.
/// The cursor is on `main`'s row, whose commit is `tip`.
fn the_effect_of(code: KeyCode, pressed: &AppState, tip: &str) {
    match code {
        KeyCode::Char('x') => {
            let base = pressed
                .compare_base
                .as_ref()
                .unwrap_or_else(|| panic!("`x` pinned no compare base: {:?}", pressed.status));
            assert_eq!(base.oid.to_string(), tip, "`x` marked the cursor's commit");
        }
        KeyCode::Char('m') => assert!(
            matches!(
                planned(pressed, "m"),
                graft_ops::operation::Operation::Merge(_)
            ),
            "`m` planned a merge, not {:?}",
            planned(pressed, "m")
        ),
        KeyCode::Char('r') => assert!(
            matches!(
                planned(pressed, "r"),
                graft_ops::operation::Operation::Rebase(_)
            ),
            "`r` planned a rebase, not {:?}",
            planned(pressed, "r")
        ),
        KeyCode::Char('u') => {
            assert!(
                pressed.operating.is_some(),
                "`u` started an undo: {:?}",
                pressed.status
            );
            let said = format!("{:?}", pressed.status).to_lowercase();
            assert!(
                said.contains("undo"),
                "`u` says on the status line what it is doing: {said}"
            );
        }
        other => panic!("no effect named for {other:?}"),
    }
}

/// The operation the preview `key` opened would run.
fn planned<'a>(state: &'a AppState, key: &str) -> &'a graft_ops::operation::Operation {
    state
        .preview
        .as_ref()
        .and_then(|preview| preview.subject.operation())
        .unwrap_or_else(|| panic!("`{key}` opened no preview: {:?}", state.status))
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
/// look at the diff before committing it. That run is **red** — see
/// [`check_capture`].
#[test]
fn capture_context_menu_for_docs_shots() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = menu_on_mains_row(&app, state);
    let home = scratch.path().display().to_string();
    let updating = updating_shots(std::env::var_os("GRAFT_UPDATE_SHOTS").as_deref());

    let mut drifted = Vec::new();
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
        if let Err(why) = check_capture(&path, &out, updating) {
            drifted.push(why);
        }
    }
    app.close();
    assert!(drifted.is_empty(), "{}", drifted.join("\n\n"));
}

/// May this run rewrite a committed capture? Only the literal `1`.
///
/// `GRAFT_UPDATE_SHOTS=0` or `=false` left exported in a shell used to mean
/// "yes" (`var_os(..).is_some()`), which is the shape of an accident, not of
/// an instruction (M5 groups 12/13 review, m1).
fn updating_shots(value: Option<&std::ffi::OsStr>) -> bool {
    value == Some(std::ffi::OsStr::new("1"))
}

/// One capture against what is committed at `path`, rewritten first when
/// `updating`.
///
/// Drift is an `Err` **whether or not** the file was rewritten. A
/// regeneration is therefore always a red run, and stays red until the new
/// capture is committed and the suite runs again without the variable — so
/// a `GRAFT_UPDATE_SHOTS` someone left exported can no longer turn a real
/// regression green, and a regenerated file is always something `git status`
/// has to show (M5 groups 12/13 review, m1).
fn check_capture(path: &Path, out: &str, updating: bool) -> Result<(), String> {
    let committed = std::fs::read_to_string(path);
    if updating {
        std::fs::write(path, out)
            .map_err(|why| format!("{} is not writable: {why}", path.display()))?;
    }
    let committed = committed.map_err(|why| {
        format!(
            "{} is missing ({why}); regenerate with GRAFT_UPDATE_SHOTS=1",
            path.display()
        )
    })?;
    if committed == out {
        return Ok(());
    }
    let rewritten = if updating {
        "It has been rewritten — read the diff, and commit it if it is right."
    } else {
        "Read the new frame, and if it is right regenerate with GRAFT_UPDATE_SHOTS=1."
    };
    Err(format!(
        "{} has drifted from what is committed. {rewritten}\n{out}",
        path.display()
    ))
}

/// A normal run fails on drift and rewrites nothing; only the literal
/// `GRAFT_UPDATE_SHOTS=1` rewrites, and that run is red too.
///
/// Over a temporary file rather than the real one, and over the policy
/// function rather than the environment: `cargo test` runs a binary's tests
/// in one process, so a test that set the variable would change what its
/// neighbours saw.
#[test]
fn only_an_explicit_regeneration_rewrites_a_capture() {
    assert!(updating_shots(Some(std::ffi::OsStr::new("1"))));
    for accident in ["", "0", "false", "no", "2"] {
        assert!(
            !updating_shots(Some(std::ffi::OsStr::new(accident))),
            "GRAFT_UPDATE_SHOTS={accident:?} is not an instruction to rewrite"
        );
    }
    assert!(!updating_shots(None));

    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path().join("capture.txt");
    std::fs::write(&path, "committed\n").expect("the scratch dir is writable");

    assert!(
        check_capture(&path, "committed\n", false).is_ok(),
        "a capture that has not drifted passes"
    );

    let drifted = check_capture(&path, "new\n", false);
    assert!(drifted.is_err(), "a normal run fails on drift: {drifted:?}");
    assert_eq!(
        std::fs::read_to_string(&path).expect("the file is still there"),
        "committed\n",
        "a normal run rewrote a committed capture"
    );

    let regenerated = check_capture(&path, "new\n", true);
    assert!(
        regenerated.is_err(),
        "a regeneration is red until the new capture is committed: {regenerated:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&path).expect("the file is still there"),
        "new\n",
        "the regeneration wrote the new capture out for `git status` to show"
    );
    assert!(
        check_capture(&path, "new\n", false).is_ok(),
        "once committed, the normal run is green again"
    );
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
// Right-click: the menu's row and the verb's row are the same row
// (m5-history-and-ghost-preview/18 F1).

/// The subject of `main`'s tip in [`forked`] — the row these tests
/// right-click, which is never the row the cursor starts on.
const MAIN_TIP_SUBJECT: &str = "main moves again";

/// `main`'s tip as `git` itself reports it. Every assertion below measures
/// the verb against this, and the menu's labels against the branch name —
/// never the menu against the verb, which is what the defect would have
/// kept agreeing with.
fn main_tip(repo: &Path) -> String {
    git(repo, &["rev-parse", "main"]).trim().to_owned()
}

/// The screen row `subject` is drawn on, read out of the frame the user
/// sees rather than from the geometry the code under test uses.
fn screen_row_of(state: &AppState, subject: &str) -> u16 {
    let drawn = frame(
        state,
        crate::app_harness::SIZE.columns,
        crate::app_harness::SIZE.rows,
    );
    let hits: Vec<usize> = drawn
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(subject))
        .map(|(y, _)| y)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "{subject:?} is drawn on exactly one row; frame: {drawn:?}"
    );
    u16::try_from(hits[0]).expect("a screen row fits in u16")
}

/// Opened, cursor left where it lands (`feature`'s tip), then a right-click
/// on `main`'s row — a row the cursor is **not** on.
fn right_click_on_mains_row(app: &App, state: AppState) -> AppState {
    let state = opened(app, state);
    assert!(
        !on_main(&state),
        "the cursor has to start away from main's row or this proves nothing"
    );
    let row = screen_row_of(&state, MAIN_TIP_SUBJECT);
    let menu = app.right_click(&state, 2, row);
    assert!(
        menu.context_menu.is_some(),
        "a right-click on a commit row opens its menu: {:?}",
        menu.status
    );
    menu
}

/// A right-click is a pick: it moves the cursor onto the row it opens the
/// menu on, so "a verb acts on the cursor" and "the menu names this row"
/// stay the same sentence (`docs/design/02-keymap.md` §2.1).
///
/// Before this, the menu opened on the clicked row and left the cursor
/// alone, and group 13's fall-through then ran the letter against the
/// cursor: the label named one commit and the verb moved another
/// (m5-history-and-ghost-preview/18 F1, review `96edfd7` M1).
#[test]
fn a_right_click_moves_the_cursor_to_the_row_it_opens_the_menu_on() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let menu = right_click_on_mains_row(&app, state);
    assert!(
        on_main(&menu),
        "the cursor followed the right-click onto main's row"
    );
    assert!(
        !menu.on_working_tree,
        "a commit row is not the working tree"
    );
    app.close();
}

/// The menu's labels and the verb it hands on both name the row that was
/// clicked — each checked against `git`, not against the other.
///
/// `m`: the label says `Merge main into feature`, and the merge that opens
/// carries `main`'s tip as its source. With the cursor left on `feature`'s
/// tip this used to refuse with `already on it` and draw nothing.
#[test]
fn m_after_a_right_click_merges_the_branch_the_menu_named() {
    let scratch = forked();
    let tip = main_tip(scratch.path());
    let (app, state) = App::open(scratch.path());
    let menu = right_click_on_mains_row(&app, state);
    let labels = menu_labels(&menu);
    assert!(
        labels
            .iter()
            .any(|label| label == "Merge main into feature (m)"),
        "the menu names main as what it would merge: {labels:?}"
    );

    let pressed = app.key(&menu, KeyCode::Char('m'));
    let pressed = app.pump(pressed, "the merge plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let op = pressed
        .preview
        .as_ref()
        .and_then(|preview| preview.subject.operation())
        .unwrap_or_else(|| panic!("`m` opened no preview: {:?}", pressed.status));
    match op {
        graft_ops::operation::Operation::Merge(params) => assert_eq!(
            params.source_tip.to_string(),
            tip,
            "the merge's source is the row the menu named"
        ),
        other => panic!("`m` planned something else: {other:?}"),
    }
    app.close();
}

/// `r`: the label says `Rebase feature onto main`, and the rebase that
/// opens carries `main`'s tip as what it replays onto.
#[test]
fn r_after_a_right_click_rebases_onto_the_branch_the_menu_named() {
    let scratch = forked();
    let tip = main_tip(scratch.path());
    let (app, state) = App::open(scratch.path());
    let menu = right_click_on_mains_row(&app, state);
    let labels = menu_labels(&menu);
    assert!(
        labels
            .iter()
            .any(|label| label == "Rebase feature onto main (r)"),
        "the menu names main as what it would rebase onto: {labels:?}"
    );

    let pressed = app.key(&menu, KeyCode::Char('r'));
    let pressed = app.pump(pressed, "the rebase plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let op = pressed
        .preview
        .as_ref()
        .and_then(|preview| preview.subject.operation())
        .unwrap_or_else(|| panic!("`r` opened no preview: {:?}", pressed.status));
    match op {
        graft_ops::operation::Operation::Rebase(params) => assert_eq!(
            params.onto_tip.to_string(),
            tip,
            "the rebase replays onto the row the menu named"
        ),
        other => panic!("`r` planned something else: {other:?}"),
    }
    app.close();
}

/// `x`: the compare base it pins is the clicked commit, not the cursor's.
#[test]
fn x_after_a_right_click_marks_the_commit_the_menu_named() {
    let scratch = forked();
    let tip = main_tip(scratch.path());
    let (app, state) = App::open(scratch.path());
    let menu = right_click_on_mains_row(&app, state);
    let pressed = app.key(&menu, KeyCode::Char('x'));
    let base = pressed
        .compare_base
        .as_ref()
        .unwrap_or_else(|| panic!("`x` pinned nothing: {:?}", pressed.status));
    assert_eq!(
        base.oid.to_string(),
        tip,
        "`x` marked the row the menu named"
    );
    app.close();
}

/// `y` is the silent one: nothing is drawn and nothing is confirmed, so a
/// wrong sha leaves the app looking correct and comes back hours later in
/// a shell command the user pasted it into. It gets its own test for that
/// reason (m5-history-and-ghost-preview/18 F1).
///
/// The commands are taken rather than dispatched: `CopyToClipboard` is
/// terminal output the event loop performs, and the job runner refuses it.
#[test]
fn y_after_a_right_click_copies_the_sha_the_menu_named() {
    let scratch = forked();
    let tip = main_tip(scratch.path());
    let (app, state) = App::open(scratch.path());
    let menu = right_click_on_mains_row(&app, state);
    let labels = menu_labels(&menu);
    assert!(
        labels.iter().any(|label| label == "Copy SHA (y)"),
        "the menu advertises `y`: {labels:?}"
    );

    let (pressed, commands) = crate::app_harness::press_for_commands(
        &menu,
        graft_core::event::KeyPress::plain(KeyCode::Char('y')),
    );
    assert_eq!(
        commands,
        vec![graft::update::Command::CopyToClipboard(tip.clone())],
        "`y` copied the sha of the row the menu named"
    );
    match &pressed.status {
        graft::update::Status::Copied { short } => assert!(
            tip.starts_with(short.as_str()),
            "the footer names the sha that was copied: {short} of {tip}"
        ),
        other => panic!("`y` said nothing: {other:?}"),
    }
    app.close();
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
