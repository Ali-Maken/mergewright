//! Emphasis in per-column pieces (ADR-0053 amendment 2026-10-10, m8 05
//! Part 2). The window and its threads are `emphasis_layer`'s.

use std::collections::BTreeSet;

use graft_core::raster::RasterFrame;
use graft_graph::raster::{
    Backdrop, Lit, PlainStrip, Selection, composite, emphasis_pieces, rasterise_backdrop,
    rasterise_emphasis, rasterise_piece,
};

/// Every (strip, cursor, backdrop) of the branchy window this samples: a
/// cursor every few rows, both backdrops (`Faded` is Focus).
fn piece_cases() -> Vec<(std::ops::Range<usize>, usize, Backdrop)> {
    let window = super::emphasis_layer::branchy();
    let mut cases = Vec::new();
    for range in window.strips() {
        for cursor in (0..window.rows.len()).step_by(8) {
            for backdrop in [Backdrop::OffThread, Backdrop::Faded] {
                cases.push((range.clone(), cursor, backdrop));
            }
        }
    }
    cases
}

/// Backdrop + every piece == backdrop + the single emphasis, exact on
/// every pixel at most one piece paints. Where two pieces paint the same
/// pixel the alpha still matches (premultiplied "over" is order-free in
/// alpha), but which lit colour lies on top may not: the whole pass draws a
/// row's node, stubs and tail above every column's edges, the pieces stack
/// by column. Counted (`past_b255`), not bounded — ADR-0053 amendment
/// 2026-10-10 and ticket 05 NOTES say why the ruling's `b × 255` bound
/// could not hold there.
#[test]
fn pieces_composite_equals_single_emphasis() {
    use super::emphasis_layer::{CELL, USER_CELL, branchy, lits};
    let window = branchy();
    let threads = window.threads();
    let cases = piece_cases();
    assert!(cases.len() >= 20, "{} selections", cases.len());
    let mut tally = Tally::default();
    for (index, (range, cursor, backdrop)) in cases.iter().enumerate() {
        // The user's cell on a sample, so fractional stroke offsets are seen.
        let cell = if index % 7 == 0 { USER_CELL } else { CELL };
        let plain = window.plain(range.clone());
        let strip = window.strip(&plain, range, cell);
        let lit = lits(&threads[*cursor], range.clone(), *backdrop);
        let selection = Selection {
            rows: &lit,
            backdrop: *backdrop,
        };
        let case = format!("strip {range:?} cursor {cursor} {backdrop:?} cell {cell:?}");
        let base =
            rasterise_backdrop(&strip, *backdrop).unwrap_or_else(|why| panic!("{case}: {why}"));
        let single =
            rasterise_emphasis(&strip, &selection).unwrap_or_else(|why| panic!("{case}: {why}"));
        let want = single
            .as_ref()
            .map_or_else(|| base.clone(), |top| composite(&base, top));
        let (got, painters) = pieces_over(&strip, &selection, &base, &case);
        tally.lit_selections += usize::from(single.is_some());
        tally.compare(&want, &got, &painters, backdrop.alpha(), &case);
    }
    assert!(
        tally.lit_selections >= 20,
        "only {} selections drew emphasis",
        tally.lit_selections
    );
    // The overlap path is exercised, not vacuous.
    assert!(tally.overlapping > 0, "no pixel was painted by two pieces");
    println!(
        "SUMMARY pieces overlapped_pixels={} past_b255={}",
        tally.overlapping, tally.reordered
    );
}

/// `base` with every piece of `selection`'s emphasis composited over it in
/// column order, and per pixel how many pieces paint it.
fn pieces_over(
    strip: &PlainStrip<'_>,
    selection: &Selection<'_>,
    base: &RasterFrame,
    case: &str,
) -> (RasterFrame, Vec<u8>) {
    let mut got = base.clone();
    let mut painters = vec![0u8; (base.width * base.height) as usize];
    for piece in emphasis_pieces(strip, selection, 0) {
        let Some(top) = rasterise_piece(strip, selection, piece)
            .unwrap_or_else(|why| panic!("{case}: piece {piece:?}: {why}"))
        else {
            continue;
        };
        assert_eq!(
            top.frame.height, base.height,
            "{case}: a piece is its strip's height"
        );
        let left = u32::from(top.first_column) * strip.cell_width;
        for y in 0..top.frame.height {
            for x in (0..top.frame.width).filter(|x| left + x < base.width) {
                if top.frame.pixels[((y * top.frame.width + x) * 4 + 3) as usize] != 0 {
                    let pixel = (y * base.width + left + x) as usize;
                    painters[pixel] = painters[pixel].saturating_add(1);
                }
            }
        }
        got = composite(&got, &top);
    }
    (got, painters)
}

