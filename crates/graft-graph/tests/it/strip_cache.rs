//! The strip cache (m1-the-graph/23, DESIGN.md §3.8): a scroll frame
//! rasterises at most one new strip, scrolling back is a hit, a theme change
//! flushes, and a layout change on one row costs only its own strip.

use std::ops::Range;
use std::time::Instant;

use graft_core::capabilities::Rgb;
use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_graph::identity::BranchIdentity;
use graft_graph::layout::layout;
use graft_graph::raster::cache::{CacheError, Flush, STRIP_ROWS, StripCache};
use graft_graph::raster::{RasterRow, Ring, Strip, rasterise};
use graft_graph::row::Row;
use graft_graph::seeds::{BranchSeed, Seeds};

const CELL: (u32, u32) = (8, 16);
/// The user's measured cell (ADR-0012's Ghostty bench).
const USER_CELL: (u32, u32) = (17, 37);
const PALETTE: [Rgb; 3] = [
    rgb(0x00, 0x77, 0xBB),
    rgb(0xEE, 0x88, 0x66),
    rgb(0xAA, 0x33, 0x77),
];
const OTHER_PALETTE: [Rgb; 3] = [
    rgb(0x11, 0x22, 0x33),
    rgb(0x44, 0x55, 0x66),
    rgb(0x77, 0x88, 0x99),
];
const MUTED: Rgb = rgb(0x88, 0x88, 0x88);

const fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}

fn oid(index: usize) -> Oid {
    let mut bytes = [0u8; Oid::SHA1_BYTES];
    bytes[..8].copy_from_slice(&(index as u64 + 1).to_be_bytes());
    Oid::from_bytes(&bytes).unwrap_or_else(|why| panic!("20 bytes is a valid oid length: {why}"))
}

fn at(seconds: i64) -> Identity {
    Identity {
        name: b"Gen".to_vec(),
        email: b"gen@example.invalid".to_vec(),
        when: Timestamp {
            seconds,
            offset_seconds: 0,
        },
    }
}

/// `count` commits in walk order: a first-parent chain, with a side commit
/// every fifth row merged back three rows up, so strips carry more than
/// one lane.
fn rows(count: usize) -> Vec<Row> {
    let commits: Vec<Commit> = (0..count)
        .map(|index| {
            let mut parents = Vec::new();
            if index + 1 < count {
                parents.push(oid(index + 1));
            }
            if index % 5 == 0 && index + 3 < count {
                parents.push(oid(index + 3));
            }
            let seconds = 2_000_000_000
                - i64::try_from(index).unwrap_or_else(|why| panic!("row {index}: {why}")) * 60;
            Commit {
                id: oid(index),
                parents,
                author: at(seconds),
                committer: at(seconds),
                message: format!("commit {index}").into_bytes(),
            }
        })
        .collect();
    let seeds = Seeds {
        default_branch: Some(BranchSeed {
            tip: oid(0),
            identity: BranchIdentity::of_name(b"main"),
        }),
        ..Seeds::default()
    };
    layout(&commits, seeds)
}

fn width_of(rows: &[Row]) -> usize {
    rows.iter()
        .map(|row| row.lanes.len().max(row.lane + 1))
        .max()
        .unwrap_or(0)
}

/// A renderer's frame: every aligned strip the viewport touches, through the
/// cache. Returns how many strips it asked for.
fn frame(
    cache: &mut StripCache,
    rows: &[Row],
    viewport: Range<usize>,
    palette: &[Rgb],
    cell: (u32, u32),
) -> usize {
    let lanes = width_of(rows);
    let mut asked = 0;
    for range in cache.strips_for(viewport) {
        let end = range.end.min(rows.len());
        if range.start >= end {
            continue;
        }
        let raster_rows: Vec<RasterRow<'_>> = rows[range.start..end]
            .iter()
            .map(RasterRow::plain)
            .collect();
        let strip = Strip {
            rows: &raster_rows,
            above: range.start.checked_sub(1).map(|above| &rows[above]),
            lane_columns: lanes,
            marker: None,
            cell_width: cell.0,
            cell_height: cell.1,
            palette,
            muted: MUTED,
        };
        let frame = cache
            .get(range.start, &strip)
            .unwrap_or_else(|why| panic!("strip at row {}: {why}", range.start));
        assert_eq!(frame.rows, end - range.start);
        asked += 1;
    }
    asked
}

