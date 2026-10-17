use std::collections::{BTreeSet, HashMap};
use std::convert::Infallible;

use graft_core::commit_handle::{CommitHandle, Link};
use proptest::prelude::*;

use super::*;
use crate::identity::BranchIdentity;
use crate::layout::layout;
use crate::layout::source::{CommitSource, Pull};
use crate::layout::stream::{Invalidated, StreamConfig, StreamingLayout};
use crate::layout::tests::properties::viewed;
use crate::layout::tests::{Handles, commit, oid};
use crate::row::Row;
use crate::seeds::BranchSeed;

mod fold_by_default;
mod real_parents;

/// A walk held in memory, readable by id.
struct VecSource {
    walk: Vec<Commit>,
    by_id: HashMap<Oid, Commit>,
    position: usize,
    pulls: usize,
    handles: Handles,
}

impl VecSource {
    fn new(walk: &[Commit]) -> Self {
        Self {
            walk: walk.to_vec(),
            by_id: walk.iter().map(|c| (c.id, c.clone())).collect(),
            position: 0,
            pulls: 0,
            handles: Handles::of(walk),
        }
    }
}

impl CommitSource for VecSource {
    type Error = Infallible;

    fn pull(&mut self, limit: usize, _stop: &dyn Fn() -> bool) -> Result<Pull, Infallible> {
        self.pulls += 1;
        let end = (self.position + limit).min(self.walk.len());
        let handles: Vec<CommitHandle> = self.walk[self.position..end]
            .iter()
            .map(|commit| {
                self.handles
                    .handle_of(&commit.id)
                    .expect("walked commits are interned")
            })
            .collect();
        self.position = end;
        Ok(Pull {
            links: self.handles.links(&self.by_id, &handles),
            done: end == self.walk.len(),
            stopped: false,
        })
    }

