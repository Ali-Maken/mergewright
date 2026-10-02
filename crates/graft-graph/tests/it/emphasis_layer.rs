//! The selection's paint as its own layer (ADR-0053, m7-power-tools/44).
//!
//! A strip is drawn from layout alone at one backdrop alpha; the selection's
//! thread is a second, cropped image drawn at `(a − b) / (1 − b)` so that,
//! composited over the strip, it gives ADR-0024's alpha `a` again. The
//! single pass this was once held against is gone (m7-power-tools/46b); the
//! raster snapshots (`raster.rs`) now pin the composite's pixels. Colour itself stays unverified (CLAUDE.md rule 5): the
//! PNGs `thread_pngs` writes are for the user's eye.

use std::collections::BTreeSet;
use std::ops::Range;
use std::time::Instant;

use graft_core::capabilities::Rgb;
use graft_core::raster::RasterFrame;
use graft_graph::ancestry::{Ancestry, Kin};
use graft_graph::fold::Toward;
use graft_graph::identity::BranchIdentity;
use graft_graph::raster::cache::{ByteBudget, STRIP_ROWS, StripCache, lit_hash};
use graft_graph::raster::{
    Backdrop, Emphasis, Emphasised, Lit, Marker, PlainRow, PlainStrip, Ring, Selection, Stub,
    composite, rasterise_backdrop, rasterise_emphasis,
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

fn composed(base: &RasterFrame, top: Option<&Emphasised>) -> RasterFrame {
    top.map_or_else(|| base.clone(), |top| composite(base, top))
}

/// `Backdrop::lowest` reads the paint (46 Signatures): every case in its doc.
#[test]
fn lowest_backdrop_table() {
    let empty = BTreeSet::new();
    let some = BTreeSet::from([0usize, 2]);
    let lit = |emphasis, thread| Lit { emphasis, thread };
    let normal = lit(Emphasis::Normal, None);
    let dimmed = lit(Emphasis::Dimmed, None);
    let cases: [(&str, Vec<Lit<'_>>, Backdrop); 7] = [
        ("empty", vec![], Backdrop::Full),
        ("all None + Normal", vec![normal; 3], Backdrop::Full),
        (
            "one Some(∅)",
            vec![normal, lit(Emphasis::Normal, Some(&empty)), normal],
            Backdrop::OffThread,
        ),
        (
            "one None + Dimmed alone",
            vec![normal, dimmed],
            Backdrop::Faded,
        ),
        (
            "None + Dimmed beside threaded rows",
            vec![
                lit(Emphasis::Normal, Some(&some)),
                dimmed,
                lit(Emphasis::Normal, Some(&empty)),
            ],
            Backdrop::Faded,
        ),
        (
            "Some(_) + Dimmed only",
            vec![
                lit(Emphasis::Dimmed, Some(&some)),
                lit(Emphasis::Dimmed, Some(&empty)),
            ],
            Backdrop::OffThread,
        ),
        (
            "threaded rows and plain rows",
            vec![normal, lit(Emphasis::Normal, Some(&some))],
            Backdrop::OffThread,
        ),
    ];
    for (name, lits, want) in cases {
        assert_eq!(Backdrop::lowest(lits), want, "{name}");
    }
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

/// `lit_hash` leaves a threaded row's own emphasis out (cache.rs `lit_hash`):
/// as 30b's `layout_hash`). That is sound only if, under `Some(thread)`, the
/// emphasis reaches no pixel of the emphasis pass or the composite — else a
/// Focus kin change would be a wrong cache hit. Flipping it on one threaded
/// row (its own lit set, and `Some(∅)`) must leave the hash and the emphasis
/// bytes alike, and `StripCache::composited` for the flipped selection —
/// drawn or a hit under the shared key — must equal the unflipped selection
/// drawn directly. On a `thread: None` row the same flip must change the
/// hash. The first row is always one of the flipped rows: its incoming edges
/// come from the row above and take the first row's paint.
#[test]
fn a_threaded_rows_emphasis_draws_nothing() {
    let window = branchy();
    let threads = window.threads();
    let cases: Vec<(Range<usize>, Backdrop)> = window
        .strips()
        .into_iter()
        .flat_map(|range| [Backdrop::OffThread, Backdrop::Faded].map(|b| (range.clone(), b)))
        .collect();
    // One thread per (strip, backdrop): every cursor of the window in each.
    let non_empty_flips: usize = std::thread::scope(|scope| {
        let workers: Vec<_> = cases
            .iter()
            .map(|(range, backdrop)| {
                let (window, threads) = (&window, &threads);
                scope.spawn(move || flip_every_cursor(window, threads, range, *backdrop))
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| {
                worker
                    .join()
                    .unwrap_or_else(|why| std::panic::resume_unwind(why))
            })
            .sum()
    });
    assert!(non_empty_flips > 0, "no flip was on a non-empty thread set");
}

/// The other emphasis.
fn flipped(emphasis: Emphasis) -> Emphasis {
    match emphasis {
        Emphasis::Normal => Emphasis::Dimmed,
        Emphasis::Dimmed => Emphasis::Normal,
    }
}

/// `a_threaded_rows_emphasis_draws_nothing` over one strip and backdrop,
/// every cursor. Returns how many flips were on a non-empty thread set.
fn flip_every_cursor(
    window: &Window,
    threads: &[Ancestry],
    range: &Range<usize>,
    backdrop: Backdrop,
) -> usize {
    let plain = window.plain(range.clone());
    let strip = window.strip(&plain, range, CELL);
    let under = backdrop_of(&strip, backdrop);
    let mut cache = StripCache::with_budget(ByteBudget(usize::MAX), 4 * threads.len());
    let empty = BTreeSet::new();
    let hash = |rows: &[Lit<'_>]| lit_hash(&Selection { rows, backdrop });
    let mut non_empty_flips = 0;
    for (cursor, thread) in threads.iter().enumerate() {
        let base = lits(thread, range.clone(), backdrop);
        let base_emphasis = emphasis_of(&strip, &base, backdrop);
        let middle = (cursor + 7) % base.len();
        for row in [0, middle] {
            for set in [base[row].thread, Some(&empty)] {
                let mut before = base.clone();
                before[row].thread = set;
                let mut after = before.clone();
                after[row].emphasis = flipped(before[row].emphasis);
                if set.is_some_and(|set| !set.is_empty()) {
                    non_empty_flips += 1;
                }
                let case =
                    format!("strip {range:?}, cursor {cursor}, {backdrop:?}, row {row}, {set:?}");
                assert_eq!(hash(&before), hash(&after), "{case}: lit_hash moved");
                let before_emphasis = if before == base {
                    base_emphasis.clone()
                } else {
                    emphasis_of(&strip, &before, backdrop)
                };
                assert_eq!(
                    before_emphasis,
                    emphasis_of(&strip, &after, backdrop),
                    "{case}: emphasis bytes moved"
                );
                let cached = cache
                    .composited(
                        range.start,
                        &strip,
                        &Selection {
                            rows: &after,
                            backdrop,
                        },
                    )
                    .unwrap_or_else(|why| panic!("{case}: composited: {why}"));
                assert!(
                    *cached == composed(&under, before_emphasis.as_ref()),
                    "{case}: composited bytes moved"
                );
            }
        }
        let mut unthreaded = base.clone();
        unthreaded[middle].thread = None;
        let mut unthreaded_flipped = unthreaded.clone();
        unthreaded_flipped[middle].emphasis = flipped(unthreaded[middle].emphasis);
        assert_ne!(
            hash(&unthreaded),
            hash(&unthreaded_flipped),
            "strip {range:?}, cursor {cursor}, {backdrop:?}: a None row's emphasis left the hash"
        );
    }
    non_empty_flips
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
    // The lit lane's own cell column lies inside the crop, and
    // `first_column` places the image over it: composited, that column is
    // brighter than the backdrop alone.
    let first = u32::from(top.first_column);
    let lane_cell = 4 * graft_graph::raster::CELLS_PER_LANE;
    assert!(
        (first..first + top.frame.columns).contains(&lane_cell),
        "lane 4's cell column {lane_cell} is outside the crop {first}..{}",
        first + top.frame.columns
    );
    let both = composite(&base, &top);
    let lit_here = (lane_cell * cell_width..(lane_cell + 1) * cell_width)
        .any(|x| (0..base.height).any(|y| both.pixel(x, y) != base.pixel(x, y)));
    assert!(
        lit_here,
        "the composite left lane 4's column as the backdrop drew it"
    );
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

/// The PNG for the user's eye: one strip and cursor drawn as backdrop +
/// emphasis (`-composite`), `OffThread`, at his cell, on a dark background.
/// Right: the lit thread at full strength, off-thread lanes at half, no line
/// wider than another.
/// Colour **unverified** (rule 5).
#[test]
fn thread_pngs() {
    let window = branchy();
    let threads = window.threads();
    let range = 32..64;
    let plain = window.plain(range.clone());
    let strip = window.strip(&plain, &range, USER_CELL);
    let lits = lits(&threads[45], range.clone(), Backdrop::OffThread);
    let base = backdrop_of(&strip, Backdrop::OffThread);
    let top = emphasis_of(&strip, &lits, Backdrop::OffThread);
    let both = composed(&base, top.as_ref());
    let shots = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/shots/m7");
    let png = on_dark(&both);
    assert!(
        png.len() <= 200 * 1024,
        "fix-scroll-thread-composite.png is {} bytes, over 200 KB",
        png.len()
    );
    if let Err(drift) =
        graft_fixtures::capture::check_bytes(&shots.join("fix-scroll-thread-composite.png"), &png)
    {
        panic!("{drift}");
    }
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

/// What one cursor move costs per strip, by layer: a tool, not a check. Run in release by `scripts/fix-scroll-emphasis-cost.sh`,
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
