//! Idle work in the folded view (ease-pass 06d): the window sent before the
//! walk knew a merge's fold is sent again once it does, and a hidden commit
//! lands on the shown merge it folds behind.

use super::super::fold::landing;
use super::*;

/// Views over `handle`'s walk, seeded as the app seeds them.
fn seeded<'o>(handle: &'o RepoHandle, pulls: &'o AtomicUsize) -> Counted<'o> {
    let refs = handle.references().expect("refs read");
    let branches = graft_git::branches::Branches::resolve(handle, None).expect("resolve");
    counted(handle, seeds_for(&refs, &branches), pulls)
}

/// Every commit of `handle`'s full view, top to bottom.
fn every_commit(handle: &RepoHandle) -> Vec<Oid> {
    let pulls = AtomicUsize::new(0);
    let mut views = seeded(handle, &pulls);
    shown(&mut views, &View::default(), 0..1);
    idle_until_caught(&mut views, 400);
    let full = views.layout(&View::default()).expect("open");
    let rows = full.rows_emitted();
    let rows = full.rows(0..rows).expect("rows read");
    rows.iter().map(|row| row.commit.id).collect()
}

/// Asks for `rows` of the folded view through `Views::answer`, as `serve`
/// does: the window it sends is the one idle work keeps up to date.
fn answered_folded(views: &mut Counted<'_>, rows: std::ops::Range<usize>) -> Window {
    let request = RowRequest {
        id: 1,
        wanted: Wanted::Rows(rows),
        view: View::folded(),
        promote: None,
    };
    let mut window = None;
    let ended = views
        .answer(request, &mut || None, &never, &mut |update| {
            if let HistoryUpdate::Window(sent) = update {
                window = Some(sent);
            }
            Delivery::Received
        })
        .expect("answers");
    assert_eq!(ended, None, "answered");
    window.expect("the answer sends its window")
}

#[test]
fn a_window_sent_before_its_merge_folded_is_sent_again_once_it_does() {
    let dir = graft_fixtures::borrow(Shape::FeatureBranches);
    let handle = RepoHandle::open(&dir).expect("opens");
    let pulls = AtomicUsize::new(0);
    let mut views = seeded(&handle, &pulls);

    // Row 1 is `Merge branch 'feature/two'`: two rows in, the walk has not
    // reached its side yet, so whether it folds is not known.
    let first = answered_folded(&mut views, 0..2);
    assert_eq!(first.fold[1].folds, None, "not known when first sent");

    let (reports, _) = idle_until_caught(&mut views, 200);

    let again: Vec<&Window> = reports
        .iter()
        .filter_map(|update| match update {
            HistoryUpdate::Window(window) => Some(window),
            _ => None,
        })
        .collect();
    assert_eq!(again.len(), 1, "sent again once: {reports:?}");
    let again = again[0];
    assert_eq!(again.request, 1, "under the same request id");
    assert_eq!(again.first, first.first);
    assert_eq!(again.rows, first.rows, "the same rows");
    assert_eq!(again.fold[1].folds, Some(true), "now known to fold");
}

#[test]
fn every_hidden_commit_lands_on_a_shown_merge_and_a_shown_commit_on_none() {
    for shape in [
        Shape::FeatureBranches,
        Shape::CrissCrossMerge,
        Shape::OctopusMerge,
        Shape::ManyLanes,
    ] {
        let dir = graft_fixtures::borrow(shape);
        let handle = RepoHandle::open(&dir).expect("opens");
        let all = every_commit(&handle);
        let pulls = AtomicUsize::new(0);
        let mut views = seeded(&handle, &pulls);
        answered_folded(&mut views, 0..1);
        idle_until_caught(&mut views, 400);
        let layout = views.layout(&View::folded()).expect("open");
        assert!(layout.is_done());

        let mut hidden = 0;
        for id in all {
            match (layout.row_of(id), landing(layout, id)) {
                (Some(_), None) => {}
                (None, Some(merge)) => {
                    hidden += 1;
                    let at = layout.row_of(merge).expect("lands on a shown row");
                    let row = layout.rows(at..at + 1).expect("row read");
                    assert!(row[0].is_merge(), "{shape:?}: lands on a merge");
                }
                other => panic!("{shape:?}: {id:?} shown and landing {other:?}"),
            }
        }
        assert!(hidden > 0, "{shape:?}: the fold hides something");
    }
}
