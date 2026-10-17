//! The product source, seeds and streaming driver, on the fixture corpus.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use graft_core::commit::Commit;
use graft_core::commit_handle::CommitHandle;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_fixtures::Shape;
use graft_git::branches::Branches;
use graft_git::history::{CHUNK, CommitTable, Order, Walk};
use graft_git::repo::{RefTarget, RepoHandle};
use graft_graph::layout::{CommitSource, StreamConfig, StreamingLayout, layout};
use graft_graph::{BranchIdentity, Follow, Included, Inclusion, Row, Seeds};

use crate::jobs::Delivery;
use crate::repos::RepoStore;

use super::*;

/// Small enough that every corpus shape spans several chunks and overflows
/// the resident budget.
const SMALL: StreamConfig = StreamConfig {
    chunk_rows: 8,
    checkpoint_every: 16,
    max_checkpoints: 4,
    resident_chunks: 2,
    buffer_rows: 4,
};

/// The repository's id table, as the store shares it.
fn table(handle: &RepoHandle) -> Arc<CommitTable> {
    Arc::new(CommitTable::open(handle).expect("the table opens"))
}

fn never() -> bool {
    false
}

/// Every commit the refs reach, in the order the app asks for.
fn every_commit(handle: &RepoHandle, shape: Shape) -> Vec<Commit> {
    let mut walk = Walk::all_refs(handle, Order::Topological, table(handle))
        .unwrap_or_else(|why| panic!("starting the walk of {shape}: {why}"));
    let mut commits = Vec::new();
    loop {
        let chunk = walk
            .next_chunk(CHUNK, &never)
            .unwrap_or_else(|why| panic!("walking {shape}: {why}"));
        commits.extend(chunk.commits);
        if chunk.done {
            return commits;
        }
    }
}

/// `source` laid out as a view that hides nothing, in `SMALL` chunks: what
/// `answer` lays out (it mends a folding view's repairs as it goes).
fn unfolded<S: CommitSource>(source: S, seeds: Seeds) -> StreamingLayout<Included<S>> {
    let included = Included::new(source, seeds, Follow::AllParents, Inclusion::default());
    let seeds = included.seeds().clone();
    StreamingLayout::new(included, seeds, SMALL)
}

/// Asks `layout` for every row there is a chunk at a time, as a screen
/// paging down would, collecting the answers.
fn reported_rows<S: CommitSource>(
    layout: &mut StreamingLayout<Included<S>>,
    shape: Shape,
) -> Vec<Row> {
    let chunk = layout.config().chunk_rows;
    let mut rows: Vec<Row> = Vec::new();
    for id in 1.. {
        let request = RowRequest {
            id,
            wanted: Wanted::Rows(rows.len()..rows.len() + chunk),
            view: View::default(),
            promote: None,
        };
        let mut done = false;
        let ended = answer(layout, request, &mut || None, &never, &mut |update| {
            if let HistoryUpdate::Window(window) = update {
                rows.extend(window.rows.iter().cloned());
                done = window.done && window.end() == window.laid_out;
            }
            Delivery::Received
        })
        .unwrap_or_else(|why| panic!("{shape}: laying out: {why}"));
        assert_eq!(ended, None, "{shape}: answered and still listening");
        if done {
            break;
        }
    }
    rows
}

/// Calls `visit` on every corpus shape this machine can hold; the one known
/// reason a shape is unavailable (a non-UTF-8 ref name on APFS) is said, not
/// passed over.
fn for_each_shape(mut visit: impl FnMut(Shape, &Path)) {
    for &shape in Shape::ALL {
        match graft_fixtures::try_borrow(shape) {
            Ok(dir) => visit(shape, &dir),
            Err(why) => {
                let reason = why.to_string();
                assert!(reason.contains("not valid UTF-8"), "{shape}: {reason}");
                eprintln!("skipped: {shape} — {reason}");
            }
        }
    }
}

/// The product source, streamed through a cache the corpus overflows, reports
/// exactly the rows the test source's whole layout has: ids, lanes and edges
/// (`graft-graph/tests/it/streaming.rs` proves that layout equals its
/// streamed test source on the same shapes).
#[test]
fn the_product_source_lays_out_every_shape_as_the_whole_layout_does() {
    for_each_shape(|shape, dir| {
        let handle = RepoHandle::open(dir).unwrap_or_else(|why| panic!("{shape}: {why}"));
        let refs = handle.references().expect("refs read");
        let branches = Branches::resolve(&handle, None).expect("branches resolve");
        let seeds = seeds_for(&refs, &branches);
        let whole = layout(&every_commit(&handle, shape), seeds.clone());

        let walk =
            Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("walk starts");
        let mut stream = unfolded(WalkSource::new(&handle, walk), seeds);
        let rows = reported_rows(&mut stream, shape);

        assert_eq!(rows, whole, "{shape}");
        assert_eq!(
            stream.work().chunks_rebuilt,
            0,
            "{shape}: reporting a fresh chunk reads the cache, it never rebuilds"
        );
    });
}

/// The seed builder's choice, on the shape that needs it: a remote ahead of
/// its local branch is a known tip under its own name.
#[test]
fn seeds_anchor_the_default_branch_and_know_remote_tips() {
    let dir = graft_fixtures::borrow(Shape::RefStates);
    let handle = RepoHandle::open(&dir).expect("opens");
    let refs = handle.references().expect("refs read");
    let branches = Branches::resolve(&handle, None).expect("branches resolve");
    let seeds = seeds_for(&refs, &branches);

    let default = branches.default_branch().expect("a default branch");
    let seed = seeds.default_branch.expect("lane 0 is seeded");
    assert_eq!(seed.identity, BranchIdentity::of_name(default.shorthand()));
    let remotes = refs
        .iter()
        .filter(|entry| entry.name.as_bytes().starts_with(b"refs/remotes/"))
        .filter(|entry| matches!(entry.target, RefTarget::Object(_)))
        .count();
    assert!(remotes > 0, "the ref-states shape has remote-tracking refs");
    let remote_seeds = seeds
        .known_branches
        .iter()
        .filter(|known| {
            refs.iter().any(|entry| {
                entry.name.as_bytes().starts_with(b"refs/remotes/")
                    && BranchIdentity::of_name(entry.name.shorthand()) == known.identity
            })
        })
        .count();
    assert_eq!(remote_seeds, remotes);
}

