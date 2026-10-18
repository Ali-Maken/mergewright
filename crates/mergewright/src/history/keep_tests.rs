//! A view change keeps the cursor on its commit, or on the merge that
//! commit now folds behind (ease-pass 06h): the request carries `keep`, the
//! walk answers `Sought::anchor_behind` for it, and the landing is the one a
//! promotion's anchor takes. And the graph mode a walk starts in.

use crate::config::MergedBranches;
use mergewright_graph::Follow;

use super::reveal_tests::{answer, oid};
use super::*;

/// A walk in `merged` whose first window holds rows 1..=5, the cursor on
/// `cursor`.
fn on_screen(merged: MergedBranches, cursor: usize) -> History {
    let mut history = History::started(merged);
    let first = history.show(5).expect("asks the first rows");
    history.apply(answer(first.id, &[1, 2, 3, 4, 5], None), 5);
    history.cursor = cursor;
    history
}

fn behind(anchor: u8, merge: u8) -> Sought {
    Sought {
        anchor_behind: Some((oid(anchor), oid(merge))),
        found: None,
    }
}

#[test]
fn a_walk_starts_in_its_mode_and_the_mode_is_read_from_the_view() {
    let shown = History::started(MergedBranches::Shown);
    assert_eq!(shown.merged_branches(), MergedBranches::Shown);
    assert_eq!(shown.view.inclusion.fold, None);
    assert_eq!(shown.view, View::default());

    let folded = History::started(MergedBranches::Folded);
    assert_eq!(folded.merged_branches(), MergedBranches::Folded);
    assert_eq!(folded.view, View::folded());
}

/// Every branch → folded with the cursor on 4, which the folded view hides
/// behind merge 2: the request keeps 4, the answer says where it went, and
/// the cursor lands on 2 with nothing opened and nothing owed.
#[test]
fn a_view_change_that_folds_the_cursors_commit_away_lands_on_its_merge() {
    let mut history = on_screen(MergedBranches::Shown, 3);
    let request = history
        .change_view(View::folded(), 5)
        .expect("a new view asks");
    assert_eq!(request.keep, Some(oid(4)), "the cursor's commit rides");
    assert!(!matches!(request.wanted, Wanted::Find { .. }), "no seek");

    history.apply(answer(request.id, &[1, 2, 5], Some(behind(4, 2))), 5);

    assert_eq!(history.selected().map(|row| row.commit.id), Some(oid(2)));
    assert_eq!(history.view, View::folded(), "nothing opened");
    assert_eq!(history.take_reveal(), None);
}

/// The kept commit still shown: it is where the cursor lands, wherever its
/// row went.
#[test]
fn a_view_change_keeping_the_cursors_commit_lands_on_it() {
    let mut history = on_screen(MergedBranches::Folded, 3);
    let request = history
        .change_view(View::default(), 5)
        .expect("a new view asks");
    history.apply(answer(request.id, &[1, 2, 6, 7, 4, 5], None), 5);
    assert_eq!(history.selected().map(|row| row.commit.id), Some(oid(4)));
}

/// `1` with the cursor on a commit off the first-parent line: not shown, no
/// `anchor_behind` (it is filtered, not folded), so the cursor keeps its
/// index — the top, where switching `Follow` puts it.
#[test]
fn first_parent_hiding_the_cursors_commit_keeps_its_index() {
    let mut history = on_screen(MergedBranches::Folded, 3);
    let view = View {
        follow: Follow::FirstParent,
        ..View::folded()
    };
    let request = history.change_view(view, 5).expect("a new view asks");
    assert_eq!(request.keep, Some(oid(4)));
    history.apply(answer(request.id, &[1, 2, 5], None), 5);
    assert_eq!(history.cursor, 0);
}

/// A reveal (ease-pass 06c): the view change's own request is replaced by a
/// find for the target; `keep` rides on it, but the find's landing wins —
/// the cursor ends on the target, not pulled back to the old commit.
#[test]
fn a_reveal_lands_on_its_target_not_on_the_kept_commit() {
    let mut history = on_screen(MergedBranches::Folded, 1);
    let changed = history.change_view(View::default(), 5);
    let find = history.seek(oid(9), 5).or(changed).expect("a find");
    assert!(matches!(find.wanted, Wanted::Find { commit, .. } if commit == oid(9)));
    assert_eq!(find.keep, Some(oid(2)));
    let found = Some(Sought {
        anchor_behind: None,
        found: Some(Found::Row),
    });
    history.apply(answer(find.id, &[1, 2, 8, 9, 3], found), 5);
    assert_eq!(history.selected().map(|row| row.commit.id), Some(oid(9)));
    assert_eq!(history.take_reveal(), None);
}

/// A promotion's request carries its anchor, never `keep` too: the walk's
/// one landing rule reads one or the other.
#[test]
fn a_promotion_request_carries_no_keep() {
    let mut history = on_screen(MergedBranches::Folded, 3);
    let changed = history.change_view(View::default(), 5).expect("asks");
    assert_eq!(changed.keep, Some(oid(4)));
    let order: std::sync::Arc<[mergewright_core::commit_handle::CommitHandle]> =
        std::sync::Arc::from([]);
    let promoting = history.promote(order, 5).expect("carries the order");
    assert!(promoting.promote.is_some());
    assert_eq!(promoting.keep, None);
}
