//! The streaming layout over real repositories.
//!
//! The unit tests in `graft_graph::layout::stream` drive the cache with
//! generated histories, where the interesting cases — deep history, dates
//! out of topological order — are cheap to make. This is where the same
//! engine meets `graft-git`: a `Walk` behind the `CommitSource` seam, wired
//! the way the app will wire it (m1-10), on the corpus shapes.
//!
//! The corpus never has a commit-graph file, so every walk here takes
//! ADR-0008's provisional path: commit date first, the whole-range
//! topological walk behind it. Its timestamps strictly increase, so the two
//! orders agree on every shape; promotion here proves the plumbing — the
//! selected commit found again by id in the promoted order, the rows
//! rebuilt from re-reads and nothing walked — and the unit tests prove the
//! reordering on skewed dates.

use std::sync::Arc;

use graft_core::commit::Commit;
use graft_core::commit_handle::{CommitHandle, Link};
use graft_core::oid::Oid;
use graft_fixtures::Shape;
use graft_git::history::{CHUNK, CommitTable, Order, Walk, WalkError, read_commit};
use graft_git::repo::RepoHandle;
use graft_graph::layout::{Anchor, CommitSource, Pull, StreamConfig, StreamingLayout, layout};
use graft_graph::row::Row;

use crate::corpus::{for_each_shape, open_with_seeds, walk_everything};

/// A cache small enough that the corpus overflows it: the 200-commit linear
/// shape is 25 chunks against a budget of 2, with 12 checkpoints due and 4
/// kept.
const SMALL: StreamConfig = StreamConfig {
    chunk_rows: 8,
    checkpoint_every: 16,
    max_checkpoints: 4,
    resident_chunks: 2,
    buffer_rows: 4,
};

/// Rows on screen at once.
const HEIGHT: usize = 5;

/// Commits per `advance`.
const BUDGET: usize = 7;

/// A `Walk` behind the layout's seam, its re-reads going through
/// [`read_commit`]. Counts what the layout asked of it.
struct WalkSource<'a> {
    handle: &'a RepoHandle,
    walk: Walk<'a>,
    pulls: usize,
    reads: usize,
}

impl<'a> WalkSource<'a> {
    fn new(handle: &'a RepoHandle, walk: Walk<'a>) -> Self {
        Self {
            handle,
            walk,
            pulls: 0,
            reads: 0,
        }
    }
}

impl CommitSource for WalkSource<'_> {
    type Error = WalkError;

    fn pull(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, WalkError> {
        self.pulls += 1;
        let chunk = self.walk.next_links(limit, stop)?;
        Ok(Pull {
            links: chunk.links,
            done: chunk.done,
            stopped: chunk.stopped,
        })
    }

    fn read(&mut self, commits: &[CommitHandle]) -> Result<Vec<Commit>, WalkError> {
        self.reads += commits.len();
        let table = Arc::clone(self.walk.table());
        commits
            .iter()
            .map(|&handle| read_commit(self.handle, table.id_of(handle)))
            .collect()
    }

    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, WalkError> {
        let table = Arc::clone(self.walk.table());
        commits
            .iter()
            .map(|&handle| {
                let commit = read_commit(self.handle, table.id_of(handle))?;
                Ok(Link {
                    commit: handle,
                    parents: commit
                        .parents
                        .iter()
                        .map(|parent| table.intern(parent))
                        .collect(),
                })
            })
            .collect()
    }

    fn handle_of(&self, id: &Oid) -> Option<CommitHandle> {
        self.walk.table().handle_of(id)
    }

    fn id_of(&self, handle: CommitHandle) -> Oid {
        self.walk.table().id_of(handle)
    }
}

/// The repository's id table, as the app shares it.
fn table(handle: &RepoHandle) -> Arc<CommitTable> {
    Arc::new(CommitTable::open(handle).unwrap_or_else(|why| panic!("opening the id table: {why}")))
}

/// Nothing asks these layouts to stop.
fn never() -> bool {
    false
}

