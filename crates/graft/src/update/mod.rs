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
//! promotion (ADR-0008); [`mouse`] answers the mouse, over the pure screen
//! geometry [`geometry`] computes (PROMPT.md 6.2, m2-details-and-diffs review
//! M8 — both used to live in this file). This file keeps only the
//! dispatcher: `update`, `answer` and `core_event`.

pub mod commit_editor;
mod commit_graph;
pub mod confirm;
mod context_menu;
pub mod detail;
mod events;
mod geometry;
mod history;
pub mod journal;
mod keys;
pub mod map;
mod mouse;
pub mod operate;
mod prelude;
pub mod radar;
mod search;
mod selection;
mod state;
mod strip;
pub mod undo;

pub use events::{AppEvent, Command, CommitGraphOffer, GitCheck, Product, Progress, Task};
pub(crate) use geometry::context_menu_area;
pub use state::{AppState, Banner, CHROME_ROWS, Harness, Quit, Screen, Size, Status};

use graft_core::event::Event;
use graft_core::job::JobId;

use crate::history::{History, Phase};
use crate::repository::RepositoryFacts;

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
    settle(&mut next, &mut commands, cursor_before);
    // The harness's prelude (m2-details-and-diffs/23): each key gets the
    // same follow-up a real key does, so one that starts a job leaves the
    // rest waiting for the answer; `take_next` hands out nothing until then.
    while let Some(key) = prelude::take_next(&mut next) {
        let cursor_before = next.history.as_ref().map_or(0, |history| history.cursor);
        answer(&mut next, &mut commands, AppEvent::Core(Event::Key(key)));
        settle(&mut next, &mut commands, cursor_before);
    }
    // The commit-graph line costs a graph row: when it comes or goes, the
    // viewport changed height, so ask for the rows that now fit.
    if next.commit_graph_line().is_some() != state.commit_graph_line().is_some() {
        show_rows(&mut next, &mut commands);
    }
    (next, commands)
}

/// What follows every answered event: the selection reconciled with the
/// cursor, the pane and the radar asking for what they now need.
fn settle(next: &mut AppState, commands: &mut Vec<Command>, cursor_before: usize) {
    selection::reconcile(next, cursor_before);
    // `local_jobs` borrowed out for the closure: `follow_selection` also
    // wants `&mut next`, and the two can't overlap.
    let mut local_jobs = next.local_jobs;
    commands.extend(selection::follow_selection(next, &mut || {
        local_jobs.next_id()
    }));
    radar::request_context(next, commands, &mut || local_jobs.next_id());
    radar::request_force_push_probes(next, commands, &mut || local_jobs.next_id());
    radar::request_duplicates_probe(next, commands, &mut || local_jobs.next_id());
    journal::request(next, commands, &mut || local_jobs.next_id());
    // Last of the radar's work: the findings the refs carry, rebuilt only
    // when the refs, `HEAD`, the default branch, the context or the policy
    // moved — so a frame looks a row up instead of walking every ref
    // (M2 review C1).
    radar::refresh_ref_findings(next);
    commands.extend(detail::open_explain(next, &mut || local_jobs.next_id()));
    next.local_jobs = local_jobs;
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
        AppEvent::Started(id, task) => started(next, commands, id, &task),
        // The detail pane names its jobs itself, before they start.
        event @ AppEvent::Produced(
            _,
            Product::Files(_)
            | Product::FileDiff(..)
            | Product::ContainedIn(..)
            | Product::EvilProbe(..),
        ) => detail::detail_event(next, commands, event),
        event @ AppEvent::Produced(
            _,
            Product::RadarContext(_) | Product::ForcePushProbe(..) | Product::DuplicatesProbe(_),
        ) => radar::radar_event(next, event),
        AppEvent::Produced(id, Product::TopologicalOrder(order)) => {
            history::sorted(next, commands, id, order);
        }
        AppEvent::Produced(_, Product::Repository(result)) => repository_answer(next, result),
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
        event @ (AppEvent::Produced(_, Product::Map(_))
        | AppEvent::Progress(_, Progress::Map { .. })) => map::map_event(next, event),
        event @ (AppEvent::Produced(_, Product::Operated(_))
        | AppEvent::Progress(_, Progress::Operating { .. })) => {
            operate_answer(next, commands, event);
        }
        AppEvent::Produced(id, Product::Undone(outcome)) => {
            let mut local_jobs = next.local_jobs;
            commands.extend(undo::undone(next, id, *outcome, &mut || {
                local_jobs.next_id()
            }));
            next.local_jobs = local_jobs;
        }
        AppEvent::Produced(id, product @ Product::Journal(_)) => {
            let taken: bool = journal::journal_event(next, id, &product);
            debug_assert!(taken, "the journal's own product");
        }
        event @ AppEvent::Produced(
            _,
            Product::CommitTemplate(_)
            | Product::HeadMessage(_)
            | Product::ExternalEditor(_)
            | Product::HandedOver(_),
        ) => commit_editor_answer(next, event),
    }
}

