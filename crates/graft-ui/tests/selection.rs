//! Selection on the criss-cross merge, 80×24: the selected commit's thread
//! drawn bold, every other lane dim, and in Focus mode the rows off the
//! thread faded as a whole (DESIGN.md §5.2).
//!
//! Each snapshot is the screen's text, then per graph row the modifier of
//! each graph cell (`b` bold, `d` dim, `.` neither; blank cells `.`) and,
//! after `|`, the subject's. Read them as pictures; do not just re-accept.

mod support;

use graft_core::capabilities::ColorDepth;
use graft_fixtures::Shape;
use graft_graph::{Ancestry, Kin};
use graft_ui::graph_column::{GraphLine, RowPaint};
use graft_ui::graph_screen::{CHROME_ROWS, GraphScreen};
use graft_ui::theme::ThemeKind;
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
    let dir = graft_fixtures::borrow(shape);
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
            let buffer = draw(&view, &theme(ThemeKind::Dark, depth));

            let mode = if focus { "focus" } else { "highlight" };
            let mut text = format!(
                "shape: {shape}, {depth_name}, {mode}, {COLUMNS}x{ROWS}, row {SELECTED} selected\n\n"
            );
            for y in 0..ROWS {
                text.push_str(line_text(&buffer, y).trim_end());
                text.push('\n');
            }
            text.push_str("\nmodifiers: graph | subject\n");
            let graph_width = laid.width();
            for (index, row) in laid.rows.iter().enumerate() {
                let y = u16::try_from(index).expect("few rows") + 1;
                let graph: String = (0..graph_width).map(|x| code(&buffer, x, y)).collect();
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

fn draw(view: &graft_ui::graph_screen::GraphView<'_>, theme: &graft_ui::theme::Theme) -> Buffer {
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
