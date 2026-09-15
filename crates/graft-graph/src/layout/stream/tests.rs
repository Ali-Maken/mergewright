//! The streaming layout against the one-call layout: whatever the chunking,
//! the eviction and the replays, the rows a caller sees are the rows
//! [`layout`] would have produced for the whole sequence.

use std::collections::HashMap;
use std::convert::Infallible;

use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::commit_handle::{CommitHandle, Link};
use proptest::prelude::*;

use super::*;
use crate::identity::BranchIdentity;
use crate::layout::layout;
use crate::layout::source::{CommitSource, Pull};
use crate::layout::tests::Handles;
use crate::row::RowShape;
use crate::seeds::{BranchSeed, Seeds};

// ---- a generated history -------------------------------------------------

/// A small deterministic generator, so a 20,000-commit history is a seed
/// and a length rather than a fixture on disk.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, bound: u32) -> usize {
        usize::try_from(self.next() % u64::from(bound)).expect("a value below a u32 fits")
    }
}

fn oid_for(index: u64, salt: u64) -> Oid {
    let mut bytes = [0u8; Oid::SHA1_BYTES];
    let mut lcg = Lcg(index.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt);
    for chunk in bytes.chunks_mut(8) {
        let word = lcg.next().to_be_bytes();
        chunk.copy_from_slice(&word[..chunk.len()]);
    }
    Oid::from_bytes(&bytes).expect("20 bytes is a valid oid length")
}

fn identity_at(seconds: i64) -> Identity {
    Identity {
        name: b"Gen".to_vec(),
        email: b"gen@example.invalid".to_vec(),
        when: Timestamp {
            seconds,
            offset_seconds: 0,
        },
    }
}

/// `count` commits in a valid walk order — every parent later than its
/// child — with bounded parallelism: a first parent is usually the very next
/// commit, sometimes a few further on, and one commit in six is a merge
/// whose second parent lies within the next couple of dozen. Timestamps
/// fall one minute per row, so commit-date order is this order.
fn history(count: usize, seed: u64) -> Vec<Commit> {
    let mut lcg = Lcg(seed);
    (0..count)
        .map(|index| {
            let mut parents = Vec::new();
            if index + 1 < count {
                let skip = if lcg.below(4) == 0 { lcg.below(6) } else { 0 };
                parents.push(oid_for((index + 1 + skip).min(count - 1) as u64, seed));
            }
            if index + 2 < count && lcg.below(6) == 0 {
                let second = (index + 2 + lcg.below(24)).min(count - 1);
                let id = oid_for(second as u64, seed);
                if !parents.contains(&id) {
                    parents.push(id);
                }
            }
            let seconds = 2_000_000_000 - i64::try_from(index).expect("a small index") * 60;
            Commit {
                id: oid_for(index as u64, seed),
                parents,
                author: identity_at(seconds),
                committer: identity_at(seconds),
                message: format!("commit {index}\n\nbody of commit {index}").into_bytes(),
            }
        })
        .collect()
}

fn seeds_for(commits: &[Commit]) -> Seeds {
    Seeds {
        default_branch: commits.first().map(|tip| BranchSeed {
            tip: tip.id,
            identity: BranchIdentity::of_name(b"main"),
        }),
        ..Seeds::default()
    }
}

/// The same commits in commit-date order after their committer dates are
/// jittered by up to `jitter` seconds: the provisional first paint of
/// ADR-0008, where some parents land above their children.
fn skewed_by_date(commits: &[Commit], jitter: u32, seed: u64) -> Vec<Commit> {
    let mut lcg = Lcg(seed);
    let mut skewed: Vec<Commit> = commits
        .iter()
        .cloned()
        .map(|mut commit| {
            let shift = i64::try_from(lcg.below(2 * jitter + 1)).expect("a small shift")
                - i64::from(jitter);
            commit.committer.when.seconds += shift;
            commit
        })
        .collect();
    skewed.sort_by(|a, b| {
        b.committer
            .when
            .seconds
            .cmp(&a.committer.when.seconds)
            .then_with(|| a.id.cmp(&b.id))
    });
    skewed
}

// ---- an in-memory source -------------------------------------------------

/// A source over a `Vec`, counting what is asked of it.
struct VecSource {
    walk: Vec<Commit>,
    by_id: HashMap<Oid, Commit>,
    position: usize,
    pulls: usize,
    reads: usize,
    handles: Handles,
}

impl VecSource {
    fn new(walk: Vec<Commit>) -> Self {
        let by_id = walk
            .iter()
            .map(|commit| (commit.id, commit.clone()))
            .collect();
        let handles = Handles::of(&walk);
        Self {
            walk,
            by_id,
            position: 0,
            pulls: 0,
            reads: 0,
            handles,
        }
    }

    /// A source that can be read by id but whose walk is a different order
    /// — or, for a promoted layout, is never pulled at all.
    fn readable(commits: &[Commit], walk: Vec<Commit>) -> Self {
        let mut source = Self::new(walk);
        source.by_id = commits
            .iter()
            .map(|commit| (commit.id, commit.clone()))
            .collect();
        source.handles = Handles::of(&[commits, &source.walk].concat());
        source
    }
}

impl CommitSource for VecSource {
    type Error = Infallible;

