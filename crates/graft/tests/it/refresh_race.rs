//! `Ctrl+R` pressed again before the previous refresh's walk has announced
//! itself, over the real job pool (m5-history-and-ghost-preview/05, the bug
//! ADR-0037 §6 reclassified out of `KNOWN_GAPS.md`).
//!
//! The ordering under test is pinned, not waited for: `App::press` runs
//! `update` and dispatches its commands without reading a single job event,
//! so the second `Ctrl+R` is guaranteed to land in the window the bug lived
//! in. Nothing here sleeps or polls; the only duration is `PATIENCE`, the
//! bound that turns a starved pool into a named failure instead of a suite
//! that never returns.

use std::time::Duration;

use graft::jobs::Dispatch;
use graft::update::{AppEvent, AppState, Product, Task, update};
use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;

use crate::app_harness::{App, PATIENCE};

fn ctrl_r(app: &App, state: &AppState) -> AppState {
    app.press(state, KeyPress::control(KeyCode::Char('r')))
}

/// Has the walk on screen answered with rows?
fn rows_arrived(state: &AppState) -> bool {
    state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .is_some_and(|window| !window.rows.is_empty())
}

/// Open a repository and let its opening walk settle: it has answered the
/// first window and is now parked on a worker, waiting for row requests.
/// That is the state a person's `Ctrl+R` actually arrives in.
fn opened() -> (App, AppState) {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Linear);
    let (app, state) = App::open(&dir);
    let state = app.pump(state, "the opening walk's first rows", |state| {
        state.refs.is_some() && state.working_tree.is_some() && rows_arrived(state)
    });
    (app, state)
}

/// The M4 acceptance walk's failure, as a test
/// (`.scratch/m4-remotes/issues/07` NOTES finding 1): two refreshes with no
/// job event read in between used to leave the first one's walk uncancelled,
/// and two walks parked on a two-worker pool starve every read queued behind
/// them — `ReadRefs` and `ReadWorkingTree` never run at all.
///
/// The verdict is the pool's, not `AppState::awaiting`'s. Each refresh asks
/// for one refs read and one working-tree read, so two refreshes owe two of
/// each; counting the answers is what tells the second refresh's apart from
/// the first's, which does come back either way. With a walk left parked
/// there is no worker for the second pair and this loop never finishes.
#[test]
fn two_refreshes_before_started_leave_one_history_job() {
    let (app, state) = opened();

    let state = ctrl_r(&app, &state);
    let mut state = ctrl_r(&app, &state);

    let mut refs = 0usize;
    let mut working_tree = 0usize;
    // How many walks the screen took a `Started` from: one refresh's walk
    // supersedes the other's, so only one of the two can be accepted.
    let mut accepted = 0usize;
    while refs < 2 || working_tree < 2 {
        let event = app.events.recv_timeout(PATIENCE).expect(
            "both refreshes' refs and working-tree reads — a pool with a walk \
             left parked on it never answers the second",
        );
        match &event {
            AppEvent::Produced(_, Product::Refs(_)) => refs += 1,
            AppEvent::Produced(_, Product::WorkingTree(_)) => working_tree += 1,
            AppEvent::Started(id, Task::ReadHistory) if state.awaiting == Some(*id) => {
                accepted += 1;
            }
            _ => {}
        }
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            app.jobs.dispatch(command);
        }
    }

    assert_eq!(
        accepted, 1,
        "exactly one of the two refreshes' walks is the one on screen"
    );
    app.close();
}

/// The cancel itself, named: the walk the first refresh asked for is told to
/// stop, and the pool reports it stopping. Nothing about this is a failure —
/// the status line must go on saying whatever it was saying.
#[test]
fn refresh_cancels_the_walk_it_replaces() {
    let (app, state) = opened();
    let status_before = state.status.clone();

    let state = ctrl_r(&app, &state);
    let replaced = state.awaiting.expect("the first refresh named its walk");
    let state = ctrl_r(&app, &state);
    let live = state.awaiting.expect("the second refresh named its walk");
    assert_ne!(replaced, live);

    let mut cancelled: Vec<JobId> = Vec::new();
    let mut state = state;
    while !cancelled.contains(&replaced) {
        let event = app
            .events
            .recv_timeout(PATIENCE)
            .expect("the replaced walk must report that it stopped");
        if let AppEvent::Core(Event::JobCancelled(id)) = &event {
            cancelled.push(*id);
        }
        assert!(
            !matches!(&event, AppEvent::Core(Event::JobFailed(id, _)) if *id == replaced),
            "a replaced walk stops; it does not fail"
        );
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            app.jobs.dispatch(command);
        }
    }

    assert!(
        !cancelled.contains(&live),
        "the walk the screen is waiting for is not the one cancelled"
    );
    assert_eq!(
        state.status, status_before,
        "a refresh that replaces a walk is ordinary work, not something to report"
    );
    app.close();
}

/// A walk that has been superseded still announces itself — the announcement
/// was in flight when the cancel was sent. The screen must drop it: taking it
/// would blank the rows the live walk has already put there.
///
/// The end-to-end half of `update::tests::stale_started_is_dropped`, over the
/// real pool, where the two `Started` events genuinely race.
#[test]
fn a_superseded_walks_started_never_reaches_the_screen() {
    let (app, state) = opened();

    let state = ctrl_r(&app, &state);
    let replaced = state.awaiting.expect("the first refresh named its walk");
    let mut state = ctrl_r(&app, &state);
    let live = state.awaiting.expect("the second refresh named its walk");
    state.history = None;

    // Every `Started` the pool announces, in the order it announced them.
    let mut history_starts: Vec<JobId> = Vec::new();
    loop {
        let event = app
            .events
            .recv_timeout(PATIENCE)
            .expect("the refreshed reads must come back");
        if let AppEvent::Started(id, Task::ReadHistory) = &event {
            history_starts.push(*id);
        }
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            app.jobs.dispatch(command);
        }
        if rows_arrived(&state) && history_starts.contains(&live) {
            break;
        }
    }

    assert!(
        history_starts.contains(&replaced),
        "the replaced walk did announce itself: {history_starts:?}"
    );
    assert_eq!(
        state.awaiting,
        Some(live),
        "and the screen still names the live walk, not the one it replaced"
    );
    // A `Started` that had been taken would have put a fresh, empty
    // `History` here in place of the rows that arrived.
    assert!(
        rows_arrived(&state),
        "the live walk's rows survived the superseded walk's announcement"
    );
    app.close();
}

/// Cancelling a walk that is parked between row requests gives its worker
/// back. It cannot see its own token from inside `recv`, so the runner has to
/// close its request channel; a walk that stayed parked would hold a thread
/// for the rest of the run.
///
/// Pinned by exhaustion rather than by timing: a one-worker pool can only
/// answer the second walk if the first one really let go.
#[test]
fn cancelling_a_parked_walk_gives_its_worker_back() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Linear);
    let (app, state) = App::open_with_workers(&dir, 1);
    let state = app.pump(state, "the parked walk's first rows", rows_arrived);
    let parked = state.awaiting.expect("the opening walk is named");

    let mut state = ctrl_r(&app, &state);
    let replacement = state.awaiting.expect("the refresh named its walk");
    assert_ne!(parked, replacement);
    state.history = None;

    let state = app.pump(
        state,
        "the replacement walk's rows — a held worker hangs here",
        rows_arrived,
    );

    assert_eq!(state.awaiting, Some(replacement));
    assert_eq!(
        app.jobs.shutdown(Duration::from_secs(10)),
        graft::jobs::Shutdown::Clean,
        "no worker is still inside the walk that was cancelled"
    );
}
