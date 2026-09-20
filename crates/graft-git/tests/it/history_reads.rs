//! `history_reads`'s read layer, against real `git`: `merge_base`,
//! `commits_between`, `forecast_merge` and `changed_paths` (m5-history-and-
//! ghost-preview/02).

use std::path::Path;
use std::process::{Command, Output};

use graft_core::oid::Oid;
use graft_fixtures::Scratch;
use graft_git::history_reads::{
    Forecast, changed_paths, commits_between, forecast_merge, merge_base,
};
use graft_git::repo::RepoHandle;
use graft_git::version;

fn open(path: &Path) -> RepoHandle {
    RepoHandle::open(path).unwrap_or_else(|why| panic!("opening {}: {why}", path.display()))
}

/// Runs `git` hermetically in `dir`; asserts nothing about the result, for a
/// command scripted to fail (a conflicting merge, say).
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

/// [`git`], asserting success — every setup step.
fn git_ok(dir: &Path, args: &[&str]) {
    let output = git(dir, args);
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn rev_parse(dir: &Path, revision: &str) -> Oid {
    let output = git(dir, &["rev-parse", revision]);
    assert!(output.status.success(), "rev-parse {revision}");
    let hex = String::from_utf8(output.stdout)
        .unwrap_or_else(|why| panic!("rev-parse's output is not UTF-8: {why}"));
    Oid::from_hex(hex.trim()).unwrap_or_else(|why| panic!("{hex:?} is not an id: {why}"))
}

fn init(dir: &Path) {
    git_ok(dir, &["init", "-q", "-b", "main"]);
}

fn write_and_commit(dir: &Path, file: &str, content: &str, message: &str) {
    std::fs::write(dir.join(file), content)
        .unwrap_or_else(|why| panic!("a fixture file writes: {why}"));
    git_ok(dir, &["add", file]);
    git_ok(dir, &["commit", "-q", "-m", message]);
}

/// A blank scratch directory to build a fixture repository in — never a
/// corpus copy, since these tests need branch topologies and conflicts the
/// corpus's own names do not promise (`copy_into_temp` gives a real *shape*,
/// not blank ground to build on).
fn scratch() -> Scratch {
    graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial)
}

#[test]
fn merge_base_matches_git_merge_base_and_is_none_for_unrelated_history() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    write_and_commit(dir, "f.txt", "base\n", "base");
    git_ok(dir, &["checkout", "-q", "-b", "a"]);
    write_and_commit(dir, "a.txt", "a\n", "a change");
    git_ok(dir, &["checkout", "-q", "-b", "b", "main"]);
    write_and_commit(dir, "b.txt", "b\n", "b change");

    let expected = git(dir, &["merge-base", "a", "b"]);
    assert!(expected.status.success());
    let expected =
        Oid::from_hex(String::from_utf8_lossy(&expected.stdout).trim()).expect("a valid id");

    let repo = open(dir);
    let a = rev_parse(dir, "a");
    let b = rev_parse(dir, "b");
    let got = merge_base(&repo, a, b)
        .expect("merge-base runs")
        .expect("a and b share an ancestor");
    assert_eq!(got, expected);

    // Two roots with nothing in common: no merge base at all.
    git_ok(dir, &["checkout", "-q", "--orphan", "orphan"]);
    write_and_commit(dir, "o.txt", "o\n", "orphan root");
    let orphan = rev_parse(dir, "orphan");
    assert_eq!(merge_base(&repo, a, orphan).expect("merge-base runs"), None);
}

#[test]
fn commits_between_is_oldest_first_and_caps_to_the_commits_nearest_the_tip() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    write_and_commit(dir, "f.txt", "0\n", "c0");
    let c0 = rev_parse(dir, "HEAD");
    write_and_commit(dir, "f.txt", "1\n", "c1");
    let c1 = rev_parse(dir, "HEAD");
    write_and_commit(dir, "f.txt", "2\n", "c2");
    let c2 = rev_parse(dir, "HEAD");
    write_and_commit(dir, "f.txt", "3\n", "c3");
    let c3 = rev_parse(dir, "HEAD");
    write_and_commit(dir, "f.txt", "4\n", "c4");
    let c4 = rev_parse(dir, "HEAD");

    let repo = open(dir);

    // Uncapped: every commit after c0, oldest first.
    let full = commits_between(&repo, c0, c4, 10, &()).expect("rev-list runs");
    assert!(!full.capped);
    assert_eq!(
        full.commits.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![c1, c2, c3, c4]
    );
    assert_eq!(full.commits[0].subject, "c1");
    assert_eq!(full.commits[0].parents, vec![c0]);
    assert_eq!(full.commits.last().unwrap().subject, "c4");

    // Capped at 2: the two commits nearest the tip, still oldest first.
    let capped = commits_between(&repo, c0, c4, 2, &()).expect("rev-list runs");
    assert!(capped.capped);
    assert_eq!(
        capped.commits.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![c3, c4]
    );
}

