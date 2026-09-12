//! The streaming layout against the one-call layout: whatever the chunking,
//! the eviction and the replays, the rows a caller sees are the rows
//! [`layout`] would have produced for the whole sequence.

use std::collections::HashMap;
use std::convert::Infallible;

use graft_core::commit::{Identity, Timestamp};
use proptest::prelude::*;

use super::*;
use crate::identity::BranchIdentity;
use crate::layout::layout;
use crate::seeds::BranchSeed;

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
}

impl VecSource {
    fn new(walk: Vec<Commit>) -> Self {
        let by_id = walk
            .iter()
            .map(|commit| (commit.id, commit.clone()))
            .collect();
        Self {
            walk,
            by_id,
            position: 0,
            pulls: 0,
            reads: 0,
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
        source
    }
}

impl CommitSource for VecSource {
    type Error = Infallible;

    fn next(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, Infallible> {
        self.pulls += 1;
        let mut commits = Vec::new();
        while commits.len() < limit && self.position < self.walk.len() {
            if stop() {
                return Ok(Pull {
                    commits,
                    done: false,
                    stopped: true,
                });
            }
            commits.push(self.walk[self.position].clone());
            self.position += 1;
        }
        Ok(Pull {
            commits,
            done: self.position == self.walk.len(),
            stopped: false,
        })
    }

    fn read(&mut self, ids: &[Oid]) -> Result<Vec<Commit>, Infallible> {
        self.reads += 1;
        Ok(ids
            .iter()
            .map(|id| {
                self.by_id.get(id).cloned().unwrap_or_else(|| {
                    panic!("the source was asked for {id}, which it never served")
                })
            })
            .collect())
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

/// A source that hands back a different commit on re-read than it did on
/// the walk. The ids match, so only the fingerprint can catch it.
struct Forgetful(VecSource);

impl CommitSource for Forgetful {
    type Error = Infallible;

    fn next(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, Infallible> {
        self.0.next(limit, stop)
    }

    fn read(&mut self, ids: &[Oid]) -> Result<Vec<Commit>, Infallible> {
        let mut commits = self.0.read(ids)?;
        for commit in &mut commits {
            commit.message = b"rewritten".to_vec();
        }
        Ok(commits)
    }
}

#[test]
fn a_rebuild_that_does_not_match_the_evicted_rows_is_an_error_not_a_display() {
    let commits = history(2_000, 3);
    let mut streaming = StreamingLayout::new(
        Forgetful(VecSource::new(commits.clone())),
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
    let ids: Vec<Oid> = topo.iter().map(|commit| commit.id).collect();
    let mut promoted = StreamingLayout::from_order(
        VecSource::readable(&topo, Vec::new()),
        ids,
        seeds.clone(),
        config,
    )
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
            let ids = commits.iter().map(|commit| commit.id).collect();
            StreamingLayout::from_order(
                VecSource::readable(commits, Vec::new()),
                ids,
                seeds_for(commits),
                config,
            )
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

// ---- contracts -----------------------------------------------------------

struct Stalling;

impl CommitSource for Stalling {
    type Error = Infallible;

    fn next(&mut self, _: usize, _: &dyn Fn() -> bool) -> Result<Pull, Infallible> {
        Ok(Pull::default())
    }

    fn read(&mut self, _: &[Oid]) -> Result<Vec<Commit>, Infallible> {
        Ok(Vec::new())
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
    let mut ids: Vec<Oid> = commits.iter().map(|commit| commit.id).collect();
    ids.push(ids[5]);
    let outcome = StreamingLayout::from_order(
        VecSource::readable(&commits, Vec::new()),
        ids,
        seeds_for(&commits),
        small(),
    );
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
    let ids: Vec<Oid> = commits.iter().map(|commit| commit.id).collect();
    let mut streaming = StreamingLayout::from_order(wrong, ids, seeds_for(&commits), small())
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
