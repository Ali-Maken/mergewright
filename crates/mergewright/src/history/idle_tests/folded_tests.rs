//! Idle work in the folded view (ease-pass 06d): the window sent before the
//! walk knew a merge's fold is sent again once it does, and a hidden commit
//! lands on the shown merge it folds behind.

use mergewright_graph::inclusion::Whereabouts;

use super::super::fold::landing;
use super::*;

/// Views over `handle`'s walk, seeded as the app seeds them.
pub(super) fn seeded<'o>(handle: &'o RepoHandle, pulls: &'o AtomicUsize) -> Counted<'o> {
    let refs = handle.references().expect("refs read");
    let branches = mergewright_git::branches::Branches::resolve(handle, None).expect("resolve");
    counted(handle, seeds_for(&refs, &branches), pulls)
}

/// Every commit of `handle`'s full view, top to bottom.
pub(super) fn every_commit(handle: &RepoHandle) -> Vec<Oid> {
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
pub(super) fn answered_folded(views: &mut Counted<'_>, rows: std::ops::Range<usize>) -> Window {
    let request = RowRequest::new(1, Wanted::Rows(rows), View::folded());
    answered(views, request).0
}

/// Answers `request` through `Views::answer`: its window, and every other
/// update sent on the way.
pub(super) fn answered(
    views: &mut Counted<'_>,
    request: RowRequest,
) -> (Window, Vec<HistoryUpdate>) {
    let mut window = None;
    let mut others = Vec::new();
    let ended = views
        .answer(request, &mut || None, &never, &mut |update| {
            match update {
                HistoryUpdate::Window(sent) => window = Some(sent),
                other => others.push(other),
            }
            Delivery::Received
        })
        .expect("answers");
    assert_eq!(ended, None, "answered");
    (window.expect("the answer sends its window"), others)
}

/// The rows around `commit` in `view`, asked as `History::seek` asks.
fn find(commit: Oid, view: View) -> RowRequest {
    let fallback = 0..3;
    RowRequest::new(
        2,
        Wanted::Find {
            commit,
            rows: 3,
            fallback,
        },
        view,
    )
}

/// A walk in the folded view laid out a row at a time until it has walked a
/// commit the fold hides, short of the end: that commit, and how far the
/// layout got.
fn walked_to_a_folded_commit(views: &mut Counted<'_>, all: &[Oid]) -> (Oid, usize) {
    let mut rows = 1;
    loop {
        answered_folded(views, 0..rows);
        let layout = views.layout(&View::folded()).expect("open");
        assert!(
            !layout.is_done(),
            "a folded commit is walked before the end"
        );
        let folded = all
            .iter()
            .find(|&&id| matches!(layout.source().whereabouts(id), Whereabouts::Folded { .. }));
        if let Some(&id) = folded {
            return (id, layout.rows_emitted());
        }
        rows += 1;
    }
}

#[test]
fn a_find_to_a_walked_folded_commit_lays_out_no_further_and_answers_its_merge() {
    let dir = mergewright_fixtures::borrow(Shape::ManyLanes);
    let handle = RepoHandle::open(&dir).expect("opens");
    let all = every_commit(&handle);
    let pulls = AtomicUsize::new(0);
    let mut views = seeded(&handle, &pulls);
    let (hidden, before) = walked_to_a_folded_commit(&mut views, &all);

    let (window, _) = answered(&mut views, find(hidden, View::folded()));

    let layout = views.layout(&View::folded()).expect("open");
    assert_eq!(layout.rows_emitted(), before, "laid out no further");
    let merge = landing(layout, hidden).expect("folded");
    assert_eq!(window.found(), Some(&Found::Folded { behind: merge }));
    assert!(
        window.rows.iter().any(|row| row.commit.id == merge),
        "the merge's rows come back"
    );
}

#[test]
fn a_find_answers_row_for_a_shown_commit_and_not_in_view_off_the_first_parent_line() {
    let dir = mergewright_fixtures::borrow(Shape::ManyLanes);
    let handle = RepoHandle::open(&dir).expect("opens");
    let all = every_commit(&handle);
    let pulls = AtomicUsize::new(0);
    let mut views = seeded(&handle, &pulls);
    let shown_row = answered_folded(&mut views, 0..4).rows[2].commit.id;
    let (window, _) = answered(&mut views, find(shown_row, View::folded()));
    assert_eq!(window.found(), Some(&Found::Row));

    let first_parent = View {
        follow: mergewright_graph::Follow::FirstParent,
        ..View::folded()
    };
    shown(&mut views, &first_parent, 0..usize::MAX);
    let line = views.layout(&first_parent).expect("open");
    let off = *all
        .iter()
        .find(|&&id| line.row_of(id).is_none())
        .expect("a commit off the first-parent line");
    let (window, _) = answered(&mut views, find(off, first_parent));
    assert_eq!(window.found(), Some(&Found::NotInView));
    assert_eq!(window.first, 0, "the fallback rows");
}

#[test]
fn a_find_answers_how_many_asked_hits_fold_even_when_superseded() {
    let dir = mergewright_fixtures::borrow(Shape::ManyLanes);
    let handle = RepoHandle::open(&dir).expect("opens");
    let all = every_commit(&handle);
    let pulls = AtomicUsize::new(0);
    let mut views = seeded(&handle, &pulls);
    let (hidden, before) = walked_to_a_folded_commit(&mut views, &all);
    let shown_row = answered_folded(&mut views, 0..2).rows[0].commit.id;
    // No commit of this walk: never walked, however far it lays out.
    let unwalked = Oid::from_sha1([0xab; 20]);
    let ask = FoldedAsk {
        search: 7,
        ids: Arc::from([shown_row, hidden, unwalked]),
    };
    let asked = |id, wanted| RowRequest {
        folded: Some(ask.clone()),
        ..RowRequest::new(id, wanted, View::folded())
    };

    // A newer request supersedes the first while it lays out: the ask rides
    // on both, so the newer one's answer carries it.
    let mut newer = Some(asked(4, Wanted::Rows(0..2)));
    let mut others = Vec::new();
    views
        .answer(
            asked(3, Wanted::Rows(0..before + 1)),
            &mut || newer.take(),
            &never,
            &mut |update| {
                if !matches!(
                    update,
                    HistoryUpdate::Window(_) | HistoryUpdate::Progress { .. }
                ) {
                    others.push(update);
                }
                Delivery::Received
            },
        )
        .expect("answers");
    assert_eq!(
        others,
        vec![HistoryUpdate::FoldedHits {
            search: 7,
            folded: 1,
            unwalked: 1
        }]
    );
}

/// Every commit asked as a search's hits, in `view` laid out to the end:
/// how many the answer counts folded.
pub(super) fn folded_of_every_commit(views: &mut Counted<'_>, all: &[Oid], view: View) -> usize {
    shown(views, &view, 0..usize::MAX);
    let request = RowRequest {
        folded: Some(FoldedAsk {
            search: 9,
            ids: Arc::from(all),
        }),
        ..RowRequest::new(5, Wanted::Rows(0..2), view)
    };
    let (_, others) = answered(views, request);
    match others.as_slice() {
        [
            HistoryUpdate::FoldedHits {
                search: 9, folded, ..
            },
        ] => *folded,
        other => panic!("one count answers the ask: {other:?}"),
    }
}

/// First-parent `1` is a view he chose: a hit off its line is not in this
/// view, never "in a folded merge" (ease-pass 06c, search count). The same
/// hits in the folded view do count some folded.
#[test]
fn hits_off_the_first_parent_line_are_not_counted_folded() {
    let dir = mergewright_fixtures::borrow(Shape::ManyLanes);
    let handle = RepoHandle::open(&dir).expect("opens");
    let all = every_commit(&handle);
    let pulls = AtomicUsize::new(0);
    let mut views = seeded(&handle, &pulls);
    let first_parent = View {
        follow: mergewright_graph::Follow::FirstParent,
        ..View::folded()
    };

    assert!(folded_of_every_commit(&mut views, &all, View::folded()) > 0);
    assert_eq!(folded_of_every_commit(&mut views, &all, first_parent), 0);
}

#[test]
fn a_window_sent_before_its_merge_folded_is_sent_again_once_it_does() {
    let dir = mergewright_fixtures::borrow(Shape::FeatureBranches);
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
        let dir = mergewright_fixtures::borrow(shape);
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