    fn read(&mut self, commits: &[CommitHandle]) -> Result<Vec<Commit>, Infallible> {
        Ok(commits
            .iter()
            .map(|&handle| self.by_id[&self.handles.id_of(handle)].clone())
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

/// Lays out to the end, repairs included, and reads every row back as
/// drawn: `commit.parents` set to `drawn_parents`, which is what a row of
/// the slice form (laid out from trimmed commits) holds in both.
fn all_rows(layout: &mut StreamingLayout<Included<VecSource>>) -> Vec<Row> {
    all_rows_as_read(layout)
        .into_iter()
        .map(|mut row| {
            row.commit.parents.clone_from(&row.drawn_parents);
            row
        })
        .collect()
}

/// [`all_rows`] with each commit as read: every real parent.
fn all_rows_as_read(layout: &mut StreamingLayout<Included<VecSource>>) -> Vec<Row> {
    let advance = advance_folded(layout, usize::MAX, &never).expect("in memory");
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
fn links_trim_parents_as_the_view_draws_them() {
    let (walk, seeds) = long_history();
    let merge = walk
        .iter()
        .find(|commit| commit.parents.len() > 1)
        .expect("the history has a merge")
        .id;
    let untrimmed = |source: &mut Included<VecSource>| {
        let handle = source.handle_of(&merge).expect("the merge has a handle");
        let links = source.links(&[handle]).expect("infallible");
        assert_eq!(links[0].commit, handle);
        assert_eq!(source.id_of(handle), merge);
        links[0].parents.len()
    };

    let mut all = Included::new(
        VecSource::new(&walk),
        seeds.clone(),
        Follow::AllParents,
        Inclusion::default(),
    );
    assert!(untrimmed(&mut all) > 1, "the full view keeps every parent");

    let mut first = Included::new(
        VecSource::new(&walk),
        seeds,
        Follow::FirstParent,
        Inclusion::default(),
    );
    assert_eq!(
        untrimmed(&mut first),
        1,
        "first-parent view keeps the first"
    );
}

fn stashes(ids: &[u8]) -> Inclusion {
    Inclusion::default().with_stashes(ids.iter().map(|&id| oid(id)).collect())
}

#[test]
fn a_stash_keeps_only_its_first_parent() {
    // A stash with staged changes: 9 = WIP (base 2, index 8).
    let walk = vec![
        commit(9, &[2, 8], 50, b"WIP on main: 2 work"),
        commit(1, &[2], 40, b"main tip"),
        commit(8, &[2], 30, b"index on main: 2 work"),
        commit(2, &[], 10, b"work"),
    ];
    let (shown, _) = apply(&walk, seeds(1), Follow::AllParents, &stashes(&[9]));
    assert_eq!(ids(&shown), vec![oid(9), oid(1), oid(2)]);
    assert_eq!(
        shown[0].parents,
        vec![oid(2)],
        "the stash draws its base only"
    );
}

#[test]
fn a_stash_with_untracked_drops_both_extra_parents() {
    // A stash with untracked files: 9 = WIP (base 2, index 8, untracked root 7).
    let walk = vec![
        commit(9, &[2, 8, 7], 50, b"WIP on main: 2 work"),
        commit(1, &[2], 40, b"main tip"),
        commit(8, &[2], 30, b"index on main: 2 work"),
        commit(7, &[], 30, b"untracked files on main: 2 work"),
        commit(2, &[], 10, b"work"),
    ];
    let (shown, view_seeds) = apply(&walk, seeds(1), Follow::AllParents, &stashes(&[9]));
    assert_eq!(ids(&shown), vec![oid(9), oid(1), oid(2)]);
    // No lane left open for 8 or 7: the rows are exactly those of a history
    // where the stash never had them.
    let clean = vec![
        commit(9, &[2], 50, b"WIP on main: 2 work"),
        commit(1, &[2], 40, b"main tip"),
        commit(2, &[], 10, b"work"),
    ];
    let rows = layout(&shown, view_seeds.clone());
    assert_eq!(rows, layout(&clean, view_seeds));
    assert_eq!(rows[1].commit.id, oid(1));
    assert!(
        rows[0]
            .outgoing
            .iter()
            .all(|&(_, target)| target != oid(8) && target != oid(7)),
        "no lane opens from the stash towards index or untracked"
    );
}

/// A stash whose base (5) is on no branch any more: rebased away.
fn stash_on_a_rebased_away_base() -> Vec<Commit> {
    vec![
        commit(9, &[5, 8], 50, b"WIP on old: 5 base"),
        commit(1, &[2], 40, b"main tip"),
        commit(8, &[5], 30, b"index on old: 5 base"),
        commit(5, &[2], 20, b"base, rebased away"),
        commit(2, &[], 10, b"root"),
    ]
}

#[test]
fn a_rebased_away_base_is_still_shown() {
    let (shown, _) = apply(
        &stash_on_a_rebased_away_base(),
        seeds(1),
        Follow::AllParents,
        &stashes(&[9]),
    );
    assert_eq!(ids(&shown), vec![oid(9), oid(1), oid(5), oid(2)]);
}

#[test]
fn hiding_a_stash_still_hides_its_first_parent_line() {
    let walk = stash_on_a_rebased_away_base();
    let hidden = Inclusion {
        hidden_stashes: [oid(9)].into(),
        ..Inclusion::default()
    };
    let both = hidden.clone().with_stashes([oid(9)].into());
    let (today, _) = apply(&walk, seeds(1), Follow::AllParents, &hidden);
    let (shown, _) = apply(&walk, seeds(1), Follow::AllParents, &both);
    assert_eq!(ids(&today), vec![oid(1), oid(2)]);
    assert_eq!(shown, today, "stash set + hidden = today's `H`");
}

#[test]
fn changing_the_stash_set_reincludes_from_the_first_changed_row() {
    let (mut walk, seeds) = long_history();
    // A stash with untracked files on commit 11, walked just after 10: row 10.
    let at = walk
        .iter()
        .position(|c| c.id == oid(10))
        .expect("10 is walked")
        + 1;
    let stash = [
        commit(200, &[11, 201, 202], 1_979, b"WIP on main: 11 main"),
        commit(201, &[11], 1_979, b"index on main: 11 main"),
        commit(202, &[], 1_979, b"untracked files on main: 11 main"),
    ];
    walk.splice(at..at, stash);
    let source = Included::new(
        VecSource::new(&walk),
        seeds.clone(),
        Follow::AllParents,
        Inclusion::default(),
    );
    let view_seeds = source.seeds().clone();
    let mut streamed = StreamingLayout::new(source, view_seeds, small());
    streamed.set_viewport(0..64);
    let original = all_rows(&mut streamed);
    assert_eq!(streamed.row_of(oid(200)), Some(10));

    let invalidated = reinclude(&mut streamed, stashes(&[200])).expect("in memory");
    assert_eq!(invalidated.from_row, 10, "replayed from the stash's row");
    let with_stash = all_rows(&mut streamed);
    assert_eq!(&with_stash[..10], &original[..10]);
    let (commits, slice_seeds) = apply(&walk, seeds, Follow::AllParents, &stashes(&[200]));
    assert_eq!(with_stash, layout(&commits, slice_seeds));
    assert!(
        with_stash
            .iter()
            .all(|row| row.commit.id != oid(201) && row.commit.id != oid(202))
    );

    reinclude(&mut streamed, Inclusion::default()).expect("in memory");
    assert_eq!(
        all_rows(&mut streamed),
        original,
        "clearing the set restores every row"
    );
}

#[test]
fn an_empty_stash_set_changes_nothing() {
    let walk = stash_on_a_rebased_away_base();
    let (shown, _) = apply(&walk, seeds(1), Follow::AllParents, &stashes(&[]));
    assert_eq!(shown, walk);
    assert_eq!(Inclusion::default().stashes(), &BTreeSet::new());
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

    /// Mark any commits as stashes, then clear them: while marked, the rows
    /// are what the slice form draws and every row above the first changed
    /// one is untouched; cleared, they are the original rows.
    #[test]
    fn marking_stashes_then_clearing_returns_the_original_rows(
        (history, follow, _) in viewed(),
        mask in proptest::collection::vec(proptest::bool::weighted(0.3), 40),
        chunk_rows in 1usize..=5,
    ) {
        let marked: BTreeSet<Oid> = history
            .commits
            .iter()
            .zip(mask.iter().cycle())
            .filter(|(_, on)| **on)
            .map(|(commit, _)| commit.id)
            .collect();
        let config = StreamConfig { chunk_rows, checkpoint_every: chunk_rows * 2, ..small() };
        let source = Included::new(
            VecSource::new(&history.commits),
            history.seeds(),
            follow,
            Inclusion::default(),
        );
        let view_seeds = source.seeds().clone();
        let mut streamed = StreamingLayout::new(source, view_seeds, config);
        let original = all_rows(&mut streamed);

        let with_stashes = Inclusion::default().with_stashes(marked);
        let invalidated = reinclude(&mut streamed, with_stashes.clone()).expect("in memory");
        let while_marked = all_rows(&mut streamed);
        let (commits, seeds) = apply(&history.commits, history.seeds(), follow, &with_stashes);
        prop_assert_eq!(&while_marked, &layout(&commits, seeds));
        let kept = invalidated.from_row.min(while_marked.len());
        prop_assert_eq!(&while_marked[..kept], &original[..kept]);

        reinclude(&mut streamed, Inclusion::default()).expect("in memory");
        prop_assert_eq!(all_rows(&mut streamed), original);
    }
}
