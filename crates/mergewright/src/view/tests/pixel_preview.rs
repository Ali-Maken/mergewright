//! m8-polish-and-ship/17c, ADR-0060: at the pixel tier an open Ghost
//! Preview draws the whole graph column at the Unicode tier. The tier in
//! force does not change — the header's `gfx` field and `F3` keep it.

use mergewright_core::capabilities::{PixelProtocol, RenderingTier};
use mergewright_ui::graph_screen::{GraphicsStatus, row_at};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::scrolled_preview;
use crate::session::Session;
use crate::session::for_tests::session;
use crate::update::AppState;
use crate::view::frame::{graph_tier, primary_pane_area, screen_area};
use crate::view::{graph_view, tier_in_force, view_with_graphics};

/// A session at the pixel tier over Kitty.
fn pixel_session() -> Session {
    let mut session = session();
    session.rendering.tier = RenderingTier::Pixel;
    session.capabilities.pixel_protocol = Some(PixelProtocol::Kitty);
    session
}

/// A preview with one ghost row, its plan landed, at 100x30.
fn previewing() -> AppState {
    previewing_at().0
}

/// [`previewing`], and the subject of a real row five below the viewport's
/// top — not the ghost, which hangs three rows down.
fn previewing_at() -> (AppState, String) {
    let (state, top, _) = scrolled_preview(120, |height| 2 * height, 3);
    assert!(
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty()),
        "the fixture's plan landed, so the picture has rows"
    );
    (state, format!("Commit {}", 120 - (top + 5)))
}

/// The same history and viewport with the preview closed.
fn closed(previewing: &AppState) -> AppState {
    AppState {
        preview: None,
        ..previewing.clone()
    }
}

/// Draws `state` through the pixel-aware entry point; the cells, and the
/// rectangle the frame asked pixels for.
fn draw_at(session: &Session, state: &AppState) -> (String, Option<ratatui::layout::Rect>) {
    let mut terminal = Terminal::new(TestBackend::new(state.size.columns, state.size.rows))
        .expect("TestBackend reports its size");
    let mut pixels = None;
    terminal
        .draw(|frame| pixels = view_with_graphics(session, state, &GraphicsStatus::Ok, frame))
        .expect("TestBackend cannot fail to draw");
    (terminal.backend().to_string(), pixels)
}

#[test]
fn graph_tier_is_unicode_only_under_a_pixel_preview() {
    let pixel = pixel_session();
    let open = previewing();

    assert_eq!(
        graph_tier(&pixel, &closed(&open)),
        tier_in_force(&pixel, &closed(&open)),
        "no preview: the tier in force"
    );
    assert_eq!(
        graph_tier(&pixel, &open),
        RenderingTier::Unicode,
        "pixel preview: unicode"
    );
    assert_eq!(
        tier_in_force(&pixel, &open),
        RenderingTier::Pixel,
        "the tier in force did not change"
    );

    // A preview whose plan has not landed draws the live rows: no ghost
    // row on screen, nothing to hold the column for.
    let mut pending = open.clone();
    if let Some(preview) = pending.preview.as_mut() {
        preview.rows.clear();
    }
    assert_eq!(
        graph_tier(&pixel, &pending),
        RenderingTier::Pixel,
        "pending plan: pixel"
    );

    // `F3` to ASCII during the preview: min(tier in force, Unicode).
    let ascii = AppState {
        renderer_override: Some(RenderingTier::Ascii),
        ..open.clone()
    };
    assert_eq!(
        graph_tier(&pixel, &ascii),
        RenderingTier::Ascii,
        "F3 to ascii: ascii"
    );

    // Below pixel the rule changes nothing.
    let unicode = session();
    assert_eq!(graph_tier(&unicode, &open), tier_in_force(&unicode, &open));
}

