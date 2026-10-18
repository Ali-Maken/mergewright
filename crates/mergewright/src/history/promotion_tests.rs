//! Promotion answered from the provisional layout (m7-power-tools/12 Part C,
//! `KNOWN_GAPS` :161): the walk keeps answering in the order on screen while
//! the promoted layout reaches the anchor between requests, then swaps once.
//! And the race of M7/04b: a promotion that lands before the first window
//! still ends in the state the normal path ends in (12c).

use std::sync::Arc;
use std::sync::mpsc;

use mergewright_core::commit_handle::CommitHandle;
use mergewright_core::oid::Oid;
use mergewright_fixtures::Shape;
use mergewright_git::history::{Order, Walk};
use mergewright_git::repo::RepoHandle;
use mergewright_graph::Seeds;
use mergewright_graph::layout::StreamConfig;

use crate::jobs::Delivery;
use crate::repos::RepoStore;

use super::scroll::Motion;
use super::views::IdleWork;
use super::*;

/// Eight rows a chunk: the 200-commit line takes 25 to lay out.
const SMALL: StreamConfig = StreamConfig {
    chunk_rows: 8,
    checkpoint_every: 16,
    max_checkpoints: 4,
    resident_chunks: 2,
    buffer_rows: 4,
};

const HEIGHT: usize = 10;

fn never() -> bool {
    false
}

/// The Linear shape's walk in commit-date order (provisional), its
/// topological order, and every commit id in that order.
struct Line {
    handle: RepoHandle,
    store: RepoStore,
    order: Arc<[CommitHandle]>,
    ids: Vec<Oid>,
}

fn line() -> Line {
    let dir = mergewright_fixtures::borrow(Shape::Linear);
    let store = RepoStore::new(dir.clone(), None);
    let order = topological_order(&store, &never)
        .expect("sorts")
        .expect("never stopped");
    let table = store.table().expect("table");
    let ids = order.iter().map(|&commit| table.id_of(commit)).collect();
    Line {
        handle: RepoHandle::open(&dir).expect("opens"),
        store,
        order,
        ids,
    }
}

type LineViews<'o> = Views<'o, WalkSource<'o>>;

fn views(line: &Line) -> LineViews<'_> {
    views_with(line, SMALL)
}

fn views_with(line: &Line, config: StreamConfig) -> LineViews<'_> {
    // The table the order's handles were interned against (51b).
    let table = line.store.table().expect("table");
    let handle = &line.handle;
    let open = move |promoted: Option<&Arc<[CommitHandle]>>| {
        Ok(match promoted {
            Some(order) => WalkSource::in_order(handle, Arc::clone(&table), Arc::clone(order)),
            None => WalkSource::new(
                handle,
                Walk::all_refs(handle, Order::CommitDate, Arc::clone(&table))?,
            ),
        })
    };
    Views::new(Seeds::default(), config, Box::new(open))
}

/// A walk that has said it is provisional: a topological order was asked
/// for, commit-date order is what it walks.
fn provisional() -> History {
    let mut history = History::started(crate::config::MergedBranches::Folded);
    history.walk = Some(WalkFacts {
        requested: Order::Topological,
        order: Order::CommitDate,
        default_branch: None,
        integration: Vec::new().into(),
    });
    history
}

/// Serves `request` alone — nobody asks again, so no idle work runs — and
/// returns what it reported, applied to `history`.
fn serve_one(
    views: &mut LineViews<'_>,
    history: &mut History,
    request: RowRequest,
) -> Vec<HistoryUpdate> {
    let (asks, requests) = mpsc::channel();
    asks.send(request).expect("the receiver is right here");
    drop(asks);
    let mut reports = Vec::new();
    let ended = serve(views, &requests, &never, &mut |update| {
        reports.push(update);
        Delivery::Received
    })
    .expect("lays out");
    assert_eq!(ended, Ended::Cancelled, "nobody asks again");
    for update in reports.clone() {
        history.apply(update, HEIGHT);
    }
    reports
}

/// One idle step, its reports applied to `history`.
fn idle_once(views: &mut LineViews<'_>, history: &mut History) -> (IdleWork, Vec<HistoryUpdate>) {
    let mut reports = Vec::new();
    let work = views
        .idle_step(&never, &mut |update| {
            reports.push(update);
            Delivery::Received
        })
        .expect("lays out");
    for update in reports.clone() {
        history.apply(update, HEIGHT);
    }
    (work, reports)
}

/// Idle steps until caught up, every report applied to `history`.
fn idle_to_the_end(views: &mut LineViews<'_>, history: &mut History) {
    for _ in 0..200 {
        let (work, _) = idle_once(views, history);
        if work == IdleWork::Caught {
            return;
        }
    }
    panic!("not caught up after 200 idle steps");
}

fn windows(reports: &[HistoryUpdate]) -> Vec<&Window> {
    reports
        .iter()
        .filter_map(|update| match update {
            HistoryUpdate::Window(window) => Some(window),
            _ => None,
        })
        .collect()
}

/// The first screen answered, the cursor moved to row 35 (on hand), and the
/// sort's order handed over: the request that carries it, not yet served.
fn sorted_with_the_cursor_on_row_35(
    views: &mut LineViews<'_>,
    history: &mut History,
    order: &Arc<[CommitHandle]>,
) -> RowRequest {
    let first = history.show(HEIGHT).expect("asks for the first screen");
    serve_one(views, history, first);
    if let Some(more) = history.move_cursor(Motion::Down(35), HEIGHT) {
        serve_one(views, history, more);
    }
    assert_eq!(history.cursor, 35);
    assert!(history.selected().is_some(), "row 35 is on hand");
    let request = history
        .promote(Arc::clone(order), HEIGHT)
        .expect("a walk still going takes the order");
    assert!(request.promote.is_some());
    request
}

