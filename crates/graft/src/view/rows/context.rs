//! The hint-context classification of the graph bar: the cursor row's own
//! context and the wrappers that lead it (`n N` for a search, a live
//! operation's `c`, `esc back to …`). Moved out of `view/rows.rs`
//! (m8/21b-7, move-only).

use super::{AppState, BadgeKind, GraphEntry, HintContext, History, RowVerbs, live_c, search_tail};

/// `row` led by `n next  N prev` while [`search_tail`] stands (m8/15b,
/// ticket 15 A2): a graph row's bar only — a row, marked or under `R`.
/// Applied before [`live_wrapped`], so at a stop the stop's keys lead and
/// `n N` follow them.
///
/// Only while the search has a hit: with none, `n`/`N` step nowhere
/// (`update::search::step_search` returns at `search.step`), so the bar
/// would offer a key that does nothing — the search's own line drops its
/// `n/N` clause there for the same reason (`search::keys_words`).
pub(super) fn search_wrapped(state: &AppState, row: HintContext) -> HintContext {
    let has_hits = state
        .search
        .as_ref()
        .is_some_and(|search| search.hits() > 0);
    if search_tail(state).is_none() || !has_hits {
        return row;
    }
    match row {
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow { .. }
        | HintContext::StashRow
        | HintContext::Rewritable(_)
        | HintContext::Marked(_) => HintContext::Searching(Box::new(row)),
        other => other,
    }
}

/// `row` wrapped in [`HintContext::OperationLive`] whenever an operation is
/// live and `row` is a graph row's bar (m6-conflicts/52, decision B) — with
/// or without a banner (a bisect has none), one `⇥` from it or not — so no
/// graph-row bar says `c commit` while `c` does otherwise.
///
/// `⇥ banner` leads only while one `Tab` lands on the banner
/// (m6-conflicts/12 scope item 5). A pane focused on a commit's files or
/// diff is wrapped too: `c` there passes through to the graph's keys
/// (`update::detail::keys` claims no plain `c`). Not the changes pane's
/// bar: it carries its own `live` ([`hint_context`]), and from its diff its
/// own `space`/`d` are not answered (the sweep in `update::tests`).
pub(super) fn live_wrapped(state: &AppState, row: HintContext) -> HintContext {
    let Some(c) = live_c(state) else {
        return row;
    };
    if !matches!(
        row,
        HintContext::WorkingTreeRow
            | HintContext::BranchRow { .. }
            | HintContext::PlainRow { .. }
            | HintContext::StashRow
            | HintContext::Marked(_)
            | HintContext::Searching(_)
            | HintContext::PaneFocused { .. }
    ) {
        return row;
    }
    HintContext::OperationLive {
        row: Box::new(row),
        c,
        way_to_banner: crate::update::banner::one_press_to_banner(state),
        // The one predicate `graph_key`'s `space` asks too (07d D1).
        space_stages: crate::update::keys::edit_stop::space_stages_here(state),
    }
}

/// `context` led by `esc back to blame` — or `esc back to history`, when
/// the trail's newest place is a history (m7-power-tools/21b) — exactly
/// when the next `Esc` goes back to the hidden trail
/// (m7-power-tools/20, 20i): the same predicate
/// the key asks (`update::place::esc_goes_back`), so a search, Focus,
/// Explain or a held failure — whose `Esc` comes first — keeps its own bar.
/// Only a graph row's or a pane's bar is wrapped; an overlay, a drag or a
/// box has its own.
pub(crate) fn back_to_place(state: &AppState, context: HintContext) -> HintContext {
    if !crate::update::place::esc_goes_back(state) {
        return context;
    }
    match context {
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow { .. }
        | HintContext::StashRow
        | HintContext::Rewritable(_)
        | HintContext::Marked(_)
        | HintContext::Searching(_)
        | HintContext::PaneFocused { .. }
        | HintContext::ChangesPane { .. }
        | HintContext::OperationLive { .. } => {
            if state.trail.top_history().is_some() {
                HintContext::BackToHistory(Box::new(context))
            } else {
                HintContext::BackToBlame(Box::new(context))
            }
        }
        other => other,
    }
}

