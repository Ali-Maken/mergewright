//! The middle of the loop: `Event ──▶ update(state) ──▶ (new state, [Command])`
//! (ARCHITECTURE.md).
//!
//! [`update`] is the only place [`AppState`] changes, and it is pure: given the
//! same state and the same event it returns the same answer, every time,
//! forever. It reads no clock, opens no file, spawns no thread and draws
//! nothing. Anything that has to touch the world leaves here as a [`Command`]
//! for the job runner (`crate::jobs`) to execute.
//!
//! That is what makes the application testable without a terminal: every test
//! below is a plain function call over plain data.
//!
//! The module is split by concern: [`events`] and [`state`] hold the plain
//! data; [`keys`], [`search`] and [`commit_graph`] answer their own slice of
//! input; [`history`] carries the walk's progress and the topological-sort
//! promotion (ADR-0008). This file keeps the dispatcher — `update`, `answer`
//! and `core_event` — plus the mouse handling, which is too small to be its
//! own file.

mod commit_graph;
pub mod detail;
mod events;
mod history;
mod keys;
mod search;
mod selection;
mod state;

pub use events::{AppEvent, Command, CommitGraphOffer, GitCheck, Product, Progress, Task};
pub use state::{AppState, Banner, CHROME_ROWS, Harness, Quit, Screen, Size, Status};

use graft_core::event::{Event, MouseButton, MouseInput, MouseKind};

use crate::history::{History, Phase};

