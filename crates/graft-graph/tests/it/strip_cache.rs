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
use graft_graph::raster::cache::{ByteBudget, CacheError, Flush, STRIP_ROWS, StripCache};
use graft_graph::raster::{Backdrop, PlainRow, PlainStrip, Ring, rasterise_backdrop};
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

/// A cache whose byte budget never binds: what a test about keys and hits
/// needs. Eviction is the budget tests' business.
fn roomy(min_strips: usize) -> StripCache {
    StripCache::with_budget(ByteBudget(usize::MAX), min_strips)
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
        let raster_rows: Vec<PlainRow<'_>> =
            rows[range.start..end].iter().map(PlainRow::of).collect();
        let strip = PlainStrip {
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
            .backdrop(range.start, &strip, Backdrop::Full)
            .unwrap_or_else(|why| panic!("strip at row {}: {why}", range.start));
        assert_eq!(frame.rows, end - range.start);
        asked += 1;
    }
    asked
}

#[test]
fn strips_are_aligned_to_the_strip_height() {
    let cache = roomy(8);
    let ranges: Vec<_> = cache.strips_for(40..70).collect();
    assert_eq!(ranges, vec![32..64, 64..96]);
    assert_eq!(cache.strips_for(0..0).count(), 0);
    assert_eq!(cache.strips_for(0..32).collect::<Vec<_>>(), vec![0..32]);
}

#[test]
fn scrolling_one_row_rasterises_at_most_one_new_strip_and_scrolling_back_is_a_hit() {
    let rows = rows(400);
    let mut cache = roomy(16);
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
    let mut cache = roomy(4);
    let raster_rows: Vec<PlainRow<'_>> = rows[32..64].iter().map(PlainRow::of).collect();
    let strip = PlainStrip {
        rows: &raster_rows,
        above: Some(&rows[31]),
        lane_columns: width_of(&rows),
        marker: None,
        cell_width: CELL.0,
        cell_height: CELL.1,
        palette: &PALETTE,
        muted: MUTED,
    };
    let direct = rasterise_backdrop(&strip, Backdrop::Full).expect("rasterises");
    assert_eq!(
        *cache
            .backdrop(32, &strip, Backdrop::Full)
            .expect("miss rasterises"),
        direct
    );
    assert_eq!(
        *cache.backdrop(32, &strip, Backdrop::Full).expect("hit"),
        direct
    );
    assert_eq!(cache.stats().hits, 1);
}

#[test]
fn a_theme_change_flushes_and_misses() {
    let rows = rows(100);
    let mut cache = roomy(8);
    frame(&mut cache, &rows, 0..50, &PALETTE, CELL);
    cache.flush(Flush::Theme);
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.resident_bytes(), 0);
    assert_eq!(cache.stats().flushes, 1);
    // Even without the flush, a different palette is a different key.
    let mut unflushed = roomy(8);
    frame(&mut unflushed, &rows, 0..50, &PALETTE, CELL);
    frame(&mut unflushed, &rows, 0..50, &OTHER_PALETTE, CELL);
    assert_eq!(unflushed.stats().rasterised, 4);
}

