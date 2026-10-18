//! The history worker laying out ahead while no request waits
//! (m7-power-tools/12 Part B): [`Views::idle_step`] over a source that
//! counts its pulls, and `serve`'s loop around it.

use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;

use graft_core::commit::Commit;
use graft_core::commit_handle::{CommitHandle, Link};
use graft_core::oid::Oid;
use graft_fixtures::Shape;
use graft_git::history::{CommitTable, Order, Walk};
use graft_git::repo::RepoHandle;
use graft_graph::Seeds;
use graft_graph::layout::{CommitSource, Pull, StreamConfig};

use crate::jobs::Delivery;

use super::views::IdleWork;
use super::*;

/// Small enough that the 200-commit line takes 25 chunks to lay out.
const SMALL: StreamConfig = StreamConfig {
    chunk_rows: 8,
    checkpoint_every: 16,
    max_checkpoints: 4,
    resident_chunks: 2,
    buffer_rows: 4,
};

fn never() -> bool {
    false
}

/// A source that counts every pull asked of it.
struct Counting<'c, S> {
    inner: S,
    pulls: &'c AtomicUsize,
}

impl<S: CommitSource> CommitSource for Counting<'_, S> {
    type Error = S::Error;

    fn pull(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, Self::Error> {
        self.pulls.fetch_add(1, Ordering::SeqCst);
        self.inner.pull(limit, stop)
    }

    fn read(&mut self, commits: &[CommitHandle]) -> Result<Vec<Commit>, Self::Error> {
        self.inner.read(commits)
    }

    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, Self::Error> {
        self.inner.links(commits)
    }

    fn handle_of(&self, id: &Oid) -> Option<CommitHandle> {
        self.inner.handle_of(id)
    }

    fn id_of(&self, handle: CommitHandle) -> Oid {
        self.inner.id_of(handle)
    }
}

type Counted<'o> = Views<'o, Counting<'o, WalkSource<'o>>>;

/// Views of `handle`'s walk whose sources count into `pulls`.
fn counted<'o>(handle: &'o RepoHandle, seeds: Seeds, pulls: &'o AtomicUsize) -> Counted<'o> {
    let open = move |_: Option<&Arc<[CommitHandle]>>| {
        let table = Arc::new(CommitTable::open(handle).expect("the table opens"));
        Ok(Counting {
            inner: WalkSource::new(handle, Walk::all_refs(handle, Order::Topological, table)?),
            pulls,
        })
    };
    Views::new(seeds, SMALL, Box::new(open))
}

/// Answers `rows` in `view`, as the first screen's request would be: the
/// view the worker then lays out ahead in.
fn shown(views: &mut Counted<'_>, view: &View, rows: std::ops::Range<usize>) {
    let request = RowRequest::new(1, Wanted::Rows(rows), view.clone());
    let layout = views.layout(view).expect("opens");
    let ended = answer(layout, request, &mut || None, &never, &mut |_| {
        Delivery::Received
    })
    .expect("lays out");
    assert_eq!(ended, None, "answered");
}

/// Idle steps until one returns `Caught`; every update reported on the way,
/// and how many steps advanced. Panics past `limit` steps.
fn idle_until_caught(views: &mut Counted<'_>, limit: usize) -> (Vec<HistoryUpdate>, usize) {
    let mut reports = Vec::new();
    for steps in 0..limit {
        let work = views
            .idle_step(&never, &mut |update| {
                reports.push(update);
                Delivery::Received
            })
            .expect("lays out");
        match work {
            IdleWork::Advanced { rows } => {
                assert!(rows <= SMALL.chunk_rows, "one step is at most a chunk");
            }
            IdleWork::Caught => return (reports, steps),
            IdleWork::Promoting { .. } => panic!("no promotion was asked for"),
            IdleWork::NobodyListening => panic!("the test listens"),
        }
    }
    panic!("not caught up after {limit} idle steps");
}

