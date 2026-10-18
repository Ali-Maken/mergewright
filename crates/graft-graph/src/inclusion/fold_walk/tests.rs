use std::collections::BTreeSet;

use super::*;

pub(super) fn h(n: u32) -> CommitHandle {
    CommitHandle(n)
}

/// Admits `walk` (commit, parents) in order; the verdict per commit.
pub(super) fn walk_all(walk: &mut FoldWalk, history: &[(u32, &[u32])]) -> Vec<(u32, Admit)> {
    history
        .iter()
        .map(|&(commit, parents)| {
            let parents: Vec<CommitHandle> = parents.iter().copied().map(h).collect();
            (commit, walk.admit(h(commit), &parents))
        })
        .collect()
}

pub(super) fn shown(verdicts: &[(u32, Admit)]) -> BTreeSet<u32> {
    verdicts
        .iter()
        .filter(|(_, admit)| matches!(admit, Admit::Shown { .. }))
        .map(|&(commit, _)| commit)
        .collect()
}

/// main: 1 (merges topic) → 2 → 4 (root); topic: 3 → 5 → 4.
const MERGED_TOPIC: &[(u32, &[u32])] = &[(1, &[2, 3]), (2, &[4]), (3, &[5]), (5, &[4]), (4, &[])];

#[test]
fn a_merged_branch_is_hidden_behind_its_merge_down_to_the_fork_point() {
    let mut walk = FoldWalk::new(&[h(1)], &[], &[]);
    let verdicts = walk_all(&mut walk, MERGED_TOPIC);
    assert_eq!(shown(&verdicts), [1, 2, 4].into());
    assert_eq!(walk.behind(h(3)), Some(h(1)));
    assert_eq!(walk.behind(h(5)), Some(h(1)));
    assert_eq!(walk.behind(h(2)), None, "shown commits sit behind nothing");
    assert_eq!(walk.folds(h(1)), Some(true));
    assert!(walk.take_late().is_empty());
}

#[test]
fn the_merge_row_draws_no_edge_into_a_folded_branch() {
    let mut walk = FoldWalk::new(&[h(1)], &[], &[]);
    walk_all(&mut walk, MERGED_TOPIC);
    let link = Link {
        commit: h(1),
        parents: Parents::Two(h(2), h(3)),
    };
    assert_eq!(walk.trim(link).parents, Parents::One(h(2)));
    let plain = Link {
        commit: h(2),
        parents: Parents::One(h(4)),
    };
    assert_eq!(walk.trim(plain.clone()), plain);
}

#[test]
fn an_opened_merge_shows_its_side_inside_it() {
    let mut walk = FoldWalk::new(&[h(1)], &[h(1)], &[]);
    let verdicts = walk_all(&mut walk, MERGED_TOPIC);
    assert_eq!(shown(&verdicts), [1, 2, 3, 4, 5].into());
    assert_eq!(walk.inside(h(3)), Some(h(1)));
    assert_eq!(walk.inside(h(5)), Some(h(1)));
    assert_eq!(walk.inside(h(4)), None, "the fork point is main's");
    assert_eq!(walk.inside(h(2)), None);
    assert_eq!(walk.folds(h(1)), Some(true), "an opened merge still folds");
}

#[test]
fn a_sync_merge_folds_nothing() {
    // feature: 7 merges main's 4 into 6; main: 1 → 2 → 4 → 5.
    // feature is an unmerged tip; main is kept.
    let history: &[(u32, &[u32])] = &[
        (1, &[2]),
        (2, &[4]),
        (7, &[6, 4]),
        (6, &[5]),
        (4, &[5]),
        (5, &[]),
    ];
    let mut walk = FoldWalk::new(&[h(1)], &[], &[]);
    let verdicts = walk_all(&mut walk, history);
    assert_eq!(shown(&verdicts), [1, 2, 4, 5, 6, 7].into());
    assert_eq!(walk.folds(h(7)), Some(false));
    let link = Link {
        commit: h(7),
        parents: Parents::Two(h(6), h(4)),
    };
    assert_eq!(walk.trim(link.clone()), link, "its edge into main is drawn");
}

