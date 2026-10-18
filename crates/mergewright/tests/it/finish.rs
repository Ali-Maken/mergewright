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

use mergewright::update::{AppEvent, AppState, Command, Product, Status, update};
use mergewright_core::event::{KeyCode, KeyPress};

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
fn about_to_merge() -> mergewright_fixtures::Scratch {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
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

/// The merge review's read, out since `c` (or `y`), landed: the review
/// opens only now (ease-pass/05s), so a test asserts on it after this.
pub(crate) fn review_landed(app: &App, state: AppState) -> AppState {
    app.pump(state, "the finish summary", |state| {
        state.finish.as_ref().is_some_and(|open| open.job.is_none())
    })
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
    assert!(state.operating.is_none(), "`c` writes nothing");
    let state = review_landed(app, state);
    assert!(
        state.finish.is_some(),
        "`c` on the merge opens the summary: {:?}",
        state.status
    );
    state
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

/// `MERGE_MSG` as `git commit --no-edit` would commit it: the comment
/// lines gone, trailing blank lines trimmed.
fn merge_msg_minus_comments(repo: &Path) -> String {
    let merge_msg = std::fs::read_to_string(repo.join(".git/MERGE_MSG")).expect("git wrote it");
    assert!(
        merge_msg.lines().any(|line| line.starts_with('#')),
        "the fixture's MERGE_MSG carries git's comment block:\n{merge_msg}"
    );
    let kept: Vec<&str> = merge_msg
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect();
    kept.join("\n").trim_end().to_owned()
}

/// ease-pass/12 decision 5: the message is read-only and `y` commits git's
/// own — `git log -1 --format=%B` is `MERGE_MSG` minus its comments.
#[test]
fn y_commits_gits_message_minus_its_comments() {
    let scratch = about_to_merge();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);
    let expected = merge_msg_minus_comments(repo);

    let state = committed(&app, &state, KeyPress::plain(KeyCode::Char('y')));
    assert!(
        !repo.join(".git/MERGE_HEAD").exists(),
        "git finished the merge"
    );
    assert_eq!(last_message(repo).trim_end(), expected);
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
    let drawn = crate::captures::frame(&state, 120, 40);
    let first = merge_msg.lines().next().expect("a first line");
    assert!(
        drawn
            .iter()
            .any(|line| line.contains(&format!("Message  {first}"))),
        "the message row is git's first line:\n{}",
        drawn.join("\n")
    );
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

    let state = committed(&app, &state, KeyPress::plain(KeyCode::Char('y')));
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
        .and_then(mergewright::history::History::selected)
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
        Status::done("Merged feature into main."),
        "drawn as `✓ Merged feature into main.` (mock :197)"
    );
    drop(state);
    app.close();
}

mod captures;
mod choices;
mod landing;
