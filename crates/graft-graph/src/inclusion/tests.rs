use std::collections::HashMap;
use std::convert::Infallible;

use proptest::prelude::*;

use super::*;
use crate::identity::BranchIdentity;
use crate::layout::layout;
use crate::layout::stream::StreamConfig;
use crate::layout::tests::properties::viewed;
use crate::layout::tests::{commit, oid};
use crate::row::Row;
use crate::seeds::BranchSeed;

/// A walk held in memory, readable by id.
struct VecSource {
    walk: Vec<Commit>,
    by_id: HashMap<Oid, Commit>,
    position: usize,
    pulls: usize,
}

impl VecSource {
    fn new(walk: &[Commit]) -> Self {
        Self {
            walk: walk.to_vec(),
            by_id: walk.iter().map(|c| (c.id, c.clone())).collect(),
            position: 0,
            pulls: 0,
        }
    }
}

impl CommitSource for VecSource {
    type Error = Infallible;

    fn next(&mut self, limit: usize, _stop: &dyn Fn() -> bool) -> Result<Pull, Infallible> {
        self.pulls += 1;
        let end = (self.position + limit).min(self.walk.len());
        let commits = self.walk[self.position..end].to_vec();
        self.position = end;
        Ok(Pull {
            commits,
            done: end == self.walk.len(),
            stopped: false,
        })
    }

    fn read(&mut self, ids: &[Oid]) -> Result<Vec<Commit>, Infallible> {
        Ok(ids.iter().map(|id| self.by_id[id].clone()).collect())
    }
}

fn never() -> bool {
    false
}

fn seeds(tip: u8) -> Seeds {
    Seeds {
        default_branch: Some(BranchSeed {
            tip: oid(tip),
            identity: BranchIdentity::of_name(b"main"),
        }),
        integration_branches: Vec::new(),
        known_branches: Vec::new(),
    }
}

fn ids(commits: &[Commit]) -> Vec<Oid> {
    commits.iter().map(|commit| commit.id).collect()
}

fn all_rows<S: CommitSource<Error = Infallible>>(layout: &mut StreamingLayout<S>) -> Vec<Row> {
    let advance = layout.advance(usize::MAX, &never).expect("in memory");
    assert!(advance.done);
    layout.rows(0..layout.rows_emitted()).expect("in memory")
}

/// main: 1 (merges topic) → 2 → 4 (root); topic: 3 → 5 → 4.
fn merged_topic() -> Vec<Commit> {
    vec![
        commit(1, &[2, 3], 50, b"Merge branch 'topic'"),
        commit(2, &[4], 40, b"main work"),
        commit(3, &[5], 30, b"topic two"),
        commit(5, &[4], 20, b"topic one"),
        commit(4, &[], 10, b"root"),
    ]
}

#[test]
fn a_folded_merge_hides_its_branch_down_to_the_fork_point() {
    let folded = Inclusion {
        folded: [oid(1)].into(),
        ..Inclusion::default()
    };
    let (shown, _) = apply(&merged_topic(), seeds(1), Follow::AllParents, &folded);
    assert_eq!(ids(&shown), vec![oid(1), oid(2), oid(4)]);
    assert_eq!(
        shown[0].parents,
        vec![oid(2)],
        "the merge draws first parent only"
    );

    let (all, _) = apply(
        &merged_topic(),
        seeds(1),
        Follow::AllParents,
        &Inclusion::default(),
    );
    assert_eq!(
        all,
        merged_topic(),
        "nothing hidden draws the walk as it is"
    );
}

#[test]
fn a_hidden_stash_takes_its_own_commits_with_it_but_not_its_base() {
    let walk = vec![
        commit(9, &[2, 8], 50, b"WIP on main: 2 work"),
        commit(1, &[2], 40, b"main tip"),
        commit(8, &[2], 30, b"index on main: 2 work"),
        commit(2, &[], 10, b"work"),
    ];
    let hidden = Inclusion {
        hidden_stashes: [oid(9)].into(),
        ..Inclusion::default()
    };
    let (shown, _) = apply(&walk, seeds(1), Follow::AllParents, &hidden);
    assert_eq!(ids(&shown), vec![oid(1), oid(2)]);
}

#[test]
fn first_parent_view_follows_the_default_branch_only() {
    let walk = vec![
        commit(6, &[3], 60, b"an unmerged branch tip"),
        commit(1, &[2, 3], 50, b"Merge branch 'topic'"),
        commit(2, &[4], 40, b"main work"),
        commit(3, &[5], 30, b"topic two"),
        commit(5, &[4], 20, b"topic one"),
        commit(4, &[], 10, b"root"),
    ];
    let (shown, seeds) = apply(&walk, seeds(1), Follow::FirstParent, &Inclusion::default());
    assert_eq!(ids(&shown), vec![oid(1), oid(2), oid(4)]);
    let rows = layout(&shown, seeds);
    assert!(rows.iter().all(|row| row.lane == 0 && row.lanes.len() == 1));
}

/// A 40-row main line with a topic merged at row 18 (its branch forking at
/// row 30), laid out in chunks of 4 with a checkpoint every 8.
fn long_history() -> (Vec<Commit>, Seeds) {
    let mut walk = Vec::new();
    let seconds = |n: u8| 1_000 - i64::from(n);
    for n in 1..=40u8 {
        let subject: &[u8] = if n == 19 {
            b"Merge branch 'topic'"
        } else {
            b"main"
        };
        let parents: Vec<u8> = match n {
            19 => vec![20, 101],
            40 => vec![],
            _ => vec![n + 1],
        };
        walk.push(commit(n, &parents, seconds(n) * 2, subject));
        if n == 19 {
            walk.push(commit(101, &[102], seconds(n) * 2 - 1, b"topic two"));
            walk.push(commit(102, &[31], seconds(n) * 2 - 1, b"topic one"));
        }
    }
    // Walk order by date: topic commits land just after the merge.
    (walk, seeds(1))
}