#[test]
fn strips_are_aligned_to_the_strip_height() {
    let cache = StripCache::new(32, 8);
    let ranges: Vec<_> = cache.strips_for(40..70).collect();
    assert_eq!(ranges, vec![32..64, 64..96]);
    assert_eq!(cache.strips_for(0..0).count(), 0);
    assert_eq!(cache.strips_for(0..32).collect::<Vec<_>>(), vec![0..32]);
}

#[test]
fn scrolling_one_row_rasterises_at_most_one_new_strip_and_scrolling_back_is_a_hit() {
    let rows = rows(400);
    let mut cache = StripCache::new(STRIP_ROWS, 16);
    frame(&mut cache, &rows, 0..50, &PALETTE, CELL);
    assert_eq!(cache.stats().rasterised, 2);
    for top in 1..=200 {
        let before = cache.stats().rasterised;
        frame(&mut cache, &rows, top..top + 50, &PALETTE, CELL);
        assert!(
            cache.stats().rasterised - before <= 1,
            "scrolling to row {top} rasterised more than one strip"
        );
    }
    let before = cache.stats();
    for top in (0..200).rev() {
        frame(&mut cache, &rows, top..top + 50, &PALETTE, CELL);
    }
    let after = cache.stats();
    assert_eq!(
        after.rasterised, before.rasterised,
        "scrolling back within the resident strips is all hits"
    );
    assert!(after.hits > before.hits);
    assert_eq!(after.flushes, 0, "scrolling never flushes");
}

#[test]
fn a_cached_strip_is_the_strip_rasterise_draws() {
    let rows = rows(100);
    let mut cache = StripCache::new(STRIP_ROWS, 4);
    let raster_rows: Vec<RasterRow<'_>> = rows[32..64].iter().map(RasterRow::plain).collect();
    let strip = Strip {
        rows: &raster_rows,
        above: Some(&rows[31]),
        lane_columns: width_of(&rows),
        marker: None,
        cell_width: CELL.0,
        cell_height: CELL.1,
        palette: &PALETTE,
        muted: MUTED,
    };
    let direct = rasterise(&strip).expect("rasterises");
    assert_eq!(*cache.get(32, &strip).expect("miss rasterises"), direct);
    assert_eq!(*cache.get(32, &strip).expect("hit"), direct);
    assert_eq!(cache.stats().hits, 1);
}

#[test]
fn a_theme_change_flushes_and_misses() {
    let rows = rows(100);
    let mut cache = StripCache::new(STRIP_ROWS, 8);
    frame(&mut cache, &rows, 0..50, &PALETTE, CELL);
    cache.flush(Flush::Theme);
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.resident_bytes(), 0);
    assert_eq!(cache.stats().flushes, 1);
    // Even without the flush, a different palette is a different key.
    let mut unflushed = StripCache::new(STRIP_ROWS, 8);
    frame(&mut unflushed, &rows, 0..50, &PALETTE, CELL);
    frame(&mut unflushed, &rows, 0..50, &OTHER_PALETTE, CELL);
    assert_eq!(unflushed.stats().rasterised, 4);
}

#[test]
fn a_cell_size_change_is_a_different_key() {
    let rows = rows(100);
    let mut cache = StripCache::new(STRIP_ROWS, 8);
    frame(&mut cache, &rows, 0..50, &PALETTE, CELL);
    frame(&mut cache, &rows, 0..50, &PALETTE, USER_CELL);
    assert_eq!(cache.stats().rasterised, 4);
    for why in [Flush::CellSize, Flush::Redetection, Flush::Resize] {
        frame(&mut cache, &rows, 0..50, &PALETTE, CELL);
        cache.flush(why);
        assert!(cache.is_empty());
    }
}

