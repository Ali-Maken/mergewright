//! A node lies whole inside its own lane (ease-pass/29): the strip leads the
//! graph column by one gutter cell (D5), so a lane's box, one cell each side
//! of its centre, holds a node sized with one cell of room each side.

use mergewright_core::capabilities::Rgb;
use mergewright_core::raster::RasterFrame;

use super::Ring;
use crate::Seeds;
use crate::layout::layout;
use crate::layout::tests::commit;
use crate::raster::{
    Backdrop, Emphasis, Lit, Marker, PlainRow, PlainStrip, STRIP_LEAD_CELLS, Selection,
    emphasis_pieces, rasterise_backdrop, rasterise_piece,
};
use crate::row::Row;

/// The cell sizes the node is held to: the user's, the fallback, a larger
/// pair and two narrow-tall ones.
pub(in crate::raster) const CELLS: [(u32, u32); 7] = [
    (9, 20),
    (8, 16),
    (10, 22),
    (17, 37),
    (24, 52),
    (6, 24),
    (7, 28),
];
pub(in crate::raster) const STYLES: [Ring; 6] = [
    Ring::Commit,
    Ring::Merge,
    Ring::Root,
    Ring::Head,
    Ring::Stash,
    Ring::Uncommitted,
];
pub(in crate::raster) const LANES: usize = 3;
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

/// One root commit with no edges, its node moved to `lane`.
pub(in crate::raster) fn lone_row(lane: usize) -> Row {
    let mut row = layout(&[commit(1, &[], 0, b"only")], Seeds::default()).remove(0);
    row.lane = lane;
    // No lane is drawn through the row: only the node.
    row.lanes.clear();
    row
}

pub(in crate::raster) fn strip<'a>(
    rows: &'a [PlainRow<'a>],
    cell: (u32, u32),
    marker: Option<Marker>,
    lane_columns: usize,
) -> PlainStrip<'a> {
    PlainStrip {
        rows,
        above: None,
        lane_columns,
        marker,
        cell_width: cell.0,
        cell_height: cell.1,
        palette: &PALETTE,
        muted: MUTED,
        roles: None,
    }
}

/// Bounding box `(min_x, max_x, min_y, max_y)` of the pixels with alpha > 0.
fn bounds(frame: &RasterFrame, x_offset: u32, y_offset: u32) -> Option<(u32, u32, u32, u32)> {
    let mut found: Option<(u32, u32, u32, u32)> = None;
    for y in 0..frame.height {
        for x in 0..frame.width {
            if frame.pixel(x, y).is_some_and(|pixel| pixel[3] > 0) {
                let (x, y) = (x + x_offset, y + y_offset);
                found = Some(found.map_or((x, x, y, y), |(x0, x1, y0, y1)| {
                    (x0.min(x), x1.max(x), y0.min(y), y1.max(y))
                }));
            }
        }
    }
    found
}

/// Lane `lane`'s box in strip pixels (D5.4): the two cells centred on its
/// centre, `[(LEAD + 2c - 0.5)w, (LEAD + 2c + 1.5)w]`, as whole pixels
/// `(left, right)` with a half pixel rounded outward.
pub(in crate::raster) fn lane_box(lane: usize, cell_width: u32) -> (u32, u32) {
    let half_cells = 2 * STRIP_LEAD_CELLS + 4 * u32::try_from(lane).expect("small");
    let left = (half_cells - 1) * cell_width / 2;
    let right = ((half_cells + 3) * cell_width).div_ceil(2);
    (left, right)
}

/// Holds `bounds` to lane `lane`'s two cells and the row band; `what` names
/// the case on failure.
fn assert_whole(bounds: Option<(u32, u32, u32, u32)>, lane: usize, cell: (u32, u32), what: &str) {
    let (min_x, max_x, min_y, max_y) = bounds.unwrap_or_else(|| panic!("{what}: nothing drawn"));
    let (lane_left, next_left) = lane_box(lane, cell.0);
    assert!(
        min_x > lane_left && max_x < next_left - 1,
        "{what}: x {min_x}..={max_x} leaves lane {lane_left}..{next_left} with a pixel to spare"
    );
    let left = min_x - lane_left;
    let right = next_left - 1 - max_x;
    assert!(
        left.abs_diff(right) <= 1,
        "{what}: margins left {left} right {right} differ by more than a pixel"
    );
    let top = min_y;
    let bottom = cell.1 - 1 - max_y;
    assert!(
        top.abs_diff(bottom) <= 1,
        "{what}: margins top {top} bottom {bottom} differ by more than a pixel"
    );
}

