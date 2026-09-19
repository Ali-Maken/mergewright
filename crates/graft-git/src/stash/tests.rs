use std::fs;

use super::*;
use crate::fixture::Fixture;

fn open(fixture: &Fixture) -> RepoHandle {
    RepoHandle::open(fixture.root()).expect("a fixture repository opens")
}

#[test]
fn a_repository_with_no_stashes_lists_nothing() {
    let fixture = Fixture::with_two_commits();
    let repo = open(&fixture);
    assert_eq!(list(&repo, &()).expect("stash list reads"), vec![]);
}

#[test]
fn two_pushes_come_back_newest_first_with_messages_and_paths() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("tracked.txt"), "first\n").expect("file written");
    fixture.git(&["add", "tracked.txt"]);
    fixture.commit("add tracked.txt");

    fs::write(fixture.root().join("tracked.txt"), "changed once\n").expect("file rewritten");
    fixture.git(&["stash", "push", "-u", "-m", "first stash"]);

    fs::write(fixture.root().join("tracked.txt"), "changed twice\n").expect("file rewritten");
    fs::write(fixture.root().join("untracked.txt"), "new\n").expect("untracked file written");
    fixture.git(&["stash", "push", "-u", "-m", "second stash"]);

    let repo = open(&fixture);
    let entries = list(&repo, &()).expect("stash list reads");

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].index, 0);
    assert_eq!(entries[0].message, "On main: second stash");
    assert_eq!(
        sorted(&entries[0].paths),
        expected_paths(&["tracked.txt", "untracked.txt"])
    );
    assert_eq!(entries[1].index, 1);
    assert_eq!(entries[1].message, "On main: first stash");
    assert_eq!(sorted(&entries[1].paths), expected_paths(&["tracked.txt"]));

    // The commit each entry names is the one `git stash list --format=%H`
    // reports at the same position.
    let hashes = fixture.git(&["stash", "list", "--format=%H"]);
    let hashes: Vec<&str> = hashes.lines().collect();
    assert_eq!(entries[0].commit.to_string(), hashes[0]);
    assert_eq!(entries[1].commit.to_string(), hashes[1]);
}

#[test]
fn a_push_with_no_untracked_files_has_no_third_parent_spawn() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("tracked.txt"), "first\n").expect("file written");
    fixture.git(&["add", "tracked.txt"]);
    fixture.commit("add tracked.txt");
    fs::write(fixture.root().join("tracked.txt"), "changed\n").expect("file rewritten");
    fixture.git(&["stash", "push", "-m", "no untracked"]);

    let repo = open(&fixture);
    let entries = list(&repo, &()).expect("stash list reads");

    assert_eq!(entries.len(), 1);
    assert_eq!(sorted(&entries[0].paths), expected_paths(&["tracked.txt"]));
}

fn expected_paths(names: &[&str]) -> Vec<std::path::PathBuf> {
    let mut paths: Vec<std::path::PathBuf> = names.iter().map(std::path::PathBuf::from).collect();
    paths.sort();
    paths
}

fn sorted(paths: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    let mut paths = paths.to_vec();
    paths.sort();
    paths
}

#[test]
fn a_stash_commit_that_cannot_be_read_is_an_error_not_a_no() {
    // The finding this guards: an unreadable stash commit used to answer
    // "no untracked files", so `StashEntry::paths` quietly left out every
    // file the stash was carrying and the entry still listed as healthy.
    let fixture = Fixture::with_two_commits();
    let repo = open(&fixture);
    let missing =
        Oid::from_hex("0123456789abcdef0123456789abcdef01234567").expect("a well-formed id");

    let error = has_untracked_commit(&repo, missing).expect_err("an unreadable commit is an error");

    assert!(matches!(error, StashError::Object(_)), "{error:?}");
    assert!(
        error.to_string().contains("could not be read"),
        "{error}: the message must say what went wrong"
    );
}

#[test]
fn a_real_stash_commit_is_read_without_error() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("tracked.txt"), "first\n").expect("file written");
    fixture.git(&["add", "tracked.txt"]);
    fixture.commit("add tracked.txt");
    fs::write(fixture.root().join("tracked.txt"), "changed\n").expect("file rewritten");
    fs::write(fixture.root().join("loose.txt"), "untracked\n").expect("file written");
    fixture.git(&["stash", "push", "-u", "-m", "with untracked"]);
    let repo = open(&fixture);
    let entry = &list(&repo, &()).expect("stash list reads")[0];

    assert!(
        has_untracked_commit(&repo, entry.commit).expect("a real stash commit reads"),
        "a `-u` stash carries a third parent"
    );
}
