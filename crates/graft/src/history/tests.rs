//! The product source, seeds and streaming driver, on the fixture corpus.

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use graft_core::commit::Commit;
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_fixtures::Shape;
use graft_git::branches::Branches;
use graft_git::history::{CHUNK, Order, Walk};
use graft_git::repo::{RefTarget, RepoHandle};
use graft_graph::layout::{CommitSource, Pull, StreamConfig, StreamingLayout, layout};
use graft_graph::{BranchIdentity, Row, Seeds};

use super::*;
use crate::update::{AppEvent, AppState, Progress, Task, update};

/// Small enough that every corpus shape spans several chunks and overflows
/// the resident budget.
const SMALL: StreamConfig = StreamConfig {
    chunk_rows: 8,
    checkpoint_every: 16,
    max_checkpoints: 4,
    resident_chunks: 2,
    buffer_rows: 4,
};

/// Generous, because it is never reached when the code works.
const PATIENCE: Duration = Duration::from_secs(30);

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

/// Runs `layout` to the end, collecting every row it reports.
fn reported_rows<S: CommitSource>(layout: &mut StreamingLayout<S>, shape: Shape) -> Vec<Row> {
    let mut rows = Vec::new();
    let ended = lay_out(layout, &never, &mut |update| {
        if let HistoryUpdate::Rows(chunk) = update {
            rows.extend(chunk.iter().cloned());
        }
        Delivery::Received
    })
    .unwrap_or_else(|why| panic!("{shape}: laying out: {why}"));
    assert_eq!(ended, Ended::Complete, "{shape}");
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
        let ended = stream(&store, Order::Topological, &never, &mut |update| {
            match update {
                HistoryUpdate::Started(started) => facts = Some(*started),
                HistoryUpdate::Rows(chunk) => rows.extend(chunk.iter().cloned()),
            }
            Delivery::Received
        })
        .expect("the walk streams");
        assert_eq!(ended, Ended::Complete);
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

/// A source over commits already read, that hands out its first chunk and
/// then waits to be released before handing out any more.
struct GatedSource {
    commits: Vec<Commit>,
    served: usize,
    gate: mpsc::Receiver<()>,
}

impl CommitSource for GatedSource {
    type Error = Infallible;

    fn next(&mut self, limit: usize, _stop: &dyn Fn() -> bool) -> Result<Pull, Infallible> {
        if self.served > 0 {
            match self.gate.recv_timeout(PATIENCE) {
                // Released, or the test dropped its sender: either way, go on.
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => panic!("the gate was never released"),
            }
        }
        let end = (self.served + limit).min(self.commits.len());
        let commits = self.commits[self.served..end].to_vec();
        self.served = end;
        Ok(Pull {
            commits,
            done: end == self.commits.len(),
            stopped: false,
        })
    }

    fn read(&mut self, ids: &[Oid]) -> Result<Vec<Commit>, Infallible> {
        let by_id: HashMap<Oid, &Commit> = self.commits.iter().map(|c| (c.id, c)).collect();
        Ok(ids.iter().map(|id| by_id[id].clone()).collect())
    }
}

/// The first chunk reaches `AppState` while the walk is still blocked on its
/// second: the screen does not wait for the last commit.
#[test]
fn a_chunk_lands_in_app_state_before_the_walk_finishes() {
    let dir = graft_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let commits = every_commit(&handle, Shape::Linear);
    let total = commits.len();
    assert!(total > SMALL.chunk_rows, "more than one chunk");

    let (release, gate) = mpsc::channel();
    let (updates, received) = mpsc::channel();
    let finished = std::sync::Arc::new(AtomicBool::new(false));
    let walker = {
        let finished = std::sync::Arc::clone(&finished);
        thread::spawn(move || {
            let source = GatedSource {
                commits,
                served: 0,
                gate,
            };
            let mut stream = StreamingLayout::new(source, Seeds::default(), SMALL);
            let ended = lay_out(
                &mut stream,
                &never,
                &mut |update| match updates.send(update) {
                    Ok(()) => Delivery::Received,
                    Err(_) => Delivery::NobodyListening,
                },
            );
            finished.store(true, Ordering::SeqCst);
            ended
        })
    };

    let id = JobId::new(9);
    let (mut state, _) = update(
        &AppState::default(),
        AppEvent::Started(id, Task::ReadHistory),
    );
    let first = received.recv_timeout(PATIENCE).expect("the first chunk");
    (state, _) = update(&state, AppEvent::Progress(id, Progress::History(first)));

    let history = state.history.as_ref().expect("a walk is on screen");
    assert_eq!(history.rows, SMALL.chunk_rows);
    assert_eq!(history.phase, Phase::Streaming);
    assert!(
        !finished.load(Ordering::SeqCst),
        "the walk is still blocked on its second chunk"
    );

    release.send(()).expect("the walker is waiting");
    drop(release);
    let mut rows = SMALL.chunk_rows;
    for update in &received {
        if let HistoryUpdate::Rows(chunk) = update {
            rows += chunk.len();
        }
    }
    let ended = walker.join().expect("the walker does not panic");
    assert_eq!(ended.expect("lays out"), Ended::Complete);
    assert_eq!(rows, total);
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
    let ended = lay_out(&mut stream, &|| stopped.load(Ordering::SeqCst), &mut |_| {
        reports += 1;
        stopped.store(true, Ordering::SeqCst);
        Delivery::Received
    })
    .expect("lays out");

    assert_eq!(ended, Ended::Cancelled);
    assert_eq!(reports, 1);
}
