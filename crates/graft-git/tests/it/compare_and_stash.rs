//! `history_reads::{compare_span, count_between, only_in_stash}` against
//! real `git` (m7-power-tools/14a).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::AtomicBool;

use graft_core::oid::Oid;
use graft_fixtures::Scratch;
use graft_git::history_reads::{CompareSpan, compare_span, count_between, only_in_stash};
use graft_git::repo::RepoHandle;
use graft_git::run::RunError;

fn open(path: &Path) -> RepoHandle {
    RepoHandle::open(path).unwrap_or_else(|why| panic!("opening {}: {why}", path.display()))
}

/// Runs `git` hermetically in `dir`; asserts nothing about the result.
fn git(dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("LC_ALL", "C")
        .env("GIT_AUTHOR_NAME", "Graft Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Graft Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .output()
        .unwrap_or_else(|why| panic!("running git (graft-git does not run without it): {why}"))
}

/// [`git`], asserting success and returning stdout.
fn git_ok(dir: &Path, args: &[&str]) -> String {
    let output = git(dir, args);
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap_or_else(|why| panic!("git's output is UTF-8: {why}"))
}

fn rev_parse(dir: &Path, revision: &str) -> Oid {
    let text = git_ok(dir, &["rev-parse", revision]);
    Oid::from_hex(text.trim()).unwrap_or_else(|why| panic!("{text:?} is not an id: {why}"))
}

/// Blank ground to build a repository in (a corpus copy, as `unreached.rs`).
fn scratch() -> Scratch {
    graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial)
}

/// A fresh repository with one commit on `main`, in its own subdirectory.
fn repo_in(scratch: &Scratch) -> PathBuf {
    let dir = scratch.path().join("repo");
    std::fs::create_dir_all(&dir).unwrap_or_else(|why| panic!("mkdir: {why}"));
    git_ok(&dir, &["init", "-q", "-b", "main"]);
    commit(&dir, "base");
    dir
}

/// A commit changing `f.txt` to `message`, returning its id.
fn commit(dir: &Path, message: &str) -> Oid {
    std::fs::write(dir.join("f.txt"), format!("{message}\n"))
        .unwrap_or_else(|why| panic!("a fixture file writes: {why}"));
    git_ok(dir, &["add", "f.txt"]);
    git_ok(dir, &["commit", "-q", "-m", message]);
    rev_parse(dir, "HEAD")
}

/// Leaves a tracked change in the worktree and stashes it; returns the entry.
fn stash(dir: &Path, text: &str, extra: &[&str]) -> Oid {
    std::fs::write(dir.join("f.txt"), format!("{text}\n"))
        .unwrap_or_else(|why| panic!("a fixture file writes: {why}"));
    let mut args = vec!["stash", "push", "-q"];
    args.extend_from_slice(extra);
    git_ok(dir, &args);
    rev_parse(dir, "stash@{0}")
}

// ---- compare_span / count_between ----

#[test]
fn compare_span_counts_commits_since_the_split() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let base = rev_parse(&dir, "main");
    git_ok(&dir, &["switch", "-q", "-c", "feature-a"]);
    commit(&dir, "a1");
    commit(&dir, "a2");
    let tip = commit(&dir, "a3");

    let got = compare_span(&open(&dir), base, tip, &()).expect("reads");
    assert_eq!(
        got,
        CompareSpan::Since {
            split: base,
            ahead: 3
        }
    );
}

#[test]
fn an_ancestor_has_nothing_ahead() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let old = rev_parse(&dir, "main");
    let new = commit(&dir, "next");

    let got = compare_span(&open(&dir), new, old, &()).expect("reads");
    assert_eq!(
        got,
        CompareSpan::Since {
            split: old,
            ahead: 0
        }
    );
}

#[test]
fn unrelated_histories_have_no_split() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let main = rev_parse(&dir, "main");
    git_ok(&dir, &["switch", "-q", "--orphan", "other"]);
    let other = commit(&dir, "other root");

    let got = compare_span(&open(&dir), main, other, &()).expect("reads");
    assert_eq!(got, CompareSpan::NoSharedHistory);
}

#[test]
fn compare_span_honours_stop() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let base = rev_parse(&dir, "main");
    let tip = commit(&dir, "next");

    let got = compare_span(&open(&dir), base, tip, &AtomicBool::new(true));
    assert!(matches!(got, Err(RunError::Stopped)), "{got:?}");
}

