//! The graph screen over a corpus shape at the three sizes DESIGN.md §1
//! designs for: 80×24, 120×40 and 200×60.
//!
//! Each snapshot is the screen's text exactly as rendered, cursor on the
//! second row. Colour is the graph column's snapshots' business; these are
//! about what sits where. Read them as pictures; do not just re-accept.

mod support;

use graft_core::capabilities::ColorDepth;
use graft_fixtures::Shape;
use graft_graph::identity::BranchIdentity;
use graft_term::{sanitize_line, sanitize_path};
use graft_ui::graph_column::RowPaint;
use graft_ui::graph_screen::{
    AuthorChip, CHROME_ROWS, GraphEntry, GraphScreen, GraphView, WalkView, initials, relative_date,
    short_sha,
};
use graft_ui::theme::ThemeKind;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use support::corpus::lay_out;
use support::theme;

/// A fixed clock reading, so the "how long ago" column is deterministic —
/// nothing to do with the fixture's own fixed commit clock
/// (`graft_fixtures::git::FIXED_DATE`), just a stand-in for "now" that never
/// changes between runs.
const NOW: i64 = 1_700_000_000;

#[test]
fn feature_branches_at_every_designed_size() {
    let dir = graft_fixtures::borrow(Shape::FeatureBranches);
    let laid = lay_out(Shape::FeatureBranches, &dir);
    let lines = laid.lines();
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);

    for (columns, rows) in [(80u16, 24u16), (120, 40), (200, 60)] {
        let height = usize::from(rows - CHROME_ROWS);
        let view = GraphView {
            location: Some(sanitize_path(std::path::Path::new("~/code/api"))),
            problem: None,
            walk: WalkView::Complete {
                rows: laid.rows.len(),
            },
            rows: lines
                .iter()
                .take(height)
                .enumerate()
                .map(|(index, line)| GraphEntry {
                    badges: Vec::new(),
                    line: if index == 1 {
                        graft_ui::graph_column::GraphLine {
                            paint: RowPaint::Highlighted,
                            ..line.clone()
                        }
                    } else {
                        line.clone()
                    },
                    subject: sanitize_line(&String::from_utf8_lossy(line.row.commit.subject())),
                    author: AuthorChip {
                        initials: sanitize_line(&initials(&String::from_utf8_lossy(
                            &line.row.commit.author.name,
                        ))),
                        identity: BranchIdentity::of_name(&line.row.commit.author.email),
                    },
                    when: relative_date(NOW, line.row.commit.date().seconds),
                    sha: short_sha(&line.row.commit.id.to_string(), 7),
                })
                .collect(),
            icons: graft_ui::graph_screen::Icons::Plain,
        };
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