    fn pull(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, Infallible> {
        self.pulls += 1;
        let mut handles = Vec::new();
        let mut stopped = false;
        while handles.len() < limit && self.position < self.walk.len() {
            if stop() {
                stopped = true;
                break;
            }
            let id = self.walk[self.position].id;
            handles.push(
                self.handles
                    .handle_of(&id)
                    .expect("walked commits are interned"),
            );
            self.position += 1;
        }
        Ok(Pull {
            links: self.handles.links(&self.by_id, &handles),
            done: !stopped && self.position == self.walk.len(),
            stopped,
        })
    }

    fn read(&mut self, commits: &[CommitHandle]) -> Result<Vec<Commit>, Infallible> {
        self.reads += 1;
        Ok(commits
            .iter()
            .map(|&handle| {
                let id = self.handles.id_of(handle);
                self.by_id.get(&id).cloned().unwrap_or_else(|| {
                    panic!("the source was asked for {id}, which it never served")
                })
            })
            .collect())
    }

    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, Infallible> {
        Ok(self.handles.links(&self.by_id, commits))
    }

    fn handle_of(&self, id: &Oid) -> Option<CommitHandle> {
        self.handles.handle_of(id)
    }

    fn id_of(&self, handle: CommitHandle) -> Oid {
        self.handles.id_of(handle)
    }
}

fn never() -> bool {
    false
}

fn small() -> StreamConfig {
    StreamConfig {
        chunk_rows: 100,
        checkpoint_every: 400,
        max_checkpoints: 64,
        resident_chunks: 8,
        buffer_rows: 200,
    }
}

fn stream(commits: &[Commit], config: StreamConfig) -> StreamingLayout<VecSource> {
    StreamingLayout::new(VecSource::new(commits.to_vec()), seeds_for(commits), config)
}

fn advance_all<S: CommitSource<Error = Infallible>>(layout: &mut StreamingLayout<S>) {
    let advance = layout
        .advance(usize::MAX, &never)
        .expect("an in-memory source");
    assert!(advance.done && !advance.stopped);
}

// ---- acceptance 1: bounded resident set ---------------------------------

#[test]
fn scrolling_the_whole_history_keeps_the_resident_set_bounded() {
    let commits = history(100_000, 1);
    let whole = layout(&commits, seeds_for(&commits));
    let config = StreamConfig::default();
    let mut streaming = stream(&commits, config);
    streaming.set_viewport(0..60);
    advance_all(&mut streaming);

    // Streaming far past the viewport left only the window resident.
    let after_walk = streaming.resident();
    assert!(
        after_walk.chunks <= config.resident_chunks,
        "after the walk: {} chunks resident, budget {}",
        after_walk.chunks,
        config.resident_chunks
    );

    let mut most_chunks = 0;
    let mut most_rows = 0;
    let height = 60;
    let mut top = 0;
    while top < commits.len() {
        streaming.set_viewport(top..top + height);
        let rows = streaming
            .rows(top..top + height)
            .expect("an in-memory source");
        assert_eq!(rows, whole[top..(top + height).min(whole.len())]);
        let resident = streaming.resident();
        most_chunks = most_chunks.max(resident.chunks);
        most_rows = most_rows.max(resident.rows);
        top += 50;
    }
    // The window is at most: one checkpoint interval back from the top
    // buffer, the buffer itself, the viewport, and the bottom buffer.
    let window_rows = config.checkpoint_every + 2 * config.buffer_rows + height;
    let window_chunks = window_rows / config.chunk_rows + 2;
    eprintln!(
        "scrolled 100,000 rows: at most {most_chunks} chunks / {most_rows} rows resident \
         (budget {} chunks, window {window_chunks} chunks); {} checkpoints, {} bytes; order {} bytes",
        config.resident_chunks,
        streaming.resident().checkpoints,
        streaming.resident().checkpoint_bytes,
        streaming.resident().order_bytes,
    );
    assert!(most_chunks <= config.resident_chunks.max(window_chunks));
    assert!(most_rows <= most_chunks * config.chunk_rows);
    assert_eq!(
        streaming.resident().checkpoints,
        49,
        "a checkpoint every 2,000 rows short of 100,000"
    );
}

/// Checkpoints are few and the oldest go first, so after a long walk none
/// may stand before a viewport far behind the frontier. The protected window
/// still reaches back only to where a checkpoint is due, not to row 0:
/// otherwise every row laid out stays resident.
#[test]
fn a_viewport_before_every_kept_checkpoint_still_evicts_the_rows_before_it() {
    let commits = history(10_000, 3);
    let whole = layout(&commits, seeds_for(&commits));
    let config = StreamConfig {
        max_checkpoints: 2,
        resident_chunks: 2,
        ..small()
    };
    let mut streaming = stream(&commits, config);
    let height = 60;
    streaming.set_viewport(5_000..5_000 + height);
    advance_all(&mut streaming);
    assert!(
        streaming.checkpoint_rows().iter().all(|&row| row > 5_000),
        "no checkpoint is kept before the viewport: {:?}",
        streaming.checkpoint_rows()
    );

    let window_rows = config.checkpoint_every + 2 * config.buffer_rows + height;
    let bound = window_rows.div_ceil(config.chunk_rows) + config.resident_chunks;
    let held = streaming.resident().chunks;
    assert!(held <= bound, "{held} chunks resident, bound {bound}");

    let rows = streaming
        .rows(5_000..5_000 + height)
        .expect("an in-memory source");
    assert_eq!(rows, whole[5_000..5_000 + height]);
    let held = streaming.resident().chunks;
    assert!(
        held <= bound,
        "after reading: {held} chunks resident, bound {bound}"
    );
}

// ---- acceptance 2: an evicted chunk rebuilds byte-identical --------------

#[test]
fn an_evicted_chunk_rebuilds_identically_when_scrolled_back_to() {
    let commits = history(4_000, 2);
    let whole = layout(&commits, seeds_for(&commits));
    let mut streaming = stream(&commits, small());
    streaming.set_viewport(0..40);
    advance_all(&mut streaming);
    assert!(streaming.is_resident(0));

    // Scroll to the bottom a screen at a time. Every chunk on the way is
    // touched; once the cache is past its budget the oldest-touched
    // chunks outside the window go, chunk 0 first.
    for top in (0..=3_900).step_by(100) {
        streaming.set_viewport(top..top + 40);
        let screen = streaming.rows(top..top + 40).expect("an in-memory source");
        assert_eq!(screen, whole[top..top + 40]);
    }
    assert!(
        !streaming.is_resident(0),
        "chunk 0 is the oldest and far outside the window"
    );
    assert!(!streaming.is_resident(1_950));
    assert!(streaming.resident().chunks <= small().resident_chunks);

    let before = streaming.work();
    streaming.set_viewport(0..40);
    let top = streaming.rows(0..40).expect("an in-memory source");
    assert_eq!(
        top,
        whole[0..40],
        "rebuilt rows are the rows that were evicted"
    );
    assert!(streaming.is_resident(0));
    let after = streaming.work();
    assert!(after.chunks_rebuilt > before.chunks_rebuilt);
    assert_eq!(
        after.commits_pulled, before.commits_pulled,
        "a rebuild re-reads by id; it never pulls the walk again"
    );
}

/// A source whose walk is honest but whose re-served links drop every
/// parent. The handles match, so only the fingerprint can catch it.
struct Forgetful(VecSource, std::collections::HashSet<CommitHandle>);

impl CommitSource for Forgetful {
    type Error = Infallible;

