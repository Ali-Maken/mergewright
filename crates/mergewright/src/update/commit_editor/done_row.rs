//! Where the graph's done line stands (ease-pass/08s M4, mock
//! `mergewright-commit-screen.html:178-179`): on its own row directly above
//! a landing bar, else in the bar's row as every status always stood.
//!
//! Read off state alone, because two sides need the same answer: the view's
//! row carve (`view::rows::graph_view` sets `GraphView::status_row` from it)
//! and the scrollers' row count (`AppState::graph_pane_rows`), which cannot
//! build a view. `view::frame` checks, every frame in a dev build, that the
//! bar this asks for is the bar the view drew.

use mergewright_ui::graph_screen::hints::{HintContext, RowVerbs};
use mergewright_ui::graph_screen::{StatusRow, status_row};

use super::{AppState, Status, landed_bar};
use crate::update::Screen;

/// [`StatusRow::AboveBar`] while a done line is on the graph's bottom row
/// and the bar under it is the commit just made's; [`StatusRow::InBar`]
/// otherwise, so a key that clears the line gives the row back.
#[must_use]
pub(crate) fn done_row(state: &AppState) -> StatusRow {
    status_row(done_shown(state), &bar_row(state))
}

/// The graph's bottom row draws a done line: the status shown is a `Done`
/// and no modal owns the row (`view::frame::footer_notice`'s `Done` arm).
#[must_use]
pub(crate) fn done_shown(state: &AppState) -> bool {
    !state.modal_owns_bottom_row() && matches!(state.shown_status(), Status::Done { .. })
}

/// The cursor row's bar as far as a landing goes: the commit just made's
/// ([`landed_bar`]) or the merge just made's (`finish::just_merged_bar`,
/// ease-pass/12; the same order as `view::rows::context`) while the graph's bar is the cursor row's at all — the
/// graph drawn, no badge in the air, no picker, the cursor on a history row
/// and the changes pane not holding the keyboard (`view::rows::hint_context`'s
/// own order). Any other bar is a plain row's here: not a landing.
fn bar_row(state: &AppState) -> HintContext {
    let changes_pane_keys = state.on_changes_pane() && !state.graph_has_focus();
    let row_bar_drawn = state.screen == Screen::Graph
        && state.graph_overlay().is_none()
        && state.drag.is_none()
        && crate::update::picker::hint_context(state).is_none()
        && !state.on_working_tree
        && !changes_pane_keys;
    let selected = state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .map(|row| row.commit.id);
    let landing = landed_bar(state.last_commit.as_deref(), selected)
        .or_else(|| crate::update::finish::just_merged_bar(state, selected));
    match landing.filter(|_| row_bar_drawn) {
        Some(row) => HintContext::GraphRow {
            row,
            verbs: RowVerbs::NONE,
        },
        None => HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        },
    }
}