/// `graph.default_branch` reaches `Branches::resolve` through the store and
/// takes lane 0 from the branch detection would have chosen.
#[test]
fn a_configured_default_branch_takes_lane_0() {
    let dir = graft_fixtures::borrow(Shape::FeatureBranches);
    let name = RefName::new("refs/heads/feature/one").expect("valid");
    let handle = RepoHandle::open(&dir).expect("opens");
    let tip = handle
        .references()
        .expect("refs read")
        .into_iter()
        .find_map(|entry| match entry.target {
            RefTarget::Object(id) if entry.name == name => Some(id),
            _ => None,
        })
        .expect("the fixture has feature/one");

    let lane_of_tip = |override_name: Option<RefName>| {
        let store = RepoStore::new(dir.clone(), override_name);
        let mut facts = None;
        let mut rows = Vec::new();
        let (asks, requests) = mpsc::channel();
        let everything = RowRequest {
            id: 1,
            wanted: Wanted::Rows(0..usize::MAX),
            view: View::default(),
            promote: None,
        };
        asks.send(everything).expect("the receiver is right here");
        // Nobody asks again, so the walk ends once it has answered.
        drop(asks);
        let ended = stream(
            &store,
            Order::Topological,
            &requests,
            &never,
            &mut |update| {
                match update {
                    HistoryUpdate::Started(started) => facts = Some(*started),
                    HistoryUpdate::Window(window) => rows.extend(window.rows.iter().cloned()),
                    HistoryUpdate::Progress { .. } | HistoryUpdate::LaidOut { .. } => {}
                }
                Delivery::Received
            },
        )
        .expect("the walk streams");
        assert_eq!(ended, Ended::Cancelled);
        let lane = rows
            .iter()
            .find(|row| row.commit.id == tip)
            .expect("the tip has a row")
            .lane;
        (facts.expect("the walk said what it is"), lane)
    };

    let (detected, detected_lane) = lane_of_tip(None);
    assert_ne!(detected.default_branch.as_ref(), Some(&name));
    assert_ne!(
        detected_lane, 0,
        "without the setting, feature/one is not lane 0"
    );

    let (configured, lane) = lane_of_tip(Some(name.clone()));
    assert_eq!(configured.default_branch, Some(name));
    assert_eq!(lane, 0);
}

/// How many rows the viewport shows in these tests.
const HEIGHT: usize = 10;

/// Answers `request` against `layout` and applies everything it reports to
/// `history`, as the job and `update` would between them. `newer` stands in
/// for requests that arrive while it works.
fn round_trip<S: CommitSource>(
    layout: &mut StreamingLayout<Included<S>>,
    history: &mut History,
    request: RowRequest,
    newer: &mut dyn FnMut() -> Option<RowRequest>,
) -> Vec<HistoryUpdate> {
    let mut reports = Vec::new();
    let ended = answer(layout, request, newer, &never, &mut |update| {
        reports.push(update);
        Delivery::Received
    })
    .expect("lays out");
    assert_eq!(ended, None);
    for update in reports.clone() {
        history.apply(update, HEIGHT);
    }
    reports
}

/// Paging from the first row to the last keeps both halves bounded: the rows
/// the layout holds, and the window the screen holds. Every row shown is the
/// row the whole layout puts there.
#[test]
fn scrolling_top_to_bottom_keeps_resident_rows_bounded() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let whole = layout(&every_commit(&handle, Shape::Linear), Seeds::default());
    let walk = Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("walk starts");
    let mut stream = unfolded(WalkSource::new(&handle, walk), Seeds::default());

    // What the layout may protect at once: the window the screen asks for
    // (seven screens), back to a checkpoint, plus its buffers — in whole
    // chunks — and the two chunks its budget lets it keep besides.
    let protected =
        (7 * HEIGHT + SMALL.checkpoint_every + 2 * SMALL.buffer_rows).div_ceil(SMALL.chunk_rows);
    let bound = (protected + SMALL.resident_chunks) * SMALL.chunk_rows;
    assert!(bound < whole.len(), "the bound is below the history's size");

    let mut history = History::started();
    let mut request = history.show(HEIGHT);
    let mut pages = 0;
    while history.phase != Phase::Complete || history.cursor + 1 < history.rows {
        if let Some(asked) = request.take() {
            round_trip(&mut stream, &mut history, asked, &mut || None);
        }
        let held = stream.resident().rows;
        assert!(
            held <= bound,
            "page {pages}: layout holds {held} rows, bound {bound}"
        );
        let window = history.window.as_ref().expect("answered");
        assert!(
            window.rows.len() <= 7 * HEIGHT,
            "page {pages}: window {}",
            window.rows.len()
        );

        let visible = history.visible(HEIGHT);
        assert_eq!(visible.len(), HEIGHT.min(history.rows - history.top));
        for (offset, row) in visible.iter().enumerate() {
            assert_eq!(
                row,
                &whole[history.top + offset],
                "page {pages}, row {offset}"
            );
        }

        request = history.move_cursor(Motion::PageDown, HEIGHT);
        pages += 1;
        assert!(pages < whole.len(), "paging reaches the end");
    }
    assert_eq!(history.rows, whole.len());
    assert_eq!(history.cursor, whole.len() - 1);
}

/// PROMPT.md's third principle: a key never waits on the worker. Each key's
/// own frame is drawn from the rows already on hand — the cursor's row and
/// the whole viewport — even when the worker's answer lands a key late, and
/// even when the next key asks again before it lands (the newest request
/// wins, so an answer overtaken by a newer request never arrives).
#[test]
fn every_key_draws_from_rows_on_hand_while_the_worker_answers_a_key_late() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let walk = Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("walk starts");
    let mut stream = unfolded(WalkSource::new(&handle, walk), Seeds::default());

    let mut history = History::started();
    let first = history.show(HEIGHT).expect("asks for the first screen");
    round_trip(&mut stream, &mut history, first, &mut || None);

    let mut keys = vec![Motion::Down(1); 3 * HEIGHT];
    keys.extend([Motion::PageDown; 8]);
    keys.extend([Motion::PageUp; 8]);
    let mut pending: Option<RowRequest> = None;
    let mut asked = 0;
    for (index, motion) in keys.into_iter().enumerate() {
        let request = history.move_cursor(motion, HEIGHT);
        assert!(
            history.selected().is_some(),
            "key {index} ({motion:?}): the cursor's row {} is not on hand",
            history.cursor
        );
        let visible = history.visible(HEIGHT).len();
        assert_eq!(
            visible,
            HEIGHT.min(history.rows - history.top),
            "key {index} ({motion:?}): viewport at {} not on hand",
            history.top
        );
        match request {
            // Overtakes the one still out; its answer lands after a later key.
            Some(newer) => {
                asked += 1;
                pending = Some(newer);
            }
            None => {
                if let Some(late) = pending.take() {
                    round_trip(&mut stream, &mut history, late, &mut || None);
                }
            }
        }
    }
    assert!(
        asked <= 12,
        "46 keys over about 11 screens asked {asked} times: a request per key, not per screen"
    );
}

