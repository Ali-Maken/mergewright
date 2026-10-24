use super::lane_centre_x;
use crate::raster::node::extent_tests::{CELLS, lone_row, strip};
use crate::raster::{
    Backdrop, CELLS_PER_LANE, PlainRow, Ring, STRIP_LEAD_CELLS, rasterise_backdrop,
};

/// ease-pass/29 D5: a lane's centre is the centre of its own cell, where the
/// Unicode tier draws the node glyph, counted from a strip that starts one
/// gutter cell left of the graph column.
#[test]
#[allow(
    clippy::float_cmp,
    reason = "Whole-number products of whole-number widths: exact in f32."
)]
fn a_lane_centres_on_its_unicode_node_cell() {
    for width in [6_u32, 7, 8, 9, 10, 17, 24] {
        for lane in 0..4_u16 {
            #[allow(clippy::cast_precision_loss, reason = "Tiny whole numbers.")]
            let (w, k, lead) = (width as f32, f32::from(lane), STRIP_LEAD_CELLS as f32);
            assert_eq!(
                lane_centre_x(usize::from(lane), w) - lead * w,
                (2.0 * k + 0.5) * w,
                "lane {lane} at width {width}"
            );
        }
    }
}

/// D5.2: a strip is `STRIP_LEAD_CELLS + lanes * CELLS_PER_LANE` cells wide.
#[test]
fn a_strip_leads_the_graph_column_by_the_lead_cells() {
    let cell = CELLS[0];
    for lanes in 1..=4_usize {
        let row = lone_row(0);
        let rows = [PlainRow {
            row: &row,
            ring: Ring::Commit,
            stubs: Vec::new(),
        }];
        let frame = rasterise_backdrop(&strip(&rows, cell, None, lanes), Backdrop::Full)
            .expect("rasterises");
        let cells = STRIP_LEAD_CELLS + u32::try_from(lanes).expect("small") * CELLS_PER_LANE;
        assert_eq!(frame.columns, cells, "{lanes} lanes");
        assert_eq!(frame.width, cells * cell.0, "{lanes} lanes");
    }
}
