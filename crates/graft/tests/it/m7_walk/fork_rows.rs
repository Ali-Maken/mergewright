//! Each fork's fate on screen (m8-polish-and-ship/14c, ADR-0059 §2): the
//! put-in's preview box and the rewrite editor's AFTER side name the fork
//! that follows and the one that stays, and a stop inside a fork's section
//! names the fork on the banner. Rule 6: the captures are for the user's
//! eye; colour unverified.

use std::path::Path;

use graft::update::AppState;
use graft_core::event::{KeyCode, KeyPress, Modifiers};

use super::rebase_line::{frame, git, open};
use super::rewrite_editor::{capital_r, place_on, replanned};

/// `file` set to `text`, committed as `subject`.
fn write_commit(repo: &Path, file: &str, text: &str, subject: &str, hour: u8) {
    std::fs::write(repo.join(file), text).unwrap_or_else(|why| panic!("write {file}: {why}"));
    git(repo, &["add", "-A"], hour);
    git(repo, &["commit", "-q", "-m", subject], hour);
}

/// `main` (m1) ← `feature-a`: part 1 (`one.txt`), part 2, part 3.
/// `feature-b` (b1, its own file) forks off part 1: it replays clean and
/// follows. `feature-c` (c1, an edit to `one.txt`) forks off part 2: a
/// change to part 1's file makes it conflict, so it stays. HEAD on
/// `feature-a`, nothing staged.
fn two_forks_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let repo = dir.path();
    git(repo, &["init", "-q", "-b", "main"], 0);
    write_commit(repo, "readme.txt", "m1\n", "m1", 0);
    git(repo, &["switch", "-q", "-c", "feature-a"], 1);
    write_commit(repo, "one.txt", "part 1\n", "feature a: part 1", 1);
    write_commit(repo, "two.txt", "part 2\n", "feature a: part 2", 2);
    write_commit(repo, "three.txt", "part 3\n", "feature a: part 3", 3);
    git(repo, &["switch", "-q", "-c", "feature-b", "feature-a~2"], 4);
    write_commit(repo, "b.txt", "b1\n", "b1", 4);
    git(repo, &["switch", "-q", "-c", "feature-c", "feature-a~1"], 5);
    write_commit(repo, "one.txt", "part 1, by c\n", "c1", 5);
    git(repo, &["switch", "-q", "feature-a"], 6);
    git(repo, &["commit-graph", "write", "--reachable"], 6);
    dir
}

fn short(repo: &Path, rev: &str) -> String {
    git(repo, &["rev-parse", "--short=7", rev], 0)
        .trim()
        .to_owned()
}

/// The two fork rows, in order, at the indent of the `branches:` rows.
fn assert_fork_rows(body: &str, repo: &Path, follows_from: &str) {
    let lines: Vec<&str> = body.lines().collect();
    let follows = format!("feature-b follows \u{b7} 1 commit from {follows_from}");
    let stays = format!(
        "feature-c stays on {} \u{b7} would conflict in one.txt",
        short(repo, "feature-a~1")
    );
    let at = |words: &str| {
        lines
            .iter()
            .position(|line| line.contains(words))
            .unwrap_or_else(|| panic!("{words:?} in {body}"))
    };
    assert!(at(&follows) < at(&stays), "follows first: {body}");
    assert!(
        !body.contains("left behind"),
        "every branch off the line is a fork here: {body}"
    );
}

/// `a` on part 1 with a fix to `one.txt` staged: `feature-b` follows from
/// part 1, `feature-c` stays on part 2.
#[test]
fn the_put_in_box_names_each_fork() {
    let dir = two_forks_repo();
    let repo = dir.path();
    std::fs::write(repo.join("one.txt"), "part 1, fixed\n")
        .unwrap_or_else(|why| panic!("write: {why}"));
    git(repo, &["add", "one.txt"], 7);
    let (app, state) = open(repo, 187, 44);
    let state = app.pump(state, "the working tree", |state| {
        state.working_tree.is_some()
    });
    let state = place_on(&app, state, repo, "feature-a~2");
    let state = app.key(&state, KeyCode::Char('a'));
    let state = app.pump(state, "the put-in's preview", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let body = frame(&state, 187, 44);
    crate::captures::check_shot("m8/fork-follows-preview-187x44.txt", &body);
    assert_fork_rows(&body, repo, &short(repo, "feature-a~2"));
    drop(state);
    app.close();
}

/// `R`, then `d` on part 1: `feature-b`'s fork point is dropped, so it
/// follows onto the row below; `feature-c`'s edit to `one.txt` now meets a
/// line without it, so it stays.
#[test]
fn the_rewrite_editor_names_each_fork() {
    let dir = two_forks_repo();
    let repo = dir.path();
    let (app, state) = open(repo, 187, 44);
    let state = place_on(&app, state, repo, "HEAD");
    let mut route = Vec::new();
    let state = capital_r(&app, &state, &mut route, 187, 44);
    let state = app.key(&state, KeyCode::Char('j'));
    let state = app.key(&state, KeyCode::Char('j'));
    let state = replanned(
        &app,
        &state,
        &mut route,
        KeyPress {
            code: KeyCode::Char('d'),
            modifiers: Modifiers::NONE,
        },
        "d",
        187,
        44,
    );
    let body = frame(&state, 187, 44);
    crate::captures::check_shot("m8/fork-follows-rewrite-editor-187x44.txt", &body);
    assert_fork_rows(&body, repo, &short(repo, "feature-a~2"));
    drop(state);
    app.close();
}

/// A rebase of Graft's todo shape, stopped by hand (`edit`) on b1 inside
/// `feature-b`'s section, then Graft opened on it: the banner names the
/// fork and where it leaves the line. The stop is set up with git, not
/// through the app — a following fork's replay is forecast clean, so the
/// app never stops there by itself.
#[test]
fn a_stop_inside_a_fork_names_it_on_the_banner() {
    let dir = two_forks_repo();
    let repo = dir.path();
    let id = |rev: &str| git(repo, &["rev-parse", rev], 0).trim().to_owned();
    let todo = format!(
        "pick {} feature a: part 1\nlabel graft-fork-1\nedit {} b1\n\
         update-ref refs/heads/feature-b\nreset graft-fork-1\n\
         pick {} feature a: part 2\npick {} feature a: part 3\n",
        id("feature-a~2"),
        id("feature-b"),
        id("feature-a~1"),
        id("feature-a"),
    );
    let todo_path = repo.join(".git").join("graft-test-todo");
    std::fs::write(&todo_path, todo).unwrap_or_else(|why| panic!("write the todo: {why}"));
    let editor = format!("sequence.editor=cp {}", todo_path.display());
    git(repo, &["-c", &editor, "rebase", "-i", "main"], 7);

    let (app, state) = open(repo, 187, 44);
    let banner = format!(
        "REBASING  step 2 of 4 \u{b7} edit {} b1 \u{b7} feature-b, forked at {}",
        short(repo, "feature-b"),
        short(repo, "feature-a~2"),
    );
    let state: AppState = app.pump(state, "the banner", |state| {
        frame(state, 187, 44).contains("REBASING  step")
    });
    let body = frame(&state, 187, 44);
    assert!(body.contains(&banner), "{banner:?} in {body}");
    crate::captures::check_shot("m8/fork-follows-banner-187x44.txt", &body);
    drop(state);
    app.close();
}
