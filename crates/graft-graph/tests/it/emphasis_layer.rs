//! The selection's paint as its own layer (ADR-0053, m7-power-tools/44).
//!
//! A strip is drawn from layout alone at one backdrop alpha; the selection's
//! thread is a second, cropped image drawn at `(a − b) / (1 − b)` so that,
//! composited over the strip, it gives ADR-0024's alpha `a` again. The anchor
//! is today's single-pass `rasterise`, which this ticket leaves byte-identical:
//! the composite must equal it wherever the emphasis layer is clear or solid,
//! and differ only on anti-aliased edges and crossings, by at most the
//! backdrop's alpha. Colour itself stays unverified (CLAUDE.md rule 5): the
//! PNGs `thread_pngs` writes are for the user's eye.

use std::collections::{BTreeSet, HashSet};
use std::ops::Range;
use std::time::Instant;

use graft_core::capabilities::Rgb;
use graft_core::raster::RasterFrame;
use graft_graph::ancestry::{Ancestry, Kin};
use graft_graph::fold::Toward;
use graft_graph::identity::BranchIdentity;
use graft_graph::raster::cache::{STRIP_ROWS, lit_hash};
use graft_graph::raster::{
    Backdrop, Emphasis, Emphasised, Lit, Marker, PlainRow, PlainStrip, RasterRow, Ring, Selection,
    Strip, Stub, composite, rasterise, rasterise_backdrop, rasterise_emphasis,
};
use graft_graph::row::Row;

/// The snapshot cell: small, so every cursor row of the window stays fast.
pub(crate) const CELL: (u32, u32) = (8, 16);
/// The user's measured cell (ADR-0012's Ghostty bench).
pub(crate) const USER_CELL: (u32, u32) = (17, 37);
pub(crate) const PALETTE: [Rgb; 6] = [
    rgb(0x00, 0x77, 0xBB),
    rgb(0xEE, 0x88, 0x66),
    rgb(0xAA, 0x33, 0x77),
    rgb(0x77, 0xAA, 0xDD),
    rgb(0xDD, 0xCC, 0x77),
    rgb(0x00, 0x99, 0x88),
];
pub(crate) const MUTED: Rgb = rgb(0x88, 0x88, 0x88);

const fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}

/// A branchy window: interleaved chains with merges crossing them, three
/// strips tall, a stub (folded-edge tail) on some rows, a head and a stash
/// ring, and the rail in a column right of every lane.
pub(crate) struct Window {
    pub(crate) rows: Vec<Row>,
    pub(crate) lane_columns: usize,
    pub(crate) marker: Option<Marker>,
}

pub(crate) fn branchy() -> Window {
    window(3 * STRIP_ROWS, 9)
}

pub(crate) fn window(count: usize, lanes: usize) -> Window {
    let rows = super::raster_pixels::synthetic(count, lanes);
    let lanes = rows
        .iter()
        .map(|row| row.lanes.len().max(row.lane + 1))
        .max()
        .unwrap_or(0);
    Window {
        rows,
        lane_columns: lanes + 1,
        marker: Some(Marker { column: lanes }),
    }
}

impl Window {
    /// The aligned strips the window covers.
    pub(crate) fn strips(&self) -> Vec<Range<usize>> {
        (0..self.rows.len())
            .step_by(STRIP_ROWS)
            .map(|start| start..(start + STRIP_ROWS).min(self.rows.len()))
            .collect()
    }

    /// The window's rows in `range`, as the layout draws them.
    pub(crate) fn plain(&self, range: Range<usize>) -> Vec<PlainRow<'_>> {
        range
            .map(|index| {
                let row = &self.rows[index];
                let mut stubs = Vec::new();
                if index % 11 == 5 {
                    stubs.push(Stub {
                        identity: BranchIdentity::of_name(b"folded-below"),
                        toward: Toward::Below,
                    });
                }
                if index % 13 == 7 {
                    stubs.push(Stub {
                        identity: BranchIdentity::of_name(b"folded-above"),
                        toward: Toward::Above,
                    });
                }
                let ring = match index {
                    2 => Ring::Head,
                    40 => Ring::Stash,
                    _ => Ring::of(row),
                };
                PlainRow { row, ring, stubs }
            })
            .collect()
    }

    pub(crate) fn strip<'a>(
        &'a self,
        rows: &'a [PlainRow<'a>],
        range: &Range<usize>,
        cell: (u32, u32),
    ) -> PlainStrip<'a> {
        PlainStrip {
            rows,
            above: range.start.checked_sub(1).map(|above| &self.rows[above]),
            lane_columns: self.lane_columns,
            marker: self.marker,
            cell_width: cell.0,
            cell_height: cell.1,
            palette: &PALETTE,
            muted: MUTED,
        }
    }

    /// The thread of every row as the cursor, over the whole window.
    pub(crate) fn threads(&self) -> Vec<Ancestry> {
        self.rows
            .iter()
            .map(|row| {
                Ancestry::of(&self.rows, row.commit.id)
                    .unwrap_or_else(|| panic!("row {} is in its own window", row.commit.id))
            })
            .collect()
    }
}

