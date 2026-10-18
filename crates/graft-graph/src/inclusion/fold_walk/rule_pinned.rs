//! The fold rule pinned on his team's dependent branches (ease-pass 06g):
//! a ref tip that a shown merge reaches through a second parent folds,
//! even when that merge is on an unmerged branch — "merged = merged into
//! any shown line" (design note, Hard-to-reverse choice 2, settled
//! 2026-10-18). Keep tips never fold.
//!
//! Commits are positions in topological order (children first). Each case
//! is checked in that order, then walked date-skewed with late repair and
//! promoted: the promoted walk shows exactly the topological answer.

use std::collections::BTreeSet;

use super::tests::properties::skewed;
use super::tests::{h, shown, walk_all};
use super::*;

/// One fixture history: per position, its parents; and the keep tips.
struct Case {
    parents: &'static [&'static [u32]],
    keep: &'static [u32],
}

impl Case {
    fn walk(&self, order: &[u32], forced: &BTreeSet<u32>) -> (FoldWalk, BTreeSet<u32>) {
        let keep: Vec<CommitHandle> = self.keep.iter().copied().map(h).collect();
        let forced: Vec<CommitHandle> = forced.iter().copied().map(h).collect();
        let mut walk = FoldWalk::new(&keep, &[], &forced);
        let history: Vec<(u32, &[u32])> = order
            .iter()
            .map(|&at| (at, self.parents[at as usize]))
            .collect();
        let verdicts = walk_all(&mut walk, &history);
        (walk, shown(&verdicts))
    }

    fn topological(&self) -> (FoldWalk, BTreeSet<u32>) {
        let len = u32::try_from(self.parents.len()).expect("small");
        let order: Vec<u32> = (0..len).collect();
        self.walk(&order, &BTreeSet::new())
    }

    /// Walked date-skewed under several noises, late commits forced until
    /// none: a superset of the topological answer; promoted (topological,
    /// nothing forced), exactly it.
    fn holds_skewed_then_promoted(&self) {
        let (_, exact) = self.topological();
        for noise in [[3u8, 1, 2, 0], [0, 3, 3, 1], [2, 2, 1, 3], [1, 0, 3, 2]] {
            let order: Vec<u32> = skewed(self.parents.len(), &noise)
                .into_iter()
                .map(|at| u32::try_from(at).expect("small"))
                .collect();
            let mut forced = BTreeSet::new();
            let shown = loop {
                let (mut walk, shown) = self.walk(&order, &forced);
                let late = walk.take_late();
                if late.is_empty() {
                    break shown;
                }
                let before = forced.len();
                forced.extend(late.iter().map(|commit| commit.0));
                assert!(forced.len() > before, "each round forces something new");
            };
            assert!(
                shown.is_superset(&exact),
                "skewed {order:?} missing {:?}",
                exact.difference(&shown).collect::<Vec<_>>()
            );
            let (_, promoted) = self.topological();
            assert_eq!(promoted, exact, "promoted equals the topological answer");
        }
    }
}

/// `development` (trunk, 4 → 5) and `sara/report`: 1, then 0 merges
/// `omar/fix` (2 → 3) into it. `sara/report` is unmerged; `omar/fix`'s ref
/// is kept, at 2.
const OMAR_INTO_SARA: Case = Case {
    parents: &[&[1, 2], &[5], &[3], &[5], &[5], &[]],
    keep: &[4],
};

#[test]
fn teammate_branch_merged_into_an_unfinished_branch_folds_under_its_merge() {
    let (walk, shown) = OMAR_INTO_SARA.topological();
    assert_eq!(
        shown,
        [0, 1, 4, 5].into(),
        "sara's tip shown, omar's hidden"
    );
    assert_eq!(
        walk.behind(h(2)),
        Some(h(0)),
        "omar's tip behind sara's merge"
    );
    assert_eq!(walk.behind(h(3)), Some(h(0)));
    assert_eq!(walk.folds(h(0)), Some(true));
    OMAR_INTO_SARA.holds_skewed_then_promoted();
}

/// [`OMAR_INTO_SARA`] one position down, then `omar/fix` gains 0 on 3.
const OMAR_MOVES_ON: Case = Case {
    parents: &[&[3], &[2, 3], &[6], &[4], &[6], &[6], &[]],
    keep: &[5],
};

#[test]
fn a_merged_branch_with_a_new_commit_shows_again_with_its_lane() {
    let (walk, shown) = OMAR_MOVES_ON.topological();
    assert_eq!(shown, (0..7).collect(), "omar's tip and first-parent chain");
    assert_eq!(walk.folds(h(1)), Some(false), "nothing behind sara's merge");
    OMAR_MOVES_ON.holds_skewed_then_promoted();
}

/// `wallet-release` at 4 (on trunk 5); `sara/fix-toc` 1 cut from 4; then
/// `wallet-release` gains 3, 2 and 0 merges it into `sara/fix-toc`.
const BASE_INTO_DEPENDENT: &[&[u32]] = &[&[1, 2], &[4], &[3], &[4], &[5], &[]];

#[test]
fn a_base_merged_into_its_dependent_folds_until_it_moves() {
    let not_head = Case {
        parents: BASE_INTO_DEPENDENT,
        keep: &[5],
    };
    let (walk, shown) = not_head.topological();
    assert_eq!(shown, [0, 1, 4, 5].into());
    assert_eq!(walk.behind(h(2)), Some(h(0)));
    assert_eq!(walk.behind(h(3)), Some(h(0)));
    assert_eq!(walk.folds(h(0)), Some(true));
    not_head.holds_skewed_then_promoted();

    let head = Case {
        parents: BASE_INTO_DEPENDENT,
        keep: &[5, 2],
    };
    let (walk, shown) = head.topological();
    assert_eq!(shown, (0..6).collect(), "HEAD's commits shown");
    assert_eq!(walk.folds(h(0)), Some(false), "the merge folds nothing");
    head.holds_skewed_then_promoted();
}

/// A teammate's branch: 2, then 1 merges HEAD's branch (3 → 5), then 0
/// merges the integration branch (4). Trunk 6.
const KEEP_INTO_TEAMMATE: Case = Case {
    parents: &[&[1, 4], &[2, 3], &[6], &[5], &[6], &[6], &[]],
    keep: &[6, 3, 4],
};

#[test]
fn keep_tips_never_fold() {
    let (walk, shown) = KEEP_INTO_TEAMMATE.topological();
    assert_eq!(
        shown,
        (0..7).collect(),
        "HEAD's and integration's lines shown"
    );
    assert_eq!(walk.folds(h(0)), Some(false));
    assert_eq!(walk.folds(h(1)), Some(false));
    KEEP_INTO_TEAMMATE.holds_skewed_then_promoted();
}

/// HEAD's `wallet-release`: 1, then 0 merges `pay-571` (2 → 3); trunk 4,
/// which does not contain `wallet-release`.
const PAY_INTO_HEAD: Case = Case {
    parents: &[&[1, 2], &[4], &[3], &[4], &[]],
    keep: &[4, 0],
};

#[test]
fn a_branch_merged_into_heads_unmerged_branch_folds() {
    let (walk, shown) = PAY_INTO_HEAD.topological();
    assert_eq!(shown, [0, 1, 4].into());
    assert_eq!(walk.behind(h(2)), Some(h(0)));
    assert_eq!(walk.behind(h(3)), Some(h(0)));
    assert_eq!(walk.folds(h(0)), Some(true));
    PAY_INTO_HEAD.holds_skewed_then_promoted();
}
