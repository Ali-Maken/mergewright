//! The pixel nodes for the user's eye (ease-pass/29): every ring style at
//! lanes 0 and 1 and the folded-edge stubs, 9x20 cells, scaled x4 with
//! nearest-neighbour on the dark theme's background.
//!
//! Right: every node a whole circle or ring, the left as round as the right,
//! lane 0's node centred over the graph column's first cell (the grey ticks
//! top and bottom mark the graph column's left edge: the strip starts one
//! gutter cell left of it) with a clear gap to the image's left edge; each stub a short line leaving its
//! node toward the lane's lower-right (`Below`) or upper-right (`Above`)
//! corner, ending inside the lane. Colour is **unverified** (CLAUDE.md
//! rule 5). Written under `MWR_UPDATE_SHOTS=1`; `MWR_NODES_SHOT=before`
//! names the file `29-nodes-before.png` (taken on the pre-fix tree).

#![allow(
    clippy::unwrap_used,
    reason = "A capture generator: a failed step should stop the test with its own message."
)]

use mergewright_core::capabilities::Rgb;
use mergewright_core::raster::RasterFrame;
use mergewright_graph::fold::Toward;
use mergewright_graph::identity::BranchIdentity;
use mergewright_graph::raster::{
    Backdrop, Marker, PlainRow, PlainStrip, Ring, STRIP_LEAD_CELLS, Stub, rasterise_backdrop,
};
use tiny_skia::{Color, IntSize, Pixmap, PixmapPaint, Transform};

const CELL: (u32, u32) = (9, 20);
const SCALE: u32 = 4;
const PALETTE: [Rgb; 1] = [Rgb {
    red: 0x00,
    green: 0x77,
    blue: 0xBB,
}];
const MUTED: Rgb = Rgb {
    red: 0x88,
    green: 0x88,
    blue: 0x88,
};
/// A grid slot: wide enough for a 3-lane strip, with a gap.
const SLOT: (u32, u32) = (7 * CELL.0 + 8, CELL.1 + 4);
const PER_ROW: u32 = 4;

fn one(ring: Ring, lane: usize, columns: usize, rail: bool, stub: Option<Toward>) -> RasterFrame {
    let mut row = super::raster_pixels::synthetic(1, 1).remove(0);
    row.lane = lane;
    row.lanes.clear();
    let stubs = stub
        .map(|toward| Stub {
            identity: BranchIdentity::of_name(b"folded"),
            toward,
        })
        .into_iter()
        .collect();
    let rows = [PlainRow {
        row: &row,
        ring,
        stubs,
    }];
    let strip = PlainStrip {
        rows: &rows,
        above: None,
        lane_columns: columns,
        marker: rail.then_some(Marker { column: lane }),
        cell_width: CELL.0,
        cell_height: CELL.1,
        palette: &PALETTE,
        muted: MUTED,
        roles: None,
    };
    rasterise_backdrop(&strip, Backdrop::Full).unwrap()
}

fn sheet() -> Vec<u8> {
    let mut frames = Vec::new();
    for ring in [
        Ring::Commit,
        Ring::Merge,
        Ring::Root,
        Ring::Head,
        Ring::Stash,
        Ring::Uncommitted,
    ] {
        frames.push(one(ring, 0, 2, false, None));
        frames.push(one(ring, 1, 2, false, None));
    }
    for ring in [Ring::Commit, Ring::Head] {
        for toward in [Toward::Below, Toward::Above] {
            frames.push(one(ring, 0, 1, false, Some(toward)));
            frames.push(one(ring, 2, 3, true, Some(toward)));
        }
    }
    let count = u32::try_from(frames.len()).unwrap();
    let rows = count.div_ceil(PER_ROW);
    let mut canvas = Pixmap::new(PER_ROW * SLOT.0, rows * SLOT.1).unwrap();
    canvas.fill(Color::from_rgba8(0x1E, 0x1E, 0x2E, 255));
    for (index, frame) in frames.iter().enumerate() {
        let index = u32::try_from(index).unwrap();
        let size = IntSize::from_wh(frame.width, frame.height).unwrap();
        let strip = Pixmap::from_vec(frame.pixels.clone(), size).unwrap();
        let x = i32::try_from((index % PER_ROW) * SLOT.0 + 4).unwrap();
        let y = i32::try_from((index / PER_ROW) * SLOT.1 + 2).unwrap();
        canvas.draw_pixmap(
            x,
            y,
            strip.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
        // The graph column's left edge: a 1 px tick at the top and bottom.
        let edge = u32::try_from(x).unwrap() + STRIP_LEAD_CELLS * CELL.0;
        let top = u32::try_from(y).unwrap();
        let across = canvas.width();
        for tick in (0..3).chain(CELL.1 - 3..CELL.1) {
            canvas.pixels_mut()[((top + tick) * across + edge) as usize] =
                Color::from_rgba8(0x88, 0x88, 0x88, 255)
                    .premultiply()
                    .to_color_u8();
        }
    }
    let mut big = Pixmap::new(canvas.width() * SCALE, canvas.height() * SCALE).unwrap();
    for y in 0..big.height() {
        for x in 0..big.width() {
            let from = canvas.pixel(x / SCALE, y / SCALE).unwrap();
            let at = (y * big.width() + x) as usize;
            big.pixels_mut()[at] = from;
        }
    }
    big.encode_png().unwrap()
}

#[test]
fn capture_ease_29_nodes() {
    let png = sheet();
    assert!(png.len() <= 200 * 1024, "the sheet is {} bytes", png.len());
    let name = match std::env::var("MWR_NODES_SHOT").as_deref() {
        Ok("before") => "29-nodes-before.png",
        _ => "29-nodes-after.png",
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/shots/ease-pass")
        .join(name);
    if let Err(drift) = mergewright_fixtures::capture::check_bytes(&path, &png) {
        panic!("{drift}");
    }
}
