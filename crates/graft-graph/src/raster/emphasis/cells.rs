//! Which cell columns of an emphasis image hold paint, and cropping to them.

use graft_core::raster::RasterFrame;
/// The first and last cell columns holding a painted pixel, or `None` when
/// none does. Premultiplied: a clear pixel is four zero bytes. Only the
/// pixels around lane columns `reach` are scanned: a node reaches less than
/// a cell either side of its lane's centre (`NodeSize::for_cell`), every
/// stroke less than that.
pub(super) fn painted_cells(
    frame: &RasterFrame,
    cell_width: u32,
    reach: (usize, usize),
) -> Option<(u32, u32)> {
    let width = usize::try_from(frame.width).ok()?;
    let cell = usize::try_from(cell_width).ok()?;
    let lane = cell * usize::try_from(super::super::CELLS_PER_LANE).ok()?;
    // From a cell left of the first lane's own cell to the end of the cell
    // right of the last lane's own cell.
    let from = (reach.0 * lane).saturating_sub(cell).min(width);
    let to = (reach.1 * lane + 2 * cell).min(width);
    painted_cells_in(frame, cell_width, from..to)
}

/// [`painted_cells`] over the pixel columns `window` of `frame`, which
/// start and end on cell edges.
pub(super) fn painted_cells_in(
    frame: &RasterFrame,
    cell_width: u32,
    window: std::ops::Range<usize>,
) -> Option<(u32, u32)> {
    let width = usize::try_from(frame.width).ok()?;
    let cell = usize::try_from(cell_width).ok()?;
    if width == 0 || cell == 0 {
        return None;
    }
    let (from, to) = (window.start.min(width), window.end.min(width));
    let painted = |byte: &u8| *byte != 0;
    // Both on cell edges, as `from` and `to` are.
    let start = |pixel: usize| pixel / cell * cell;
    let end = |pixel: usize| (pixel / cell + 1) * cell;
    // The painted cells found so far, as a pixel range `[left, right)`:
    // only pixels outside it can widen it, so a line scans those alone.
    let mut span: Option<(usize, usize)> = None;
    for line in frame.pixels.chunks_exact(width * 4) {
        let pixels = |range: std::ops::Range<usize>| &line[range.start * 4..range.end * 4];
        span = match span {
            None => {
                let Some(byte) = pixels(from..to).iter().position(painted) else {
                    continue;
                };
                let first = from + byte / 4;
                let after = end(first);
                let last = pixels(after..to)
                    .iter()
                    .rposition(painted)
                    .map_or(first, |byte| after + byte / 4);
                Some((start(first), end(last)))
            }
            Some((left, right)) => {
                let first = pixels(from..left).iter().position(painted);
                let last = pixels(right..to).iter().rposition(painted);
                Some((
                    first.map_or(left, |byte| start(from + byte / 4)),
                    last.map_or(right, |byte| end(right + byte / 4)),
                ))
            }
        };
    }
    let (left, right) = span?;
    let cell_of = |pixel: usize| u32::try_from(pixel / cell).ok();
    Some((cell_of(left)?, cell_of(right - 1)?))
}

/// Cell columns `first..=last` of `frame`, in its own buffer: each line
/// moves left to where the cropped image's line goes, which is never right
/// of where it was, then the buffer is cut to the cropped size.
pub(super) fn crop(frame: RasterFrame, cell_width: u32, first: u32, last: u32) -> RasterFrame {
    let columns = last - first + 1;
    let width = columns * cell_width;
    let bytes = |pixels: u32| pixels as usize * 4;
    let (stride, left, span) = (bytes(frame.width), bytes(first * cell_width), bytes(width));
    let mut pixels = frame.pixels;
    let lines = frame.height as usize;
    for line in 0..lines {
        let from = line * stride + left;
        pixels.copy_within(from..from + span, line * span);
    }
    pixels.truncate(span * lines);
    RasterFrame {
        width,
        height: frame.height,
        rows: frame.rows,
        columns,
        pixels,
    }
}
