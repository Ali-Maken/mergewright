//! m6-conflicts/68 on a real repository: a skip that finishes the rebase
//! says it skipped. Graft begins the rebase, so the skip closes Graft's own
//! entry; git's log and the journal are read back rather than trusted from
//! the line.
//!
//! m6-conflicts/78 (review Minor 1): the words are `<d>, skipped a commit`,
//! true whichever commit the skip left out — the last one (the two-stop
//! rebase at step 2), or the first of three with the other two applying
//! cleanly after it.

use std::path::Path;

use graft::update::{AppState, Status};
use graft_core::event::KeyCode;
use graft_core::journal::{EntryStatus, JournalEntry, OperationKind};

use super::file_list::{at_step_two, graft_rebase_at_step_one, on_branch};
use crate::app_harness::{App, git};
use graft_core::on_disk::CURRENT_DIR;

/// `s`, `y` from the banner at a stop, and the rebase's journal entry once
/// the skip has finished it.
fn skip_to_the_end(app: &App, state: &AppState, repo: &Path) -> (AppState, JournalEntry) {
    let mut state = app.key(state, KeyCode::Escape);
    if !state.banner_focus {
        state = app.key(&state, KeyCode::Tab);
    }
    assert!(state.banner_focus, "{:?}", state.status);
    let state = app.key(&state, KeyCode::Char('s'));
    assert!(state.confirm.is_some(), "`s` asks first");
    let state = app.key(&state, KeyCode::Char('y'));
    assert!(state.operating.is_some(), "`y` sent the skip");
    let state = app.pump(state, "the skip that finishes the rebase", |state| {
        state.operating.is_none()
            && state.in_progress.is_none()
            && matches!(state.status, Status::Done { .. })
    });
    let entries = graft_ops::journal::Journal::open(
        &repo.join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl")),
    )
    .expect("the journal the rebase wrote is readable")
    .entries()
    .to_vec();
    let rebase = entries
        .iter()
        .rev()
        .find(|entry| entry.kind == OperationKind::Rebase)
        .expect("the rebase is journaled")
        .clone();
    assert_eq!(
        rebase.status,
        EntryStatus::Committed,
        "the skip finished it"
    );
    assert!(
        !repo.join(".git/rebase-merge").exists(),
        "the rebase is over"
    );
    (state, rebase)
}

/// The line, in the state and as drawn.
fn says_skipped(state: &AppState, rebase: &JournalEntry) {
    assert_eq!(
        state.status,
        Status::done(format!("{}, skipped a commit", rebase.description))
    );
    let body = crate::captures::frame(state, 120, 40).join("\n");
    assert!(
        body.contains(&format!(
            "\u{2713} {}, skipped a commit",
            rebase.description
        )),
        "{body}"
    );
}

#[test]
fn a_final_skip_says_it_skipped() {
    let (scratch, app, state) = graft_rebase_at_step_one();
    let repo = scratch.path();
    let state = at_step_two(&app, state);
    // The commit step 2 is replaying: the one the skip leaves out.
    let skipped = git(repo, &["log", "-1", "--format=%s", "REBASE_HEAD"]);
    let skipped = skipped.trim().to_owned();
    assert!(!skipped.is_empty());

    let (state, rebase) = skip_to_the_end(&app, &state, repo);
    says_skipped(&state, &rebase);
    let log = git(repo, &["log", "--format=%s", "HEAD"]);
    assert!(
        !log.lines().any(|subject| subject == skipped),
        "{skipped:?} was skipped: {log}"
    );
    drop(state);
    app.close();
    drop(scratch);
}

fn commit(repo: &Path, message: &str) {
    git(
        repo,
        &[
            "-c",
            "user.name=Graft Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            message,
        ],
    );
}

/// `topic` three commits ahead of a shared base, `main` one: `topic`'s
/// first commit and `main`'s both rewrite `a.txt`; `topic`'s other two add
/// files of their own. Rebasing `topic` onto `main` stops on the first and
/// only the first.
fn three_commits_one_stop() -> tempfile::TempDir {
    let scratch = tempfile::TempDir::with_prefix("graft-first-skip-")
        .expect("a temp directory for the repository");
    let repo = scratch.path();
    git(
        repo,
        &["init", "-q", "-b", "main", "--object-format=sha1", "."],
    );
    let write = |path: &str, text: &str| {
        std::fs::write(repo.join(path), text).expect("the scratch repo is writable");
        git(repo, &["add", path]);
    };
    write("a.txt", "base\n");
    commit(repo, "base");
    git(repo, &["checkout", "-q", "-b", "topic"]);
    write("a.txt", "topic\n");
    commit(repo, "topic rewrites a");
    write("b.txt", "b\n");
    commit(repo, "topic adds b");
    write("c.txt", "c\n");
    commit(repo, "topic adds c");
    git(repo, &["checkout", "-q", "main"]);
    write("a.txt", "main\n");
    commit(repo, "main rewrites a");
    git(repo, &["checkout", "-q", "topic"]);
    scratch
}

/// Review Minor 1: three commits, a stop on the first, `s y`, and the other
/// two apply cleanly. The skipped commit was the first, not the last, and
/// the line says only that a commit was skipped.
#[test]
fn a_first_stop_skip_that_finishes_says_skipped() {
    let scratch = three_commits_one_stop();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = app.pump(state, "the graph, its badges and the tree", |state| {
        state.refs.is_some() && state.history.is_some() && state.working_tree.is_some()
    });
    let mut state = app.key(&state, KeyCode::Char('g'));
    for _ in 0..10 {
        if on_branch(&state, "main") {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    assert!(on_branch(&state, "main"), "no row carries `main`");
    let state = app.key(&state, KeyCode::Char('r'));
    let state = app.pump(state, "the rebase preview's plan", |state| {
        state.preview.as_ref().is_some_and(|preview| {
            !matches!(
                preview.plan,
                graft::update::detail::Loading::Pending(_) | graft::update::detail::Loading::Idle
            )
        })
    });
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the rebase stopping on its first commit", |state| {
        state.operating.is_none()
            && state.in_progress.is_some()
            && state.working_tree.as_ref().is_some_and(|tree| {
                tree.entries
                    .iter()
                    .any(|entry| entry.unmerged && entry.path == Path::new("a.txt"))
            })
    });
    let stopped_on = git(repo, &["log", "-1", "--format=%s", "REBASE_HEAD"]);
    assert_eq!(stopped_on.trim(), "topic rewrites a", "the first commit");

    let (state, rebase) = skip_to_the_end(&app, &state, repo);
    says_skipped(&state, &rebase);
    let log = git(repo, &["log", "--format=%s", "HEAD"]);
    let subjects: Vec<&str> = log.lines().collect();
    assert_eq!(
        subjects,
        ["topic adds c", "topic adds b", "main rewrites a", "base"],
        "the first was skipped, the other two applied"
    );
    drop(state);
    app.close();
    drop(scratch);
}
