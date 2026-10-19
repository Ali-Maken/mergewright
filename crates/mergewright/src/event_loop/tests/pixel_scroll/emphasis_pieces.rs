//! Emphasis in per-column pieces on Kitty (ADR-0053 amendment 2026-10-10,
//! m8 05 Part 2): a `j` sends only the pieces whose key the terminal does not
//! hold, and places the rest by id.
//!
//! What a frame needs is worked out here from the view alone
//! (`emphasis_pieces` over each strip's layers); what went is read back from
//! the bytes. The two are counted against each other.

use std::collections::BTreeSet;

use mergewright_core::capabilities::PixelProtocol;
use mergewright_graph::raster::cache::STRIP_ROWS;
use mergewright_graph::raster::{continuation_key, emphasis_pieces};
use mergewright_term::kitty::EMPHASIS_ID_BIT;

use super::super::*;
use super::{branchy, drive, js, press, states, transmits};
use crate::view::strips::{StripPaint, pixel_rows};

/// Image ids placed in `stream`.
pub(super) fn placed(stream: &str) -> Vec<u32> {
    stream
        .split("a=p,i=")
        .skip(1)
        .filter_map(|rest| rest.split(',').next()?.parse().ok())
        .collect()
}

/// Emphasis image ids placed in `stream`, sorted.
pub(super) fn emphasis_placed(stream: &str) -> Vec<u32> {
    let mut ids: Vec<u32> = placed(stream)
        .into_iter()
        .filter(|id| id & EMPHASIS_ID_BIT != 0)
        .collect();
    ids.sort_unstable();
    ids
}

/// The pieces `state`'s frame needs, as (strip first row, piece key). The
/// strip's backdrop key is stood in by its first row: a piece's own key
/// holds everything its pixels are drawn from, roles included. The cursor
/// row's continuation band (ease-pass 13) is uploaded the same way, known
/// by its key alone: it is in the set under `usize::MAX`.
fn pieces_of(session: &Session, state: &AppState) -> BTreeSet<(usize, u64)> {
    let pixels = pixel_rows(state, crate::view::max_lanes(state), STRIP_ROWS);
    let paint = StripPaint::for_frame(session, state);
    let cursor = state.history.as_ref().map(|history| history.cursor);
    let mut needed = BTreeSet::new();
    for strip in &pixels.strips {
        strip.with_layers(&pixels, &paint, |plain, selection| {
            for piece in emphasis_pieces(plain, selection) {
                needed.insert((strip.first_row, piece.key));
            }
            // No uncommitted row in these runs: strip row = history row.
            let row = cursor.and_then(|cursor| cursor.checked_sub(strip.first_row));
            if let Some(row) = row.filter(|&row| row < plain.rows.len()) {
                let band = continuation_key(plain, selection, row).expect("one lit per row");
                needed.extend(band.map(|key| (usize::MAX, key)));
            }
        });
    }
    needed
}

/// Per frame after the first: (emphasis transmits, pieces needed that no
/// earlier frame needed). A strip may go again when the focus moves onto
/// another branch (ease-pass 13: roles re-key it); its pieces' keys hold
/// the roles' strokes and colours, so the stand-in still tells them apart.
fn sent_and_new(session: &Session, state: &AppState, events: Vec<AppEvent>) -> Vec<(usize, usize)> {
    let walk = states(state.clone(), &events);
    let run = drive(session, state.clone(), events);
    let mut held: BTreeSet<(usize, u64)> = pieces_of(session, &walk[0]);
    let mut counts = Vec::new();
    for (k, frame) in run.frames.iter().enumerate().skip(1) {
        let sent = transmits(frame).iter().filter(|t| t.is_emphasis()).count();
        let needed = pieces_of(session, &walk[k]);
        let new = needed.difference(&held).count();
        held.extend(needed);
        counts.push((sent, new));
    }
    counts
}

/// Held `j` down the merge-heavy fan: every frame sends exactly the pieces
/// whose key the terminal did not hold (here only where the thread crosses
/// into the next chain). On linear history nothing goes after the first
/// frame: the whole graph is the thread, and the cursor's own row is not
/// in the paint (`Lit` holds only emphasis and thread), so no piece moves.
#[test]
fn transmits_equal_changed_pieces() {
    let (session, state) = branchy(PixelProtocol::Kitty, false);
    let counts = sent_and_new(&session, &state, js(20));
    eprintln!("fan (sent, new) per j: {counts:?}");
    for (k, (sent, new)) in counts.iter().enumerate() {
        assert_eq!(sent, new, "j {}: transmits vs pieces not held", k + 1);
    }
    let moved = counts.iter().filter(|(sent, _)| *sent > 0).count();
    assert!(moved > 0, "premise: the thread moved");
    assert!(moved < counts.len(), "a j inside a chain sends no piece");

    let (session, mut state) = branchy(PixelProtocol::Kitty, false);
    let mut history = crate::view::tests::history_of(60, 0);
    history.strip_rows = Some(STRIP_ROWS);
    state.history = Some(history);
    assert!(
        !pieces_of(&session, &state).is_empty(),
        "premise: emphasis draws"
    );
    let counts = sent_and_new(&session, &state, js(20));
    assert!(counts.iter().all(|c| *c == (0, 0)), "linear: {counts:?}");
}

/// `j` down the fan, then `k` back up as far: every piece the way back needs
/// was placed on the way down, so the way back sends nothing.
#[test]
fn back_and_forth_retransmits_nothing() {
    let (session, state) = branchy(PixelProtocol::Kitty, false);
    let mut events = js(20);
    events.extend(vec![press(KeyCode::Char('k')); 20]);
    let counts = sent_and_new(&session, &state, events);
    let (down, back) = counts.split_at(20);
    assert!(
        down.iter().any(|(sent, _)| *sent > 0),
        "premise: j sent pieces"
    );
    let sent_back: usize = back.iter().map(|(sent, _)| sent).sum();
    assert_eq!(sent_back, 0, "k back over held pieces: {back:?}");
}
