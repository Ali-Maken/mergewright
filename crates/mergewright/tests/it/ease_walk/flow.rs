//! The merge mock's flow on the mock-shaped repository: the preview, the
//! stop on the clash screen, the picks, the review and the landing. Each
//! helper waits on the state the next key decides from, never on a sleep.
//!
//! Branches (see `fixture.rs`): `integration` clashes with `wallet-release`
//! once (`report.md`); `pay-307` three times (`notes.md` twice, `report.md`
//! once) with `checks.md` combined by git; `ops/ci` merges clean.

use std::path::Path;

use mergewright::session::Clock;
use mergewright::update::AppState;
use mergewright::update::detail::Loading;
use mergewright_core::event::KeyCode;
use mergewright_git::state::InProgress;

use super::fixture::mock;
use super::screen::{cursor_on, drawn, open, sized};
use crate::app_harness::{App, git};

/// The mock repository opened at 120 columns, at rest.
pub fn opened() -> (tempfile::TempDir, App, AppState) {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 120);
    (scratch, app, state)
}

/// The preview's plan landed.
pub fn planned(app: &App, state: AppState) -> AppState {
    app.pump(state, "the merge plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !matches!(preview.plan, Loading::Idle | Loading::Pending(_)))
    })
}

/// The cursor on the row whose subject starts with `subject`, then `m`.
pub fn previewed(app: &App, state: AppState, subject: &str) -> AppState {
    let state = cursor_on(app, state, subject);
    planned(app, app.key(&state, KeyCode::Char('m')))
}

/// `y` in the preview (or its list), and the stop's first file read: the
/// clash screen as the stop lands on it.
pub fn stop_after_y(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('y'));
    let state = app.pump(state, "the operation", |state| state.operating.is_none());
    let state = app.pump(state, "the stop's first file", |state| {
        state
            .in_progress
            .as_ref()
            .is_some_and(|detail| detail.kind == InProgress::Merge)
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.file.is_some() && open.set_job.is_none())
            && !mergewright::update::resolver::listed_files(state).is_empty()
    });
    sized(app, &state, 120, super::screen::ROWS)
}

/// The mock repository, `m y` on `branch`'s row: stopped on the clash
/// screen at 120 columns.
pub fn stopped(branch: &str) -> (tempfile::TempDir, App, AppState) {
    let (scratch, app, state) = opened();
    let state = crate::clash_stops::stop_by(&app, state, branch.as_bytes(), 'm', InProgress::Merge);
    let state = sized(&app, &state, 120, super::screen::ROWS);
    (scratch, app, state)
}

/// A resolver key (`o t b u n …`) and the take or read it starts, landed.
pub fn pick(app: &App, state: &AppState, key: char) -> AppState {
    let state = app.key(state, KeyCode::Char(key));
    app.pump(state, "the pick", |state| {
        state
            .resolver
            .as_ref()
            .is_none_or(|open| open.job.is_none() && open.in_flight.is_none())
    })
}

/// `y` on the clash screen: the review, its read landed.
pub fn reviewed(app: &App, state: &AppState) -> AppState {
    crate::finish::review_landed(app, app.key(state, KeyCode::Char('y')))
}

/// `y` on the review: the merge committed, landed on the graph, at rest.
pub fn committed(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('y'));
    assert!(state.operating.is_some(), "`y` on the review commits");
    let state = app.pump(state, "the continue", |state| state.operating.is_none());
    let state = app.pump(state, "the landing the finish owes", |state| {
        state.in_progress.is_none() && !state.land_on_head
    });
    crate::captures::at_rest(app, state)
}

/// Pumped until no line of the frame says `reading…`.
pub fn read_in(app: &App, state: AppState) -> AppState {
    app.pump(state, "nothing reading", |state| {
        drawn(state)
            .iter()
            .all(|row| !row.contains("reading\u{2026}"))
    })
}

/// The frame against the wall clock: frames after a write read ages "in
/// the future" against the fixtures' fixed clock (14's Gotchas).
pub fn wall(state: &AppState) -> Vec<String> {
    crate::captures::frame_at(
        state,
        state.size.columns,
        state.size.rows,
        Some(Clock::Wall),
    )
}

/// HEAD's parents.
pub fn head_parents(repo: &Path) -> usize {
    git(repo, &["log", "-1", "--format=%P"])
        .split_whitespace()
        .count()
}

/// Paths git still holds unmerged.
pub fn unmerged(repo: &Path) -> String {
    git(repo, &["ls-files", "-u"])
}

/// The open resolver's path and cursor block.
pub fn at_clash(state: &AppState) -> (String, Option<usize>) {
    let Some(open) = state.resolver.as_ref() else {
        panic!("the clash screen is open");
    };
    (open.path.display().to_string(), open.cursor_block)
}