/// The selection's paint over `range` as the app gives it
/// (`graft/src/view/strips.rs` `Painter::row`): every row carries the
/// thread's lit columns; Focus (the `Faded` backdrop) also dims the rows
/// unrelated to the cursor, which the thread then overrides.
pub(crate) fn lits(thread: &Ancestry, range: Range<usize>, backdrop: Backdrop) -> Vec<Lit<'_>> {
    range
        .map(|index| Lit {
            emphasis: if backdrop == Backdrop::Faded && thread.kin(index) == Kin::Unrelated {
                Emphasis::Dimmed
            } else {
                Emphasis::Normal
            },
            thread: Some(thread.lit(index)),
        })
        .collect()
}

/// Today's single pass over the same rows, ring, stubs and paint.
fn legacy(strip: &PlainStrip<'_>, lits: &[Lit<'_>]) -> RasterFrame {
    let rows: Vec<RasterRow<'_>> = strip
        .rows
        .iter()
        .zip(lits)
        .map(|(plain, lit)| RasterRow {
            row: plain.row,
            ring: plain.ring,
            emphasis: lit.emphasis,
            thread: lit.thread,
            stubs: plain.stubs.clone(),
        })
        .collect();
    rasterise(&Strip {
        rows: &rows,
        above: strip.above,
        lane_columns: strip.lane_columns,
        marker: strip.marker,
        cell_width: strip.cell_width,
        cell_height: strip.cell_height,
        palette: strip.palette,
        muted: strip.muted,
    })
    .unwrap_or_else(|why| panic!("legacy rasterise: {why}"))
}

fn backdrop_of(strip: &PlainStrip<'_>, backdrop: Backdrop) -> RasterFrame {
    rasterise_backdrop(strip, backdrop).unwrap_or_else(|why| panic!("backdrop: {why}"))
}

fn emphasis_of(strip: &PlainStrip<'_>, lits: &[Lit<'_>], backdrop: Backdrop) -> Option<Emphasised> {
    rasterise_emphasis(
        strip,
        &Selection {
            rows: lits,
            backdrop,
        },
    )
    .unwrap_or_else(|why| panic!("emphasis: {why}"))
}

/// ADR-0024's alpha for every element of a row, as today's pass gives it:
/// each drawn lane column and the rail. How many fall below the backdrop
/// (they would be overdrawn by it, so the layer split cannot express them).
fn below_backdrop(strip: &PlainStrip<'_>, lits: &[Lit<'_>], backdrop: Backdrop) -> usize {
    let alpha_at = |lit: &Lit<'_>, column: usize| match lit.thread {
        Some(thread) if thread.contains(&column) => 1.0,
        Some(_) => graft_graph::raster::OFF_THREAD_ALPHA,
        None => match lit.emphasis {
            Emphasis::Normal => 1.0,
            Emphasis::Dimmed => graft_graph::raster::FADED_ALPHA,
        },
    };
    let rail = |lit: &Lit<'_>| match (lit.thread, lit.emphasis) {
        (Some(_), _) => graft_graph::raster::OFF_THREAD_ALPHA,
        (None, Emphasis::Normal) => 1.0,
        (None, Emphasis::Dimmed) => graft_graph::raster::FADED_ALPHA,
    };
    let b = backdrop.alpha();
    strip
        .rows
        .iter()
        .zip(lits)
        .map(|(plain, lit)| {
            let row = plain.row;
            let lanes = (0..row.lanes.len().max(row.lane + 1))
                .filter(|&column| column == row.lane || row.lanes[column].is_some())
                .filter(|&column| alpha_at(lit, column) < b)
                .count();
            lanes + usize::from(strip.marker.is_some() && rail(lit) < b)
        })
        .sum()
}

