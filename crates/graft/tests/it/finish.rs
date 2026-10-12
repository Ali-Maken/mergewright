//! The finish summary against a real merge (m6-conflicts/19, the app half
//! built by m6-conflicts/34): `c` on the resolved merge's banner, the
//! summary read from the repository, and `y` committing through the real
//! `git merge --continue`. What landed is read back from git, never from
//! the status line.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::update::{AppEvent, AppState, Command, Product, Status, update};
use graft_core::event::{KeyCode, KeyPress};

use crate::app_harness::{App, git, git_as_graft, press_for_commands};
use crate::conflict_interim::{
    mark_resolved, open_changes_pane, stopped_on_the_conflict, tab_back_to_the_banner,
};

fn write(repo: &Path, name: &str, body: &str) {
    std::fs::write(repo.join(name), body).expect("the scratch repo is writable");
}

/// `feature` and `main` both rewrote `shared.rs` and `notes.md`, so merging
/// `feature` stops on those two; `feature` also changed one line of
/// `README.md`, which git merges on its own.
fn about_to_merge() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    // The merge commit is the app's `git merge --continue`: the fixture's
    // own name, so no machine's identity decides whether it can commit.
    git(repo, &["config", "user.name", "Graft Fixture"]);
    git(repo, &["config", "user.email", "fixture@example.invalid"]);
    write(repo, "shared.rs", "fn main() {}\n");
    write(repo, "notes.md", "the plan\n");
    write(
        repo,
        "README.md",
        "# Project\n\nline one\nline two\nline three\n",
    );
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-q", "-m", "the shared files"]);
    git(repo, &["switch", "--quiet", "-c", "feature"]);
    write(repo, "shared.rs", "fn main() { theirs() }\n");
    write(repo, "notes.md", "their plan\n");
    write(
        repo,
        "README.md",
        "# Project\n\nline one\nline TWO\nline three\n",
    );
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-q", "-m", "the feature's take"]);
    git(repo, &["switch", "--quiet", "main"]);
    write(repo, "shared.rs", "fn main() { ours() }\n");
    write(repo, "notes.md", "our plan\n");
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-q", "-m", "main's take"]);
    scratch
}

/// The merge started with `m`, both files resolved and marked, back on the
/// banner, `c` pressed and the summary read.
fn summary_open(app: &App, state: AppState, repo: &Path) -> AppState {
    let state = stopped_on_the_conflict(app, state);
    write(repo, "shared.rs", "fn main() { both() }\n");
    write(repo, "notes.md", "the agreed plan\n");
    let state = app.key(&state, KeyCode::Tab);
    let state = open_changes_pane(app, state);
    let state = mark_resolved(app, state, "notes.md");
    let state = mark_resolved(app, state, "shared.rs");
    let state = tab_back_to_the_banner(app, state);
    let state = app.key(&state, KeyCode::Char('c'));
    assert!(
        state.finish.is_some(),
        "`c` on the merge opens the summary: {:?}",
        state.status
    );
    assert!(state.operating.is_none(), "and writes nothing");
    app.pump(state, "the finish summary", |state| {
        state.finish.as_ref().is_some_and(|open| open.job.is_none())
    })
}

/// `y` (or whatever `key` is) and the `--continue` it runs, to its answer.
fn committed(app: &App, state: &AppState, key: KeyPress) -> AppState {
    let state = app.press(state, key);
    assert!(
        state.operating.is_some(),
        "the key commits: {:?}",
        state.finish.as_ref().map(|open| &open.notice)
    );
    app.pump(state, "the continue", |state| state.operating.is_none())
}

/// The last commit's whole message, as git stored it.
fn last_message(repo: &Path) -> String {
    git(repo, &["log", "-1", "--format=%B"])
}

fn typed(app: &App, state: AppState, text: &str) -> AppState {
    text.chars().fold(state, |state, character| {
        app.key(&state, KeyCode::Char(character))
    })
}

