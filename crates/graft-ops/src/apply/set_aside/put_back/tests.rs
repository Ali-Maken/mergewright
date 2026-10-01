#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;

use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};

use super::*;

fn refused() -> RunError {
    RunError::Unparsable("git refused".to_owned())
}

fn entry(path: &str) -> StatusEntry {
    StatusEntry {
        path: PathBuf::from(path),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    }
}

fn status(entries: Vec<StatusEntry>) -> WorkingTreeStatus {
    WorkingTreeStatus { entries }
}

/// T8: git failed and a tracked file changed → `Wrote`, which
/// `run_put_back` never follows with a second apply.
#[test]
fn a_failed_apply_that_changed_a_tracked_file_wrote() {
    let changed = status(vec![StatusEntry {
        unstaged: Some(Change::Modified),
        ..entry("f.txt")
    }]);
    assert!(matches!(judge(Err(refused()), &changed), Read::Wrote(_)));
    let staged = status(vec![StatusEntry {
        staged: Some(Change::Added),
        ..entry("n.txt")
    }]);
    assert!(matches!(judge(Err(refused()), &staged), Read::Wrote(_)));
}

#[test]
fn a_failed_apply_with_only_untracked_files_wrote_nothing() {
    let untracked = status(vec![StatusEntry {
        untracked: true,
        ..entry("junk")
    }]);
    assert!(matches!(
        judge(Err(refused()), &untracked),
        Read::WroteNothing(_)
    ));
    assert!(matches!(
        judge(Err(refused()), &status(vec![])),
        Read::WroteNothing(_)
    ));
}

#[test]
fn unmerged_is_a_clash_whatever_the_exit_code() {
    let clash = status(vec![StatusEntry {
        unmerged: true,
        ..entry("f.txt")
    }]);
    assert!(matches!(
        judge(Ok(()), &clash),
        Read::Clashed(paths) if paths == [PathBuf::from("f.txt")]
    ));
    assert!(matches!(judge(Err(refused()), &clash), Read::Clashed(_)));
}

/// A4 (review m7-30-06d Major 1): `M`, `D`, `T` are always named; an
/// `A` only when its content moved on between the index and the tree.
#[test]
fn lost_names_an_added_path_only_when_its_content_moved_on() {
    let printed = b"M\0a.txt\0A\0edited.txt\0D\0gone.txt\0T\0link\0A\0same.txt\0A\0deleted.txt\0";
    let staged = name_status(printed).unwrap();
    let moved_on: BTreeSet<PathBuf> = ["edited.txt", "deleted.txt", "a.txt", "unstaged.txt"]
        .map(PathBuf::from)
        .into();
    assert_eq!(
        lost(&staged, &moved_on),
        ["a.txt", "deleted.txt", "edited.txt", "gone.txt", "link"].map(PathBuf::from)
    );
    assert_eq!(
        lost(&staged, &BTreeSet::new()),
        ["a.txt", "gone.txt", "link"].map(PathBuf::from)
    );
    // Sorted and deduplicated whatever order the input came in.
    let twice = [
        (b'M', PathBuf::from("b")),
        (b'D', PathBuf::from("a")),
        (b'M', PathBuf::from("b")),
    ];
    assert_eq!(
        lost(&twice, &BTreeSet::new()),
        ["a", "b"].map(PathBuf::from)
    );
    assert!(name_status(b"M\0").is_err());
}

#[test]
fn name_only_reads_every_path() {
    assert_eq!(
        name_only(b"a.txt\0dir/b.txt\0"),
        ["a.txt", "dir/b.txt"].map(PathBuf::from).into()
    );
}