/// One case's pixels, by what the emphasis layer holds there (ADR-0053
/// Consequences, ticket 44 Decided 7).
#[derive(Clone, Copy, Debug, Default)]
struct Tally {
    /// Emphasis clear: the backdrop alone, which must be the single pass.
    clear: usize,
    /// …of which differ from the single pass (must be 0).
    clear_differ: usize,
    /// Emphasis solid and the composite is the single pass.
    solid: usize,
    /// Emphasis solid, the single pass drew something else over the lit
    /// element: a crossing, where the lit line is now on top.
    crossings: usize,
    /// The largest channel difference at a crossing.
    crossing_max: u8,
    /// Emphasis partly covering: an anti-aliased edge.
    edge: usize,
    /// The largest channel difference on an edge.
    edge_max: u8,
    /// Edges differing by more than `ceil(b × 255)` (must be 0).
    edge_over: usize,
    /// Pixels either image paints.
    painted: usize,
}

fn tally(legacy: &RasterFrame, composite: &RasterFrame, top: Option<&Emphasised>, b: f32) -> Tally {
    assert_eq!(
        (legacy.width, legacy.height, legacy.columns),
        (composite.width, composite.height, composite.columns),
        "the composite is the single pass's size"
    );
    let tolerance = ceil_255(b);
    let cell_width = legacy.width / legacy.columns.max(1);
    // The emphasis alpha under every pixel of the strip: 0 outside the crop.
    let mut under = vec![0u8; legacy.pixels.len() / 4];
    if let Some(top) = top {
        let (width, left) = (
            top.frame.width as usize,
            (u32::from(top.first_column) * cell_width) as usize,
        );
        for (y, line) in top.frame.pixels.chunks_exact(width * 4).enumerate() {
            let at = y * legacy.width as usize + left;
            for (x, pixel) in line.as_chunks::<4>().0.iter().enumerate() {
                under[at + x] = pixel[3];
            }
        }
    }
    let mut tally = Tally::default();
    let singles = legacy.pixels.as_chunks::<4>().0;
    let boths = composite.pixels.as_chunks::<4>().0;
    for ((single, both), &emphasis) in singles.iter().zip(boths).zip(&under) {
        if single[3] > 0 || both[3] > 0 {
            tally.painted += 1;
        }
        let difference = single
            .iter()
            .zip(both)
            .map(|(&left, &right)| left.abs_diff(right))
            .max()
            .unwrap_or(0);
        match emphasis {
            0 => {
                tally.clear += 1;
                tally.clear_differ += usize::from(single != both);
            }
            255 if single == both => tally.solid += 1,
            255 => {
                tally.crossings += 1;
                tally.crossing_max = tally.crossing_max.max(difference);
            }
            _ => {
                tally.edge += 1;
                tally.edge_max = tally.edge_max.max(difference);
                tally.edge_over += usize::from(difference > tolerance);
            }
        }
    }
    tally
}

/// `ceil(b × 255)`: the most a backdrop of alpha `b` can move a channel.
fn ceil_255(b: f32) -> u8 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0.0..=255.0.
    let bound = (b * 255.0).ceil() as u8;
    bound
}

fn composed(base: &RasterFrame, top: Option<&Emphasised>) -> RasterFrame {
    top.map_or_else(|| base.clone(), |top| composite(base, top))
}