#[test]
fn forecast_merge_finds_a_real_conflict_and_a_clean_pair() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    write_and_commit(dir, "shared.txt", "base\n", "base");
    let base = rev_parse(dir, "HEAD");
    git_ok(dir, &["checkout", "-q", "-b", "ours"]);
    write_and_commit(dir, "shared.txt", "ours\n", "ours change");
    let ours = rev_parse(dir, "HEAD");
    git_ok(dir, &["checkout", "-q", "-b", "theirs", "main"]);
    write_and_commit(dir, "shared.txt", "theirs\n", "theirs change");
    let theirs = rev_parse(dir, "HEAD");
    // Diverges from `ours` without touching the same file: a clean merge.
    git_ok(dir, &["checkout", "-q", "-b", "clean", "main"]);
    write_and_commit(dir, "clean.txt", "clean\n", "clean change");
    let clean = rev_parse(dir, "HEAD");

    let repo = open(dir);
    let git_version = version::system_git()
        .expect("the system git is found")
        .version();

    let conflict = forecast_merge(&repo, &git_version, None, ours, theirs, &())
        .expect("never an error for a real forecast");
    assert_eq!(
        conflict,
        Forecast::Conflicts {
            paths: vec![std::path::PathBuf::from("shared.txt")]
        }
    );

    let clean_forecast = forecast_merge(&repo, &git_version, None, ours, clean, &())
        .expect("never an error for a real forecast");
    assert_eq!(clean_forecast, Forecast::Clean);

    // With an explicit merge base (a commit's own parent): the same
    // conflict, forecast the way a rebase/cherry-pick would ask for it.
    let with_base = forecast_merge(&repo, &git_version, Some(base), ours, theirs, &())
        .expect("never an error for a real forecast");
    assert_eq!(
        with_base,
        Forecast::Conflicts {
            paths: vec![std::path::PathBuf::from("shared.txt")]
        }
    );
}

#[test]
fn forecast_is_not_checkable_never_an_error() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    write_and_commit(dir, "f.txt", "base\n", "base");
    let head = rev_parse(dir, "HEAD");

    let repo = open(dir);

    // An old git: never checkable, never an error.
    let old = version::GitVersion {
        major: 2,
        minor: 30,
        patch: 0,
    };
    assert!(matches!(
        forecast_merge(&repo, &old, None, head, head, &()).expect("never an error"),
        Forecast::NotCheckable(_)
    ));

    // New enough, but the answer `merge-tree` gives is not the shape this
    // reader trusts (an id naming no commit: exit 1, nothing on stdout,
    // not a conflict's tree id and paths).
    let git_version = version::system_git()
        .expect("the system git is found")
        .version();
    let bad = Oid::from_hex("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef").expect("a valid id");
    assert!(matches!(
        forecast_merge(&repo, &git_version, None, head, bad, &()).expect("never an error"),
        Forecast::NotCheckable(_)
    ));
}

#[test]
fn changed_paths_matches_diff_tree_name_only_and_caps() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    write_and_commit(dir, "a.txt", "a\n", "a");
    let from = rev_parse(dir, "HEAD");
    write_and_commit(dir, "b.txt", "b\n", "b");
    write_and_commit(dir, "c.txt", "c\n", "c");
    let to = rev_parse(dir, "HEAD");

    let repo = open(dir);
    let (paths, capped) = changed_paths(&repo, from, to, 10).expect("diff-tree runs");
    assert!(!capped);
    assert_eq!(
        paths,
        vec![
            std::path::PathBuf::from("b.txt"),
            std::path::PathBuf::from("c.txt")
        ]
    );

    let (paths, capped) = changed_paths(&repo, from, to, 1).expect("diff-tree runs");
    assert!(capped);
    assert_eq!(paths, vec![std::path::PathBuf::from("b.txt")]);
}
