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

/// The first and last cell rows of `frame` holding a painted pixel, or
/// `None` when none does.
pub(super) fn painted_rows(frame: &RasterFrame) -> Option<(usize, usize)> {
    let width = usize::try_from(frame.width).ok()?;
    let lines = usize::try_from(frame.height).ok()?;
    let cell = lines.checked_div(frame.rows)?;
    if width == 0 || cell == 0 {
        return None;
    }
    let painted = |line: &[u8]| line.iter().any(|&byte| byte != 0);
    let mut scan = frame.pixels.chunks_exact(width * 4);
    let first = scan.position(painted)?;
    let last = frame.pixels.chunks_exact(width * 4).rposition(painted)?;
    Some((first / cell, last / cell))
}

/// Cell columns `columns.0..=columns.1` and cell rows `rows.0..=rows.1` of
/// `frame`, in its own buffer: each kept line moves to where the cropped
/// image's line goes, which is never right of or below where it was, then
/// the buffer is cut to the cropped size.
pub(super) fn crop(
    frame: RasterFrame,
    cell_width: u32,
    columns: (u32, u32),
    rows: (usize, usize),
) -> RasterFrame {
    let (first, last) = columns;
    debug_assert!(
        rows.0 <= rows.1 && rows.1 < frame.rows,
        "rows within the frame"
    );
    #[allow(
        clippy::cast_possible_truncation,
        reason = "rows × cell height is the frame's u32 height, so both fit a u32."
    )]
    let (cell_height, kept) = (
        frame.height / (frame.rows.max(1) as u32),
        (rows.1 + 1 - rows.0) as u32,
    );
    let count = last - first + 1;
    let width = count * cell_width;
    let bytes = |pixels: u32| pixels as usize * 4;
    let (stride, left, span) = (bytes(frame.width), bytes(first * cell_width), bytes(width));
    let height = kept * cell_height;
    let (top, lines) = (rows.0 * cell_height as usize, height as usize);
    let mut pixels = frame.pixels;
    for line in 0..lines {
        let from = (top + line) * stride + left;
        pixels.copy_within(from..from + span, line * span);
    }
    pixels.truncate(span * lines);
    RasterFrame {
        width,
        height,
        rows: kept as usize,
        columns: count,
        pixels,
    }
}
