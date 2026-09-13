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

use std::path::PathBuf;

use graft_core::capabilities::RenderingTier;
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers, MouseButton, MouseInput, MouseKind};
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::history::Order;
use graft_git::version::SystemGit;

use crate::history::{History, HistoryUpdate, Motion, Phase, Promotion, RowRequest, View};
use crate::repository::RepositoryFacts;
use crate::search::{Search, SearchState};
use graft_graph::Follow;

/// Everything the loop can be told.
///
/// [`Event`] is what `core` can describe on its own — a key, a resize, a job
/// that ended. It cannot carry a repository: `core` does not know what one is,
/// and moving the git types down into it to make it possible would invert the
/// layering (ADR-0007). So `app` wraps it, and jobs that have something to
/// hand over hand it over here. Symmetric with [`Command`]: commands go out,
/// products come back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppEvent {
    /// Something `core` can describe.
    Core(Event),
    /// A job has been queued, and this is the id its result will carry.
    ///
    /// Announced before the work is queued, so the state knows which id it is
    /// waiting for before any result can arrive — otherwise `Esc` has nothing
    /// to cancel until the job it would cancel has already finished. The
    /// runner sends it down the same channel as the result, so it is always
    /// the earlier of the two.
    Started(JobId, Task),
    /// A job that ended with something the application keeps.
    Produced(JobId, Product),
    /// A running job has something to show before it ends. Always between
    /// the job's [`AppEvent::Started`] and its one ending event.
    Progress(JobId, Progress),
}

/// What a running job hands over before it ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    /// The history walk has started, or laid out its next rows.
    History(HistoryUpdate),
    /// A search read another chunk of history and found these commits.
    Search(std::sync::Arc<[graft_core::oid::Oid]>),
}

/// What a job was started to do.
///
/// Only enough to tell the one job whose result the screen waits for from the
/// ones that simply land when they land.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Task {
    /// Open the repository and read what the status line shows.
    OpenRepository,
    /// Ask this machine's `git` what version it is.
    CheckSystemGit,
    /// Walk history.
    ReadHistory,
    /// Read the refs each commit's badges show.
    ReadRefs,
    /// Read the working tree's staged/unstaged/untracked counts.
    ReadWorkingTree,
    /// Work out the strict topological order a provisional walk promotes to.
    PromoteHistory,
    /// Search history for a query (`/`).
    Search,
}

/// What a job brought back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Product {
    /// The repository opened, or it did not. A path that is not a repository
    /// is an ordinary answer to an ordinary question, not a job failure —
    /// [`Event::JobFailed`] stays for jobs that broke.
    Repository(Result<RepositoryFacts, String>),
    /// What the `git` on this machine turned out to be.
    SystemGit(GitCheck),
    /// The refs on each commit and how far each branch is from its
    /// upstream, or why they could not be read.
    Refs(Result<crate::refs::RefBadges, String>),
    /// The working tree's staged/unstaged/untracked counts, or why they
    /// could not be read.
    WorkingTree(Result<graft_git::status::WorkingTreeCounts, String>),
    /// Every commit in strict topological order, for the walk on screen to
    /// promote to (ADR-0008).
    TopologicalOrder(std::sync::Arc<[graft_core::oid::Oid]>),
}

/// What running `git --version` found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitCheck {
    /// `git` ran and said which version it is.
    Ran(SystemGit),
    /// `git` could not be run at all, and this is why. Not fatal at M0 —
    /// nothing has shelled out to it yet — but the user needs to know now
    /// rather than at the first operation that needs it.
    Unavailable(String),
}

/// Everything Graft knows, as one value.
///
/// Minimal for now, on purpose: this ticket builds the loop, not the
/// application. Screens, the repository and the selection arrive with the
/// tickets that need them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppState {
    /// Set once Graft is leaving, and why. The event loop reads it after every
    /// event and returns, which drops the `TerminalSession` and hands the
    /// terminal back — a fatal quit is reported only after that, on a terminal
    /// that is the user's again.
    pub quit: Option<Quit>,
    /// What a harness wrapping the run asked for; default in the shipped
    /// binary.
    pub harness: Harness,
    /// What the status line has to say.
    pub status: Status,
    /// The terminal's last reported size, in character cells.
    pub size: Size,
    /// The repository Graft was pointed at. Empty in a default state, which is
    /// only ever a starting point: `event_loop::run` fills it in before the
    /// first frame.
    pub repo_path: PathBuf,
    /// What the repository turned out to be, once the job that opened it has
    /// answered. `None` until then, which is the first frame.
    pub repository: Option<RepositoryFacts>,
    /// What the system `git` turned out to be, once that job has answered.
    pub system_git: Option<GitCheck>,
    /// The history job whose answer the screen is waiting for.
    ///
    /// One at a time, on purpose: asking again supersedes the previous ask,
    /// and a result carrying any other id belongs to a question that has
    /// since been withdrawn. That is what makes a stale result harmless — it
    /// is dropped here, before it can reach a screen.
    pub awaiting: Option<JobId>,
    /// The topological sort whose order the screen will promote to, while
    /// it runs (ADR-0008). A result from any other sort is dropped.
    pub promoting: Option<JobId>,
    /// The history walk the screen shows: its rows so far and where it is.
    /// `None` until one has been asked for.
    pub history: Option<History>,
    /// The refs on each commit, once the refs job has answered.
    pub refs: Option<crate::refs::RefBadges>,
    /// The working tree's staged/unstaged/untracked counts, once that job
    /// has answered. `None` until then, and while it fails: a failed read
    /// goes on the status line rather than showing a stale count.
    pub working_tree: Option<graft_git::status::WorkingTreeCounts>,
    /// Which screen is on show. The graph, unless the user asked for the
    /// status screen — the only place tier and cell size are shown.
    pub screen: Screen,
    /// The user pressed `+`: every lane draws in its real column instead of
    /// folding behind `+N` (DESIGN.md §3.5). A view-time flag only — the
    /// rows are the same either way, so toggling it asks nothing of the
    /// history job.
    pub lanes_expanded: bool,
    /// Focus mode (`F`, DESIGN.md §5.2): rows off the selection's thread
    /// fade. A paint decision like `lanes_expanded`; the walk is not asked.
    pub focus: bool,
    /// The hover tooltip's row: an absolute history row number, set by
    /// pointer movement over the graph (DESIGN.md §2.15, PROMPT.md §1.3;
    /// m1-the-graph/17). `None` when the pointer is not over a row. A
    /// pinned tooltip (below) takes priority over this one on screen, but
    /// hover keeps tracking the pointer underneath it regardless.
    pub tooltip: Option<usize>,
    /// The tooltip `Enter` pinned open, if any: an absolute history row
    /// number that stays put under hover and scrolling — the commit it was
    /// opened for, not the cursor (m1-the-graph/34's choice, so the tooltip
    /// can be read and copied while the user keeps browsing). `Enter`
    /// toggles it; `Esc` and a second `Enter` close it.
    pub pinned_tooltip: Option<usize>,
    /// The tier-drop key (`F3`) forced this tier for the rest of the run, or
    /// `None` for whatever `Session::settle` decided. A view-time override
    /// like `lanes_expanded` — the rows are untouched, only how they draw
    /// changes (`graph.renderer`, m1-the-graph/19).
    pub renderer_override: Option<RenderingTier>,
    /// The tier `Session::settle` decided, copied in when the loop starts so
    /// `update` knows what `F3` drops from and whether row requests pad to
    /// pixel strips. `None` in a state built without a session (tests,
    /// headless), which draws as the Unicode tier would.
    pub settled_tier: Option<RenderingTier>,
    /// The `/` prompt while it is open: what has been typed so far.
    pub search_prompt: Option<String>,
    /// The search on screen, running or finished, until `Esc` closes it.
    pub search: Option<crate::search::Search>,
}

/// The screens Graft can show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    /// The history graph. What Graft opens on.
    #[default]
    Graph,
    /// What Graft opened and what it is drawing with.
    Status,
}

impl Screen {
    /// The other screen: `F2` swaps between the two.
    #[must_use]
    pub fn toggled(self) -> Screen {
        match self {
            Screen::Graph => Screen::Status,
            Screen::Status => Screen::Graph,
        }
    }
}

/// Screen rows that are not the graph: one line of header above it and one of
/// status below. The graph screen (DESIGN.md §1.3) draws within this.
pub const CHROME_ROWS: u16 = graft_ui::graph_screen::CHROME_ROWS;

impl AppState {
    /// The tier the graph draws at: `F3`'s override, else the settled one.
    #[must_use]
    pub fn tier(&self) -> RenderingTier {
        self.renderer_override
            .or(self.settled_tier)
            .unwrap_or(RenderingTier::Unicode)
    }

    /// How many rows of history the graph pane shows at the current size.
    /// Never zero, so a movement always has somewhere to go.
    #[must_use]
    pub fn graph_rows(&self) -> usize {
        let chrome = CHROME_ROWS.saturating_add(self.harness.banner_rows());
        usize::from(self.size.rows.saturating_sub(chrome)).max(1)
    }
}

/// What a harness wrapping `event_loop::run` asks for (the graph gallery,
/// m1-the-graph/35). The default asks for nothing, which is what the
/// shipped binary passes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Harness {
    /// Lines pinned under the header for the whole run: the gallery says
    /// which stop this is and what to look for (m1-the-graph/38). Each
    /// line costs a graph row.
    pub banner: Option<Banner>,
    /// Keys the loop does not interpret: pressing one sets `quit` to
    /// [`Quit::HandedBack`] so the caller can act on it (the gallery steps
    /// to the next repository). Not consulted while the search prompt is
    /// open, where every printable key is text.
    pub hand_back: Vec<KeyCode>,
}

impl Harness {
    /// Screen rows the banner takes, none without one.
    #[must_use]
    pub fn banner_rows(&self) -> u16 {
        self.banner.as_ref().map_or(0, Banner::rows)
    }
}

/// What a harness pins under the header (m1-the-graph/38). The view appends
/// ` · <tier>` to the title for the rendering tier in force, since only the
/// run knows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Banner {
    /// The first line, in the accent colour: where the user is and which
    /// keys move on.
    pub title: String,
    /// The lines under it, in the info colour: what to look for.
    pub lines: Vec<String>,
}

impl Banner {
    /// Screen rows the banner takes: the title and every line.
    #[must_use]
    pub fn rows(&self) -> u16 {
        u16::try_from(self.lines.len().saturating_add(1)).unwrap_or(u16::MAX)
    }
}

