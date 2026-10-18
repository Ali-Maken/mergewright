//! `/`'s own share of [`super::update`]: the prompt, the running search and
//! stepping between hits.

use std::sync::Arc;

use mergewright_core::event::{Event, KeyCode, KeyPress};
use mergewright_core::job::JobId;
use mergewright_core::oid::Oid;

use crate::search::{Search, SearchState};

use super::events::Command;
use super::state::{AppState, Screen};

/// The search's keys, answered before any other binding: everything but
/// `Ctrl-C` while the prompt is open, then `/`, `n` and `N`. `true` if the key
/// was the search's.
pub(super) fn search_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let control_c = key.code == KeyCode::Char('c') && key.modifiers.control;
    if next.search_prompt.is_some() && !control_c {
        prompt_key(next, commands, key);
    } else if is_search_key(key)
        && next.screen == Screen::Graph
        && next.history.is_some()
        && graph_is_shown(next)
    {
        next.search_prompt = Some(String::new());
    } else if let Some(forward) = search_step(key)
        && next.search.is_some()
        && graph_is_shown(next)
    {
        step_search(next, commands, forward);
    } else {
        return false;
    }
    true
}

/// The graph's search answers only while the graph is what's shown
/// (m8/09e). Under the Map, the journal or the stash list the frame draws the
/// overlay, not the graph screen, so the prompt (drawn only in the graph's
/// footer) would be unseen and `n`/`N` would move a hidden cursor. Every
/// other overlay is already in [`AppState::modal_owns_keys`].
fn graph_is_shown(state: &AppState) -> bool {
    state.graph_overlay().is_none()
}

/// `/`: open the search prompt over the graph.
fn is_search_key(key: KeyPress) -> bool {
    key.code == KeyCode::Char('/') && !key.modifiers.control && !key.modifiers.alt
}

/// `n` / `N`: the next or previous search hit, wrapping. `Some(true)` is `n`.
fn search_step(key: KeyPress) -> Option<bool> {
    if key.modifiers.control || key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char('n') => Some(true),
        KeyCode::Char('N') => Some(false),
        _ => None,
    }
}

/// A key while the `/` prompt is open: text goes into the query, `Enter`
/// searches, `Esc` closes the prompt and nothing else.
fn prompt_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) {
    let Some(prompt) = next.search_prompt.as_mut() else {
        return;
    };
    match key.code {
        KeyCode::Char(typed) if !key.modifiers.control && !key.modifiers.alt => prompt.push(typed),
        KeyCode::Backspace => {
            prompt.pop();
        }
        KeyCode::Escape => next.search_prompt = None,
        KeyCode::Enter => {
            // An empty query closes the prompt: nothing to search for.
            if let Some(query) = crate::search::Query::parse(prompt) {
                start_search(next, commands, query);
            }
            next.search_prompt = None;
        }
        _ => {}
    }
}

/// Replaces any search on screen with one for `query`, remembering where the
/// selection is so `Esc` can put it back.
pub(in crate::update) fn start_search(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    query: crate::search::Query,
) {
    let prior = next
        .history
        .as_ref()
        .map_or((0, 0), |history| (history.cursor, history.top));
    let (unstarted, prior) = match next.search.take() {
        Some(older) => {
            if let Some(job) = older.job {
                commands.push(Command::CancelJob(job));
            }
            // A search replacing a running one goes back to where the first
            // began, not to a hit of the one it replaced.
            let prior = match older.state {
                SearchState::Running { .. } => older.prior,
                _ => prior,
            };
            (older.unstarted + 1, prior)
        }
        None => (1, prior),
    };
    next.search = Some(Search::new(query.clone(), unstarted, prior));
    commands.push(Command::Search(query));
}

/// A search job has started. Starts arrive in the order searches were asked
/// for, so only the last outstanding one belongs to the search on screen;
/// any other is superseded and cancelled.
pub(super) fn search_started(next: &mut AppState, commands: &mut Vec<Command>, id: JobId) {
    let Some(search) = next.search.as_mut() else {
        commands.push(Command::CancelJob(id));
        return;
    };
    search.unstarted = search.unstarted.saturating_sub(1);
    let wanted = search.unstarted == 0 && matches!(search.state, SearchState::Running { .. });
    if wanted {
        search.job = Some(id);
    } else {
        commands.push(Command::CancelJob(id));
    }
}

/// Is job `id` the search on screen?
pub(in crate::update) fn searching(next: &AppState, id: JobId) -> bool {
    next.search
        .as_ref()
        .is_some_and(|search| search.job == Some(id))
}

/// Hits from search job `id`: counted if it is the search on screen, dropped
/// if it was superseded or cancelled. The first hit is jumped to at once.
pub(super) fn search_progress(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    hits: std::sync::Arc<[Oid]>,
) {
    if !searching(next, id) {
        return;
    }
    let Some(search) = next.search.as_mut() else {
        return;
    };
    search.batches.push(hits);
    search.state = SearchState::Running {
        hits: search.hits(),
    };
    if search.current.is_none() {
        step_search(next, commands, true);
    }
}

