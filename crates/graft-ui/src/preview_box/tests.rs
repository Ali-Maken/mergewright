//! Ghost Preview box: docking (the box's own footer is the area's last row,
//! never over the screen footer), what it draws *between* those two rows,
//! planning, and too-small.

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
    let mut view = PreviewBox::new(sanitize_line("Preview: rebase feature/auth onto main"));
    view.lines = vec![
        sanitize_line("2 commits replayed · feature/auth 7f6e5d4 → ghost"),
        sanitize_line("working tree: 3 files updated · may conflict: src/auth.rs"),
        sanitize_line("undo: u restores feature/auth and the tree"),
    ];
    view.commands = vec![sanitize_line("git rebase refs/heads/main")];
    view
}

/// Render `view` into the whole of a `width × height` rect and return its
/// rows, trailing blanks trimmed.
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

/// Render `view` docked into a `width × height` screen through
/// [`preview_box_area`], with a stand-in screen footer on the last row —
/// what a caller actually does. Returns every row of the screen, untrimmed
/// at the start so column 0 lines up.
fn docked(view: &PreviewBox, theme: &Theme, width: u16, height: u16) -> Vec<String> {
    let screen = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(screen);
    buf.set_string(
        0,
        screen.bottom() - 1,
        "m merge  r rebase  screen footer",
        Style::default(),
    );
    (&PreviewBoxWidget { view, theme }).render(preview_box_area(screen, view), &mut buf);
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
    let inner = Rect::new(2, 10, 60, preview_rows(&view, 60));
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

/// The box is a box: at its own height every row between the title border
/// and the footer border is a `│ … │` content row, and the plan, the
/// command and the buttons are all on one of them. The old test asserted
/// only the first and last rows, which is how a widget that drew a
/// borderless hole between them passed (review Critical 1).
#[test]
fn every_interior_row_is_a_bordered_content_row() {
    let theme = theme();
    let view = rebase_view();
    let height = preview_rows(&view, 80);
    let drawn = rows(&view, &theme, 80, height);

    assert!(
        drawn[0].starts_with('╭') && drawn[0].ends_with('╮'),
        "{drawn:#?}"
    );
    let last = usize::from(height) - 1;
    assert!(
        drawn[last].starts_with('╰') && drawn[last].ends_with('╯'),
        "{drawn:#?}"
    );
    for (index, row) in drawn.iter().enumerate().take(last).skip(1) {
        assert!(
            row.starts_with('│') && row.ends_with('│'),
            "interior row {index} is not inside the box: {row:?}"
        );
    }
    assert!(drawn[1].contains("2 commits replayed"), "{drawn:#?}");
    assert!(drawn[4].contains("git rebase"), "{drawn:#?}");
    assert!(
        drawn[last - 1].contains("▸[ Cancel ]") && drawn[last - 1].contains("[ Confirm ]"),
        "the buttons belong on the last interior row: {drawn:#?}"
    );
}

/// Docked into a rect taller than it needs, the box still draws a box:
/// every interior row bordered, the buttons still right above the footer.
/// This is the shape that used to come out as a title, a hole and a footer.
#[test]
fn a_taller_rect_still_draws_a_box_on_every_interior_row() {
    let theme = theme();
    let view = rebase_view();
    let height = preview_rows(&view, 80) + 6;
    let drawn = rows(&view, &theme, 80, height);
    let last = usize::from(height) - 1;
    for (index, row) in drawn.iter().enumerate().take(last).skip(1) {
        assert!(
            row.starts_with('│') && row.ends_with('│'),
            "interior row {index} is not inside the box: {row:?}"
        );
    }
    assert!(
        drawn[last - 1].contains("▸[ Cancel ]"),
        "the buttons stay right above the footer: {drawn:#?}"
    );
}

/// The box's own footer (the bottom border with `←→ choose  ⏎ act  esc
/// cancel`) is the *last row of the area it was given*, and the screen
/// footer row stays untouched — the docking contract in the ticket's scope
/// item 4, through the function a caller really uses.
#[test]
fn preview_box_docks_above_the_footer_row_untouched() {
    let theme = theme();
    let view = rebase_view();
    let screen = Rect::new(0, 0, 80, 40);
    let area = preview_box_area(screen, &view);
    assert_eq!(
        area.height,
        preview_rows(&view, 80),
        "docked at its own height"
    );
    assert_eq!(
        area.bottom(),
        screen.bottom() - 1,
        "the footer row is left alone"
    );

    let drawn = docked(&view, &theme, 80, 40);
    let footer = usize::from(screen.height) - 1;
    assert!(
        drawn[footer].contains("screen footer"),
        "{:?}",
        drawn[footer]
    );
    let box_footer = footer - 1;
    assert!(
        drawn[box_footer].contains("choose") && drawn[box_footer].contains("cancel"),
        "expected the box's own footer on the area's last row: {:?}",
        drawn[box_footer]
    );
    let box_top = box_footer + 1 - usize::from(area.height);
    assert!(drawn[box_top].starts_with('╭'), "{:?}", drawn[box_top]);
    for (index, row) in drawn.iter().enumerate().take(box_footer).skip(box_top + 1) {
        assert!(
            row.starts_with('│') && row.ends_with('│'),
            "interior row {index} is not inside the box: {row:?}"
        );
    }
    assert!(
        drawn[..box_top].iter().all(String::is_empty),
        "the box drew above the rect it was docked into: {drawn:#?}"
    );
}

#[test]
fn planning_draws_predicting_in_place_of_lines() {
    let theme = theme();
    let mut view = rebase_view();
    view.planning = true;
    let drawn = rows(&view, &theme, 80, preview_rows(&view, 80));
    assert!(
        drawn.iter().any(|line| line.contains("predicting…")),
        "{drawn:#?}"
    );
    assert!(
        !drawn.iter().any(|line| line.contains("2 commits replayed")),
        "the plan's lines must not show while still planning: {drawn:#?}"
    );
}

/// A screen below [`PREVIEW_MIN`] gets one row for the message and nothing
/// else — and the decision is `preview_box_area`'s, so the widget is never
/// handed a box-sized rect it cannot fill.
#[test]
fn too_small_screen_shows_one_line_and_nothing_else() {
    let theme = theme();
    let view = rebase_view();
    let area = preview_box_area(Rect::new(0, 0, 30, 8), &view);
    assert_eq!(area.height, 1, "{area:?}");

    let drawn = docked(&view, &theme, 30, 8);
    // Everything but the screen's own footer row, which `docked` draws.
    let non_empty: Vec<&String> = drawn[..drawn.len() - 1]
        .iter()
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(non_empty.len(), 1, "{drawn:#?}");
    assert!(
        non_empty[0].contains("preview needs 40x12"),
        "{non_empty:?}"
    );
}

/// A rect too short for a frame draws the message rather than a frame with
/// nothing legible in it — the widget's own floor, below `PREVIEW_MIN`'s
/// screen floor.
#[test]
fn a_rect_below_the_box_floor_draws_the_message() {
    let theme = theme();
    let view = rebase_view();
    let drawn = rows(&view, &theme, 80, MIN_BOX_ROWS - 1);
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

/// ADR-0038 §1: the only constructor puts `Cancel` first and the cursor on
/// it, and `with_choices` puts the cursor back — there is no way to build a
/// preview whose acting choice is where the cursor rests.
#[test]
fn the_only_constructor_opens_on_cancel() {
    let view = PreviewBox::new(sanitize_line("Preview: reset main to 7f6e5d4"));
    assert_eq!(view.selected(), 0);
    assert_eq!(view.choices()[0].as_str(), "Cancel");

    let mut three = view.with_choices(vec![
        sanitize_line("Cancel"),
        sanitize_line("Reset"),
        sanitize_line("Reset and stash"),
    ]);
    assert_eq!(three.selected(), 0, "with_choices resets the cursor");
    three.left();
    assert_eq!(three.selected(), 0, "left of the first choice stays put");
    three.right();
    three.right();
    three.right();
    assert_eq!(three.selected(), 2, "right of the last choice stays put");
}

/// Not an assertion — dumps the docked box, the planning state and the
/// too-small screen, every row as drawn. **No blank-row filter**: the
/// filter this test used to apply is what hid the box the widget did not
/// draw (review Critical 1), so a capture is only evidence if it shows the
/// grid whole. Run by hand to regenerate: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft-ui preview_box::tests::capture_for_docs_shots`.
#[test]
fn capture_for_docs_shots() {
    let mut shots = graft_fixtures::Captures::new();
    let theme = theme();
    let mut out = String::new();
    out.push_str(
        "# docs/shots/m5-ui-preview-box.txt — Ghost Preview box\n\
         #\n\
         # What right looks like: the box is docked to the bottom of the\n\
         # screen at its own height, its own footer (\"←→ choose  ⏎ act  esc\n\
         # cancel\") on its last row and the screen's own footer row below it\n\
         # untouched. Every row between the two borders is a `│ … │` row —\n\
         # there is no hole in the box at any size. Every choice is\n\
         # bracketed; the cursor's carries `▸` and rests on `Cancel`\n\
         # (ADR-0038 §1). While `planning` is set, the plan's lines are\n\
         # replaced by `predicting…` — no plan is shown before it is\n\
         # computed. Below 40x12 of screen the box is one row: \"preview\n\
         # needs 40x12 · esc closes\".\n\
         #\n\
         # Every row of the screen is printed, blank ones included. Colour\n\
         # is unverified: this is a character grid (CLAUDE.md rule 5), so\n\
         # the reversed cursor button cannot be judged here — the `▸` can.\n\
         \n",
    );
    let ready = rebase_view();
    let mut planning = ready.clone();
    planning.planning = true;
    for (label, view, w, h) in [
        ("ready, docked in 80x24", ready.clone(), 80, 24),
        ("ready, docked in 160x48", ready.clone(), 160, 48),
        ("planning, docked in 80x24", planning, 80, 24),
        ("screen too small, 30x8", ready, 30, 8),
    ] {
        out.push_str(&format!("=== {label} ===\n"));
        for line in docked(&view, &theme, w, h) {
            out.push_str(&line);
            out.push('\n');
        }
        out.push('\n');
    }
    shots.check(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m5-ui-preview-box.txt"
        ),
        out,
    );
    shots.finish();
}
