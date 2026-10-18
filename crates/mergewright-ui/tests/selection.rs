//! Selection on the criss-cross merge, 80×24: the selected commit's thread
//! drawn bold, every other lane dim, and in Focus mode the rows off the
//! thread faded as a whole (DESIGN.md §5.2).
//!
//! Each snapshot is the screen's text, then per graph row the modifier of
//! each graph cell (`b` bold, `d` dim, `.` neither; blank cells `.`) and,
//! after `|`, the subject's. Read them as pictures; do not just re-accept.

mod support;

use mergewright_core::capabilities::ColorDepth;
use mergewright_fixtures::Shape;
use mergewright_graph::{Ancestry, Kin};
use mergewright_ui::graph_column::{GraphLine, RowPaint};
use mergewright_ui::graph_screen::{CHROME_ROWS, GUTTER, GraphScreen};
use mergewright_ui::theme::ThemeKind;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Modifier;

use support::corpus::lay_out;
use support::{screen, theme};

const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
/// `beta: work`: two descendants above it, one unrelated row, the root below.
const SELECTED: usize = 4;

#[test]
fn ancestry_highlight_and_focus_on_criss_cross() {
    let shape = Shape::CrissCrossMerge;
    let dir = mergewright_fixtures::borrow(shape);
    let laid = lay_out(shape, &dir);
    let selected = laid.rows[SELECTED].commit.id;
    let ancestry = Ancestry::of(&laid.rows, selected).expect("selected row is laid out");
    let kin: Vec<Kin> = (0..laid.rows.len()).map(|row| ancestry.kin(row)).collect();
    assert!(
        kin.contains(&Kin::Unrelated),
        "the picture shows a faded row"
    );

    for (depth_name, depth) in [
        ("truecolor", ColorDepth::TrueColor),
        ("16", ColorDepth::Palette16),
    ] {
        for focus in [false, true] {
            let lines: Vec<GraphLine<'_>> = laid
                .lines()
                .into_iter()
                .enumerate()
                .map(|(index, line)| GraphLine {
                    thread: Some(ancestry.lit(index).clone()),
                    paint: if focus && ancestry.kin(index) == Kin::Unrelated {
                        RowPaint::Dimmed
                    } else {
                        line.paint
                    },
                    ..line
                })
                .collect();
            let height = usize::from(ROWS - CHROME_ROWS);
            let mut view = screen::view(&lines, height, SELECTED);
            view.filters.focused = focus;
            let theme = theme(ThemeKind::Dark, depth);
            let buffer = draw(&view, &theme);

            let mode = if focus { "focus" } else { "highlight" };
            let mut text = format!(
                "shape: {shape}, {depth_name}, {mode}, {COLUMNS}x{ROWS}, row {SELECTED} selected\n\n"
            );
            for y in 0..ROWS {
                text.push_str(line_text(&buffer, y).trim_end());
                text.push('\n');
            }
            text.push_str("\nmodifiers: graph | subject\n");
            let graph_width = screen::graph_width(&view, &theme, COLUMNS, ROWS);
            for (index, row) in laid.rows.iter().enumerate() {
                let y = u16::try_from(index).expect("few rows") + 1;
                // The graph column starts after the two-cell gutter (ADR-0058).
                let graph: String = (GUTTER..GUTTER + graph_width)
                    .map(|x| code(&buffer, x, y))
                    .collect();
                let subject = String::from_utf8_lossy(row.commit.subject()).into_owned();
                let at = line_text(&buffer, y)
                    .find(&subject)
                    .expect("the subject is on its row");
                let x = u16::try_from(line_text(&buffer, y)[..at].chars().count()).expect("narrow");
                text.push_str(&format!("{graph}| {}\n", code(&buffer, x, y)));
            }
            insta::assert_snapshot!(format!("criss-cross-{depth_name}-{mode}"), text);
        }
    }
}

fn draw(view: &mergewright_ui::graph_screen::GraphView<'_>, theme: &mergewright_ui::theme::Theme) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).expect("test backend");
    terminal
        .draw(|frame| frame.render_widget(GraphScreen::new(view, theme), frame.area()))
        .expect("draws");
    terminal.backend().buffer().clone()
}