/// Scrolls from the first row with `step` while the walk is still laying
/// out, answering every request, until the viewport stops moving. The view
/// never goes back towards the top (m1-the-graph/32: once the walk finished,
/// every answer snapped the wheel's viewport back to the selection on row 0),
/// and it ends on the oldest commit.
fn scroll_from_launch(step: impl Fn(&mut History) -> Option<RowRequest>) {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let whole = layout(&every_commit(&handle, Shape::Linear), Seeds::default());
    let walk = Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("walk starts");
    let mut stream = unfolded(WalkSource::new(&handle, walk), Seeds::default());

    let mut history = History::started();
    let mut request = history.show(HEIGHT);
    let mut steps = 0;
    loop {
        if let Some(asked) = request.take() {
            round_trip(&mut stream, &mut history, asked, &mut || None);
        }
        let before = history.top;
        request = step(&mut history);
        if let Some(asked) = request.take() {
            round_trip(&mut stream, &mut history, asked, &mut || None);
        }
        assert!(
            history.top >= before,
            "step {steps}: the viewport went back from row {before} to {}",
            history.top
        );
        if history.top == before && request.is_none() && history.phase == Phase::Complete {
            break;
        }
        steps += 1;
        assert!(steps < 2 * whole.len(), "scrolling reaches the end");
    }
    assert_eq!(history.rows, whole.len());
    let last = history.visible(HEIGHT).last().expect("rows on screen");
    assert_eq!(
        last,
        whole.last().expect("history"),
        "ends on the oldest commit"
    );
}

#[test]
fn the_wheel_from_launch_stops_at_the_oldest_commit_and_never_wraps() {
    scroll_from_launch(|history| history.scroll(3, HEIGHT));
}

#[test]
fn page_down_from_launch_stops_at_the_oldest_commit_and_never_wraps() {
    scroll_from_launch(|history| history.move_cursor(Motion::PageDown, HEIGHT));
}

/// `G` on a cold walk, then `Esc`: the walk stops within a chunk of the
/// replacement arriving, the viewport never moves, and nothing the jump sent
/// — its progress or a late answer — lands.
#[test]
fn a_withdrawn_jump_stops_walking_and_lands_nothing() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let walk = Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("walk starts");
    let mut stream = unfolded(WalkSource::new(&handle, walk), Seeds::default());

    let mut history = History::started();
    let first = history
        .show(HEIGHT)
        .expect("a fresh walk asks for its first screen");
    round_trip(&mut stream, &mut history, first, &mut || None);
    let (top, cursor, window) = (history.top, history.cursor, history.window.clone());
    let laid_before = stream.rows_emitted();

    let jump = history
        .move_cursor(Motion::Bottom, HEIGHT)
        .expect("a cold `G` is a request");
    assert_eq!(jump.wanted, Wanted::Last(2 * HEIGHT));
    let replacement = history
        .withdraw_jump(HEIGHT)
        .expect("`Esc` replaces the jump");
    assert_eq!(history.jump, None);

    let mut arrives = Some(replacement.clone());
    let reports = round_trip(&mut stream, &mut history, jump.clone(), &mut || {
        arrives.take()
    });

    assert_eq!(
        stream.rows_emitted(),
        laid_before + SMALL.chunk_rows,
        "stopped walking one chunk after the replacement arrived"
    );
    assert!(!stream.is_done());
    assert!(
        reports.iter().any(
            |update| matches!(update, HistoryUpdate::Window(w) if w.request == replacement.id)
        ),
        "the replacement is answered"
    );
    assert_eq!(
        (history.top, history.cursor),
        (top, cursor),
        "the viewport stays put"
    );
    assert_eq!(
        history.rows,
        stream.rows_emitted(),
        "the replacement's answer says how far the walk got"
    );
    assert_eq!(
        history.window.as_ref().map(|w| w.request),
        Some(replacement.id)
    );
    assert_eq!(
        history.window.as_ref().map(|w| &w.rows),
        window.as_ref().map(|w| &w.rows),
        "the same rows, answered afresh"
    );

    assert_late_jump_answers_are_dropped(&mut history, jump.id);
}

/// A late answer to request `jump`, and late progress, as if the jump had
/// finished before hearing it was withdrawn: neither moves anything.
fn assert_late_jump_answers_are_dropped(history: &mut History, jump: u64) {
    let before = history.clone();
    let late = Window {
        request: jump,
        first: 190,
        rows: Vec::new().into(),
        laid_out: 200,
        done: true,
        promoted: false,
        fold: [].into(),
        anchor_behind: None,
    };
    history.apply(HistoryUpdate::Window(late), HEIGHT);
    history.apply(
        HistoryUpdate::Progress {
            request: jump,
            laid_out: 200,
        },
        HEIGHT,
    );
    assert_eq!(*history, before, "nothing from the withdrawn jump lands");
}

/// Once `stop` has said stop, nothing more is reported.
#[test]
fn nothing_is_reported_once_stop_is_asked() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let walk = Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("walk starts");
    let mut stream = unfolded(WalkSource::new(&handle, walk), Seeds::default());

    let stopped = AtomicBool::new(false);
    let mut reports = 0;
    let jump = RowRequest {
        id: 1,
        wanted: Wanted::Last(HEIGHT),
        view: View::default(),
        promote: None,
    };
    let ended = answer(
        &mut stream,
        jump,
        &mut || None,
        &|| stopped.load(Ordering::SeqCst),
        &mut |_| {
            reports += 1;
            stopped.store(true, Ordering::SeqCst);
            Delivery::Received
        },
    )
    .expect("lays out");

    assert_eq!(ended, Some(Ended::Cancelled));
    assert_eq!(reports, 1);
}

/// A walk whose request sender is gone ends: nobody can ask it for rows again,
/// so it gives its worker back.
#[test]
fn a_walk_nobody_can_ask_ends() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let mut opened = 0;
    let open = |_: Option<&Arc<[CommitHandle]>>| {
        opened += 1;
        Ok(WalkSource::new(
            &handle,
            Walk::all_refs(&handle, Order::Topological, table(&handle))?,
        ))
    };
    let mut views = Views::new(Seeds::default(), SMALL, Box::new(open));
    let (asks, requests) = mpsc::channel::<RowRequest>();
    drop(asks);

    let ended = serve(&mut views, &requests, &never, &mut |_| Delivery::Received);

    assert_eq!(ended.expect("nothing to lay out"), Ended::Cancelled);
    drop(views);
    assert_eq!(opened, 0, "nothing was asked, so nothing was walked");
}

