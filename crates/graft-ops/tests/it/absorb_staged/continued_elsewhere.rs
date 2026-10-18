//! An absorb stopped, then continued elsewhere: the staging is said gone.
//! Moved out of `absorb_staged.rs` to keep that file under its size row.

use super::*;

/// M1: `c` after amending `f3` at the stop lands on a tree the plan did not
/// name, so `S` itself goes back with `--index`. `B3` is already in `HEAD`,
/// so git cannot reinstate the index and writes nothing: the plain apply
/// runs, the words name what lost its staging, and `u` puts back the split
/// exactly (ADR-0045 amendment 2026-09-26 b).
#[test]
fn absorb_stopped_then_continued_elsewhere_says_the_staging_went() {
    let repo = branch();
    stage_placed(&repo);
    index_only_stay(&repo);
    let before = state_of(&repo);
    absorb_stopped_at_f3(&repo);
    repo.write("a.txt", b"a1 amended\na2\na3\na4\na5\n");
    repo.git(&["commit", "-q", "--amend", "--no-edit", "-a"]);

    let (applied, unread) =
        match resume_with(&repo, Resume::Continue { solved: Vec::new() }, NOW + 1) {
            Err(ApplyError::AfterState { applied, unread }) => (*applied, unread),
            other => panic!("the staging should not all come back: {other:?}"),
        };

    assert_eq!(applied.entry.status, EntryStatus::Committed);
    let [AfterState::StagingNotKept { paths }] = unread.as_slice() else {
        panic!("{unread:?}");
    };
    assert_eq!(paths, &["b.txt", "old.txt"].map(std::path::PathBuf::from));
    f2_carries_b3(&repo);
    assert_eq!(
        stash_list(&repo),
        "",
        "S dropped: the backup holds the split"
    );
    run_undo(&repo, NOW + 2, false).unwrap_or_else(|why| panic!("{why:?}"));
    assert_eq!(state_of(&repo), before, "undo exact, O3 staged again");
}
