//! `ConflictSet::solved`: which unmerged files the resolver's rule calls
//! solved on disk (ease-pass/05b `file_state`, read by a relaunch in 05c).
//! A marker document with every clash gone is solved; a binary conflict, a
//! modify/delete and a document with a clash left are not.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use crate::conflict_fixtures::{conflict_set, mergewright_merge_stops, paths};
use crate::support::TempRepo;

/// `a.txt` and `b.txt` clash as text, `pic.bin` as binary, and `gone.txt`
/// is modified on `main` and deleted on `feature`.
fn four_shapes() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"1\n2\n3\n");
    repo.write("b.txt", b"x\n");
    repo.write("pic.bin", b"\x00base\x00");
    repo.write("gone.txt", b"kept\n");
    repo.commit_all("base");
    repo.git(&["branch", "feature"]);
    repo.write("a.txt", b"1\nM\n3\n");
    repo.write("b.txt", b"M\n");
    repo.write("pic.bin", b"\x00main\x00");
    repo.write("gone.txt", b"kept and changed\n");
    repo.commit_all("main's side");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("a.txt", b"1\nF\n3\n");
    repo.write("b.txt", b"F\n");
    repo.write("pic.bin", b"\x00feature\x00");
    repo.git(&["rm", "-q", "gone.txt"]);
    repo.commit_all("feature's side");
    repo.git(&["switch", "-q", "main"]);
    repo
}

#[test]
fn a_relaunch_reads_the_files_solved_on_disk_and_nothing_else() {
    let repo = four_shapes();
    mergewright_merge_stops(&repo);
    let set = conflict_set(&repo);
    assert!(
        set.solved.is_empty(),
        "nothing picked yet: {:?}",
        set.solved
    );

    // Every clash of `a.txt` picked (its markers gone), not staged: what the
    // resolver leaves before the merge's `y`.
    repo.write("a.txt", b"1\nM\n3\n");
    let set = conflict_set(&repo);
    assert_eq!(set.solved, paths(&["a.txt"]));
    for open in ["b.txt", "pic.bin", "gone.txt"] {
        assert!(
            set.paths.contains(&std::path::PathBuf::from(open)),
            "{open} is listed: {:?}",
            set.paths
        );
    }
}

#[test]
fn a_staged_file_is_done_not_solved() {
    let repo = four_shapes();
    mergewright_merge_stops(&repo);
    repo.write("a.txt", b"1\nM\n3\n");
    repo.git(&["add", "a.txt"]);
    let set = conflict_set(&repo);
    assert!(set.solved.is_empty(), "{:?}", set.solved);
}
