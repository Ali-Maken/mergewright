//! The walk's half of a view change (ease-pass 06h): a request's `keep`
//! answered as a promotion's anchor is, and a folded ask riding on a view
//! change counted under the new view.

use super::super::fold::landing;
use super::folded_tests::{
    answered, answered_folded, every_commit, folded_of_every_commit, seeded,
};
use super::*;

/// A request in the folded view keeping a commit the fold hides: the
/// answer names its merge, and the merge's row is in the window. Keeping a
/// shown commit names nothing.
#[test]
fn a_kept_commit_the_view_folds_away_is_answered_with_its_merge() {
    let dir = mergewright_fixtures::borrow(Shape::ManyLanes);
    let handle = RepoHandle::open(&dir).expect("opens");
    let all = every_commit(&handle);
    let pulls = AtomicUsize::new(0);
    let mut views = seeded(&handle, &pulls);
    answered_folded(&mut views, 0..1);
    idle_until_caught(&mut views, 400);
    let layout = views.layout(&View::folded()).expect("open");
    let (hidden, merge) = all
        .iter()
        .find_map(|&id| Some((id, landing(layout, id)?)))
        .expect("the fold hides a commit");
    let at = layout.row_of(merge).expect("the merge is shown");

    let request = RowRequest {
        keep: Some(hidden),
        ..RowRequest::new(7, Wanted::Rows(at..at + 3), View::folded())
    };
    let (window, _) = answered(&mut views, request);
    let sought = window.sought.as_deref().expect("the walk says where");
    assert_eq!(sought.anchor_behind, Some((hidden, merge)));
    assert!(window.rows.iter().any(|row| row.commit.id == merge));

    let shown = window.rows[0].commit.id;
    let request = RowRequest {
        keep: Some(shown),
        ..RowRequest::new(8, Wanted::Rows(at..at + 3), View::folded())
    };
    let (window, _) = answered(&mut views, request);
    assert_eq!(
        window
            .sought
            .as_deref()
            .and_then(|sought| sought.anchor_behind),
        None
    );
}

/// The count a view change's request carries is the new view's: none
/// folded with nothing folding, fewer with a merge opened.
#[test]
fn a_folded_ask_on_a_view_change_is_counted_under_the_new_view() {
    let dir = mergewright_fixtures::borrow(Shape::ManyLanes);
    let handle = RepoHandle::open(&dir).expect("opens");
    let all = every_commit(&handle);
    let pulls = AtomicUsize::new(0);
    let mut views = seeded(&handle, &pulls);

    let folded = folded_of_every_commit(&mut views, &all, View::folded());
    assert!(folded > 0, "the fold hides something");
    let layout = views.layout(&View::folded()).expect("open");
    let merge = all
        .iter()
        .find_map(|&id| landing(layout, id))
        .expect("a folding merge");
    assert_eq!(
        folded_of_every_commit(&mut views, &all, View::default()),
        0,
        "every branch shown: nothing folded"
    );

    let mut opened = View::folded();
    assert!(opened.inclusion.opened_mut().expect("folds").open(merge));
    let fewer = folded_of_every_commit(&mut views, &all, opened);
    assert!(fewer < folded, "{fewer} of {folded}: its hits not counted");
}