#[test]
fn a_cell_size_change_is_a_different_key() {
    let rows = rows(100);
    let mut cache = roomy(8);
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
    let mut cache = roomy(16);
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

/// The bytes one full backdrop strip of `rows` weighs at `cell`.
fn strip_bytes(rows: &[Row], cell: (u32, u32)) -> usize {
    STRIP_ROWS * cell.1 as usize * width_of(rows) * 2 * cell.0 as usize * 4
}

#[test]
fn the_cache_is_bounded_by_bytes_and_evicts_the_least_recently_used() {
    let rows = rows(400);
    let budget = ByteBudget(3 * strip_bytes(&rows, CELL));
    let mut cache = StripCache::with_budget(budget, 1);
    frame(&mut cache, &rows, 0..32, &PALETTE, CELL);
    frame(&mut cache, &rows, 32..64, &PALETTE, CELL);
    frame(&mut cache, &rows, 64..96, &PALETTE, CELL);
    frame(&mut cache, &rows, 0..32, &PALETTE, CELL); // touch 0..32
    frame(&mut cache, &rows, 96..128, &PALETTE, CELL); // evicts 32..64
    assert_eq!(cache.len(), 3);
    assert_eq!(cache.stats().evicted, 1);
    assert!(cache.resident_bytes() <= budget.0);
    assert!(!cache.minimum_wins());
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

/// The same budget holds more narrow strips than wide ones: the cache
/// weighs strips, it does not count them.
#[test]
fn a_budget_holds_more_small_strips_than_big_ones() {
    let rows = rows(400);
    let budget = ByteBudget(6 * strip_bytes(&rows, CELL));
    let mut small = StripCache::with_budget(budget, 1);
    let mut big = StripCache::with_budget(budget, 1);
    for top in (0..320).step_by(STRIP_ROWS) {
        frame(&mut small, &rows, top..top + STRIP_ROWS, &PALETTE, CELL);
        frame(&mut big, &rows, top..top + STRIP_ROWS, &PALETTE, USER_CELL);
    }
    assert_eq!(small.len(), 6);
    assert!(big.len() < small.len(), "{} user-cell strips", big.len());
    assert!(!big.is_empty());
    assert!(small.resident_bytes() <= budget.0 && big.resident_bytes() <= budget.0);
}

/// A budget too small for the viewport would resend every strip every
/// frame. The viewport's minimum wins, and says so.
#[test]
fn the_viewport_minimum_wins_over_a_budget_too_small() {
    let rows = rows(400);
    let mut cache = StripCache::with_budget(ByteBudget(1), 2);
    frame(&mut cache, &rows, 0..64, &PALETTE, CELL);
    assert_eq!(cache.len(), 2, "both viewport strips stay");
    assert!(cache.minimum_wins());
    let before = cache.stats().rasterised;
    frame(&mut cache, &rows, 0..64, &PALETTE, CELL);
    assert_eq!(cache.stats().rasterised, before, "the viewport is all hits");
    frame(&mut cache, &rows, 64..96, &PALETTE, CELL);
    assert_eq!(cache.len(), 2);
    // The viewport grows: the minimum follows it.
    cache.keep_at_least(3);
    frame(&mut cache, &rows, 0..96, &PALETTE, CELL);
    assert_eq!(cache.len(), 3);
    assert_eq!(cache.min_strips(), 3);
    // A roomy budget: the minimum is not what holds them.
    let mut roomy = StripCache::with_budget(ByteBudget(usize::MAX), 2);
    frame(&mut roomy, &rows, 0..64, &PALETTE, CELL);
    assert!(!roomy.minimum_wins());
}

#[test]
fn a_misaligned_or_oversized_strip_is_refused() {
    let rows = rows(100);
    let mut cache = roomy(4);
    let raster_rows: Vec<PlainRow<'_>> = rows[0..33].iter().map(PlainRow::of).collect();
    let strip = |rows| PlainStrip {
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
        cache.backdrop(5, &strip(&raster_rows[..27]), Backdrop::Full),
        Err(CacheError::Misaligned { .. })
    ));
    assert!(matches!(
        cache.backdrop(0, &strip(&raster_rows), Backdrop::Full),
        Err(CacheError::Misaligned { .. })
    ));
    assert!(matches!(
        cache.backdrop(0, &strip(&raster_rows[..0]), Backdrop::Full),
        Err(CacheError::Misaligned { .. })
    ));
    assert!(matches!(
        cache
            .backdrop(0, &strip(&raster_rows[..32]), Backdrop::Full)
            .map(|_| ()),
        Ok(())
    ));
}

#[test]
fn resident_bytes_for_a_viewport_plus_buffer() {
    // The user's cell, 12 lanes, a 60-row viewport with one strip of buffer
    // either side: the most the renderer keeps resident.
    let rows = rows(1_000);
    let capacity = StripCache::capacity_for(STRIP_ROWS, 60, STRIP_ROWS);
    // No room past the minimum: exactly the viewport and its buffer stay.
    let mut cache = StripCache::with_budget(ByteBudget(0), capacity);
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
/// A tool, not a check — it prints numbers and asserts nothing, so it stays
/// `#[ignore]`d and is the one test in the workspace that does: `cargo test
/// --release -p graft-graph --test it strip_height_numbers -- --ignored
/// --nocapture`.
#[test]
#[ignore = "timing tool; run by hand in release when choosing the strip height"]
fn strip_height_numbers() {
    let rows = rows(4_000);
    let lanes = 12;
    for height in [8, 16, 32, 64, 128] {
        let raster_rows: Vec<PlainRow<'_>> = rows[1_024..1_024 + height]
            .iter()
            .map(PlainRow::of)
            .collect();
        let strip = PlainStrip {
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
                rasterise_backdrop(&strip, Backdrop::Full).expect("rasterises");
                start.elapsed().as_micros()
            })
            .collect();
        raster.sort_unstable();
        let mut hit: Vec<u128> = (0..15)
            .map(|_| {
                let start = Instant::now();
                std::hint::black_box(graft_graph::raster::cache::plain_layout_hash(&strip));
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
    let mut cache = roomy(4);
    let virtual_row = PlainRow {
        ring: Ring::Uncommitted,
        ..PlainRow::of(&rows[39])
    };
    let mut with_virtual = vec![virtual_row.clone()];
    with_virtual.extend(rows[0..32].iter().map(PlainRow::of));
    let strip = |rows| PlainStrip {
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
        .backdrop(0, &strip(&with_virtual), Backdrop::Full)
        .unwrap_or_else(|why| panic!("32 history rows and the virtual row: {why}"));
    assert_eq!(frame.rows, 33);
    cache
        .backdrop(0, &strip(&with_virtual[1..]), Backdrop::Full)
        .unwrap_or_else(|why| panic!("the same strip without it: {why}"));
    assert_eq!(cache.len(), 2, "with and without the virtual row key apart");

    let mut two_over = vec![virtual_row];
    two_over.extend(rows[0..33].iter().map(PlainRow::of));
    assert!(matches!(
        cache.backdrop(0, &strip(&two_over), Backdrop::Full),
        Err(CacheError::Misaligned { .. })
    ));
}

/// m1-the-graph/39: a stub is drawn from the row (by ticket 40), so a row
/// gaining one must key apart from the same row without it.
#[test]
fn a_row_gaining_a_stub_changes_the_layout_hash() {
    use graft_graph::fold::Toward;
    use graft_graph::identity::BranchIdentity;
    use graft_graph::raster::Stub;
    use graft_graph::raster::cache::plain_layout_hash as layout_hash;

    let rows = rows(32);
    let plain: Vec<PlainRow<'_>> = rows.iter().map(PlainRow::of).collect();
    let mut stubbed = plain.clone();
    stubbed[5].stubs.push(Stub {
        identity: BranchIdentity::of_name(b"topic"),
        toward: Toward::Below,
    });
    let strip = |rows| PlainStrip {
        rows,
        above: None,
        lane_columns: 3,
        marker: None,
        cell_width: CELL.0,
        cell_height: CELL.1,
        palette: &PALETTE,
        muted: MUTED,
    };
    assert_ne!(layout_hash(&strip(&plain)), layout_hash(&strip(&stubbed)));
    let mut above = stubbed.clone();
    above[5].stubs[0].toward = Toward::Above;
    assert_ne!(layout_hash(&strip(&stubbed)), layout_hash(&strip(&above)));
}

/// ADR-0053: a strip is drawn from layout and one backdrop alpha, so moving
/// the cursor over every row of a branchy window — the thread changing from
/// row to row — never misses a strip once the viewport is resident.
#[test]
fn the_selection_never_misses_a_backdrop() {
    use super::emphasis_layer::{CELL, branchy, lits};
    use graft_graph::raster::cache::plain_layout_hash;
    use graft_graph::raster::{Backdrop, Selection};

    let window = branchy();
    let threads = window.threads();
    let strips = window.strips();
    assert!(strips.len() >= 3 && window.lane_columns > 8);
    let mut cache = roomy(2 * strips.len());
    let mut keys: Vec<Option<u64>> = vec![None; strips.len()];
    for (cursor, thread) in threads.iter().enumerate() {
        for (index, range) in strips.iter().enumerate() {
            let plain = window.plain(range.clone());
            let strip = window.strip(&plain, range, CELL);
            let hash = plain_layout_hash(&strip);
            assert_eq!(
                *keys[index].get_or_insert(hash),
                hash,
                "cursor {cursor}: strip {range:?}'s layout hash moved"
            );
            if cursor > 0 {
                assert!(
                    cache.contains_backdrop(range.start, &strip, Backdrop::OffThread),
                    "cursor {cursor}: strip {range:?} is not resident"
                );
            }
            cache
                .backdrop(range.start, &strip, Backdrop::OffThread)
                .unwrap_or_else(|why| panic!("strip {range:?}: {why}"));
            let lits = lits(thread, range.clone(), Backdrop::OffThread);
            cache
                .emphasis(
                    range.start,
                    &strip,
                    &Selection {
                        rows: &lits,
                        backdrop: Backdrop::OffThread,
                    },
                )
                .unwrap_or_else(|why| panic!("emphasis {range:?}: {why}"));
        }
        assert_eq!(
            cache.stats().rasterised,
            strips.len() as u64,
            "cursor {cursor}: a backdrop was rasterised again"
        );
    }
    assert_eq!(
        cache.stats().hits,
        (strips.len() * (threads.len() - 1)) as u64
    );
}

/// Holding `j` then `k` over ten rows draws each distinct selection's
/// emphasis once; going back over it is all hits.
#[test]
fn emphasis_is_cached_by_selection() {
    use super::emphasis_layer::{CELL, branchy, lits};
    use graft_graph::raster::cache::lit_hash;
    use graft_graph::raster::{Backdrop, Emphasis, Selection};
    use std::collections::HashSet;

    let window = branchy();
    let threads = window.threads();
    let strips = window.strips();
    let mut cache = roomy(64);
    let mut seen: HashSet<(usize, u64)> = HashSet::new();
    let mut asked = 0u64;
    let path = (40..50).chain((40..50).rev()).chain(40..50);
    for cursor in path {
        for range in &strips {
            let plain = window.plain(range.clone());
            let strip = window.strip(&plain, range, CELL);
            let lits = lits(&threads[cursor], range.clone(), Backdrop::Faded);
            let selection = Selection {
                rows: &lits,
                backdrop: Backdrop::Faded,
            };
            seen.insert((range.start, lit_hash(&selection)));
            cache
                .emphasis(range.start, &strip, &selection)
                .unwrap_or_else(|why| panic!("emphasis {range:?}: {why}"));
            asked += 1;
        }
    }
    let stats = cache.stats();
    assert_eq!(stats.emphasis_rasterised, seen.len() as u64);
    assert_eq!(stats.emphasis_hits, asked - seen.len() as u64);
    assert!(stats.emphasis_hits > 0);

    // Under a thread the paint never reads a row's own emphasis (as the
    // legacy key, 30b): toggling it is the same selection, a hit.
    let range = &strips[1];
    let plain = window.plain(range.clone());
    let strip = window.strip(&plain, range, CELL);
    let mut lits = lits(&threads[45], range.clone(), Backdrop::OffThread);
    let draw = |cache: &mut StripCache, rows: &[graft_graph::raster::Lit<'_>]| {
        cache
            .emphasis(
                range.start,
                &strip,
                &Selection {
                    rows,
                    backdrop: Backdrop::OffThread,
                },
            )
            .expect("emphasis")
    };
    let lit = draw(&mut cache, &lits);
    let before = cache.stats().emphasis_rasterised;
    lits[3].emphasis = match lits[3].emphasis {
        Emphasis::Normal => Emphasis::Dimmed,
        Emphasis::Dimmed => Emphasis::Normal,
    };
    assert_eq!(draw(&mut cache, &lits), lit, "the same paint");
    assert_eq!(cache.stats().emphasis_rasterised, before);
}

/// Memory: every layer the cache holds counts — backdrops, emphasis images
/// and composited strips — and a flush drops them all.
#[test]
fn resident_bytes_counts_every_layer() {
    use super::emphasis_layer::{CELL, branchy, lits};
    use graft_graph::raster::{Backdrop, Selection};

    let window = branchy();
    let threads = window.threads();
    let strips = window.strips();
    let mut cache = roomy(8);
    let mut expected = 0;
    for range in &strips {
        let plain = window.plain(range.clone());
        let strip = window.strip(&plain, range, CELL);
        let lits = lits(&threads[40], range.clone(), Backdrop::OffThread);
        let selection = Selection {
            rows: &lits,
            backdrop: Backdrop::OffThread,
        };
        let base = cache
            .backdrop(range.start, &strip, Backdrop::OffThread)
            .expect("backdrop");
        expected += base.pixels.len();
        if let Some(top) = cache
            .emphasis(range.start, &strip, &selection)
            .expect("emphasis")
        {
            expected += top.frame.pixels.len();
            let composited = cache
                .composited(range.start, &strip, &selection)
                .expect("composited");
            expected += composited.pixels.len();
        }
    }
    assert!(
        cache.stats().emphasis_rasterised >= 2,
        "the thread lit at least two strips"
    );
    assert_eq!(cache.resident_bytes(), expected);
    cache.flush(Flush::Theme);
    assert_eq!(cache.resident_bytes(), 0);
    assert!(cache.is_empty());
}