fn line_text(buffer: &Buffer, y: u16) -> String {
    (0..COLUMNS).map(|x| buffer[(x, y)].symbol()).collect()
}

fn code(buffer: &Buffer, x: u16, y: u16) -> char {
    let cell = &buffer[(x, y)];
    if cell.symbol().trim().is_empty() {
        '.'
    } else if cell.modifier.contains(Modifier::BOLD) {
        'b'
    } else if cell.modifier.contains(Modifier::DIM) {
        'd'
    } else {
        '.'
    }
}

/// m5-history-and-ghost-preview/35A. **The cursor row, against its
/// neighbours, in rendered attributes.**
///
/// The user: *"really and I mean really hard to notice which row I'm on."*
/// Of the two possible causes — the style is not applied, or the assertion
/// was too weak to tell — **it was the second, and it was this file.**
/// `ancestry_highlight_and_focus_on_criss_cross` above is the only other
/// test here, and its `code()` helper reads `cell.modifier` alone, mapping
/// every cell to `b`/`d`/`.`. It never touches `cell.bg` or `cell.fg`. So
/// the raised background the cursor row is supposed to carry was asserted
/// **nowhere in this file** — a cursor row with no background at all would
/// have kept every snapshot here green.
///
/// The style *was* applied (`graph_screen/render.rs`'s `row_emphasis`
/// painted `Token::SurfaceRaised` across the row), and it was applied too
/// faintly to find: `#14161B` → `#232730` in the dark theme, about a
/// six-percent step in lightness, with the foreground colour unchanged.
/// m5-history-and-ghost-preview/44 gave the row its own token
/// (`Token::Selection`) with a contrast floor `surface-raised` was never
/// held to, and moved this row (and the graph's, the stash's, the
/// journal's, the files pane's) onto it. Which half of that is legible in
/// his terminal is his call and is not ticked here (rule 6). What this test
/// holds is the half a test can: that the row differs from its neighbours
/// by three **named** attributes, and that one of them is a shape rather
/// than a colour.
///
/// Rule 5 says a *capture* cannot prove colour, and that is true — the
/// `.txt` grids drop attributes. It does not say colour cannot be tested.
/// `TestBackend`'s `Buffer` carries `fg`, `bg` and `modifier` per cell.
#[test]
fn the_cursor_row_differs_from_its_neighbours_by_three_named_attributes() {
    use mergewright_ui::paint::color;
    use mergewright_ui::theme::Token;

    let shape = Shape::CrissCrossMerge;
    let dir = mergewright_fixtures::borrow(shape);
    let laid = lay_out(shape, &dir);
    let lines = laid.lines();
    let height = usize::from(ROWS - CHROME_ROWS);
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);
    let buffer = draw(&screen::view(&lines, height, SELECTED), &theme);

    let cursor = u16::try_from(SELECTED).expect("few rows") + 1;
    let above = cursor - 1;
    let below = cursor + 1;

    // 1. The background fill, across the whole row — the attribute this
    //    file never asserted.
    //
    //    m5-history-and-ghost-preview/44: this used to be `Token::SurfaceRaised`,
    //    which measured 1.21:1 against `bg` here — under the WCAG 1.4.11 floor
    //    (3:1). `Token::Selection` carries this job now; `SurfaceRaised` keeps
    //    its own, smaller step for a raised panel's background.
    assert_the_fill(&buffer, color(theme.token(Token::Selection)), cursor);

    // 2. The `▸` in gutter cell 0, in the accent colour and bold, at
    //    TrueColor too (m5/35's shape, moved to the gutter by ADR-0058).
    let accent = color(theme.token(Token::Accent));
    let rule = 0;
    assert_eq!(buffer[(rule, cursor)].symbol(), "▸");
    assert_eq!(buffer[(rule, cursor)].fg, accent);
    assert!(buffer[(rule, cursor)].modifier.contains(Modifier::BOLD));
    for y in [above, below] {
        assert!(
            !(0..COLUMNS).any(|x| buffer[(x, y)].symbol() == "▸"),
            "row {y} carries no mark"
        );
    }

    // 3. Bold on the subject text, which the neighbours do not have.
    let subject = (0..COLUMNS)
        .find(|x| {
            buffer[(*x, cursor)]
                .symbol()
                .chars()
                .all(char::is_alphabetic)
                && buffer[(*x, cursor)].symbol() != " "
                && *x > rule
        })
        .expect("the cursor row has a subject");
    assert!(
        buffer[(subject, cursor)].modifier.contains(Modifier::BOLD),
        "the cursor row's subject is bold"
    );
}

