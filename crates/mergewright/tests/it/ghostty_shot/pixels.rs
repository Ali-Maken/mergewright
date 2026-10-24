//! The Ghostty shot's pixels: strips stacked, cropped to the lines the real
//! frame places, and the cursor row's continuation band.

use mergewright::config::Loaded;
use mergewright::event_loop::placements;
use mergewright::session::Session;
use mergewright::update::AppState;
use mergewright::view::strips::{PixelRows, StripPaint};
use mergewright_core::capabilities::RenderingTier;
use mergewright_graph::raster::cache::{ByteBudget, StripCache};

use super::Line;
use crate::frames::run::capabilities;

/// The paint of a Kitty terminal with deflate on.
pub(super) fn kitty_paint() -> StripPaint {
    let session = Session::settle(&Loaded::default(), {
        let mut caps = capabilities(RenderingTier::Pixel);
        caps.kitty_deflate = true;
        caps
    });
    StripPaint::of(&session)
}

/// The kept lines' pixels, one cell row per line: the strip's, the band's or
/// nothing, and the history rows they hold.
pub(super) fn crop(
    kept: &[Option<Line>],
    first: usize,
    stacked: &[u8],
    drawn: &[Option<usize>],
    band: Option<&(u32, Vec<u8>)>,
    (width, cell_height): (u32, usize),
) -> Result<(Vec<u8>, (usize, usize)), String> {
    let stride = width as usize * 4;
    // One cell row per kept line: the strip's pixels, the band's, or nothing.
    let mut canvas = vec![0u8; kept.len() * cell_height * stride];
    let mut rows = (usize::MAX, 0);
    for (n, line) in kept.iter().enumerate() {
        let source: Option<&[u8]> = match line {
            Some(Line::Row(row)) => {
                rows = (rows.0.min(*row), rows.1.max(*row + 1));
                let k = drawn.iter().position(|d| *d == Some(*row));
                k.map(|k| &stacked[k * cell_height * stride..(k + 1) * cell_height * stride])
            }
            Some(Line::Uncommitted) => {
                let k = drawn.iter().position(Option::is_none);
                k.map(|k| &stacked[k * cell_height * stride..(k + 1) * cell_height * stride])
            }
            Some(Line::Second) => band.map(|(_, pixels)| pixels.as_slice()),
            None => None,
        };
        if let Some(source) = source {
            canvas[n * cell_height * stride..][..source.len().min(cell_height * stride)]
                .copy_from_slice(&source[..source.len().min(cell_height * stride)]);
        } else if !matches!(line, Some(Line::Second) | None) {
            return Err(format!("line {} ({line:?}) is in no strip", first + n));
        }
    }
    Ok((canvas, rows))
}

/// The cursor row's second line as the real frame draws it (the strip's
/// continuation band, `Band::of`): one cell row of pixels, `None` when no
/// lane passes through it.
pub(super) fn continuation(
    state: &AppState,
    frame: &PixelRows,
    paint: &StripPaint,
) -> Result<Option<(u32, Vec<u8>)>, String> {
    use mergewright_graph::raster::rasterise_continuation;
    for placed in placements(
        state,
        frame,
        u16::try_from(state.graph_pane_rows()).unwrap_or(u16::MAX),
    ) {
        let Some((row, _)) = placed.room.band else {
            continue;
        };
        let Some(strip) = placed.strip_of(frame) else {
            continue;
        };
        let band = strip
            .with_layers(frame, paint, |plain, selection| {
                rasterise_continuation(plain, selection, row)
            })
            .map_err(|why| format!("continuation band: {why:?}"))?;
        return Ok(band.map(|band| (band.width, band.pixels)));
    }
    Ok(None)
}

/// The frame's strips, composited, one under the other.
pub(crate) struct Stack {
    /// Every drawn row, top to bottom: its history row, or `None` for the
    /// uncommitted row (which pushes the rows under it down).
    pub(crate) drawn: Vec<Option<usize>>,
    /// Premultiplied RGBA, `width` pixels a line.
    pub(crate) pixels: Vec<u8>,
    pub(crate) width: u32,
}

pub(crate) fn stack(frame: &PixelRows, paint: &StripPaint) -> Result<Stack, String> {
    let mut cache = StripCache::with_budget(ByteBudget(usize::MAX), frame.strips.len() + 1);
    let mut drawn = Vec::new();
    let mut pixels = Vec::new();
    let mut width = None;
    let mut next_row = None;
    // The pinned uncommitted row off `HEAD` is an image of its own, stacked
    // first as the `None` row (ease-pass/20).
    if let Some(strip) = &frame.pinned {
        let raster = strip
            .with_layers(frame, paint, |plain, selection| {
                cache.composited(strip.first_row, plain, selection)
            })
            .map_err(|why| format!("composited the pinned row: {why:?}"))?;
        width = Some(raster.width);
        drawn.push(None);
        pixels.extend_from_slice(&raster.pixels);
    }
    for strip in &frame.strips {
        let raster = strip
            .with_layers(frame, paint, |plain, selection| {
                cache.composited(strip.first_row, plain, selection)
            })
            .map_err(|why| format!("composited strip {}: {why:?}", strip.first_row))?;
        let follows = next_row.is_none_or(|next| next == strip.first_row);
        if !follows || *width.get_or_insert(raster.width) != raster.width {
            return Err(format!(
                "strip at row {} (width {}) does not follow row {next_row:?} (width {width:?})",
                strip.first_row, raster.width
            ));
        }
        let uncommitted = strip.uncommitted_at();
        let mut history_row = strip.first_row;
        for k in 0..strip.row_count() {
            if uncommitted == Some(k) {
                drawn.push(None);
            } else {
                drawn.push(Some(history_row));
                history_row += 1;
            }
        }
        next_row = Some(history_row);
        pixels.extend_from_slice(&raster.pixels);
    }
    Ok(Stack {
        drawn,
        pixels,
        width: width.ok_or("no strip on screen")?,
    })
}

/// Premultiplied RGBA over a stand-in dark background (as
/// `mergewright-graph/tests/it/emphasis_layer.rs::on_dark`), as a PNG.
pub(crate) fn on_dark(pixels: Vec<u8>, width: u32, height: u32) -> Result<Vec<u8>, String> {
    use tiny_skia::{Color, IntSize, Pixmap, PixmapPaint, Transform};
    let size = IntSize::from_wh(width, height).ok_or("an empty crop has no PNG")?;
    let strip = Pixmap::from_vec(pixels, size).ok_or("the pixel buffer does not match its size")?;
    let mut canvas = Pixmap::new(width, height).ok_or("no canvas for the background")?;
    canvas.fill(Color::from_rgba8(0x1E, 0x1E, 0x2E, 255));
    canvas.draw_pixmap(
        0,
        0,
        strip.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    canvas
        .encode_png()
        .map_err(|why| format!("encoding a PNG: {why}"))
}