#[test]
fn every_node_style_lies_whole_inside_its_lane() {
    for cell in CELLS {
        for ring in STYLES {
            for lane in 0..LANES {
                let row = lone_row(lane);
                let rows = [PlainRow {
                    row: &row,
                    ring,
                    stubs: Vec::new(),
                }];
                let frame = rasterise_backdrop(&strip(&rows, cell, None, LANES), Backdrop::Full)
                    .expect("one lone row rasterises");
                assert_whole(
                    bounds(&frame, 0, 0),
                    lane,
                    cell,
                    &format!("{ring:?} at {}x{} lane {lane}", cell.0, cell.1),
                );
            }
        }
    }
}

#[test]
fn a_node_piece_holds_its_node_whole() {
    for cell in CELLS {
        for ring in STYLES {
            for lane in 0..LANES {
                let row = lone_row(lane);
                let rows = [PlainRow {
                    row: &row,
                    ring,
                    stubs: Vec::new(),
                }];
                let strip = strip(&rows, cell, None, LANES);
                let thread = std::collections::BTreeSet::from([lane]);
                let lit = [Lit {
                    emphasis: Emphasis::Normal,
                    thread: Some(&thread),
                }];
                let selection = Selection {
                    rows: &lit,
                    backdrop: Backdrop::OffThread,
                };
                let piece = emphasis_pieces(&strip, &selection)
                    .into_iter()
                    .find(|piece| piece.kind == crate::raster::PieceKind::Node)
                    .expect("a lit node has a node piece");
                let drawn = rasterise_piece(&strip, &selection, piece)
                    .expect("the piece rasterises")
                    .expect("the piece draws something");
                let found = bounds(
                    &drawn.frame,
                    u32::from(drawn.first_column) * cell.0,
                    u32::from(drawn.first_row) * cell.1,
                );
                assert_whole(
                    found,
                    lane,
                    cell,
                    &format!("{ring:?} piece at {}x{} lane {lane}", cell.0, cell.1),
                );
            }
        }
    }
}

/// ease-pass/29 D5: the lead cell, left of the graph column, holds nothing
/// but lane 0's node: no edge, arc, rail or stub reaches left of lane 0's box.
#[test]
fn the_lead_cell_holds_only_lane_zeros_node() {
    use crate::raster::PlainRow;
    // A merge off lane 0 onto lane 1, with the rail on lane 0 and a stub.
    let commits = [
        commit(4, &[3, 2], 4, b"merge"),
        commit(3, &[1], 3, b"main"),
        commit(2, &[1], 2, b"side"),
        commit(1, &[], 1, b"root"),
    ];
    let rows = layout(&commits, Seeds::default());
    for cell in CELLS {
        let plain: Vec<PlainRow<'_>> = rows
            .iter()
            .map(|row| PlainRow {
                row,
                ring: if row.lane == 0 {
                    Ring::Head
                } else {
                    Ring::Commit
                },
                stubs: vec![crate::raster::Stub {
                    identity: crate::identity::BranchIdentity::of_name(b"f"),
                    toward: crate::fold::Toward::Below,
                }],
            })
            .collect();
        let strip = strip(&plain, cell, Some(Marker { column: 0 }), 2);
        let frame = rasterise_backdrop(&strip, Backdrop::Full).expect("rasterises");
        let (box_left, _) = lane_box(0, cell.0);
        for (index, row) in rows.iter().enumerate() {
            let top = u32::try_from(index).expect("small") * cell.1;
            for y in top..top + cell.1 {
                for x in 0..STRIP_LEAD_CELLS * cell.0 {
                    let ink = frame.pixel(x, y).is_some_and(|pixel| pixel[3] > 0);
                    // Left of lane 0's box nothing ever draws; in the rest of
                    // the lead cell only a lane-0 node does.
                    let allowed = x >= box_left && row.lane == 0;
                    assert!(
                        !ink || allowed,
                        "{}x{} row {index} (lane {}): ink at ({x},{y}) in the lead cell",
                        cell.0,
                        cell.1,
                        row.lane
                    );
                }
            }
        }
    }
}
