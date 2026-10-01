//! The pixel rasteriser on real corpus shapes (m1-the-graph/22).
//!
//! Buffers are pinned at an 8×16 cell (PROMPT.md §7.2 item 3) and
//! snapshotted as text: one character per pixel, `' '` for alpha 0, `.` for
//! a faint anti-aliased edge, and a letter naming the palette slot of
//! anything more solid. A text snapshot survives a sub-unit rounding change
//! a byte hash would not, and a reviewer can read it. Whether the strips
//! *look* good is judged by eye from the PNGs `write_shots` produces.

use std::collections::BTreeSet;
use std::time::Instant;

use graft_core::capabilities::Rgb;
use graft_core::raster::RasterFrame;
use graft_fixtures::Shape;
use graft_graph::fold::Toward;
use graft_graph::identity::BranchIdentity;
use graft_graph::layout::layout;
use graft_graph::raster::{
    Backdrop, Emphasis, Lit, Marker, PlainRow, PlainStrip, RasterError, Ring, Selection, Stub,
    composite, rasterise_backdrop, rasterise_emphasis,
};
use graft_graph::row::Row;

use super::corpus::{open_with_seeds, walk_everything};

const SNAPSHOT_CELL: (u32, u32) = (8, 16);
/// The user's measured cell (ADR-0012's Ghostty bench).
const USER_CELL: (u32, u32) = (17, 37);
const SHAPES: [&str; 3] = ["foxtrot-merge", "criss-cross-merge", "octopus-merge"];

/// Paul Tol's hues, as the UI theme's lane palette holds them.
const PALETTE: [Rgb; 12] = [
    rgb(0x00, 0x77, 0xBB),
    rgb(0xEE, 0x88, 0x66),
    rgb(0xAA, 0x33, 0x77),
    rgb(0x77, 0xAA, 0xDD),
    rgb(0xDD, 0xCC, 0x77),
    rgb(0x00, 0x99, 0x88),
    rgb(0xCC, 0xBB, 0x44),
    rgb(0xCC, 0x33, 0x11),
    rgb(0xFF, 0xAA, 0xBB),
    rgb(0xCC, 0x66, 0x77),
    rgb(0x88, 0xCC, 0xEE),
    rgb(0x99, 0x99, 0x33),
];
const MUTED: Rgb = rgb(0x88, 0x88, 0x88);

const fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}

fn rows_of(name: &str) -> Vec<Row> {
    let shape = *Shape::ALL
        .iter()
        .find(|shape| shape.name() == name)
        .unwrap_or_else(|| panic!("no corpus shape named {name}"));
    let dir = graft_fixtures::try_borrow(shape)
        .unwrap_or_else(|why| panic!("borrowing the {shape} fixture: {why}"));
    let (handle, seeds) = open_with_seeds(shape, &dir);
    let (commits, _) = walk_everything(&handle, shape);
    layout(&commits, seeds)
}

fn width_of(rows: &[Row]) -> usize {
    rows.iter()
        .map(|row| row.lanes.len().max(row.lane + 1))
        .max()
        .unwrap_or(0)
}

fn strip<'a>(rows: &'a [PlainRow<'a>], lane_columns: usize, cell: (u32, u32)) -> PlainStrip<'a> {
    PlainStrip {
        rows,
        above: None,
        lane_columns,
        marker: None,
        cell_width: cell.0,
        cell_height: cell.1,
        palette: &PALETTE,
        muted: MUTED,
    }
}

fn plain(rows: &[Row]) -> Vec<PlainRow<'_>> {
    rows.iter().map(PlainRow::of).collect()
}

