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

pub mod banner;
pub mod commit_editor;
mod commit_graph;
pub mod confirm;
mod context_menu;
pub mod credential;
pub mod detail;
pub mod drag;
mod events;
mod geometry;
mod history;
pub mod journal;
pub mod keys;
pub mod map;
mod mouse;
pub mod name_prompt;
pub mod operate;
mod prelude;
pub mod preview;
pub mod radar;
mod relaunch;
pub mod remote;
pub mod resolver;
mod search;
mod selection;
pub mod stash;
mod state;
mod status_read;
mod strip;
pub mod undo;

pub use events::{
    AppEvent, Command, CommitGraphOffer, GitCheck, Product, Progress, TakeRequest, Task,
};
pub(crate) use geometry::context_menu_area;
pub use relaunch::LiveOrigin;
pub use state::{
    AppState, Banner, CHROME_ROWS, GraphOverlay, Harness, HeadLanding, Quit, Screen, Size, Status,
    Stop,
};

use graft_core::event::{Event, KeyPress};
use graft_core::job::JobId;

use crate::history::{History, Phase};
use crate::repository::RepositoryFacts;
use credential::credential_needed;

/// The state Graft starts in, and the jobs that fill it.
///
/// Both the real loop (`crate::event_loop`) and the integration harnesses go
/// through here rather than writing the opening list out again, because the
/// history walk's id is minted here: `AppState::awaiting` has to name the
/// walk from the moment it is asked for, and a second list that forgot to do
/// that would put the `Ctrl+R` race back for whoever used it
/// (m5-history-and-ghost-preview/05).
///
/// Every one of these reads the world — the repository, `git --version`, the
/// history, the refs, the working tree — so every one is a job: the first
/// frame is drawn while they are still running, and says so, rather than
/// waiting on the disk.
#[must_use]
pub fn opening(repo_path: std::path::PathBuf, size: Size) -> (AppState, Vec<Command>) {
    let mut state = AppState {
        size,
        repo_path,
        ..AppState::default()
    };
    let walk = state.local_jobs.next_id();
    state.awaiting = Some(walk);
    let refs = state.local_jobs.next_id();
    let tree = state.local_jobs.next_id();
    let read_tree = status_read::ask(&mut state, tree);
    let commands = vec![
        Command::OpenRepository,
        Command::CheckSystemGit,
        // The one with work in it. Asked for in topological order, which
        // ADR-0008 downgrades to commit date when the repository has no
        // commit-graph file.
        Command::ReadHistory(walk, graft_git::history::Order::Topological),
        // Badges: ahead/behind walks commits.
        Command::ReadRefs(refs),
        // The uncommitted-changes row.
        read_tree,
    ];
    (state, commands)
}

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
    // However the write ended, its question goes with it.
    credential::close_if_orphaned(next);
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
    remote::request_at_launch(next, commands, &mut || local_jobs.next_id());
    // Last of the radar's work: the findings the refs carry, rebuilt only
    // when the refs, `HEAD`, the default branch, the context or the policy
    // moved — so a frame looks a row up instead of walking every ref
    // (M2 review C1).
    radar::refresh_ref_findings(next);
    remote::forget_stale_divergence(next);
    commands.extend(detail::open_explain(next, &mut || local_jobs.next_id()));
    next.local_jobs = local_jobs;
}

