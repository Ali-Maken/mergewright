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
use graft::session::Session;
use graft::update::AppState;
use graft_core::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::app_harness::{App, git, git_as_graft};
use crate::hint_bar::capabilities;

/// A fork: `main` with three commits, `feature` branched off the first and
/// carrying two of its own, `feature` checked out. Rebasing `feature` onto
/// `main` replays two commits, which is a picture with ghosts in it.
fn forked() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    commit(path, "base.txt", "base\n", "add the base");
    git(path, &["branch", "feature"]);
    commit(path, "main-one.txt", "one\n", "main moves on");
    commit(path, "main-two.txt", "two\n", "main moves again");
    git(path, &["switch", "feature"]);
    commit(path, "work-one.txt", "one\n", "start the feature");
    commit(path, "work-two.txt", "two\n", "finish the feature");
    scratch
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
    let session = Session::settle(&Loaded::default(), capabilities());
    let mut state = state.clone();
    state.size = graft::update::Size { columns, rows };
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
             #    plan's own words \u{2014} how many steps, what the index and working tree\n\
             #    would be, and the commands it would run. No hand-written git line.\n\
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
