//! The awkward repositories, not the happy path: opening one of these wrongly
//! is what makes a git tool untrustworthy (PROMPT.md §6.3).
//!
//! Not covered here: a path that is not valid UTF-8. APFS and HFS+ reject
//! filenames that are not valid UTF-8, so such a fixture cannot be created on
//! the platform these tests run on. The API takes that case seriously anyway —
//! every path in and out of this module is a `Path`, never a `String` — and
//! the behaviour itself belongs in a Linux run of the heavy tier.

use std::path::Path;

use super::*;
use crate::fixture::Fixture;

/// Compares two paths by their canonical form, so that a symlinked temporary
/// directory does not fail a test that is otherwise right.
#[track_caller]
fn assert_same_path(actual: &Path, expected: &Path) {
    let canonical = |path: &Path| {
        path.canonicalize()
            .unwrap_or_else(|error| panic!("{} cannot be resolved: {error}", path.display()))
    };
    assert_eq!(canonical(actual), canonical(expected));
}

/// The full ref name of a `HEAD` that is on a branch, born or unborn.
#[track_caller]
fn branch_name(head: &HeadState) -> &[u8] {
    match head {
        HeadState::Branch(name) | HeadState::Unborn(name) => name.as_bytes(),
        HeadState::Detached(id) => panic!("expected a branch, HEAD is detached at {id}"),
    }
}

#[test]
fn opens_a_repository_at_its_root() {
    let fixture = Fixture::with_two_commits();

    let repo = RepoHandle::open(fixture.root()).expect("a repository with two commits opens");

    assert_same_path(repo.git_dir(), &fixture.root().join(".git"));
    assert_same_path(repo.workdir().expect("not bare"), fixture.root());
    assert!(!repo.is_bare());
    assert_eq!(
        branch_name(&repo.head().expect("HEAD is readable")),
        b"refs/heads/main"
    );
}

#[test]
fn discovers_the_repository_from_a_subdirectory() {
    let fixture = Fixture::with_two_commits();
    let nested = fixture.subdirectory("src/deeply/nested");

    let repo = RepoHandle::open(&nested).expect("discovery walks up to the repository");

    assert_same_path(repo.git_dir(), &fixture.root().join(".git"));
    assert_same_path(repo.workdir().expect("not bare"), fixture.root());
}

#[test]
fn an_empty_repository_has_an_unborn_head() {
    let fixture = Fixture::empty_repo();

    let repo = RepoHandle::open(fixture.root()).expect("a repository with no commits opens");
    let head = repo
        .head()
        .expect("HEAD is readable before the first commit");

    // The branch does not exist yet, but its name does: it is where the first
    // commit will land, and it is what the status line has to show.
    assert_eq!(
        head,
        HeadState::Unborn(RefName::new("refs/heads/main").expect("valid"))
    );
    assert_same_path(repo.workdir().expect("not bare"), fixture.root());
}

#[test]
fn a_detached_head_reports_the_commit_it_points_at() {
    let fixture = Fixture::with_two_commits();
    let first = fixture.rev_parse("HEAD~1");
    fixture.git(&["checkout", "--detach", &first]);

    let repo = RepoHandle::open(fixture.root()).expect("a detached repository opens");
    let head = repo.head().expect("HEAD is readable while detached");

    assert_eq!(
        head,
        HeadState::Detached(Oid::from_hex(&first).expect("git printed hex"))
    );
}

#[test]
fn a_bare_repository_has_no_working_tree() {
    let fixture = Fixture::bare_repo();

    let repo = RepoHandle::open(fixture.root()).expect("a bare repository opens");

    assert!(repo.is_bare());
    assert_eq!(repo.workdir(), None);
    assert_same_path(repo.git_dir(), fixture.root());
}

#[test]
fn a_linked_worktree_keeps_its_own_head_and_shares_the_repository() {
    let fixture = Fixture::with_two_commits();
    let worktree = fixture.root().join("linked");
    fixture.git(&["worktree", "add", "-b", "side", "linked"]);

    let repo = RepoHandle::open(&worktree).expect("a linked worktree opens");

    assert_same_path(repo.workdir().expect("not bare"), &worktree);
    // Its own git dir under the main repository's, but the same common dir —
    // which is how a worktree is told apart from an ordinary repository.
    assert_same_path(
        repo.git_dir(),
        &fixture.root().join(".git").join("worktrees").join("linked"),
    );
    assert_same_path(repo.common_dir(), &fixture.root().join(".git"));
    // `HEAD` is per-worktree: the main checkout is still on `main`.
    assert_eq!(
        branch_name(&repo.head().expect("HEAD is readable")),
        b"refs/heads/side"
    );

    let main = RepoHandle::open(fixture.root()).expect("the main worktree still opens");
    assert_eq!(
        branch_name(&main.head().expect("HEAD is readable")),
        b"refs/heads/main"
    );
}

#[test]
fn a_submodules_working_directory_opens_the_submodule_not_its_parent() {
    let inner = Fixture::with_two_commits();
    let outer = Fixture::with_two_commits();
    // `protocol.file.allow` is `user` by default since git 2.38.1, so a
    // submodule from a local path has to be allowed explicitly. Still no
    // network: the source is a directory on this machine.
    outer.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        &inner.root().to_string_lossy(),
        "vendor/inner",
    ]);
    let submodule = outer.root().join("vendor").join("inner");

    let repo = RepoHandle::open(&submodule).expect("a submodule working directory opens");

    // A submodule's `.git` is a file pointing into the parent's `.git/modules`.
    // Opening its working directory must follow that pointer rather than
    // walking up to the parent repository.
    assert_same_path(repo.workdir().expect("not bare"), &submodule);
    assert_same_path(
        repo.git_dir(),
        &outer
            .root()
            .join(".git")
            .join("modules")
            .join("vendor")
            .join("inner"),
    );
}

#[test]
fn a_plain_directory_is_not_a_repository() {
    let fixture = Fixture::empty_dir();

    let error = RepoHandle::open(fixture.root()).expect_err("a bare directory is not a repository");

    assert!(
        matches!(error, OpenError::NotARepo { .. }),
        "expected NotARepo, got {error:?}"
    );
    assert_same_path(error.path(), fixture.root());
}

#[test]
fn a_repository_below_the_path_is_not_found() {
    // Discovery walks up, never down: a directory of checkouts is not itself
    // a repository, and answering with the first one found inside it would
    // silently operate on the wrong repository.
    let fixture = Fixture::empty_dir();
    let child = fixture.subdirectory("checkout");
    fixture.git_in(&child, &["init", "-b", "main"]);

    let error = RepoHandle::open(fixture.root()).expect_err("the repository is below, not above");

    assert!(
        matches!(error, OpenError::NotARepo { .. }),
        "expected NotARepo, got {error:?}"
    );
}

#[test]
fn a_path_that_does_not_exist_is_reported_as_such() {
    let fixture = Fixture::empty_dir();
    let missing = fixture.root().join("no-such-directory");

    let error = RepoHandle::open(&missing).expect_err("a missing path cannot be opened");

    assert!(
        matches!(error, OpenError::Inaccessible { .. }),
        "expected Inaccessible, got {error:?}"
    );
    assert_eq!(error.path(), missing);
}

#[test]
fn every_open_error_names_the_path_it_was_given() {
    // The message drives the exit screen, so the path has to survive into it.
    let fixture = Fixture::empty_dir();

    let error = RepoHandle::open(fixture.root()).expect_err("not a repository");

    assert!(
        error
            .to_string()
            .contains(&fixture.root().display().to_string()),
        "{error} should name {}",
        fixture.root().display()
    );
}