/// A strip as the app shows it under a selection: the backdrop at the
/// lowest alpha `lits` paint (`Backdrop::lowest`), with the emphasis layer
/// composited over it when there is one (ADR-0053).
pub(crate) fn layered(
    strip: &PlainStrip<'_>,
    lits: &[Lit<'_>],
) -> Result<RasterFrame, RasterError> {
    let backdrop = Backdrop::lowest(lits.iter().copied());
    let base = rasterise_backdrop(strip, backdrop)?;
    let sel = Selection {
        rows: lits,
        backdrop,
    };
    Ok(match rasterise_emphasis(strip, &sel)? {
        Some(top) => composite(&base, &top),
        None => base,
    })
}

fn render(rows: &[Row], cell: (u32, u32)) -> RasterFrame {
    let raster_rows = plain(rows);
    // No selection: every row `Lit::default()`, so the `Full` backdrop and no
    // emphasis layer.
    let lits = vec![Lit::default(); rows.len()];
    layered(&strip(&raster_rows, width_of(rows), cell), &lits)
        .unwrap_or_else(|why| panic!("rasterising a corpus strip: {why}"))
}

/// The palette slot nearest a premultiplied pixel's colour.
fn slot(pixel: [u8; 4]) -> usize {
    let alpha = u32::from(pixel[3]);
    let channel = |value: u8| i64::from(u32::from(value) * 255 / alpha);
    let (red, green, blue) = (channel(pixel[0]), channel(pixel[1]), channel(pixel[2]));
    (0..PALETTE.len())
        .min_by_key(|&index| {
            let hue = PALETTE[index];
            (red - i64::from(hue.red)).pow(2)
                + (green - i64::from(hue.green)).pow(2)
                + (blue - i64::from(hue.blue)).pow(2)
        })
        .unwrap_or_else(|| panic!("the palette is empty"))
}

fn art(frame: &RasterFrame) -> String {
    let mut text = String::new();
    for y in 0..frame.height {
        for x in 0..frame.width {
            let pixel = frame
                .pixel(x, y)
                .unwrap_or_else(|| panic!("({x}, {y}) is outside the frame"));
            text.push(match pixel[3] {
                0 => ' ',
                1..128 => '.',
                _ => char::from(
                    b'a' + u8::try_from(slot(pixel))
                        .unwrap_or_else(|why| panic!("slot overflows a letter: {why}")),
                ),
            });
        }
        text.push_str("|\n");
    }
    text
}

#[test]
fn strips_snapshot_at_a_pinned_cell() {
    for name in SHAPES {
        let frame = render(&rows_of(name), SNAPSHOT_CELL);
        insta::assert_snapshot!(format!("raster-{name}"), art(&frame));
    }
}

#[test]
fn colour_never_exceeds_alpha_and_untouched_pixels_are_clear() {
    for name in SHAPES {
        let rows = rows_of(name);
        let (cell_width, cell_height) = SNAPSHOT_CELL;
        let frame = render(&rows, SNAPSHOT_CELL);
        for chunk in frame.pixels.as_chunks::<4>().0 {
            assert!(
                chunk[..3].iter().all(|&channel| channel <= chunk[3]),
                "{name}: {chunk:?} is not premultiplied"
            );
        }
        // Rows with no bend draw only vertical lines and the node, so the
        // centre of every gap cell between two lanes touches nothing.
        for (index, row) in rows.iter().enumerate() {
            let bends = !row.incoming.is_empty()
                || row.outgoing.iter().any(|&(column, _)| column != row.lane);
            if bends {
                continue;
            }
            let top = u32::try_from(index).unwrap() * cell_height;
            for lane in 0..u32::try_from(width_of(&rows)).unwrap() {
                let x = (lane * 2 + 1) * cell_width + cell_width / 2;
                for y in top..top + cell_height {
                    assert_eq!(
                        frame.pixel(x, y),
                        Some([0, 0, 0, 0]),
                        "{name}: row {index}, gap right of lane {lane}, y {y} should be untouched"
                    );
                }
            }
        }
    }
}

#[test]
fn a_fork_is_an_antialiased_curve_not_an_elbow() {
    let rows = rows_of("octopus-merge");
    let merge = &rows[0];
    assert!(
        merge.outgoing.iter().any(|&(column, _)| column == 1),
        "octopus row 0 opens lane 1: {merge:?}"
    );
    let (cell_width, cell_height) = USER_CELL;
    let frame = render(std::slice::from_ref(merge), USER_CELL);
    let (node_x, lane_x) = (cell_width / 2, 2 * cell_width + cell_width / 2);
    // Below 0.3 of the band the arcs to lanes 2 and 3 have already left for
    // the right, so this window holds lane 1's arc alone.
    let mut centroids = Vec::new();
    let mut faint = 0;
    for y in cell_height / 2 + cell_height * 3 / 10..cell_height {
        let (mut sum, mut weight) = (0u64, 0u64);
        for x in node_x + cell_width / 2..lane_x + cell_width / 2 {
            let alpha = u64::from(frame.pixel(x, y).unwrap()[3]);
            if (1..255).contains(&alpha) {
                faint += 1;
            }
            sum += u64::from(x) * alpha;
            weight += alpha;
        }
        assert!(weight > 0, "the arc crosses y {y}");
        centroids.push(sum / weight);
    }
    assert!(
        centroids.windows(2).all(|pair| pair[0] <= pair[1]),
        "the arc only ever moves toward its lane: {centroids:?}"
    );
    let mut distinct = centroids.clone();
    distinct.dedup();
    assert!(
        distinct.len() >= 3,
        "a curve, not a vertical drop after an elbow: {centroids:?}"
    );
    assert!(
        centroids[0] + 2 < u64::from(lane_x),
        "still bending at the window's top: {centroids:?}"
    );
    assert!(faint > 0, "anti-aliased edges leave partial alpha");
}

const RINGS: [Ring; 6] = [
    Ring::Commit,
    Ring::Merge,
    Ring::Root,
    Ring::Head,
    Ring::Stash,
    Ring::Uncommitted,
];

/// A lone node: the base commit with every edge taken away.
fn lone(rows: &[Row]) -> Row {
    let mut row = rows
        .last()
        .unwrap_or_else(|| panic!("an empty fixture"))
        .clone();
    row.lane = 0;
    row.incoming.clear();
    row.outgoing.clear();
    row.lanes.truncate(1);
    row
}

fn ring_mask(row: &Row, ring: Ring, colour: Rgb) -> Vec<bool> {
    let raster_row = [PlainRow {
        ring,
        ..PlainRow::of(row)
    }];
    let palette = [colour];
    let frame = rasterise_backdrop(
        &PlainStrip {
            palette: &palette,
            ..strip(&raster_row, 1, USER_CELL)
        },
        Backdrop::Full,
    )
    .unwrap_or_else(|why| panic!("rasterising {ring:?}: {why}"));
    frame
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| pixel[3] > 127)
        .collect()
}