/// `n` / `N`, and the first hit: move the selection to the next hit.
fn step_search(next: &mut AppState, commands: &mut Vec<Command>, forward: bool) {
    let height = next.graph_rows();
    let Some(search) = next.search.as_mut() else {
        return;
    };
    let Some(target) = search.step(forward) else {
        return;
    };
    // A hit moves the history cursor, not the working-tree row: leave it
    // before asking `History` to seek.
    next.on_working_tree = false;
    let Some(history) = next.history.as_mut() else {
        return;
    };
    let request = history.seek(target, height);
    search.seeking = history.seeking;
    if search.seeking.is_none() {
        search.missed = history.selected().map(|row| row.commit.id) != Some(target);
    }
    if let (Some(request), Some(walk)) = (request, next.awaiting) {
        commands.push(Command::ShowRows(walk, request));
    }
}

/// After a window from walk `walk` lands: if it answered a search's seek, say
/// whether the hit is in this view, and fill in any rows the new viewport
/// needs beyond what came back. Called from `history::walk_progress`.
pub(in crate::update) fn seek_landed(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    walk: JobId,
) {
    let height = next.graph_rows();
    let (Some(search), Some(history)) = (next.search.as_mut(), next.history.as_mut()) else {
        return;
    };
    let Some(target) = search.seeking else {
        return;
    };
    if history.seeking.is_some() {
        return;
    }
    search.seeking = None;
    search.missed = history.selected().map(|row| row.commit.id) != Some(target);
    if let Some(request) = history.show(height) {
        commands.push(Command::ShowRows(walk, request));
    }
}

/// `Esc` on a running search: cancel its job, withdraw any jump to a hit, and
/// put the selection back where it was.
pub(in crate::update) fn cancel_search(next: &mut AppState, commands: &mut Vec<Command>) {
    let height = next.graph_rows();
    let Some(search) = next.search.as_mut() else {
        return;
    };
    if let Some(job) = search.job.take() {
        commands.push(Command::CancelJob(job));
    }
    search.state = SearchState::Cancelled {
        hits: search.hits(),
    };
    search.seeking = None;
    let (cursor, top) = search.prior;
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.restore(cursor, top, height)
        && let Some(walk) = next.awaiting
    {
        commands.push(Command::ShowRows(walk, request));
    }
}

/// The search job ended — finished, cancelled or failed: record how. A
/// failure shows on the search's own line, by state, not the status line.
/// Finished, its hits are final: the walk is asked how many it folds.
pub(super) fn search_job_ended(next: &mut AppState, commands: &mut Vec<Command>, event: Event) {
    let Some(search) = next.search.as_mut() else {
        return;
    };
    let hits = search.hits();
    let state = match event {
        Event::JobFailed(_, failure) => SearchState::Failed(failure.to_string()),
        Event::JobCancelled(_) => SearchState::Cancelled { hits },
        _ => SearchState::Done { hits },
    };
    search.job = None;
    search.state = state;
    ask_folded_now(next, commands);
}

/// Puts the finished search's fold count on the walk's requests (ease-pass
/// 06c): how many of its hits the default fold hides now. It rides on every
/// request until answered ([`folded_hits`]). `false` when there is nothing
/// to ask — no finished search with hits, or no walk.
pub(in crate::update) fn ask_folded(next: &mut AppState) -> bool {
    let (Some(search), Some(history)) = (next.search.as_mut(), next.history.as_mut()) else {
        return false;
    };
    if !matches!(search.state, SearchState::Done { .. }) || search.hits() == 0 {
        return false;
    }
    let ids: Arc<[Oid]> = search
        .batches
        .iter()
        .flat_map(|batch| batch.iter().copied())
        .collect();
    search.folded_ask = Some(history.ask_folded(ids));
    true
}

/// [`ask_folded`], with a request sent now for the ask to ride on: none is
/// due when the search finishes or the walk completes. Not while the first
/// request still waits for `HEAD` — that request will carry it.
fn ask_folded_now(next: &mut AppState, commands: &mut Vec<Command>) {
    if !ask_folded(next) {
        return;
    }
    let height = next.graph_rows();
    if let (Some(history), Some(walk)) = (next.history.as_mut(), next.awaiting)
        && !history.head_pending
        && let Some(request) = history.ask_again(height)
    {
        commands.push(Command::ShowRows(walk, request));
    }
}

/// The walk has just laid out to the end. If its last count of the search's
/// hits left some unreached, it is asked once more — unless an ask is out
/// already, whose answer will count them.
pub(in crate::update) fn walk_completed(next: &mut AppState, commands: &mut Vec<Command>) {
    let unreached = next
        .search
        .as_ref()
        .is_some_and(|search| search.unwalked > 0);
    let asking = next
        .history
        .as_ref()
        .is_some_and(crate::history::History::folded_ask_out);
    if unreached && !asking {
        ask_folded_now(next, commands);
    }
}

/// The walk's count of a search's hits the default fold hides: the
/// search's own if it answers the search's newest ask, else dropped (a
/// newer ask is out, or the search it was for has gone).
pub(in crate::update) fn folded_hits(
    next: &mut AppState,
    asked: u64,
    folded: usize,
    unwalked: usize,
) {
    if let Some(search) = next.search.as_mut()
        && search.folded_ask == Some(asked)
    {
        search.folded = folded;
        search.unwalked = unwalked;
    }
}

#[cfg(test)]
mod tests;