#[test]
fn an_edited_subject_commits_with_that_message() {
    let scratch = about_to_merge();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);

    let state = app.key(&state, KeyCode::Tab);
    let subject_len = state.finish.as_ref().map_or(0, |open| open.subject.len());
    let state = (0..subject_len).fold(state, |state, _| app.key(&state, KeyCode::Backspace));
    let state = typed(&app, state, "Merge feature: the agreed plan");
    let state = committed(&app, &state, KeyPress::plain(KeyCode::Enter));

    assert_eq!(
        last_message(repo).lines().next(),
        Some("Merge feature: the agreed plan")
    );
    assert!(
        !repo.join(".git/MERGE_HEAD").exists(),
        "git finished the merge"
    );
    drop(state);
    app.close();
}

#[test]
fn a_hash_line_typed_in_the_body_lands() {
    let scratch = about_to_merge();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);

    let state = app.key(&state, KeyCode::Tab);
    let state = app.key(&state, KeyCode::Tab);
    let state = typed(&app, state, "#123 fixes login");
    let state = committed(&app, &state, KeyPress::control(KeyCode::Enter));

    let message = last_message(repo);
    assert!(
        message.lines().any(|line| line == "#123 fixes login"),
        "the typed line is in the commit:\n{message}"
    );
    drop(state);
    app.close();
}

/// `Handover::edit_message`'s own write/open/read-back
/// (`event_loop::outside::edit_message_in`) minus the terminal suspend,
/// which needs a real terminal: the fixture's `$EDITOR` **run as a
/// process** through the app's own `editor_command`.
///
/// The editor is the one git names with the environment's editor variables
/// taken out — git ranks `GIT_EDITOR` first, so a developer's own would
/// otherwise replace the fixture's (as `m5_walk::gestures::fixture_editor`).
fn run_the_fixture_editor(repo: &Path, message: &str) -> String {
    let asked = std::process::Command::new("git")
        .args(["var", "GIT_EDITOR"])
        .current_dir(repo.join(".git"))
        .env_remove("GIT_EDITOR")
        .env_remove("VISUAL")
        .env_remove("EDITOR")
        .output()
        .expect("git var runs");
    assert!(asked.status.success(), "git names an editor");
    let editor = String::from_utf8_lossy(&asked.stdout).trim().to_owned();
    let app_dir = repo
        .join(".git")
        .join(graft_core::on_disk::Generation::Graft.dir());
    graft::event_loop::outside::edit_message_in(&app_dir, message, |file| {
        let status =
            graft::event_loop::outside::editor_command(&editor, file, None, Some(repo)).status();
        match status {
            Ok(status) if status.success() => Ok(()),
            other => Err(format!("{editor}: {other:?}")),
        }
    })
    .expect("the editor ran and left the file")
}

/// `Ctrl+E` hands the message to `$EDITOR`: a real process, the fixture's
/// `core.editor`, a script that appends `#123 fixes login`. The harness'
/// job pool cannot run a handover — that is the loop's, never a job's — so
/// the test takes the command `update` issued, runs the editor the way the
/// loop's `Handover::edit_message` does, and feeds back what it wrote. Only
/// the terminal suspend around the process is not exercised.
#[test]
fn a_hash_line_from_the_editor_lands() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = about_to_merge();
    let repo = scratch.path();
    let script = repo.join(".git/fixture-editor.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\nfor f in \"$@\"; do :; done\nprintf '\\n#123 fixes login\\n' >> \"$f\"\n",
    )
    .expect("writable");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    git(
        repo,
        &[
            "config",
            "--local",
            "core.editor",
            script.to_str().expect("a UTF-8 scratch path"),
        ],
    );
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);

    let (state, commands) = press_for_commands(&state, KeyPress::control(KeyCode::Char('e')));
    let Some(Command::EditCommitMessage { id, message }) = commands
        .iter()
        .find(|command| matches!(command, Command::EditCommitMessage { .. }))
    else {
        panic!("`Ctrl+E` hands the message over: {commands:?}");
    };
    let edited = run_the_fixture_editor(repo, message);
    assert!(
        edited.lines().any(|line| line == "#123 fixes login"),
        "the editor ran: {edited:?}"
    );
    let (state, _) = update(
        &state,
        AppEvent::Produced(*id, Product::ExternalEditor(Ok(edited))),
    );
    let body = state
        .finish
        .as_ref()
        .map(|open| open.body.text())
        .expect("the summary is still open");
    assert!(
        body.lines().any(|line| line == "#123 fixes login"),
        "the editor's line is in the body: {body:?}"
    );

    let state = committed(&app, &state, KeyPress::control(KeyCode::Enter));
    let message = last_message(repo);
    assert!(
        message.lines().any(|line| line == "#123 fixes login"),
        "the editor's line is in the commit:\n{message}"
    );
    drop(state);
    app.close();
}