/// One strip, cursor and backdrop against the single pass: the tally
/// printed, and its exact classes asserted.
fn one_case(
    strip: &PlainStrip<'_>,
    lits: &[Lit<'_>],
    base: &RasterFrame,
    backdrop: Backdrop,
    label: &str,
) -> Tally {
    let single = legacy(strip, lits);
    let top = emphasis_of(strip, lits, backdrop);
    let both = composed(base, top.as_ref());
    let tally = tally(&single, &both, top.as_ref(), backdrop.alpha());
    eprintln!(
        "{label}: clear {} (differ {}), solid {}, crossings {} (max {}), edge {} (max {}, over \
         {}), painted {}",
        tally.clear,
        tally.clear_differ,
        tally.solid,
        tally.crossings,
        tally.crossing_max,
        tally.edge,
        tally.edge_max,
        tally.edge_over,
        tally.painted,
    );
    assert_eq!(
        tally.clear_differ, 0,
        "{label}: where the emphasis is clear the composite must be the single pass"
    );
    assert_eq!(
        tally.edge_over,
        0,
        "{label}: an edge moved more than ceil(b × 255) = {}",
        ceil_255(backdrop.alpha())
    );
    tally
}

#[test]
fn composite_matches_the_single_pass() {
    let window = branchy();
    assert!(
        window.lane_columns > 8,
        "the window is {} lanes",
        window.lane_columns
    );
    assert!(window.strips().len() >= 3);
    let threads = window.threads();
    let (mut below, mut cases, mut repeats, mut crossings, mut painted) = (0, 0, 0, 0, 0);
    // Every distinct paint at the snapshot cell, where a 1.5 px stroke
    // centred on a pixel edge never covers a pixel whole; and every fourth
    // at the user's, where strokes and nodes do, so the solid and crossing
    // classes are exercised too (all of them would take ~20 s in debug).
    for (cell, sample) in [(CELL, 1), (USER_CELL, 4)] {
        for range in window.strips() {
            let plain = window.plain(range.clone());
            let strip = window.strip(&plain, &range, cell);

            // No thread on screen: the backdrop is the single pass, exactly.
            let none = vec![Lit::default(); plain.len()];
            assert_eq!(
                backdrop_of(&strip, Backdrop::Full),
                legacy(&strip, &none),
                "strip {range:?}: Full is today's plain strip"
            );
            assert_eq!(emphasis_of(&strip, &none, Backdrop::Full), None);

            for backdrop in [Backdrop::OffThread, Backdrop::Faded] {
                let base = backdrop_of(&strip, backdrop);
                // A cursor whose paint over this strip equals an earlier
                // one's draws the same pixels: run each distinct paint once.
                let mut seen = HashSet::new();
                for (cursor, thread) in threads.iter().enumerate() {
                    let lits = lits(thread, range.clone(), backdrop);
                    below += below_backdrop(&strip, &lits, backdrop);
                    if !seen.insert(lit_hash(&Selection {
                        rows: &lits,
                        backdrop,
                    })) {
                        repeats += 1;
                        continue;
                    }
                    if (seen.len() - 1) % sample != 0 {
                        continue;
                    }
                    let label = format!(
                        "{}x{} strip {range:?} cursor {cursor:>2} {backdrop:?}",
                        cell.0, cell.1
                    );
                    let tally = one_case(&strip, &lits, &base, backdrop, &label);
                    crossings += tally.crossings;
                    painted += tally.painted;
                    cases += 1;
                }
            }
        }
    }
    eprintln!(
        "{cases} cases drawn ({repeats} cursors repeating an earlier paint skipped, the user's \
         cell sampled 1 in 4): {crossings} crossing pixels of {painted} painted ({} per \
         100 000), {below} elements below the backdrop",
        crossings * 100_000 / painted.max(1)
    );
    assert_eq!(below, 0, "an element drawn below the backdrop's alpha");
    assert!(
        crossings * 200 <= painted,
        "crossings are {crossings} of {painted} painted pixels, over 0.5%"
    );
}

#[test]
fn an_empty_thread_is_not_no_thread() {
    let window = branchy();
    let threads = window.threads();
    let range = 32..64;
    let plain = window.plain(range.clone());
    let strip = window.strip(&plain, &range, CELL);
    let empty = BTreeSet::new();
    for backdrop in [Backdrop::OffThread, Backdrop::Faded] {
        let base = lits(&threads[40], range.clone(), backdrop);
        let mut with_empty = base.clone();
        let mut with_none = base.clone();
        with_empty[9].thread = Some(&empty);
        with_none[9].thread = None;
        let hash = |rows: &[Lit<'_>]| lit_hash(&Selection { rows, backdrop });
        assert_ne!(
            hash(&with_empty),
            hash(&with_none),
            "{backdrop:?}: Some(∅) and None hash alike"
        );
        // Some(∅) draws the row at OFF_THREAD_ALPHA; None at the row's own
        // emphasis (Normal: full). The alphas differ, so must the images.
        assert_ne!(
            emphasis_of(&strip, &with_empty, backdrop),
            emphasis_of(&strip, &with_none, backdrop),
            "{backdrop:?}: Some(∅) and None paint alike"
        );
    }
}

#[test]
fn emphasis_is_cropped_to_lit_columns() {
    let window = branchy();
    let range = 32..64;
    let plain = window.plain(range.clone());
    let strip = window.strip(&plain, &range, CELL);
    // Only column 4 on the thread: its lane, its nodes and the arcs leaving
    // it are lit; lanes 0..=2 far left and the rail never are.
    let lit: BTreeSet<usize> = [4].into();
    let lits = vec![
        Lit {
            emphasis: Emphasis::Normal,
            thread: Some(&lit),
        };
        plain.len()
    ];
    let backdrop = Backdrop::OffThread;
    let base = backdrop_of(&strip, backdrop);
    let single = legacy(&strip, &lits);
    let top = emphasis_of(&strip, &lits, backdrop).expect("column 4 is lit");
    let cell_width = CELL.0;
    assert!(
        top.frame.columns < base.columns,
        "cropped: {} of {} columns",
        top.frame.columns,
        base.columns
    );
    assert_eq!(top.frame.width, top.frame.columns * cell_width);
    assert_eq!((top.frame.height, top.frame.rows), (base.height, base.rows));
    // Tight: the first and last cell columns each hold a painted pixel.
    let painted_in = |cell: u32| {
        (cell * cell_width..(cell + 1) * cell_width).any(|x| {
            (0..top.frame.height).any(|y| top.frame.pixel(x, y).is_some_and(|pixel| pixel[3] > 0))
        })
    };
    assert!(painted_in(0), "the crop's first column is empty");
    assert!(
        painted_in(top.frame.columns - 1),
        "the crop's last column is empty"
    );
    // Nothing lit is lost: every cell column where the single pass differs
    // from the backdrop lies inside the crop.
    let first = u32::from(top.first_column);
    for x in 0..base.width {
        let differs = (0..base.height).any(|y| base.pixel(x, y) != single.pixel(x, y));
        if differs {
            let cell = x / cell_width;
            assert!(
                (first..first + top.frame.columns).contains(&cell),
                "cell column {cell} is lit but outside the crop {first}..{}",
                first + top.frame.columns
            );
        }
    }
    // And `first_column` places it: the composite is the single pass.
    let tally = tally(
        &single,
        &composite(&base, &top),
        Some(&top),
        backdrop.alpha(),
    );
    assert_eq!(tally.clear_differ, 0);
    assert_eq!(tally.edge_over, 0);
}

#[test]
fn no_paint_no_image() {
    let window = branchy();
    let threads = window.threads();
    let range = 0..32;
    let plain = window.plain(range.clone());
    let strip = window.strip(&plain, &range, CELL);
    // Full: nothing is above the backdrop, whatever the paint says.
    let threaded = lits(&threads[5], range.clone(), Backdrop::OffThread);
    assert_eq!(emphasis_of(&strip, &threaded, Backdrop::Full), None);
    let dimmed = vec![
        Lit {
            emphasis: Emphasis::Dimmed,
            thread: None,
        };
        plain.len()
    ];
    assert_eq!(emphasis_of(&strip, &dimmed, Backdrop::Full), None);
    // OffThread with an empty thread on every row: every element sits at
    // the backdrop's own alpha.
    let empty = BTreeSet::new();
    let off = vec![
        Lit {
            emphasis: Emphasis::Normal,
            thread: Some(&empty),
        };
        plain.len()
    ];
    assert_eq!(emphasis_of(&strip, &off, Backdrop::OffThread), None);
}

/// The PNGs for the user's eye: the same strip and cursor drawn today
/// (`-legacy`) and as backdrop + emphasis (`-composite`), `OffThread`, at
/// his cell, on a dark background. Right: the same lit thread in both,
/// off-thread lanes at the same half strength, no line wider than another.
/// Colour **unverified** (rule 5).
#[test]
fn thread_pngs() {
    let window = branchy();
    let threads = window.threads();
    let range = 32..64;
    let plain = window.plain(range.clone());
    let strip = window.strip(&plain, &range, USER_CELL);
    let lits = lits(&threads[45], range.clone(), Backdrop::OffThread);
    let single = legacy(&strip, &lits);
    let base = backdrop_of(&strip, Backdrop::OffThread);
    let top = emphasis_of(&strip, &lits, Backdrop::OffThread);
    let both = composed(&base, top.as_ref());
    let shots = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/shots/m7");
    // Both checked before failing, so one regeneration writes both.
    let drifted: Vec<String> = [(&single, "legacy"), (&both, "composite")]
        .into_iter()
        .filter_map(|(frame, name)| {
            let png = on_dark(frame);
            assert!(
                png.len() <= 200 * 1024,
                "fix-scroll-thread-{name}.png is {} bytes, over 200 KB",
                png.len()
            );
            graft_fixtures::capture::check_bytes(
                &shots.join(format!("fix-scroll-thread-{name}.png")),
                &png,
            )
            .err()
        })
        .collect();
    assert!(drifted.is_empty(), "{}", drifted.join("\n"));
}

/// A frame over the dark theme's background, as a PNG.
fn on_dark(frame: &RasterFrame) -> Vec<u8> {
    use tiny_skia::{Color, IntSize, Pixmap, PixmapPaint, Transform};
    let size = IntSize::from_wh(frame.width, frame.height)
        .unwrap_or_else(|| panic!("an empty frame has no PNG"));
    let strip = Pixmap::from_vec(frame.pixels.clone(), size)
        .unwrap_or_else(|| panic!("the pixel buffer does not match its size"));
    let mut canvas = Pixmap::new(frame.width, frame.height)
        .unwrap_or_else(|| panic!("no canvas for the background"));
    canvas.fill(Color::from_rgba8(0x1E, 0x1E, 0x2E, 255));
    canvas.draw_pixmap(
        0,
        0,
        strip.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    canvas
        .encode_png()
        .unwrap_or_else(|why| panic!("encoding a PNG: {why}"))
}

/// What one cursor move costs per strip, against today's full strip: a
/// tool, not a check. Run in release by `scripts/fix-scroll-emphasis-cost.sh`,
/// which writes `docs/perf/44-scroll-emphasis-layer.txt`.
#[test]
#[ignore = "timing tool; run in release by scripts/fix-scroll-emphasis-cost.sh"]
fn emphasis_cost() {
    // The branchy window, and one as wide as his 187-column terminal: 93
    // lanes and the rail are 188 cells. Its chains are 93 rows apart, so all
    // 93 are open from row 93 until the first chain ends; the strip measured
    // is the window's fourth (rows 96..128).
    for (name, window, strip_index) in [("branchy", branchy(), 1), ("wide-187", window(320, 93), 3)]
    {
        let threads = window.threads();
        let range = strip_index * STRIP_ROWS..(strip_index + 1) * STRIP_ROWS;
        let plain = window.plain(range.clone());
        let strip = window.strip(&plain, &range, USER_CELL);
        let mut today = Vec::new();
        let mut backdrop = Vec::new();
        let mut emphasis = Vec::new();
        let mut compose = Vec::new();
        let mut cropped = Vec::new();
        // Four rounds over the strip's 32 cursors: 128 samples a layer.
        let cursors = threads.iter().skip(range.start).take(STRIP_ROWS);
        for thread in cursors
            .clone()
            .chain(cursors.clone())
            .chain(cursors.clone())
            .chain(cursors)
        {
            let lits = lits(thread, range.clone(), Backdrop::OffThread);
            let start = Instant::now();
            std::hint::black_box(legacy(&strip, &lits));
            today.push(start.elapsed().as_micros());
            let start = Instant::now();
            let base = std::hint::black_box(backdrop_of(&strip, Backdrop::OffThread));
            backdrop.push(start.elapsed().as_micros());
            let start = Instant::now();
            let top = std::hint::black_box(emphasis_of(&strip, &lits, Backdrop::OffThread));
            emphasis.push(start.elapsed().as_micros());
            if let Some(top) = &top {
                cropped.push(u128::from(top.frame.columns));
                let start = Instant::now();
                std::hint::black_box(composite(&base, top));
                compose.push(start.elapsed().as_micros());
            }
        }
        let columns = 2 * window.lane_columns;
        for (what, times) in [
            ("today-full-strip", &mut today),
            ("backdrop", &mut backdrop),
            ("emphasis", &mut emphasis),
            ("composite", &mut compose),
        ] {
            times.sort_unstable();
            eprintln!(
                "emphasis-cost: {name} {columns} cells 32 rows {}x{} {what}: n={} p50={} µs p99={} µs max={} µs",
                USER_CELL.0,
                USER_CELL.1,
                times.len(),
                percentile(times, 50),
                percentile(times, 99),
                times.last().copied().unwrap_or(0),
            );
        }
        cropped.sort_unstable();
        eprintln!(
            "emphasis-cost: {name} emphasis width p50={} p99={} of {columns} cells",
            percentile(&cropped, 50),
            percentile(&cropped, 99),
        );
    }
}

/// Nearest-rank percentile of sorted samples.
fn percentile(sorted: &[u128], percent: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (sorted.len() * percent).div_ceil(100).max(1);
    sorted[rank - 1]
}