#[test]
fn idle_steps_reach_the_end_of_history_then_return_caught() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let pulls = AtomicUsize::new(0);
    let mut views = counted(&handle, Seeds::default(), &pulls);
    let view = View::default();
    shown(&mut views, &view, 0..10);
    assert!(views.layout(&view).expect("open").rows_emitted() < 200);

    let (_, steps) = idle_until_caught(&mut views, 200);

    let layout = views.layout(&view).expect("open");
    assert!(layout.is_done(), "laid out to the end of history");
    assert_eq!(layout.rows_emitted(), 200);
    assert!(
        (190 / SMALL.chunk_rows..=190 / SMALL.chunk_rows + 2).contains(&steps),
        "a chunk per step: {steps} steps for 190 rows"
    );
}

#[test]
fn a_caught_up_idle_step_pulls_nothing() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let pulls = AtomicUsize::new(0);
    let mut views = counted(&handle, Seeds::default(), &pulls);
    shown(&mut views, &View::default(), 0..10);
    idle_until_caught(&mut views, 200);
    let before = pulls.load(Ordering::SeqCst);

    for _ in 0..3 {
        let work = views
            .idle_step(&never, &mut |update| {
                panic!("a caught-up step reports nothing: {update:?}")
            })
            .expect("nothing to lay out");
        assert!(matches!(work, IdleWork::Caught), "{work:?}");
    }
    assert_eq!(
        pulls.load(Ordering::SeqCst),
        before,
        "no pull once caught up"
    );
}

#[test]
fn an_idle_step_before_any_request_opens_nothing() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let pulls = AtomicUsize::new(0);
    let mut views = counted(&handle, Seeds::default(), &pulls);
    let work = views
        .idle_step(&never, &mut |update| panic!("nothing to say: {update:?}"))
        .expect("nothing to lay out");
    assert!(matches!(work, IdleWork::Caught), "{work:?}");
    assert_eq!(pulls.load(Ordering::SeqCst), 0);
}

/// `stop` says "a request is waiting" as soon as the step's first pull has
/// begun: the step ends inside that pull.
#[test]
fn a_request_preempts_the_idle_step_within_one_pull() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let pulls = AtomicUsize::new(0);
    let mut views = counted(&handle, Seeds::default(), &pulls);
    shown(&mut views, &View::default(), 0..10);
    let before = pulls.load(Ordering::SeqCst);

    let waiting = || pulls.load(Ordering::SeqCst) > before;
    let work = views
        .idle_step(&waiting, &mut |update| panic!("nothing to say: {update:?}"))
        .expect("lays out");

    assert!(matches!(work, IdleWork::Advanced { .. }), "{work:?}");
    assert_eq!(
        pulls.load(Ordering::SeqCst) - before,
        1,
        "one pull, no more"
    );
}

/// Through `serve`: the walk rests between requests with its sender still
/// held, and a cancel ends it within one pull of being asked.
#[test]
fn a_cancel_stops_the_idle_step_within_one_pull() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let pulls = AtomicUsize::new(0);
    let mut views = counted(&handle, Seeds::default(), &pulls);
    let (asks, requests) = mpsc::channel();
    asks.send(RowRequest::new(1, Wanted::Rows(0..10), View::default()))
        .expect("the receiver is right here");

    // Cancelled once the idle advance has made one pull past the answer.
    let answered_at = Cell::new(None::<usize>);
    let cancelled = AtomicBool::new(false);
    let stop = || {
        if let Some(at) = answered_at.get()
            && pulls.load(Ordering::SeqCst) > at
        {
            cancelled.store(true, Ordering::SeqCst);
        }
        cancelled.load(Ordering::SeqCst)
    };
    let ended = serve(&mut views, &requests, &stop, &mut |update| {
        if matches!(update, HistoryUpdate::Window(_)) {
            answered_at.set(Some(pulls.load(Ordering::SeqCst)));
        }
        Delivery::Received
    })
    .expect("lays out");

    assert_eq!(ended, Ended::Cancelled);
    let at = answered_at.get().expect("the request was answered");
    assert_eq!(
        pulls.load(Ordering::SeqCst) - at,
        1,
        "stopped within one pull"
    );
    assert!(
        views.layout(&View::default()).expect("open").rows_emitted() < 200,
        "not laid out to the end"
    );
    drop(asks);
}

