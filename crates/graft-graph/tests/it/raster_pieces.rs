//! Emphasis in pieces of three kinds — runs, arcs, nodes — keyed on their
//! content (ADR-0053 amendment 2026-10-10, amended after m8 05
//! continuation 1). The window and its threads are `emphasis_layer`'s.

use std::collections::{BTreeSet, HashMap};
use std::ops::Range;

use graft_graph::raster::{
    Backdrop, Selection, composite, emphasis_pieces, rasterise_backdrop, rasterise_emphasis,
    rasterise_piece,
};

mod support;

use super::emphasis_layer::{CELL, USER_CELL, branchy, lits};
use support::{
    Picture, Tally, both_caps, differ, owner_of, picture, piece_cases, piece_keys, pieces_over,
};

/// Backdrop + every piece in `emphasis_pieces` order == backdrop + the
/// single emphasis: exact where at most one piece paints a pixel; where
/// two do, alpha within 2 and every channel within `b × 255`.
#[test]
fn pieces_composite_equals_single_emphasis() {
    let mut tally = Tally::default();
    let mut selections = 0;
    for window in both_caps() {
        let threads = window.threads();
        for (index, (range, cursor, backdrop)) in piece_cases(&window).iter().enumerate() {
            // The user's cell on a sample, so fractional stroke offsets are seen.
            let cell = if index % 7 == 0 { USER_CELL } else { CELL };
            let plain = window.plain(range.clone());
            let strip = window.strip(&plain, range, cell);
            let lit = lits(&threads[*cursor], range.clone(), *backdrop);
            let selection = Selection {
                rows: &lit,
                backdrop: *backdrop,
            };
            let case = format!(
                "marker {:?} strip {range:?} cursor {cursor} {backdrop:?} cell {cell:?}",
                window.marker
            );
            let base =
                rasterise_backdrop(&strip, *backdrop).unwrap_or_else(|why| panic!("{case}: {why}"));
            let single = rasterise_emphasis(&strip, &selection)
                .unwrap_or_else(|why| panic!("{case}: {why}"));
            let want = single
                .as_ref()
                .map_or_else(|| base.clone(), |top| composite(&base, top));
            let (got, painters) = pieces_over(&strip, &selection, &base, &case);
            selections += 1;
            tally.lit_selections += usize::from(single.is_some());
            tally.compare(&want, &got, &painters, backdrop.alpha(), &case);
        }
    }
    println!(
        "SUMMARY pieces selections={selections} lit={} overlapped_pixels={} past_b255={}",
        tally.lit_selections, tally.overlapping, tally.reordered
    );
    assert!(selections >= 20, "{selections} selections");
    assert!(
        tally.lit_selections >= 20,
        "only {} selections drew emphasis",
        tally.lit_selections
    );
    // The overlap path is exercised, not vacuous.
    assert!(tally.overlapping > 0, "no pixel was painted by two pieces");
    assert_eq!(tally.reordered, 0, "overlapped pixels past b × 255");
}

/// A piece's key holds its own elements' alphas and nothing else: moving
/// one column of one row on or off the thread changes the key of exactly
/// the piece owning what that row draws there (or makes or unmakes it);
/// no other piece's key moves.
#[test]
fn piece_key_moves_only_with_its_pixels() {
    let window = branchy();
    let threads = window.threads();
    let range = 32..64;
    let plain = window.plain(range.clone());
    let strip = window.strip(&plain, &range, CELL);
    let mut moved = 0;
    for backdrop in [Backdrop::OffThread, Backdrop::Faded] {
        for cursor in [33, 40, 47, 58] {
            let base = lits(&threads[cursor], range.clone(), backdrop);
            let before = piece_keys(&strip, &base, backdrop);
            for row in [0, 5, 17, 31] {
                let drawn = plain[row].row;
                for column in 0..window.lane_columns {
                    let mut set: BTreeSet<usize> = base[row].thread.cloned().unwrap_or_default();
                    if !set.remove(&column) {
                        set.insert(column);
                    }
                    let mut changed = base.clone();
                    changed[row].thread = Some(&set);
                    let after = piece_keys(&strip, &changed, backdrop);
                    let owner = owner_of(&window, drawn, row, column);
                    let case = format!("{backdrop:?} cursor {cursor} row {row} column {column}");
                    for other in before.keys().chain(after.keys()) {
                        if Some(*other) != owner {
                            assert_eq!(
                                before.get(other),
                                after.get(other),
                                "{case}: {other:?} moved"
                            );
                        }
                    }
                    if let Some(owner) = owner {
                        assert_ne!(
                            before.get(&owner),
                            after.get(&owner),
                            "{case}: its owner {owner:?} stayed"
                        );
                        moved += 1;
                    }
                }
            }
        }
    }
    assert!(moved > 0, "no column drew anything");
}