/// A job said it had started. Three tasks take a note of the id; the rest are
/// not waited on by `start`, and are welcome whenever they arrive.
///
/// The commit graph's two are answered before this, in [`answer`].
fn started(next: &mut AppState, commands: &mut Vec<Command>, id: JobId, task: &Task) {
    match task {
        Task::ReadHistory => history_started(next, commands, id),
        Task::PromoteHistory => history::sort_started(next, commands, id),
        Task::Search => search::search_started(next, commands, id),
        Task::OpenRepository
        | Task::CheckSystemGit
        | Task::ReadRefs
        | Task::ReadWorkingTree
        | Task::ListFiles { .. }
        | Task::FileDiff { .. }
        | Task::ContainedIn { .. }
        | Task::RadarContext { .. }
        | Task::EvilProbe { .. }
        | Task::ForcePushProbe { .. }
        | Task::DuplicatesProbe { .. }
        | Task::Map
        | Task::Operate
        | Task::Undo
        | Task::ReadJournal
        | Task::ReadCommitTemplate
        | Task::ReadHeadMessage
        | Task::WriteCommitGraph
        | Task::DeclineCommitGraph => {}
    }
}

/// The three products the commit editor waits on
/// (m3-working-tree-undo-and-stash/05).
fn commit_editor_answer(next: &mut AppState, event: AppEvent) {
    match event {
        AppEvent::Produced(id, Product::CommitTemplate(template)) => {
            commit_editor::template_arrived(next, id, template);
        }
        AppEvent::Produced(id, Product::HeadMessage(message)) => {
            commit_editor::head_message_arrived(next, id, message);
        }
        AppEvent::Produced(id, Product::ExternalEditor(result)) => {
            commit_editor::editor_returned(next, id, result);
        }
        // `e`'s file, or `Ctrl+Z`'s suspend: nothing came back but whether
        // the terminal handover worked, and a failure is the user's to see.
        AppEvent::Produced(_, Product::HandedOver(Err(why))) => {
            next.status = Status::Sticky(why);
        }
        AppEvent::Produced(_, Product::HandedOver(Ok(()))) => {}
        other => debug_assert!(false, "not the commit editor's: {other:?}"),
    }
}

/// The two events an operate job can send: a running write's description
/// (`Progress::Operating`), and its answer (`Product::Operated`). Split out
/// of [`answer`] to keep that match within the line budget.
fn operate_answer(next: &mut AppState, commands: &mut Vec<Command>, event: AppEvent) {
    match event {
        AppEvent::Produced(id, Product::Operated(outcome)) => {
            let mut local_jobs = next.local_jobs;
            commands.extend(operate::operated(next, id, *outcome, &mut || {
                local_jobs.next_id()
            }));
            next.local_jobs = local_jobs;
        }
        AppEvent::Progress(id, Progress::Operating { description })
            if next.operating == Some(id) =>
        {
            next.status = Status::Busy(description);
        }
        _ => {}
    }
}

/// The repository opened, or failed to — `answer` routes only
/// [`Product::Repository`] here.
fn repository_answer(next: &mut AppState, result: Result<RepositoryFacts, String>) {
    match result {
        Ok(facts) => repository_opened(next, facts),
        Err(failure) => repository_open_failed(next, failure),
    }
}

/// The repository opened, or re-opened on a refresh.
fn repository_opened(next: &mut AppState, facts: RepositoryFacts) {
    next.repository = Some(facts);
    // Whatever went wrong last time has been superseded by a repository
    // that opened.
    next.status = Status::Ready;
    commit_graph::decide_commit_graph_offer(next);
}