/// Every row `view` shows of `shape`, laid out through `views`.
fn rows_in<S: CommitSource>(views: &mut Views<'_, S>, view: &View, shape: Shape) -> Vec<Row> {
    let layout = views
        .layout(view)
        .unwrap_or_else(|why| panic!("{shape}: opening the view: {why}"));
    let mut rows = Vec::new();
    let everything = RowRequest {
        id: 1,
        wanted: Wanted::Rows(0..usize::MAX),
        view: view.clone(),
        promote: None,
    };
    let ended = answer(layout, everything, &mut || None, &never, &mut |update| {
        if let HistoryUpdate::Window(window) = update {
            rows.extend(window.rows.iter().cloned());
        }
        Delivery::Received
    })
    .unwrap_or_else(|why| panic!("{shape}: laying out: {why}"));
    assert_eq!(ended, None, "{shape}");
    rows
}

/// The app's views of the merge shapes: folding by default hides the merged
/// branches and the full view brings back the very rows there were; first-parent view is one
/// straight lane and is laid out on its own, leaving the full view's rows as
/// they were. Two sources are opened, one per `Follow`, never more.
#[test]
fn views_fold_unfold_and_switch_to_first_parent_over_one_walk() {
    for shape in [Shape::FoxtrotMerge, Shape::CrissCrossMerge] {
        let dir = graft_fixtures::borrow(shape);
        let handle = RepoHandle::open(&dir).expect("opens");
        let refs = handle.references().expect("refs read");
        let branches = Branches::resolve(&handle, None).expect("branches resolve");
        let mut opened = 0;
        let open = |_: Option<&Arc<[CommitHandle]>>| {
            opened += 1;
            Ok(WalkSource::new(
                &handle,
                Walk::all_refs(&handle, Order::Topological, table(&handle))?,
            ))
        };
        let mut views = Views::new(seeds_for(&refs, &branches), SMALL, Box::new(open));

        let full = View::default();
        let all = rows_in(&mut views, &full, shape);
        let folded_view = View::folded();
        let folded = rows_in(&mut views, &folded_view, shape);
        assert!(
            folded.len() < all.len(),
            "{shape}: folding hides the branch"
        );
        assert_eq!(rows_in(&mut views, &full, shape), all, "{shape}: unfolded");

        let first_parent = View {
            follow: Follow::FirstParent,
            ..View::default()
        };
        let straight = rows_in(&mut views, &first_parent, shape);
        assert!(!straight.is_empty(), "{shape}");
        assert!(
            straight.iter().all(|row| row.lane == 0),
            "{shape}: first-parent view is one lane"
        );
        let rebuilt_before = views.layout(&full).expect("open").work().chunks_rebuilt;
        assert_eq!(
            rows_in(&mut views, &full, shape),
            all,
            "{shape}: back again"
        );
        assert_eq!(
            views.layout(&full).expect("open").work().chunks_rebuilt,
            rebuilt_before,
            "{shape}: switching views leaves the full view's cache alone"
        );
        drop(views);
        assert_eq!(opened, 2, "{shape}: one source per Follow");
    }
}

/// A newer request in another view stops the answer in hand, sends nothing
/// for it, and `serve` answers the newer one in its own view.
#[test]
fn a_request_in_another_view_interrupts_and_is_answered_next() {
    let shape = Shape::FoxtrotMerge;
    let dir = graft_fixtures::borrow(shape);
    let handle = RepoHandle::open(&dir).expect("opens");
    let refs = handle.references().expect("refs read");
    let branches = Branches::resolve(&handle, None).expect("branches resolve");
    let open = |_: Option<&Arc<[CommitHandle]>>| {
        Ok(WalkSource::new(
            &handle,
            Walk::all_refs(&handle, Order::Topological, table(&handle))?,
        ))
    };
    let mut views = Views::new(seeds_for(&refs, &branches), SMALL, Box::new(open));
    let first_parent = View {
        follow: Follow::FirstParent,
        ..View::default()
    };
    let straight = rows_in(&mut views, &first_parent, shape);

    let jump = RowRequest {
        id: 1,
        wanted: Wanted::Last(HEIGHT),
        view: View::default(),
        promote: None,
    };
    let switch = RowRequest {
        id: 2,
        wanted: Wanted::Rows(0..usize::MAX),
        view: first_parent,
        promote: None,
    };
    let mut newer = Some(switch.clone());
    let mut windows = 0;
    let layout = views.layout(&jump.view).expect("opens");
    let ended = answer(layout, jump, &mut || newer.take(), &never, &mut |update| {
        if matches!(update, HistoryUpdate::Window(_)) {
            windows += 1;
        }
        Delivery::Received
    })
    .expect("lays out");
    assert_eq!(ended, None);
    assert_eq!(windows, 0, "nothing is sent for the interrupted jump");

    let (asks, requests) = mpsc::channel();
    asks.send(switch).expect("the receiver is right here");
    drop(asks);
    let mut answered = Vec::new();
    let ended = serve(&mut views, &requests, &never, &mut |update| {
        if let HistoryUpdate::Window(window) = update {
            answered.extend(window.rows.iter().cloned());
        }
        Delivery::Received
    })
    .expect("lays out");
    assert_eq!(ended, Ended::Cancelled);
    assert_eq!(answered, straight, "answered in the view it named");
}

/// The selection is a row number in its view, so it names the same commit
/// after its chunk is evicted and laid out again: page away until the
/// selected row's chunk is gone, page back, and the same commit is selected.
#[test]
fn selection_stays_on_its_commit_after_its_chunk_is_evicted_and_rebuilt() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let walk = Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("walk starts");
    let mut stream = unfolded(WalkSource::new(&handle, walk), Seeds::default());

    let mut history = History::started();
    let mut request = history.show(HEIGHT);
    let settle = |stream: &mut StreamingLayout<_>, history: &mut History, request| {
        if let Some(asked) = request {
            round_trip(stream, history, asked, &mut || None);
        }
    };
    settle(&mut stream, &mut history, request.take());
    request = history.move_cursor(Motion::Down(5), HEIGHT);
    settle(&mut stream, &mut history, request.take());
    let chosen = history.selected().expect("row 5 arrived").commit.id;
    let row = history.cursor;
    assert!(stream.is_resident(row));

    let mut pages = 0;
    while stream.is_resident(row) {
        request = history.move_cursor(Motion::PageDown, HEIGHT);
        settle(&mut stream, &mut history, request.take());
        pages += 1;
        assert!(pages < 50, "paging away evicts the selected row's chunk");
    }
    assert_ne!(history.selected().map(|r| r.commit.id), Some(chosen));
    for _ in 0..pages {
        request = history.move_cursor(Motion::PageUp, HEIGHT);
        settle(&mut stream, &mut history, request.take());
    }
    assert_eq!(history.cursor, row);
    assert!(
        stream.work().chunks_rebuilt > 0,
        "the chunk was laid out again"
    );
    assert_eq!(history.selected().map(|r| r.commit.id), Some(chosen));
}