/// Row 35's anchor is past four chunks of the promoted layout. Before Part C
/// the walk laid those out before answering anything; now it answers from
/// the layout on screen, and so does every key meanwhile.
#[test]
fn a_request_during_a_promotion_is_answered_from_the_provisional_layout() {
    let line = line();
    let mut views = views(&line);
    let mut history = provisional();
    let request = sorted_with_the_cursor_on_row_35(&mut views, &mut history, &line.order);

    let reports = serve_one(&mut views, &mut history, request);

    let answered = windows(&reports);
    assert_eq!(answered.len(), 1, "{reports:?}");
    assert!(
        !answered[0].promoted,
        "the rows are the provisional order's"
    );
    assert!(views.is_promoting(), "the promoted layout is still pending");
    assert!(
        matches!(history.promotion, Some(Promotion::Swapping { .. })),
        "the screen still waits for the promoted order"
    );
    let walk = history.walk.as_ref().expect("facts");
    assert_eq!(walk.order, Order::CommitDate, "nothing promoted yet");

    // A key meanwhile carries the order too, and is answered the same way.
    let key = (0..5)
        .find_map(|_| history.move_cursor(Motion::PageDown, HEIGHT))
        .expect("paging down past the rows on hand asks");
    assert!(key.promote.is_some());
    let reports = serve_one(&mut views, &mut history, key);
    let answered = windows(&reports);
    assert_eq!(answered.len(), 1, "{reports:?}");
    assert!(!answered[0].promoted);
    assert!(views.is_promoting());
}

/// Idle steps lay the promoted layout out a chunk at a time, sending
/// nothing, until the anchor has a row; that step swaps it in and answers
/// the last request again from it: one window, in the promoted order, with
/// the selected commit where it was.
#[test]
fn the_promoted_layout_replaces_the_provisional_once_the_anchor_has_a_row() {
    let line = line();
    let mut views = views(&line);
    let mut history = provisional();
    let request = sorted_with_the_cursor_on_row_35(&mut views, &mut history, &line.order);
    serve_one(&mut views, &mut history, request);
    let chosen = history.selected().map(|row| row.commit.id);
    let (cursor, top) = (history.cursor, history.top);

    let mut steps = 0;
    let promoted = loop {
        steps += 1;
        assert!(steps <= 25, "the anchor has a row within 25 chunks");
        let (work, reports) = idle_once(&mut views, &mut history);
        assert!(matches!(work, IdleWork::Promoting { .. }), "{work:?}");
        if let [HistoryUpdate::Window(window)] = reports.as_slice() {
            break window.clone();
        }
        assert!(reports.is_empty(), "nothing sent on the way: {reports:?}");
        assert!(views.is_promoting());
    };

    // Row 35 needs 36 rows laid out: five chunks of eight, not fewer.
    assert!(
        (36_usize.div_ceil(SMALL.chunk_rows)..=36_usize.div_ceil(SMALL.chunk_rows) + 1)
            .contains(&steps),
        "{steps} steps"
    );
    assert!(promoted.promoted);
    assert!(!views.is_promoting(), "swapped in");
    assert_eq!(history.promotion, None, "the promotion has landed");
    let walk = history.walk.as_ref().expect("facts");
    assert_eq!(walk.order, Order::Topological);
    assert_eq!(history.selected().map(|row| row.commit.id), chosen);
    // Linear history numbers every row the same in both orders.
    assert_eq!((history.cursor, history.top), (cursor, top));
    assert_eq!(chosen, Some(line.ids[35]));
}

/// M7/04b's race: the sort's order reaches the screen before the first
/// window, so the first request served carries it with nothing selected.
/// With the app's chunk (500 rows) the 200-commit line fits in one: the
/// normal path swaps in a promoted layout with that chunk laid out and
/// completes. The race must end the same — every row, `Complete`, promoted —
/// not with the 40 rows asked for, `Streaming` for good.
#[test]
fn a_promotion_before_the_first_window_ends_as_the_normal_path_does() {
    let line = line();
    let app = StreamConfig::default();
    assert!(app.chunk_rows >= 200, "the line fits in one chunk");

    let mut raced = provisional();
    let mut race_views = views_with(&line, app);
    let _skipped = raced.show(HEIGHT).expect("asks for the first screen");
    let request = raced
        .promote(Arc::clone(&line.order), HEIGHT)
        .expect("a walk still going takes the order");
    assert!(request.promote.is_some());
    // The worker skips to the newest request: only this one is served.
    serve_one(&mut race_views, &mut raced, request);
    idle_to_the_end(&mut race_views, &mut raced);

    let mut normal = provisional();
    let mut normal_views = views_with(&line, app);
    let first = normal.show(HEIGHT).expect("asks for the first screen");
    serve_one(&mut normal_views, &mut normal, first);
    let request = normal
        .promote(Arc::clone(&line.order), HEIGHT)
        .expect("a walk still going takes the order");
    serve_one(&mut normal_views, &mut normal, request);
    idle_to_the_end(&mut normal_views, &mut normal);

    let header = |history: &History| {
        (
            history.phase.clone(),
            history.rows,
            history.promotion.clone(),
            history.walk.as_ref().map(|walk| walk.order),
            crate::view::order_notice(history),
        )
    };
    assert_eq!(
        header(&normal),
        (Phase::Complete, 200, None, Some(Order::Topological), None)
    );
    assert_eq!(header(&raced), header(&normal));
}
