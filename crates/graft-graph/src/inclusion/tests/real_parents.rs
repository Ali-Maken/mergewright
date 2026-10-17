//! No git write may see a folded view (ease-pass 06 D1): a row's
//! `commit.parents` is the commit's real parents under every inclusion,
//! and only `drawn_parents` carries what the view trims.

use super::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(graft_fixtures::proptest_cases()))]

    /// Under every inclusion `viewed()` generates, streamed through
    /// `Included`: every row's `commit.parents` is the source commit's
    /// parents, and its `drawn_parents` is what the slice form draws.
    #[test]
    fn every_row_keeps_its_real_parents_under_every_inclusion(
        (history, follow, inclusion) in viewed(),
        chunk_rows in 1usize..=5,
    ) {
        let real: HashMap<Oid, Vec<Oid>> = history
            .commits
            .iter()
            .map(|commit| (commit.id, commit.parents.clone()))
            .collect();
        let (commits, seeds) = apply(&history.commits, history.seeds(), follow, &inclusion);
        let drawn: HashMap<Oid, Vec<Oid>> = layout(&commits, seeds.clone())
            .into_iter()
            .map(|row| (row.commit.id, row.drawn_parents))
            .collect();
        let config = StreamConfig { chunk_rows, checkpoint_every: chunk_rows * 2, ..small() };
        let source = Included::new(VecSource::new(&history.commits), history.seeds(), follow, inclusion);
        let mut streamed = StreamingLayout::new(source, seeds, config);
        let advance = advance_folded(&mut streamed, usize::MAX, &never).expect("in memory");
        prop_assert!(advance.done);
        let rows = streamed.rows(0..streamed.rows_emitted()).expect("in memory");
        prop_assert_eq!(rows.len(), drawn.len());
        for row in &rows {
            prop_assert_eq!(&row.commit.parents, &real[&row.commit.id], "real parents of {}", row.commit.id);
            prop_assert_eq!(&row.drawn_parents, &drawn[&row.commit.id], "drawn parents of {}", row.commit.id);
            prop_assert_eq!(row.is_merge(), real[&row.commit.id].len() >= 2);
        }
    }
}

/// A folded merge, and a merge in the first-parent view, are still merges
/// to `is_merge`; each draws one parent.
#[test]
fn a_folded_merge_and_a_first_parent_merge_are_merges() {
    let walk = merged_topic();
    let folded =
        Inclusion::folded_by_default(Vec::new(), crate::inclusion::OpenedMerges::default());
    for (follow, inclusion) in [
        (Follow::AllParents, folded),
        (Follow::FirstParent, Inclusion::default()),
    ] {
        let source = Included::new(VecSource::new(&walk), seeds(1), follow, inclusion);
        let view_seeds = source.seeds().clone();
        let mut streamed = StreamingLayout::new(source, view_seeds, small());
        let rows = all_rows_as_read(&mut streamed);
        let merge = rows
            .iter()
            .find(|row| row.commit.id == oid(1))
            .expect("the merge is shown");
        assert_eq!(merge.commit.parents, vec![oid(2), oid(3)], "{follow:?}");
        assert_eq!(merge.drawn_parents, vec![oid(2)], "{follow:?}");
        assert!(merge.is_merge(), "{follow:?}");
    }
}
