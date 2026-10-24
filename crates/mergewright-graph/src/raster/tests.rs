use super::node::NodeSize;
use super::node::extent_tests::{CELLS, STYLES, lone_row, strip};
use super::{
    Backdrop, CELLS_PER_LANE, Marker, PlainRow, Ring, Stub, rasterise_backdrop, stroke_width,
};
use crate::fold::Toward;
use crate::identity::BranchIdentity;

#[test]
#[allow(
    clippy::float_cmp,
    reason = "Exact clamp bounds and an exact division; no rounding involved."
)]
fn stroke_width_clamps_by_cell_width() {
    assert_eq!(stroke_width(8), 1.5);
    assert_eq!(stroke_width(24), 2.5);
    assert_eq!(stroke_width(17), 17.0 / 8.0);
}

/// One strip shape the stub is held to.
struct Case {
    what: &'static str,
    columns: usize,
    lane: usize,
    /// The rail is on the node's own column.
    rail: bool,
}

const CASES: [Case; 4] = [
    Case {
        what: "lane 0 of 1",
        columns: 1,
        lane: 0,
        rail: false,
    },
    Case {
        what: "lane 1 of 3",
        columns: 3,
        lane: 1,
        rail: false,
    },
    Case {
        what: "last of 3",
        columns: 3,
        lane: 2,
        rail: false,
    },
    Case {
        what: "rail",
        columns: 3,
        lane: 2,
        rail: true,
    },
];

/// The pixels `stubs` add to a lone row of `ring` in `case`.
fn stub_pixels(
    ring: Ring,
    cell: (u32, u32),
    case: &Case,
    toward: Toward,
) -> (Vec<(u32, u32)>, u32) {
    let row = lone_row(case.lane);
    let stub = Stub {
        identity: BranchIdentity::of_name(b"folded"),
        toward,
    };
    let marker = case.rail.then_some(Marker { column: case.lane });
    let draw = |stubs: Vec<Stub>| {
        let rows = [PlainRow {
            row: &row,
            ring,
            stubs,
        }];
        rasterise_backdrop(&strip(&rows, cell, marker, case.columns), Backdrop::Full)
            .expect("one row rasterises")
    };
    let (with, without) = (draw(vec![stub]), draw(Vec::new()));
    let mut lit = Vec::new();
    for y in 0..with.height {
        for x in 0..with.width {
            let (a, b) = (with.pixel(x, y), without.pixel(x, y));
            if a != b && a.is_some_and(|pixel| pixel[3] > 0) {
                lit.push((x, y));
            }
        }
    }
    (lit, with.width)
}

/// ease-pass/29 D3: a folded edge's stub is a short tail from its node's
/// rim into its own lane's corner, never out of the lane.
#[test]
fn a_stub_leaves_its_node_into_its_lane_corner() {
    for cell in CELLS {
        for ring in STYLES {
            for toward in [Toward::Below, Toward::Above] {
                for case in &CASES {
                    let name =
                        format!("{ring:?} {toward:?} at {}x{} {}", cell.0, cell.1, case.what);
                    let (lit, _) = stub_pixels(ring, cell, case, toward);
                    assert_stub(&name, ring, cell, case, toward, &lit);
                }
            }
        }
    }
}

fn assert_stub(
    name: &str,
    ring: Ring,
    cell: (u32, u32),
    case: &Case,
    toward: Toward,
    lit: &[(u32, u32)],
) {
    let size = NodeSize::for_cell(px(cell.0), px(cell.1), stroke_width(cell.0));
    let reach = size.reach(ring);
    let lane_left = u32::try_from(case.lane).expect("small") * CELLS_PER_LANE * cell.0;
    let next_left = lane_left + CELLS_PER_LANE * cell.0;
    let (cx, cy) = (px(lane_left + cell.0), px(cell.1) / 2.0);
    assert!(!lit.is_empty(), "{name}: no stub drawn");
    let (mut nearest, mut farthest) = (f32::MAX, 0.0_f32);
    for &(x, y) in lit {
        assert!(
            x > lane_left && x < next_left - 1 && y < cell.1,
            "{name}: pixel ({x},{y}) outside lane {lane_left}..{next_left}"
        );
        let (dx, dy) = (px(x) + 0.5 - cx, px(y) + 0.5 - cy);
        assert!(dx > 0.0, "{name}: pixel ({x},{y}) is left of the centre");
        let wrong_side = match toward {
            Toward::Below => dy < -0.5,
            Toward::Above => dy > 0.5,
        };
        assert!(!wrong_side, "{name}: pixel ({x},{y}) is on the wrong side");
        nearest = nearest.min(dx.hypot(dy));
        farthest = farthest.max(dx.hypot(dy));
    }
    assert!(
        nearest >= reach - 1.5,
        "{name}: overdraws the node ({nearest} < {reach})"
    );
    assert!(
        nearest <= reach + size.gap_px() + 1.0,
        "{name}: floats off the node ({nearest})"
    );
    assert!(
        farthest >= reach + 1.5,
        "{name}: only {} past reach",
        farthest - reach
    );
    eprintln!(
        "STUBLEN {ring:?} {}x{} {} {:.1}",
        cell.0,
        cell.1,
        case.what,
        farthest - reach
    );
}

#[allow(clippy::cast_precision_loss, reason = "Tiny pixel counts.")]
fn px(value: u32) -> f32 {
    value as f32
}
