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

use graft::session::Clock;
use graft::update::AppState;
use graft_core::event::KeyCode;

use crate::app_harness::{App, git, git_as_graft};
use graft_core::on_disk::CURRENT_DIR;

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
pub(crate) const FORK_EPOCH: i64 = 1_609_459_200;

/// The instant the fixture's relative dates are drawn against: a minute
/// after its newest commit, so a capture of it reads the same age forever
/// instead of drifting with real time (`session::Clock::Fixed`).
pub const FORK_NOW: i64 = FORK_EPOCH + 6 * 60;

/// The same for [`left_behind`], whose newest commit is at minute 67 —
/// sixty `redesign` steps past anything [`forked`] makes.
///
/// It needs its own instant because drawing that fixture against
/// [`FORK_NOW`] would put 61 commits in the future, and a future commit is
/// not rendered as an age at all: `relative_date` returns the word `future`
/// (`graft-ui/src/graph_screen/columns.rs:61`). A pinned clock that is too
/// early changes what the capture *shows*, which is the one thing pinning
/// it must not do.
const LEFT_BEHIND_NOW: i64 = FORK_EPOCH + 70 * 60;

/// A fork: `main` with three commits, `feature` branched off the first and
/// carrying two of its own, `feature` checked out. Rebasing `feature` onto
/// `main` replays two commits, which is a picture with ghosts in it.
pub fn forked() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    crate::app_harness::pin_identity(path);
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
pub(crate) fn dated_commit(repo: &Path, name: &str, body: &str, subject: &str, minute: i64) {
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
pub fn opened(app: &App, state: AppState) -> AppState {
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
    preview_opened_by(app, state, 'r')
}

/// `key` (`m` or `r`) on `main`'s row, and the plan landed.
fn preview_opened_by(app: &App, state: AppState, key: char) -> AppState {
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
    let state = app.key(&state, KeyCode::Char(key));
    assert!(
        state.preview.is_some(),
        "`{key}` opens a preview: {:?}",
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
pub fn on_main(state: &AppState) -> bool {
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

pub fn rows_drawn(state: &AppState) -> Vec<String> {
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
pub fn frame(state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    frame_at(state, columns, rows, None)
}

/// [`frame`], with the instant its relative dates are drawn against. `None`
/// is the wall clock; a capture that must not age passes
/// `Some(Clock::Fixed(…))`.
pub fn frame_at(state: &AppState, columns: u16, rows: u16, clock: Option<Clock>) -> Vec<String> {
    crate::captures::frame_at(state, columns, rows, clock)
}

/// `r` on `main`'s row draws a rebase of `feature` onto it: ghosts for the
/// two replayed commits, and a box that names them.
#[test]
fn r_on_a_fork_draws_ghosts_for_the_commits_it_would_replay() {
    let scratch = forked();
    let (app, state) = App::open_with_clock(
        scratch.path(),
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
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
    let (app, state) = App::open_with_clock(
        scratch.path(),
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
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
    let (app, state) = App::open_with_clock(
        scratch.path(),
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
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
/// Regenerate on purpose: `MWR_UPDATE_SHOTS=1 cargo test -p mergewright --test it preview::capture_preview_for_docs_shots`.
#[test]
fn capture_preview_for_docs_shots() {
    let scratch = forked();
    let (app, state) = App::open_with_clock(
        scratch.path(),
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let state = preview_of_a_rebase(&app, state);
    let mut drifted = Vec::new();

    for (label, columns, rows) in [("80x24", 80u16, 24u16), ("160x48", 160, 48)] {
        let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(FORK_NOW)));
        let mut out = format!(
            "# m5-ui-preview-rebase-{label}.txt: {columns}x{rows}, drawn by `view::view`\n\
             # from a real AppState — a real repository (`main` three commits, `feature`\n\
             # forked off the first with two of its own, `feature` checked out), opened\n\
             # over the real job pool, cursor moved to `main`'s row and `r` pressed\n\
             # through `update`, then the real `graft_ops::preview::plan` run\n\
             # (crates/graft/tests/it/preview.rs, capture_preview_for_docs_shots,\n\
             # asserted). Not a pty capture: the terminal is ratatui's TestBackend, so\n\
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
            out.push_str(line);
            out.push('\n');
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots")
            .join(format!("m5-ui-preview-rebase-{label}.txt"));
        if let Err(why) = graft_fixtures::check(&path, &out) {
            drifted.push(why);
        }
    }
    app.close();
    assert!(drifted.is_empty(), "{}", drifted.join("\n\n"));
}

/// `m` on `main`'s row: the merge box (ease-pass 10) has keys, not buttons,
/// and its footer names `y` once, no `m merge` alias. Regenerate on
/// purpose with `MWR_UPDATE_SHOTS=1`.
#[test]
fn capture_fix_preview_verb_for_docs_shots() {
    let scratch = forked();
    let (app, state) = App::open_with_clock(
        scratch.path(),
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let state = preview_opened_by(&app, state, 'm');
    let drawn = frame_at(&state, 120, 40, Some(Clock::Fixed(FORK_NOW)));
    app.close();
    let text = drawn.join("\n");
    assert!(text.contains("╰ y merge  esc cancel  ⏎ see the "), "{text}");
    assert!(!text.contains("[ Cancel ]"), "{text}");
    let mut out = String::from(
        "# fix-preview-verb-120x40.txt: 120x40, drawn by `view::view` from a real\n\
         # AppState over a real repository (`main` three commits, `feature` forked off\n\
         # the first with two of its own, `feature` checked out); cursor on `main`'s\n\
         # row, `m` pressed through `update`, the real plan landed\n\
         # (crates/graft/tests/it/preview.rs, capture_fix_preview_verb_for_docs_shots).\n\
         # TestBackend character grid: colour ships unverified (CLAUDE.md rule 5).\n\
         #\n\
         # What right looks like:\n\
         #  - The merge box (ease-pass 10); its bottom border reads\n\
         #    `y merge  esc cancel  ⏎ see the N commits`.\n\
         #  - No buttons row, no `m merge` on the footer.\n\
         #\n\n",
    );
    for line in &drawn {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/shots/m6/fix-preview-verb-120x40.txt");
    let checked = graft_fixtures::check(&path, &out);
    assert!(checked.is_ok(), "{}", checked.expect_err("it drifted"));
}

/// `m m` on the real path: the second `m` confirms, the cursor never moved.
#[test]
fn m_again_merges_through_the_real_path() {
    let scratch = forked();
    let before = git(scratch.path(), &["rev-parse", "HEAD"]);
    let (app, state) = App::open_with_clock(
        scratch.path(),
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let state = preview_opened_by(&app, state, 'm');
    let state = app.key(&state, KeyCode::Char('m'));
    assert!(state.preview.is_none(), "the second `m` confirmed");
    let state = app.settle_write(state, "`git merge`");
    drop(state);
    app.close();
    assert_ne!(
        git(scratch.path(), &["rev-parse", "HEAD"]),
        before,
        "HEAD moved"
    );
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
    graft_ops::journal::Journal::open(&repo.join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl")))
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

    let (app, state) = App::open_with_clock(
        repo,
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
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
        graft::update::Status::Done { what: line, .. } => line.clone(),
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
/// Regenerate on purpose: `MWR_UPDATE_SHOTS=1 cargo test -p mergewright --test it preview::capture_time_machine_for_docs_shots`.
#[test]
fn capture_time_machine_for_docs_shots() {
    let scratch = three_to_stage();
    let repo = scratch.path();
    // `three_to_stage` commits here and now, so the id is different every run.
    // The capture shows it, so it is zeroed out before the file is compared —
    // otherwise no two runs could ever agree (m5-history-and-ghost-preview/40).
    let base = git(repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let (app, state) = App::open(repo);
    let state = stage_all_three(&app, state);
    let state = journal_at_the_oldest_row(&app, &state);
    let state = app.key(&state, KeyCode::Enter);
    let state = planned_preview(&app, state);
    let mut drifted = Vec::new();

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
             # asserted). Not a pty capture: ratatui's TestBackend, a character grid\n\
             # with no colour attributes \u{2014} CLAUDE.md rule 5, colour ships\n\
             # unverified. The scratch repository's temporary path is replaced by\n\
             # `/code/api`, and the base commit's id \u{2014} invented by this run, so\n\
             # different every time \u{2014} is zeroed to `0000000`; nothing else is\n\
             # touched.\n\
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
            out.push_str(line);
            out.push('\n');
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots")
            .join(format!("m5-ui-preview-time-machine-{label}.txt"));
        if let Err(why) =
            graft_fixtures::check(&path, &crate::captures::without_invented_id(&out, &base))
        {
            drifted.push(why);
        }
    }
    app.close();
    assert!(drifted.is_empty(), "{}", drifted.join("\n\n"));
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
    let (app, state) = App::open_with_clock(
        scratch.path(),
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
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
/// `redesign` is what puts the distance there: sixty commits newer than
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
    // Sixty, not twenty-five: at 160x48 the whole fixture fitted on one
    // screen, so the capture that exists to show a *scrolled* viewport was
    // not scrolled at that size (m5/20 item 5).
    for n in 0..60 {
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
    preview_scrolled_at(app, state, 80, 24)
}

/// The same, at a given screen size.
///
/// The size matters to the result, not just to the drawing: `history.top`,
/// the graph's row budget and the layout all come off `state.size`, so a
/// session scrolled at one size and photographed at another is a picture of
/// neither (m5/20 item 5).
fn preview_scrolled_at(app: &App, state: AppState, columns: u16, rows: u16) -> AppState {
    let mut state = opened(app, state);
    // A screen the fixture is taller than, without the fixture having to be
    // a hundred real commits: the harness opens at 120x40.
    state = app.resize(&state, columns, rows);
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
    let (app, state) = App::open_with_clock(
        scratch.path(),
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let state = preview_scrolled_into_history(&app, state);

    let drawn = frame(&state, 80, 24).join("\n");
    assert!(
        drawn.contains("cut release 1.0"),
        "the row the cursor is on is on screen:\n{drawn}"
    );
    assert!(
        !drawn.contains("redesign step 59"),
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
/// Regenerate on purpose: `MWR_UPDATE_SHOTS=1 cargo test -p mergewright --test it preview::capture_scrolled_preview_for_docs_shots`.
#[test]
fn capture_scrolled_preview_for_docs_shots() {
    let scratch = left_behind();
    let mut drifted = Vec::new();
    let mut tops: Vec<(&str, usize)> = Vec::new();

    for (label, columns, rows) in [("80x24", 80u16, 24u16), ("160x48", 160, 48)] {
        // A fresh session per size, scrolled at that size. One state
        // photographed at two sizes gave one `history.top` for two
        // pictures, and the caption was then true of at most one of them
        // (m5/20 item 5).
        let (app, state) = App::open_with_clock(
            scratch.path(),
            graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
        );
        let state = preview_scrolled_at(&app, state, columns, rows);
        let top = state.history.as_ref().expect("a history").top;
        tops.push((label, top));
        let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(LEFT_BEHIND_NOW)));
        let mut out = scrolled_caption(label, columns, rows, top);
        out.push('\n');
        for line in &drawn {
            out.push_str(line);
            out.push('\n');
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots")
            .join(format!("m5-ui-preview-scrolled-{label}.txt"));
        if let Err(why) = graft_fixtures::check(&path, &out) {
            drifted.push(why);
        }
        drop(state);
        app.close();
    }

    // The point of a session per size. A taller screen reaches the same
    // commit with a different viewport top, so one number cannot be true of
    // both captions \u2014 which is exactly what the committed pair claimed
    // before m5/20 item 5.
    assert!(
        tops[0].1 != tops[1].1,
        "the two captures are scrolled to different places, so each caption \
         needs its own `history.top`: {tops:?}"
    );
    assert!(drifted.is_empty(), "{}", drifted.join("\n\n"));
}

/// The caption on one scrolled-preview capture: `top` is that session's own
/// `history.top`, so the caption can never claim a viewport the frame does
/// not have.
fn scrolled_caption(label: &str, columns: u16, rows: u16, top: usize) -> String {
    format!(
        "# m5-ui-preview-scrolled-{label}.txt: {columns}x{rows}, drawn by `view::view`\n\
         # from a real AppState. A real repository: `old-release` cut at the base,\n\
         # three commits on `main`, `topic` forked off its tip with two of its own,\n\
         # and `redesign` \u{2014} sixty commits newer than all of them \u{2014} pushing\n\
         # `old-release` far down the graph. Opened over the real job pool, the\n\
         # cursor walked down to `old-release`'s row with `j` **at this screen\n\
         # size** (so the viewport really is scrolled: `history.top` is {top} here,\n\
         # not 0 \u{2014} it differs by screen, which is why each capture has its own\n\
         # session and its own number), then `r` pressed through `update` and the\n\
         # real `graft_ops::preview::plan` run (crates/graft/tests/it/preview.rs,\n\
         # capture_scrolled_preview_for_docs_shots, asserted). Not a pty capture:\n\
         # ratatui's TestBackend, a character grid with no colour attributes in it\n\
         # \u{2014} CLAUDE.md rule 5, colour ships unverified. The scratch repository's\n\
         # temporary path is replaced by `/code/api`; nothing else is touched.\n\
         #\n\
         # What right looks like:\n\
         #  - The rows on screen are the rows the user was looking at. `cut release\n\
         #    1.0` \u{2014} the row the cursor was on when `r` was pressed \u{2014} is visible,\n\
         #    and the newest commits (`redesign step 59` and its neighbours, a\n\
         #    screen or more above) are **not**. Before ticket 17 this capture\n\
         #    showed exactly those rows instead, and the user's own were off-screen.\n\
         #  - The five predicted rows sit right above `cut release 1.0`: hollow\n\
         #    node glyphs, dashed edges, `\u{2014}` in place of a short sha and an age.\n\
         #    They are `main`'s three commits and `topic`'s two, replayed.\n\
         #  - **An `after rebase` label on its own row above them**, and the box's\n\
         #    ref line reads `topic e7e39a7 \u{2192} 5 new commits`. Before m5/20 the\n\
         #    block had no label and no separator, the five subjects were word for\n\
         #    word the five real rows above them, and the line said `\u{2192} ghost`\n\
         #    \u{2014} a word the interface never defines. The first available reading\n\
         #    was `these commits are in my history twice`, which is the confusion\n\
         #    M2 exists to prevent.\n\
         #  - `topic`'s badge has left its own row and sits on the newest of them,\n\
         #    directly under the label.\n\
         #  - The highlighted row is `cut release 1.0`: the commit the key was\n\
         #    pressed on, not whichever row happens to sit where the box's choice\n\
         #    cursor points. (Highlight is a colour, so the grid cannot show it \u{2014}\n\
         #    unverified by construction, rule 5.)\n\
         #  - The box is docked above the footer, every interior row bordered, and\n\
         #    `[ Cancel ]` comes first with the cursor on it (ADR-0038 \u{a7}1).\n\
         #\n"
    )
}

/// m5/20 item 4: the predicted block is labelled, and the box's ref line
/// counts commits instead of naming a word the interface never defines.
///
/// Against the real drawn frame, not the wording functions: the label is a
/// **row**, and a row that is built but windowed off the screen is not a
/// label the user can read.
#[test]
fn the_predicted_block_is_labelled_and_named_in_the_users_words() {
    let scratch = left_behind();
    let (app, state) = App::open_with_clock(
        scratch.path(),
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let state = preview_scrolled_into_history(&app, state);

    let drawn = frame(&state, 80, 24).join("\n");
    assert!(
        drawn.contains("after rebase"),
        "the predicted block names the outcome on a row of its own:\n{drawn}"
    );
    assert!(
        drawn.contains("5 new commits"),
        "and the box counts the commits rather than saying `ghost`:\n{drawn}"
    );
    assert!(
        !drawn.contains("ghost"),
        "and `ghost` is nowhere on the screen:\n{drawn}"
    );
    // The label is a separator, not a replacement: the rows it heads are
    // still there, and still the ones the cursor was next to.
    assert!(
        drawn.contains("cut release 1.0") && drawn.contains("start the topic"),
        "the label did not push the picture off the screen:\n{drawn}"
    );
    app.close();
}

/// The cue the user said already works (m5/20 item 4): `topic`'s badge
/// leaves its own row and lands on the newest predicted commit. The label
/// row must not take it, and must not come between the two.
#[test]
fn the_branch_badge_still_lands_on_the_newest_predicted_row() {
    let scratch = left_behind();
    let (app, state) = App::open_with_clock(
        scratch.path(),
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let state = preview_scrolled_into_history(&app, state);

    let drawn = frame(&state, 80, 24);
    let label = drawn
        .iter()
        .position(|line| line.contains("after rebase"))
        .unwrap_or_else(|| panic!("no label row:\n{}", drawn.join("\n")));
    let badge = drawn
        .iter()
        .position(|line| line.contains("[◆ topic]"))
        .unwrap_or_else(|| panic!("no topic badge:\n{}", drawn.join("\n")));
    assert_eq!(
        badge,
        label + 1,
        "the badge sits on the row directly under the label:\n{}",
        drawn.join("\n")
    );
    assert!(
        drawn[badge].contains("finish the topic"),
        "and that row is the newest predicted commit:\n{}",
        drawn.join("\n")
    );
    app.close();
}

/// **`forecast` is our word too**, and `(forecast)` broke a second rule:
/// `02-keymap.md` §2.1, extended 2026-09-21 — a trailing parenthesis
/// carries a key, not a word, anywhere the user might press what is inside
/// it. The preview's two lines said `no conflicts expected (forecast)` and
/// `may conflict (forecast): …`; `expected` and `may` already say it is a
/// prediction.
///
/// Swept the same way as `ghost` rather than asserted on the two lines,
/// for the same reason: a spot check only ever catches the instance
/// somebody already found.
#[test]
fn no_user_facing_string_says_forecast() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut offenders = Vec::new();
    for crate_name in ["graft/src", "graft-ui/src"] {
        walk_rust(&root.join(crate_name), &mut |path, line_no, line| {
            for literal in string_literals(line) {
                if literal.to_ascii_lowercase().contains("forecast") {
                    offenders.push(format!("{}:{line_no}: {literal}", path.display()));
                }
            }
        });
    }
    assert!(
        offenders.is_empty(),
        "these strings could reach the screen and say `forecast`:\n{}",
        offenders.join("\n")
    );
}

/// m5/20 item 4, the deeper point: **`ghost` is our word.** A sweep of
/// every string literal in the two crates that draw, not a spot check of
/// the one line the user happened to catch.
///
/// Comments, attributes and test modules are skipped: the word belongs in
/// code, tickets and ADRs — `PreviewState::ghost_index` and
/// `RowPaint::Ghost` keep it. What it may never be is something the user
/// reads.
#[test]
fn no_user_facing_string_says_ghost() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut offenders = Vec::new();
    for crate_name in ["graft/src", "graft-ui/src"] {
        walk_rust(&root.join(crate_name), &mut |path, line_no, line| {
            for literal in string_literals(line) {
                if literal.to_ascii_lowercase().contains("ghost") {
                    offenders.push(format!("{}:{line_no}: {literal}", path.display()));
                }
            }
        });
    }
    assert!(
        offenders.is_empty(),
        "these strings could reach the screen and say `ghost`:\n{}",
        offenders.join("\n")
    );
}

/// Every `.rs` file under `dir` that is not a test module, line by line,
/// with comment lines, attribute lines and developer assertions already
/// dropped.
///
/// A `debug_assert!`'s message is read by whoever broke the invariant, not
/// by the user \u2014 it never reaches a frame. Its continuation lines are
/// skipped along with its first, which is why this tracks the macro across
/// lines rather than matching one.
fn walk_rust(dir: &Path, visit: &mut impl FnMut(&Path, usize, &str)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        panic!("{} is readable", dir.display());
    };
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "tests") {
                continue;
            }
            walk_rust(&path, visit);
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "rs")
            || path.file_name().is_some_and(|name| name == "tests.rs")
        {
            continue;
        }
        let body = std::fs::read_to_string(&path).expect("a readable source file");
        let mut in_tests = false;
        let mut in_assert = false;
        for (index, line) in body.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("mod tests {") || trimmed.starts_with("mod tests;") {
                in_tests = true;
            }
            let opens_assert = [
                "debug_assert",
                "assert!",
                "assert_eq!",
                "panic!",
                "unreachable!",
            ]
            .iter()
            .any(|macro_name| trimmed.starts_with(macro_name));
            let was_in_assert = in_assert;
            if opens_assert {
                in_assert = true;
            }
            if in_assert && trimmed.ends_with(");") {
                in_assert = false;
            }
            if in_tests
                || was_in_assert
                || opens_assert
                || trimmed.starts_with("//")
                || trimmed.starts_with("#[")
            {
                continue;
            }
            visit(&path, index + 1, line);
        }
    }
}

/// The double-quoted string literals in one line of Rust, escapes left as
/// written. Good enough for a word sweep: it is looking for a word, not
/// parsing the language.
fn string_literals(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '"' {
            continue;
        }
        let mut literal = String::new();
        while let Some(ch) = chars.next() {
            match ch {
                '\\' => {
                    chars.next();
                }
                '"' => break,
                _ => literal.push(ch),
            }
        }
        found.push(literal);
    }
    found
}