#[test]
fn rings_differ_in_shape_whatever_the_colour() {
    let row = lone(&rows_of("octopus-merge"));
    let masks: Vec<Vec<bool>> = RINGS
        .iter()
        .map(|&ring| {
            let blue = ring_mask(&row, ring, rgb(0x00, 0x77, 0xBB));
            let sand = ring_mask(&row, ring, rgb(0xDD, 0xCC, 0x77));
            assert_eq!(blue, sand, "{ring:?}: shape must not depend on colour");
            assert!(blue.iter().any(|&on| on), "{ring:?} draws something");
            blue
        })
        .collect();
    for (a, mask_a) in masks.iter().enumerate() {
        for (b, mask_b) in masks.iter().enumerate().skip(a + 1) {
            assert_ne!(
                mask_a, mask_b,
                "{:?} and {:?} look alike",
                RINGS[a], RINGS[b]
            );
        }
    }
    // The centre pixel: hollow rings are hollow, filled ones filled.
    let (width, height) = (USER_CELL.0 * 2, USER_CELL.1);
    let centre = usize::try_from((height / 2) * width + USER_CELL.0 / 2).unwrap();
    let hollow: Vec<bool> = masks.iter().map(|mask| !mask[centre]).collect();
    assert_eq!(hollow, [false, false, true, false, false, true]);
}