/// Any two pieces with one key rasterise byte-equal with the same
/// anchor-relative crop, across strips, cursors, Focus, both cells and
/// both lane caps.
#[test]
fn equal_keys_equal_pixels() {
    let mut seen: HashMap<u64, Picture> = HashMap::new();
    let (mut pieces, mut selections, mut mismatched) = (0usize, 0usize, 0usize);
    for window in both_caps() {
        let threads = window.threads();
        for (range, cursor, backdrop) in piece_cases(&window) {
            for cell in [CELL, USER_CELL] {
                let plain = window.plain(range.clone());
                let strip = window.strip(&plain, &range, cell);
                let lit = lits(&threads[cursor], range.clone(), backdrop);
                let selection = Selection {
                    rows: &lit,
                    backdrop,
                };
                selections += 1;
                for piece in emphasis_pieces(&strip, &selection) {
                    let case = format!(
                        "marker {:?} strip {range:?} cursor {cursor} {backdrop:?} cell {cell:?} {piece:?}",
                        window.marker
                    );
                    let Some(top) = rasterise_piece(&strip, &selection, piece)
                        .unwrap_or_else(|why| panic!("{case}: {why}"))
                    else {
                        continue;
                    };
                    pieces += 1;
                    let picture = picture(&top, piece.column, piece.kind);
                    match seen.get(&piece.key) {
                        Some(earlier) if *earlier != picture => {
                            println!("{case}: {}", differ(earlier, &picture));
                            mismatched += 1;
                        }
                        Some(_) => {}
                        None => {
                            seen.insert(piece.key, picture);
                        }
                    }
                }
            }
        }
    }
    println!(
        "SUMMARY equal_keys selections={selections} pieces={pieces} distinct_keys={}",
        seen.len()
    );
    assert_eq!(mismatched, 0, "pieces with one key and other pixels");
    assert!(selections >= 20, "{selections} selections");
    assert!(seen.len() < pieces, "no key was shared: nothing compared");
}

/// Every piece is cropped to the cells its pixels touch: a drawn pixel in
/// its first and last cell column and its first and last cell row.
#[test]
fn pieces_crop_both_axes() {
    let mut checked = 0;
    let mut shorter = 0;
    for window in both_caps() {
        let threads = window.threads();
        for (index, (range, cursor, backdrop)) in piece_cases(&window).iter().enumerate() {
            let cell = if index % 5 == 0 { USER_CELL } else { CELL };
            let plain = window.plain(range.clone());
            let strip = window.strip(&plain, range, cell);
            let lit = lits(&threads[*cursor], range.clone(), *backdrop);
            let selection = Selection {
                rows: &lit,
                backdrop: *backdrop,
            };
            for piece in emphasis_pieces(&strip, &selection) {
                let case = format!("strip {range:?} cursor {cursor} {backdrop:?} {piece:?}");
                let Some(top) = rasterise_piece(&strip, &selection, piece)
                    .unwrap_or_else(|why| panic!("{case}: {why}"))
                else {
                    continue;
                };
                let frame = &top.frame;
                let (cw, ch) = (strip.cell_width, strip.cell_height);
                let painted = |xs: Range<u32>, ys: Range<u32>| {
                    ys.into_iter().any(|y| {
                        xs.clone()
                            .any(|x| frame.pixels[((y * frame.width + x) * 4 + 3) as usize] != 0)
                    })
                };
                let (w, h) = (frame.width, frame.height);
                assert!(w > 0 && h > 0, "{case}: empty");
                assert!(painted(0..cw, 0..h), "{case}: first column clear");
                assert!(painted(w - cw..w, 0..h), "{case}: last column clear");
                assert!(painted(0..w, 0..ch), "{case}: first row clear");
                assert!(painted(0..w, h - ch..h), "{case}: last row clear");
                checked += 1;
                shorter += usize::from(frame.rows < strip.rows.len());
            }
        }
    }
    assert!(checked > 0, "no piece drew");
    assert!(shorter > 0, "no piece was cropped in rows");
}

/// Under the full backdrop nothing draws above it: no pieces.
#[test]
fn full_backdrop_has_no_pieces() {
    let window = branchy();
    let threads = window.threads();
    let range = 0..32;
    let plain = window.plain(range.clone());
    let strip = window.strip(&plain, &range, CELL);
    let lit = lits(&threads[3], range, Backdrop::OffThread);
    let off = Selection {
        rows: &lit,
        backdrop: Backdrop::OffThread,
    };
    assert!(!emphasis_pieces(&strip, &off).is_empty());
    let full = Selection {
        rows: &lit,
        backdrop: Backdrop::Full,
    };
    assert!(
        emphasis_pieces(&strip, &full).is_empty(),
        "Full: no emphasis"
    );
}