/// A view change can move every row number; the cursor finds its commit
/// among the rows that come back.
#[test]
fn a_view_change_puts_the_cursor_back_on_the_selected_commit() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let whole = layout(&every_commit(&handle, Shape::Linear), Seeds::default());
    let mut history = History::started();
    let window = |request, first: usize, rows: &[Row]| {
        HistoryUpdate::Window(Window {
            request,
            first,
            rows: rows.to_vec().into(),
            laid_out: whole.len(),
            done: false,
            promoted: false,
            fold: [].into(),
            anchor_behind: None,
        })
    };
    history.asked = 1;
    history.apply(window(1, 0, &whole[..20]), HEIGHT);
    history.cursor = 5;
    let chosen = whole[5].commit.id;

    let mut view = View::default();
    view.inclusion.hidden_stashes.insert(whole[0].commit.id);
    let asked = history.change_view(view, HEIGHT).expect("asks for rows");
    // Two rows above the selection went away in the new view.
    history.apply(window(asked.id, 0, &whole[2..22]), HEIGHT);
    assert_eq!(history.cursor, 3);
    assert_eq!(history.selected().map(|r| r.commit.id), Some(chosen));
    assert_eq!(history.reanchor, None);
}

/// A seek still out when the view changes is asked again in the new view,
/// and lands when that answer comes (m8-polish-and-ship/45 A: the launch's
/// seek to `HEAD` was dropped by the stash list's view change, and the
/// cursor stayed on the stash row).
#[test]
fn a_seek_still_out_survives_a_view_change() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let whole = layout(&every_commit(&handle, Shape::Linear), Seeds::default());
    let mut history = History::started();
    let window = |request, first: usize, rows: &[Row]| {
        HistoryUpdate::Window(Window {
            request,
            first,
            rows: rows.to_vec().into(),
            laid_out: whole.len(),
            done: false,
            promoted: false,
            fold: [].into(),
            anchor_behind: None,
        })
    };
    history.asked = 1;
    history.apply(window(1, 0, &whole[..20]), HEIGHT);
    let sought = whole[30].commit.id;
    history
        .seek(sought, HEIGHT)
        .expect("asks the walk to find it");

    let mut view = View::default();
    view.inclusion.hidden_stashes.insert(whole[0].commit.id);
    let asked = history.change_view(view, HEIGHT).expect("asks for rows");
    assert!(
        matches!(asked.wanted, Wanted::Find { commit, .. } if commit == sought),
        "the view change asks for the sought commit again: {:?}",
        asked.wanted
    );
    history.apply(window(asked.id, 20, &whole[20..40]), HEIGHT);
    assert_eq!(history.selected().map(|r| r.commit.id), Some(sought));
    assert_eq!(history.jump, None);
    assert_eq!(history.seeking, None);
}