/// A history walk started: it replaces whatever walk came before.
fn history_started(next: &mut AppState, commands: &mut Vec<Command>, id: graft_core::job::JobId) {
    // The newest ask wins. Anything the previous walk was going to
    // say is now about a question nobody asked.
    next.awaiting = Some(id);
    let mut history = History::started();
    history.strip_rows = keys::strip_rows_for(next.tier());
    // A refresh keeps what the user folded, hid or switched to, and puts the
    // cursor back on the commit it was on — including a `Ctrl+R` that
    // rebuilt the id table for a commit-graph the walk had not picked up yet
    // (m1-the-graph/54): `update` cannot see that the table was rebound
    // without new cross-thread plumbing, so every walk restart reanchors,
    // the same as `change_view` already does.
    if let Some(previous) = &next.history {
        history.view = previous.view.clone();
        history.reanchor = previous.selected().map(|row| row.commit.id);
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
        Product::WorkingTree(Ok(read)) => {
            next.working_tree = Some(read.status);
            next.in_progress = read.in_progress;
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
        | Product::ContainedIn(..)
        | Product::RadarContext(_)
        | Product::EvilProbe(..)
        | Product::ForcePushProbe(..)
        | Product::DuplicatesProbe(_)
        | Product::Map(_)
        | Product::Operated(_)
        | Product::Undone(_)
        | Product::Journal(_)
        | Product::CommitTemplate(_)
        | Product::HeadMessage(_)
        | Product::ExternalEditor(_)
        | Product::HandedOver(_) => {}
    }
}

/// A repository failed to open, or to re-open.
fn repository_open_failed(next: &mut AppState, failure: String) {
    if next.repository.is_none() {
        // Nothing was ever opened, so there is no screen to show this on and
        // nothing for the user to do with Graft. Leaving, with the reason,
        // beats an empty screen with an error in a corner.
        next.quit = Some(Quit::Fatal(failure));
    } else {
        // A refresh of a repository that is already open failed — the
        // directory went away mid-run, say. The screen still has something to
        // show, so this goes on the status line.
        next.status = Status::Failed(failure);
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
            map::resized(next, commands);
        }
        event @ (Event::JobFailed(id, _) | Event::JobFinished(id) | Event::JobCancelled(id))
            if search::searching(next, id) || commit_graph::commit_graph_job(next, id) =>
        {
            side_job_ended(next, event);
        }
        Event::JobFailed(id, failure) => job_failed(next, commands, id, &failure),
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
            //
            // Two exceptions, both about writes. A write still in flight
            // owns the line: only the history walk ends in `JobFinished`
            // (every other job produces a `Product`), and a walk finishing
            // says nothing about a write that has not answered yet — wiping
            // `<description>` there would leave the write keys dead with
            // nothing on screen saying why. And a write that *failed* owns
            // it too: the refresh that failure itself asks for finishes a
            // moment later, and clearing on that wiped a rejected hook's
            // message off the screen before it could be read. A read
            // failure is the only one a later success makes stale
            // (m3/04 review gap, m3-working-tree-undo-and-stash/05).
            if !matches!(next.status, Status::Busy(_) | Status::Sticky(_)) {
                next.status = Status::Ready;
            }
        }
        Event::JobCancelled(id) => job_cancelled(next, commands, id),
        Event::Mouse(mouse) => mouse::mouse_event(next, commands, mouse),
        Event::Paste(text) => {
            // The commit editor is the only thing that takes a paste so far.
            // A paste arriving anywhere else is not an error and there is
            // nothing to report: the user pasted at a screen with no text
            // field on it.
            let _taken: bool = commit_editor::paste(next, &text);
        }
        Event::FocusGained => focus_gained(next, commands),
        // Nothing is bound to this one. It reaches `update` so that the
        // ticket that needs it has somewhere obvious to add the handling.
        Event::FocusLost => {}
    }
}

/// How long after a focus-gained refresh the next one is ignored. A
/// terminal reports focus on every alt-tab, some report it twice, and tmux
/// reports it for every pane switch; without this, resting on the window
/// edge would re-read the working tree as fast as the reports arrive
/// (m3-working-tree-undo-and-stash/06, design note change 7).
const FOCUS_DEBOUNCE_MS: i64 = 500;

/// The terminal came back to the front: whatever was done to the repository
/// in another window is what the user now expects to see.
///
/// The working tree only. Not the history walk — restarting it on every
/// alt-tab would throw away the rows already on screen and move the cursor
/// under the user's hands; `Ctrl+R` is still the way to ask for everything.
fn focus_gained(next: &mut AppState, commands: &mut Vec<Command>) {
    // Nothing to re-read before the repository has opened: the first read
    // is already on its way.
    if next.repository.is_none() {
        return;
    }
    let now = next.harness.clock.now_millis();
    if next
        .last_focus_refresh
        .is_some_and(|last| now.saturating_sub(last) < FOCUS_DEBOUNCE_MS)
    {
        return;
    }
    next.last_focus_refresh = Some(now);
    commands.push(Command::ReadWorkingTree);
}

/// A job was cancelled. Not a failure and not news — the application asked
/// for it — so the status line keeps whatever it was already saying.
fn job_cancelled(next: &mut AppState, commands: &mut Vec<Command>, id: graft_core::job::JobId) {
    // A walk cancelled by anything other than `Esc` — shutdown, say — must
    // not leave the screen waiting for it either.
    if next.awaiting == Some(id) {
        next.awaiting = None;
        history::set_phase(next, Phase::Cancelled);
        history::give_up_promotion(next, commands);
    }
    if next.promoting == Some(id) {
        history::give_up_promotion(next, commands);
    }
    let mut local_jobs = next.local_jobs;
    commands.extend(operate::write_job_ended(next, id, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
}

/// A job failed: whatever was waiting on it stops waiting, the failure is
/// put on the status line, and a write that died part-way is cleared.
fn job_failed(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: graft_core::job::JobId,
    failure: &dyn std::fmt::Display,
) {
    if next.awaiting == Some(id) {
        next.awaiting = None;
        history::set_phase(next, Phase::Failed(failure.to_string()));
        history::give_up_promotion(next, commands);
    }
    if next.promoting == Some(id) {
        history::give_up_promotion(next, commands);
    }
    // Surfaced, not swallowed: the user is told which job failed and why. A
    // panicking job says so in the same place — it is still a failure, and
    // the crash report's path rides along with it.
    next.status = Status::Failed(format!("{id} failed: {failure}"));
    let mut local_jobs = next.local_jobs;
    commands.extend(operate::write_job_ended(next, id, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
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