/// Emphasis fades a row's alpha; it never widens a stroke (ADR-0024): a
/// dimmed row paints no more pixels than normal, only fainter ones.
#[test]
fn emphasis_fades_without_thickening_the_strokes() {
    let rows = rows_of("foxtrot-merge");
    let columns = width_of(&rows);
    let draw = |emphasis| {
        let raster_rows = plain(&rows);
        let lits = vec![
            Lit {
                emphasis,
                thread: None,
            };
            rows.len()
        ];
        layered(&strip(&raster_rows, columns, SNAPSHOT_CELL), &lits).unwrap()
    };
    let painted = |frame: &RasterFrame| {
        frame
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] > 0)
            .count()
    };
    let (normal, dimmed) = (draw(Emphasis::Normal), draw(Emphasis::Dimmed));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // FADED_ALPHA is 0.0..=1.0.
    let max_dimmed_alpha = (graft_graph::raster::FADED_ALPHA * 255.0).round() as u8;
    assert!(
        dimmed
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[3] <= max_dimmed_alpha)
    );
    assert!(
        normal
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] == 255)
    );
    // Same shape drawn, only fainter: the painted footprint does not grow.
    assert_eq!(painted(&dimmed), painted(&normal));
}

/// The longest run of `alpha > 0.5` pixels across one pixel row.
fn solid_run(frame: &RasterFrame, y: u32) -> usize {
    let mut best = 0;
    let mut run = 0;
    for x in 0..frame.width {
        let alpha = frame.pixel(x, y).unwrap_or_else(|| panic!("({x}, {y})"))[3];
        if alpha > 127 {
            run += 1;
            best = best.max(run);
        } else {
            run = 0;
        }
    }
    best
}

/// The highest alpha seen in one pixel row over `x_range`.
fn peak_alpha(frame: &RasterFrame, y: u32, x_range: std::ops::Range<u32>) -> u8 {
    x_range
        .filter_map(|x| frame.pixel(x, y))
        .map(|pixel| pixel[3])
        .max()
        .unwrap_or(0)
}

/// ADR-0024's cause and fix, at the user's own cell: the ancestry thread
/// used to widen lane 0 to a slab (`stroke * 1.6`) because the cursor
/// starts on the top row, so lane 0 is on the thread by default (`ticket
/// 42`'s finding). Now the thread is contrast only: on-thread and
/// off-thread lines are the same width, and off-thread is about half the
/// on-thread alpha.
#[test]
fn the_thread_is_alpha_not_a_wider_stroke() {
    use std::collections::BTreeSet;

    let rows = super::raster_pixels::synthetic(20, 5);
    let lane_columns = width_of(&rows);
    let lit: BTreeSet<usize> = [0].into_iter().collect();
    // Rows 8..16 at row 11 (index 3) are a plain straight-through band for
    // every lane (not a multiple of 7, so no merge arc; not the strip's own
    // edge row).
    let draw = |thread: Option<&BTreeSet<usize>>| {
        let raster_rows = plain(&rows[8..16]);
        let lits = vec![
            Lit {
                emphasis: Emphasis::Normal,
                thread,
            };
            raster_rows.len()
        ];
        layered(
            &PlainStrip {
                rows: &raster_rows,
                above: Some(&rows[7]),
                lane_columns,
                marker: None,
                cell_width: USER_CELL.0,
                cell_height: USER_CELL.1,
                palette: &PALETTE,
                muted: MUTED,
            },
            &lits,
        )
        .unwrap()
    };
    // Near the band's top edge: clear of the node disc drawn at its mid.
    let y = 3 * USER_CELL.1 + 2;
    let off = draw(None);
    let on = draw(Some(&lit));
    for frame in [&off, &on] {
        assert!(
            solid_run(frame, y) <= 3,
            "a lane's solid (alpha > 0.5) run must read as a line, not a slab: {}",
            solid_run(frame, y)
        );
    }
    // Lane 0 (x ~8) is on the thread; lane 1 (x ~42) is not.
    let on_thread_peak = peak_alpha(&on, y, 0..20);
    let off_thread_peak = peak_alpha(&on, y, 35..50);
    assert!(
        (f32::from(off_thread_peak) - f32::from(on_thread_peak) / 2.0).abs() <= 0.05 * 255.0,
        "off-thread ({off_thread_peak}) should be about half on-thread ({on_thread_peak})"
    );
}

