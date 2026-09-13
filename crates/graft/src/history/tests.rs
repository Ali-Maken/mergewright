//! The product source, seeds and streaming driver, on the fixture corpus.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use graft_core::commit::Commit;
use graft_core::ref_name::RefName;
use graft_fixtures::Shape;
use graft_git::branches::Branches;
use graft_git::history::{CHUNK, Order, Walk};
use graft_git::repo::{RefTarget, RepoHandle};
use graft_graph::layout::{CommitSource, StreamConfig, StreamingLayout, layout};
use graft_graph::{BranchIdentity, Row, Seeds};

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

fn never() -> bool {
    false
}

/// Every commit the refs reach, in the order the app asks for.
fn every_commit(handle: &RepoHandle, shape: Shape) -> Vec<Commit> {
    let mut walk = Walk::all_refs(handle, Order::Topological)
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

/// Asks `layout` for every row there is a chunk at a time, as a screen
/// paging down would, collecting the answers.
fn reported_rows<S: CommitSource>(layout: &mut StreamingLayout<S>, shape: Shape) -> Vec<Row> {
    let chunk = layout.config().chunk_rows;
    let mut rows: Vec<Row> = Vec::new();
    for id in 1.. {
        let request = RowRequest {
            id,
            wanted: Wanted::Rows(rows.len()..rows.len() + chunk),
            view: View::default(),
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

        let walk = Walk::all_refs(&handle, Order::Topological).expect("walk starts");
        let mut stream = StreamingLayout::new(WalkSource::new(&handle, walk), seeds, SMALL);
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
                    HistoryUpdate::Progress { .. } => {}
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
    layout: &mut StreamingLayout<S>,
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
    let walk = Walk::all_refs(&handle, Order::Topological).expect("walk starts");
    let mut stream = StreamingLayout::new(WalkSource::new(&handle, walk), Seeds::default(), SMALL);

    // What the layout may protect at once: the window the screen asks for
    // (three screens), back to a checkpoint, plus its buffers — in whole
    // chunks — and the two chunks its budget lets it keep besides.
    let protected =
        (3 * HEIGHT + SMALL.checkpoint_every + 2 * SMALL.buffer_rows).div_ceil(SMALL.chunk_rows);
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
            window.rows.len() <= 3 * HEIGHT,
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

/// `G` on a cold walk, then `Esc`: the walk stops within a chunk of the
/// replacement arriving, the viewport never moves, and nothing the jump sent
/// — its progress or a late answer — lands.
#[test]
fn a_withdrawn_jump_stops_walking_and_lands_nothing() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let walk = Walk::all_refs(&handle, Order::Topological).expect("walk starts");
    let mut stream = StreamingLayout::new(WalkSource::new(&handle, walk), Seeds::default(), SMALL);

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
    let walk = Walk::all_refs(&handle, Order::Topological).expect("walk starts");
    let mut stream = StreamingLayout::new(WalkSource::new(&handle, walk), Seeds::default(), SMALL);

    let stopped = AtomicBool::new(false);
    let mut reports = 0;
    let jump = RowRequest {
        id: 1,
        wanted: Wanted::Last(HEIGHT),
        view: View::default(),
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
    let open = || {
        opened += 1;
        Ok(WalkSource::new(
            &handle,
            Walk::all_refs(&handle, Order::Topological)?,
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

/// The app's views of the merge shapes: folding a merge hides its branch and
/// unfolding brings back the very rows there were; first-parent view is one
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
        let open = || {
            opened += 1;
            Ok(WalkSource::new(
                &handle,
                Walk::all_refs(&handle, Order::Topological)?,
            ))
        };
        let mut views = Views::new(seeds_for(&refs, &branches), SMALL, Box::new(open));

        let full = View::default();
        let all = rows_in(&mut views, &full, shape);
        let merge = all
            .iter()
            .find(|row| row.commit.parents.len() > 1)
            .unwrap_or_else(|| panic!("{shape} has a merge"))
            .commit
            .id;
        let folded_view = View {
            inclusion: Inclusion {
                folded: [merge].into(),
                ..Inclusion::default()
            },
            ..View::default()
        };
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
    let open = || {
        Ok(WalkSource::new(
            &handle,
            Walk::all_refs(&handle, Order::Topological)?,
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
    };
    let switch = RowRequest {
        id: 2,
        wanted: Wanted::Rows(0..usize::MAX),
        view: first_parent,
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
