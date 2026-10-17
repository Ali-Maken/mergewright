//! The scratch index's checks that need no repository.

#![allow(clippy::expect_used, reason = "a failing test should panic")]

use super::*;

fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("mwr-run-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the directory");
    dir
}

/// A copy gone before a later `Scratch` line is a failure in its own words,
/// never an empty index handed to git (review ease-pass-04, Major 1).
#[test]
fn a_scratch_index_gone_mid_plan_is_a_failure_not_an_empty_index() {
    let dir = scratch_dir("gone");
    let path = dir.join("commit-index-1-0");
    std::fs::write(&path, b"DIRC").expect("the copy");
    assert!(still_there(&path).is_ok());

    std::fs::remove_file(&path).expect("another hand removes it");

    let failed = still_there(&path).expect_err("gone is an error");
    let reason = failed.reason();
    assert!(
        reason.starts_with("the commit's index copy is gone: "),
        "{reason}"
    );
    assert!(!reason.contains("could not run git"), "{reason}");
    std::fs::remove_dir_all(&dir).expect("clean up");
}

/// A failed copy says so, not "could not run git" (review ease-pass-04,
/// Minor 3).
#[test]
fn a_failed_copy_is_worded_as_the_copy() {
    let failed = not_run(COPY_FAILED, "Permission denied");
    assert_eq!(
        failed.reason(),
        "could not copy the index for the commit: Permission denied"
    );
}
