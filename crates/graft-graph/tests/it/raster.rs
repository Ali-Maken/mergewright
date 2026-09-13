//! The pixel rasteriser on real corpus shapes (m1-the-graph/22).
//!
//! Buffers are pinned at an 8×16 cell (PROMPT.md §7.2 item 3) and
//! snapshotted as text: one character per pixel, `' '` for alpha 0, `.` for
//! a faint anti-aliased edge, and a letter naming the palette slot of
//! anything more solid. A text snapshot survives a sub-unit rounding change
//! a byte hash would not, and a reviewer can read it. Whether the strips
//! *look* good is judged by eye from the PNGs `write_shots` produces.

use std::time::Instant;

use graft_core::capabilities::Rgb;
use graft_core::raster::RasterFrame;
use graft_fixtures::Shape;
use graft_graph::layout::layout;
use graft_graph::raster::{Emphasis, Marker, RasterRow, Ring, Strip, rasterise};
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

fn strip<'a>(rows: &'a [RasterRow<'a>], lane_columns: usize, cell: (u32, u32)) -> Strip<'a> {
    Strip {
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

fn plain(rows: &[Row]) -> Vec<RasterRow<'_>> {
    rows.iter().map(RasterRow::plain).collect()
}

fn render(rows: &[Row], cell: (u32, u32)) -> RasterFrame {
    let raster_rows = plain(rows);
    rasterise(&strip(&raster_rows, width_of(rows), cell))
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
    let raster_row = [RasterRow {
        ring,
        ..RasterRow::plain(row)
    }];
    let palette = [colour];
    let frame = rasterise(&Strip {
        palette: &palette,
        ..strip(&raster_row, 1, USER_CELL)
    })
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

#[test]
fn emphasis_fades_or_thickens_the_strokes() {
    let rows = rows_of("foxtrot-merge");
    let columns = width_of(&rows);
    let draw = |emphasis| {
        let raster_rows: Vec<RasterRow<'_>> = rows
            .iter()
            .map(|row| RasterRow {
                emphasis,
                ..RasterRow::plain(row)
            })
            .collect();
        rasterise(&strip(&raster_rows, columns, SNAPSHOT_CELL)).unwrap()
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
    let (normal, dimmed, bold) = (
        draw(Emphasis::Normal),
        draw(Emphasis::Dimmed),
        draw(Emphasis::Highlighted),
    );
    assert!(
        dimmed
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[3] <= 90)
    );
    assert!(
        normal
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] == 255)
    );
    assert!(painted(&bold) > painted(&normal));
}

#[test]
fn the_marker_column_draws_its_count() {
    let rows = rows_of("octopus-merge");
    let row = &rows[1];
    let raster_rows = [RasterRow::plain(row)];
    let marker = Marker {
        column: 4,
        folded: 3,
    };
    let frame = rasterise(&Strip {
        marker: Some(marker),
        ..strip(&raster_rows, 5, SNAPSHOT_CELL)
    })
    .unwrap();
    let left = 8 * SNAPSHOT_CELL.0;
    let solid = (left..frame.width)
        .flat_map(|x| (0..frame.height).map(move |y| (x, y)))
        .filter(|&(x, y)| frame.pixel(x, y).unwrap()[3] == 255)
        .count();
    // `+3` at scale 1: 5 pixels for the plus, 11 for the three.
    assert_eq!(solid, 16, "the label's pixels, crisp:\n{}", art(&frame));
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
            let frame = rasterise(&strip).unwrap();
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

/// Writes the eyeball PNGs under `docs/shots/`. A tool, not a check: run it
/// by hand with `cargo test -p graft-graph --test it write_shots -- --ignored`.
#[test]
#[ignore = "writes docs/shots PNGs; run by hand when the drawing changes"]
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
    let raster_rows: Vec<RasterRow<'_>> = rings
        .iter()
        .zip(RINGS)
        .map(|(row, ring)| RasterRow {
            ring,
            ..RasterRow::plain(row)
        })
        .collect();
    let frame = rasterise(&strip(&raster_rows, RINGS.len(), USER_CELL)).unwrap();
    save(
        &frame,
        &shots.join("22-raster-rings-on-dark.png"),
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
    out.save_png(path)
        .unwrap_or_else(|why| panic!("writing {}: {why}", path.display()));
}