/// Why Graft is leaving.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Quit {
    /// The user asked to.
    Requested,
    /// The user pressed a key the caller asked to have handed back
    /// (`Harness::hand_back`), so the loop stops for the caller to act.
    HandedBack(KeyCode),
    /// Graft cannot go on, and this is the line to print. The terminal is
    /// restored first: a message written into the alternate screen vanishes
    /// with it, which is how "it just exits and says nothing" happens.
    Fatal(String),
}

impl AppState {
    /// Is the loop finished?
    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.quit.is_some()
    }
}

/// What the status line has to say.
///
/// Modelled as a state rather than as a string with an `error:` prefix: a
/// field that always looks like an error teaches the user to ignore it, which
/// is how a real failure gets missed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Status {
    /// Nothing to report.
    #[default]
    Ready,
    /// A background job failed, and this is what it said. Shown until
    /// something replaces it — a failure is never dropped on the floor.
    Failed(String),
}

/// A terminal size in character cells.
///
/// Either number can be zero. A pty that has not been sized yet reports `0×0`,
/// and that is what silently broke the graph spike's layout (ADR-0012), so the
/// size is carried as-is and every consumer is expected to cope.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Size {
    /// Width in character cells.
    pub columns: u16,
    /// Height in character cells.
    pub rows: u16,
}

/// Work for the job runner: something [`update`] wants done that it cannot do
/// itself, because doing it would mean touching the world.
///
/// Plain data, with no closure and no handle in it, so that `update` stays a
/// function whose answer can be compared with `assert_eq!`. Turning a command
/// into the code that runs it is the job runner's business (`crate::jobs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Open the repository Graft was pointed at and check it is one.
    ///
    /// No path: which repository this run is about is settled before the loop
    /// starts and is the job runner's to hold (`crate::repos`), not something
    /// a keystroke gets to change. This is also what `Ctrl+R` (manual
    /// refresh, DESIGN.md §2.1) re-runs, and it is where anything cached
    /// about the repository is thrown away.
    OpenRepository,
    /// Find out whether the `git` on this machine is new enough (2.38,
    /// DESIGN.md §3.1). Runs once, at startup: it shells out, so it is a job.
    CheckSystemGit,
    /// Walk history in this order and bring the commits back.
    ///
    /// The order asked for is not always the order that comes back: without a
    /// commit-graph file a topological walk falls to commit date, and says so
    /// (ADR-0008).
    ReadHistory(Order),
    /// Read the refs for the badges, counting ahead/behind off the loop's
    /// thread.
    ReadRefs,
    /// Read the working tree's staged/unstaged/untracked counts off the
    /// loop's thread.
    ReadWorkingTree,
    /// Work out the whole range's strict topological order, for a walk whose
    /// order is provisional (ADR-0008). Only the newest such job's answer is
    /// used.
    PromoteHistory,
    /// Hand history job `JobId` a request for rows. The rows come back as a
    /// [`HistoryUpdate::Window`] tagged with the request's id.
    ShowRows(JobId, RowRequest),
    /// Stop a job whose result is no longer wanted. Cooperative: a job that has
    /// not started is dropped, and a running one stops at its next check.
    CancelJob(JobId),
    /// Search history for a query off the loop's thread (`/`).
    Search(crate::search::Query),
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

    match event {
        AppEvent::Core(event) => core_event(&mut next, &mut commands, event),
        AppEvent::Started(id, Task::ReadHistory) => {
            // The newest ask wins. Anything the previous walk was going to
            // say is now about a question nobody asked.
            next.awaiting = Some(id);
            let mut history = History::started();
            history.strip_rows = strip_rows_for(next.tier());
            // A refresh keeps what the user folded, hid or switched to.
            if let Some(previous) = &next.history {
                history.view = previous.view.clone();
            }
            if let Some(request) = history.show(next.graph_rows()) {
                commands.push(Command::ShowRows(id, request));
            }
            next.history = Some(history);
        }
        AppEvent::Started(id, Task::PromoteHistory) => sort_started(&mut next, &mut commands, id),
        AppEvent::Started(id, Task::Search) => search_started(&mut next, &mut commands, id),
        // Neither of these is waited on: their results are welcome whenever
        // they arrive, and there is nothing for `Esc` to cancel.
        AppEvent::Started(
            _,
            Task::OpenRepository | Task::CheckSystemGit | Task::ReadRefs | Task::ReadWorkingTree,
        ) => {}
        AppEvent::Produced(id, Product::TopologicalOrder(order)) => {
            sorted(&mut next, &mut commands, id, order);
        }
        AppEvent::Produced(_, Product::Repository(Ok(facts))) => {
            next.repository = Some(facts);
            // Whatever went wrong last time has been superseded by a
            // repository that opened.
            next.status = Status::Ready;
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
        AppEvent::Produced(_, Product::SystemGit(check)) => {
            next.system_git = Some(check);
        }
        AppEvent::Produced(_, Product::Refs(Ok(refs))) => {
            next.refs = Some(refs);
        }
        AppEvent::Produced(_, Product::Refs(Err(failure))) => {
            // The graph still draws without badges; the reason goes on the
            // status line rather than leaving them silently missing.
            next.status = Status::Failed(failure);
        }
        AppEvent::Produced(_, Product::WorkingTree(Ok(counts))) => {
            next.working_tree = Some(counts);
        }
        AppEvent::Produced(_, Product::WorkingTree(Err(failure))) => {
            // The graph still draws without the uncommitted row; the reason
            // goes on the status line rather than leaving it silently
            // missing.
            next.status = Status::Failed(failure);
        }
        AppEvent::Progress(id, Progress::History(update)) => {
            walk_progress(&mut next, &mut commands, id, update);
        }
        AppEvent::Progress(id, Progress::Search(hits)) => {
            search_progress(&mut next, &mut commands, id, hits);
        }
    }

    (next, commands)
}

/// One update from history job `id`, taken in if it is the walk on screen.
fn walk_progress(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    update: HistoryUpdate,
) {
    let height = next.graph_rows();
    if next.awaiting == Some(id)
        && let Some(history) = next.history.as_mut()
    {
        // ADR-0008: a provisional order is sorted properly off-thread.
        if let HistoryUpdate::Started(facts) = &update
            && facts.is_provisional()
        {
            commands.push(Command::PromoteHistory);
        }
        history.apply(update, height);
        seek_landed(next, commands, id);
    }
    // Otherwise it is from a walk that was cancelled or superseded. Dropped
    // here, which is the whole reason the id travels with it: no chunk lands
    // after `Esc`.
}

/// A topological sort has started (ADR-0008). Only the newest is wanted: an
/// older one still running is superseded and cancelled.
fn sort_started(next: &mut AppState, commands: &mut Vec<Command>, id: JobId) {
    if let Some(older) = next.promoting.replace(id) {
        commands.push(Command::CancelJob(older));
    }
    if let Some(history) = next.history.as_mut() {
        history.promotion = Some(Promotion::Sorting);
    }
}

/// A topological sort has produced its `order`: handed to the walk on screen
/// if it is from the sort still wanted, dropped if it was cancelled or
/// superseded.
fn sorted(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    order: std::sync::Arc<[graft_core::oid::Oid]>,
) {
    if next.promoting != Some(id) {
        return;
    }
    next.promoting = None;
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut() {
        match (next.awaiting, history.promote(order, height)) {
            (Some(walk), Some(request)) => commands.push(Command::ShowRows(walk, request)),
            // The walk has gone; nobody would take the order up.
            _ => history.promotion = Some(Promotion::Kept),
        }
    }
}

