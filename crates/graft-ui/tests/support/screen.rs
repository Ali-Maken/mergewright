//! A graph screen's view over laid-out corpus rows, the way the app fills one.

use graft_graph::identity::BranchIdentity;
use graft_term::{sanitize_line, sanitize_path};
use graft_ui::graph_column::{GraphLine, RowPaint};
use graft_ui::graph_screen::{
    AuthorChip, Compaction, Filters, GraphEntry, GraphView, Icons, WalkView, initials,
    relative_date, short_sha,
};

/// A fixed clock reading, so the "how long ago" column is deterministic —
/// nothing to do with the fixture's own fixed commit clock
/// (`graft_fixtures::git::FIXED_DATE`), just a stand-in for "now" that never
/// changes between runs.
const NOW: i64 = 1_700_000_000;

/// The first `height` of `lines` as a finished walk, the row at `cursor`
/// highlighted.
pub fn view<'a>(lines: &[GraphLine<'a>], height: usize, cursor: usize) -> GraphView<'a> {
    GraphView {
        location: Some(sanitize_path(std::path::Path::new("~/code/api"))),
        problem: None,
        walk: WalkView::Complete { rows: lines.len() },
        rows: lines
            .iter()
            .take(height)
            .enumerate()
            .map(|(index, line)| GraphEntry {
                badges: Vec::new(),
                line: if index == cursor {
                    GraphLine {
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
                when: relative_date(line.row.commit.date().seconds, NOW),
                sha: short_sha(&line.row.commit.id.to_string(), 7),
                compare_marked: false,
                marker: None,
            })
            .collect(),
        icons: Icons::Plain,
        compaction: Compaction::at(8),
        filters: Filters::default(),
        reveal: None,
        order: None,
        ascii: false,
        pixel: false,
        graphics: None,
        notice: None,
        search: None,
        compare_footer: None,
        commit_graph: None,
        banner: Vec::new(),
        detail_shown: false,
        marker_column: false,
    }
}