/// What `pieces_composite_equals_single_emphasis` saw across its cases.
#[derive(Default)]
struct Tally {
    lit_selections: usize,
    overlapping: usize,
    /// Overlapped pixels a channel of which is off by more than `b × 255`.
    reordered: usize,
}

impl Tally {
    fn compare(
        &mut self,
        want: &RasterFrame,
        got: &RasterFrame,
        painters: &[u8],
        backdrop: f32,
        case: &str,
    ) {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "An alpha in 0..=1 times 255."
        )]
        let bound = (backdrop * 255.0).ceil() as u8;
        let pairs = want
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(got.pixels.as_chunks::<4>().0);
        for (pixel, (want, got)) in pairs.enumerate() {
            if painters[pixel] <= 1 {
                assert_eq!(
                    want, got,
                    "{case}: pixel {pixel} ({} painters)",
                    painters[pixel]
                );
                continue;
            }
            self.overlapping += 1;
            assert!(
                want[3].abs_diff(got[3]) <= 2,
                "{case}: overlapped pixel {pixel} alpha {} vs {}",
                want[3],
                got[3]
            );
            if want.iter().zip(got).any(|(w, g)| w.abs_diff(*g) > bound) {
                self.reordered += 1;
            }
        }
    }
}

/// A piece's key holds its own column's alphas and nothing else: moving one
/// column of one row on or off the thread changes exactly that column's
/// key (or makes or unmakes its piece), when the row draws anything there;
/// no other piece's key moves.
#[test]
fn piece_key_moves_only_with_its_column() {
    use super::emphasis_layer::{CELL, branchy, lits};
    let window = branchy();
    let threads = window.threads();
    let range = 32..64;
    let plain = window.plain(range.clone());
    let strip = window.strip(&plain, &range, CELL);
    let keys = |lit: &[Lit<'_>]| piece_keys(&strip, lit);
    let mut moved = 0;
    for cursor in [33, 40, 47, 58] {
        let base = lits(&threads[cursor], range.clone(), Backdrop::OffThread);
        let before = keys(&base);
        for row in [0, 5, 17, 31] {
            let drawn = &plain[row].row;
            for column in 0..window.lane_columns {
                let mut set: BTreeSet<usize> = base[row].thread.cloned().unwrap_or_default();
                if !set.remove(&column) {
                    set.insert(column);
                }
                let mut changed = base.clone();
                changed[row].thread = Some(&set);
                let after = keys(&changed);
                let draws_here = drawn.lane == column
                    || drawn.lanes.get(column).copied().flatten().is_some()
                        && window.marker.map(|m| m.column) != Some(column);
                let case = format!("cursor {cursor} row {row} column {column}");
                for other in before.keys().chain(after.keys()) {
                    if *other != column {
                        assert_eq!(
                            before.get(other),
                            after.get(other),
                            "{case}: column {other} moved"
                        );
                    }
                }
                if draws_here {
                    assert_ne!(
                        before.get(&column),
                        after.get(&column),
                        "{case}: its own key stayed"
                    );
                    moved += 1;
                } else {
                    assert_eq!(
                        before.get(&column),
                        after.get(&column),
                        "{case}: nothing drawn, key moved"
                    );
                }
            }
        }
    }
    assert!(moved > 0, "no column drew anything");
}

/// Each piece's key by its column, off-thread backdrop, strip key 7.
fn piece_keys(strip: &PlainStrip<'_>, lit: &[Lit<'_>]) -> std::collections::BTreeMap<usize, u64> {
    let selection = Selection {
        rows: lit,
        backdrop: Backdrop::OffThread,
    };
    emphasis_pieces(strip, &selection, 7)
        .into_iter()
        .map(|piece| (piece.column, piece.key))
        .collect()
}

/// A different strip key is a different piece, whatever the alphas.
#[test]
fn piece_key_carries_the_strip_key() {
    use super::emphasis_layer::{CELL, branchy, lits};
    let window = branchy();
    let threads = window.threads();
    let range = 0..32;
    let plain = window.plain(range.clone());
    let strip = window.strip(&plain, &range, CELL);
    let lit = lits(&threads[3], range, Backdrop::OffThread);
    let selection = Selection {
        rows: &lit,
        backdrop: Backdrop::OffThread,
    };
    let (one, two) = (
        emphasis_pieces(&strip, &selection, 1),
        emphasis_pieces(&strip, &selection, 2),
    );
    assert!(!one.is_empty());
    for (a, b) in one.iter().zip(&two) {
        assert_eq!(a.column, b.column);
        assert_ne!(a.key, b.key);
    }
    let full = Selection {
        rows: &lit,
        backdrop: Backdrop::Full,
    };
    assert!(
        emphasis_pieces(&strip, &full, 1).is_empty(),
        "Full: no emphasis"
    );
}