fn small() -> StreamConfig {
    StreamConfig {
        chunk_rows: 4,
        checkpoint_every: 8,
        max_checkpoints: 64,
        resident_chunks: 64,
        buffer_rows: 4,
    }
}

#[test]
fn folding_invalidates_only_the_rows_from_the_merge_down() {
    let (walk, seeds) = long_history();
    let source = Included::new(
        VecSource::new(&walk),
        seeds,
        Follow::AllParents,
        Inclusion::default(),
    );
    let view_seeds = source.seeds().clone();
    let mut layout = StreamingLayout::new(source, view_seeds, small());
    let original = all_rows(&mut layout);
    assert_eq!(original.len(), 42);
    let merge_row = layout.row_of(oid(19)).expect("the merge is laid out");
    assert_eq!(merge_row, 18);
    let checkpoints_before = layout.checkpoint_rows();
    let pulls_before = layout.source().inner().pulls;

    let folded = Inclusion {
        folded: [oid(19)].into(),
        ..Inclusion::default()
    };
    let invalidated = reinclude(&mut layout, folded).expect("in memory");

    // The invalidated set, exactly: chunks from the merge's (4, cut back to
    // rows 16..18) to the last (10, rows 40..42); checkpoints past row 18.
    assert_eq!(
        invalidated,
        Invalidated {
            from_row: 18,
            chunks: (4..=10).collect(),
            checkpoints: vec![24, 32, 40],
        }
    );
    assert_eq!(checkpoints_before, vec![8, 16, 24, 32, 40]);
    assert_eq!(
        layout.checkpoint_rows(),
        vec![8, 16],
        "checkpoints above the merge survive"
    );
    assert_eq!(
        layout.source().inner().pulls,
        pulls_before,
        "a reinclude re-reads; it never walks again"
    );
    for row in 0..18 {
        assert!(layout.is_resident(row), "row {row} above the merge is kept");
    }

    let refolded = all_rows(&mut layout);
    assert_eq!(layout.work().chunks_rebuilt, 0, "nothing kept was rebuilt");
    assert_eq!(&refolded[..18], &original[..18]);
    assert_eq!(
        refolded.len(),
        40,
        "the topic's two commits are folded away"
    );
    assert!(refolded.iter().all(|row| row.commit.id != oid(101)));

    reinclude(&mut layout, Inclusion::default()).expect("in memory");
    assert_eq!(
        all_rows(&mut layout),
        original,
        "unfolding restores every row"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(graft_fixtures::proptest_cases()))]

    /// Streaming a walk through `Included` lays out exactly what laying out
    /// the slice form's output does, whatever the chunking.
    #[test]
    fn streaming_a_view_matches_laying_out_its_slice(
        (history, follow, inclusion) in viewed(),
        chunk_rows in 1usize..=5,
    ) {
        let (commits, seeds) = apply(&history.commits, history.seeds(), follow, &inclusion);
        let expected = layout(&commits, seeds.clone());
        let source = Included::new(VecSource::new(&history.commits), history.seeds(), follow, inclusion);
        prop_assert_eq!(source.seeds(), &seeds);
        let config = StreamConfig { chunk_rows, checkpoint_every: chunk_rows * 2, ..small() };
        let mut streamed = StreamingLayout::new(source, seeds, config);
        prop_assert_eq!(all_rows(&mut streamed), expected);
    }

    /// Fold a merge, then unfold it: the rows are the original rows. While
    /// folded, the rows are what the slice form draws and every row above
    /// the merge is untouched.
    #[test]
    fn fold_then_unfold_returns_the_original_rows(
        (history, _, _) in viewed(),
        pick in any::<prop::sample::Index>(),
        chunk_rows in 1usize..=5,
    ) {
        let merges: Vec<Oid> = history
            .commits
            .iter()
            .filter(|commit| commit.parents.len() > 1)
            .map(|commit| commit.id)
            .collect();
        prop_assume!(!merges.is_empty());
        let merge = merges[pick.index(merges.len())];
        let config = StreamConfig { chunk_rows, checkpoint_every: chunk_rows * 2, ..small() };
        let source = Included::new(
            VecSource::new(&history.commits),
            history.seeds(),
            Follow::AllParents,
            Inclusion::default(),
        );
        let mut streamed = StreamingLayout::new(source, history.seeds(), config);
        let original = all_rows(&mut streamed);

        let folded = Inclusion { folded: [merge].into(), ..Inclusion::default() };
        let invalidated = reinclude(&mut streamed, folded.clone()).expect("in memory");
        let while_folded = all_rows(&mut streamed);
        let (commits, seeds) = apply(&history.commits, history.seeds(), Follow::AllParents, &folded);
        prop_assert_eq!(&while_folded, &layout(&commits, seeds));
        let kept = invalidated.from_row;
        prop_assert_eq!(&while_folded[..kept], &original[..kept]);

        reinclude(&mut streamed, Inclusion::default()).expect("in memory");
        prop_assert_eq!(all_rows(&mut streamed), original);
    }
}
