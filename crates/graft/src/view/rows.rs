//! Building [`GraphView`]: the rows the walk last sent, folded and painted
//! for this frame.

use graft_graph::Ancestry;
use graft_graph::fold::Keep;
use graft_graph::row::Row;
use graft_term::{sanitize_line, sanitize_path};
use graft_ui::graph_column::RowPaint;
use graft_ui::graph_screen::{
    Compaction, Filters, GraphEntry, GraphView, OrderNotice, SearchLine, WalkView,
};
use std::collections::BTreeSet;

use crate::history::{History, Phase, Promotion, WalkFacts};
use crate::update::{AppState, Status};

use super::entries::{Thread, thread, uncommitted_shown, visible_rows};

/// Everything the graph screen draws: the rows in the viewport, the cursor
/// among them, and where the walk is.
///
/// The rows are borrowed from the window the walk last sent; only the
/// subjects, which came out of the repository, are copied — sanitised.
#[must_use]
pub fn graph_view(state: &AppState, now_seconds: i64) -> GraphView<'_> {
    let location = state
        .repository
        .as_ref()
        .map(|facts| sanitize_path(&facts.path));
    let problem = match &state.status {
        Status::Ready | Status::Copied { .. } => None,
        Status::Failed(message) => Some(sanitize_line(message)),
    };
    let Some(history) = &state.history else {
        return GraphView {
            location,
            problem,
            walk: WalkView::Opening,
            rows: Vec::new(),
            icons: graft_ui::graph_screen::Icons::default(),
            compaction: Compaction::at(crate::config::DEFAULT_MAX_LANES),
            filters: Filters::default(),
            reveal: None,
            order: None,
            ascii: false,
            pixel: false,
            graphics: None,
            notice: None,
            search: search_line(state),
            commit_graph: state.commit_graph_line(),
            banner: Vec::new(),
        };
    };

    let walk = walk_view(history);
    let thread = thread(history);
    let rows = visible_rows(state, history, thread.as_ref(), now_seconds);
    let compaction = compaction(state, history, thread.as_ref(), &rows);
    let reveal = reveal_index(state, history, &rows);

    GraphView {
        location,
        problem,
        walk,
        rows,
        icons: graft_ui::graph_screen::Icons::default(),
        compaction,
        filters: Filters {
            first_parent: history.view.follow == graft_graph::Follow::FirstParent,
            folded: history.view.inclusion.folded.len(),
            stash_hidden: !history.view.inclusion.hidden_stashes.is_empty(),
            focused: state.focus,
        },
        reveal,
        order: order_notice(history),
        ascii: false,
        pixel: false,
        graphics: None,
        notice: None,
        search: search_line(state),
        commit_graph: state.commit_graph_line(),
        banner: Vec::new(),
    }
}

/// The header's words for where the walk is.
fn walk_view(history: &History) -> WalkView {
    match &history.phase {
        Phase::Streaming if history.jump.is_some() => WalkView::Jumping {
            laid_out: history.rows,
        },
        Phase::Streaming => WalkView::Reading {
            laid_out: history.rows,
        },
        Phase::Complete => WalkView::Complete { rows: history.rows },
        Phase::Cancelled => WalkView::Cancelled {
            laid_out: history.rows,
        },
        Phase::Failed(reason) => WalkView::Failed(sanitize_line(reason)),
    }
}

/// The footer's search line: the prompt while it is open, else the search's
/// state. The query came from the keyboard and the line is drawn, so it is
/// sanitised like repository text.
fn search_line(state: &AppState) -> Option<SearchLine> {
    if let Some(typed) = &state.search_prompt {
        return Some(SearchLine::Prompt(sanitize_line(typed)));
    }
    let search = state.search.as_ref()?;
    Some(SearchLine::Status {
        text: sanitize_line(&search.line()),
        failed: matches!(search.state, crate::search::SearchState::Failed(_)),
    })
}

/// What the header says about the order the rows are in (ADR-0008): nothing
/// once they are in the order asked for; while commit-date order stands in
/// for a topological one, that it is being sorted — or, once the sort is
/// stopped or has failed, that the order stays as it is.
pub(crate) fn order_notice(history: &History) -> Option<OrderNotice> {
    let provisional = history.walk.as_ref().is_some_and(WalkFacts::is_provisional);
    if !provisional {
        return None;
    }
    Some(match history.promotion {
        Some(Promotion::Kept) => OrderNotice::Provisional,
        Some(Promotion::Sorting | Promotion::Swapping { .. }) | None => OrderNotice::Sorting,
    })
}

/// Which of `rows` the footer reveals in full, if any (ADR-0023): the
/// hovered row, else the cursor row while a peek is on, else none. `hover`
/// and the cursor are absolute history row numbers; `rows` is indexed from
/// the top of the viewport, offset by one when the uncommitted row draws
/// above it (`visible_rows`) — the same arithmetic, so the two never
/// disagree about which row is which. `None` when the row named is not
/// actually on screen (scrolled past, or the uncommitted row itself, which
/// has no subject to reveal).
fn reveal_index(state: &AppState, history: &History, rows: &[GraphEntry<'_>]) -> Option<usize> {
    let absolute = match (state.hover, state.peek) {
        (Some(row), _) => row,
        (None, true) => history.cursor,
        (None, false) => return None,
    };
    let offset = usize::from(uncommitted_shown(state));
    let index = offset + absolute.checked_sub(history.top)?;
    (index < rows.len()).then_some(index)
}

/// How the graph folds its lanes: the session's limit is filled in by
/// [`super::view`], the expansion flag is the user's, and [`keep_for`]
/// weighs the reserved integration lanes (`1..=n`, after lane 0), the
/// cursor's own column and the selection's thread ahead of everything else
/// (ADR-0025) — a priority a fold still overrides once the budget runs out,
/// not a veto.
fn compaction(
    state: &AppState,
    history: &History,
    thread: Option<&Thread>,
    rows: &[GraphEntry<'_>],
) -> Compaction {
    let integration = history
        .walk
        .as_ref()
        .map_or(0, |walk| walk.integration.len());
    let reserved: BTreeSet<usize> = (1..=integration).collect();
    let cursor = rows
        .iter()
        .position(|entry| entry.line.paint == RowPaint::Highlighted);
    let drawn: Vec<Row> = rows
        .iter()
        .map(|entry| entry.line.row.as_ref())
        .cloned()
        .collect();
    Compaction {
        max_lanes: crate::config::DEFAULT_MAX_LANES,
        expanded: state.lanes_expanded,
        keep: keep_for(
            &drawn,
            cursor,
            &reserved,
            thread.map(|thread| &thread.ancestry),
        ),
    }
}

/// The columns a fold weighs above the ordinary rank (ADR-0025), built once
/// and shared by the text and pixel tiers so a lane never folds differently
/// between them: the cursor's own column when its row is among `rows`,
/// `reserved` as given, and the selection's thread (`Ancestry::columns`).
pub(super) fn keep_for(
    rows: &[Row],
    cursor: Option<usize>,
    reserved: &BTreeSet<usize>,
    thread: Option<&Ancestry>,
) -> Keep {
    Keep {
        selected: cursor.and_then(|at| rows.get(at)).map(|row| row.lane),
        reserved: reserved.clone(),
        preferred: thread.map(Ancestry::columns).unwrap_or_default(),
    }
}
