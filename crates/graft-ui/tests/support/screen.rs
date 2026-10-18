//! A graph screen's view over laid-out corpus rows, the way the app fills one.

use graft_graph::identity::BranchIdentity;
use graft_term::{sanitize_line, sanitize_path};
use graft_ui::graph_column::{GraphLine, RowPaint};
use graft_ui::graph_screen::{
    AuthorChip, Compaction, Filters, GraphEntry, GraphScreen, GraphView, HintContext, Icons,
    Location, WalkView, initials, relative_date, short_sha,
};
use graft_ui::theme::Theme;
use ratatui::layout::Rect;

/// A fixed clock reading, so the "how long ago" column is deterministic —
/// nothing to do with the fixture's own fixed commit clock
/// (`graft_fixtures::git::FIXED_DATE`), just a stand-in for "now" that never
/// changes between runs.
const NOW: i64 = 1_700_000_000;

/// One row as the graph draws it, highlighted when `cursor`.
fn entry<'a>(line: &GraphLine<'a>, cursor: bool) -> GraphEntry<'a> {
    GraphEntry {
        badges: Vec::new(),
        line: if cursor {
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
        tag: None,
        fold: None,
    }
}

/// The first `height` of `lines` as a finished walk, the row at `cursor`
/// highlighted.
pub fn view<'a>(lines: &[GraphLine<'a>], height: usize, cursor: usize) -> GraphView<'a> {
    GraphView {
        keys: graft_ui::keymap::Keymap::shipped(),
        location: Some(Location {
            repo: sanitize_line("api"),
            path: sanitize_path(std::path::Path::new("~/code/api")),
            branch: sanitize_line("main"),
            practice: false,
        }),
        spaced: false,
        problem: None,
        walk: WalkView::Complete { rows: lines.len() },
        rows: lines
            .iter()
            .take(height)
            .enumerate()
            .map(|(index, line)| entry(line, index == cursor))
            .collect(),
        icons: Icons::Plain,
        compaction: Compaction::at(8),
        filters: Filters::default(),
        reveal: None,
        order: None,
        ascii: false,
        pixel: false,
        graphics: None,
        push: None,
        notice: None,
        status_row: graft_ui::graph_screen::StatusRow::InBar,
        search: None,
        search_tail: None,
        compare_footer: None,
        stopped: None,
        commit_graph: None,
        banner: Vec::new(),
        divider_shown: false,
        hint_context: HintContext::PlainRow {
            verbs: graft_ui::graph_screen::hints::RowVerbs::NONE,
        },
        marker_column: false,
        banner_op: None,
        pending: None,
        tour: None,
    }
}

/// Cells the graph column takes when `view` draws into a `columns`×`rows`
/// screen: the screen's own geometry, through the fold it draws — a lane
/// with no commit in view is not drawn (ease-pass 06b), so the laid-out
/// width overstates it.
pub fn graph_width(view: &GraphView<'_>, theme: &Theme, columns: u16, rows: u16) -> u16 {
    GraphScreen::new(view, theme)
        .graph_area(Rect::new(0, 0, columns, rows))
        .width
}