#[test]
fn folds_is_unknown_until_the_second_parent_is_admitted() {
    let mut walk = FoldWalk::new(&[h(1)], &[], &[]);
    walk.admit(h(1), &[h(2), h(3)]);
    assert_eq!(walk.folds(h(1)), None);
    walk.admit(h(2), &[h(4)]);
    assert_eq!(walk.folds(h(1)), None);
    walk.admit(h(3), &[h(5)]);
    assert_eq!(walk.folds(h(1)), Some(true));
}

#[test]
fn a_sync_merge_admitted_before_mains_line_reaches_its_parent_is_rejoined() {
    // main: 1 → 2 → 4; feature 7 merges 4 into 6 and is walked before 2
    // (dated after it): at 7, nothing has said 4 is shown yet.
    let history: &[(u32, &[u32])] = &[(1, &[2]), (7, &[6, 4]), (2, &[4]), (6, &[4]), (4, &[])];
    let mut walk = FoldWalk::new(&[h(1)], &[], &[]);
    let verdicts = walk_all(&mut walk, history);
    assert_eq!(shown(&verdicts), [1, 2, 4, 6, 7].into());
    assert_eq!(walk.take_rejoined(), vec![(h(7), h(4))]);
    assert_eq!(walk.folds(h(7)), Some(false));
    let link = Link {
        commit: h(7),
        parents: Parents::Two(h(6), h(4)),
    };
    assert_eq!(walk.trim(link.clone()).parents, Parents::One(h(6)));
    walk.rejoin(h(7), h(4));
    assert_eq!(walk.trim(link.clone()), link, "rejoined: the edge is drawn");
    assert_eq!(walk.rejoins(), &[(h(7), h(4))]);
}

#[test]
fn a_parent_walked_before_its_shown_child_is_reported_late() {
    // Date order with a backward clock: main's 2 is walked before 1, its
    // child, after a hidden branch commit (3) already pointed at it.
    let history: &[(u32, &[u32])] = &[(9, &[8, 3]), (8, &[1]), (3, &[2]), (2, &[]), (1, &[2])];
    // 9 is main's tip; 1 is on main's first-parent line below 8.
    let mut walk = FoldWalk::new(&[h(9)], &[], &[]);
    let verdicts = walk_all(&mut walk, history);
    assert!(
        matches!(verdicts[3].1, Admit::Hidden { .. }),
        "2 hid too early"
    );
    assert_eq!(walk.take_late(), vec![h(2)]);
    assert!(walk.take_late().is_empty(), "drains");

    let mut forced = FoldWalk::new(&[h(9)], &[], &[h(2)]);
    let verdicts = walk_all(&mut forced, history);
    assert!(shown(&verdicts).contains(&2));
    assert!(forced.take_late().is_empty());
}

#[test]
fn a_mark_after_new_counts_like_one_given_to_new() {
    let mut walk = FoldWalk::new(&[], &[], &[]);
    walk.mark(h(1), Mark::Keep);
    walk.mark(h(1), Mark::Opened);
    let verdicts = walk_all(&mut walk, MERGED_TOPIC);
    assert_eq!(shown(&verdicts), [1, 2, 3, 4, 5].into());
}

pub(super) mod properties {
    //! The fold over generated histories, checked against a reference
    //! worked out from the whole graph at once (no order involved).

    use std::collections::{BTreeMap, BTreeSet};

    use graft_core::oid::Oid;
    use proptest::prelude::*;

    use super::super::*;
    use crate::layout::tests::properties::{History, history};

    /// A generated history as positions: parents by position, all later.
    struct Graph {
        parents: Vec<Vec<usize>>,
        keep: Vec<usize>,
        opened: BTreeSet<usize>,
    }

