//! Merged branches folded by default (ease-pass 06): through `apply`,
//! `Included` and `reinclude`. `FoldWalk`'s own rule is tested beside it.

use super::*;

fn folded() -> Inclusion {
    Inclusion::folded_by_default(Vec::new(), OpenedMerges::default())
}

fn opened(merges: &[Oid]) -> Inclusion {
    let mut inclusion = folded();
    let opened = inclusion.opened_mut().expect("folding by default");
    for &merge in merges {
        opened.open(merge);
    }
    inclusion
}

fn ids_of(rows: &[Row]) -> Vec<Oid> {
    rows.iter().map(|row| row.commit.id).collect()
}

#[test]
fn a_merged_branch_folds_into_its_merge_down_to_the_fork_point() {
    let (shown, _) = apply(&merged_topic(), seeds(1), Follow::AllParents, &folded());
    assert_eq!(ids(&shown), vec![oid(1), oid(2), oid(4)]);
    assert_eq!(
        shown[0].parents,
        vec![oid(2)],
        "the merge draws first parent only"
    );

    let (open, _) = apply(
        &merged_topic(),
        seeds(1),
        Follow::AllParents,
        &opened(&[oid(1)]),
    );
    assert_eq!(open, merged_topic(), "opened, the merge shows its side");
}

#[test]
fn a_folding_merge_streams_its_first_parent_only() {
    let (walk, seeds) = long_history();
    let mut source = Included::new(VecSource::new(&walk), seeds, Follow::AllParents, folded());
    while !source.pull(usize::MAX, &never).expect("in memory").done {}
    let handle = source.handle_of(&oid(19)).expect("the merge has a handle");
    let links = source.links(&[handle]).expect("in memory");
    assert_eq!(
        links[0].parents.len(),
        1,
        "a folded merge keeps its first parent"
    );
    let fold = source.fold().expect("folding by default");
    assert_eq!(fold.folds(handle), Some(true));
    let topic = source.handle_of(&oid(101)).expect("walked");
    assert_eq!(
        fold.behind(topic),
        Some(handle),
        "the topic hides behind 19"
    );
}

#[test]
fn opening_a_merge_invalidates_only_the_rows_from_the_merge_down() {
    let (walk, seeds) = long_history();
    let source = Included::new(VecSource::new(&walk), seeds, Follow::AllParents, folded());
    let view_seeds = source.seeds().clone();
    let mut layout = StreamingLayout::new(source, view_seeds, small());
    // The whole history on screen: every chunk is materialised as it is laid
    // out, so any rebuild below is one the reinclude caused.
    layout.set_viewport(0..42);
    let original = all_rows(&mut layout);
    assert_eq!(original.len(), 40, "the topic's two commits fold away");
    assert!(original.iter().all(|row| row.commit.id != oid(101)));
    let merge_row = layout.row_of(oid(19)).expect("the merge is laid out");
    assert_eq!(merge_row, 18);
    let checkpoints_before = layout.checkpoint_rows();
    let pulls_before = layout.source().inner().pulls;

    let invalidated = reinclude(&mut layout, opened(&[oid(19)])).expect("in memory");

    // The invalidated set, exactly: chunks from the merge's (4, cut back to
    // rows 16..18) to the last (9, rows 36..40); checkpoints past row 18.
    assert_eq!(
        invalidated,
        Invalidated {
            from_row: 18,
            chunks: (4..=9).collect(),
            checkpoints: vec![24, 32],
        }
    );
    assert_eq!(checkpoints_before, vec![8, 16, 24, 32]);
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

    let open = all_rows(&mut layout);
    assert_eq!(layout.work().chunks_rebuilt, 0, "nothing kept was rebuilt");
    assert_eq!(&open[..18], &original[..18]);
    assert_eq!(
        ids_of(&open[18..21]),
        [oid(19), oid(101), oid(102)],
        "exactly the merge's side appears, under it"
    );
    assert_eq!(open.len(), 42);

    reinclude(&mut layout, folded()).expect("in memory");
    assert_eq!(
        all_rows(&mut layout),
        original,
        "closing restores every row"
    );
}

/// `commits` with each pushed up to three places later: some parents now
/// come before their children, as a backward clock leaves the provisional
/// date order.
fn date_skewed(commits: &[Commit], noise: &[u8]) -> Vec<Commit> {
    let mut order: Vec<(usize, usize)> = (0..commits.len())
        .map(|index| (index + usize::from(noise[index % noise.len()] % 4), index))
        .collect();
    order.sort_unstable();
    order
        .into_iter()
        .map(|(_, index)| commits[index].clone())
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(graft_fixtures::proptest_cases()))]

    /// Streamed in a date-skewed order, late repair leaves every commit the
    /// exact (topological) fold shows still shown — extra rows, never
    /// missing ones — and the rows are what the slice form draws for that
    /// order.
    #[test]
    fn a_date_skewed_walk_streams_every_commit_the_exact_fold_shows(
        (history, _, _) in viewed(),
        noise in proptest::collection::vec(any::<u8>(), 1..40),
        chunk_rows in 1usize..=5,
    ) {
        let skewed = date_skewed(&history.commits, &noise);
        let (exact, _) = apply(&history.commits, history.seeds(), Follow::AllParents, &folded());
        let config = StreamConfig { chunk_rows, checkpoint_every: chunk_rows * 2, ..small() };
        let source = Included::new(
            VecSource::new(&skewed),
            history.seeds(),
            Follow::AllParents,
            folded(),
        );
        let view_seeds = source.seeds().clone();
        let mut streamed = StreamingLayout::new(source, view_seeds, config);
        let rows = all_rows(&mut streamed);
        let shown: BTreeSet<Oid> = ids_of(&rows).into_iter().collect();
        for commit in &exact {
            prop_assert!(shown.contains(&commit.id), "{} hidden", commit.id);
        }
        let (commits, seeds) = apply(&skewed, history.seeds(), Follow::AllParents, &folded());
        prop_assert_eq!(rows, layout(&commits, seeds));
    }

    /// Folded by default, open a merge, then close it: the rows are the
    /// original rows. While open, the rows are what the slice form draws
    /// and no row above the merge comes or goes.
    #[test]
    fn open_then_close_returns_the_original_rows(
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
            folded(),
        );
        let mut streamed = StreamingLayout::new(source, history.seeds(), config);
        let original = all_rows(&mut streamed);
        let (commits, seeds) = apply(&history.commits, history.seeds(), Follow::AllParents, &folded());
        prop_assert_eq!(&original, &layout(&commits, seeds));

        let open = opened(&[merge]);
        let invalidated = reinclude(&mut streamed, open.clone()).expect("in memory");
        let while_open = all_rows(&mut streamed);
        let (commits, seeds) = apply(&history.commits, history.seeds(), Follow::AllParents, &open);
        prop_assert_eq!(&while_open, &layout(&commits, seeds));
        // The merge's side lies below it, so no row above it comes or goes.
        // A row above may still redraw: an unopened merge whose second
        // parent the opening just showed draws that edge again (a rejoin),
        // which `advance_folded` repairs — hence ids, not whole rows.
        let kept = invalidated.from_row.min(while_open.len());
        prop_assert_eq!(ids_of(&while_open[..kept]), ids_of(&original[..kept]));

        reinclude(&mut streamed, folded()).expect("in memory");
        prop_assert_eq!(all_rows(&mut streamed), original);
    }
}