/// ADR-0008's background job, run to the end: every commit the refs reach,
/// in topological order, whether or not there is a commit-graph file.
fn topological_order(handle: &RepoHandle, shape: Shape) -> Vec<Oid> {
    let tips = handle
        .commit_tips()
        .unwrap_or_else(|why| panic!("reading the refs of the {shape} fixture: {why}"));
    let mut walk = Walk::full_topological(handle, &tips, table(handle))
        .unwrap_or_else(|why| panic!("starting the topological walk of {shape}: {why}"));
    assert!(!walk.is_provisional());
    let mut ids = Vec::new();
    loop {
        let chunk = walk
            .next_chunk(CHUNK, &never)
            .unwrap_or_else(|why| panic!("walking the {shape} fixture topologically: {why}"));
        ids.extend(chunk.commits.iter().map(|commit| commit.id));
        if chunk.done {
            return ids;
        }
    }
}

/// The walk the app starts with: topological asked for, whatever the
/// repository can serve in force.
fn first_paint_walk(handle: &RepoHandle, shape: Shape) -> Walk<'_> {
    Walk::all_refs(handle, Order::Topological, table(handle))
        .unwrap_or_else(|why| panic!("starting the walk of the {shape} fixture: {why}"))
}

/// Advances `stream` in budgets until its source is exhausted.
fn advance_to_the_end(stream: &mut StreamingLayout<WalkSource<'_>>, shape: Shape) {
    while !stream.is_done() {
        let step = stream
            .advance(BUDGET, &never)
            .unwrap_or_else(|why| panic!("{shape}: advancing the streaming layout: {why}"));
        assert!(!step.stopped, "{shape}: nothing asked the layout to stop");
    }
}

/// `stream`'s rows at `top`, with the viewport moved there first.
fn screen_at(stream: &mut StreamingLayout<WalkSource<'_>>, top: usize, shape: Shape) -> Vec<Row> {
    stream.set_viewport(top..top + HEIGHT);
    stream
        .rows(top..top + HEIGHT)
        .unwrap_or_else(|why| panic!("{shape}: reading rows {top}..{}: {why}", top + HEIGHT))
}

/// What the linear shape's scroll down and back cost: reads, rebuilds and
/// pass-throughs, counted.
fn assert_linear_work(stream: &StreamingLayout<WalkSource<'_>>, rows: usize) {
    let shape = Shape::Linear;
    let resident = stream.resident();
    let work = stream.work();
    assert!(
        work.chunks_rebuilt > 0,
        "{shape}: the scroll back rebuilt evicted chunks"
    );
    // Rows are hydrated by handle: every row the layout built, the
    // first time and on each rebuild, plus any merge read only to
    // name a lane, went through read_commit.
    assert_eq!(
        stream.source().reads,
        work.hydrated + work.merge_reads,
        "{shape}: every read went through read_commit"
    );
    // Chunks that passed through were laid out without a read; the
    // scroll then materialised every one of them.
    assert!(
        work.hydrated < work.commits_pulled + work.commits_reread,
        "{shape}: not every row laid out was read"
    );
    assert_eq!(resident.passed_through, 0, "{shape}: every row was shown");
    assert_eq!(
        work.links_read,
        work.commits_pulled + work.commits_reread,
        "{shape}"
    );
    assert!(work.commits_reread > 0, "{shape}");
    eprintln!(
        "{shape}: {} rows, {} pulled, {} reread, {} chunks rebuilt, {} checkpoints kept",
        rows, work.commits_pulled, work.commits_reread, work.chunks_rebuilt, resident.checkpoints,
    );
}

#[test]
fn every_shape_streams_through_a_small_cache_as_the_whole_layout_would() {
    for_each_shape(|shape, dir| {
        let (handle, seeds) = open_with_seeds(shape, dir);
        let (commits, _) = walk_everything(&handle, shape);
        let whole = layout(&commits, seeds.clone());

        let source = WalkSource::new(&handle, first_paint_walk(&handle, shape));
        let mut stream = StreamingLayout::new(source, seeds, SMALL);
        advance_to_the_end(&mut stream, shape);
        assert_eq!(stream.expected_rows(), Some(whole.len()), "{shape}");
        let below_the_window = whole
            .len()
            .saturating_sub(SMALL.chunk_rows + SMALL.buffer_rows);
        assert!(
            stream.resident().passed_through >= below_the_window,
            "{shape}: rows past the top's window pass through unread"
        );

        // Down through the history a screen at a time, then back to the top:
        // every screen is the slice of the whole layout, whether its chunk
        // was still held or had to be rebuilt.
        let tops: Vec<usize> = (0..whole.len()).step_by(HEIGHT).collect();
        for &top in tops.iter().chain(tops.iter().rev()) {
            let end = (top + HEIGHT).min(whole.len());
            assert_eq!(
                screen_at(&mut stream, top, shape),
                whole[top..end],
                "{shape} at {top}"
            );
            assert!(
                stream.resident().chunks <= SMALL.resident_chunks + 1,
                "{shape}: {} chunks held at row {top}, budget {}",
                stream.resident().chunks,
                SMALL.resident_chunks,
            );
        }

        let resident = stream.resident();
        let work = stream.work();
        assert!(resident.checkpoints <= SMALL.max_checkpoints, "{shape}");
        assert_eq!(
            work.commits_pulled,
            whole.len(),
            "{shape}: every commit walked once"
        );
        if shape == Shape::Linear {
            assert_linear_work(&stream, whole.len());
        }
    });
}

