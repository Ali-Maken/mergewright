use std::fs;

use super::{WorkingTreeCounts, counts};
use crate::fixture::Fixture;
use crate::repo::RepoHandle;

fn open(fixture: &Fixture) -> RepoHandle {
    RepoHandle::open(fixture.root()).expect("a fixture repository opens")
}

#[test]
fn a_clean_tree_reports_nothing_and_is_clean() {
    let fixture = Fixture::with_two_commits();
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads");

    assert_eq!(result, WorkingTreeCounts::default());
    assert!(result.is_clean());
}

#[test]
fn a_staged_file_counts_as_staged_only() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("staged.txt"), b"content").expect("file written");
    fixture.git(&["add", "staged.txt"]);
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads");

    assert_eq!(
        result,
        WorkingTreeCounts {
            staged: 1,
            unstaged: 0,
            untracked: 0,
        }
    );
    assert!(!result.is_clean());
}

#[test]
fn a_modified_tracked_file_counts_as_unstaged_only() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("tracked.txt"), b"first").expect("file written");
    fixture.git(&["add", "tracked.txt"]);
    fixture.commit("add tracked.txt");
    fs::write(fixture.root().join("tracked.txt"), b"changed").expect("file rewritten");
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads");

    assert_eq!(
        result,
        WorkingTreeCounts {
            staged: 0,
            unstaged: 1,
            untracked: 0,
        }
    );
}

#[test]
fn a_new_file_counts_as_untracked_only() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("new.txt"), b"content").expect("file written");
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads");

    assert_eq!(
        result,
        WorkingTreeCounts {
            staged: 0,
            unstaged: 0,
            untracked: 1,
        }
    );
}

#[test]
fn staged_unstaged_and_untracked_all_count_together() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("tracked.txt"), b"first").expect("file written");
    fixture.git(&["add", "tracked.txt"]);
    fixture.commit("add tracked.txt");
    fs::write(fixture.root().join("tracked.txt"), b"changed").expect("unstaged edit");
    fs::write(fixture.root().join("staged.txt"), b"content").expect("staged file written");
    fixture.git(&["add", "staged.txt"]);
    fs::write(fixture.root().join("untracked.txt"), b"content").expect("untracked file written");
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads");

    assert_eq!(
        result,
        WorkingTreeCounts {
            staged: 1,
            unstaged: 1,
            untracked: 1,
        }
    );
}

#[test]
fn an_unborn_branch_with_a_staged_file_counts_it_as_staged() {
    // No commits at all: `HEAD` does not resolve to a tree. gix's status
    // falls back to the empty tree for the staged (tree-vs-index) side, so a
    // file added to the index before the first commit still counts.
    let fixture = Fixture::empty_repo();
    fs::write(fixture.root().join("first.txt"), b"content").expect("file written");
    fixture.git(&["add", "first.txt"]);
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads on an unborn branch");

    assert_eq!(
        result,
        WorkingTreeCounts {
            staged: 1,
            unstaged: 0,
            untracked: 0,
        }
    );
}

#[test]
fn a_bare_repository_is_reported_clean() {
    let fixture = Fixture::bare_repo();
    let repo = open(&fixture);

    let result = counts(&repo).expect("a bare repository short-circuits, not errors");

    assert_eq!(result, WorkingTreeCounts::default());
    assert!(result.is_clean());
}

#[test]
fn ignored_files_are_not_counted_as_untracked() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join(".gitignore"), b"ignored.txt\n").expect(".gitignore written");
    fixture.git(&["add", ".gitignore"]);
    fixture.commit("add .gitignore");
    fs::write(fixture.root().join("ignored.txt"), b"content").expect("ignored file written");
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads");

    assert_eq!(
        result,
        WorkingTreeCounts {
            staged: 0,
            unstaged: 0,
            untracked: 0,
        }
    );
}