#[test]
fn idle_steps_send_no_event() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let pulls = AtomicUsize::new(0);
    let mut views = counted(&handle, Seeds::default(), &pulls);
    shown(&mut views, &View::default(), 0..10);

    let (mut reports, _) = idle_until_caught(&mut views, 200);
    for _ in 0..3 {
        let (more, _) = idle_until_caught(&mut views, 1);
        reports.extend(more);
    }

    assert_eq!(
        reports,
        [HistoryUpdate::LaidOut { rows: 200 }],
        "only `LaidOut`, once"
    );
}

/// Switching to the folded view leaves it unfinished after one row; the next
/// idle steps lay it out to its end (mending its repairs) and say so.
#[test]
fn an_inclusion_change_resumes_the_idle_step() {
    let shape = Shape::FoxtrotMerge;
    let dir = graft_fixtures::borrow(shape);
    let handle = RepoHandle::open(&dir).expect("opens");
    let refs = handle.references().expect("refs read");
    let branches = graft_git::branches::Branches::resolve(&handle, None).expect("resolve");
    let pulls = AtomicUsize::new(0);
    let mut views = counted(&handle, seeds_for(&refs, &branches), &pulls);
    let full = View::default();
    shown(&mut views, &full, 0..1);
    let (first, _) = idle_until_caught(&mut views, 200);
    let all = views.layout(&full).expect("open").rows_emitted();
    assert_eq!(first, [HistoryUpdate::LaidOut { rows: all }]);

    // Folding by default is a view of its own; asking it for one row leaves
    // the rest of the folded view to lay out.
    let folded = View::folded();
    shown(&mut views, &folded, 0..1);
    assert!(
        !views.layout(&folded).expect("open").is_done(),
        "the fold moved the frontier back"
    );

    let (again, _) = idle_until_caught(&mut views, 200);

    let layout = views.layout(&folded).expect("open");
    assert!(layout.is_done());
    let rows = layout.rows_emitted();
    assert!(rows < all, "folding hides the merged branch");
    assert_eq!(again, [HistoryUpdate::LaidOut { rows }]);
}

/// The whole loop on a thread, as the pool runs it: a request is answered,
/// the walk lays out to the end unasked and says so once, then answers the
/// next request from what it laid out — `G` pulls nothing.
#[test]
fn serve_lays_out_to_the_end_between_requests() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let (asks, requests) = mpsc::channel();
    let (told, updates) = mpsc::channel();
    let pulls = AtomicUsize::new(0);
    let counter = &pulls;
    std::thread::scope(|scope| {
        // The receiver is not `Sync`: it moves to the worker, as in the pool.
        let worker = scope.spawn(|| {
            let (requests, told) = (requests, told);
            let handle = RepoHandle::open(&dir).expect("opens");
            let mut views = counted(&handle, Seeds::default(), counter);
            serve(
                &mut views,
                &requests,
                &never,
                &mut |update| match told.send(update) {
                    Ok(()) => Delivery::Received,
                    Err(_) => Delivery::NobodyListening,
                },
            )
        });
        let ask = |id, wanted| RowRequest::new(id, wanted, View::default());
        asks.send(ask(1, Wanted::Rows(0..10)))
            .expect("the worker listens");
        let next = || updates.recv().expect("the worker reports");
        // Request 1 reports progress on its way to rows 0..10; then its rows.
        loop {
            match next() {
                HistoryUpdate::Progress { request: 1, .. } => {}
                HistoryUpdate::Window(window) if window.request == 1 => break,
                other => panic!("before request 1's rows: {other:?}"),
            }
        }
        assert_eq!(
            next(),
            HistoryUpdate::LaidOut { rows: 200 },
            "laid out to the end unasked, with nothing sent on the way"
        );
        let before = pulls.load(Ordering::SeqCst);

        asks.send(ask(2, Wanted::Last(10)))
            .expect("the worker listens");
        match next() {
            HistoryUpdate::Window(window) => {
                assert_eq!(window.request, 2);
                assert_eq!(window.end(), 200, "the last rows");
            }
            other => panic!("`G` answered at once, no progress first: {other:?}"),
        }
        assert_eq!(pulls.load(Ordering::SeqCst), before, "`G` pulled nothing");

        drop(asks);
        let ended = worker.join().expect("the worker returns");
        assert_eq!(ended.expect("lays out"), Ended::Cancelled);
    });
    assert!(
        updates.try_recv().is_err(),
        "nothing after the second answer"
    );
}

mod folded_tests;