/// ADR-0058 §2: the fill starts at the marker slot, right of the graph
/// column; the gutter, the graph column (every lane of this three-lane
/// shape) and the one-cell margin keep the pane's ground. The rows above
/// and below are not filled.
#[allow(
    clippy::expect_used,
    reason = "a test helper: a row that is not drawn has nothing to check; the panic names it"
)]
fn assert_the_fill(buffer: &Buffer, selection: ratatui::style::Color, cursor: u16) {
    let (above, below) = (cursor - 1, cursor + 1);
    let slot = (0..COLUMNS)
        .find(|x| buffer[(*x, cursor)].bg == selection)
        .expect("the cursor row is filled");
    let lanes_left_of_fill = (GUTTER..slot)
        .filter(|x| !buffer[(*x, cursor)].symbol().trim().is_empty())
        .count();
    assert!(
        lanes_left_of_fill >= 1,
        "the graph column, with its lane glyphs, sits left of the fill"
    );
    for x in 0..slot {
        assert_eq!(
            buffer[(x, cursor)].bg,
            buffer[(x, above)].bg,
            "column {x} (gutter or graph) of the cursor row keeps the ground"
        );
    }
    let subject = line_text(buffer, cursor).find("work").expect("subject");
    assert!(
        u16::try_from(line_text(buffer, cursor)[..subject].chars().count()).expect("narrow") > slot,
        "the subject is on the fill"
    );
    assert_ne!(buffer[(COLUMNS - 1, cursor)].bg, selection, "the margin");
    for x in [slot + 1, 20, 40, COLUMNS - 2] {
        assert_eq!(
            buffer[(x, cursor)].bg,
            selection,
            "column {x} of the cursor row is filled with selection"
        );
        assert_ne!(
            buffer[(x, above)].bg,
            selection,
            "column {x} of the row above is not"
        );
        assert_ne!(
            buffer[(x, below)].bg,
            selection,
            "column {x} of the row below is not"
        );
    }
}

/// The same row with no hue to spend: at `ColorDepth::None` there is no
/// background step at all, so the **shape** has to carry it alone. This is
/// the tick for "the cursor is still findable without hue" — and the reason
/// 35 made the rule unconditional rather than raising the background's
/// contrast, which would have done nothing here.
#[test]
fn without_colour_the_cursor_row_is_still_marked_by_its_shape() {
    let shape = Shape::CrissCrossMerge;
    let dir = mergewright_fixtures::borrow(shape);
    let laid = lay_out(shape, &dir);
    let lines = laid.lines();
    let height = usize::from(ROWS - CHROME_ROWS);

    for depth in [ColorDepth::None, ColorDepth::Palette16] {
        let theme = theme(ThemeKind::Dark, depth);
        let buffer = draw(&screen::view(&lines, height, SELECTED), &theme);
        let cursor = u16::try_from(SELECTED).expect("few rows") + 1;

        assert_eq!(
            buffer[(0, cursor)].symbol(),
            "▸",
            "{depth:?}: the cursor row carries the gutter mark"
        );
        assert!(
            !(0..COLUMNS).any(|x| buffer[(x, cursor - 1)].symbol() == "▸"),
            "{depth:?}: the row above does not"
        );
        // And no background to lean on, which is the point.
        assert_eq!(
            buffer[(40, cursor)].bg,
            buffer[(40, cursor - 1)].bg,
            "{depth:?}: there is no background step at this depth"
        );
    }
}
