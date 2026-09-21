//! Drawing one open Ghost Preview (m5-history-and-ghost-preview/08).
//!
//! While a preview is open the graph pane draws the preview's **slice**
//! instead of the live window: the rows around the cursor with the ghosts
//! laid into them, and the refs shown where they would land rather than
//! where they are. Nothing here reads the repository — the slice and its
//! layout were built once, in `update::preview`, from rows the walk had
//! already sent (design note hard choice 1: a preview never loads more
//! history mid-preview).

use graft_graph::identity::BranchIdentity;
use graft_term::sanitize_line;
use graft_ui::graph_column::{GraphLine, NodeKind, RowPaint};
use graft_ui::graph_screen::{
    AuthorChip, GraphEntry, SHA_WIDTH, initials, relative_date, short_sha,
};

use crate::update::AppState;
use crate::update::preview::PreviewState;

use super::entries::badge_view;

/// What a ghost row shows where a real row shows a short SHA and an age:
/// the commit does not exist, so there is nothing true to put there.
const NOT_YET: &str = "\u{2014}";

/// The preview's rows for this frame, in drawing order.
///
/// Starts at [`PreviewState::view_start`], not at `rows[0]`: the slice is
/// three screens wide so a ghost has somewhere to hang, and only the middle
/// one is the screen the user was looking at when they pressed the key.
///
/// Empty while the plan is still being made — the box says `predicting…`
/// and the graph keeps the rows it had, so the screen does not blink.
pub(super) fn preview_entries<'a>(
    state: &'a AppState,
    preview: &'a PreviewState,
    now: i64,
) -> Vec<GraphEntry<'a>> {
    let badges = crate::update::preview::preview_badges(state);
    let mailmap = state.repository.as_ref().map(|facts| &facts.mailmap);
    let label = preview.outcome_label();

    // Built for the whole slice first, labels and all, and only then
    // windowed. The label rows are drawn rows: choosing the window against
    // `PreviewState::rows` and inserting them afterwards would hand the
    // screen one row more than it has, and the row it lost would be the
    // bottom one — the row the user pressed the key on (ticket 17 F1, the
    // Critical this milestone already paid for once).
    let mut drawn: Vec<GraphEntry<'a>> = Vec::with_capacity(preview.rows.len() + 1);
    let mut at_start = 0;
    let mut at_selected = 0;
    let mut previous_ghost = false;
    for (index, row) in preview.rows.iter().enumerate() {
        let ghost = preview.ghost_index.contains_key(&row.commit.id);
        if ghost && !previous_ghost {
            drawn.push(label_entry(row, &label));
        }
        previous_ghost = ghost;
        if index == preview.view_start {
            // The label above a run that *starts* at the user's top row
            // belongs to the picture: point at it, not past it.
            at_start = drawn.len() - usize::from(ghost);
        }
        if preview.selected_row == Some(index) {
            at_selected = drawn.len();
        }
        drawn.push(entry(
            row,
            ghost,
            preview.selected_row == Some(index),
            badges.as_ref(),
            mailmap,
            now,
        ));
    }

    let start = window_start(state, preview, at_start, at_selected);
    drawn.drain(..start.min(drawn.len()));
    drawn
}

/// Where the drawn window starts inside the rows built above — `at_start`
/// is the user's own top row and `at_selected` the row the key was pressed
/// on, both as positions in that list rather than in
/// [`PreviewState::rows`].
///
/// The user's own top row, and never above it. The box docks over the
/// bottom of the graph, though, so the picture is shorter than the screen
/// those rows were scrolled on; when that would push the row the key was
/// pressed on off the bottom, the window slides down by exactly as much as
/// it must to keep it. A picture the user cannot find their own commit in
/// is not a preview of anything they asked for.
fn window_start(
    state: &AppState,
    preview: &PreviewState,
    at_start: usize,
    at_selected: usize,
) -> usize {
    if preview.selected_row.is_none() {
        return at_start;
    }
    let visible = state.graph_rows();
    at_start.max(at_selected.saturating_sub(visible.saturating_sub(1)))
}