#[test]
fn promotion_on_a_real_repository_finds_the_selected_commit_again() {
    let shape = Shape::FeatureBranches;
    let dir = graft_fixtures::borrow(shape);
    let (handle, seeds) = open_with_seeds(shape, &dir);

    // First paint: the provisional order, streamed, with a commit selected
    // part-way down the screen.
    let (commits, walked) = walk_everything(&handle, shape);
    assert!(walked.provisional, "the corpus has no commit-graph file");
    let source = WalkSource::new(&handle, first_paint_walk(&handle, shape));
    let mut first = StreamingLayout::new(source, seeds.clone(), SMALL);
    advance_to_the_end(&mut first, shape);
    let selected = commits.len() / 2;
    let top = selected - 2;
    let screen = screen_at(&mut first, top, shape);
    let anchor = Anchor {
        commit: screen[selected - top].commit.id,
        offset: selected - top,
    };

    // The background job: the topological order of everything, whole.
    let ids = topological_order(&handle, shape);
    assert_eq!(ids.len(), commits.len());
    let in_topological_order: Vec<Commit> = ids
        .iter()
        .map(|&id| read_commit(&handle, id).expect("a walked commit reads back"))
        .collect();
    let whole = layout(&in_topological_order, seeds.clone());

    // Promotion: the same commits in the promoted order, re-read rather than
    // walked, advanced until the selected commit has a row again.
    let source = WalkSource::new(&handle, first_paint_walk(&handle, shape));
    // This walk's table starts empty (the corpus has no shared, persisted
    // table across the two walks this test drives) — the app hands the
    // topological order the same `Arc<CommitTable>` its walk used
    // (`WalkSource::in_order`), so here the promoted ids are interned into
    // this source's table before being resolved, standing in for that.
    let order: Vec<CommitHandle> = ids
        .iter()
        .map(|&id| source.walk.table().intern(&id))
        .collect();
    let mut promoted = StreamingLayout::from_order(source, order, seeds, SMALL)
        .expect("a walk emits every commit once");
    assert_eq!(promoted.expected_rows(), Some(commits.len()));
    let viewport = loop {
        if let Some(viewport) = promoted.reanchor(&anchor, HEIGHT) {
            break viewport;
        }
        assert!(
            !promoted.is_done(),
            "the selected commit is in the promoted order"
        );
        promoted
            .advance(BUDGET, &never)
            .expect("the promoted layout advances");
    };

    // The selected commit is back at its offset the moment it has a row,
    // even though the rows below it may not exist yet ...
    let rows = screen_at(&mut promoted, viewport.start, shape);
    assert_eq!(rows[anchor.offset].commit.id, anchor.commit);
    assert!(rows.len() <= HEIGHT);
    assert_eq!(rows, whole[viewport.start..viewport.start + rows.len()]);

    // ... and they fill in under it as the promoted layout catches up,
    // without moving it.
    advance_to_the_end(&mut promoted, shape);
    let rows = screen_at(&mut promoted, viewport.start, shape);
    assert_eq!(rows[anchor.offset].commit.id, anchor.commit);
    let end = viewport.end.min(whole.len());
    assert_eq!(rows, whole[viewport.start..end]);
    // A promoted layout never walks; it reads every row once, plus any merge
    // read only to name a lane.
    let (work, source) = (promoted.work(), promoted.source());
    let seen = (source.pulls, work.hydrated, source.reads);
    assert_eq!(seen, (0, commits.len(), commits.len() + work.merge_reads));
}