/// The click's half of the column (`view::hit`, `update::geometry`) reads
/// no tier; the drawn half must now put lane glyphs in exactly the cells
/// the hit-test measures as the graph column, on a real (ghost-free) row.
#[test]
fn preview_hit_test_uses_the_graph_tier() {
    let pixel = pixel_session();
    let (state, real) = previewing_at();
    let (cells, pixels) = draw_at(&pixel, &state);
    assert_eq!(
        pixels, None,
        "no pixel rectangle while the preview holds the column\n{cells}"
    );

    let lines: Vec<&str> = cells.lines().collect();
    let pane = primary_pane_area(&state, screen_area(&state)).expect("the graph is on screen");
    let view = graph_view(&state, state.harness.clock.now_seconds());
    let (y, line) = lines
        .iter()
        .enumerate()
        .find(|(_, line)| line.contains(real.as_str()))
        .expect("a real row is drawn");
    let y = u16::try_from(y).expect("fits");
    let (entry, columns) = row_at(&view, pane, y).expect("the hit-test finds a row there");
    assert!(
        entry.subject.as_str().contains(real.as_str()),
        "the hit row is the drawn row"
    );
    let graph: String = line
        .chars()
        .skip(1) // TestBackend's leading quote
        .skip(usize::from(pane.x))
        .take(usize::from(columns.subject_x - pane.x))
        .collect();
    assert!(
        graph.chars().any(|cell| cell != ' '),
        "the graph column the hit-test measures holds drawn glyphs: {graph:?}\n{cells}"
    );
}

/// `docs/shots/m8/polish-ghost-pixel-120x36.txt`: the **text layer** of a
/// pixel-tier frame, closed and then with the preview open. What it can
/// show: closed, the graph column's cells are blank (the strips are images
/// over them, which a character grid cannot hold); open, the same column
/// holds the Unicode graph — `●` real nodes, `○` the ghost, `│` lanes —
/// and the header still says `gfx ok`. What it cannot show: that no image
/// sits over the text in a real terminal (the event-loop tests assert no
/// placement in the bytes), and colour (rule 5).
#[test]
fn capture_a_pixel_preview_for_docs_shots() {
    let pixel = pixel_session();
    let mut open = previewing();
    // Ages against the pinned instant, not the wall (m8/19d-b2).
    open.harness.clock = crate::session::Clock::Fixed(mergewright_fixtures::PINNED_NOW);
    open.size = crate::update::Size {
        columns: 120,
        rows: 36,
    };
    let (before, _) = draw_at(&pixel, &closed(&open));
    let (during, pixels) = draw_at(&pixel, &open);
    assert_eq!(pixels, None, "{during}");
    let grid = |cells: &str| {
        let mut out = String::new();
        for line in cells.lines() {
            let line = line.strip_prefix('"').unwrap_or(line);
            let line = line.strip_suffix('"').unwrap_or(line);
            out.push_str(line.trim_end());
            out.push('\n');
        }
        out
    };
    let body = format!(
        "# m8/polish-ghost-pixel-120x36: drawn by `view::view_with_graphics` at the pixel\n\
         # tier (Kitty) from an AppState (crates/mergewright/src/view/tests/pixel_preview.rs).\n\
         # The TEXT LAYER only: a character grid, no images, no colour (rule 5 —\n\
         # colour unverified). Whether a strip image is placed is asserted on the\n\
         # bytes by event_loop::tests::pixel_scroll, not shown here.\n\
         #\n\
         # Frame 1, preview closed. Right: the graph column's cells are blank — the\n\
         # strips are drawn there as images; the header ends `gfx ok`.\n\n\
         {}\n\
         # Frame 2, the same screen with a rebase preview open (ADR-0060). Right:\n\
         # the graph column now holds the Unicode graph in text — `●` real nodes,\n\
         # `○` the ghost row `replayed work`, `│` lanes — one picture, no blank\n\
         # column; the header still ends `gfx ok` (the tier in force is Pixel).\n\n\
         {}",
        grid(&before),
        grid(&during),
    );
    mergewright_fixtures::capture(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots/m8/polish-ghost-pixel-120x36.txt"),
        body,
    );
}

#[test]
fn gfx_field_holds_during_a_preview() {
    let pixel = pixel_session();
    let open = previewing();
    let header = |cells: &str| cells.lines().next().unwrap_or_default().to_owned();

    let (before, _) = draw_at(&pixel, &closed(&open));
    let (during, _) = draw_at(&pixel, &open);
    assert!(header(&before).contains("gfx ok"), "{before}");
    assert!(
        header(&during).contains("gfx ok"),
        "the tier in force did not change, so neither does gfx\n{during}"
    );
}