/// The half of [`update`] that answers one event, before the viewport is
/// squared with the commit-graph line.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per event, matched exhaustively and dispatched straight to its \
              module: the way to shorten it is to hand a whole event to a module that \
              matches it again behind a catch-all, which is what m4-remotes/05's review \
              (M4) had taken out of here"
)]
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
        AppEvent::Produced(id, product @ Product::Stashes(_)) => stash_answer(next, id, &product),
        // The overlay's file preview reads through `ListFiles` like the
        // detail pane does, under its own id; the guard asks which of the
        // two is waiting for this one.
        AppEvent::Produced(id, product @ Product::Files(_))
            if stash::claims(next, id, &product) =>
        {
            stash_answer(next, id, &product);
        }
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
            id,
            product @ (Product::SystemGit(_) | Product::Refs(_) | Product::WorkingTree(_)),
        ) => took_fact(next, commands, id, product),
        AppEvent::Progress(id, Progress::History(update)) => {
            history::walk_progress(next, commands, id, update);
        }
        AppEvent::Progress(id, Progress::Search(hits)) => {
            search::search_progress(next, commands, id, hits);
        }
        event @ (AppEvent::Produced(_, Product::Map(_))
        | AppEvent::Progress(_, Progress::Map { .. })) => map::map_event(next, event),
        event @ (AppEvent::Produced(_, Product::Operated(_))
        | AppEvent::Progress(
            _,
            Progress::Operating { .. } | Progress::CredentialNeeded { .. },
        )) => operate_answer(next, commands, event),
        AppEvent::Produced(id, Product::Resumed(outcome)) => {
            let mut local_jobs = next.local_jobs;
            commands.extend(banner::resumed(next, id, *outcome, &mut || {
                local_jobs.next_id()
            }));
            next.local_jobs = local_jobs;
        }
        AppEvent::Produced(_, Product::Mergetool { path, result }) => {
            mergetool_returned(next, commands, &path, result);
        }
        AppEvent::Produced(id, Product::ConflictOpened(result)) => {
            resolver::conflict_opened(next, id, result);
        }
        AppEvent::Produced(id, Product::Taken(result)) => {
            let mut local_jobs = next.local_jobs;
            resolver::taken(next, id, result, commands, &mut || local_jobs.next_id());
            next.local_jobs = local_jobs;
        }
        AppEvent::Produced(id, Product::BaseLoaded(result)) => {
            resolver::base_loaded(next, id, result);
        }
        AppEvent::Produced(id, Product::ConflictSet(result)) => {
            resolver::conflict_set_read(next, id, result);
        }
        AppEvent::Produced(id, Product::Preview(result)) => {
            preview::preview_produced(next, id, result);
        }
        AppEvent::Produced(id, Product::RevertedTo(done)) => {
            reverted_to_answer(next, commands, id, *done);
        }
        AppEvent::Produced(id, Product::Undone(done)) => undone_answer(next, commands, id, *done),
        AppEvent::Produced(id, product @ Product::Journal(_)) => {
            journal_answer(next, commands, id, &product);
        }
        AppEvent::Produced(id, Product::Remotes(result)) => {
            remote::remotes_arrived(next, id, result);
        }
        AppEvent::Progress(id, Progress::RemoteProgress { brief }) => {
            remote::remote_progress(next, id, &brief);
        }
        // `e` from inside the resolver: `$EDITOR` had the file, so the
        // document on screen is not what is on disk any more. Before the
        // commit editor's arm, which is where every other `HandedOver`
        // goes — the resolver is modal, so while it is open there is no
        // other handover this can be (`Ctrl+Z` is swallowed by
        // `resolver_key`, which runs before `suspend_on_ctrl_z`).
        AppEvent::Produced(_, Product::HandedOver(result)) if next.resolver.is_some() => {
            if let Err(why) = result {
                next.status = Status::sticky(why);
            }
            let mut local_jobs = next.local_jobs;
            resolver::handed_back(next, commands, &mut || local_jobs.next_id());
            next.local_jobs = local_jobs;
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

/// `git mergetool` came back.
///
/// The working tree is re-read **either way**: a tool that exited non-zero
/// may still have written the file before it did, and a stale `CONFLICTS`
/// list is how a user comes to press `c` on a conflict they have already
/// resolved. A failure keeps `a abort` in its keys, like every other line
/// the conflict interim puts up.
fn mergetool_returned(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    path: &std::path::Path,
    result: Result<(), String>,
) {
    match result {
        // Nothing to report: the banner and the file list say what the
        // state is now, and a "mergetool ok" line would be a status field
        // reading like news when nothing happened.
        Ok(()) => {
            if matches!(next.status, Status::Busy(_)) {
                next.status = Status::Ready;
            }
        }
        Err(why) => {
            next.status = Status::Sticky {
                reason: why,
                keys: Some(format!("e edits {}  a abort", path.display())),
            };
        }
    }
    let tree = next.local_jobs.next_id();
    commands.push(status_read::ask(next, tree));
    commands.extend(next.detail.refresh(&mut || next.local_jobs.next_id()));
    // The resolver's `M`: its document is as stale as the file list was.
    // Both handovers land here now — `e` through `Product::HandedOver` in
    // [`answer`] — so the two behave the same, which they never did
    // (m6-conflicts/04; `e` scheduled no read at all).
    if next.resolver.is_some() {
        let mut local_jobs = next.local_jobs;
        resolver::handed_back(next, commands, &mut || local_jobs.next_id());
        next.local_jobs = local_jobs;
    }
}

/// The journal read answered.
fn journal_answer(next: &mut AppState, commands: &mut Vec<Command>, id: JobId, product: &Product) {
    let mut local_jobs = next.local_jobs;
    let taken: bool =
        journal::journal_event(next, commands, id, product, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    debug_assert!(taken, "the journal's own product");
}

/// The stash overlay's own product — the list, or the preview's file list.
fn stash_answer(next: &mut AppState, id: JobId, product: &Product) {
    let taken: bool = stash::stash_event(next, id, product);
    debug_assert!(taken, "the stash overlay's own product");
}

/// The journal's time machine answered, the same way.
fn reverted_to_answer(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    outcome: crate::jobs::undo::RevertedTo,
) {
    let mut local_jobs = next.local_jobs;
    commands.extend(undo::reverted_to(next, id, outcome, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
}

/// One `u`/`U` answered, with `local_jobs` borrowed out for the closure the
/// way every other call site does it.
fn undone_answer(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    outcome: crate::jobs::undo::Undone,
) {
    let mut local_jobs = next.local_jobs;
    commands.extend(undo::undone(next, id, outcome, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
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
        | Task::Resume
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
        | Task::PlanPreview
        | Task::RevertTo
        | Task::ReadStashes
        | Task::ReadRemotes
        | Task::OpenConflict
        | Task::TakeSide
        | Task::LoadBase
        | Task::ReadConflictSet
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
            next.status = Status::sticky(why);
        }
        AppEvent::Produced(_, Product::HandedOver(Ok(()))) => {}
        other => debug_assert!(false, "not the commit editor's: {other:?}"),
    }
}

/// The events an operate job can send: a running write's description
/// (`Progress::Operating`), git's credential question
/// (`Progress::CredentialNeeded`, m4-remotes/06) and its answer
/// (`Product::Operated`). Split out of [`answer`] to keep that match within
/// the line budget.
fn operate_answer(next: &mut AppState, commands: &mut Vec<Command>, event: AppEvent) {
    match event {
        AppEvent::Produced(id, Product::Operated(outcome)) => {
            // Read before the outcome is handed over: only a write that
            // actually ran moves the resolver on. A `git add` that was
            // refused leaves the file exactly where it was, and jumping to
            // the next conflict would hide the refusal behind a new screen.
            let staged = matches!(
                *outcome,
                crate::jobs::operate::Operated::Done(_)
                    | crate::jobs::operate::Operated::Incomplete { .. }
            );
            let mut local_jobs = next.local_jobs;
            commands.extend(operate::operated(next, id, *outcome, &mut || {
                local_jobs.next_id()
            }));
            // `⏎`'s `git add` on the open file, or a whole-file `o`/`t`
            // (which also stages): on to the next conflicted one, or out.
            // `d`'s restart is the other write a resolver can launch, and
            // goes the other way — back to the same file, freshly read.
            // Both compare the id with the one the resolver launched, so
            // only the write that matches acts.
            if staged {
                resolver::resolved_advance(next, commands, id, &mut || local_jobs.next_id());
                resolver::restart_landed(next, commands, id, &mut || local_jobs.next_id());
            }
            next.local_jobs = local_jobs;
        }
        // `launch` already put the write's words on the line; this only
        // restores them if something else took the line meanwhile. It never
        // overwrites a `Busy` — that would wipe a remote write's
        // `· esc cancels`, its progress, or `Esc`'s `cancelling`.
        AppEvent::Progress(id, Progress::Operating { description })
            if next.operating == Some(id) && !matches!(next.status, Status::Busy(_)) =>
        {
            next.status = Status::Busy(description);
        }
        // git is waiting on an answer only the user has.
        AppEvent::Progress(id, Progress::CredentialNeeded { prompt, pulse_seed }) => {
            credential_needed(next, id, &prompt, pulse_seed);
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
    // A read failure from last time has been superseded by a repository
    // that opened. Nothing else is: a write's `Sticky` failure stays until
    // `Esc` (PROMPT.md §3), and the refresh that very failure asked for is
    // what lands here — clearing it wiped `push rejected: …` before it could
    // be read (m4-remotes/05). A running write's `Busy` and a finished
    // one's `Done` are the write's line, not the repository's.
    if matches!(next.status, Status::Failed(_)) {
        next.status = Status::Ready;
    }
    commit_graph::decide_commit_graph_offer(next);
}

/// A history walk started: it replaces whatever walk came before.
///
/// The walk the screen is waiting for was named when its command was built
/// (`Command::ReadHistory`), so this event tells `update` nothing it does not
/// already know — except which walk it is from. A `Started` under any other
/// id belongs to a walk already superseded and cancelled, and taking it would
/// blank the fresh walk's rows and put its cursor back on the wrong commit
/// (m5-history-and-ghost-preview/05).
fn history_started(next: &mut AppState, commands: &mut Vec<Command>, id: graft_core::job::JobId) {
    if next.awaiting != Some(id) {
        return;
    }
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
    // A finished `--continue` waits on this walk too: a seek into the
    // walk it replaced would be thrown away with it (m6-conflicts/12).
    banner::walk_started(next, commands);
}

/// A working-tree read came back: the status, git's in-progress state and
/// the mergetool, and whatever landing they settle.
fn working_tree_read(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    read: crate::working_tree::WorkingTree,
) {
    // Read before the assignment: the launch read is the one that finds no
    // working tree yet. See `relaunch::land_on_live_operation`.
    let first_read = next.working_tree.is_none();
    if let Some(disagreement) = undo::reread_disagrees(next, read.in_progress.as_ref()) {
        next.status = disagreement;
    }
    next.working_tree = Some(read.status);
    next.in_progress = read.in_progress;
    next.mergetool = read.mergetool;
    // The banner cannot have focus once there is no banner: the operation
    // finished, was aborted, or was finished from another terminal. Leaving
    // the flag set would make every key vanish into a modal nothing
    // (m5-history-and-ghost-preview/09).
    if next.in_progress.is_none() {
        next.banner_focus = false;
        // Nor can anything be stopped: the operation ended, in another
        // terminal or by an answer this read confirms. A stop left standing
        // would word a reason for an operation git no longer has
        // (m6-conflicts/12).
        next.stopped = None;
        next.live_landing_owed = false;
    } else {
        // The launch lands on git's word alone; the journal only says whose
        // operation it is (m6-conflicts/12).
        let mut local_jobs = next.local_jobs;
        relaunch::working_tree_read_landed(next, commands, first_read, &mut || {
            local_jobs.next_id()
        });
        next.local_jobs = local_jobs;
    }
}

/// A fact about the repository beside the graph arrived: the system git
/// check, the refs, or the working tree counts. `answer` routes only those.
fn took_fact(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: graft_core::job::JobId,
    product: Product,
) {
    match product {
        Product::SystemGit(check) => {
            next.system_git = Some(check);
        }
        Product::Refs(Ok(refs)) => {
            next.refs = Some(std::sync::Arc::new(refs));
            // A finished `--continue` waits on this read for the new
            // `HEAD` (m6-conflicts/12); any other read passes it by.
            banner::refs_answered(next, commands, id);
        }
        Product::Refs(Err(failure)) => {
            // The graph still draws without badges; the reason goes on the
            // status line rather than leaving them silently missing.
            next.status = Status::Failed(failure);
            // The read a finished `--continue` waited on has no `HEAD` to
            // give: the landing is dropped (the failure is on screen)
            // rather than spent on the stale refs still held.
            if next.head_landing.refs == Some(id) {
                next.land_on_head = false;
                next.head_landing = state::HeadLanding::default();
            }
        }
        Product::WorkingTree(Ok(read)) => {
            let owed = status_read::answered(next, id);
            working_tree_read(next, commands, *read);
            if owed {
                status_read::settle(next);
            }
        }
        Product::WorkingTree(Err(failure)) => {
            // The read the line waited on has no status to give; the
            // failure below replaces the line either way.
            let _owed: bool = status_read::answered(next, id);
            // The graph still draws without the uncommitted row; the reason
            // goes on the status line rather than leaving it silently
            // missing.
            next.status = Status::Failed(failure);
        }
        // Not a fact: `answer`'s pattern keeps these from arriving.
        Product::Repository(_)
        | Product::Resumed(_)
        | Product::Mergetool { .. }
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
        | Product::Preview(_)
        | Product::RevertedTo(_)
        | Product::Stashes(_)
        | Product::Remotes(_)
        | Product::CommitTemplate(_)
        | Product::HeadMessage(_)
        | Product::ExternalEditor(_)
        | Product::HandedOver(_)
        | Product::ConflictOpened(_)
        | Product::Taken(_)
        | Product::BaseLoaded(_)
        | Product::ConflictSet(_) => {}
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

/// The name prompt's turn at a key, right after the credential prompt's and
/// before the search prompt's: a one-field modal that claims every key but
/// `Ctrl+C`, so a branch named `search` cannot open the search prompt
/// halfway through being typed (m3/06 review, Critical 1).
///
/// `local_jobs` is borrowed out because the closure and `next` cannot
/// overlap.
fn name_prompt_claimed(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed = name_prompt::name_prompt_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The half of [`answer`] that answers the events `core` defines.
fn core_event(next: &mut AppState, commands: &mut Vec<Command>, event: Event) {
    match event {
        // The search's keys go first: the open prompt takes every letter.
        // A handed-back key is checked next, before any graph binding, so
        // a harness can claim a key whatever the graph would do with it.
        Event::Key(key) => {
            // The credential prompt before anything, the search prompt
            // included: it opens by itself over whatever was on screen, and
            // a password typed while the search prompt happened to be open
            // must land in the credential field, not the search.
            if credential::credential_key(next, commands, key) {
                return;
            }
            if name_prompt_claimed(next, commands, key) {
                return;
            }
            if next.search_prompt.is_some() || !next.harness.hand_back.contains(&key.code) {
                // An open search prompt takes every key. Otherwise `/`, `n`
                // and `N` are the graph's, and a modal pane or box that is
                // up owns them (§2.1): a `/` in a commit subject is a
                // character, `n` in the resolver its next block
                // (m6-conflicts/07 F1, final fix round). No modal opens over
                // an open prompt — the prompt takes the key that would —
                // so skipping the search here drops no text.
                let search_answers = next.search_prompt.is_some() || !next.modal_owns_keys();
                if !(search_answers && search::search_key(next, commands, key)) {
                    keys::key_event(next, commands, key);
                }
            } else {
                next.quit = Some(Quit::HandedBack(key.code));
            }
        }
        Event::Resize { columns, rows } => {
            // Ticket 10 piece 4: a resize cancels a drag, the same as any
            // other event that moves the badge out from under the pointer.
            // Harmless today (the next `Drag` re-reads `over` against the
            // new layout), but a resize mid-drag is cheap to get right and
            // leaving it in the air is one more state the next reader has
            // to reason about (review m5-10-30-31 m2).
            next.drag = None;
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
            if !matches!(next.status, Status::Busy(_) | Status::Sticky { .. }) {
                next.status = Status::Ready;
            }
        }
        Event::JobCancelled(id) => job_cancelled(next, commands, id),
        Event::Mouse(mouse) => mouse::mouse_event(next, commands, mouse),
        Event::Paste(text) => {
            // The credential prompt, over everything, first; then the
            // commit editor. A paste arriving anywhere else is not an error
            // and there is nothing to report: the user pasted at a screen
            // with no text field on it.
            if !credential::credential_paste(next, &text)
                && !name_prompt::name_prompt_paste(next, &text)
            {
                let _taken: bool = commit_editor::paste(next, &text);
            }
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
    let tree = next.local_jobs.next_id();
    commands.push(status_read::ask(next, tree));
    // Another window may have stashed or popped while this one was away.
    // Only while the overlay is up: nothing else on screen reads the list.
    let mut local_jobs = next.local_jobs;
    stash::refresh(next, commands, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
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
    // The status read the line waited on will not answer.
    if status_read::answered(next, id) {
        status_read::abandon(next);
    }
    let mut local_jobs = next.local_jobs;
    // The write `Esc` stopped: the answer the user asked for. Any other
    // cancelled write (shutdown) is cleared as before.
    match remote::cancelled(next, id, &mut || local_jobs.next_id()) {
        Some(refresh) => commands.extend(refresh),
        None => commands.extend(operate::write_job_ended(next, id, &mut || {
            local_jobs.next_id()
        })),
    }
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
    //
    // A status read the line waited on stops being waited on; the failure
    // replaces the line, so whether it was that read changes nothing more.
    let _owed: bool = status_read::answered(next, id);
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
