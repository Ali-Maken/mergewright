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
use graft_ui::graph_column::{GraphLine, RowPaint};
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
/// Empty while the plan is still being made — the box says `predicting…`
/// and the graph keeps the rows it had, so the screen does not blink.
pub(super) fn preview_entries<'a>(
    state: &'a AppState,
    preview: &'a PreviewState,
    now: i64,
) -> Vec<GraphEntry<'a>> {
    let badges = crate::update::preview::preview_badges(state);
    let mailmap = state.repository.as_ref().map(|facts| &facts.mailmap);
    preview
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let ghost = preview.ghost_index.contains_key(&row.commit.id);
            entry(
                row,
                ghost,
                index == preview.cursor,
                badges.as_ref(),
                mailmap,
                now,
            )
        })
        .collect()
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