#[test]
fn a_layout_change_on_one_row_invalidates_only_its_strip() {
    let original = rows(200);
    let mut cache = StripCache::new(STRIP_ROWS, 16);
    frame(&mut cache, &original, 0..160, &PALETTE, CELL);
    assert_eq!(cache.stats().rasterised, 5);
    let mut changed = original.clone();
    // Row 70 sits mid-strip (64..96): it is neither a strip's first row nor
    // the row above one. Recolour its own lane.
    changed[70].lanes[0] = Some(BranchIdentity::of_name(b"renamed"));
    frame(&mut cache, &changed, 0..160, &PALETTE, CELL);
    assert_eq!(
        cache.stats().rasterised,
        6,
        "only strip 64..96 re-rasterises"
    );
    // A different commit on the same row is new content, too.
    let mut reworded = original.clone();
    reworded[5].commit.id = oid(10_000);
    frame(&mut cache, &reworded, 0..32, &PALETTE, CELL);
    assert_eq!(cache.stats().rasterised, 7);
    // A message edit the rasteriser never reads is not.
    let mut message = original.clone();
    message[5].commit.message = b"ignored".to_vec();
    frame(&mut cache, &message, 0..32, &PALETTE, CELL);
    assert_eq!(cache.stats().rasterised, 7);
}

#[test]
fn emphasis_is_part_of_the_key() {
    let rows = rows(40);
    let mut cache = StripCache::new(STRIP_ROWS, 4);
    let lanes = width_of(&rows);
    let draw = |cache: &mut StripCache, dim: bool| {
        let raster_rows: Vec<RasterRow<'_>> = rows[0..32]
            .iter()
            .map(|row| RasterRow {
                emphasis: if dim {
                    graft_graph::raster::Emphasis::Dimmed
                } else {
                    graft_graph::raster::Emphasis::Normal
                },
                ..RasterRow::plain(row)
            })
            .collect();
        let strip = Strip {
            rows: &raster_rows,
            above: None,
            lane_columns: lanes,
            marker: None,
            cell_width: CELL.0,
            cell_height: CELL.1,
            palette: &PALETTE,
            muted: MUTED,
        };
        cache.get(0, &strip).expect("rasterises");
    };
    draw(&mut cache, false);
    draw(&mut cache, true);
    draw(&mut cache, false);
    assert_eq!(cache.stats().rasterised, 2);
    assert_eq!(cache.stats().hits, 1);
}

#[test]
fn the_cache_is_bounded_and_evicts_the_least_recently_used() {
    let rows = rows(400);
    let mut cache = StripCache::new(STRIP_ROWS, 3);
    frame(&mut cache, &rows, 0..32, &PALETTE, CELL);
    frame(&mut cache, &rows, 32..64, &PALETTE, CELL);
    frame(&mut cache, &rows, 64..96, &PALETTE, CELL);
    frame(&mut cache, &rows, 0..32, &PALETTE, CELL); // touch 0..32
    frame(&mut cache, &rows, 96..128, &PALETTE, CELL); // evicts 32..64
    assert_eq!(cache.len(), 3);
    assert_eq!(cache.stats().evicted, 1);
    let before = cache.stats().rasterised;
    frame(&mut cache, &rows, 0..32, &PALETTE, CELL);
    assert_eq!(cache.stats().rasterised, before, "0..32 was recently used");
    frame(&mut cache, &rows, 32..64, &PALETTE, CELL);
    assert_eq!(
        cache.stats().rasterised,
        before + 1,
        "32..64 was the eviction"
    );
}

#[test]
fn a_misaligned_or_oversized_strip_is_refused() {
    let rows = rows(100);
    let mut cache = StripCache::new(STRIP_ROWS, 4);
    let raster_rows: Vec<RasterRow<'_>> = rows[0..33].iter().map(RasterRow::plain).collect();
    let strip = |rows| Strip {
        rows,
        above: None,
        lane_columns: 2,
        marker: None,
        cell_width: CELL.0,
        cell_height: CELL.1,
        palette: &PALETTE,
        muted: MUTED,
    };
    assert!(matches!(
        cache.get(5, &strip(&raster_rows[..27])),
        Err(CacheError::Misaligned { .. })
    ));
    assert!(matches!(
        cache.get(0, &strip(&raster_rows)),
        Err(CacheError::Misaligned { .. })
    ));
    assert!(matches!(
        cache.get(0, &strip(&raster_rows[..0])),
        Err(CacheError::Misaligned { .. })
    ));
    assert!(matches!(
        cache.get(0, &strip(&raster_rows[..32])).map(|_| ()),
        Ok(())
    ));
}