    fn pull(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, Infallible> {
        let pull = self.0.pull(limit, stop)?;
        self.1.extend(pull.links.iter().map(|link| link.commit));
        Ok(pull)
    }

    fn read(&mut self, handles: &[CommitHandle]) -> Result<Vec<Commit>, Infallible> {
        self.0.read(handles)
    }

    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, Infallible> {
        let mut links = self.0.links(commits)?;
        for link in &mut links {
            if self.1.contains(&link.commit) {
                link.parents = graft_core::commit_handle::Parents::None;
            }
        }
        Ok(links)
    }

    fn handle_of(&self, id: &Oid) -> Option<CommitHandle> {
        self.0.handle_of(id)
    }

    fn id_of(&self, handle: CommitHandle) -> Oid {
        self.0.id_of(handle)
    }
}

#[test]
fn a_rebuild_that_does_not_match_the_evicted_rows_is_an_error_not_a_display() {
    let commits = history(2_000, 3);
    let mut streaming = StreamingLayout::new(
        Forgetful(
            VecSource::new(commits.clone()),
            std::collections::HashSet::new(),
        ),
        seeds_for(&commits),
        small(),
    );
    streaming.set_viewport(1_900..1_940);
    advance_all(&mut streaming);
    assert!(!streaming.is_resident(0));

    streaming.set_viewport(0..40);
    match streaming.rows(0..40) {
        Err(StreamError::RebuildDiffers { start: 0, end: 100 }) => {}
        other => panic!("expected RebuildDiffers for rows 0..100, got {other:?}"),
    }
}

// ---- acceptance 3: a deep jump resumes from a checkpoint -----------------

#[test]
fn a_jump_to_an_evicted_row_replays_from_the_nearest_checkpoint_not_row_zero() {
    let commits = history(20_000, 4);
    let whole = layout(&commits, seeds_for(&commits));
    let config = small();
    let mut streaming = stream(&commits, config);
    streaming.set_viewport(0..40);
    advance_all(&mut streaming);
    assert_eq!(
        streaming.checkpoint_rows().len(),
        49,
        "every 400 rows short of 20,000"
    );

    let before = streaming.work();
    streaming.set_viewport(15_000..15_040);
    let rows = streaming.rows(15_000..15_040).expect("an in-memory source");
    assert_eq!(rows, whole[15_000..15_040]);
    let after = streaming.work();

    // Chunk 150 is rows 15,000..15,100; the checkpoint at or before it is
    // 14,800. Exactly that span is replayed — not 15,100 rows from the top.
    assert_eq!(after.rows_replayed - before.rows_replayed, 300);
    assert_eq!(after.commits_reread - before.commits_reread, 300);
    assert_eq!(after.commits_pulled, before.commits_pulled);
    eprintln!(
        "jump to row 15,000: replayed {} rows from checkpoint 14,800",
        after.rows_replayed - before.rows_replayed
    );
}

#[test]
fn a_cold_jump_is_advanced_in_budgets_and_can_be_stopped_and_resumed() {
    let commits = history(7_000, 5);
    let whole = layout(&commits, seeds_for(&commits));
    let mut streaming = stream(&commits, small());
    let target = 5_000;

    // A driver that wants row 5,000 from cold advances in budgets, checking
    // for cancellation between them; nothing here blocks on the whole walk.
    let mut calls = 0;
    while streaming.rows_emitted() < target + 40 {
        let stop = |calls: usize| move || calls == 3;
        let advance = streaming
            .advance(1_000, &stop(calls))
            .expect("an in-memory source");
        calls += 1;
        if advance.stopped {
            assert_eq!(calls, 4, "the fourth call saw the stop flag");
            assert_eq!(advance.rows_emitted, 0);
            assert_eq!(
                streaming.rows_emitted(),
                3_000,
                "progress is visible while stopped"
            );
        }
    }
    assert!(!streaming.is_done());
    assert_eq!(
        streaming.rows_emitted(),
        6_000,
        "the driver stopped once the target was covered"
    );
    assert_eq!(
        streaming.source().pulls,
        streaming.rows_emitted() / 100,
        "one pull per chunk: a stopped advance pulls nothing, a resumed one repeats nothing"
    );

    streaming.set_viewport(target..target + 40);
    let rows = streaming
        .rows(target..target + 40)
        .expect("an in-memory source");
    assert_eq!(rows, whole[target..target + 40]);
    assert_eq!(
        streaming.expected_rows(),
        None,
        "the walk has not said how long it is"
    );
}

// ---- acceptance 4: promotion keeps the selection ------------------------

/// The provisional date-order first paint: the user has scrolled some way
/// in and selected the commit ten rows down the screen. Returns the anchor
/// and the row it sat on.
fn selected_on_first_paint(
    dated: &[Commit],
    seeds: &Seeds,
    config: StreamConfig,
) -> (Anchor, usize) {
    let mut provisional =
        StreamingLayout::new(VecSource::new(dated.to_vec()), seeds.clone(), config);
    provisional.set_viewport(3_000..3_040);
    advance_all(&mut provisional);
    let on_screen = provisional.rows(3_000..3_040).expect("an in-memory source");
    let anchor = Anchor {
        commit: on_screen[10].commit.id,
        offset: 10,
    };
    let row = provisional
        .row_of(anchor.commit)
        .expect("the selected commit is laid out");
    assert_eq!(row, 3_010);
    (anchor, row)
}

#[test]
fn promotion_from_date_order_to_topological_keeps_the_selected_commit_on_screen() {
    let topo = history(6_000, 6);
    let dated = skewed_by_date(&topo, 300, 7);
    assert_ne!(
        dated.iter().map(|c| c.id).collect::<Vec<_>>(),
        topo.iter().map(|c| c.id).collect::<Vec<_>>(),
        "the skew must actually reorder something"
    );
    let seeds = seeds_for(&topo);
    let config = small();
    let (anchor, provisional_row) = selected_on_first_paint(&dated, &seeds, config);

    // Background: the topological walk finished and produced its order.
    let source = VecSource::readable(&topo, Vec::new());
    let order: Vec<CommitHandle> = topo
        .iter()
        .map(|commit| {
            CommitSource::handle_of(&source, &commit.id).expect("interned by VecSource::readable")
        })
        .collect();
    let mut promoted = StreamingLayout::from_order(source, order, seeds.clone(), config)
        .expect("a topological walk yields each commit once");
    assert_eq!(promoted.expected_rows(), Some(6_000));
    let target = promoted
        .row_of(anchor.commit)
        .expect("the order knows every row up front");
    assert_ne!(
        target, provisional_row,
        "the skew moved the selected commit"
    );

    // Advance until the anchor can be honoured, then re-anchor by commit id.
    let mut viewport = None;
    while viewport.is_none() {
        let advance = promoted.advance(500, &never).expect("an in-memory source");
        assert!(advance.rows_emitted > 0 || advance.done);
        viewport = promoted.reanchor(&anchor, 40);
    }
    let viewport = viewport.expect("found above");
    assert!(
        promoted.rows_emitted() <= target + 500,
        "the job stopped as soon as the anchor was covered"
    );
    promoted.set_viewport(viewport.clone());
    let rows = promoted
        .rows(viewport.clone())
        .expect("an in-memory source");
    assert_eq!(
        rows[10].commit.id, anchor.commit,
        "still selected, same place on screen"
    );
    assert_eq!(viewport.start + 10, target);

    // The promoted rows are the topological layout, and nothing was pulled
    // from the walk to get them: the order was the walker state.
    let whole = layout(&topo, seeds);
    assert_eq!(rows, whole[viewport]);
    assert_eq!(promoted.source().pulls, 0);
    assert_eq!(promoted.work().commits_pulled, 0);
}

// ---- acceptance 5 / R2-1: checkpoints are O(1) in remaining history ------

#[test]
fn checkpoint_size_does_not_grow_with_history_on_either_path() {
    let config = StreamConfig::default();
    let per_checkpoint = |commits: &[Commit], promoted: bool| -> (usize, usize, usize) {
        let mut streaming = if promoted {
            let source = VecSource::readable(commits, Vec::new());
            let order: Vec<CommitHandle> = commits
                .iter()
                .map(|commit| {
                    CommitSource::handle_of(&source, &commit.id)
                        .expect("interned by VecSource::readable")
                })
                .collect();
            StreamingLayout::from_order(source, order, seeds_for(commits), config)
                .expect("distinct ids")
        } else {
            stream(commits, config)
        };
        advance_all(&mut streaming);
        let resident = streaming.resident();
        (
            resident.checkpoints,
            resident.checkpoint_bytes,
            resident.checkpoint_bytes / resident.checkpoints.max(1),
        )
    };

    let short = history(10_000, 8);
    let long = history(40_000, 8);
    for promoted in [false, true] {
        let (short_count, short_bytes, short_each) = per_checkpoint(&short, promoted);
        let (long_count, long_bytes, long_each) = per_checkpoint(&long, promoted);
        eprintln!(
            "{}: 10,000 rows -> {short_count} checkpoints, {short_bytes} bytes ({short_each} each); \
             40,000 rows -> {long_count} checkpoints, {long_bytes} bytes ({long_each} each)",
            if promoted {
                "promoted (from_order)"
            } else {
                "streamed (new)"
            }
        );
        assert_eq!(short_count, 4);
        assert_eq!(long_count, 19);
        assert!(
            long_each <= short_each * 2 && short_each <= long_each * 2,
            "a checkpoint's size tracks open lanes, not rows above it: {short_each} vs {long_each}"
        );
        assert!(
            long_each < 16 * 1024,
            "a checkpoint is kilobytes, got {long_each}"
        );
    }
}

// ---- merge lanes named across a chunk boundary ---------------------------

#[test]
fn a_merge_lane_open_across_a_chunk_boundary_is_named_with_one_read() {
    use crate::layout::tests::commit;
    // M merges T into main; A continues main. Chunk 0 is M, A (the merge on
    // hand); chunk 1 is T, R, still drawing M's lane with M above it.
    let commits = vec![
        commit(1, &[2, 3], 400, b"Merge branch 'topic'"),
        commit(2, &[4], 300, b"main work"),
        commit(3, &[4], 200, b"topic work"),
        commit(4, &[], 100, b"root"),
    ];
    let seeds = Seeds {
        default_branch: Some(BranchSeed {
            tip: commits[0].id,
            identity: BranchIdentity::of_name(b"main"),
        }),
        ..Seeds::default()
    };
    let config = StreamConfig {
        chunk_rows: 2,
        checkpoint_every: 2,
        max_checkpoints: 8,
        resident_chunks: 8,
        buffer_rows: 0,
    };
    let whole = layout(&commits, seeds.clone());
    let mut streaming = StreamingLayout::new(VecSource::new(commits.clone()), seeds, config);
    streaming.set_viewport(0..4);

    streaming.advance(2, &never).expect("an in-memory source");
    assert_eq!(streaming.work().hydrated, 2);
    assert_eq!(
        streaming.work().merge_reads,
        0,
        "the merge is in the chunk that draws its lane"
    );

    streaming.advance(2, &never).expect("an in-memory source");
    assert_eq!(streaming.work().hydrated, 4);
    assert_eq!(
        streaming.work().merge_reads,
        1,
        "one read names the lane the merge above opened"
    );

    let rows = streaming.rows(0..4).expect("an in-memory source");
    assert_eq!(rows, whole, "deferred names resolve to today's identities");
    assert_eq!(rows[2].lanes[1], Some(BranchIdentity::of_name(b"topic")));
}

// ---- rows hydrate on demand (m1-the-graph/52b) --------------------------

/// The 5k synthetic pair: a history in topological order, and the same
/// commits in jittered commit-date order.
fn five_thousand_pair() -> [(Vec<Commit>, Seeds); 2] {
    let commits = history(5_000, 11);
    let seeds = seeds_for(&commits);
    let skewed = skewed_by_date(&commits, 600, 11);
    [(commits, seeds.clone()), (skewed, seeds)]
}

#[test]
fn chunks_passed_through_rebuild_to_the_rows_a_layout_materialising_everything_gives() {
    for (walk, seeds) in five_thousand_pair() {
        let total = walk.len();
        let whole = layout(&walk, seeds.clone());

        let mut everything =
            StreamingLayout::new(VecSource::new(walk.clone()), seeds.clone(), small());
        everything.set_viewport(0..total);
        advance_all(&mut everything);
        assert_eq!(everything.resident().passed_through, 0);
        let materialised = everything.rows(0..total).expect("an in-memory source");

        // Viewport at the top, advanced in odd budgets so chunks pass through
        // a part at a time.
        let mut streaming = StreamingLayout::new(VecSource::new(walk), seeds, small());
        streaming.set_viewport(0..40);
        while !streaming.is_done() {
            streaming.advance(37, &never).expect("an in-memory source");
        }
        let config = small();
        let window = config.chunk_rows + config.buffer_rows;
        assert_eq!(streaming.resident().passed_through, total - window);
        assert_eq!(
            streaming.work().hydrated,
            window,
            "only the top's window was read"
        );

        let rows = streaming
            .rows(0..total)
            .expect("every rebuilt chunk matches its fingerprint");
        assert_eq!(rows, materialised, "field by field, lanes included");
        assert_eq!(rows, whole);
        assert_eq!(streaming.resident().passed_through, 0);
    }
}

#[test]
fn a_cold_jump_with_the_viewport_at_the_top_reads_only_the_top() {
    let config = small();
    for (walk, seeds) in five_thousand_pair() {
        let mut streaming = StreamingLayout::new(VecSource::new(walk), seeds, config);
        streaming.set_viewport(0..40);
        advance_all(&mut streaming);
        let work = streaming.work();
        assert!(
            work.hydrated <= 2 * config.chunk_rows + config.buffer_rows,
            "{work:?}"
        );
        assert_eq!(work.links_read, work.commits_pulled);
        eprintln!("cold jump, viewport at the top: {work:?}");

        let bottom = streaming.rows_emitted() - 40;
        streaming.set_viewport(bottom..bottom + 40);
        streaming
            .rows(bottom..bottom + 40)
            .expect("an in-memory source");
        let work = streaming.work();
        assert!(work.links_read - work.commits_pulled >= work.rows_replayed);
        assert!(work.rows_replayed <= config.checkpoint_every + config.chunk_rows);
    }
}

#[test]
fn a_deep_read_names_only_the_merge_lanes_open_at_its_first_row() {
    let config = StreamConfig {
        chunk_rows: 50,
        checkpoint_every: 200,
        max_checkpoints: 64,
        resident_chunks: 4,
        buffer_rows: 0,
    };
    let deep = 3_000;
    for (walk, seeds) in five_thousand_pair() {
        // The merge lanes open at `deep`, from the shapes themselves.
        let mut source = VecSource::new(walk.clone());
        let handles: Vec<CommitHandle> = walk
            .iter()
            .map(|commit| source.handle_of(&commit.id).expect("interned"))
            .collect();
        let links = source
            .links(&handles[..=deep])
            .expect("an in-memory source");
        let mut cursor = LayoutCursor::new(seeds.clone(), &source);
        let shapes = cursor.extend(&links, &source, &mut EmittedOrder::new());
        let mut open: Vec<CommitHandle> = shapes[deep]
            .lanes
            .iter()
            .flatten()
            .filter_map(|identity| match *identity {
                crate::lane::LaneIdentity::Merge { merge, .. } => Some(merge),
                crate::lane::LaneIdentity::Known(_) => None,
            })
            .collect();
        open.sort_unstable();
        open.dedup();

        let mut streaming = StreamingLayout::new(source, seeds, config);
        streaming.set_viewport(0..10);
        advance_all(&mut streaming);
        let before = streaming.work();
        streaming.set_viewport(deep..deep + 10);
        streaming
            .rows(deep..deep + 10)
            .expect("an in-memory source");
        let after = streaming.work();

        assert_eq!(
            after.hydrated - before.hydrated,
            config.chunk_rows,
            "one chunk read"
        );
        assert!(
            after.merge_reads - before.merge_reads <= open.len(),
            "{} merge reads, {} merge lanes open at row {deep}",
            after.merge_reads - before.merge_reads,
            open.len()
        );
        eprintln!(
            "deep read: {} merge reads, {} merge lanes open",
            after.merge_reads - before.merge_reads,
            open.len()
        );
    }
}

/// Distinct `Merge` identities drawn on `shape`: what naming its lanes may
/// cost when a batch starts on it.
fn merge_lanes_open(shape: &RowShape) -> usize {
    let mut open: Vec<CommitHandle> = shape
        .lanes
        .iter()
        .flatten()
        .filter_map(|identity| match *identity {
            crate::lane::LaneIdentity::Merge { merge, .. } => Some(merge),
            crate::lane::LaneIdentity::Known(_) => None,
        })
        .collect();
    open.sort_unstable();
    open.dedup();
    open.len()
}

#[test]
fn passing_through_the_rest_of_history_reads_nothing() {
    let config = small();
    // Viewport 0..40 plus `buffer_rows` meets chunks 0, 1 and 2.
    let window = 3 * config.chunk_rows;
    for (walk, seeds) in five_thousand_pair() {
        let mut to_the_end =
            StreamingLayout::new(VecSource::new(walk.clone()), seeds.clone(), config);
        to_the_end.set_viewport(0..40);
        advance_all(&mut to_the_end);

        let mut window_only =
            StreamingLayout::new(VecSource::new(walk.clone()), seeds.clone(), config);
        window_only.set_viewport(0..40);
        while window_only.rows_emitted() < window {
            window_only
                .advance(window - window_only.rows_emitted(), &never)
                .expect("an in-memory source");
        }
        assert!(!window_only.is_done());

        let (end, top) = (to_the_end.work(), window_only.work());
        assert_eq!(top.hydrated, window, "the window itself was read");
        assert_eq!(end.hydrated, top.hydrated, "pass-through hydrates nothing");
        assert_eq!(
            end.merge_reads, top.merge_reads,
            "pass-through names nothing"
        );

        // 52's rule: each materialised chunk after the first is its own
        // batch, and may read once per merge lane open at its first row.
        let mut source = VecSource::new(walk.clone());
        let handles: Vec<CommitHandle> = walk
            .iter()
            .map(|commit| source.handle_of(&commit.id).expect("interned"))
            .collect();
        let links = source
            .links(&handles[..window])
            .expect("an in-memory source");
        let mut cursor = LayoutCursor::new(seeds, &source);
        let shapes = cursor.extend(&links, &source, &mut EmittedOrder::new());
        let bound: usize = (1..3)
            .map(|chunk| merge_lanes_open(&shapes[chunk * config.chunk_rows]))
            .sum();
        assert!(
            end.merge_reads <= bound,
            "{} merge reads, bound {bound}",
            end.merge_reads
        );
        eprintln!("cold jump: {} merge reads, bound {bound}", end.merge_reads);
    }
}

// ---- contracts -----------------------------------------------------------

struct Stalling;

impl CommitSource for Stalling {
    type Error = Infallible;

