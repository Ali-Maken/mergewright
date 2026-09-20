//! Ghost Preview box: docking (the box's own footer is the area's last row,
//! never over the screen footer), planning, and too-small.

use graft_term::sanitize_line;

use super::*;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(graft_core::capabilities::ColorDepth::TrueColor, false),
    )
}

fn rebase_view() -> PreviewBox {
    PreviewBox {
        title: sanitize_line("Preview: rebase feature/auth onto main"),
        lines: vec![
            sanitize_line("2 commits replayed · feature/auth 7f6e5d4 → ghost"),
            sanitize_line("working tree: 3 files updated · may conflict: src/auth.rs"),
            sanitize_line("undo: u restores feature/auth and the tree"),
        ],
        commands: vec![sanitize_line("git rebase refs/heads/main")],
        choices: vec![sanitize_line("Cancel"), sanitize_line("Confirm")],
        cursor: 0,
        planning: false,
    }
}

fn rows(view: &PreviewBox, theme: &Theme, width: u16, height: u16) -> Vec<String> {
    let frame = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(frame);
    (&PreviewBoxWidget { view, theme }).render(frame, &mut buf);
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buf[(x, y)].symbol().to_owned())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// Every widget in this ticket must draw nothing outside its own `Rect`: a
/// box docked into a small area at the graph's bottom, drawn at an offset,
/// never bleeds past the columns or rows it was given.
#[test]
fn preview_box_stays_inside_its_rect() {
    let theme = theme();
    let view = rebase_view();
    let outer = Rect::new(0, 0, 80, 24);
    let inner = Rect::new(2, 10, 60, preview_rows(&view, 60).max(PREVIEW_MIN.1));
    let mut buf = Buffer::empty(outer);
    (&PreviewBoxWidget {
        view: &view,
        theme: &theme,
    })
        .render(inner, &mut buf);
    for y in 0..outer.height {
        for x in 0..outer.width {
            let inside = x >= inner.x
                && x < inner.x + inner.width
                && y >= inner.y
                && y < inner.y + inner.height;
            if !inside {
                assert_eq!(
                    buf[(x, y)].symbol(),
                    " ",
                    "drew outside its rect at ({x}, {y})"
                );
            }
        }
    }
}

/// The box's own footer (the bottom border with `←→ choose  ⏎ act  esc
/// cancel`) is the *last row of the area it was given*, and the screen
/// footer row (one row below the docked area, in this test's stand-in)
/// stays untouched — the docking contract in the ticket's scope item 4.
#[test]
fn preview_box_docks_above_the_footer_row_untouched() {
    let theme = theme();
    let view = rebase_view();
    let box_height = preview_rows(&view, 80).max(PREVIEW_MIN.1);
    let screen = Rect::new(0, 0, 80, 40);
    let footer_y = screen.bottom() - 1;
    let docked = Rect::new(0, footer_y - box_height, 80, box_height);
    let mut buf = Buffer::empty(screen);
    buf.set_string(
        0,
        footer_y,
        "m merge  r rebase  screen footer",
        ratatui::style::Style::default(),
    );
    (&PreviewBoxWidget {
        view: &view,
        theme: &theme,
    })
        .render(docked, &mut buf);
    let footer_row: String = (0..screen.width)
        .map(|x| buf[(x, footer_y)].symbol().to_owned())
        .collect();
    assert!(
        footer_row.contains("screen footer"),
        "the preview box drew over the screen's own footer row: {footer_row:?}"
    );
    let last_box_row: String = (0..screen.width)
        .map(|x| buf[(x, docked.bottom() - 1)].symbol().to_owned())
        .collect();
    assert!(
        last_box_row.contains("choose") && last_box_row.contains("cancel"),
        "expected the box's own footer on the last row of its area: {last_box_row:?}"
    );
}

#[test]
fn planning_draws_predicting_in_place_of_lines() {
    let theme = theme();
    let mut view = rebase_view();
    view.planning = true;
    let height = preview_rows(&view, 80).max(PREVIEW_MIN.1);
    let drawn = rows(&view, &theme, 80, height);
    assert!(
        drawn.iter().any(|line| line.contains("predicting…")),
        "{drawn:#?}"
    );
    assert!(
        !drawn.iter().any(|line| line.contains("2 commits replayed")),
        "the plan's lines must not show while still planning: {drawn:#?}"
    );
}

#[test]
fn too_small_shows_one_line_and_nothing_else() {
    let theme = theme();
    let view = rebase_view();
    let drawn = rows(&view, &theme, 30, 8);
    let non_empty: Vec<&String> = drawn.iter().filter(|line| !line.is_empty()).collect();
    assert_eq!(non_empty.len(), 1, "{drawn:#?}");
    assert!(
        non_empty[0].contains("preview needs 40x12"),
        "{non_empty:?}"
    );
}

#[test]
fn preview_rows_accounts_for_every_line() {
    let view = rebase_view();
    // top border + 3 lines + 1 command + choices row + bottom border.
    assert_eq!(preview_rows(&view, 80), 1 + 3 + 1 + 1 + 1);
}

/// Not an assertion — dumps the ready, planning and too-small states at
/// 80×24 and 160×48 (planning/too-small at a size that fits their message).
/// Run by hand to regenerate: `cargo test -p graft-ui
/// preview_box::tests::capture_for_docs_shots -- --ignored --nocapture`.
#[test]
#[ignore = "writes docs/shots/m5-ui-preview-box.txt; run by hand to regenerate"]
fn capture_for_docs_shots() {
    let theme = theme();
    let mut out = String::new();
    out.push_str(
        "# docs/shots/m5-ui-preview-box.txt — Ghost Preview box\n\
         #\n\
         # What right looks like: docked to the bottom of its area, its own\n\
         # footer (\"←→ choose  ⏎ act  esc cancel\") on the area's last row,\n\
         # never the screen's own footer row. While `planning` is set, the\n\
         # lines are replaced by `predicting…` — no plan is shown before it\n\
         # is computed. Below 40x12 the box draws one line, \"preview needs\n\
         # 40x12 · esc closes\", and nothing else.\n\
         \n",
    );
    let ready = rebase_view();
    let mut planning = ready.clone();
    planning.planning = true;
    for (label, view, w, h) in [
        ("ready, 80x24", ready.clone(), 80, 24),
        ("ready, 160x48", ready.clone(), 160, 48),
        ("planning, 80x24", planning.clone(), 80, 24),
        ("too small, 30x8", ready, 30, 8),
    ] {
        out.push_str(&format!("=== {label} ===\n"));
        for line in rows(&view, &theme, w, h) {
            if !line.trim().is_empty() {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out.push('\n');
    }
    std::fs::write(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m5-ui-preview-box.txt"
        ),
        out,
    )
    .expect("docs/shots is writable");
}
