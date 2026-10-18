//! A finish or abort of a merge begun in the shell runs in that merge.
//! Moved out of `tests.rs` to keep that file under its size row.

use super::*;

/// m8-polish-and-ship/13d Q4: finishing or aborting a merge begun in the
/// shell runs only while git is merging, so a stop of it is read against
/// that merge. Neither opens one: recovery and the banner ask
/// `opens_in_progress`, which stays as it was.
#[test]
fn a_finish_or_abort_of_a_shell_merge_runs_in_that_merge() {
    for kind in [OperationKind::FinishMerge, OperationKind::AbortMerge] {
        assert_eq!(runs_in_progress(kind), Some(InProgress::Merge), "{kind:?}");
        assert_eq!(opens_in_progress(kind), None, "{kind:?}");
    }
    assert_eq!(
        runs_in_progress(OperationKind::Merge),
        Some(InProgress::Merge)
    );
    assert_eq!(
        runs_in_progress(OperationKind::Rewrite),
        Some(InProgress::Rebase)
    );
    // Run while a conflict is up, but not only while a kind is live: a
    // stash put back that clashed leaves unmerged paths with none.
    for kind in [
        OperationKind::ResolveConflict,
        OperationKind::RestartConflict,
        OperationKind::Commit,
        OperationKind::Discard,
        OperationKind::Pull,
    ] {
        assert_eq!(runs_in_progress(kind), None, "{kind:?}");
    }
}