/// 40's strip: one lane of four commits beside the rail (column 1). Row 0 is
/// a merge whose second parent folded (`Below`) and which also lost a forked
/// branch (`Above`, listed second, so not drawn); row 2 had a branch fork off
/// above it that folded (`Above`).
fn folded_rows() -> Vec<Row> {
    let base = lone(&rows_of("foxtrot-merge"));
    (0..4)
        .map(|index| {
            let mut row = base.clone();
            if index < 3 {
                row.outgoing = vec![(0, base.commit.id)];
            }
            row
        })
        .collect()
}

fn topic() -> BranchIdentity {
    BranchIdentity::of_name(b"topic")
}

fn feature() -> BranchIdentity {
    BranchIdentity::of_name(b"feature")
}

fn folded_raster_rows(rows: &[Row], stubs: bool) -> Vec<PlainRow<'_>> {
    let below = Stub {
        identity: topic(),
        toward: Toward::Below,
    };
    let above = Stub {
        identity: feature(),
        toward: Toward::Above,
    };
    rows.iter()
        .enumerate()
        .map(|(index, row)| PlainRow {
            ring: if index == 0 {
                Ring::Merge
            } else {
                Ring::Commit
            },
            stubs: match (stubs, index) {
                (true, 0) => vec![below, above],
                (true, 2) => vec![above],
                _ => Vec::new(),
            },
            ..PlainRow::of(row)
        })
        .collect()
}

fn folded_frame(rows: &[Row], above: Option<&Row>, stubs: bool, cell: (u32, u32)) -> RasterFrame {
    let raster_rows = folded_raster_rows(rows, stubs);
    rasterise_backdrop(
        &PlainStrip {
            above,
            marker: Some(Marker { column: 1 }),
            ..strip(&raster_rows, 2, cell)
        },
        Backdrop::Full,
    )
    .unwrap_or_else(|why| panic!("rasterising the folded strip: {why}"))
}

#[test]
fn folded_merge_snapshot() {
    let rows = folded_rows();
    insta::assert_snapshot!(
        "raster-folded-merge",
        art(&folded_frame(&rows, None, true, SNAPSHOT_CELL))
    );
}

/// Pixels a strip's stubs changed: `(x, y, pixel)` of the drawing with stubs
/// wherever it differs from the same strip drawn without them.
fn tail_pixels(rows: &[Row]) -> Vec<(u32, u32, [u8; 4])> {
    let with = folded_frame(rows, None, true, USER_CELL);
    let without = folded_frame(rows, None, false, USER_CELL);
    (0..with.height)
        .flat_map(|y| (0..with.width).map(move |x| (x, y)))
        .filter_map(|(x, y)| {
            let at = |frame: &RasterFrame| {
                frame
                    .pixel(x, y)
                    .unwrap_or_else(|| panic!("({x}, {y}) is outside the frame"))
            };
            let pixel = at(&with);
            (pixel != at(&without)).then_some((x, y, pixel))
        })
        .collect()
}