    impl Graph {
        fn of(history: &History, opened_mask: &[bool]) -> Self {
            let at: BTreeMap<Oid, usize> = history
                .commits
                .iter()
                .enumerate()
                .map(|(position, commit)| (commit.id, position))
                .collect();
            let parents: Vec<Vec<usize>> = history
                .commits
                .iter()
                .map(|commit| commit.parents.iter().map(|id| at[id]).collect())
                .collect();
            let seeds = history.seeds();
            let keep = seeds
                .default_branch
                .iter()
                .chain(&seeds.integration_branches)
                .map(|seed| at[&seed.tip])
                .collect();
            let opened = parents
                .iter()
                .enumerate()
                .zip(opened_mask.iter().cycle())
                .filter(|((_, parents), on)| **on && parents.len() > 1)
                .map(|((position, _), _)| position)
                .collect();
            Graph {
                parents,
                keep,
                opened,
            }
        }

        /// The exact shown set: the first-parent closure of the keep tips,
        /// every commit nothing points at, and the opened merges' second
        /// parents — opened merges counting only if shown themselves.
        fn exact(&self, opened: &BTreeSet<usize>) -> BTreeSet<usize> {
            let mut pointed = vec![false; self.parents.len()];
            for parents in &self.parents {
                for &parent in parents {
                    pointed[parent] = true;
                }
            }
            let mut work: Vec<usize> = self.keep.clone();
            work.extend((0..self.parents.len()).filter(|&position| !pointed[position]));
            let mut shown = BTreeSet::new();
            while let Some(commit) = work.pop() {
                if !shown.insert(commit) {
                    continue;
                }
                let parents = &self.parents[commit];
                let reach = if opened.contains(&commit) {
                    parents.len()
                } else {
                    1
                };
                work.extend(parents.iter().take(reach));
            }
            shown
        }

        fn handles(positions: impl IntoIterator<Item = usize>) -> Vec<CommitHandle> {
            positions.into_iter().map(handle).collect()
        }

        fn walk(
            &self,
            order: &[usize],
            forced: &BTreeSet<usize>,
        ) -> (FoldWalk, BTreeMap<usize, Admit>) {
            let mut walk = FoldWalk::new(
                &Self::handles(self.keep.iter().copied()),
                &Self::handles(self.opened.iter().copied()),
                &Self::handles(forced.iter().copied()),
            );
            let verdicts = order
                .iter()
                .map(|&position| {
                    let parents = Self::handles(self.parents[position].iter().copied());
                    (position, walk.admit(handle(position), &parents))
                })
                .collect();
            (walk, verdicts)
        }
    }

    fn handle(position: usize) -> CommitHandle {
        CommitHandle(u32::try_from(position).expect("small"))
    }

    fn shown_of(verdicts: &BTreeMap<usize, Admit>) -> BTreeSet<usize> {
        verdicts
            .iter()
            .filter(|(_, admit)| matches!(admit, Admit::Shown { .. }))
            .map(|(&position, _)| position)
            .collect()
    }

    /// The walk order with each commit pushed up to three places later:
    /// some parents now come before their children, as a backward clock
    /// leaves the provisional date order.
    pub(in crate::inclusion::fold_walk) fn skewed(len: usize, noise: &[u8]) -> Vec<usize> {
        let mut order: Vec<usize> = (0..len).collect();
        order.sort_by_key(|&position| position + usize::from(noise[position % noise.len()] % 4));
        order
    }

    fn graphs() -> impl Strategy<Value = (History, Vec<bool>)> {
        (
            history(),
            proptest::collection::vec(proptest::bool::weighted(0.3), 32),
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(graft_fixtures::proptest_cases()))]

