//! `docs/shots/ease-pass/main-pixel-120.txt`: the Pixel tier's main screen
//! with the cursor row's second line, as text — the text layer the view
//! draws, and a line-by-line map of what the Kitty commands place in the
//! graph column. The pixels themselves are not in it: a character grid
//! cannot hold an image, and colour is unverified (rule 5). How they look
//! is the user's to judge in Ghostty (rule 6).

use mergewright_core::capabilities::PixelProtocol;
use mergewright_core::event::KeyCode;
use mergewright_term::kitty::EMPHASIS_ID_BIT;
use mergewright_ui::graph_screen::GraphicsStatus;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

use super::super::{branchy, press, transmits};
use super::{Put, puts};
use crate::event_loop::graphics::{Graphics, NoReplies};
use crate::session::{Clock, Session};
use crate::update::{AppState, update};
use crate::view::view_with_graphics;

/// The text layer, trimmed, and the graph column the frame asked pixels for.
fn text_layer(session: &Session, state: &AppState) -> (String, Rect) {
    let mut terminal = Terminal::new(TestBackend::new(state.size.columns, state.size.rows))
        .expect("TestBackend reports its size");
    let mut graph = None;
    terminal
        .draw(|frame| graph = view_with_graphics(session, state, &GraphicsStatus::Ok, frame))
        .expect("TestBackend cannot fail to draw");
    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        let line: String = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect();
        text.push_str(line.trim_end());
        text.push('\n');
    }
    (text, graph.expect("the pixel tier draws pixels"))
}

/// What covers each line of the graph column: a strip image and the strip
/// row it shows there, or the continuation band. Emphasis pieces (the
/// thread, drawn over the strips) are counted, not mapped.
fn placement_map(stream: &str, graph: Rect, cell_height: u32) -> String {
    let all = puts(stream);
    let strips: Vec<u32> = {
        let mut ids: Vec<u32> = all
            .iter()
            .filter(|put| put.id & EMPHASIS_ID_BIT == 0)
            .map(|put| put.id)
            .collect();
        ids.dedup();
        ids
    };
    let mut lines = vec![String::from("-"); usize::from(graph.height)];
    let mut pieces = 0;
    for &Put {
        row, id, y, rows, ..
    } in &all
    {
        let Some(line) = row.checked_sub(graph.y).map(usize::from) else {
            continue;
        };
        if id & EMPHASIS_ID_BIT == 0 {
            let name = strips.iter().position(|&strip| strip == id).unwrap_or(0);
            let first = y / cell_height;
            for k in 0..rows {
                let at = line + usize::try_from(k).expect("small");
                lines[at] = format!("strip {} row {}", ["A", "B", "C"][name], first + k);
            }
        } else if rows == 1 && lines[line] == "-" && is_band(&all, row) {
            lines[line] = String::from("band: the lanes straight through");
        } else {
            pieces += 1;
        }
    }
    let mut out = String::new();
    for (line, what) in lines.iter().enumerate() {
        out.push_str(&format!("{line:>3}  {what}\n"));
    }
    out.push_str(&format!(
        "({pieces} emphasis placements over the strips: the thread)\n"
    ));
    out
}

/// The band is the only image on its line: no strip covers it.
fn is_band(all: &[Put], row: u16) -> bool {
    !all.iter().any(|put| {
        put.id & EMPHASIS_ID_BIT == 0
            && (put.row..put.row + u16::try_from(put.rows).expect("rows fit")).contains(&row)
    })
}

#[test]
fn capture_the_pixel_main_screen_with_the_cursor_row_s_second_line() {
    let (session, mut state) = branchy(PixelProtocol::Kitty, false);
    state.harness.clock = Clock::Fixed(mergewright_fixtures::PINNED_NOW);
    state.history.as_mut().expect("history").cursor = 5;
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    let cell = session.rendering.cell_size.height;

    let (before, graph) = text_layer(&session, &state);
    let mut out = Vec::new();
    graphics.frame(&session, &state, Some(graph), &mut out);
    let first = placement_map(&String::from_utf8_lossy(&out), graph, cell);

    let moved = update(&state, press(KeyCode::Char('j'))).0;
    let (after, graph_after) = text_layer(&session, &moved);
    assert_eq!(graph, graph_after);
    let mut out = Vec::new();
    graphics.frame(&session, &moved, Some(graph), &mut out);
    let stream = String::from_utf8_lossy(&out);
    let sent = transmits(&stream);
    let strips_sent = sent.iter().filter(|t| !t.is_emphasis()).count();
    let second = placement_map(&stream, graph, cell);

    let body = format!(
        "# ease-pass/13: the Pixel tier (Kitty, as Ghostty is probed) at 120x40, drawn by\n\
         # `view::view_with_graphics` and `event_loop::graphics` from an AppState: six\n\
         # chains of 8 under an octopus merge (crates/mergewright/src/event_loop/tests/\n\
         # pixel_scroll/tall/capture.rs). The TEXT LAYER and a MAP of the placements the\n\
         # Kitty commands make in the graph column ({}x{} cells from column {}, row {}).\n\
         # No pixels, no colour (rule 5): how the lanes look is yours to judge in Ghostty.\n\
         #\n\
         # Frame 1, the cursor on the 6th row. What right looks like: under the cursor\n\
         # row its second line (`Test · N years ago · <sha7>`), every row below one line\n\
         # lower; in the map, strip A shows rows 0..=5, the band takes the second line's\n\
         # line, strip A's rows 6.. continue one line lower (the same image placed twice),\n\
         # and strip B follows, one line lower than its rows say. No line is blank.\n\n\
         {before}\n{first}\n\
         # Frame 2, after `j`. What right looks like: the second line now under the 7th\n\
         # row; strip A split one row later; strips sent this frame: {strips_sent} (a `j`\n\
         # inside one strip moves placements only).\n\n\
         {after}\n{second}",
        graph.width, graph.height, graph.x, graph.y,
    );
    assert_eq!(strips_sent, 0, "a j inside a strip sends no strip");
    mergewright_fixtures::capture(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots/ease-pass/main-pixel-120.txt"),
        body,
    );
}