    fn pull(&mut self, _: usize, _: &dyn Fn() -> bool) -> Result<Pull, Infallible> {
        Ok(Pull::default())
    }

    fn read(&mut self, _: &[CommitHandle]) -> Result<Vec<Commit>, Infallible> {
        Ok(Vec::new())
    }

    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, Infallible> {
        assert!(
            commits.is_empty(),
            "a source that served nothing has no handles"
        );
        Ok(Vec::new())
    }

    fn handle_of(&self, _: &Oid) -> Option<CommitHandle> {
        None
    }

    fn id_of(&self, handle: CommitHandle) -> Oid {
        panic!("a source that served nothing has no handle {handle:?}")
    }
}

#[test]
fn a_source_that_returns_nothing_without_finishing_is_an_error_not_a_spin() {
    let mut streaming = StreamingLayout::new(Stalling, Seeds::default(), small());
    assert!(matches!(
        streaming.advance(10, &never),
        Err(StreamError::Stalled)
    ));
}

#[test]
fn a_walk_that_repeats_a_commit_is_refused() {
    let mut commits = history(30, 9);
    commits.push(commits[3].clone());
    let mut streaming = stream(&commits, small());
    match streaming.advance(usize::MAX, &never) {
        Err(StreamError::Repeated(RepeatedCommit {
            first: 3,
            again: 30,
            ..
        })) => {}
        other => panic!("expected the repeat to be refused, got {other:?}"),
    }
}

#[test]
fn a_promoted_order_that_repeats_a_commit_is_refused() {
    let commits = history(30, 10);
    let source = VecSource::readable(&commits, Vec::new());
    let mut order: Vec<CommitHandle> = commits
        .iter()
        .map(|commit| {
            CommitSource::handle_of(&source, &commit.id).expect("interned by VecSource::readable")
        })
        .collect();
    order.push(order[5]);
    let outcome = StreamingLayout::from_order(source, order, seeds_for(&commits), small());
    assert!(matches!(
        outcome.map(|_| ()),
        Err(StreamError::Repeated(RepeatedCommit {
            first: 5,
            again: 30,
            ..
        }))
    ));
}

#[test]
fn a_source_that_answers_a_read_with_the_wrong_commit_is_caught() {
    let commits = history(30, 11);
    // A promoted layout over a source whose id table maps every id to the
    // wrong commit: the walk order is right, the reads lie.
    let mut wrong = VecSource::readable(&commits, Vec::new());
    let shifted: Vec<Commit> = commits
        .iter()
        .cycle()
        .skip(1)
        .take(commits.len())
        .cloned()
        .collect();
    wrong.by_id = commits
        .iter()
        .zip(&shifted)
        .map(|(asked, served)| (asked.id, served.clone()))
        .collect();
    let order: Vec<CommitHandle> = commits
        .iter()
        .map(|commit| {
            CommitSource::handle_of(&wrong, &commit.id).expect("interned by VecSource::readable")
        })
        .collect();
    let mut streaming = StreamingLayout::from_order(wrong, order, seeds_for(&commits), small())
        .expect("distinct ids");
    match streaming.advance(10, &never) {
        Err(StreamError::WrongCommit { asked, got }) => {
            assert_eq!(asked, commits[0].id);
            assert_eq!(got, commits[1].id);
        }
        other => panic!("expected WrongCommit, got {other:?}"),
    }
}

#[test]
fn an_empty_promoted_order_is_done_before_it_starts() {
    let mut streaming =
        StreamingLayout::from_order(Stalling, Vec::new(), Seeds::default(), small())
            .expect("nothing to repeat");
    assert!(streaming.is_done());
    assert_eq!(streaming.expected_rows(), Some(0));
    let advance = streaming.advance(10, &never).expect("nothing to do");
    assert_eq!(
        advance,
        Advance {
            rows_emitted: 0,
            done: true,
            stopped: false
        }
    );
}

// ---- the property ----------------------------------------------------------

fn tier() -> ProptestConfig {
    ProptestConfig::with_cases(graft_fixtures::proptest_cases())
}

#[derive(Clone, Debug)]
enum Op {
    Advance(usize),
    Look(usize, usize),
}

fn ops() -> impl Strategy<Value = Vec<Op>> {
    proptest::collection::vec(
        prop_oneof![
            (1usize..=40).prop_map(Op::Advance),
            (0usize..400, 1usize..=12).prop_map(|(top, height)| Op::Look(top, height)),
        ],
        1..=40,
    )
}

fn configs() -> impl Strategy<Value = StreamConfig> {
    (1usize..=7, 1usize..=3, 1usize..=4, 1usize..=3, 0usize..=10).prop_map(
        |(chunk_rows, checkpoint_chunks, resident_chunks, max_checkpoints, buffer_rows)| {
            StreamConfig {
                chunk_rows,
                checkpoint_every: chunk_rows * checkpoint_chunks,
                max_checkpoints,
                resident_chunks,
                buffer_rows,
            }
        },
    )
}

proptest! {
    #![proptest_config(tier())]

    /// Under any chunking, any viewport moves, any eviction and any
    /// checkpoint policy, every row the streaming layout shows is the row
    /// the one-call layout produces for the same position.
    #[test]
    fn streaming_shows_exactly_what_the_whole_layout_would(
        count in 0usize..300,
        seed in any::<u64>(),
        config in configs(),
        ops in ops(),
    ) {
        let commits = history(count, seed);
        let whole = layout(&commits, seeds_for(&commits));
        let mut streaming = stream(&commits, config);
        for op in ops {
            match op {
                Op::Advance(budget) => {
                    let advance = streaming.advance(budget, &never).expect("an in-memory source");
                    prop_assert_eq!(advance.done, streaming.rows_emitted() == count);
                }
                Op::Look(top, height) => {
                    streaming.set_viewport(top..top + height);
                    let rows = streaming.rows(top..top + height).expect("an in-memory source");
                    let end = (top + height).min(streaming.rows_emitted());
                    let start = top.min(end);
                    prop_assert_eq!(&rows, &whole[start..end]);
                }
            }
            prop_assert!(streaming.resident().checkpoints <= config.max_checkpoints);
        }
        streaming.set_viewport(0..0);
        advance_all(&mut streaming);
        let all = streaming.rows(0..count).expect("an in-memory source");
        prop_assert_eq!(&all, &whole);
    }
}

// ---- checkpoints thin to an even grid (m1-the-graph/60) ------------------

/// `count` commits, first-parent only, no merges: a linear chain, so a jump
/// replays exactly one link per row.
fn linear(count: usize) -> Vec<Commit> {
    (0..count)
        .map(|index| {
            let parents = if index + 1 < count {
                vec![oid_for((index + 1) as u64, 0)]
            } else {
                Vec::new()
            };
            let seconds = 2_000_000_000 - i64::try_from(index).expect("a small index") * 60;
            Commit {
                id: oid_for(index as u64, 0),
                parents,
                author: identity_at(seconds),
                committer: identity_at(seconds),
                message: format!("commit {index}").into_bytes(),
            }
        })
        .collect()
}

/// The invariant (m1-the-graph/60): the held checkpoints are exactly the
/// multiples of the current stride short of the frontier, and never more
/// than the cap.
fn assert_checkpoint_invariant<S: CommitSource<Error = Infallible>>(
    streaming: &StreamingLayout<S>,
    max_checkpoints: usize,
) {
    let stride = streaming.checkpoint_stride();
    let frontier = streaming.rows_emitted();
    // A row on the grid is due as it is passed, including the frontier
    // itself right after `advance` reaches it — except at the very end of
    // history, where nothing resumes from a checkpoint anyway.
    let bound = if streaming.is_done() {
        frontier.saturating_sub(1)
    } else {
        frontier
    };
    let expected: Vec<usize> = (1..=bound / stride).map(|k| k * stride).collect();
    let held = streaming.checkpoint_rows();
    assert_eq!(
        held, expected,
        "held checkpoints must be exactly the grid short of row {frontier} at stride {stride}"
    );
    assert!(
        held.len() <= max_checkpoints,
        "{} checkpoints held, cap {max_checkpoints}",
        held.len()
    );
}

#[test]
fn checkpoints_thin_to_an_even_grid_as_history_grows() {
    let commits = linear(1_000);
    let config = StreamConfig {
        chunk_rows: 10,
        checkpoint_every: 10,
        max_checkpoints: 8,
        resident_chunks: 1_000,
        buffer_rows: 0,
    };
    let mut streaming = stream(&commits, config);
    streaming.set_viewport(0..10);
    loop {
        let advance = streaming.advance(7, &never).expect("an in-memory source");
        assert_checkpoint_invariant(&streaming, config.max_checkpoints);
        if advance.done {
            break;
        }
    }
    assert_eq!(
        streaming.checkpoint_stride(),
        160,
        "10 -> 20 -> 40 -> 80 -> 160 as 1,000 rows pass 8 checkpoints four times over"
    );
}

/// Below the cap, nothing ever thins: a jump replays from the checkpoint at
/// the original `checkpoint_every`, exactly as before this ticket.
#[test]
fn under_the_cap_the_stride_never_changes() {
    let commits = linear(1_000);
    let config = small(); // checkpoint_every: 400, max_checkpoints: 64
    let mut streaming = stream(&commits, config);
    streaming.set_viewport(0..10);
    advance_all(&mut streaming);
    assert_eq!(streaming.checkpoint_stride(), config.checkpoint_every);
    assert_checkpoint_invariant(&streaming, config.max_checkpoints);
}

/// A jump to rows evicted from both the row cache and the checkpoint grid
/// replays at most one stride's worth of links plus one chunk — not the
/// whole history back to row 0.
#[test]
fn a_jump_after_thinning_replays_at_most_one_stride_plus_a_chunk() {
    let commits = linear(1_000);
    let config = StreamConfig {
        chunk_rows: 10,
        checkpoint_every: 10,
        max_checkpoints: 8,
        resident_chunks: 2,
        buffer_rows: 0,
    };
    let mut streaming = stream(&commits, config);
    streaming.set_viewport(0..10);
    advance_all(&mut streaming);
    assert_eq!(streaming.checkpoint_stride(), 160);

    // Scroll far away so row 500's chunk is evicted from the row cache.
    streaming.set_viewport(990..1_000);
    streaming
        .rows(990..1_000)
        .expect("an in-memory source, everything laid out");
    assert!(!streaming.is_resident(500));

    let before = streaming.work().rows_replayed;
    streaming.set_viewport(500..510);
    let rows = streaming
        .rows(500..510)
        .expect("an in-memory source, everything laid out");
    assert_eq!(rows.len(), 10);
    let replayed = streaming.work().rows_replayed - before;
    let bound = streaming.checkpoint_stride() + config.chunk_rows;
    assert!(
        replayed <= bound,
        "jump to row 500 replayed {replayed} rows, bound {bound} \
         (one stride of {} plus one chunk of {})",
        streaming.checkpoint_stride(),
        config.chunk_rows
    );
}

/// `truncate` drops checkpoints past the cut and keeps the grid invariant;
/// the stride itself never shrinks back down, even though the truncated
/// layout now holds fewer checkpoints than the cap allows.
#[test]
fn truncate_keeps_the_grid_invariant_and_never_shrinks_the_stride() {
    let commits = linear(1_000);
    let config = StreamConfig {
        chunk_rows: 10,
        checkpoint_every: 10,
        max_checkpoints: 8,
        resident_chunks: 1_000,
        buffer_rows: 0,
    };
    let mut streaming = stream(&commits, config);
    streaming.set_viewport(0..10);
    advance_all(&mut streaming);
    assert_checkpoint_invariant(&streaming, config.max_checkpoints);
    let stride_before = streaming.checkpoint_stride();
    assert_eq!(stride_before, 160);

    streaming.truncate(430).expect("an in-memory source");
    assert_checkpoint_invariant(&streaming, config.max_checkpoints);
    assert_eq!(
        streaming.checkpoint_stride(),
        stride_before,
        "the stride does not shrink just because truncate left the grid sparser"
    );

    streaming.set_viewport(0..430);
    advance_all(&mut streaming);
    assert_checkpoint_invariant(&streaming, config.max_checkpoints);
    assert_eq!(streaming.checkpoint_stride(), stride_before);
}