/// The drawn cursor row's context: its first local-branch badge or plain,
/// with its verbs, under `R rewrite` when offered and `Marked` when flagged.
pub(super) fn cursor_row_context(state: &AppState, cursor: &GraphEntry<'_>) -> HintContext {
    // A stash's row first, before any badge (m8-polish-and-ship/15 A4): the
    // same `is_stash` on the same selected commit that `m`/`r` refuse on
    // (`update::preview::history`), so the bar and the keys agree. Never
    // `Rewritable`; marked like any row.
    if cursor_is_stash(state) {
        return if cursor.marker.is_some() {
            HintContext::Marked(Box::new(HintContext::StashRow))
        } else {
            HintContext::StashRow
        };
    }
    let verbs = row_verbs(state);
    // The commit just made (C13), ahead of the row kinds: its bar names
    // the undo that gives the edits back. Off it, the row's own bar.
    let selected = state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .map(|row| row.commit.id);
    let landed = crate::update::commit_editor::landed_bar(state.last_commit.as_deref(), selected)
        .map(|row| HintContext::GraphRow { row, verbs });
    let base = landed.unwrap_or_else(|| {
        cursor
            .badges
            .iter()
            .find_map(|badge| match badge.kind {
                BadgeKind::Branch { current, .. } => {
                    Some(HintContext::BranchRow { current, verbs })
                }
                _ => None,
            })
            .unwrap_or(HintContext::PlainRow { verbs })
    });
    let base = if rewrite_offered(state) {
        HintContext::Rewritable(Box::new(base))
    } else {
        base
    };
    if cursor.marker.is_some() {
        HintContext::Marked(Box::new(base))
    } else {
        base
    }
}

/// The selected commit is a stash's (`RefBadges::is_stash`).
fn cursor_is_stash(state: &AppState) -> bool {
    state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .zip(state.refs.as_deref())
        .is_some_and(|(row, refs)| refs.is_stash(row.commit.id))
}

/// The cursor row's own verbs (m7-power-tools/40), each from the check its
/// key runs: `o take` where the `.` menu's Cherry-pick slot is live
/// (`can_take`), `a amend` on `HEAD`'s own row while `a`'s cheap blocks are
/// clear (`amend_blocked`). Neither while an operation is live — `a`
/// refuses then (P14) and so does `o` (`update::preview::preview_refusal`)
/// — nor while a pane has the keyboard.
fn row_verbs(state: &AppState) -> RowVerbs {
    if !state.graph_has_focus()
        || state.on_working_tree
        || crate::update::commit_editor::live_operation(state).is_some()
    {
        return RowVerbs::NONE;
    }
    let on_head = state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .zip(state.refs.as_deref().and_then(crate::refs::RefBadges::head))
        .is_some_and(|(row, head)| row.commit.id == head);
    RowVerbs {
        take: crate::update::can_take(state),
        amend: on_head && !crate::update::amend_into::amend_blocked(state),
    }
}

/// Whether the bar offers `R rewrite` on the cursor's commit row
/// (m7-power-tools/08, ruling 10). `update::rewrite_editor` has no
/// membership fact at key time, so from what is in state: the graph has the
/// keyboard, `HEAD` is on a branch, nothing is live, and the row is not on
/// the default branch's chain — unless `HEAD` *is* the default branch, or
/// that chain is unread, when every commit row offers it. Anywhere else `R`
/// still answers, refusing in `plan`'s words.
fn rewrite_offered(state: &AppState) -> bool {
    if !state.graph_has_focus() || live_c(state).is_some() {
        return false;
    }
    let Some(refs) = state.refs.as_deref() else {
        return false;
    };
    let Some(head) = refs.head() else {
        return false;
    };
    let Some(branch) = refs.on(head).iter().find_map(|label| match label {
        crate::refs::RefLabel::Branch {
            name,
            current: true,
            ..
        } => Some(name),
        _ => None,
    }) else {
        return false;
    };
    let default = state
        .history
        .as_ref()
        .and_then(|history| history.walk.as_ref())
        .and_then(|walk| walk.default_branch.as_ref());
    if default.is_some_and(|name| name.shorthand() == branch.as_slice()) {
        return true;
    }
    let Some(row) = state
        .history
        .as_ref()
        .and_then(History::selected)
        .map(|row| row.commit.id)
    else {
        return false;
    };
    match &state.radar.context {
        crate::update::detail::Loading::Ready(context) => !context.on_default_chain(&row),
        _ => true,
    }
}