#[test]
fn count_between_counts_what_leaves() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    commit(&dir, "x");
    let old = commit(&dir, "y");
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~2"]);
    commit(&dir, "x rewritten");
    commit(&dir, "y rewritten");
    let new = commit(&dir, "z");

    assert_eq!(count_between(&open(&dir), new, old, &()).expect("reads"), 2);
    assert_eq!(count_between(&open(&dir), old, new, &()).expect("reads"), 3);
}

#[test]
fn count_between_honours_stop() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let base = rev_parse(&dir, "main");
    let tip = commit(&dir, "next");

    let got = count_between(&open(&dir), base, tip, &AtomicBool::new(true));
    assert!(matches!(got, Err(RunError::Stopped)), "{got:?}");
}

// ---- only_in_stash ----

/// `main` at `base`, a commit `A` on it, a stash made on `A`, then `A`
/// rewritten off `main` (reset and a new commit). Returns `(A, stash)`.
fn stash_on_a_rebased_away_base(dir: &Path, extra: &[&str]) -> (Oid, Oid) {
    let a = commit(dir, "A");
    let entry = stash(dir, "wip", extra);
    git_ok(dir, &["reset", "-q", "--hard", "HEAD~1"]);
    commit(dir, "A rewritten");
    (a, entry)
}

#[test]
fn only_in_stash_finds_the_rebased_away_base() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let (a, entry) = stash_on_a_rebased_away_base(&dir, &[]);

    let got = only_in_stash(&open(&dir), &[entry], 10, &()).expect("reads");
    assert_eq!(got.commits, vec![(a, 0)]);
    assert!(!got.capped);
    assert_eq!(got.skipped, 0);
}

#[test]
fn only_in_stash_never_names_index_or_untracked_commits() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    std::fs::write(dir.join("new.txt"), "untracked\n")
        .unwrap_or_else(|why| panic!("a fixture file writes: {why}"));
    let (a, entry) = stash_on_a_rebased_away_base(&dir, &["-u"]);
    let index = rev_parse(&dir, &format!("{entry}^2"));
    let untracked = rev_parse(&dir, &format!("{entry}^3"));

    let got = only_in_stash(&open(&dir), &[entry], 10, &()).expect("reads");
    assert_eq!(got.commits, vec![(a, 0)]);
    assert!(
        !got.commits
            .iter()
            .any(|(id, _)| *id == index || *id == untracked)
    );
}

#[test]
fn a_commit_two_stashes_reach_gets_the_newest() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let a = commit(&dir, "A");
    stash(&dir, "wip 1", &[]);
    stash(&dir, "wip 2", &[]);
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~1"]);
    commit(&dir, "A rewritten");
    let entries = [rev_parse(&dir, "stash@{0}"), rev_parse(&dir, "stash@{1}")];

    let got = only_in_stash(&open(&dir), &entries, 10, &()).expect("reads");
    assert_eq!(got.commits, vec![(a, 0)]);
}

#[test]
fn a_stash_on_a_live_branch_has_nothing_only_in_it() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    commit(&dir, "A");
    let entry = stash(&dir, "wip", &[]);

    let got = only_in_stash(&open(&dir), &[entry], 10, &()).expect("reads");
    assert!(got.commits.is_empty(), "{got:?}");
    assert!(!got.capped);
}

#[test]
fn only_in_stash_is_capped() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    commit(&dir, "A1");
    commit(&dir, "A2");
    commit(&dir, "A3");
    let entry = stash(&dir, "wip", &[]);
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~3"]);

    let got = only_in_stash(&open(&dir), &[entry], 2, &()).expect("reads");
    assert!(got.capped);
    assert_eq!(got.commits.len(), 2);
}

#[test]
fn only_in_stash_skips_an_entry_git_no_longer_has() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let (a, entry) = stash_on_a_rebased_away_base(&dir, &[]);
    let gone = Oid::from_hex("1111111111111111111111111111111111111111").expect("hex");

    let got = only_in_stash(&open(&dir), &[gone, entry], 10, &()).expect("reads");
    assert_eq!(got.commits, vec![(a, 1)]);
    assert_eq!(got.skipped, 1);
}

#[test]
fn only_in_stash_honours_stop() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let (_, entry) = stash_on_a_rebased_away_base(&dir, &[]);

    let got = only_in_stash(&open(&dir), &[entry], 10, &AtomicBool::new(true));
    assert!(matches!(got, Err(RunError::Stopped)), "{got:?}");
}
