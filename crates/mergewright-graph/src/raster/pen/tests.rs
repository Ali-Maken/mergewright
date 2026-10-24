use super::lane_centre_x;

/// ease-pass/29: a lane's centre is the boundary between its two cells.
#[test]
#[allow(
    clippy::float_cmp,
    reason = "Whole-number products of whole-number widths: exact in f32."
)]
fn a_lane_centres_on_the_boundary_of_its_two_cells() {
    for width in [6_u32, 7, 8, 9, 10, 17, 24] {
        for lane in 0..4_u16 {
            #[allow(clippy::cast_precision_loss, reason = "Tiny whole numbers.")]
            let (w, k) = (width as f32, f32::from(lane));
            assert_eq!(
                lane_centre_x(usize::from(lane), w),
                (2.0 * k + 1.0) * w,
                "lane {lane} at width {width}"
            );
        }
    }
}
