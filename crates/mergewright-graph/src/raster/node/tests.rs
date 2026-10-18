use super::Ring;
use crate::Seeds;
use crate::layout::layout;
use crate::layout::tests::commit;
use crate::row::Row;

/// Merge 4 of `main` 2 and `topic` 3 off root 1. Rows: 4, 3, 2, 1.
fn merged() -> Vec<Row> {
    let commits = [
        commit(4, &[2, 3], 40, b"Merge branch 'topic'"),
        commit(3, &[1], 30, b"topic: work"),
        commit(2, &[1], 20, b"main: work"),
        commit(1, &[], 0, b"root"),
    ];
    layout(&commits, Seeds::default())
}

/// ease-pass 06r M2, the pixel twin of mergewright-ui's
/// `a_merge_whose_side_is_not_drawn_is_still_a_merge`: a merge folded by
/// default (or trimmed by the first-parent view) draws one parent and is
/// still a merge, because the ring reads git's parents (06 D1).
#[test]
fn a_merge_whose_side_is_not_drawn_is_still_a_merge() {
    let mut folded = merged()[0].clone();
    assert_eq!(folded.commit.parents.len(), 2, "row 0 is the merge");
    folded.drawn_parents.truncate(1);
    folded.outgoing.truncate(1);
    assert_eq!(Ring::of(&folded), Ring::Merge);
}

/// The rest of the shape is git's parents too: one is a commit, none a root.
#[test]
fn one_parent_is_a_commit_and_none_a_root() {
    let rows = merged();
    assert_eq!(Ring::of(&rows[1]), Ring::Commit);
    assert_eq!(Ring::of(&rows[3]), Ring::Root);
}
