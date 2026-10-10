//! What the `raster_pieces` tests share: the cases they walk, compositing
//! pieces, owners, keys and pictures.

use std::collections::HashMap;
use std::ops::Range;

use graft_core::raster::RasterFrame;
use graft_graph::raster::{
    Backdrop, Emphasised, Lit, PieceKind, PlainStrip, Selection, composite, emphasis_pieces,
    rasterise_piece,
};

use super::super::emphasis_layer::{Window, branchy};

/// Every (strip, cursor, backdrop) of `window` this samples: a cursor
/// every few rows, both backdrops (`Faded` is Focus).
pub(super) fn piece_cases(window: &Window) -> Vec<(Range<usize>, usize, Backdrop)> {
    let mut cases = Vec::new();
    for range in window.strips() {
        for cursor in (0..window.rows.len()).step_by(8) {
            for backdrop in [Backdrop::OffThread, Backdrop::Faded] {
                cases.push((range.clone(), cursor, backdrop));
            }
        }
    }
    cases
}

/// The branchy window with the rail (lanes fold) and without it.
pub(super) fn both_caps() -> [Window; 2] {
    let mut unfolded = branchy();
    unfolded.marker = None;
    [branchy(), unfolded]
}

/// `base` with every piece of `selection`'s emphasis composited over it in
/// `emphasis_pieces` order, and per pixel how many pieces paint it.
pub(super) fn pieces_over(
    strip: &PlainStrip<'_>,
    selection: &Selection<'_>,
    base: &RasterFrame,
    case: &str,
) -> (RasterFrame, Vec<u8>) {
    let mut got = base.clone();
    let mut painters = vec![0u8; (base.width * base.height) as usize];
    for piece in emphasis_pieces(strip, selection) {
        let Some(top) = rasterise_piece(strip, selection, piece)
            .unwrap_or_else(|why| panic!("{case}: piece {piece:?}: {why}"))
        else {
            continue;
        };
        let left = u32::from(top.first_column) * strip.cell_width;
        let below = u32::from(top.first_row) * strip.cell_height;
        assert!(
            below + top.frame.height <= base.height,
            "{case}: {piece:?} below the strip"
        );
        for y in 0..top.frame.height {
            for x in (0..top.frame.width).filter(|x| left + x < base.width) {
                if top.frame.pixels[((y * top.frame.width + x) * 4 + 3) as usize] != 0 {
                    let pixel = ((below + y) * base.width + left + x) as usize;
                    painters[pixel] = painters[pixel].saturating_add(1);
                }
            }
        }
        got = composite(&got, &top);
    }
    (got, painters)
}

/// What `pieces_composite_equals_single_emphasis` saw across its cases.
#[derive(Default)]
pub(super) struct Tally {
    pub(super) lit_selections: usize,
    pub(super) overlapping: usize,
    /// Overlapped pixels a channel of which is off by more than `b × 255`.
    pub(super) reordered: usize,
}

impl Tally {
    pub(super) fn compare(
        &mut self,
        want: &RasterFrame,
        got: &RasterFrame,
        painters: &[u8],
        backdrop: f32,
        case: &str,
    ) {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "An alpha in 0..=1 times 255."
        )]
        let bound = (backdrop * 255.0).ceil() as u8;
        let pairs = want
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(got.pixels.as_chunks::<4>().0);
        for (pixel, (want, got)) in pairs.enumerate() {
            if painters[pixel] <= 1 {
                assert_eq!(
                    want, got,
                    "{case}: pixel {pixel} ({} painters)",
                    painters[pixel]
                );
                continue;
            }
            self.overlapping += 1;
            assert!(
                want[3].abs_diff(got[3]) <= 2,
                "{case}: overlapped pixel {pixel} alpha {} vs {}",
                want[3],
                got[3]
            );
            if want.iter().zip(got).any(|(w, g)| w.abs_diff(*g) > bound) {
                self.reordered += 1;
                println!("{case}: past b × 255 at pixel {pixel}: {want:?} vs {got:?}");
            }
        }
    }
}

/// The piece owning what `row` (strip row `index`) draws in lane column
/// `column`, as (column, kind), or `None` when it draws nothing there.
pub(super) fn owner_of(
    window: &Window,
    row: &graft_graph::row::Row,
    index: usize,
    column: usize,
) -> Option<(usize, PieceKind)> {
    if row.lane == column {
        return Some((column, PieceKind::Node));
    }
    row.lanes.get(column).copied().flatten()?;
    let arc = row.incoming.contains(&column) || row.outgoing.iter().any(|&(out, _)| out == column);
    if arc {
        let row = u16::try_from(index).unwrap_or_else(|why| panic!("strip row {index}: {why}"));
        Some((column, PieceKind::Arc { row }))
    } else if window.marker.map(|marker| marker.column) == Some(column) {
        None
    } else {
        Some((column, PieceKind::Run))
    }
}

/// Each piece's key by (column, kind).
pub(super) fn piece_keys(
    strip: &PlainStrip<'_>,
    lit: &[Lit<'_>],
    backdrop: Backdrop,
) -> HashMap<(usize, PieceKind), u64> {
    let selection = Selection {
        rows: lit,
        backdrop,
    };
    let pieces = emphasis_pieces(strip, &selection);
    let keys: HashMap<_, _> = pieces
        .iter()
        .map(|piece| ((piece.column, piece.kind), piece.key))
        .collect();
    assert_eq!(keys.len(), pieces.len(), "one piece per (column, kind)");
    keys
}

/// What a piece rasterised to, relative to its anchor: pixels, size, and
/// its crop's offset from the anchor in cells and rows.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Picture {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    offset: (i64, i64),
}

/// Where two pictures differ: offsets, and the first few differing bytes
/// as (x, y, was, now).
pub(super) fn differ(earlier: &Picture, now: &Picture) -> String {
    let width = earlier.width as usize;
    let bytes: Vec<_> = earlier
        .pixels
        .iter()
        .zip(&now.pixels)
        .enumerate()
        .filter(|(_, (was, is))| was != is)
        .map(|(at, (was, is))| (at / 4 % width, at / 4 / width, *was, *is))
        .collect();
    format!(
        "offsets {:?} vs {:?}, sizes {}x{} vs {}x{}, {} bytes differ: {:?}",
        earlier.offset,
        now.offset,
        earlier.width,
        earlier.height,
        now.width,
        now.height,
        bytes.len(),
        &bytes[..bytes.len().min(8)]
    )
}

pub(super) fn picture(top: &Emphasised, column: usize, kind: PieceKind) -> Picture {
    let anchor_row = match kind {
        PieceKind::Arc { row } => i64::from(row),
        PieceKind::Run | PieceKind::Node => 0,
    };
    let anchor_cell = i64::try_from(column).unwrap_or_else(|why| panic!("{column}: {why}")) * 2;
    Picture {
        width: top.frame.width,
        height: top.frame.height,
        pixels: top.frame.pixels.clone(),
        offset: (
            i64::from(top.first_column) - anchor_cell,
            i64::from(top.first_row) - anchor_row,
        ),
    }
}