/// The label row drawn above a run of ghosts: `after rebase`, on a row of
/// its own, with no node and no trailing columns.
///
/// Its lane occupancy is the first ghost's, so every **real** lane running
/// past the block keeps an unbroken `│` through the label; the ghost's own
/// column is blank, because the ghost lane starts on the row below. Nothing
/// on this row is a commit, so nothing on it claims to be one: no node
/// glyph ([`NodeKind::Absent`]), no author, no age, no sha.
fn label_entry<'a>(ghost: &graft_graph::row::Row, label: &str) -> GraphEntry<'a> {
    let mut line = GraphLine::owned(graft_graph::row::Row {
        // Never compared against a ref or a selection: `view` builds this
        // row fresh every frame and nothing else holds it. It keeps the
        // ghost's synthetic id only because a `Row` must have one.
        commit: ghost.commit.clone(),
        lane: ghost.lane,
        incoming: Vec::new(),
        outgoing: Vec::new(),
        lanes: ghost.lanes.clone(),
        marker: None,
    });
    line.node = NodeKind::Absent;
    line.paint = RowPaint::Dimmed;
    GraphEntry {
        badges: Vec::new(),
        line,
        subject: sanitize_line(label),
        author: AuthorChip {
            initials: sanitize_line(""),
            identity: BranchIdentity::of_name(b""),
        },
        when: String::new(),
        sha: String::new(),
        compare_marked: false,
        marker: None,
    }
}

/// One row of the slice, real or ghost.
fn entry<'a>(
    row: &'a graft_graph::row::Row,
    ghost: bool,
    highlighted: bool,
    badges: Option<&crate::update::preview::BadgeOverlay<'a>>,
    mailmap: Option<&graft_git::mailmap::Mailmap>,
    now: i64,
) -> GraphEntry<'a> {
    let mut line = GraphLine::plain(row);
    if ghost {
        line.paint = RowPaint::Ghost;
        // The lanes this ghost node itself touches: its own, the ones
        // closing into it and every edge leaving it. A real lane merely
        // passing through the row is none of the preview's business and
        // keeps its solid glyph and its colour (DESIGN.md §1.7,
        // `RowPaint::Ghost`).
        line.ghost_lanes = std::iter::once(row.lane)
            .chain(row.incoming.iter().copied())
            .chain(row.outgoing.iter().map(|(lane, _)| *lane))
            .collect();
    } else if let Some(badges) = badges
        && badges.head() == Some(row.commit.id)
    {
        line.node = graft_ui::graph_column::NodeKind::Head;
    }
    if highlighted {
        line.paint = if ghost {
            // A highlighted ghost is still a ghost: losing the hollow glyph
            // here would draw the cursor's row as a commit that exists.
            RowPaint::Ghost
        } else {
            RowPaint::Highlighted
        };
    }
    let (name, email) = match mailmap {
        Some(mailmap) => mailmap.resolve(&row.commit.author),
        None => (
            row.commit.author.name.clone(),
            row.commit.author.email.clone(),
        ),
    };
    let subject = String::from_utf8_lossy(row.commit.subject());
    GraphEntry {
        badges: badges.map_or_else(Vec::new, |badges| {
            badges.on(row.commit.id).iter().map(badge_view).collect()
        }),
        line,
        subject: sanitize_line(&subject),
        author: AuthorChip {
            initials: sanitize_line(&initials(&String::from_utf8_lossy(&name))),
            identity: BranchIdentity::of_name(&email),
        },
        when: if ghost {
            NOT_YET.to_owned()
        } else {
            relative_date(row.commit.date().seconds, now)
        },
        sha: if ghost {
            NOT_YET.to_owned()
        } else {
            short_sha(&row.commit.id.to_string(), usize::from(SHA_WIDTH))
        },
        // Neither belongs to a picture: the compare pin and the Mistake
        // Radar are about history as it is, and a preview is about history
        // as it would be.
        compare_marked: false,
        marker: None,
    }
}