/// Sends `request` through the worker's loop — and the idle steps of any
/// promotion it began — and applies what comes back, as the job and `update`
/// would between them.
fn served<S: CommitSource>(views: &mut Views<'_, S>, history: &mut History, request: RowRequest)
where
    S::Error: std::fmt::Debug,
{
    answered(views, history, request);
    swapped_in(views, history);
}

/// `request`'s own answer through the worker's loop, applied — in the order
/// on screen, if it began a promotion.
fn answered<S: CommitSource>(views: &mut Views<'_, S>, history: &mut History, request: RowRequest)
where
    S::Error: std::fmt::Debug,
{
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
    for update in reports {
        history.apply(update, HEIGHT);
    }
}

/// A promotion begun reaches its anchor between requests, and is swapped in
/// there (m7-power-tools/12 Part C): its answer, applied.
fn swapped_in<S: CommitSource>(views: &mut Views<'_, S>, history: &mut History)
where
    S::Error: std::fmt::Debug,
{
    let mut reports = Vec::new();
    while views.is_promoting() {
        views
            .idle_step(&never, &mut |update| {
                reports.push(update);
                Delivery::Received
            })
            .expect("lays out");
    }
    for update in reports {
        history.apply(update, HEIGHT);
    }
}

/// ADR-0008 end to end on the one shape where commit-date order is not
/// topological: rows first in date order, then promoted in one step, with the
/// selected commit still selected at the same place on screen — though its
/// row number has moved.
#[test]
fn promotion_keeps_the_selected_commit_where_it_is_on_screen() {
    let shape = Shape::SkewedDates;
    let dir = graft_fixtures::borrow(shape);
    let store = RepoStore::new(dir.clone(), None);
    let order = topological_order(&store, &never)
        .expect("sorts")
        .expect("never stopped");
    let handle = RepoHandle::open(&dir).expect("opens");
    let refs = handle.references().expect("refs read");
    let branches = Branches::resolve(&handle, None).expect("branches resolve");
    // The table `order`'s handles were interned against: shared, not a fresh
    // one, so `in_order` can resolve them (they were pre-51b's bug).
    let table = store.table().expect("table");
    let open = |promoted: Option<&Arc<[CommitHandle]>>| {
        Ok(match promoted {
            Some(order) => WalkSource::in_order(&handle, Arc::clone(&table), Arc::clone(order)),
            None => WalkSource::new(
                &handle,
                Walk::all_refs(&handle, Order::CommitDate, Arc::clone(&table))?,
            ),
        })
    };
    let mut views = Views::new(seeds_for(&refs, &branches), SMALL, Box::new(open));

    let mut history = History::started();
    history.walk = Some(WalkFacts {
        requested: Order::Topological,
        order: Order::CommitDate,
        default_branch: None,
        integration: Vec::new().into(),
    });
    let first = history.show(HEIGHT).expect("asks for the first screen");
    served(&mut views, &mut history, first);
    if let Some(more) = history.move_cursor(Motion::Down(12), HEIGHT) {
        served(&mut views, &mut history, more);
    }
    let subject = |history: &History| {
        let row = history.selected().expect("a row is selected");
        String::from_utf8_lossy(row.commit.subject()).into_owned()
    };
    // Date order: feature, the fast commit, Main 12 .. Main 01, base.
    assert_eq!((history.cursor, history.top), (12, 3));
    assert_eq!(subject(&history), "Main 02");
    let chosen = history.selected().map(|row| row.commit.id);

    let promote = history
        .promote(order, HEIGHT)
        .expect("a walk still going takes the order");
    assert!(promote.promote.is_some());
    served(&mut views, &mut history, promote);

    assert_eq!(history.promotion, None, "the promotion has landed");
    let walk = history.walk.as_ref().expect("facts");
    assert_eq!(walk.order, Order::Topological);
    assert!(!walk.is_provisional());
    assert_eq!(history.selected().map(|row| row.commit.id), chosen);
    // Topological order puts the fast commit below Main 01: Main 02 moves
    // up a row, and the viewport with it.
    assert_eq!((history.cursor, history.top), (11, 2));
    let on_screen: Vec<_> = history
        .visible(HEIGHT)
        .iter()
        .map(|row| String::from_utf8_lossy(row.commit.subject()).into_owned())
        .collect();
    assert_eq!(on_screen[0], "Main 11", "{on_screen:?}");
}

/// The race of m1-the-graph/27c: a promotion is handed to the walk but has not
/// landed when a key asks for rows. The worker skips to the newest request,
/// so only the key's request is served, and it carries the promotion. Returns
/// the history after that answer, and the promoted order.
fn key_while_promotion_swaps(
    press: impl FnOnce(&mut History, &[Oid]) -> Option<RowRequest>,
) -> (History, Arc<[Oid]>) {
    let shape = Shape::Linear;
    let dir = graft_fixtures::borrow(shape);
    let store = RepoStore::new(dir.clone(), None);
    let order = topological_order(&store, &never)
        .expect("sorts")
        .expect("never stopped");
    let handle = RepoHandle::open(&dir).expect("opens");
    let refs = handle.references().expect("refs read");
    let branches = Branches::resolve(&handle, None).expect("branches resolve");
    // The table `order`'s handles were interned against: shared, not a fresh
    // one, so `in_order` can resolve them (they were pre-51b's bug).
    let table = store.table().expect("table");
    let ids: Arc<[Oid]> = order.iter().map(|&commit| table.id_of(commit)).collect();
    let open = |promoted: Option<&Arc<[CommitHandle]>>| {
        Ok(match promoted {
            Some(order) => WalkSource::in_order(&handle, Arc::clone(&table), Arc::clone(order)),
            None => WalkSource::new(
                &handle,
                Walk::all_refs(&handle, Order::CommitDate, Arc::clone(&table))?,
            ),
        })
    };
    let mut views = Views::new(seeds_for(&refs, &branches), SMALL, Box::new(open));
    let mut history = History::started();
    history.walk = Some(WalkFacts {
        requested: Order::Topological,
        order: Order::CommitDate,
        default_branch: None,
        integration: Vec::new().into(),
    });
    let first = history.show(HEIGHT).expect("asks for the first screen");
    served(&mut views, &mut history, first);
    assert_eq!(history.phase, Phase::Streaming, "the walk is not done yet");

    let unsent = history.promote(Arc::clone(&order), HEIGHT);
    assert!(unsent.is_some(), "a walk still going takes the order");
    let request = press(&mut history, &ids).expect("the key asks for rows");
    assert!(
        request.promote.is_some(),
        "the key's request carries the order"
    );
    served(&mut views, &mut history, request);
    assert_eq!(history.promotion, None, "the promotion has landed");
    (history, ids)
}

#[test]
fn g_pressed_while_promotion_swaps_reaches_the_bottom() {
    let (history, order) =
        key_while_promotion_swaps(|history, _| history.move_cursor(Motion::Bottom, HEIGHT));

    assert_eq!(history.jump, None);
    assert_eq!(history.phase, Phase::Complete);
    assert_eq!(history.rows, order.len());
    assert_eq!(history.cursor, order.len() - 1);
    assert_eq!(history.top, order.len() - HEIGHT);
    assert_eq!(
        history.selected().map(|row| row.commit.id),
        order.last().copied()
    );
}

#[test]
fn a_search_jump_in_flight_when_promotion_lands_reaches_its_match() {
    let (history, order) = key_while_promotion_swaps(|history, order| {
        let far = *order.last().expect("commits");
        history.seek(far, HEIGHT)
    });

    assert_eq!((history.jump, history.seeking), (None, None));
    assert_eq!(
        history.selected().map(|row| row.commit.id),
        order.last().copied()
    );
    assert!(
        (history.top..history.top + HEIGHT).contains(&history.cursor),
        "the hit is on screen: top {}, cursor {}",
        history.top,
        history.cursor
    );
}

/// The other side of the race: the jump is asked first, and the promotion's
/// request supersedes it before any answer. Only the promotion's request is
/// served, so it must carry the jump.
fn promotion_lands_on_a_jump_in_flight(
    press: impl FnOnce(&mut History, &[Oid]) -> Option<RowRequest>,
) -> (History, Arc<[Oid]>) {
    let shape = Shape::Linear;
    let dir = graft_fixtures::borrow(shape);
    let store = RepoStore::new(dir.clone(), None);
    let order = topological_order(&store, &never)
        .expect("sorts")
        .expect("never stopped");
    let handle = RepoHandle::open(&dir).expect("opens");
    let refs = handle.references().expect("refs read");
    let branches = Branches::resolve(&handle, None).expect("branches resolve");
    // The table `order`'s handles were interned against: shared, not a fresh
    // one, so `in_order` can resolve them (they were pre-51b's bug).
    let table = store.table().expect("table");
    let ids: Arc<[Oid]> = order.iter().map(|&commit| table.id_of(commit)).collect();
    let open = |promoted: Option<&Arc<[CommitHandle]>>| {
        Ok(match promoted {
            Some(order) => WalkSource::in_order(&handle, Arc::clone(&table), Arc::clone(order)),
            None => WalkSource::new(
                &handle,
                Walk::all_refs(&handle, Order::CommitDate, Arc::clone(&table))?,
            ),
        })
    };
    let mut views = Views::new(seeds_for(&refs, &branches), SMALL, Box::new(open));
    let mut history = History::started();
    history.walk = Some(WalkFacts {
        requested: Order::Topological,
        order: Order::CommitDate,
        default_branch: None,
        integration: Vec::new().into(),
    });
    let first = history.show(HEIGHT).expect("asks for the first screen");
    served(&mut views, &mut history, first);
    assert_eq!(history.phase, Phase::Streaming, "the walk is not done yet");

    let unsent = press(&mut history, &ids);
    assert!(unsent.is_some(), "the key asks for rows");
    let request = history
        .promote(Arc::clone(&order), HEIGHT)
        .expect("a walk still going takes the order");
    served(&mut views, &mut history, request);
    assert_eq!(history.promotion, None, "the promotion has landed");
    (history, ids)
}

#[test]
fn promotion_asked_after_g_still_reaches_the_bottom() {
    let (history, order) = promotion_lands_on_a_jump_in_flight(|history, _| {
        history.move_cursor(Motion::Bottom, HEIGHT)
    });

    assert_eq!(history.jump, None);
    assert_eq!(history.cursor, order.len() - 1);
    assert_eq!(
        history.selected().map(|row| row.commit.id),
        order.last().copied()
    );
}

#[test]
fn promotion_asked_after_a_search_jump_still_reaches_its_match() {
    let (history, order) = promotion_lands_on_a_jump_in_flight(|history, order| {
        history.seek(*order.last().expect("commits"), HEIGHT)
    });

    assert_eq!((history.jump, history.seeking), (None, None));
    assert_eq!(
        history.selected().map(|row| row.commit.id),
        order.last().copied()
    );
}

/// A move onto rows not fetched yet has no commit to anchor the swap to: it
/// keeps its row numbers, and does not snap back to the last commit selected.
#[test]
fn page_down_past_the_rows_on_hand_while_promotion_swaps_keeps_its_place() {
    let (history, order) = key_while_promotion_swaps(|history, _| {
        // A walk still laying out reports rows past the window it sent, so
        // the second key lands on a row laid out but not fetched (a request
        // lays out only the rows it names, 30c).
        let progress = HistoryUpdate::Progress {
            request: history.asked,
            laid_out: 3 * HEIGHT,
        };
        history.apply(progress, HEIGHT);
        assert_eq!(history.move_cursor(Motion::PageDown, HEIGHT), None);
        history.move_cursor(Motion::PageDown, HEIGHT)
    });
    // Linear history: the promoted order numbers every row as before.
    assert_eq!((history.cursor, history.top), (2 * HEIGHT, 2 * HEIGHT));
    assert_eq!(
        history.selected().map(|row| row.commit.id),
        Some(order[2 * HEIGHT])
    );
}

/// m8/13m: a promotion's request is answered twice — in the order on screen
/// first, then, once the promoted layout reaches the anchor, in the new order
/// (Part C). A `j` between the two moves the cursor over rows already on hand
/// and asks for nothing. The swap must land on the commit the cursor is on
/// when it lands, not the one it was on when the first answer came: that put
/// `j j` back on the top row whenever the keys fell in the gap, which is what
/// made `quit_waits::signal` fail under load.
#[test]
fn a_move_between_the_two_answers_of_a_promotion_is_kept() {
    let shape = Shape::Linear;
    let dir = graft_fixtures::borrow(shape);
    let store = RepoStore::new(dir.clone(), None);
    let order = topological_order(&store, &never)
        .expect("sorts")
        .expect("never stopped");
    let handle = RepoHandle::open(&dir).expect("opens");
    let refs = handle.references().expect("refs read");
    let branches = Branches::resolve(&handle, None).expect("branches resolve");
    let table = store.table().expect("table");
    let ids: Vec<Oid> = order.iter().map(|&commit| table.id_of(commit)).collect();
    let open = |promoted: Option<&Arc<[CommitHandle]>>| {
        Ok(match promoted {
            Some(order) => WalkSource::in_order(&handle, Arc::clone(&table), Arc::clone(order)),
            None => WalkSource::new(
                &handle,
                Walk::all_refs(&handle, Order::CommitDate, Arc::clone(&table))?,
            ),
        })
    };
    let mut views = Views::new(seeds_for(&refs, &branches), SMALL, Box::new(open));
    let mut history = History::started();
    history.walk = Some(WalkFacts {
        requested: Order::Topological,
        order: Order::CommitDate,
        default_branch: None,
        integration: Vec::new().into(),
    });
    let first = history.show(HEIGHT).expect("asks for the first screen");
    served(&mut views, &mut history, first);
    assert_eq!(history.cursor, 0);

    let request = history
        .promote(Arc::clone(&order), HEIGHT)
        .expect("a walk still going takes the order");
    // The request's own answer, in the order on screen.
    answered(&mut views, &mut history, request);
    assert!(
        matches!(history.promotion, Some(Promotion::Swapping { .. })),
        "the swap has not landed yet"
    );

    // `j j`, over rows on hand: nothing is asked.
    assert_eq!(history.move_cursor(Motion::Down(1), HEIGHT), None);
    assert_eq!(history.move_cursor(Motion::Down(1), HEIGHT), None);
    assert_eq!(history.selected().map(|row| row.commit.id), Some(ids[2]));

    // The promoted layout reaches the anchor and swaps in.
    swapped_in(&mut views, &mut history);
    assert_eq!(history.promotion, None, "the promotion has landed");
    // Linear history: the promoted order numbers every row as before.
    assert_eq!(history.cursor, 2, "the moves are kept");
    assert_eq!(history.selected().map(|row| row.commit.id), Some(ids[2]));
}

/// `count` rows of a straight line, as a window starting at `first`.
fn window_of(first: usize, count: usize) -> Window {
    use graft_core::commit::{Identity, Timestamp};
    let oid = |index: usize| {
        let mut bytes = [0u8; Oid::SHA1_BYTES];
        bytes[..8].copy_from_slice(&(index as u64 + 1).to_be_bytes());
        Oid::from_bytes(&bytes).expect("20 bytes")
    };
    let who = Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    };
    let commits: Vec<Commit> = (first..first + count)
        .map(|index| Commit {
            id: oid(index),
            parents: vec![oid(index + 1)],
            author: who.clone(),
            committer: who.clone(),
            message: b"row".to_vec(),
        })
        .collect();
    let rows = layout(&commits, Seeds::default());
    Window {
        request: 1,
        first,
        laid_out: first + count,
        done: true,
        rows: rows.into(),
        promoted: false,
        fold: [].into(),
        anchor_behind: None,
    }
}

/// A complete history of `rows` rows, scrolled to `top` with the cursor there.
fn scrolled(rows: usize, top: usize, strip_rows: Option<usize>) -> History {
    History {
        rows,
        phase: Phase::Complete,
        top,
        cursor: top,
        strip_rows,
        ..History::started()
    }
}

fn asked_rows(request: Option<RowRequest>) -> Option<std::ops::Range<usize>> {
    request.map(|request| match request.wanted {
        Wanted::Rows(rows) => rows,
        Wanted::Last(count) => panic!("asked for the last {count} rows"),
        Wanted::Find { commit, .. } => panic!("asked for the rows around {commit}"),
    })
}

/// The text tiers keep the viewport and two screens either side on hand, so
/// the next keys are drawn from rows already there, and ask for three
/// screens either side once they don't (m1-the-graph/30c).
#[test]
fn text_tiers_keep_two_screens_either_side_on_hand_and_ask_for_three() {
    let mut history = scrolled(1000, 60, None);
    assert_eq!(asked_rows(history.show(20)), Some(0..140));

    // Each check below starts with that request answered.
    // The viewport and a screen either side is not enough: a second page
    // before the answer lands would wait.
    history.outstanding = None;
    history.window = Some(window_of(40, 60));
    assert_eq!(asked_rows(history.show(20)), Some(0..140));
    // Two screens either side is.
    history.outstanding = None;
    history.window = Some(window_of(20, 100));
    assert_eq!(asked_rows(history.show(20)), None);
}

/// A request already out for the rows a key needs is not sent again: the
/// newest answer is on its way.
#[test]
fn rows_already_asked_for_are_not_asked_for_again() {
    let mut history = scrolled(1000, 60, None);
    history.phase = Phase::Streaming;
    assert_eq!(asked_rows(history.show(20)), Some(0..140));
    assert_eq!(asked_rows(history.move_cursor(Motion::Down(1), 20)), None);
    // The first page is still covered by it; the second is past it and asks.
    assert_eq!(asked_rows(history.move_cursor(Motion::PageDown, 20)), None);
    assert!(history.move_cursor(Motion::PageDown, 20).is_some());
}

/// A viewport starting mid-strip asks for whole strips and the row above
/// the first, and is not covered by a window that holds only its strips.
#[test]
fn a_viewport_starting_mid_strip_asks_for_whole_aligned_strips() {
    let mut history = scrolled(1000, 40, Some(32));
    // Three screens either side, 0..120, widen to strips 0..128; strip 0 has
    // no row above it.
    assert_eq!(asked_rows(history.show(20)), Some(0..128));

    let mut deeper = scrolled(1000, 100, Some(32));
    // 40..180 widens to strips 32..192, plus row 31 above.
    assert_eq!(asked_rows(deeper.show(20)), Some(31..192));

    // What must be on hand is two screens either side, 60..160: strips
    // 32..160 and row 31 above. The viewport's strip alone, or those strips
    // without row 31, are not enough. Each check starts answered.
    for (first, count, asks) in [(95, 33, true), (32, 128, true), (31, 129, false)] {
        deeper.outstanding = None;
        deeper.window = Some(window_of(first, count));
        assert_eq!(deeper.show(20).is_some(), asks, "window {first}+{count}");
    }
}

/// The last strip of a finished history is short; the rows that exist are
/// enough.
#[test]
fn the_last_partial_strip_at_the_end_of_history_is_covered_by_the_rows_there() {
    let mut history = scrolled(50, 40, Some(32));
    // Two screens either side, 20..70, widen to strips 0..96: rows 0..50
    // exist.
    history.window = Some(window_of(0, 50));
    assert_eq!(asked_rows(history.show(10)), None);

    // One row short of the end is not.
    history.window = Some(window_of(0, 49));
    assert!(history.show(10).is_some());
}

/// A search hit past everything the first screen laid out is reached by
/// laying the walk out to it (`Wanted::Find`), and the cursor lands on it,
/// on screen.
#[test]
fn a_search_hit_past_the_first_chunk_is_laid_out_to_and_selected() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let walk = Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("walk starts");
    let mut stream = unfolded(WalkSource::new(&handle, walk), Seeds::default());
    let mut history = History::started();
    let first = history
        .show(HEIGHT)
        .expect("a fresh walk asks for its first screen");
    round_trip(&mut stream, &mut history, first, &mut || None);
    let far = every_commit(&handle, Shape::Linear)
        .iter()
        .map(|commit| commit.id)
        .find(|id| stream.row_of(*id).is_none())
        .expect("the linear shape has commits past the first screen");

    let seek = history.seek(far, HEIGHT).expect("a far hit is a request");
    assert!(matches!(seek.wanted, Wanted::Find { commit, .. } if commit == far));
    assert_eq!(history.jump, Some(seek.id), "a pending find is a jump");
    round_trip(&mut stream, &mut history, seek, &mut || None);

    assert_eq!(history.selected().map(|row| row.commit.id), Some(far));
    assert_eq!((history.jump, history.seeking), (None, None));
    assert!(
        (history.top..history.top + HEIGHT).contains(&history.cursor),
        "the hit is on screen: top {}, cursor {}",
        history.top,
        history.cursor
    );
}

/// A wheel scroll that arrives while a far search jump is still walking must
/// not strand `seeking`: the answer that eventually lands is for the wheel's
/// own (newer) request, never for the jump's, so only withdrawing the jump
/// when the wheel asks — symmetric with `select_at` — clears it in time
/// (m1-the-graph/65, `KNOWN_GAPS.md:197`, review M3).
#[test]
fn a_wheel_scroll_during_a_far_seek_withdraws_the_seek() {
    let far = {
        let mut bytes = [0u8; Oid::SHA1_BYTES];
        bytes[0] = 0xAB;
        Oid::from_bytes(&bytes).expect("20 bytes")
    };
    // No window on hand: `seek` cannot find `far` among rows it already
    // has, so it sends a `Find` request and starts a jump, as it would for
    // a hit far down a real walk.
    let mut history = scrolled(1000, 0, None);
    let seek = history.seek(far, HEIGHT).expect("a far hit is a request");
    assert_eq!(history.jump, Some(seek.id));
    assert_eq!(history.seeking, Some(far));

    // The wheel arrives next, before the seek's `Find` answer lands.
    let scroll = history
        .scroll(3, HEIGHT)
        .expect("rows off the top of a cold walk need asking for");
    assert_ne!(
        scroll.id, seek.id,
        "the wheel's request supersedes the seek's"
    );
    assert_eq!(
        (history.jump, history.seeking),
        (None, None),
        "the wheel scroll must withdraw the stranded seek itself: no answer \
         will ever carry the seek's request id"
    );

    // Only the wheel's own request is ever answered — as in the real
    // failure, the seek's `Find` answer never arrives once superseded.
    history.apply(
        HistoryUpdate::Window(Window {
            request: scroll.id,
            ..window_of(0, 43)
        }),
        HEIGHT,
    );
    assert_eq!(
        (history.jump, history.seeking),
        (None, None),
        "seeking must not reappear once the wheel's own answer lands"
    );
}

/// A request just past the frontier lays out the rows it names, not the rest
/// of their chunk: a whole chunk read from the repository costs frames a page
/// key cannot spare (ticket 30c).
#[test]
fn a_request_lays_out_only_the_rows_it_names() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let open = |_: Option<&Arc<[CommitHandle]>>| {
        Ok(WalkSource::new(
            &handle,
            Walk::all_refs(&handle, Order::Topological, table(&handle))?,
        ))
    };
    let mut views = Views::new(Seeds::default(), SMALL, Box::new(open));
    let request = RowRequest {
        id: 1,
        wanted: Wanted::Rows(0..3),
        view: View::default(),
        promote: None,
    };
    let mut laid_out = None;
    let layout = views.layout(&request.view).expect("opens");
    let ended = answer(layout, request, &mut || None, &never, &mut |update| {
        if let HistoryUpdate::Window(window) = update {
            laid_out = Some(window.laid_out);
        }
        Delivery::Received
    })
    .expect("lays out");

    assert_eq!(ended, None, "answered");
    const {
        assert!(
            SMALL.chunk_rows > 3,
            "the request ends inside the first chunk"
        );
    };
    assert_eq!(laid_out, Some(3), "no row past the request was laid out");
}
