//! Side-by-side pairing of a hunk's removed and added lines.

use std::ops::Range;

use super::{Hunk, LineKind};

/// One paired run: the removed lines and added lines that sit side by side,
/// index-wise, at the same position in a change block.
///
/// A hunk's lines look like runs of `Context`, then a block of `Removed`
/// lines immediately followed by a block of `Added` lines, then `Context`
/// again. [`pairs`] turns each such block into (at most) one `Pair`: the
/// first `removed.len().min(added.len())` lines of each side, paired
/// index-wise. Lines past that — the run's longer side — are left out of
/// every pair's range ("unpaired"), for the caller to render as a plain
/// removal or addition rather than a side-by-side change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    /// Index range into `hunk.lines` for this run's removed lines that were
    /// paired.
    pub removed: Range<usize>,
    /// Index range into `hunk.lines` for this run's added lines that were
    /// paired, same length as `removed`.
    pub added: Range<usize>,
}

/// Pairs up a hunk's removed/added runs for side-by-side display.
///
/// Scans `hunk.lines` for maximal runs of `Removed` lines immediately
/// followed by maximal runs of `Added` lines. Each such run yields one
/// `Pair` covering `min(removed_len, added_len)` lines from its start; a run
/// with nothing on the other side (pure removal or pure addition) yields no
/// `Pair` at all. Pairs never overlap and never extend past `hunk.lines`.
pub fn pairs(hunk: &Hunk) -> Vec<Pair> {
    let lines = &hunk.lines;
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].kind != LineKind::Removed {
            i += 1;
            continue;
        }
        let removed_start = i;
        let mut j = i;
        while j < lines.len() && lines[j].kind == LineKind::Removed {
            j += 1;
        }
        let removed_len = j - removed_start;
        if j < lines.len() && lines[j].kind == LineKind::Added {
            let added_start = j;
            let mut k = j;
            while k < lines.len() && lines[k].kind == LineKind::Added {
                k += 1;
            }
            let added_len = k - added_start;
            let min_len = removed_len.min(added_len);
            if min_len > 0 {
                out.push(Pair {
                    removed: removed_start..removed_start + min_len,
                    added: added_start..added_start + min_len,
                });
            }
            i = k;
        } else {
            i = j;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::DiffLine;

    fn line(kind: LineKind) -> DiffLine {
        DiffLine {
            kind,
            text: Vec::new(),
            old_no: None,
            new_no: None,
            no_newline_at_eof: false,
        }
    }

    fn hunk(kinds: &[LineKind]) -> Hunk {
        Hunk {
            old_start: 1,
            old_lines: 0,
            new_start: 1,
            new_lines: 0,
            heading: Vec::new(),
            lines: kinds.iter().copied().map(line).collect(),
        }
    }

    use LineKind::{Added, Context, Removed};

    #[test]
    fn equal_run_pairs_fully() {
        let h = hunk(&[Context, Removed, Removed, Added, Added, Context]);
        let p = pairs(&h);
        assert_eq!(
            p,
            vec![Pair {
                removed: 1..3,
                added: 3..5
            }]
        );
    }

    #[test]
    fn unequal_run_pairs_the_shorter_length() {
        let h = hunk(&[Removed, Removed, Removed, Added, Context]);
        let p = pairs(&h);
        assert_eq!(
            p,
            vec![Pair {
                removed: 0..1,
                added: 3..4
            }]
        );
    }

    #[test]
    fn pure_removal_yields_no_pair() {
        let h = hunk(&[Context, Removed, Removed, Context]);
        assert_eq!(pairs(&h), Vec::new());
    }

    #[test]
    fn pure_addition_yields_no_pair() {
        let h = hunk(&[Context, Added, Added, Context]);
        assert_eq!(pairs(&h), Vec::new());
    }

    #[test]
    fn two_runs_in_one_hunk() {
        let h = hunk(&[
            Removed, Added, Context, Removed, Removed, Added, Added, Added,
        ]);
        let p = pairs(&h);
        assert_eq!(
            p,
            vec![
                Pair {
                    removed: 0..1,
                    added: 1..2
                },
                Pair {
                    removed: 3..5,
                    added: 5..7
                },
            ]
        );
    }

    mod properties {
        use proptest::prelude::*;

        use super::*;

        /// The case count for whichever tier this was compiled for: 64 in
        /// the fast tier, 1000 with `--features heavy-tests` (PROMPT.md
        /// §7.1). `graft-fixtures` owns the two numbers.
        fn tier() -> ProptestConfig {
            ProptestConfig::with_cases(graft_fixtures::proptest_cases())
        }

        fn kind() -> impl Strategy<Value = LineKind> {
            prop_oneof![Just(Context), Just(Removed), Just(Added)]
        }

        proptest! {
            #![proptest_config(tier())]

            /// Every pair's ranges land inside `hunk.lines`, no two pairs
            /// overlap (on either side), every `Removed`/`Added` line lies
            /// in exactly one pair's range or is unpaired, and no `Context`
            /// line is ever inside a pair's range.
            #[test]
            fn pairs_are_disjoint_in_range_and_never_swallow_context(kinds in prop::collection::vec(kind(), 0..40)) {
                let h = hunk(&kinds);
                let found = pairs(&h);

                for p in &found {
                    prop_assert_eq!(p.removed.len(), p.added.len());
                    prop_assert!(p.removed.end <= h.lines.len());
                    prop_assert!(p.added.end <= h.lines.len());
                    for idx in p.removed.clone() {
                        prop_assert_eq!(h.lines[idx].kind, Removed);
                    }
                    for idx in p.added.clone() {
                        prop_assert_eq!(h.lines[idx].kind, Added);
                    }
                }

                for (i, a) in found.iter().enumerate() {
                    for b in &found[i + 1..] {
                        let overlaps = |x: &std::ops::Range<usize>, y: &std::ops::Range<usize>| {
                            x.start < y.end && y.start < x.end
                        };
                        prop_assert!(!overlaps(&a.removed, &b.removed));
                        prop_assert!(!overlaps(&a.added, &b.added));
                    }
                }

                for (idx, line) in h.lines.iter().enumerate() {
                    if line.kind == Context {
                        continue;
                    }
                    let covering = found
                        .iter()
                        .filter(|p| {
                            (line.kind == Removed && p.removed.contains(&idx))
                                || (line.kind == Added && p.added.contains(&idx))
                        })
                        .count();
                    prop_assert!(covering <= 1, "line {idx} covered by {covering} pairs");
                }
            }
        }
    }
}
