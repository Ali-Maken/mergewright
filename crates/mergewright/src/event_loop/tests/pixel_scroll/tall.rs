//! The cursor row's second line at the Pixel tier (ease-pass 13, M3): the
//! cursor's strip is placed twice from one upload with disjoint rows, the
//! continuation band sits in the gap, strips below drop one row, and a `j`
//! that keeps the plan rasterises nothing. Read off the command bytes; what
//! a terminal draws is the user's to judge (ADR-0012).

use mergewright_core::capabilities::PixelProtocol;
use mergewright_term::kitty::EMPHASIS_ID_BIT;
use ratatui::layout::Rect;

use super::{branchy, drive, js, planned, press, repainted, states, transmits};
use crate::event_loop::graphics::{Graphics, NoReplies};
use crate::session::Session;
use crate::update::{AppState, update};
use mergewright_core::event::KeyCode;

mod capture;

/// One Kitty placement: screen row, image id, placement id, source `y`
/// in pixels and rows tall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Put {
    row: u16,
    id: u32,
    p: u32,
    y: u32,
    rows: u32,
}

/// Every `a=p` in `stream`, with the screen row its cursor move names.
fn puts(stream: &str) -> Vec<Put> {
    stream
        .split("\x1b7\x1b[")
        .skip(1)
        .filter_map(|chunk| {
            let (row, rest) = chunk.split_once(';')?;
            let command = rest.split_once("\x1b_G")?.1;
            let header = command.split([';', '\x1b']).next()?;
            let keys: Vec<&str> = header.split(',').collect();
            if !keys.contains(&"a=p") {
                return None;
            }
            let get = |name: &str| -> Option<u32> {
                keys.iter()
                    .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))?
                    .parse()
                    .ok()
            };
            Some(Put {
                row: row.parse::<u16>().ok()? - 1,
                id: get("i")?,
                p: get("p")?,
                y: get("y")?,
                rows: get("r")?,
            })
        })
        .collect()
}

/// The graph column the view would give `state`: from row 3, its pane tall.
fn column(state: &AppState) -> Rect {
    Rect {
        x: 2,
        y: 3,
        width: 60,
        height: u16::try_from(state.graph_pane_rows()).unwrap(),
    }
}

fn frame(graphics: &mut Graphics<NoReplies>, session: &Session, state: &AppState) -> String {
    let mut out = Vec::new();
    graphics.frame(session, state, Some(column(state)), &mut out);
    String::from_utf8_lossy(&out).into_owned()
}

/// Branchy history (49 rows, strips 0..32 and 32..49 on screen), cursor on
/// row `cursor`, no uncommitted row.
fn at(protocol: PixelProtocol, cursor: usize) -> (Session, AppState) {
    let (session, mut state) = branchy(protocol, false);
    state.history.as_mut().unwrap().cursor = cursor;
    assert!(
        !state.uncommitted_row_pinned(),
        "premise: no uncommitted row"
    );
    (session, state)
}

#[test]
fn kitty_places_the_cursor_strip_twice_the_band_between_and_the_next_strip_lower() {
    let (session, state) = at(PixelProtocol::Kitty, 5);
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    let cell = session.rendering.cell_size.height;
    let graph = column(&state);
    let shown = u32::from(graph.height) - 1;
    assert!(shown > 32, "premise: both strips on screen ({shown} lines)");

    let check = |stream: &str, cursor: u32| {
        let all = puts(stream);
        let strips: Vec<Put> = all
            .iter()
            .copied()
            .filter(|put| put.id & EMPHASIS_ID_BIT == 0)
            .collect();
        let first = strips[0].id;
        let line = |n: u32| graph.y + u16::try_from(n).unwrap();
        assert_eq!(
            strips,
            [
                // Rows 0..=cursor where they were.
                Put {
                    row: line(0),
                    id: first,
                    p: 1,
                    y: 0,
                    rows: cursor + 1,
                },
                // The rest of the one upload, one row lower: disjoint rows.
                Put {
                    row: line(cursor + 2),
                    id: first,
                    p: 2,
                    y: (cursor + 1) * cell,
                    rows: 32 - (cursor + 1),
                },
                // The next strip, one row lower than its rows say.
                Put {
                    row: line(33),
                    id: strips[2].id,
                    p: 1,
                    y: 0,
                    rows: shown - 32,
                },
            ],
            "cursor {cursor}"
        );
        // The second line's row holds exactly one image: the band.
        let band: Vec<&Put> = all
            .iter()
            .filter(|put| put.row == line(cursor + 1))
            .collect();
        assert_eq!(band.len(), 1, "cursor {cursor}: {all:?}");
        assert_eq!((band[0].rows, band[0].y), (1, 0));
        assert_ne!(band[0].id & EMPHASIS_ID_BIT, 0, "known by its key");
    };
    check(&frame(&mut graphics, &session, &state), 5);

    // `j` within chain 1: the same plan, the split moves, nothing re-sent.
    let moved = update(&state, press(KeyCode::Char('j'))).0;
    assert_eq!(planned(&moved).0, planned(&state).0, "premise: same plan");
    let stream = frame(&mut graphics, &session, &moved);
    check(&stream, 6);
    let strip_transmits = transmits(&stream)
        .iter()
        .filter(|sent| !sent.is_emphasis())
        .count();
    assert_eq!(strip_transmits, 0, "a j inside the strip sends no strip");
}

