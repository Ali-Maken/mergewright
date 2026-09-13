//! The graph screen over a corpus shape at the three sizes DESIGN.md §1
//! designs for: 80×24, 120×40 and 200×60.
//!
//! Each snapshot is the screen's text exactly as rendered, cursor on the
//! second row. Colour is the graph column's snapshots' business; these are
//! about what sits where. Read them as pictures; do not just re-accept.

mod support;

use graft_core::capabilities::ColorDepth;
use graft_fixtures::Shape;
use graft_ui::graph_screen::{CHROME_ROWS, GraphScreen};
use graft_ui::theme::ThemeKind;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use support::corpus::lay_out;
use support::{screen, theme};

#[test]
fn feature_branches_at_every_designed_size() {
    let dir = graft_fixtures::borrow(Shape::FeatureBranches);
    let laid = lay_out(Shape::FeatureBranches, &dir);
    let lines = laid.lines();
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);

    for (columns, rows) in [(80u16, 24u16), (120, 40), (200, 60)] {
        let view = screen::view(&lines, usize::from(rows - CHROME_ROWS), 1);
        let mut terminal = Terminal::new(TestBackend::new(columns, rows))
            .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
        terminal
            .draw(|frame| frame.render_widget(GraphScreen::new(&view, &theme), frame.area()))
            .unwrap_or_else(|why| panic!("drawing into a test backend cannot fail: {why}"));
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..rows {
            let line: String = (0..columns).map(|x| buffer[(x, y)].symbol()).collect();
            text.push_str(line.trim_end());
            text.push('\n');
        }
        insta::assert_snapshot!(format!("feature-branches-{columns}x{rows}"), text);
    }
}