/// Answer one event: the new state, and the work it wants done.
///
/// Takes the old state by reference and returns a new one instead of mutating
/// in place, so a caller can always compare before with after — which is what
/// undo, and the tests below, rely on.
#[must_use]
pub fn update(state: &AppState, event: AppEvent) -> (AppState, Vec<Command>) {
    let mut next = state.clone();
    let mut commands = Vec::new();
    let cursor_before = state.history.as_ref().map_or(0, |history| history.cursor);
    answer(&mut next, &mut commands, event);
    selection::reconcile(&mut next, cursor_before);
    // `local_jobs` borrowed out for the closure: `follow_selection` also
    // wants `&mut next`, and the two can't overlap.
    let mut local_jobs = next.local_jobs;
    commands.extend(selection::follow_selection(&mut next, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
    // The commit-graph line costs a graph row: when it comes or goes, the
    // viewport changed height, so ask for the rows that now fit.
    if next.commit_graph_line().is_some() != state.commit_graph_line().is_some() {
        show_rows(&mut next, &mut commands);
    }
    (next, commands)
}

/// The half of [`update`] that answers one event, before the viewport is
/// squared with the commit-graph line.
fn answer(next: &mut AppState, commands: &mut Vec<Command>, event: AppEvent) {
    match event {
        event @ (AppEvent::Started(_, Task::WriteCommitGraph | Task::DeclineCommitGraph)
        | AppEvent::Produced(
            _,
            Product::CommitGraph(_) | Product::CommitGraphDeclined(_),
        )) => {
            commit_graph::commit_graph_event(next, event);
        }
        AppEvent::Core(event) => core_event(next, commands, event),
        AppEvent::Started(id, Task::ReadHistory) => history_started(next, commands, id),
        AppEvent::Started(id, Task::PromoteHistory) => history::sort_started(next, commands, id),
        AppEvent::Started(id, Task::Search) => search::search_started(next, commands, id),
        // The detail pane names its jobs itself, before they start.
        event @ AppEvent::Produced(
            _,
            Product::Files(_) | Product::FileDiff(..) | Product::ContainedIn(..),
        ) => {
            detail::detail_event(next, commands, event);
        }
        // Not waited on by start: welcome whenever they arrive (the detail
        // pane named its jobs itself, before they started).
        AppEvent::Started(
            _,
            Task::OpenRepository
            | Task::CheckSystemGit
            | Task::ReadRefs
            | Task::ReadWorkingTree
            | Task::ListFiles { .. }
            | Task::FileDiff { .. }
            | Task::ContainedIn { .. },
        ) => {}
        AppEvent::Produced(id, Product::TopologicalOrder(order)) => {
            history::sorted(next, commands, id, order);
        }
        AppEvent::Produced(_, Product::Repository(Ok(facts))) => {
            next.repository = Some(facts);
            // Whatever went wrong last time has been superseded by a
            // repository that opened.
            next.status = Status::Ready;
            commit_graph::decide_commit_graph_offer(next);
        }
        AppEvent::Produced(_, Product::Repository(Err(failure))) => {
            if next.repository.is_none() {
                // Nothing was ever opened, so there is no screen to show this
                // on and nothing for the user to do with Graft. Leaving, with
                // the reason, beats an empty screen with an error in a corner.
                next.quit = Some(Quit::Fatal(failure));
            } else {
                // A refresh of a repository that is already open failed — the
                // directory went away mid-run, say. The screen still has
                // something to show, so this goes on the status line.
                next.status = Status::Failed(failure);
            }
        }
        AppEvent::Produced(
            _,
            product @ (Product::SystemGit(_) | Product::Refs(_) | Product::WorkingTree(_)),
        ) => took_fact(next, product),
        AppEvent::Progress(id, Progress::History(update)) => {
            history::walk_progress(next, commands, id, update);
        }
        AppEvent::Progress(id, Progress::Search(hits)) => {
            search::search_progress(next, commands, id, hits);
        }
    }
}

/// A history walk started: it replaces whatever walk came before.
fn history_started(next: &mut AppState, commands: &mut Vec<Command>, id: graft_core::job::JobId) {
    // The newest ask wins. Anything the previous walk was going to
    // say is now about a question nobody asked.
    next.awaiting = Some(id);
    let mut history = History::started();
    history.strip_rows = keys::strip_rows_for(next.tier());
    // A refresh keeps what the user folded, hid or switched to.
    if let Some(previous) = &next.history {
        history.view = previous.view.clone();
    }
    if let Some(request) = history.show(next.graph_rows()) {
        commands.push(Command::ShowRows(id, request));
    }
    next.history = Some(history);
}

/// A fact about the repository beside the graph arrived: the system git
/// check, the refs, or the working tree counts. `answer` routes only those.
fn took_fact(next: &mut AppState, product: Product) {
    match product {
        Product::SystemGit(check) => {
            next.system_git = Some(check);
        }
        Product::Refs(Ok(refs)) => {
            next.refs = Some(std::sync::Arc::new(refs));
        }
        Product::Refs(Err(failure)) => {
            // The graph still draws without badges; the reason goes on the
            // status line rather than leaving them silently missing.
            next.status = Status::Failed(failure);
        }
        Product::WorkingTree(Ok(counts)) => {
            next.working_tree = Some(counts);
        }
        Product::WorkingTree(Err(failure)) => {
            // The graph still draws without the uncommitted row; the reason
            // goes on the status line rather than leaving it silently
            // missing.
            next.status = Status::Failed(failure);
        }
        // Not a fact: `answer`'s pattern keeps these from arriving.
        Product::Repository(_)
        | Product::TopologicalOrder(_)
        | Product::CommitGraph(_)
        | Product::CommitGraphDeclined(_)
        | Product::Files(_)
        | Product::FileDiff(..)
        | Product::ContainedIn(..) => {}
    }
}

/// The half of [`answer`] that answers the events `core` defines.
fn core_event(next: &mut AppState, commands: &mut Vec<Command>, event: Event) {
    match event {
        // The search's keys go first: the open prompt takes every letter.
        // A handed-back key is checked next, before any graph binding, so
        // a harness can claim a key whatever the graph would do with it.
        Event::Key(key) => {
            if next.search_prompt.is_some() || !next.harness.hand_back.contains(&key.code) {
                if !search::search_key(next, commands, key) {
                    keys::key_event(next, commands, key);
                }
            } else {
                next.quit = Some(Quit::HandedBack(key.code));
            }
        }
        Event::Resize { columns, rows } => {
            next.size = Size { columns, rows };
            let height = next.graph_rows();
            if let Some(history) = next.history.as_mut()
                && let Some(request) = history.show(height)
                && let Some(id) = next.awaiting
            {
                commands.push(Command::ShowRows(id, request));
            }
        }
        event @ (Event::JobFailed(id, _) | Event::JobFinished(id) | Event::JobCancelled(id))
            if search::searching(next, id) || commit_graph::commit_graph_job(next, id) =>
        {
            side_job_ended(next, event);
        }
        Event::JobFailed(id, failure) => {
            if next.awaiting == Some(id) {
                next.awaiting = None;
                history::set_phase(next, Phase::Failed(failure.to_string()));
                history::give_up_promotion(next, commands);
            }
            if next.promoting == Some(id) {
                history::give_up_promotion(next, commands);
            }
            // Surfaced, not swallowed: the user is told which job failed and
            // why. A panicking job says so in the same place — it is still a
            // failure, and the crash report's path rides along with it.
            next.status = Status::Failed(format!("{id} failed: {failure}"));
        }
        Event::JobFinished(id) => {
            // A sort that finished without producing its order: nothing
            // will swap in, so the order stays provisional.
            if next.promoting == Some(id) {
                history::give_up_promotion(next, commands);
            }
            if next.awaiting == Some(id) {
                next.awaiting = None;
                history::set_phase(next, Phase::Complete);
            }
            // The last thing Graft was asked to do worked, so a failure still
            // on the status line is stale. Nothing keyed by job id is kept yet,
            // which is also what makes a result for a job whose screen has gone
            // harmless: there is nothing for it to overwrite.
            next.status = Status::Ready;
        }
        // Cancellation is not a failure and not news: the application asked for
        // it. The status line keeps whatever it was already saying.
        Event::JobCancelled(id) => {
            // A walk cancelled by anything other than `Esc` — shutdown, say —
            // must not leave the screen waiting for it either.
            if next.awaiting == Some(id) {
                next.awaiting = None;
                history::set_phase(next, Phase::Cancelled);
                history::give_up_promotion(next, commands);
            }
            if next.promoting == Some(id) {
                history::give_up_promotion(next, commands);
            }
        }
        Event::Mouse(mouse) => mouse_event(next, commands, mouse),
        // Nothing is bound to these yet. They reach `update` so that the
        // tickets that need them have somewhere obvious to add the handling.
        Event::Paste(_) | Event::FocusGained | Event::FocusLost => {}
    }
}

/// A search or a commit-graph job ended without the answer it was for;
/// `core_event`'s guard routes only those here.
fn side_job_ended(next: &mut AppState, event: Event) {
    let (Event::JobFailed(id, _) | Event::JobFinished(id) | Event::JobCancelled(id)) = &event
    else {
        return;
    };
    if search::searching(next, *id) {
        search::search_job_ended(next, event);
    } else {
        commit_graph::commit_graph_job_ended(next, event);
    }
}

/// Screen rows above the graph body: the header line `graph_screen` draws at
/// the top (`CHROME_ROWS` also counts the footer, which a mouse row never
/// needs to skip past — `graph_rows` already excludes it). A harness banner
/// adds its rows; see [`rows_above_graph`].
const HEADER_ROWS: u16 = 1;

/// Screen rows above the first graph row: the header and any banner.
fn rows_above_graph(state: &AppState) -> u16 {
    HEADER_ROWS.saturating_add(state.harness.banner_rows())
}

/// Rows the wheel moves the viewport per notch (DESIGN.md §2.15). The
/// keyboard's `j`/`k` move the selection; the wheel moves the viewport
/// instead and leaves it alone, the way every other terminal pager's wheel
/// does.
const WHEEL_LINES: isize = 3;

/// The half of [`core_event`] that answers the mouse (DESIGN.md §2.15).
fn mouse_event(next: &mut AppState, commands: &mut Vec<Command>, mouse: MouseInput) {
    match mouse.kind {
        MouseKind::ScrollUp => scroll(next, commands, -WHEEL_LINES),
        MouseKind::ScrollDown => scroll(next, commands, WHEEL_LINES),
        MouseKind::Down(MouseButton::Left) => {
            if let Some(row) = graph_row_at(next, mouse.row) {
                next.on_working_tree = false;
                select_row(next, commands, row);
            } else if next.uncommitted_row_shown() && mouse.row == rows_above_graph(next) {
                // The first body row while the working-tree row is shown:
                // `graph_row_at` returns `None` for it (offset 0, displaced
                // by the virtual row), so this is the one case its offset
                // math does not already cover.
                next.on_working_tree = true;
            }
        }
        // Any-motion reporting (`terminal.rs`) is what makes this arrive with
        // no button held — hover, not a drag.
        MouseKind::Moved => next.hover = graph_row_at(next, mouse.row),
        _ => {}
    }
}

/// Scrolls the viewport by `lines` wheel notches; the selection stays put.
fn scroll(next: &mut AppState, commands: &mut Vec<Command>, lines: isize) {
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.scroll(lines, height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}

/// Selects the row a click landed on.
fn select_row(next: &mut AppState, commands: &mut Vec<Command>, row: usize) {
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.select_at(row, height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}

/// The absolute history row a screen row names, if it names a row at all:
/// `None` over the header, the footer, the uncommitted-changes row (selected
/// through the flag, not a `History` row — see `mouse_event`), or a row
/// below what has laid out so far.
fn graph_row_at(state: &AppState, screen_row: u16) -> Option<usize> {
    let history = state.history.as_ref()?;
    let offset = usize::from(screen_row.checked_sub(rows_above_graph(state))?);
    if offset >= state.graph_rows() {
        return None;
    }
    let offset = if crate::view::uncommitted_shown(state) {
        offset.checked_sub(1)?
    } else {
        offset
    };
    let absolute = history.top + offset;
    (absolute < history.rows).then_some(absolute)
}

/// Ask the walk for the rows the viewport now holds, if it needs any.
fn show_rows(next: &mut AppState, commands: &mut Vec<Command>) {
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.show(height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}

#[cfg(test)]
mod tests;