#[test]
fn a_folded_edge_is_a_tail_that_fades_out_before_the_next_column() {
    assert_ne!(topic().slot(PALETTE.len()), feature().slot(PALETTE.len()));
    let rows = folded_rows();
    let (cell_width, cell_height) = USER_CELL;
    let tail = tail_pixels(&rows);
    // The next column's centre line is lane 1's centre, x 42.5, where the
    // rail is: a tail pixel spans x..x+1, so it must start left of 42.
    let next_centre = cell_width * 5 / 2;
    for band in [0, 2] {
        let top = band * cell_height;
        let mid = top + cell_height / 2;
        let own: Vec<_> = tail
            .iter()
            .filter(|&&(_, y, _)| (top..top + cell_height).contains(&y))
            .collect();
        assert!(!own.is_empty(), "band {band} draws a tail");
        for &&(x, y, _) in &own {
            assert!(
                x < next_centre,
                "band {band}: tail pixel at x {x} reaches the next column"
            );
            // Row 0's first stub leads down, row 2's up; one tail a row.
            if band == 0 {
                assert!(y + 2 >= mid, "band 0's tail rises to y {y}");
            } else {
                assert!(y <= mid + 2, "band 2's tail falls to y {y}");
            }
        }
        let peak = |column: u32| {
            own.iter()
                .filter(|&&&(x, _, _)| x == column)
                .map(|&&(_, _, pixel)| pixel[3])
                .max()
                .unwrap_or(0)
        };
        let first = own.iter().map(|&&(x, _, _)| x).min().unwrap();
        let last = own.iter().map(|&&(x, _, _)| x).max().unwrap();
        assert!(
            last - first >= cell_width,
            "band {band}: a tail about a lane long, {first}..={last}"
        );
        assert!(
            f32::from(peak(last)) < 0.1 * f32::from(peak(first)),
            "band {band}: the tail fades, {} at x {first} to {} at x {last}",
            peak(first),
            peak(last)
        );
        let solid = own
            .iter()
            .max_by_key(|&&&(_, _, pixel)| pixel[3])
            .map(|&&(_, _, pixel)| pixel)
            .unwrap();
        let expected = if band == 0 { topic() } else { feature() };
        assert_eq!(
            slot(solid),
            expected.slot(PALETTE.len()),
            "band {band}: the first stub's colour"
        );
    }
}

#[test]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Sample rows inside a 37 px band.
fn the_rail_is_a_grey_dotted_line_whose_dashes_never_jump() {
    let rows = folded_rows();
    let (_, cell_height) = USER_CELL;
    let frame = folded_frame(&rows, None, true, USER_CELL);
    // Lane 1's centre line is x 42.5; the rail takes the whole pixel 42.
    let x = 42;
    let alpha = |frame: &RasterFrame, y: u32| frame.pixel(x, y).unwrap()[3];
    for y in 0..frame.height {
        let [red, green, blue, _] = frame.pixel(x, y).unwrap();
        assert!(
            red.abs_diff(green) <= 1 && green.abs_diff(blue) <= 1,
            "rail pixel at y {y} has colour: {red} {green} {blue}"
        );
    }
    let third = f64::from(cell_height) / 3.0;
    for band in 0..4 {
        let top = f64::from(band * cell_height);
        for dash in 0..3 {
            let start = top + f64::from(dash) * third;
            let on = (start + third / 4.0) as u32;
            let off = (start + third * 0.75) as u32;
            assert_eq!(
                alpha(&frame, on),
                255,
                "band {band} dash {dash} is on at y {on}"
            );
            assert_eq!(
                alpha(&frame, off),
                0,
                "band {band} dash {dash} is off at y {off}"
            );
        }
    }
    // Every band repeats the one before, and a strip starting mid-history
    // draws its band 0 as the whole strip drew that row.
    for y in 0..cell_height {
        assert_eq!(
            alpha(&frame, y),
            alpha(&frame, y + cell_height),
            "row edge at y {y}"
        );
    }
    let later = folded_frame(&rows[2..], Some(&rows[1]), true, USER_CELL);
    for y in 0..cell_height {
        assert_eq!(
            alpha(&later, y),
            alpha(&frame, y + 2 * cell_height),
            "strip edge at y {y}"
        );
    }
}