/// The half of [`update`] that answers the events `core` defines.
fn core_event(next: &mut AppState, commands: &mut Vec<Command>, event: Event) {
    match event {
        // The search's keys go first: the open prompt takes every letter.
        // A handed-back key is checked next, before any graph binding, so
        // a harness can claim a key whatever the graph would do with it.
        Event::Key(key) => {
            if next.search_prompt.is_some() || !next.harness.hand_back.contains(&key.code) {
                if !search_key(next, commands, key) {
                    key_event(next, commands, key);
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
            if searching(next, id) =>
        {
            search_job_ended(next, event);
        }
        Event::JobFailed(id, failure) => {
            if next.awaiting == Some(id) {
                next.awaiting = None;
                set_phase(next, Phase::Failed(failure.to_string()));
                give_up_promotion(next, commands);
            }
            if next.promoting == Some(id) {
                give_up_promotion(next, commands);
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
                give_up_promotion(next, commands);
            }
            if next.awaiting == Some(id) {
                next.awaiting = None;
                set_phase(next, Phase::Complete);
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
                set_phase(next, Phase::Cancelled);
                give_up_promotion(next, commands);
            }
            if next.promoting == Some(id) {
                give_up_promotion(next, commands);
            }
        }
        Event::Mouse(mouse) => mouse_event(next, commands, mouse),
        // Nothing is bound to these yet. They reach `update` so that the
        // tickets that need them have somewhere obvious to add the handling.
        Event::Paste(_) | Event::FocusGained | Event::FocusLost => {}
    }
}

/// The half of [`core_event`] that answers a key.
fn key_event(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) {
    if is_quit(key) {
        next.quit = Some(Quit::Requested);
    } else if is_refresh(key) {
        // The walk on screen would otherwise sit waiting for row
        // requests that now go to its replacement.
        if let Some(id) = next.awaiting.take() {
            commands.push(Command::CancelJob(id));
        }
        if let Some(id) = next.promoting.take() {
            commands.push(Command::CancelJob(id));
        }
        commands.push(Command::OpenRepository);
        commands.push(Command::ReadHistory(Order::Topological));
        commands.push(Command::ReadRefs);
        commands.push(Command::ReadWorkingTree);
    } else if is_screen_toggle(key) {
        next.screen = next.screen.toggled();
    } else if is_lane_toggle(key) {
        next.lanes_expanded = !next.lanes_expanded;
    } else if is_focus_toggle(key) {
        next.focus = !next.focus;
    } else if is_tier_cycle(key) {
        cycle_tier(next, commands);
    } else if is_tooltip_key(key) {
        toggle_pinned_tooltip(next);
    } else if let Some(change) = view_change(key) {
        let height = next.graph_rows();
        let stash = next.refs.as_ref().and_then(crate::refs::RefBadges::stash);
        if let Some(history) = next.history.as_mut()
            && let Some(view) = change.apply(history, stash)
            && let Some(request) = history.change_view(view, height)
            && let Some(id) = next.awaiting
        {
            commands.push(Command::ShowRows(id, request));
        }
    } else if let Some(motion) = motion_for(key) {
        let height = next.graph_rows();
        if let Some(history) = next.history.as_mut()
            && let Some(request) = history.move_cursor(motion, height)
            && let Some(id) = next.awaiting
        {
            commands.push(Command::ShowRows(id, request));
        }
    } else if key.code == KeyCode::Escape {
        escape_key(next, commands);
    }
}

/// `Enter` pins the tooltip open (m1-the-graph/34): a second `Enter`
/// closes it, wherever the cursor has moved to since — `take()` reports
/// whether one was already open before deciding whether to open a new
/// one. Pinning is the keyboard's route to the same content a hover opens
/// (DESIGN.md §2.15): the selection's row, looked up the same way
/// `graph_row_at` looks up a hovered one.
fn toggle_pinned_tooltip(next: &mut AppState) {
    if next.pinned_tooltip.take().is_none() {
        next.pinned_tooltip = next.history.as_ref().map(|history| history.cursor);
    }
}

/// `Esc`: closes the tooltip if one is open — pinned first, since it is the
/// one that survives everything else — else backs a `G` still walking out
/// first, then leaves Focus mode, then cancels the walk — the lightest
/// thing on screen goes first (m1-the-graph/17, DESIGN.md §2.2).
fn escape_key(next: &mut AppState, commands: &mut Vec<Command>) {
    if next.pinned_tooltip.take().is_some() {
        return;
    }
    if next.tooltip.take().is_some() {
        return;
    }
    let height = next.graph_rows();
    // A search still running: stop it and put the selection back.
    if next
        .search
        .as_ref()
        .is_some_and(|search| matches!(search.state, SearchState::Running { .. }))
    {
        cancel_search(next, commands);
        return;
    }
    // A `G` still walking is what `Esc` backs out of first; the
    // walk itself goes on serving the rows already on screen.
    if let Some(id) = next.awaiting
        && let Some(history) = next.history.as_mut()
        && let Some(request) = history.withdraw_jump(height)
    {
        commands.push(Command::ShowRows(id, request));
        return;
    }
    // Then a finished search's line.
    if next.search.take().is_some() {
        return;
    }
    // Then Focus mode: leaving it is not a reason to stop the walk.
    if next.focus {
        next.focus = false;
        return;
    }
    // Then a topological sort still running: the rows stay in the order they
    // are in, and the screen says that order is provisional. Once the order
    // has been handed to the walk it is no longer the sort's to cancel.
    if next.promoting.is_some() {
        give_up_promotion(next, commands);
        return;
    }
    // Stop waiting first, then ask the job to stop. The order
    // matters: a job that finishes between the two lines still
    // sends its result, and it has to find nobody waiting for it.
    if let Some(id) = next.awaiting.take() {
        commands.push(Command::CancelJob(id));
        set_phase(next, Phase::Cancelled);
    }
}

/// The search's keys, answered before any other binding: everything but
/// `Ctrl-C` while the prompt is open, then `/`, `n` and `N`. `true` if the key
/// was the search's.
fn search_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let control_c = key.code == KeyCode::Char('c') && key.modifiers.control;
    if next.search_prompt.is_some() && !control_c {
        prompt_key(next, commands, key);
    } else if is_search_key(key) && next.screen == Screen::Graph && next.history.is_some() {
        next.search_prompt = Some(String::new());
    } else if let Some(forward) = search_step(key)
        && next.search.is_some()
    {
        step_search(next, commands, forward);
    } else {
        return false;
    }
    true
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
fn start_search(next: &mut AppState, commands: &mut Vec<Command>, query: crate::search::Query) {
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
fn search_started(next: &mut AppState, commands: &mut Vec<Command>, id: JobId) {
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
fn searching(next: &AppState, id: JobId) -> bool {
    next.search
        .as_ref()
        .is_some_and(|search| search.job == Some(id))
}

/// Hits from search job `id`: counted if it is the search on screen, dropped
/// if it was superseded or cancelled. The first hit is jumped to at once.
fn search_progress(
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
/// needs beyond what came back.
fn seek_landed(next: &mut AppState, commands: &mut Vec<Command>, walk: JobId) {
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
fn cancel_search(next: &mut AppState, commands: &mut Vec<Command>) {
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
fn search_job_ended(next: &mut AppState, event: Event) {
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
}

/// Stops promoting to topological order: cancels the sort if it is still
/// running, and leaves the order on screen marked provisional. A promotion
/// already swapping in is left alone unless the walk itself has ended — the
/// walk may have taken the order up, and only its answer can say.
fn give_up_promotion(next: &mut AppState, commands: &mut Vec<Command>) {
    if let Some(id) = next.promoting.take() {
        commands.push(Command::CancelJob(id));
    }
    let walk_gone = next.awaiting.is_none();
    if let Some(history) = next.history.as_mut() {
        match history.promotion {
            Some(Promotion::Sorting) => history.promotion = Some(Promotion::Kept),
            Some(Promotion::Swapping { .. }) if walk_gone => {
                history.promotion = Some(Promotion::Kept);
            }
            _ => {}
        }
    }
}

/// Moves the walk on screen to `phase`. Called only for the walk the screen
/// was waiting for, which always has a [`History`].
fn set_phase(next: &mut AppState, phase: Phase) {
    if let Some(history) = next.history.as_mut() {
        history.phase = phase;
    }
}

/// The movement a key asks for, if it is one (DESIGN.md §2.2): vim keys and
/// arrows, a screen at a time, and the ends.
fn motion_for(key: KeyPress) -> Option<Motion> {
    if key.modifiers.control || key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Motion::Down(1)),
        KeyCode::Char('k') | KeyCode::Up => Some(Motion::Up(1)),
        KeyCode::PageDown => Some(Motion::PageDown),
        KeyCode::PageUp => Some(Motion::PageUp),
        KeyCode::Char('g') | KeyCode::Home => Some(Motion::Top),
        KeyCode::Char('G') | KeyCode::End => Some(Motion::Bottom),
        _ => None,
    }
}

/// The two keys that mean "leave" (DESIGN.md §2.2).
///
/// `Ctrl+C` is here as well as `q` because a terminal application that ignores
/// it feels broken, and because raw mode means the terminal no longer turns it
/// into a signal on Graft's behalf.
fn is_quit(key: KeyPress) -> bool {
    let plain_q = key.code == KeyCode::Char('q') && !key.modifiers.control && !key.modifiers.alt;
    let control_c = key.code == KeyCode::Char('c') && key.modifiers.control;
    plain_q || control_c
}

/// `F2`: swap between the graph and the status screen. Not one of DESIGN.md
/// §2.2–§2.3's keys, which are all taken; the status screen is a diagnostic.
fn is_screen_toggle(key: KeyPress) -> bool {
    key.code == KeyCode::Function(2) && !key.modifiers.control && !key.modifiers.alt
}

/// `+` expands the lanes folded behind `+N`, and folds them again
/// (DESIGN.md §3.5).
/// `F`: Focus mode on the selected commit (DESIGN.md §2.3, §5.2).
fn is_focus_toggle(key: KeyPress) -> bool {
    key.code == KeyCode::Char('F') && !key.modifiers.control && !key.modifiers.alt
}

fn is_lane_toggle(key: KeyPress) -> bool {
    key.code == KeyCode::Char('+') && !key.modifiers.control && !key.modifiers.alt
}

/// `F3` pressed: the next tier down, and from ASCII back to the tier the
/// terminal settled on — never above it. The history's row requests follow.
fn cycle_tier(next: &mut AppState, commands: &mut Vec<Command>) {
    let top = next.settled_tier.unwrap_or(RenderingTier::Unicode);
    let lower = match next.tier() {
        RenderingTier::Pixel => Some(RenderingTier::Unicode),
        RenderingTier::Unicode => Some(RenderingTier::Ascii),
        RenderingTier::Ascii => None,
    };
    // Back at the top the override clears, so the footer has nothing extra
    // to say (`graft::view`). On an ASCII-only terminal that is every press.
    next.renderer_override = lower.filter(|&tier| tier != top);
    let tier = next.tier();
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut() {
        // Off pixels, requests stop padding to whole strips; back on, they
        // pad again.
        history.strip_rows = strip_rows_for(tier);
        if let Some(request) = history.show(height)
            && let Some(id) = next.awaiting
        {
            commands.push(Command::ShowRows(id, request));
        }
    }
}

/// Rows per history request at `tier`: whole pixel strips at the pixel tier
/// (`History::strip_rows`), exactly the viewport otherwise.
fn strip_rows_for(tier: RenderingTier) -> Option<usize> {
    (tier == RenderingTier::Pixel).then_some(graft_graph::raster::cache::STRIP_ROWS)
}

/// `F3`: cycle the renderer tier — pixels, Unicode, ASCII, then back to the
/// terminal's own tier — and say so (DESIGN.md §4.2 — config wins over
/// detection, and a key press wins the same way over both; tickets
/// m1-the-graph/19, 24d, 33). Not one of DESIGN.md's keymap, like `F2`.
fn is_tier_cycle(key: KeyPress) -> bool {
    key.code == KeyCode::Function(3) && !key.modifiers.control && !key.modifiers.alt
}

/// `Enter`: open the tooltip for the selection — the keyboard's route to the
/// content a hover shows (DESIGN.md §2.15, PROMPT.md §1.3). Free on the graph
/// pane: DESIGN.md §2.3 binds it only on a sidebar row, which m1 has none of.
fn is_tooltip_key(key: KeyPress) -> bool {
    key.code == KeyCode::Enter && key.modifiers == Modifiers::NONE
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
                select_row(next, commands, row);
            }
        }
        // Any-motion reporting (`terminal.rs`) is what makes this arrive with
        // no button held — hover, not a drag.
        MouseKind::Moved => next.tooltip = graph_row_at(next, mouse.row),
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
/// `None` over the header, the footer, the uncommitted-changes row (nothing
/// selectable there — `d` already reaches it directly), or a row below what
/// has laid out so far.
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

/// A key that changes which history is shown (PROMPT.md §1.3). DESIGN.md
/// §2.3 assigns none of these; they are recorded there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewChange {
    /// `Space` on a merge: fold its merged branch into it, or unfold it.
    Fold,
    /// `1`: the first-parent view, or back to every parent.
    FirstParent,
    /// `H`: hide the stash, or show it again.
    Stashes,
}

impl ViewChange {
    /// The view after this change, or `None` where it does nothing: `Space`
    /// off a merge, `H` with no stash.
    fn apply(self, history: &History, stash: Option<Oid>) -> Option<View> {
        let mut view = history.view.clone();
        match self {
            ViewChange::Fold => {
                let commit = &history.selected()?.commit;
                // A folded merge is drawn with one parent, so it is known
                // by the fold, not by its parents.
                if !view.inclusion.folded.remove(&commit.id) {
                    if commit.parents.len() < 2 {
                        return None;
                    }
                    view.inclusion.folded.insert(commit.id);
                }
            }
            ViewChange::FirstParent => {
                view.follow = match view.follow {
                    Follow::AllParents => Follow::FirstParent,
                    Follow::FirstParent => Follow::AllParents,
                };
            }
            ViewChange::Stashes => {
                if view.inclusion.hidden_stashes.is_empty() {
                    view.inclusion.hidden_stashes.insert(stash?);
                } else {
                    view.inclusion.hidden_stashes.clear();
                }
            }
        }
        Some(view)
    }
}

/// The view change a key asks for, if it is one.
fn view_change(key: KeyPress) -> Option<ViewChange> {
    if key.modifiers.control || key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char(' ') => Some(ViewChange::Fold),
        KeyCode::Char('1') => Some(ViewChange::FirstParent),
        KeyCode::Char('H') => Some(ViewChange::Stashes),
        _ => None,
    }
}

/// Manual refresh (DESIGN.md §2.1): re-read the repository now, rather than
/// waiting for whatever would have noticed on its own.
fn is_refresh(key: KeyPress) -> bool {
    key.code == KeyCode::Char('r') && key.modifiers.control
}

#[cfg(test)]
mod tests {
    use graft_core::event::Modifiers;
    use graft_core::job::{JobFailure, JobId};

    use graft_core::commit::{Commit, Identity, Timestamp};
    use graft_core::oid::Oid;
    use graft_graph::Row;

    use super::*;
    use std::ops::Range;

    use crate::history::{WalkFacts, Wanted, Window};
    use crate::repository::Head;
    use graft_ui::graph_screen::OrderNotice;

    fn identity() -> Identity {
        Identity {
            name: b"Test".to_vec(),
            email: b"test@example.invalid".to_vec(),
            when: Timestamp {
                seconds: 0,
                offset_seconds: 0,
            },
        }
    }

    fn key(code: KeyCode) -> AppEvent {
        AppEvent::Core(Event::Key(KeyPress::plain(code)))
    }

    #[test]
    fn graft_opens_on_the_graph_and_f2_swaps_to_the_status_screen_and_back() {
        let state = AppState::default();
        assert_eq!(state.screen, Screen::Graph);

        let (status, commands) = update(&state, key(KeyCode::Function(2)));
        assert_eq!(status.screen, Screen::Status);
        assert!(commands.is_empty(), "{commands:?}");

        let (graph, _) = update(&status, key(KeyCode::Function(2)));
        assert_eq!(graph.screen, Screen::Graph);
    }

    /// `F3` from `settled` and on, `presses` times: the override after each.
    fn f3_presses(settled: Option<RenderingTier>, presses: usize) -> Vec<Option<RenderingTier>> {
        let mut state = AppState {
            settled_tier: settled,
            ..AppState::default()
        };
        (0..presses)
            .map(|_| {
                let (next, commands) = update(&state, key(KeyCode::Function(3)));
                assert!(commands.is_empty(), "a view-time flag: {commands:?}");
                state = next;
                state.renderer_override
            })
            .collect()
    }

    #[test]
    fn f3_on_a_unicode_terminal_toggles_unicode_and_ascii() {
        // No session (`settled_tier` None) counts as Unicode, the same way.
        for settled in [Some(RenderingTier::Unicode), None] {
            assert_eq!(
                f3_presses(settled, 3),
                vec![Some(RenderingTier::Ascii), None, Some(RenderingTier::Ascii)],
                "back at the top the override clears: nothing extra to say"
            );
        }
    }

    #[test]
    fn f3_on_an_ascii_only_terminal_does_nothing() {
        assert_eq!(f3_presses(Some(RenderingTier::Ascii), 2), vec![None, None]);
    }

    #[test]
    fn at_the_pixel_tier_history_asks_for_whole_strips() {
        let state = AppState {
            settled_tier: Some(RenderingTier::Pixel),
            ..AppState::default()
        };
        let (started, _) = update(&state, AppEvent::Started(JobId::new(1), Task::ReadHistory));
        let history = started.history.expect("a walk started");
        assert_eq!(
            history.strip_rows,
            Some(graft_graph::raster::cache::STRIP_ROWS)
        );

        let (unicode, _) = update(
            &AppState::default(),
            AppEvent::Started(JobId::new(1), Task::ReadHistory),
        );
        assert_eq!(unicode.history.expect("a walk started").strip_rows, None);
    }

    #[test]
    fn f3_cycles_pixels_unicode_ascii_and_back_to_whole_strips() {
        let state = AppState {
            settled_tier: Some(RenderingTier::Pixel),
            ..AppState::default()
        };
        let (started, _) = update(&state, AppEvent::Started(JobId::new(1), Task::ReadHistory));
        let strip_rows = |state: &AppState| state.history.as_ref().and_then(|h| h.strip_rows);

        let (unicode, _) = update(&started, key(KeyCode::Function(3)));
        assert_eq!(unicode.tier(), RenderingTier::Unicode);
        assert_eq!(strip_rows(&unicode), None);

        let (ascii, _) = update(&unicode, key(KeyCode::Function(3)));
        assert_eq!(ascii.tier(), RenderingTier::Ascii);
        assert_eq!(strip_rows(&ascii), None);

        let (pixel, _) = update(&ascii, key(KeyCode::Function(3)));
        assert_eq!(pixel.renderer_override, None, "the settled tier again");
        assert_eq!(pixel.tier(), RenderingTier::Pixel);
        assert_eq!(
            strip_rows(&pixel),
            Some(graft_graph::raster::cache::STRIP_ROWS),
            "requests pad to whole strips again"
        );
    }

    /// A walk in flight whose window holds a plain commit (row 0) and a
    /// merge (row 1), the cursor on row `cursor`.
    fn merge_on_screen(id: JobId, cursor: usize) -> (AppState, Oid) {
        let commit = |byte: u8, parents: Vec<Oid>| Row {
            commit: Commit {
                id: Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid"),
                parents,
                author: identity(),
                committer: identity(),
                message: b"a commit".to_vec(),
            },
            lane: 0,
            incoming: Vec::new(),
            outgoing: Vec::new(),
            lanes: Vec::new(),
            marker: None,
        };
        let parent = |byte| Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("an oid");
        let plain = commit(1, vec![parent(2)]);
        let merge = commit(2, vec![parent(3), parent(4)]);
        let merge_id = merge.commit.id;
        let window = Window {
            request: 1,
            first: 0,
            rows: vec![plain, merge].into(),
            laid_out: 2,
            done: false,
            promoted: false,
        };
        let (mut state, _) = update(
            &awaiting(id),
            AppEvent::Progress(id, Progress::History(HistoryUpdate::Window(window))),
        );
        state.history.as_mut().expect("a walk").cursor = cursor;
        (state, merge_id)
    }

    /// The view the last `ShowRows` asks for.
    fn asked_view(commands: &[Command]) -> View {
        match commands {
            [Command::ShowRows(_, request)] => request.view.clone(),
            other => panic!("expected one row request, got {other:?}"),
        }
    }

    #[test]
    fn space_folds_the_merge_under_the_cursor_and_unfolds_it() {
        let id = JobId::new(5);
        let (state, merge) = merge_on_screen(id, 1);

        let (folded, commands) = update(&state, key(KeyCode::Char(' ')));
        let view = asked_view(&commands);
        assert_eq!(view.inclusion.folded, [merge].into());
        assert_eq!(folded.history.as_ref().map(|h| &h.view), Some(&view));
        assert_eq!(
            folded.history.as_ref().map(|h| h.cursor),
            Some(1),
            "stays put"
        );

        let (unfolded, commands) = update(&folded, key(KeyCode::Char(' ')));
        assert_eq!(asked_view(&commands), View::default());
        assert_eq!(unfolded.history.map(|h| h.view), Some(View::default()));
    }

    #[test]
    fn space_off_a_merge_changes_nothing() {
        let id = JobId::new(5);
        let (state, _) = merge_on_screen(id, 0);
        let (next, commands) = update(&state, key(KeyCode::Char(' ')));
        assert!(commands.is_empty(), "{commands:?}");
        assert_eq!(next.history, state.history);
    }

    #[test]
    fn one_switches_to_first_parent_view_from_the_top_and_back() {
        let id = JobId::new(6);
        let (state, _) = merge_on_screen(id, 1);

        let (straight, commands) = update(&state, key(KeyCode::Char('1')));
        assert_eq!(asked_view(&commands).follow, Follow::FirstParent);
        let history = straight.history.as_ref().expect("a walk");
        assert_eq!((history.top, history.cursor), (0, 0));

        let (full, commands) = update(&straight, key(KeyCode::Char('1')));
        assert_eq!(asked_view(&commands), View::default());
        assert_eq!(full.history.map(|h| h.view), Some(View::default()));
    }

    #[test]
    fn capital_h_hides_the_stash_and_shows_it_again() {
        let id = JobId::new(7);
        let (mut state, _) = merge_on_screen(id, 0);
        let (none, commands) = update(&state, key(KeyCode::Char('H')));
        assert!(
            commands.is_empty(),
            "no stash, nothing to hide: {commands:?}"
        );
        assert_eq!(none.history, state.history);

        let stash = Oid::from_bytes(&[9; Oid::SHA1_BYTES]).expect("an oid");
        let name = graft_core::ref_name::RefName::new("refs/stash").expect("valid");
        let badges = crate::refs::badges(
            &[(name, stash)],
            &graft_git::repo::HeadState::Detached(stash),
            &std::collections::HashMap::new(),
            |_, _| Err("no upstreams here".to_owned()),
        )
        .expect("badges");
        assert_eq!(badges.stash(), Some(stash));
        state.refs = Some(badges);

        let (hidden, commands) = update(&state, key(KeyCode::Char('H')));
        assert_eq!(
            asked_view(&commands).inclusion.hidden_stashes,
            [stash].into()
        );
        let (shown, commands) = update(&hidden, key(KeyCode::Char('H')));
        assert_eq!(asked_view(&commands), View::default());
        assert_eq!(shown.history.map(|h| h.view), Some(View::default()));
    }

    #[test]
    fn plus_expands_and_collapses_the_lanes_without_touching_the_walk() {
        let id = JobId::new(3);
        let (state, _) = update(&awaiting(id), rows(id, 4));
        let before = state.history.clone();

        let (expanded, commands) = update(&state, key(KeyCode::Char('+')));
        assert!(expanded.lanes_expanded);
        assert!(
            commands.is_empty(),
            "expanding is a view decision: {commands:?}"
        );
        assert_eq!(expanded.history, before, "the rows are not laid out again");

        let (folded, commands) = update(&expanded, key(KeyCode::Char('+')));
        assert!(!folded.lanes_expanded);
        assert!(commands.is_empty(), "{commands:?}");
        assert_eq!(folded.history, before);
    }

    #[test]
    fn capital_f_toggles_focus_and_esc_leaves_it_without_touching_the_walk() {
        let id = JobId::new(3);
        let (state, _) = update(&awaiting(id), rows(id, 4));
        let before = state.history.clone();

        let (focused, commands) = update(&state, key(KeyCode::Char('F')));
        assert!(focused.focus);
        assert!(commands.is_empty(), "focus is paint: {commands:?}");
        assert_eq!(focused.history, before);

        let (left, commands) = update(&focused, key(KeyCode::Escape));
        assert!(!left.focus);
        assert!(commands.is_empty(), "Esc leaves focus first: {commands:?}");
        assert_eq!(left.awaiting, Some(id), "the walk goes on");
        assert_eq!(left.history, before);
    }

    #[test]
    fn a_click_selects_the_row_under_the_pointer() {
        let id = JobId::new(20);
        let (mut state, merge_id) = merge_on_screen(id, 0);
        state.size = Size {
            columns: 80,
            rows: 5,
        };

        let click = core(Event::Mouse(MouseInput {
            kind: MouseKind::Down(MouseButton::Left),
            column: 10,
            row: HEADER_ROWS + 1,
            modifiers: Modifiers::NONE,
        }));
        let (next, _) = update(&state, click);

        assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(1));
        assert_eq!(
            next.history
                .as_ref()
                .and_then(History::selected)
                .map(|row| row.commit.id),
            Some(merge_id)
        );
    }

    #[test]
    fn a_click_under_a_harness_banner_lands_on_the_row_the_banner_pushed_down() {
        let id = JobId::new(20);
        let (mut state, merge_id) = merge_on_screen(id, 0);
        state.size = Size {
            columns: 80,
            rows: 7,
        };
        state.harness.banner = Some(Banner {
            title: String::from("gallery 3/23"),
            lines: vec![String::from("shows: a merge")],
        });
        assert_eq!(state.graph_rows(), 3, "header, two banner rows, footer");

        // The screen row a click on the header would have hit without the
        // banner is now the banner's title: nothing selected.
        let on_banner = core(Event::Mouse(MouseInput {
            kind: MouseKind::Down(MouseButton::Left),
            column: 10,
            row: HEADER_ROWS,
            modifiers: Modifiers::NONE,
        }));
        let (next, _) = update(&state, on_banner);
        assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(0));

        let click = core(Event::Mouse(MouseInput {
            kind: MouseKind::Down(MouseButton::Left),
            column: 10,
            row: HEADER_ROWS + 2 + 1,
            modifiers: Modifiers::NONE,
        }));
        let (next, _) = update(&state, click);
        assert_eq!(
            next.history
                .as_ref()
                .and_then(History::selected)
                .map(|row| row.commit.id),
            Some(merge_id)
        );
    }

    #[test]
    fn the_wheel_scrolls_the_viewport_and_leaves_the_selection_alone() {
        let id = JobId::new(21);
        let (mut state, _) = merge_on_screen(id, 0);
        state.size = Size {
            columns: 80,
            rows: 5,
        };

        let scroll_down = core(Event::Mouse(MouseInput {
            kind: MouseKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: Modifiers::NONE,
        }));
        let (down, _) = update(&state, scroll_down);
        assert_eq!(down.history.as_ref().map(|history| history.top), Some(1));
        assert_eq!(down.history.as_ref().map(|history| history.cursor), Some(0));

        let scroll_up = core(Event::Mouse(MouseInput {
            kind: MouseKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: Modifiers::NONE,
        }));
        let (up, _) = update(&down, scroll_up);
        assert_eq!(up.history.as_ref().map(|history| history.top), Some(0));
    }

    #[test]
    fn hover_and_enter_open_the_tooltip_on_the_same_row() {
        let id = JobId::new(22);
        let (mut state, _) = merge_on_screen(id, 1);
        state.size = Size {
            columns: 80,
            rows: 5,
        };

        let hover = core(Event::Mouse(MouseInput {
            kind: MouseKind::Moved,
            column: 5,
            row: HEADER_ROWS + 1,
            modifiers: Modifiers::NONE,
        }));
        let (hovered, _) = update(&state, hover);
        assert_eq!(hovered.tooltip, Some(1), "hover names the row under it");

        let (opened, _) = update(&state, key(KeyCode::Enter));
        assert_eq!(
            opened.pinned_tooltip, hovered.tooltip,
            "the keyboard names the same row as the hover"
        );
    }

    #[test]
    fn hovering_another_row_does_not_move_or_close_a_pinned_tooltip() {
        let id = JobId::new(23);
        let (state, _) = merge_on_screen(id, 1);
        let (opened, _) = update(&state, key(KeyCode::Enter));
        assert_eq!(opened.pinned_tooltip, Some(1));

        let hover = core(Event::Mouse(MouseInput {
            kind: MouseKind::Moved,
            column: 5,
            row: HEADER_ROWS,
            modifiers: Modifiers::NONE,
        }));
        let (hovered, _) = update(&opened, hover);
        assert_eq!(
            hovered.pinned_tooltip,
            Some(1),
            "hover cannot overwrite the pin"
        );
        assert_ne!(
            hovered.tooltip, hovered.pinned_tooltip,
            "hover still tracks the pointer underneath it"
        );
    }

    #[test]
    fn moving_the_cursor_does_not_move_a_pinned_tooltip() {
        let id = JobId::new(23);
        let (state, _) = merge_on_screen(id, 1);
        let (opened, _) = update(&state, key(KeyCode::Enter));
        assert_eq!(opened.pinned_tooltip, Some(1));

        let (moved, _) = update(&opened, key(KeyCode::Char('j')));
        assert_eq!(
            moved.pinned_tooltip,
            Some(1),
            "the pin stays on the commit it was opened for"
        );
    }

    #[test]
    fn enter_again_closes_the_pin() {
        let id = JobId::new(23);
        let (state, _) = merge_on_screen(id, 1);
        let (opened, _) = update(&state, key(KeyCode::Enter));
        assert_eq!(opened.pinned_tooltip, Some(1));

        let (closed, _) = update(&opened, key(KeyCode::Enter));
        assert_eq!(closed.pinned_tooltip, None);
    }

    #[test]
    fn esc_closes_the_tooltip_before_anything_else() {
        let id = JobId::new(23);
        let (state, _) = merge_on_screen(id, 1);
        let (opened, _) = update(&state, key(KeyCode::Enter));
        assert_eq!(opened.pinned_tooltip, Some(1));

        let (closed, commands) = update(&opened, key(KeyCode::Escape));
        assert_eq!(closed.pinned_tooltip, None);
        assert!(
            commands.is_empty(),
            "closing the tooltip asks nothing of the walk"
        );
        assert_eq!(
            closed.awaiting, opened.awaiting,
            "the walk itself is untouched"
        );
    }

    /// Moving the selection inside the rows on hand asks the history job for
    /// nothing: the thread and its fade are worked out from those rows, so a
    /// selection change makes no layout or walk call.
    #[test]
    fn a_selection_change_inside_the_window_asks_for_no_rows() {
        let id = JobId::new(3);
        let (mut state, _) = update(&awaiting(id), rows_for(id, 1, 0..30, 500));
        state.focus = true;
        for code in [KeyCode::Char('j'), KeyCode::Char('j'), KeyCode::Char('k')] {
            let (moved, commands) = update(&state, key(code));
            assert!(commands.is_empty(), "{code:?}: {commands:?}");
            state = moved;
        }
        assert_eq!(state.history.as_ref().map(|h| h.cursor), Some(1));
    }

    fn core(event: Event) -> AppEvent {
        AppEvent::Core(event)
    }

    fn facts(path: &str, branch: &str) -> RepositoryFacts {
        RepositoryFacts {
            path: PathBuf::from(path),
            bare: false,
            head: Head::Branch(branch.as_bytes().to_vec()),
            mailmap: graft_git::mailmap::Mailmap::default(),
        }
    }

    #[test]
    fn q_quits() {
        let (next, commands) = update(&AppState::default(), key(KeyCode::Char('q')));

        assert_eq!(next.quit, Some(Quit::Requested));
        assert!(commands.is_empty());
    }

    #[test]
    fn control_c_quits() {
        let event = core(Event::Key(KeyPress::control(KeyCode::Char('c'))));

        let (next, _) = update(&AppState::default(), event);

        assert_eq!(next.quit, Some(Quit::Requested));
    }

    #[test]
    fn control_q_is_not_quit() {
        // Only the bare `q` is bound. A modified one belongs to whatever
        // binding claims it later, and must not fall through to quitting.
        let event = core(Event::Key(KeyPress::control(KeyCode::Char('q'))));

        let (next, _) = update(&AppState::default(), event);

        assert_eq!(next.quit, None);
    }

    #[test]
    fn a_bare_c_is_not_quit() {
        let (next, _) = update(&AppState::default(), key(KeyCode::Char('c')));

        assert_eq!(next.quit, None);
    }

    #[test]
    fn unbound_events_change_nothing() {
        let before = AppState::default();
        let unbound = [
            key(KeyCode::Char('z')),
            key(KeyCode::Function(7)),
            core(Event::Paste("hello".to_owned())),
            core(Event::FocusGained),
            core(Event::FocusLost),
            core(Event::JobCancelled(JobId::new(1))),
        ];

        for event in unbound {
            let (next, commands) = update(&before, event.clone());

            assert_eq!(next, before, "{event:?} changed the state");
            assert!(commands.is_empty(), "{event:?} asked for work");
        }
    }

    #[test]
    fn resize_is_remembered_including_a_degenerate_one() {
        let (next, _) = update(
            &AppState::default(),
            core(Event::Resize {
                columns: 0,
                rows: 0,
            }),
        );

        assert_eq!(
            next.size,
            Size {
                columns: 0,
                rows: 0
            }
        );

        let (next, _) = update(
            &next,
            core(Event::Resize {
                columns: 120,
                rows: 40,
            }),
        );

        assert_eq!(
            next.size,
            Size {
                columns: 120,
                rows: 40
            }
        );
    }

    #[test]
    fn a_failed_job_reaches_the_status_line() {
        let failure = JobFailure::Failed("not a git repository".to_owned());

        let (next, commands) = update(
            &AppState::default(),
            core(Event::JobFailed(JobId::new(9), failure)),
        );

        assert_eq!(
            next.status,
            Status::Failed("job 9 failed: not a git repository".to_owned())
        );
        assert!(commands.is_empty());
    }

    #[test]
    fn a_panicking_job_says_so_and_names_its_report() {
        let failure = JobFailure::Panicked {
            message: "lane 3 vanished".to_owned(),
            report: Some("/state/graft/crash-1.log".to_owned()),
        };

        let (next, _) = update(
            &AppState::default(),
            core(Event::JobFailed(JobId::new(2), failure)),
        );

        assert_eq!(
            next.status,
            Status::Failed(
                "job 2 failed: panicked: lane 3 vanished — report: /state/graft/crash-1.log"
                    .to_owned()
            )
        );
    }

    #[test]
    fn a_job_that_worked_clears_a_stale_failure() {
        let failed = AppState {
            status: Status::Failed("job 1 failed: not a git repository".to_owned()),
            ..AppState::default()
        };

        let (next, _) = update(&failed, core(Event::JobFinished(JobId::new(2))));

        assert_eq!(next.status, Status::Ready);
    }

    #[test]
    fn a_cancelled_job_leaves_the_status_line_alone() {
        // Cancellation is something Graft asked for. Reporting it would put an
        // error in front of the user when nothing is wrong (CLAUDE.md).
        let failed = AppState {
            status: Status::Failed("job 1 failed: not a git repository".to_owned()),
            ..AppState::default()
        };

        let (next, _) = update(&failed, core(Event::JobCancelled(JobId::new(1))));

        assert_eq!(next, failed);
    }

    #[test]
    fn control_r_asks_for_the_repository_to_be_re_read() {
        let state = AppState {
            repo_path: PathBuf::from("/code/api"),
            ..AppState::default()
        };

        let (next, commands) = update(
            &state,
            core(Event::Key(KeyPress::control(KeyCode::Char('r')))),
        );

        assert_eq!(
            commands,
            vec![
                Command::OpenRepository,
                Command::ReadHistory(Order::Topological),
                Command::ReadRefs,
                Command::ReadWorkingTree,
            ]
        );
        // Asking for work is all it does: the state is the job's to change,
        // once the job comes back.
        assert_eq!(next, state);
    }

    #[test]
    fn a_bare_r_asks_for_nothing() {
        // `r` is rebase on the history screen (DESIGN.md §2.3), not refresh.
        let (_, commands) = update(&AppState::default(), key(KeyCode::Char('r')));

        assert!(commands.is_empty());
    }

    #[test]
    fn the_same_event_always_gives_the_same_answer() {
        // Purity's observable half: no clock, no counter, no hidden state, so
        // repeating a call repeats the answer.
        let state = AppState {
            quit: None,
            harness: Harness::default(),
            status: Status::Failed("earlier".to_owned()),
            size: Size {
                columns: 80,
                rows: 24,
            },
            repo_path: PathBuf::from("/code/api"),
            repository: Some(facts("/code/api", "main")),
            system_git: None,
            awaiting: None,
            promoting: None,
            history: None,
            refs: None,
            working_tree: None,
            screen: Screen::Graph,
            lanes_expanded: false,
            focus: false,
            tooltip: None,
            pinned_tooltip: None,
            renderer_override: None,
            settled_tier: None,
            search_prompt: None,
            search: None,
        };

        let first = update(&state, key(KeyCode::Char('q')));
        let second = update(&state, key(KeyCode::Char('q')));

        assert_eq!(first, second);
        assert_eq!(state.quit, None, "the input state was mutated");
    }

    #[test]
    fn an_opened_repository_becomes_what_the_screen_shows() {
        let (next, commands) = update(
            &AppState::default(),
            AppEvent::Produced(
                JobId::new(1),
                Product::Repository(Ok(facts("/code/api", "main"))),
            ),
        );

        assert_eq!(next.repository, Some(facts("/code/api", "main")));
        assert_eq!(next.status, Status::Ready);
        assert!(commands.is_empty());
    }

    #[test]
    fn a_repository_that_opened_clears_an_earlier_failure() {
        let failed = AppState {
            status: Status::Failed("could not re-read the repository".to_owned()),
            repository: Some(facts("/code/api", "main")),
            ..AppState::default()
        };

        let (next, _) = update(
            &failed,
            AppEvent::Produced(
                JobId::new(2),
                Product::Repository(Ok(facts("/code/api", "release"))),
            ),
        );

        assert_eq!(next.repository, Some(facts("/code/api", "release")));
        assert_eq!(next.status, Status::Ready);
    }

    #[test]
    fn a_path_that_is_not_a_repository_ends_the_run() {
        // Nothing was ever open, so there is nothing to show. Graft leaves and
        // says why, on a terminal it has handed back.
        let (next, commands) = update(
            &AppState::default(),
            AppEvent::Produced(
                JobId::new(1),
                Product::Repository(Err("/tmp is not inside a git repository".to_owned())),
            ),
        );

        assert_eq!(
            next.quit,
            Some(Quit::Fatal(
                "/tmp is not inside a git repository".to_owned()
            ))
        );
        assert!(commands.is_empty());
    }

    #[test]
    fn a_failed_refresh_of_an_open_repository_only_reaches_the_status_line() {
        let open = AppState {
            repository: Some(facts("/code/api", "main")),
            ..AppState::default()
        };

        let (next, _) = update(
            &open,
            AppEvent::Produced(
                JobId::new(3),
                Product::Repository(Err("the repository went away".to_owned())),
            ),
        );

        assert_eq!(next.quit, None, "a refresh must not end the run");
        assert_eq!(
            next.status,
            Status::Failed("the repository went away".to_owned())
        );
        // The repository already read stays on screen: stale beats blank.
        assert_eq!(next.repository, Some(facts("/code/api", "main")));
    }

    #[test]
    fn what_the_system_git_is_is_remembered() {
        let (next, commands) = update(
            &AppState::default(),
            AppEvent::Produced(
                JobId::new(4),
                Product::SystemGit(GitCheck::Unavailable("no git on PATH".to_owned())),
            ),
        );

        assert_eq!(
            next.system_git,
            Some(GitCheck::Unavailable("no git on PATH".to_owned()))
        );
        assert!(commands.is_empty());
    }

    /// What a walk says first: the order it is in.
    fn started(requested: Order, order: Order) -> AppEvent {
        AppEvent::Progress(
            JobId::new(0),
            Progress::History(HistoryUpdate::Started(Box::new(WalkFacts {
                requested,
                order,
                default_branch: None,
                integration: Vec::new().into(),
            }))),
        )
    }

    /// The same event, from job `id`.
    fn from(id: JobId, event: AppEvent) -> AppEvent {
        match event {
            AppEvent::Progress(_, progress) => AppEvent::Progress(id, progress),
            other => other,
        }
    }

    /// Job `id`'s answer to the first request, with `n` rows laid out. The
    /// rows themselves do not matter here, only whether they land, so they are
    /// copies of one row.
    fn rows(id: JobId, n: usize) -> AppEvent {
        let row = Row {
            commit: Commit {
                id: Oid::from_bytes(&[7; Oid::SHA1_BYTES]).expect("20 bytes is an oid"),
                parents: Vec::new(),
                author: identity(),
                committer: identity(),
                message: b"a commit".to_vec(),
            },
            lane: 0,
            incoming: Vec::new(),
            outgoing: Vec::new(),
            lanes: Vec::new(),
            marker: None,
        };
        AppEvent::Progress(
            id,
            Progress::History(HistoryUpdate::Window(Window {
                request: 1,
                first: 0,
                rows: vec![row; n.min(3)].into(),
                laid_out: n,
                done: false,
                promoted: false,
            })),
        )
    }

    /// The state of an application with a walk in flight.
    fn awaiting(id: JobId) -> AppState {
        let (next, commands) = update(
            &AppState::default(),
            AppEvent::Started(id, Task::ReadHistory),
        );

        // A default state is zero rows tall, which shows one row: the first
        // screen, and one more screen below it.
        assert_eq!(
            commands,
            vec![Command::ShowRows(id, request(1, Wanted::Rows(0..4)))],
            "a walk that has started is asked for its first screen"
        );
        assert_eq!(next.awaiting, Some(id));
        assert_eq!(next.history.as_ref().map(|history| history.asked), Some(1));
        next
    }

    /// Where the walk on screen is, and how many rows it has.
    fn progress(state: &AppState) -> (Phase, usize) {
        let history = state.history.as_ref().expect("a walk was asked for");
        (history.phase.clone(), history.rows)
    }

    #[test]
    fn rows_land_as_they_arrive_and_the_walk_completes() {
        let id = JobId::new(4);
        let state = awaiting(id);

        let (state, _) = update(&state, rows(id, 500));
        assert_eq!(progress(&state), (Phase::Streaming, 500));
        let (state, _) = update(&state, rows(id, 512));
        assert_eq!(progress(&state), (Phase::Streaming, 512));

        let (state, commands) = update(&state, core(Event::JobFinished(id)));
        assert_eq!(progress(&state), (Phase::Complete, 512));
        assert_eq!(state.awaiting, None);
        assert!(commands.is_empty());
    }

    fn request(id: u64, wanted: Wanted) -> RowRequest {
        RowRequest {
            id,
            wanted,
            view: View::default(),
            promote: None,
        }
    }

    fn sized(columns: u16, rows: u16) -> AppEvent {
        core(Event::Resize { columns, rows })
    }

    /// Moving past the rows held asks the walk for the rows there, and
    /// only then: moving within them asks for nothing.
    #[test]
    fn moving_past_the_window_asks_for_the_rows_there() {
        let id = JobId::new(4);
        let (state, _) = update(&awaiting(id), sized(80, 12));
        // Ten graph rows: the resize asked for them, and they arrive.
        let (state, _) = update(&state, rows_for(id, 2, 0..30, 500));

        let (state, commands) = update(&state, key(KeyCode::Char('j')));
        assert!(commands.is_empty(), "row 1 is held: {commands:?}");
        assert_eq!(state.history.as_ref().map(|h| h.cursor), Some(1));

        // Page 1 (10..20) is drawn from the rows held, but the second page
        // after it is not held: ask now, three screens either side, so
        // neither waits.
        let (_, commands) = update(&state, key(KeyCode::PageDown));
        assert_eq!(
            commands,
            vec![Command::ShowRows(id, request(3, Wanted::Rows(0..50)))]
        );
    }

    /// `G` on a walk not laid out to its end asks for the last rows and leaves
    /// the viewport where it is; `Esc` then withdraws the jump — not the walk.
    #[test]
    fn esc_withdraws_a_jump_before_it_cancels_the_walk() {
        let id = JobId::new(4);
        let (state, _) = update(&awaiting(id), sized(80, 12));
        let (state, _) = update(&state, rows_for(id, 2, 0..30, 500));

        let (jumping, commands) = update(&state, key(KeyCode::Char('G')));
        assert_eq!(
            commands,
            vec![Command::ShowRows(id, request(3, Wanted::Last(20)))]
        );
        let history = jumping.history.as_ref().expect("a walk");
        assert_eq!((history.top, history.cursor, history.jump), (0, 0, Some(3)));

        let (withdrawn, commands) = update(&jumping, key(KeyCode::Escape));
        assert_eq!(
            commands,
            vec![Command::ShowRows(id, request(4, Wanted::Rows(0..40)))]
        );
        assert_eq!(withdrawn.awaiting, Some(id), "the walk goes on");
        assert_eq!(withdrawn.history.as_ref().and_then(|h| h.jump), None);

        let (after, _) = update(&withdrawn, rows_for(id, 3, 480..500, 500));
        assert_eq!(after, withdrawn, "the jump's answer does not land");
    }

    /// Job `id`'s answer to request `request`: `span` rows, `laid_out` so far.
    fn rows_for(id: JobId, request: u64, span: Range<usize>, laid_out: usize) -> AppEvent {
        let AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(window))) =
            rows(id, span.len())
        else {
            unreachable!("`rows` answers with a window");
        };
        let one = window.rows[0].clone();
        AppEvent::Progress(
            id,
            Progress::History(HistoryUpdate::Window(Window {
                request,
                first: span.start,
                rows: vec![one; span.len()].into(),
                laid_out,
                done: false,
                promoted: false,
            })),
        )
    }

    #[test]
    fn esc_cancels_the_walk_the_screen_is_waiting_for() {
        let id = JobId::new(4);
        let (state, _) = update(&awaiting(id), rows(id, 500));

        let (next, commands) = update(&state, key(KeyCode::Escape));

        assert_eq!(commands, vec![Command::CancelJob(id)]);
        assert_eq!(
            next.awaiting, None,
            "nothing is waited on once `Esc` has been pressed"
        );
        assert_eq!(progress(&next), (Phase::Cancelled, 500));
    }

    #[test]
    fn esc_with_no_walk_running_does_nothing() {
        let before = AppState::default();

        let (next, commands) = update(&before, key(KeyCode::Escape));

        assert_eq!(next, before);
        assert!(commands.is_empty());
    }

    /// The race `Esc` has to survive: the walk lays out another chunk between
    /// the key press and the cancel flag being read, and sends it anyway. It
    /// must not appear, and neither may the walk's ending undo the cancel.
    #[test]
    fn no_chunk_lands_after_esc() {
        let id = JobId::new(4);
        let (state, _) = update(&awaiting(id), rows(id, 500));
        let (cancelled, _) = update(&state, key(KeyCode::Escape));

        let (next, commands) = update(&cancelled, rows(id, 500));
        assert_eq!(progress(&next), (Phase::Cancelled, 500));
        assert!(commands.is_empty());

        for ending in [Event::JobFinished(id), Event::JobCancelled(id)] {
            let (after, _) = update(&next, core(ending));
            assert_eq!(progress(&after), (Phase::Cancelled, 500));
        }
    }

    /// Two walks in flight: the older one's rows are about a question nobody
    /// is asking any more.
    #[test]
    fn a_superseded_walk_loses_to_the_newer_one() {
        let old = JobId::new(1);
        let new = JobId::new(2);
        let (state, _) = update(&awaiting(old), rows(old, 3));
        let (state, _) = update(&state, AppEvent::Started(new, Task::ReadHistory));
        assert_eq!(progress(&state), (Phase::Streaming, 0));

        let (state, _) = update(&state, rows(old, 500));
        assert_eq!(
            progress(&state),
            (Phase::Streaming, 0),
            "stale rows dropped"
        );

        let (state, _) = update(&state, rows(new, 2));
        assert_eq!(progress(&state), (Phase::Streaming, 2));
    }

    /// ADR-0008 in the state: a repository with no commit-graph file answers
    /// in commit-date order, and the screen is told the order is not final.
    #[test]
    fn a_walk_that_fell_back_to_date_order_says_it_is_provisional() {
        let id = JobId::new(7);

        let (next, _) = update(
            &awaiting(id),
            from(id, started(Order::Topological, Order::CommitDate)),
        );
        let walk = next.history.and_then(|history| history.walk);
        assert!(walk.expect("the walk said what it is").is_provisional());

        let (next, _) = update(
            &awaiting(id),
            from(id, started(Order::Topological, Order::Topological)),
        );
        let walk = next.history.and_then(|history| history.walk);
        assert!(!walk.expect("the walk said what it is").is_provisional());
    }

    /// A provisional walk and the topological sort it asked for, started as
    /// job `sort`.
    fn sorting(walk: JobId, sort: JobId) -> AppState {
        let (next, commands) = update(
            &awaiting(walk),
            from(walk, started(Order::Topological, Order::CommitDate)),
        );
        assert!(
            commands.contains(&Command::PromoteHistory),
            "a provisional walk asks for its order: {commands:?}"
        );
        let (next, _) = update(&next, AppEvent::Started(sort, Task::PromoteHistory));
        assert_eq!(next.promoting, Some(sort));
        next
    }

    fn order() -> std::sync::Arc<[Oid]> {
        vec![Oid::from_bytes(&[7; Oid::SHA1_BYTES]).expect("20 bytes is an oid")].into()
    }

    /// A walk already in the order asked for has nothing to promote.
    #[test]
    fn a_walk_in_its_final_order_asks_for_no_sort() {
        let id = JobId::new(7);
        let (_, commands) = update(
            &awaiting(id),
            from(id, started(Order::Topological, Order::Topological)),
        );
        assert!(!commands.contains(&Command::PromoteHistory));
    }

    /// The order from a sort that has been replaced by a newer one never
    /// reaches the walk; the newest sort's does.
    #[test]
    fn a_superseded_sort_is_never_applied() {
        let (walk, old, new) = (JobId::new(1), JobId::new(2), JobId::new(3));
        let state = sorting(walk, old);
        let (state, commands) = update(&state, AppEvent::Started(new, Task::PromoteHistory));
        assert_eq!(commands, vec![Command::CancelJob(old)]);

        let (state, commands) = update(
            &state,
            AppEvent::Produced(old, Product::TopologicalOrder(order())),
        );
        assert!(commands.is_empty(), "{commands:?}");
        let history = state.history.as_ref().expect("a walk");
        assert_eq!(history.promotion, Some(Promotion::Sorting));

        let (state, commands) = update(
            &state,
            AppEvent::Produced(new, Product::TopologicalOrder(order())),
        );
        let [Command::ShowRows(to, request)] = commands.as_slice() else {
            panic!("the newest order goes to the walk: {commands:?}");
        };
        assert_eq!(*to, walk);
        assert!(request.promote.is_some());
        assert_eq!(state.promoting, None);
    }

    /// `Esc` while sorting stops the sort and keeps the rows where they are,
    /// and the header says the order stays provisional.
    #[test]
    fn esc_while_sorting_keeps_the_provisional_order_and_says_so() {
        let (walk, sort) = (JobId::new(1), JobId::new(2));
        let state = sorting(walk, sort);
        let history = state.history.as_ref().expect("a walk");
        assert_eq!(
            crate::view::order_notice(history),
            Some(OrderNotice::Sorting)
        );

        let (state, commands) = update(&state, key(KeyCode::Escape));
        assert_eq!(commands, vec![Command::CancelJob(sort)]);
        assert_eq!(state.awaiting, Some(walk), "the walk itself goes on");
        let history = state.history.as_ref().expect("a walk");
        assert_eq!(history.promotion, Some(Promotion::Kept));
        assert_eq!(
            crate::view::order_notice(history),
            Some(OrderNotice::Provisional)
        );

        // The sort's late answer changes nothing.
        let (state, commands) = update(
            &state,
            AppEvent::Produced(sort, Product::TopologicalOrder(order())),
        );
        assert!(commands.is_empty(), "{commands:?}");
        let (_, commands) = update(&state, core(Event::JobCancelled(sort)));
        assert!(commands.is_empty(), "{commands:?}");
    }

    /// A sort that fails leaves the order provisional, not an error.
    #[test]
    fn a_failed_sort_leaves_the_order_provisional() {
        let (walk, sort) = (JobId::new(1), JobId::new(2));
        let (state, _) = update(
            &sorting(walk, sort),
            core(Event::JobFailed(sort, JobFailure::Failed("oom".to_owned()))),
        );
        assert_eq!(state.promoting, None);
        let history = state.history.as_ref().expect("a walk");
        assert_eq!(history.phase, Phase::Streaming);
        assert_eq!(history.promotion, Some(Promotion::Kept));
    }

    /// A walk that failed or was cancelled by something other than `Esc` —
    /// shutdown, say — must not leave the screen waiting for ever, and says
    /// which it was.
    #[test]
    fn a_walk_that_ends_without_finishing_says_how() {
        let id = JobId::new(3);

        let (cancelled, _) = update(&awaiting(id), core(Event::JobCancelled(id)));
        assert_eq!(cancelled.awaiting, None);
        assert_eq!(progress(&cancelled), (Phase::Cancelled, 0));

        let (failed, _) = update(
            &awaiting(id),
            core(Event::JobFailed(
                id,
                JobFailure::Failed("the pack is corrupt".to_owned()),
            )),
        );
        assert_eq!(failed.awaiting, None);
        let (phase, _) = progress(&failed);
        assert!(
            matches!(&phase, Phase::Failed(reason) if reason.contains("the pack is corrupt")),
            "{phase:?}"
        );
    }

    /// Purity's other half, which no runtime test can observe: that `update`
    /// cannot reach the world at all.
    ///
    /// This module is small and self-contained precisely so this check is
    /// possible — it reads its own source and fails if anything that talks to
    /// the outside (a file, a clock, a thread, a terminal) has been imported
    /// or called above the tests. A reviewer would have to notice; the test
    /// cannot help noticing.
    #[test]
    fn update_cannot_reach_the_world() {
        let source = include_str!("update.rs");
        let (code, _tests) = source
            .split_once("#[cfg(test)]")
            .expect("this module has a test section");

        for forbidden in [
            "std::io",
            "std::fs",
            "std::time",
            "std::thread",
            "std::process",
            "std::net",
            "std::env",
            "SystemTime",
            "Instant",
            "print!",
            "println!",
            "eprint!",
            "eprintln!",
            "crossterm",
            "ratatui",
        ] {
            assert!(
                !code.contains(forbidden),
                "`update` must stay pure, but its module mentions `{forbidden}`"
            );
        }
    }

    // Search (m1-the-graph/26).

    use crate::search::{Query, Search, SearchState};

    fn hit(byte: u8) -> Oid {
        Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
    }

    fn found(id: JobId, hits: &[u8]) -> AppEvent {
        AppEvent::Progress(
            id,
            Progress::Search(hits.iter().map(|byte| hit(*byte)).collect()),
        )
    }

    fn typed(state: &AppState, text: &str) -> (AppState, Vec<Command>) {
        let mut state = state.clone();
        let mut commands = Vec::new();
        for code in std::iter::once(KeyCode::Char('/'))
            .chain(text.chars().map(KeyCode::Char))
            .chain(std::iter::once(KeyCode::Enter))
        {
            let (next, more) = update(&state, key(code));
            state = next;
            commands.extend(more);
        }
        (state, commands)
    }

    /// A walk `walk` with rows 0..30 on hand, commit `i + 1` on row `i`, of 500
    /// laid out; ten graph rows.
    fn on_hand(walk: JobId) -> AppState {
        let (state, _) = update(&awaiting(walk), sized(80, 12));
        let AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(mut window))) =
            rows_for(walk, 2, 0..30, 500)
        else {
            unreachable!("`rows_for` answers with a window");
        };
        window.rows = (0..30u8)
            .map(|at| {
                let mut row = window.rows[0].clone();
                row.commit.id = hit(at + 1);
                row
            })
            .collect();
        let (state, _) = update(
            &state,
            AppEvent::Progress(walk, Progress::History(HistoryUpdate::Window(window))),
        );
        state
    }

    fn search_of(state: &AppState) -> &Search {
        state.search.as_ref().expect("a search")
    }

    #[test]
    fn slash_opens_a_prompt_that_takes_every_letter_and_enter_searches() {
        let state = on_hand(JobId::new(4));
        let (state, _) = update(&state, key(KeyCode::Char('/')));
        let (state, _) = update(&state, key(KeyCode::Char('q')));
        assert_eq!(state.quit, None, "q is text while the prompt is open");
        let (state, _) = update(&state, key(KeyCode::Char('x')));
        let (state, _) = update(&state, key(KeyCode::Backspace));
        assert_eq!(state.search_prompt.as_deref(), Some("q"));

        let (state, commands) = update(&state, key(KeyCode::Enter));
        assert_eq!(state.search_prompt, None);
        assert_eq!(
            commands,
            vec![Command::Search(Query::parse("q").expect("q"))]
        );
        assert_eq!(search_of(&state).state, SearchState::Running { hits: 0 });
    }

    #[test]
    fn an_empty_query_or_esc_closes_the_prompt_without_searching() {
        let state = on_hand(JobId::new(4));
        let (closed, commands) = typed(&state, "  ");
        assert_eq!(
            (closed.search_prompt.clone(), closed.search.clone()),
            (None, None)
        );
        assert!(commands.is_empty());
        let (open, _) = update(&state, key(KeyCode::Char('/')));
        let (closed, commands) = update(&open, key(KeyCode::Escape));
        assert_eq!((closed.search_prompt, commands), (None, vec![]));
    }

    /// Hits count as partial while the job runs, the first is jumped to at
    /// once, and the count settles when the job finishes.
    #[test]
    fn the_count_is_partial_while_running_and_settles_when_done() {
        let walk = JobId::new(4);
        let (state, _) = typed(&on_hand(walk), "fix");
        let job = JobId::new(9);
        let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
        let (state, _) = update(&state, found(job, &[3]));
        assert_eq!(search_of(&state).state.describe(), "1 match so far");
        let history = state.history.as_ref().expect("a walk");
        assert_eq!(history.cursor, 2, "the first hit is selected at once");
        assert!(search_of(&state).line().contains("(1/1)"));

        let (state, _) = update(&state, found(job, &[12, 20]));
        assert_eq!(search_of(&state).state.describe(), "3 matches so far");
        let (state, _) = update(&state, core(Event::JobFinished(job)));
        assert_eq!(search_of(&state).state, SearchState::Done { hits: 3 });
        assert_eq!(search_of(&state).state.describe(), "3 matches");
    }

    #[test]
    fn n_and_capital_n_walk_the_hits_and_wrap() {
        let walk = JobId::new(4);
        let (state, _) = typed(&on_hand(walk), "fix");
        let job = JobId::new(9);
        let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
        let (state, _) = update(&state, found(job, &[3, 12, 20]));
        let cursor = |state: &AppState| state.history.as_ref().map(|h| h.cursor);
        assert_eq!(cursor(&state), Some(2));
        let (state, _) = update(&state, key(KeyCode::Char('n')));
        assert_eq!(cursor(&state), Some(11));
        let (state, _) = update(&state, key(KeyCode::Char('n')));
        let (state, _) = update(&state, key(KeyCode::Char('n')));
        assert_eq!(cursor(&state), Some(2), "n wraps past the last hit");
        let (state, _) = update(&state, key(KeyCode::Char('N')));
        assert_eq!(cursor(&state), Some(19), "N wraps before the first");
    }

    /// A hit not in the rows on hand is a find request to the walk; the
    /// answer lands the cursor on it.
    #[test]
    fn a_far_hit_asks_the_walk_to_lay_out_to_it() {
        let walk = JobId::new(4);
        let (state, _) = typed(&on_hand(walk), "fix");
        let job = JobId::new(9);
        let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
        let (state, commands) = update(&state, found(job, &[200]));
        let [Command::ShowRows(to, asked)] = commands.as_slice() else {
            panic!("one request: {commands:?}");
        };
        assert_eq!(*to, walk);
        assert!(matches!(asked.wanted, Wanted::Find { commit, .. } if commit == hit(200)));
        assert_eq!(search_of(&state).seeking, Some(hit(200)));
    }

    /// Results from a superseded search job are dropped: its start is
    /// cancelled, and its hits never count.
    #[test]
    fn a_superseded_search_jobs_hits_are_discarded() {
        let walk = JobId::new(4);
        let (state, _) = typed(&on_hand(walk), "fix");
        // Typed again before the first search's job said it started.
        let (state, _) = typed(&state, "bug");
        let (old, new) = (JobId::new(9), JobId::new(10));
        let (state, commands) = update(&state, AppEvent::Started(old, Task::Search));
        assert_eq!(commands, vec![Command::CancelJob(old)]);
        let (state, _) = update(&state, AppEvent::Started(new, Task::Search));
        let (state, _) = update(&state, found(old, &[3]));
        assert_eq!(
            search_of(&state).hits(),
            0,
            "the old job's hits are dropped"
        );
        let (state, _) = update(&state, core(Event::JobFinished(old)));
        assert_eq!(search_of(&state).state, SearchState::Running { hits: 0 });
        assert_eq!(search_of(&state).job, Some(new));
        assert_eq!(search_of(&state).query.text(), "bug");
    }

    #[test]
    fn esc_cancels_a_running_search_and_restores_the_selection() {
        let walk = JobId::new(4);
        let (state, _) = typed(&on_hand(walk), "fix");
        let job = JobId::new(9);
        let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
        let (state, _) = update(&state, found(job, &[6]));
        assert_eq!(state.history.as_ref().map(|h| h.cursor), Some(5));

        let (state, commands) = update(&state, key(KeyCode::Escape));
        assert!(commands.contains(&Command::CancelJob(job)), "{commands:?}");
        assert_eq!(
            state.history.as_ref().map(|h| (h.cursor, h.top)),
            Some((0, 0))
        );
        assert_eq!(search_of(&state).state, SearchState::Cancelled { hits: 1 });
        assert_eq!(state.awaiting, Some(walk), "the walk goes on");

        let (state, _) = update(&state, core(Event::JobCancelled(job)));
        assert_eq!(search_of(&state).state, SearchState::Cancelled { hits: 1 });
        let (state, _) = update(&state, key(KeyCode::Escape));
        assert_eq!(state.search, None, "a second Esc closes the line");
    }

    /// No matches, cancelled and failed are three states that read
    /// differently, and a failure stays on the search's line.
    #[test]
    fn no_matches_cancelled_and_failed_are_distinct() {
        let walk = JobId::new(4);
        let (state, _) = typed(&on_hand(walk), "fix");
        let job = JobId::new(9);
        let (state, _) = update(&state, AppEvent::Started(job, Task::Search));

        let (none, _) = update(&state, core(Event::JobFinished(job)));
        let (cancelled, _) = update(&state, core(Event::JobCancelled(job)));
        let failure = JobFailure::Failed("walk broke".to_owned());
        let (failed, _) = update(&state, core(Event::JobFailed(job, failure)));

        let lines = [none, cancelled, failed].map(|state| search_of(&state).state.describe());
        assert_eq!(lines[0], "no matches");
        assert_eq!(lines[1], "search cancelled, 0 matches found");
        assert_eq!(lines[2], "search failed: walk broke");
    }

    #[test]
    fn a_handed_back_key_ends_the_run_naming_the_key() {
        let state = AppState {
            harness: Harness {
                hand_back: vec![KeyCode::Char(']')],
                ..Harness::default()
            },
            ..AppState::default()
        };
        let (next, commands) = update(&state, key(KeyCode::Char(']')));
        assert_eq!(next.quit, Some(Quit::HandedBack(KeyCode::Char(']'))));
        assert!(commands.is_empty());
        assert!(next.should_quit());

        let (next, _) = update(&state, key(KeyCode::Char('j')));
        assert_eq!(next.quit, None, "other keys are the graph's");
    }

    #[test]
    fn an_open_search_prompt_keeps_a_handed_back_key_as_text() {
        let state = AppState {
            harness: Harness {
                hand_back: vec![KeyCode::Char(']')],
                ..Harness::default()
            },
            search_prompt: Some(String::from("fox")),
            ..AppState::default()
        };
        let (next, _) = update(&state, key(KeyCode::Char(']')));
        assert_eq!(next.quit, None);
        assert_eq!(next.search_prompt.as_deref(), Some("fox]"));
    }
}
