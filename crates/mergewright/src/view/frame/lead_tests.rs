//! ease-pass/29 D5: a strip starts in the gutter's gap cell, so an overlay
//! that covers it (a confirm or preview box snapped to the frame edge,
//! ADR-0040) cuts the image there too.

use ratatui::layout::Rect;

use super::pixels_above_overlays;

#[test]
fn a_box_over_only_the_gutter_cell_still_cuts_the_strips() {
    let graph = Rect::new(2, 1, 40, 20);
    // The gap cell is column 1; the box covers column 0..2 on lines 8..14.
    let gutter_box = Rect::new(0, 8, 2, 6);
    assert_eq!(
        pixels_above_overlays(Some(graph), [gutter_box].into_iter()),
        Some(Rect::new(2, 1, 40, 7)),
        "image under the box's lines is cut"
    );
    // Column 0 alone (the cursor slot) covers no image.
    let slot_box = Rect::new(0, 8, 1, 6);
    assert_eq!(
        pixels_above_overlays(Some(graph), [slot_box].into_iter()),
        Some(graph),
        "the cursor slot has no image over it"
    );
}