#[test]
fn iterm2_writes_two_bands_of_the_cursor_strip_the_band_and_the_lower_strip() {
    let (session, state) = at(PixelProtocol::Iterm2, 5);
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    let graph = column(&state);
    let stream = frame(&mut graphics, &session, &state);
    // Each band: `ESC 7`, a cursor move to its cell, the OSC 1337.
    let bands: Vec<(u16, String)> = stream
        .split("\x1b7\x1b[")
        .skip(1)
        .filter_map(|chunk| {
            let (row, rest) = chunk.split_once(';')?;
            let rest = rest.split_once("H\x1b]1337;File=")?.1;
            let height = rest.split(";height=").nth(1)?.split(';').next()?;
            Some((row.parse::<u16>().ok()? - 1 - graph.y, height.to_owned()))
        })
        .collect();
    let shown = graph.height - 1;
    assert_eq!(
        bands,
        [
            (0, "6".to_owned()),
            (7, "26".to_owned()),
            (33, (shown - 32).to_string()),
            (6, "1".to_owned()),
        ]
    );
    // A `j`: both crops of the cursor strip are new bands (settle test 2).
    let before = graphics.bands_encoded();
    let moved = update(&state, press(KeyCode::Char('j'))).0;
    frame(&mut graphics, &session, &moved);
    assert!(graphics.bands_encoded() >= before + 2);
}

/// Held `j` on iTerm2 (was `iterm2_bands_are_composites`, before the tall
/// row): each frame writes the cursor strip's two composited crops, the
/// band and the other strip. Both crops move with the cursor, so every `j`
/// encodes two PNGs at least (settle test 2) — the cost the lag script
/// measures; a strip the cursor is not in is encoded again only where its
/// selection paint changed.
#[test]
fn iterm2_held_j_writes_four_bands_and_re_encodes_the_cursor_strip_s_crops() {
    let (session, state) = branchy(PixelProtocol::Iterm2, false);
    let run = drive(&session, state, js(20));
    for frame in &run.frames {
        assert_eq!(frame.matches("\x1b]1337;File=").count(), 4, "2 + band + 1");
        assert!(transmits(frame).is_empty(), "no Kitty bytes on iTerm2");
    }
    let repainted = u64::try_from(repainted(&run.paints)).expect("few");
    let encoded = run.graphics.bands_encoded();
    eprintln!("iterm2: {encoded} encodes, {repainted} strips repainted over 20 j");
    assert!(encoded >= 4 + 2 * 20, "both crops every j: {encoded}");
    assert!(encoded <= 4 + 3 * 20 + repainted, "{encoded}");
}

#[test]
fn a_j_that_keeps_the_plan_rasterises_no_strip_above_or_below() {
    // Cursor in the second strip: the first is wholly above it.
    let (session, state) = at(PixelProtocol::Kitty, 33);
    let events = vec![press(KeyCode::Char('j')); 8];
    let walk = states(state, &events);
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    frame(&mut graphics, &session, &walk[0]);
    let mut kept = 0;
    for pair in walk.windows(2) {
        let before = graphics.strip_stats().rasterised;
        frame(&mut graphics, &session, &pair[1]);
        if planned(&pair[1]).0 == planned(&pair[0]).0 {
            kept += 1;
            assert_eq!(graphics.strip_stats().rasterised, before, "no raster");
        }
    }
    assert!(kept >= 4, "premise: most j keep the plan ({kept} of 8)");
}
