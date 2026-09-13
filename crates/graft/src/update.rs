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

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_git::history::Order;
use graft_git::version::SystemGit;

use crate::history::{History, HistoryUpdate, Motion, Phase, RowRequest};
use crate::repository::RepositoryFacts;

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
    /// The history walk the screen shows: its rows so far and where it is.
    /// `None` until one has been asked for.
    pub history: Option<History>,
    /// The refs on each commit, once the refs job has answered.
    pub refs: Option<crate::refs::RefBadges>,
    /// Which screen is on show. The graph, unless the user asked for the
    /// status screen — the only place tier and cell size are shown.
    pub screen: Screen,
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
    /// How many rows of history the graph pane shows at the current size.
    /// Never zero, so a movement always has somewhere to go.
    #[must_use]
    pub fn graph_rows(&self) -> usize {
        usize::from(self.size.rows.saturating_sub(CHROME_ROWS)).max(1)
    }
}

/// Why Graft is leaving.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Quit {
    /// The user asked to.
    Requested,
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
    /// Hand history job `JobId` a request for rows. The rows come back as a
    /// [`HistoryUpdate::Window`] tagged with the request's id.
    ShowRows(JobId, RowRequest),
    /// Stop a job whose result is no longer wanted. Cooperative: a job that has
    /// not started is dropped, and a running one stops at its next check.
    CancelJob(JobId),
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
            if let Some(request) = history.show(next.graph_rows()) {
                commands.push(Command::ShowRows(id, request));
            }
            next.history = Some(history);
        }
        // Neither of these is waited on: their results are welcome whenever
        // they arrive, and there is nothing for `Esc` to cancel.
        AppEvent::Started(_, Task::OpenRepository | Task::CheckSystemGit | Task::ReadRefs) => {}
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
        AppEvent::Progress(id, Progress::History(update)) => {
            let height = next.graph_rows();
            if next.awaiting == Some(id)
                && let Some(history) = next.history.as_mut()
            {
                history.apply(update, height);
            }
            // Otherwise it is from a walk that was cancelled or superseded.
            // Dropped here, which is the whole reason the id travels with it:
            // no chunk lands after `Esc`.
        }
    }

    (next, commands)
}

/// The half of [`update`] that answers the events `core` defines.
fn core_event(next: &mut AppState, commands: &mut Vec<Command>, event: Event) {
    match event {
        Event::Key(key) => key_event(next, commands, key),
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
        Event::JobFailed(id, failure) => {
            if next.awaiting == Some(id) {
                next.awaiting = None;
                set_phase(next, Phase::Failed(failure.to_string()));
            }
            // Surfaced, not swallowed: the user is told which job failed and
            // why. A panicking job says so in the same place — it is still a
            // failure, and the crash report's path rides along with it.
            next.status = Status::Failed(format!("{id} failed: {failure}"));
        }
        Event::JobFinished(id) => {
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
            }
        }
        // Nothing is bound to these yet. They reach `update` so that the
        // tickets that need them have somewhere obvious to add the handling.
        Event::Mouse(_) | Event::Paste(_) | Event::FocusGained | Event::FocusLost => {}
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
        commands.push(Command::OpenRepository);
        commands.push(Command::ReadHistory(Order::Topological));
        commands.push(Command::ReadRefs);
    } else if is_screen_toggle(key) {
        next.screen = next.screen.toggled();
    } else if let Some(motion) = motion_for(key) {
        let height = next.graph_rows();
        if let Some(history) = next.history.as_mut()
            && let Some(request) = history.move_cursor(motion, height)
            && let Some(id) = next.awaiting
        {
            commands.push(Command::ShowRows(id, request));
        }
    } else if key.code == KeyCode::Escape {
        let height = next.graph_rows();
        // A `G` still walking is what `Esc` backs out of first; the
        // walk itself goes on serving the rows already on screen.
        if let Some(id) = next.awaiting
            && let Some(history) = next.history.as_mut()
            && let Some(request) = history.withdraw_jump(height)
        {
            commands.push(Command::ShowRows(id, request));
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

/// Manual refresh (DESIGN.md §2.1): re-read the repository now, rather than
/// waiting for whatever would have noticed on its own.
fn is_refresh(key: KeyPress) -> bool {
    key.code == KeyCode::Char('r') && key.modifiers.control
}

#[cfg(test)]
mod tests {
    use graft_core::event::{Modifiers, MouseInput, MouseKind};
    use graft_core::job::{JobFailure, JobId};

    use graft_core::commit::{Commit, Identity, Timestamp};
    use graft_core::oid::Oid;
    use graft_graph::Row;

    use super::*;
    use std::ops::Range;

    use crate::history::{WalkFacts, Wanted, Window};
    use crate::repository::Head;

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
            key(KeyCode::Enter),
            key(KeyCode::Function(7)),
            core(Event::Mouse(MouseInput {
                kind: MouseKind::Moved,
                column: 3,
                row: 4,
                modifiers: Modifiers::NONE,
            })),
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
            status: Status::Failed("earlier".to_owned()),
            size: Size {
                columns: 80,
                rows: 24,
            },
            repo_path: PathBuf::from("/code/api"),
            repository: Some(facts("/code/api", "main")),
            system_git: None,
            awaiting: None,
            history: None,
            refs: None,
            screen: Screen::Graph,
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
            vec![Command::ShowRows(id, request(1, Wanted::Rows(0..2)))],
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
        RowRequest { id, wanted }
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

        let (mut state, commands) = update(&state, key(KeyCode::Char('j')));
        assert!(commands.is_empty(), "row 1 is held: {commands:?}");
        assert_eq!(state.history.as_ref().map(|h| h.cursor), Some(1));

        for page in 1..=2 {
            let (paged, commands) = update(&state, key(KeyCode::PageDown));
            assert!(commands.is_empty(), "page {page} is held: {commands:?}");
            state = paged;
        }
        let (_, commands) = update(&state, key(KeyCode::PageDown));
        assert_eq!(
            commands,
            vec![Command::ShowRows(id, request(3, Wanted::Rows(20..50)))]
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
            vec![Command::ShowRows(id, request(4, Wanted::Rows(0..20)))]
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
}
