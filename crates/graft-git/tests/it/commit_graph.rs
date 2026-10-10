//! Building a commit-graph with the real system `git` (m1-the-graph/54), on
//! writable copies of corpus shapes: the fast tier's 200-commit line.

use std::path::Path;
use std::process::Command;

use graft_fixtures::Shape;
use graft_git::commit_graph::{self, CommitGraphError, CommitGraphStatus, Offer};
use graft_git::repo::RepoHandle;

fn open(path: &Path) -> RepoHandle {
    RepoHandle::open(path).unwrap_or_else(|why| panic!("opening {}: {why}", path.display()))
}

fn git(directory: &Path, arguments: &[&str]) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .unwrap_or_else(|why| panic!("running git (Graft does not run without it): {why}"))
}

#[test]
fn a_written_commit_graph_holds_every_commit_and_git_verifies_it() {
    let copy = graft_fixtures::copy_into_temp(Shape::Linear);
    let repo = open(copy.path());
    assert_eq!(
        commit_graph::status(&repo).expect("status reads"),
        CommitGraphStatus::Absent
    );

    let written = commit_graph::write(&repo).expect("git writes a commit-graph");
    assert_eq!(written.commits, 200);
    assert_eq!(
        commit_graph::status(&repo).expect("status reads"),
        CommitGraphStatus::Present { commits: 200 }
    );

    let verify = git(copy.path(), &["commit-graph", "verify"]);
    assert!(
        verify.status.success(),
        "git commit-graph verify: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
}

#[test]
fn a_decline_is_remembered_and_seen_from_a_linked_worktree() {
    let copy = graft_fixtures::copy_into_temp(Shape::Linear);
    let repo = open(copy.path());
    assert_eq!(
        commit_graph::offer_state(&repo).expect("reads"),
        Offer::Undecided
    );

    commit_graph::remember_declined(&repo).expect("the decline writes");
    assert_eq!(
        commit_graph::offer_state(&repo).expect("reads"),
        Offer::Declined
    );

    let worktrees = tempfile::tempdir().expect("a temporary directory");
    let linked = worktrees.path().join("linked");
    let added = git(
        copy.path(),
        &[
            "worktree",
            "add",
            "-b",
            "linked",
            linked.to_str().expect("utf-8 temp path"),
        ],
    );
    assert!(
        added.status.success(),
        "git worktree add: {}",
        String::from_utf8_lossy(&added.stderr)
    );
    let from_worktree = open(&linked);
    assert_eq!(
        commit_graph::offer_state(&from_worktree).expect("reads"),
        Offer::Declined,
        "one decline covers every worktree"
    );
}

#[test]
fn the_decline_lives_under_the_names_in_force() {
    use graft_core::on_disk::{MIGRATED_MARKER, OnDiskNames};

    // Old names: the decline sits where every earlier build wrote it.
    let copy = graft_fixtures::copy_into_temp(Shape::Linear);
    let repo = open(copy.path());
    assert_eq!(repo.on_disk_names(), OnDiskNames::Old);
    commit_graph::remember_declined(&repo).expect("the decline writes");
    let common = repo.common_dir().to_path_buf();
    assert!(common.join("graft").join("commit-graph-offer").is_file());

    // New names: the marker switches the dir; the old file is not read.
    let new_dir = common.join(OnDiskNames::New.dir());
    std::fs::create_dir_all(&new_dir).expect("makes the dir");
    std::fs::write(new_dir.join(MIGRATED_MARKER), "clean\n").expect("writes the marker");
    let repo = open(copy.path());
    assert_eq!(repo.on_disk_names(), OnDiskNames::New);
    assert_eq!(
        commit_graph::offer_state(&repo).expect("reads"),
        Offer::Undecided
    );
    commit_graph::remember_declined(&repo).expect("the decline writes");
    assert!(new_dir.join("commit-graph-offer").is_file());
    assert_eq!(
        commit_graph::offer_state(&repo).expect("reads"),
        Offer::Declined
    );
}

#[test]
fn an_unwritable_object_store_is_git_s_own_failure_not_a_panic() {
    let whoami = Command::new("id")
        .arg("-u")
        .output()
        .unwrap_or_else(|why| panic!("running id: {why}"));
    if String::from_utf8_lossy(&whoami.stdout).trim() == "0" {
        // Root writes through any mode bits; nothing to prove.
        return;
    }
    let copy = graft_fixtures::copy_into_temp(Shape::Linear);
    let repo = open(copy.path());
    let info = repo.common_dir().join("objects").join("info");
    std::fs::create_dir_all(&info).expect("objects/info exists");
    set_mode(&info, 0o555);

    let result = commit_graph::write(&repo);
    // Put the mode back before asserting, so the copy can be deleted.
    set_mode(&info, 0o755);

    match result {
        Err(CommitGraphError::Exited {
            stderr_first_line, ..
        }) => assert!(!stderr_first_line.is_empty()),
        other => panic!("expected git to exit non-zero, got {other:?}"),
    }
}

fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .unwrap_or_else(|why| panic!("chmod {}: {why}", path.display()));
}
