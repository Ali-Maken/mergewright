//! `conflict::operation_conflicts`: the files an operation conflicted on,
//! read from git and nothing else (m6-conflicts/09, Amendment 2 hard
//! choice 2). Truth is git's own state in a throwaway repository.
//!
//! **What git records, measured on git 2.54.0 (Apple Git-157):** a stopped
//! merge, pull (merge), rebase (merge backend), cherry-pick and revert each
//! write a `# Conflicts:` block into `$GIT_DIR/MERGE_MSG`, one `#\t<path>`
//! line per conflicted file, the path unquoted even with a space in it. A
//! conflicted `stash pop` writes no `MERGE_MSG` at all.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use crate::conflict_fixtures::{
    conflict_set, downstream_of, graft_merge_stops, graft_stage, paths, two_file_conflict,
};
use crate::support::{NOW, TempRepo};

#[test]
fn a_merge_lists_resolved_files_too() {
    let repo = two_file_conflict();
    graft_merge_stops(&repo);
    repo.write("a.txt", b"1\nboth\n3\n");
    graft_stage(&repo, "a.txt", NOW + 1);

    let set = conflict_set(&repo);
    // (a) the one still unmerged, then (b) git's list adds the resolved one.
    assert_eq!(set.paths, paths(&["b.txt", "a.txt"]));
    assert!(set.recorded, "git's list was found");
}

#[test]
fn a_relaunch_lists_the_same_files() {
    let repo = two_file_conflict();
    graft_merge_stops(&repo);
    // Resolved in the shell: no step of Graft's saw it.
    repo.write("a.txt", b"1\nboth\n3\n");
    repo.git(&["add", "a.txt"]);
    // `conflict_set` opens a new handle and reads fresh facts: nothing of
    // this run is carried over, as after a relaunch.
    let first = conflict_set(&repo);
    let again = conflict_set(&repo);
    assert_eq!(first.paths, paths(&["b.txt", "a.txt"]));
    assert_eq!(again.paths, first.paths);
    assert!(again.recorded);
}

/// Resolve `a.txt` in the shell so only git's list can name it, then read.
fn after_resolving_a(repo: &TempRepo) -> graft_ops::conflict::ConflictSet {
    repo.write("a.txt", b"1\nboth\n3\n");
    repo.git(&["add", "a.txt"]);
    conflict_set(repo)
}

/// A kind whose stop writes git's list: `a.txt` resolved in the shell, so
/// only the list can name it.
fn assert_listed(repo: &TempRepo, kind: &str) {
    let set = after_resolving_a(repo);
    assert_eq!(set.paths, paths(&["b.txt", "a.txt"]), "{kind}");
    assert!(set.recorded, "{kind} writes Conflicts:");
}

fn stopped_shell_merge() -> TempRepo {
    let repo = two_file_conflict();
    assert!(repo.try_git(&["merge", "feature"]).is_err());
    repo
}

/// `git pull --no-rebase` of another repository's `feature`; the upstream
/// is returned so it outlives the read.
fn stopped_pull() -> (TempRepo, TempRepo) {
    let upstream = two_file_conflict();
    let repo = downstream_of(&upstream, true);
    let from = upstream.path().to_str().expect("utf-8 temp path");
    assert!(
        repo.try_git(&["pull", "--no-rebase", "--no-edit", from, "feature"])
            .is_err()
    );
    assert!(repo.exists(".git/MERGE_HEAD"), "the pull stopped mid-merge");
    (repo, upstream)
}

fn stopped_rebase() -> TempRepo {
    let repo = two_file_conflict();
    repo.git(&["switch", "-q", "feature"]);
    assert!(repo.try_git(&["rebase", "main"]).is_err());
    repo
}

fn stopped_pick() -> TempRepo {
    let repo = two_file_conflict();
    assert!(repo.try_git(&["cherry-pick", "feature"]).is_err());
    repo
}

/// `feature`'s commit reverted under a later edit of both files.
fn stopped_revert() -> TempRepo {
    let repo = two_file_conflict();
    repo.git(&["switch", "-q", "feature"]);
    repo.write("a.txt", b"1\nG\n3\n");
    repo.write("b.txt", b"G\n");
    repo.commit_all("later on feature");
    assert!(repo.try_git(&["revert", "--no-edit", "HEAD~1"]).is_err());
    repo
}

fn stopped_stash_pop() -> TempRepo {
    let repo = two_file_conflict();
    repo.write("a.txt", b"1\nS\n3\n");
    repo.git(&["stash", "-q"]);
    repo.write("a.txt", b"1\nT\n3\n");
    repo.commit_all("moved on");
    assert!(repo.try_git(&["stash", "pop"]).is_err());
    repo
}

#[test]
fn each_kind_says_what_git_records() {
    assert_listed(&stopped_shell_merge(), "merge");
    assert_listed(&stopped_pull().0, "pull");
    assert_listed(&stopped_rebase(), "rebase");
    assert_listed(&stopped_pick(), "cherry-pick");
    assert_listed(&stopped_revert(), "revert");

    // stash pop: git writes no MERGE_MSG, so only the unmerged path is known
    let repo = stopped_stash_pop();
    assert!(
        !repo.exists(".git/MERGE_MSG"),
        "stash pop writes no MERGE_MSG"
    );
    let set = conflict_set(&repo);
    assert_eq!(set.paths, paths(&["a.txt"]), "stash pop");
    assert!(!set.recorded, "stash pop records nothing");
}

#[test]
fn a_path_with_a_space_is_read_whole() {
    let repo = TempRepo::init();
    repo.write("b c.txt", b"x\n");
    repo.commit_all("base");
    repo.git(&["branch", "feature"]);
    repo.write("b c.txt", b"M\n");
    repo.commit_all("main");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("b c.txt", b"F\n");
    repo.commit_all("feature");
    repo.git(&["switch", "-q", "main"]);
    assert!(repo.try_git(&["merge", "feature"]).is_err());
    repo.write("b c.txt", b"both\n");
    repo.git(&["add", "b c.txt"]);
    let set = conflict_set(&repo);
    assert_eq!(set.paths, paths(&["b c.txt"]));
    assert!(set.recorded);
}

#[test]
fn the_comment_character_is_git_s() {
    let repo = two_file_conflict();
    repo.git(&["config", "core.commentChar", ";"]);
    assert!(repo.try_git(&["merge", "feature"]).is_err());
    assert!(
        String::from_utf8_lossy(&repo.read(".git/MERGE_MSG")).contains("; Conflicts:"),
        "git wrote its list under ';'"
    );
    let set = after_resolving_a(&repo);
    assert_eq!(set.paths, paths(&["b.txt", "a.txt"]));
    assert!(set.recorded);
}

#[test]
fn a_graft_step_is_remembered_without_git_s_list() {
    let repo = two_file_conflict();
    graft_merge_stops(&repo);
    repo.write("a.txt", b"1\nboth\n3\n");
    graft_stage(&repo, "a.txt", NOW + 1);
    // Take git's list away: the backup chain alone still names `a.txt`.
    std::fs::remove_file(repo.path().join(".git/MERGE_MSG")).expect("MERGE_MSG goes");
    let set = conflict_set(&repo);
    assert_eq!(set.paths, paths(&["b.txt", "a.txt"]));
    assert!(set.recorded, "the chain is a record too");
}

#[test]
fn nothing_in_progress_is_an_empty_unrecorded_set() {
    let repo = two_file_conflict();
    let set = conflict_set(&repo);
    assert!(set.paths.is_empty());
    assert!(!set.recorded);
}
