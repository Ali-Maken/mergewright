//! The blame job against real git (review m7-20e-20g Major 3): `^`'s diff
//! read across a rename, the refusals, and the error mapping.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_fixtures::git::{FIXED_DATE, run_at};
use graft_git::blame::{BlameError, Rev};

use super::blame::blame_failure;
use super::*;
use crate::update::place::{BlameAnswer, BlameFailure, BlameFrom, BlameLoad, open_blame};
use crate::update::{AppEvent, AppState, Command, Product, Status, update};

fn git(repo: &Path, args: &[&str]) -> String {
    let out = run_at(repo, args, FIXED_DATE).unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    String::from_utf8(out).expect("git speaks utf-8 here")
}

fn oid(repo: &Path, rev: &str) -> Oid {
    Oid::from_hex(git(repo, &["rev-parse", rev]).trim()).expect("an oid")
}

/// Commit 1: `a.rs` = `l1`…`l10`. Commit 2: `git mv a.rs b.rs`, two lines
/// inserted after `l2`, `l6` → `L6`.
fn renamed_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let repo = dir.path();
    git(repo, &["init", "-q", "-b", "main"]);
    let one = "l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nl10\n";
    std::fs::write(repo.join("a.rs"), one).expect("write a.rs");
    git(repo, &["add", "a.rs"]);
    git(repo, &["commit", "-q", "-m", "one"]);
    git(repo, &["mv", "a.rs", "b.rs"]);
    let two = "l1\nl2\nn1\nn2\nl3\nl4\nl5\nL6\nl7\nl8\nl9\nl10\n";
    std::fs::write(repo.join("b.rs"), two).expect("write b.rs");
    git(repo, &["add", "b.rs"]);
    git(repo, &["commit", "-q", "-m", "two"]);
    dir
}

/// Run the job `commands` asks for, against `store`.
fn run_job(
    store: &RepoStore,
    commands: &[Command],
) -> (JobId, Result<Arc<BlameAnswer>, BlameFailure>) {
    let (job, request) = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadBlame(job, request) => Some((*job, request)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a blame job in {commands:?}"));
    (job, read_blame(store, request, &Cancel::new()))
}

fn deliver(
    state: &AppState,
    job: JobId,
    answer: Result<Arc<BlameAnswer>, BlameFailure>,
) -> AppState {
    update(state, AppEvent::Produced(job, Product::Blame(answer))).0
}

/// The line under the top place's cursor: its number and its text.
fn cursor_at(state: &AppState) -> (u32, String) {
    let blame = state.trail.on_screen().expect("a place on screen");
    let BlameLoad::Ready { blame: lines, .. } = &blame.body else {
        panic!("not ready: {:?}", blame.body);
    };
    let line = &lines.lines[blame.cursor];
    (
        line.final_line,
        String::from_utf8_lossy(&line.text).into_owned(),
    )
}

#[test]
fn caret_across_a_rename_lands_where_the_line_came_from() {
    let dir = renamed_repo();
    let repo = dir.path();
    let (one, two) = (oid(repo, "HEAD~1"), oid(repo, "HEAD"));
    let store = RepoStore::new(repo.to_path_buf(), None);

    // git's own word on line 8: it came from commit 1's `a.rs`.
    let porcelain = git(
        repo,
        &[
            "blame",
            "--porcelain",
            "-L",
            "8,8",
            &two.to_string(),
            "--",
            "b.rs",
        ],
    );
    assert!(
        porcelain.contains(&format!("previous {one} a.rs")),
        "git names commit 1's a.rs as the line's previous:\n{porcelain}"
    );

    let mut state = AppState::default();
    let commands = open_blame(
        &mut state,
        BlameFrom::Commit {
            commit: two,
            path: PathBuf::from("b.rs"),
            line: 8,
        },
    );
    let (job, answer) = run_job(&store, &commands);
    let state = deliver(&state, job, answer);
    assert_eq!(cursor_at(&state), (8, "L6".to_owned()));

    let (asked, commands) = update(
        &state,
        AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char('^')))),
    );
    let (job, answer) = run_job(&store, &commands);
    let parent = deliver(&asked, job, answer);
    let top = parent.trail.on_screen().expect("the older blame");
    assert_eq!(top.rev, Rev::Commit(one), "blames commit 1");
    assert_eq!(top.path, PathBuf::from("a.rs"), "at its path there");
    assert_eq!(
        cursor_at(&parent),
        (6, "l6".to_owned()),
        "where L6 came from"
    );
}

/// The job on the session fixture's `path` in the working tree, and the
/// words the place then says.
fn refused(path: &str) -> (BlameFailure, String) {
    let store = RepoStore::new(graft_fixtures::borrow(graft_fixtures::Shape::Session), None);
    let mut state = AppState::default();
    let commands = open_blame(
        &mut state,
        BlameFrom::WorkingTree {
            path: PathBuf::from(path),
            line: Some(1),
        },
    );
    let (job, answer) = run_job(&store, &commands);
    let failure = answer.clone().expect_err("git refused");
    let after = deliver(&state, job, answer);
    assert!(!after.trail.is_live(), "nothing opened");
    let Status::Sticky { reason, .. } = after.status else {
        panic!("the refusal in words: {:?}", after.status);
    };
    (failure, reason)
}

#[test]
fn the_blame_job_refuses_a_binary_path() {
    let (failure, words) = refused("assets/pic.bin");
    assert_eq!(failure, BlameFailure::Binary);
    assert!(words == "assets/pic.bin is binary · no blame", "{words}");
}

#[test]
fn the_blame_job_refuses_an_untracked_path() {
    let (failure, words) = refused("src/new.rs");
    assert_eq!(failure, BlameFailure::Untracked);
    assert!(
        words == "src/new.rs isn't committed yet · no blame",
        "{words}"
    );
}

#[test]
fn blame_failure_maps_every_error() {
    use graft_git::diff::ParseError;
    use graft_git::run::RunError;
    let run = BlameError::Run(RunError::GitNotFound);
    assert_eq!(
        blame_failure(&run),
        BlameFailure::Failed("git is not installed, or not on the PATH".to_owned())
    );
    let parse = BlameError::Parse(ParseError {
        line: 3,
        what: "a header",
    });
    assert_eq!(
        blame_failure(&parse),
        BlameFailure::Failed("git blame's output made no sense at line 3: a header".to_owned())
    );
    assert_eq!(blame_failure(&BlameError::Binary), BlameFailure::Binary);
    assert_eq!(
        blame_failure(&BlameError::Untracked),
        BlameFailure::Untracked
    );
    assert_eq!(
        blame_failure(&BlameError::Stopped),
        BlameFailure::Failed("blame was stopped".to_owned())
    );
    let shallow = BlameError::Shallow {
        path: PathBuf::from(".git/shallow"),
        source: std::io::Error::other("permission denied\nsecond line"),
    };
    assert_eq!(
        blame_failure(&shallow),
        BlameFailure::Failed("couldn't read .git/shallow: permission denied".to_owned())
    );
    assert_eq!(
        blame_failure(&BlameError::TooLarge {
            cap: 64 * 1024 * 1024
        }),
        BlameFailure::Failed("this file's blame is over 64 MiB, too large to show".to_owned())
    );
    assert_eq!(
        blame_failure(&BlameError::Changed {
            first: 3,
            second: 4
        }),
        BlameFailure::Failed("the file changed while blame read it; open blame again".to_owned())
    );
}