#[test]
fn resident_bytes_for_a_viewport_plus_buffer() {
    // The user's cell, 12 lanes, a 60-row viewport with one strip of buffer
    // either side: the most the renderer keeps resident.
    let rows = rows(1_000);
    let capacity = StripCache::capacity_for(STRIP_ROWS, 60, STRIP_ROWS);
    let mut cache = StripCache::new(STRIP_ROWS, capacity);
    for top in (0..600).step_by(7) {
        frame(&mut cache, &rows, top..top + 60, &PALETTE, USER_CELL);
    }
    let lanes = width_of(&rows);
    let per_strip = STRIP_ROWS * USER_CELL.1 as usize * lanes * 2 * USER_CELL.0 as usize * 4;
    assert_eq!(cache.len(), capacity);
    assert_eq!(cache.resident_bytes(), capacity * per_strip);
    eprintln!(
        "strip cache: {capacity} strips of {STRIP_ROWS} rows × {lanes} lanes at {}×{} px = {} KiB resident ({} KiB a strip)",
        USER_CELL.0,
        USER_CELL.1,
        cache.resident_bytes() / 1024,
        per_strip / 1024
    );
}

/// The numbers behind [`STRIP_ROWS`]: per-strip rasterise time and bytes at
/// the user's cell, and what one strip costs as a scroll frame's worst case.
/// A tool, not a check: `cargo test --release -p graft-graph --test it
/// strip_height_numbers -- --ignored --nocapture`.
#[test]
#[ignore = "timing tool; run by hand in release when choosing the strip height"]
fn strip_height_numbers() {
    let rows = rows(4_000);
    let lanes = 12;
    for height in [8, 16, 32, 64, 128] {
        let raster_rows: Vec<RasterRow<'_>> = rows[1_024..1_024 + height]
            .iter()
            .map(RasterRow::plain)
            .collect();
        let strip = Strip {
            rows: &raster_rows,
            above: Some(&rows[1_023]),
            lane_columns: lanes,
            marker: None,
            cell_width: USER_CELL.0,
            cell_height: USER_CELL.1,
            palette: &PALETTE,
            muted: MUTED,
        };
        let mut raster: Vec<u128> = (0..15)
            .map(|_| {
                let start = Instant::now();
                rasterise(&strip).expect("rasterises");
                start.elapsed().as_micros()
            })
            .collect();
        raster.sort_unstable();
        let mut hit: Vec<u128> = (0..15)
            .map(|_| {
                let start = Instant::now();
                std::hint::black_box(graft_graph::raster::cache::layout_hash(&strip));
                start.elapsed().as_nanos()
            })
            .collect();
        hit.sort_unstable();
        let bytes = height * USER_CELL.1 as usize * lanes * 2 * USER_CELL.0 as usize * 4;
        eprintln!(
            "{height:>3} rows × {lanes} lanes at {}×{}: rasterise median {} µs, key {} ns, {} KiB a strip",
            USER_CELL.0,
            USER_CELL.1,
            raster[7],
            hit[7],
            bytes / 1024
        );
    }
}

/// The uncommitted row is not a history row: a full strip with it on top is
/// still aligned, and keys apart from the same strip without it.
#[test]
fn the_uncommitted_row_does_not_count_toward_the_strip_height() {
    let rows = rows(40);
    let mut cache = StripCache::new(STRIP_ROWS, 4);
    let virtual_row = RasterRow {
        ring: Ring::Uncommitted,
        ..RasterRow::plain(&rows[39])
    };
    let mut with_virtual = vec![virtual_row];
    with_virtual.extend(rows[0..32].iter().map(RasterRow::plain));
    let strip = |rows| Strip {
        rows,
        above: None,
        lane_columns: 3,
        marker: None,
        cell_width: CELL.0,
        cell_height: CELL.1,
        palette: &PALETTE,
        muted: MUTED,
    };
    let frame = cache
        .get(0, &strip(&with_virtual))
        .unwrap_or_else(|why| panic!("32 history rows and the virtual row: {why}"));
    assert_eq!(frame.rows, 33);
    cache
        .get(0, &strip(&with_virtual[1..]))
        .unwrap_or_else(|why| panic!("the same strip without it: {why}"));
    assert_eq!(cache.len(), 2, "with and without the virtual row key apart");

    let mut two_over = vec![virtual_row];
    two_over.extend(rows[0..33].iter().map(RasterRow::plain));
    assert!(matches!(
        cache.get(0, &strip(&two_over)),
        Err(CacheError::Misaligned { .. })
    ));
}
