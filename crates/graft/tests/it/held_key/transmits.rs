//! What the held run sent to the Kitty layer: transmits by layer, and the
//! emphasis area the pieces cost (m8 05 amendment 1).

use graft_term::kitty::EMPHASIS_ID_BIT;

use super::{CELL_HEIGHT, CELL_WIDTH, HELD, Reading, STRIP_ROWS};
use crate::drain_sink::DrainSink;

/// Emphasis area in strip-tall cells per held `j`, to a tenth (floored).
pub(super) fn area_per_key(area: u64) -> String {
    let tenths = area * 10 / (STRIP_ROWS * HELD as u64);
    format!("{}.{}", tenths / 10, tenths % 10)
}

/// Kitty transmits while held, by layer, and the run's cost lines (46):
/// recorded, not judged. `total` is every byte the run wrote.
pub(super) fn count_transmits(drain: &DrainSink, held_end: u64, total: u64, reading: &mut Reading) {
    let transmits = drain.kitty_transmits();
    for transmit in &transmits {
        if transmit.offset >= held_end {
            break;
        }
        if transmit.image_id & EMPHASIS_ID_BIT != 0 {
            reading.emphasis_transmits_held += 1;
        } else {
            reading.strip_transmits_held += 1;
        }
    }
    // Cost lines (46): recorded, not judged.
    reading.bytes_per_key = total / HELD as u64;
    reading.deflated_transmits = transmits.iter().filter(|t| t.deflated).count();
    let mut widths: Vec<u32> = transmits
        .iter()
        .filter(|t| t.image_id & EMPHASIS_ID_BIT != 0)
        .filter_map(|t| t.width_px)
        .map(|px| px / CELL_WIDTH)
        .collect();
    widths.sort_unstable();
    reading.emphasis_area = transmits
        .iter()
        .filter(|t| t.image_id & EMPHASIS_ID_BIT != 0)
        .filter_map(|t| {
            Some(u64::from(t.width_px? / CELL_WIDTH) * u64::from(t.height_px? / CELL_HEIGHT))
        })
        .sum();
    reading.emphasis_cells_p99 =
        (!widths.is_empty()).then(|| widths[(widths.len() * 99 / 100).min(widths.len() - 1)]);
}