#[test]
fn the_summary_shows_no_git_comment_lines() {
    let scratch = about_to_merge();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);

    let merge_msg = std::fs::read_to_string(repo.join(".git/MERGE_MSG")).expect("git wrote it");
    assert!(
        merge_msg.lines().any(|line| line.starts_with('#')),
        "the fixture's MERGE_MSG carries git's comment block:\n{merge_msg}"
    );
    let open = state.finish.as_ref().expect("open");
    assert_eq!(
        open.subject.text(),
        merge_msg.lines().next().expect("a first line"),
        "the subject is git's first line"
    );
    let drawn = crate::captures::frame(&state, 120, 40);
    let message_lines: Vec<&String> = drawn
        .iter()
        .filter(|line| line.trim_start().starts_with('#'))
        .collect();
    assert!(
        message_lines.is_empty(),
        "no drawn line starts with `#`:\n{}",
        drawn.join("\n")
    );
    drop(state);
    app.close();
}

#[test]
fn a_hook_refusal_keeps_the_summary_open() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = about_to_merge();
    let repo = scratch.path();
    let hook = repo.join(".git/hooks/commit-msg");
    std::fs::create_dir_all(hook.parent().expect("a hooks dir")).expect("writable");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho 'commit-msg: a ticket number is required' >&2\nexit 1\n",
    )
    .expect("writable");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);
    let state = app.key(&state, KeyCode::Tab);
    let state = typed(&app, state, " (kept)");
    let subject = state
        .finish
        .as_ref()
        .map(|open| open.subject.text().to_owned())
        .expect("open");

    let state = committed(&app, &state, KeyPress::plain(KeyCode::Enter));
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
    assert_eq!(open.committing, None);
    assert!(
        repo.join(".git/MERGE_HEAD").exists(),
        "nothing was committed"
    );
    drop(state);
    app.close();
}

#[test]
fn done_lands_on_the_merge_commit() {
    let scratch = about_to_merge();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);

    let state = committed(&app, &state, KeyPress::plain(KeyCode::Char('y')));
    let state = app.pump(state, "the landing the finish owes", |state| {
        state.in_progress.is_none() && !state.land_on_head
    });
    assert!(state.finish.is_none(), "the summary closed");
    assert!(state.graph_has_focus(), "the graph has the keyboard");
    let head = git(repo, &["rev-parse", "HEAD"]);
    let selected = state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
        .map(|row| row.commit.id.to_string());
    assert_eq!(
        selected.as_deref(),
        Some(head.trim()),
        "the cursor is on HEAD"
    );
    let parents = git(repo, &["rev-list", "--parents", "-1", "HEAD"]);
    assert_eq!(
        parents.split_whitespace().count(),
        3,
        "a merge commit: itself and two parents: {parents}"
    );
    assert_eq!(
        state.status,
        Status::done("merged feature into main"),
        "drawn as `merged feature into main ✓`"
    );
    drop(state);
    app.close();
}

mod captures;
mod choices;