/// Acceptance: render time for a 32-row strip at the user's cell. Reported,
/// not asserted — the budget belongs to the frame-budget ticket.
#[test]
fn reports_render_time_for_a_32_row_strip() {
    let rows: Vec<Row> = SHAPES
        .iter()
        .flat_map(|name| rows_of(name))
        .cycle()
        .take(32)
        .collect();
    let raster_rows = plain(&rows);
    let strip = strip(&raster_rows, width_of(&rows), USER_CELL);
    let mut times: Vec<u128> = (0..9)
        .map(|_| {
            let start = Instant::now();
            let frame = rasterise_backdrop(&strip, Backdrop::Full).unwrap();
            assert_eq!(frame.height, 32 * USER_CELL.1);
            start.elapsed().as_micros()
        })
        .collect();
    times.sort_unstable();
    eprintln!(
        "raster 32 rows × {} lanes at 17×37: median {} µs, min {} µs ({} build)",
        width_of(&rows),
        times[4],
        times[0],
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}

/// The eyeball PNGs under `docs/shots/`, **asserted** against what the
/// rasteriser draws now.
///
/// It used only to write them, from an `#[ignore]`d test — which is how 42
/// captures drifted across four milestones with nothing going red
/// (m5-history-and-ghost-preview/40). Regenerate on purpose with
/// `GRAFT_UPDATE_SHOTS=1`; that run is red too, so a rewritten file cannot
/// pass unnoticed.
///
/// The comparison is on **pixels**, not on the PNG bytes: the encoder is
/// `tiny-skia`'s, and its output could change with a version bump without one
/// pixel of the picture moving. Pixels are what the user's eye judges, so
/// pixels are what this asserts.
#[test]
fn write_shots() {
    let shots = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/shots");
    for name in SHAPES {
        let frame = render(&rows_of(name), USER_CELL);
        save(&frame, &shots.join(format!("22-raster-{name}.png")), None);
        save(
            &frame,
            &shots.join(format!("22-raster-{name}-on-dark.png")),
            Some((0x1E, 0x1E, 0x2E)),
        );
    }
    let row = lone(&rows_of("octopus-merge"));
    // Each ring in its own lane, left to right in `RINGS` order.
    let rings: Vec<Row> = (0..RINGS.len())
        .map(|lane| {
            let mut shifted = row.clone();
            shifted.lane = lane;
            shifted.lanes = vec![None; lane];
            shifted.lanes.push(row.lanes[0]);
            shifted
        })
        .collect();
    let raster_rows: Vec<PlainRow<'_>> = rings
        .iter()
        .zip(RINGS)
        .map(|(row, ring)| PlainRow {
            ring,
            ..PlainRow::of(row)
        })
        .collect();
    let frame =
        rasterise_backdrop(&strip(&raster_rows, RINGS.len(), USER_CELL), Backdrop::Full).unwrap();
    save(
        &frame,
        &shots.join("22-raster-rings-on-dark.png"),
        Some((0x1E, 0x1E, 0x2E)),
    );
    // 42: the same ring shapes, at 42's stroke width (ADR-0024).
    save(
        &frame,
        &shots.join("42-raster-rings-on-dark.png"),
        Some((0x1E, 0x1E, 0x2E)),
    );
    write_thread_shot(&shots);
    // 40: folded edges as fading tails beside the dotted rail (ADR-0022).
    let rows = folded_rows();
    save(
        &folded_frame(&rows, None, true, USER_CELL),
        &shots.join("40-raster-folded-on-dark.png"),
        Some((0x1E, 0x1E, 0x2E)),
    );
}

/// 42: lane 0 on the selection's thread among four others — a line to
/// follow, brighter than the rest, never a slab (ADR-0024).
fn write_thread_shot(shots: &std::path::Path) {
    let thread_rows = super::raster_pixels::synthetic(20, 5);
    let lit: BTreeSet<usize> = [0].into_iter().collect();
    let raster_rows = plain(&thread_rows[4..16]);
    let lits = vec![
        Lit {
            emphasis: Emphasis::Normal,
            thread: Some(&lit),
        };
        raster_rows.len()
    ];
    let thread_frame = layered(
        &PlainStrip {
            rows: &raster_rows,
            above: Some(&thread_rows[3]),
            lane_columns: width_of(&thread_rows),
            marker: None,
            cell_width: USER_CELL.0,
            cell_height: USER_CELL.1,
            palette: &PALETTE,
            muted: MUTED,
        },
        &lits,
    )
    .unwrap_or_else(|why| panic!("thread shot: {why}"));
    save(
        &thread_frame,
        &shots.join("42-raster-thread-on-dark.png"),
        Some((0x1E, 0x1E, 0x2E)),
    );
}

fn save(frame: &RasterFrame, path: &std::path::Path, backdrop: Option<(u8, u8, u8)>) {
    use tiny_skia::{Color, IntSize, Pixmap, PixmapPaint, Transform};
    let size = IntSize::from_wh(frame.width, frame.height)
        .unwrap_or_else(|| panic!("{}: an empty frame has no PNG", path.display()));
    let strip = Pixmap::from_vec(frame.pixels.clone(), size)
        .unwrap_or_else(|| panic!("{}: pixel buffer does not match its size", path.display()));
    let out = match backdrop {
        None => strip,
        Some((red, green, blue)) => {
            let mut canvas = Pixmap::new(frame.width, frame.height)
                .unwrap_or_else(|| panic!("{}: no canvas for the backdrop", path.display()));
            canvas.fill(Color::from_rgba8(red, green, blue, 255));
            canvas.draw_pixmap(
                0,
                0,
                strip.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
            canvas
        }
    };
    check_png(&out, path);
}

/// One rendered pixmap against the PNG committed at `path` — or, when
/// regenerating, the file rewritten first and the run failed anyway.
///
/// Decoding the committed file and comparing pixel buffers keeps the check
/// about the picture rather than about the encoder. The size is compared
/// first, so a resized strip says so instead of reporting a mismatch at pixel
/// zero.
fn check_png(out: &tiny_skia::Pixmap, path: &std::path::Path) {
    let updating = graft_fixtures::updating();
    if updating {
        out.save_png(path)
            .unwrap_or_else(|why| panic!("writing {}: {why}", path.display()));
    }
    let committed = tiny_skia::Pixmap::load_png(path).unwrap_or_else(|why| {
        panic!(
            "{}: the committed capture could not be read ({why}). If it is simply \
             missing, create it with GRAFT_UPDATE_SHOTS=1.",
            path.display()
        )
    });
    assert_eq!(
        (committed.width(), committed.height()),
        (out.width(), out.height()),
        "{}: the committed capture is a different size from what the rasteriser draws. \
         Regenerate on purpose with GRAFT_UPDATE_SHOTS=1 and read the diff.",
        path.display()
    );
    let differing = committed
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(out.data().as_chunks::<4>().0)
        .filter(|(left, right)| left != right)
        .count();
    assert_eq!(
        differing,
        0,
        "{}: {differing} pixels differ from what the rasteriser draws. A capture is the \
         evidence the user judges by eye (CLAUDE.md rule 6), so it must match. If the new \
         drawing is the right one, regenerate with GRAFT_UPDATE_SHOTS=1 and read the diff.",
        path.display()
    );
    assert!(
        !updating,
        "{}: regenerated. This run is red so the change cannot pass unnoticed — read the \
         diff under docs/shots/ and commit it if it is right.",
        path.display()
    );
}
