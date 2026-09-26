//! The history worker lays out ahead while nobody asks (m7-power-tools/12
//! Part B), over the real job pool: once it says it has reached the end,
//! `G` lands without laying out, and nothing wakes the loop after.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::fmt::Write as _;
use std::path::Path;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use graft::history::HistoryUpdate;
use graft::jobs::Dispatch;
use graft::session::Clock;
use graft::update::{AppEvent, AppState, Progress, update};
use graft_core::event::KeyCode;

use crate::app_harness::{App, PATIENCE, git};
use crate::captures::frame_at;

/// Commits in the generated line: a few thousand, several of the app's
/// 500-row chunks past what the first screen asks for.
const COMMITS: usize = 3_000;

/// A straight line of `COMMITS` commits on `main`, built with one
/// `git fast-import` (no commit-graph file, so the order is provisional and
/// is promoted, as in a repository nobody has run `git gc` in). Dated one
/// minute apart, so the ids are the same every run.
fn long_line(dir: &Path) {
    graft_fixtures::git::init_repo(dir).expect("git init");
    let mut stream = String::new();
    for i in 1..=COMMITS {
        let message = format!("commit {i}\n");
        let content = format!("{i}\n");
        let at = 1_600_000_000 + i * 60;
        writeln!(stream, "commit refs/heads/main").expect("a string");
        writeln!(stream, "mark :{i}").expect("a string");
        writeln!(
            stream,
            "committer Graft Test <test@example.invalid> {at} +0000"
        )
        .expect("a string");
        write!(stream, "data {}\n{message}", message.len()).expect("a string");
        if i > 1 {
            writeln!(stream, "from :{}", i - 1).expect("a string");
        }
        writeln!(stream, "M 644 inline file.txt").expect("a string");
        write!(stream, "data {}\n{content}\n", content.len()).expect("a string");
    }
    graft_fixtures::git::run_isolated(dir, &["fast-import", "--quiet"], Some(stream.as_bytes()))
        .expect("fast-import");
    git(dir, &["reset", "-q", "--hard", "main"]);
}

/// Is `event` from the history walk the app is showing?
fn from_the_walk(state: &AppState, event: &AppEvent) -> Option<HistoryUpdate> {
    match event {
        AppEvent::Progress(id, Progress::History(update)) if state.awaiting == Some(*id) => {
            Some(update.clone())
        }
        _ => None,
    }
}

/// Pumps until the walk says it has laid out to the end **in the final
/// order**: a `LaidOut` sent before the promotion is of a layout the
/// promotion replaces, and a `G` after it would lay out again.
fn laid_out_to_the_end(app: &App, mut state: AppState) -> AppState {
    loop {
        let event = app
            .events
            .recv_timeout(PATIENCE)
            .unwrap_or_else(|why| panic!("waiting for `LaidOut`: {why}; state: {state:?}"));
        let laid_out = matches!(
            from_the_walk(&state, &event),
            Some(HistoryUpdate::LaidOut { .. })
        );
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            app.jobs.dispatch(command);
        }
        // No walk until the repository has opened.
        let Some(history) = state.history.as_ref() else {
            continue;
        };
        let final_order = history
            .walk
            .as_ref()
            .is_some_and(|walk| !walk.is_provisional());
        if laid_out && final_order {
            assert_eq!(history.laid_out, Some(COMMITS), "every commit has a row");
            return state;
        }
    }
}

#[test]
fn g_after_laid_out_lands_without_laying_out() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let path = scratch.path();
    long_line(path);
    let (app, state) = App::open(path);
    let state = laid_out_to_the_end(&app, state);

    let mut state = app.key(&state, KeyCode::Char('G'));
    let mut on_the_way = Vec::new();
    while state.history.as_ref().expect("a walk").jump.is_some() {
        let event = app
            .events
            .recv_timeout(PATIENCE)
            .unwrap_or_else(|why| panic!("waiting for `G` to land: {why}"));
        if let Some(update) = from_the_walk(&state, &event)
            && !matches!(update, HistoryUpdate::Window(_))
        {
            on_the_way.push(update);
        }
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            app.jobs.dispatch(command);
        }
    }

    assert_eq!(on_the_way, [], "no progress between `G` and its rows");
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(history.cursor, COMMITS - 1, "the cursor is on the last row");
    let frame = frame_at(
        &state,
        120,
        40,
        Some(Clock::Fixed(graft_fixtures::CORPUS_NOW)),
    )
    .join("\n");
    assert!(!frame.contains("laying out"), "{frame}");
    let root = git(path, &["rev-list", "--max-parents=0", "HEAD"]);
    let root = root.trim().get(..7).expect("a full object id");
    assert!(frame.contains(root), "the root commit is drawn:\n{frame}");
    app.close();
}

#[test]
fn no_wake_after_laid_out() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let path = scratch.path();
    long_line(path);
    let (app, state) = App::open(path);
    let mut state = laid_out_to_the_end(&app, state);
    // Startup's other jobs (the working tree, the radar and its probes) run
    // on their own and may still be landing; the walk may not say more.
    let startup_done = |state: &AppState| {
        use graft::update::detail::Loading;
        state.working_tree.is_some()
            && matches!(state.radar.context, Loading::Ready(_))
            && state.radar.built_from.is_some()
            && !matches!(state.radar.duplicates, Loading::Pending(_))
            && !state
                .radar
                .force_push
                .values()
                .any(|verdict| matches!(verdict, Loading::Pending(_)))
    };
    while !startup_done(&state) {
        let event = app
            .events
            .recv_timeout(PATIENCE)
            .unwrap_or_else(|why| panic!("waiting for startup's jobs: {why}"));
        if let Some(update) = from_the_walk(&state, &event) {
            panic!("the walk spoke after `LaidOut`: {update:?}");
        }
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            app.jobs.dispatch(command);
        }
    }

    match app.events.recv_timeout(Duration::from_millis(500)) {
        Err(RecvTimeoutError::Timeout) => {}
        Ok(event) => panic!("a wake with no input after `LaidOut`: {event:?}"),
        Err(RecvTimeoutError::Disconnected) => panic!("the job pool went away"),
    }
    app.close();
}