        /// Topological order: the walk shows exactly the reference set, hides
        /// every other commit behind a chain ending at a shown, unopened
        /// merge, and — its rejoins drawn — draws an edge exactly where both
        /// ends are shown.
        #[test]
        fn in_topological_order_the_fold_is_exact((history, mask) in graphs()) {
            let graph = Graph::of(&history, &mask);
            let order: Vec<usize> = (0..graph.parents.len()).collect();
            let (mut walk, verdicts) = graph.walk(&order, &BTreeSet::new());
            let exact = graph.exact(&graph.opened);
            prop_assert_eq!(&shown_of(&verdicts), &exact);
            prop_assert!(walk.take_late().is_empty(), "nothing is late in topological order");

            for (&position, admit) in &verdicts {
                if let Admit::Hidden { behind } = *admit {
                    prop_assert_eq!(walk.behind(handle(position)), Some(behind));
                    let mut at = behind.index();
                    let mut steps = 0;
                    while !exact.contains(&at) {
                        prop_assert!(graph.parents[at].len() > 1, "a chain runs through merges");
                        at = walk.behind(handle(at)).expect("a hidden merge sits behind one").index();
                        steps += 1;
                        prop_assert!(steps <= graph.parents.len(), "the chain ends");
                    }
                    prop_assert!(graph.parents[at].len() > 1, "it ends at a merge");
                    prop_assert!(!graph.opened.contains(&at), "an unopened one");
                }
            }

            let unopened = graph.exact(&BTreeSet::new());
            for &position in &exact {
                match walk.inside(handle(position)) {
                    None => prop_assert!(unopened.contains(&position), "shown for itself"),
                    Some(merge) => {
                        prop_assert!(graph.opened.contains(&merge.index()));
                        prop_assert!(exact.contains(&merge.index()));
                        prop_assert!(!unopened.contains(&position), "shown only by opening");
                    }
                }
            }

            for (merge, parent) in walk.take_rejoined() {
                walk.rejoin(merge, parent);
            }
            for &position in &exact {
                let parents = &graph.parents[position];
                let link = Link {
                    commit: handle(position),
                    parents: Graph::handles(parents.iter().copied()).into_iter().collect(),
                };
                let drawn: Vec<usize> =
                    walk.trim(link).parents.iter().map(CommitHandle::index).collect();
                let both_shown: Vec<usize> =
                    parents.iter().copied().filter(|parent| exact.contains(parent)).collect();
                prop_assert_eq!(drawn, both_shown, "row {}", position);

                if parents.len() > 1 {
                    let seconds = &parents[1..];
                    let folds = walk.folds(handle(position));
                    if graph.opened.contains(&position) {
                        let without: BTreeSet<usize> =
                            graph.opened.iter().copied().filter(|&m| m != position).collect();
                        let alone = graph.exact(&without);
                        if seconds.iter().any(|p| !alone.contains(p)) {
                            prop_assert_eq!(folds, Some(true));
                        }
                        if seconds.iter().all(|p| unopened.contains(p)) {
                            prop_assert_eq!(folds, Some(false));
                        }
                    } else {
                        let hides = seconds.iter().any(|p| !exact.contains(p));
                        prop_assert_eq!(folds, Some(hides));
                    }
                }
            }
        }

        /// Date-skewed order: forcing what `take_late` reports until nothing
        /// is late shows everything the exact fold shows (extra rows, never
        /// missing ones); walked again in topological order (promotion,
        /// nothing forced) it is exact.
        #[test]
        fn in_date_skewed_order_late_repair_never_hides_a_shown_commit(
            (history, mask) in graphs(),
            noise in proptest::collection::vec(any::<u8>(), 32),
        ) {
            let graph = Graph::of(&history, &mask);
            let len = graph.parents.len();
            let order = skewed(len, &noise);
            let exact = graph.exact(&graph.opened);

            let mut forced = BTreeSet::new();
            let mut rounds = 0;
            let shown = loop {
                let (mut walk, verdicts) = graph.walk(&order, &forced);
                let late = walk.take_late();
                if late.is_empty() {
                    break shown_of(&verdicts);
                }
                let before = forced.len();
                forced.extend(late.iter().map(|commit| commit.index()));
                prop_assert!(forced.len() > before, "each round forces something new");
                rounds += 1;
                prop_assert!(rounds <= len, "repair ends");
            };
            prop_assert!(
                shown.is_superset(&exact),
                "missing: {:?}",
                exact.difference(&shown).collect::<Vec<_>>()
            );
            for &tip in &graph.keep {
                let mut at = Some(tip);
                while let Some(commit) = at {
                    prop_assert!(shown.contains(&commit), "keep tip's first-parent line");
                    at = graph.parents[commit].first().copied();
                }
            }

            let topological: Vec<usize> = (0..len).collect();
            let (_, promoted) = graph.walk(&topological, &BTreeSet::new());
            prop_assert_eq!(shown_of(&promoted), exact);
        }
    }
}
