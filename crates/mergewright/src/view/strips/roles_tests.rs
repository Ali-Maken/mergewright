//! Roles and colours in the pixel tier's strips (ease-pass 13): `#` mutes
//! every lane, and a strip's key follows its lanes' roles.

use mergewright_core::capabilities::Rgb;
use mergewright_core::raster::RasterFrame;
use mergewright_graph::raster::cache::{ByteBudget, STRIP_ROWS, StripCache};
use mergewright_graph::raster::rasterise_backdrop;

use super::tests::fan;
use super::{PixelRows, StripPaint, pixel_rows};
use crate::history::History;
use crate::session::for_tests::session;
use crate::update::{AppState, Size};

const MAX_LANES: usize = 12;

fn state(history: History) -> AppState {
    AppState {
        history: Some(history),
        size: Size {
            columns: 80,
            rows: 40,
        },
        ..AppState::default()
    }
}

fn at(mut history: History, row: usize) -> History {
    history.top = 32;
    history.cursor = row;
    history
}

/// Every strip's backdrop, rasterised straight (no cache), and its key.
fn drawn(rows: &PixelRows, paint: &StripPaint) -> Vec<(RasterFrame, u64)> {
    let cache = StripCache::with_budget(ByteBudget(usize::MAX), 8);
    rows.strips
        .iter()
        .map(|strip| {
            strip.with_layers(rows, paint, |plain, selection| {
                let frame = rasterise_backdrop(plain, selection.backdrop)
                    .unwrap_or_else(|why| panic!("strip {}: {why}", strip.first_row));
                let key = cache
                    .backdrop_key(strip.first_row, plain, selection.backdrop)
                    .unwrap_or_else(|why| panic!("strip {}: {why}", strip.first_row));
                (frame, key)
            })
        })
        .collect()
}

/// A premultiplied pixel's straight colour.
fn straight(pixel: &[u8]) -> Rgb {
    let un = |c: u8| u8::try_from(u32::from(c) * 255 / u32::from(pixel[3])).unwrap_or(u8::MAX);
    Rgb {
        red: un(pixel[0]),
        green: un(pixel[1]),
        blue: un(pixel[2]),
    }
}

fn near(a: Rgb, b: Rgb) -> bool {
    let d = |x: u8, y: u8| x.abs_diff(y) <= 8;
    d(a.red, b.red) && d(a.green, b.green) && d(a.blue, b.blue)
}

/// Settle test 1 (ease-pass 13): with `#` pressed every lane pixel is the
/// muted hue, each strip re-keys, and every stroke keeps its shape (the
/// alpha channel is byte-equal).
#[test]
fn colours_off_mutes_every_strip_lane() {
    let session = session();
    let mut state = state(at(fan(2, 40), 35));
    let rows = pixel_rows(&state, MAX_LANES, STRIP_ROWS);
    assert!(!rows.strips.is_empty());
    let on = drawn(&rows, &StripPaint::for_frame(&session, &state));

    state.look.colours_off = true;
    let paint = StripPaint::for_frame(&session, &state);
    let off = drawn(&rows, &paint);

    let muted = session.theme.raster_muted();
    for ((on, on_key), (off, off_key)) in on.iter().zip(&off) {
        assert_ne!(on_key, off_key, "colours off re-keys the strip");
        let alphas = |frame: &RasterFrame| -> Vec<u8> {
            frame
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .map(|pixel| pixel[3])
                .collect()
        };
        assert_eq!(alphas(on), alphas(off), "shapes stay");
        let mut lit = 0;
        for pixel in off
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|pixel| pixel[3] > 64)
        {
            lit += 1;
            assert!(
                near(straight(pixel), muted),
                "{:?} is not the muted {muted:?}",
                straight(pixel)
            );
        }
        assert!(lit > 0, "the strip draws something");
    }
}

/// Each strip's backdrop through `cache`.
fn through(cache: &mut StripCache, rows: &PixelRows, paint: &StripPaint) {
    for strip in &rows.strips {
        strip
            .with_layers(rows, paint, |plain, selection| {
                cache.backdrop(strip.first_row, plain, selection.backdrop)
            })
            .unwrap_or_else(|why| panic!("strip {}: {why}", strip.first_row));
    }
}

/// `j` within one branch keeps every role, so every strip hits; moving
/// the cursor onto another branch changes the lanes' roles and re-keys.
#[test]
fn a_strip_rekeys_on_a_role_change_and_hits_across_j_within_a_branch() {
    let session = session();
    // Rows 1..=40 are chain 1's, 41..=80 chain 2's; strip 32 holds both.
    let mut state = state(at(fan(2, 40), 35));
    let paint = StripPaint::for_frame(&session, &state);
    let keys = |rows: &PixelRows| -> Vec<u64> {
        drawn(rows, &paint)
            .into_iter()
            .map(|(_, key)| key)
            .collect()
    };
    let mut cache = StripCache::with_budget(ByteBudget(usize::MAX), 8);
    let first = pixel_rows(&state, MAX_LANES, STRIP_ROWS);
    through(&mut cache, &first, &paint);
    let rasterised = cache.stats().rasterised;

    state.history = Some(at(fan(2, 40), 36));
    let same_branch = pixel_rows(&state, MAX_LANES, STRIP_ROWS);
    assert_eq!(keys(&first), keys(&same_branch));
    through(&mut cache, &same_branch, &paint);
    assert_eq!(
        cache.stats().rasterised,
        rasterised,
        "j within a branch re-rasterises nothing"
    );

    state.history = Some(at(fan(2, 40), 45));
    let other_branch = pixel_rows(&state, MAX_LANES, STRIP_ROWS);
    assert_ne!(first.roles, other_branch.roles, "the focus moved");
    assert_ne!(keys(&first)[0], keys(&other_branch)[0]);
    through(&mut cache, &other_branch, &paint);
    assert!(
        cache.stats().rasterised > rasterised,
        "a focus change re-rasterises"
    );
}
