//! Running one scenario: the fixture, its setup, the app driven headless
//! over the real job pool at the scenario's size, the verdict, and `fsck`.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use graft::jobs::Dispatch as _;
use graft::update::detail::Loading;
use graft::update::{AppState, Status, update};
use graft_core::event::KeyCode;

use super::fixture::Fixture;
use super::gestures::{on_badge, on_subject};
use super::steps::{Scenario, Step, Wait};
use super::truth::{Claim, Marks, truth};
use crate::app_harness::{App, PATIENCE};
use crate::repo_truth::{check, fsck_clean};

/// Run one scenario start to finish: the fixture, its setup, the app, the
/// verdict, and `fsck --no-dangling` clean.
pub fn run_scenario(scenario: &Scenario) -> Result<(), String> {
    let fixture = Fixture::build(scenario.shape, scenario.remote)?;
    let mut marks = Marks::default();
    let driven = catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        (scenario.setup)(&fixture)?;
        marks.before = truth(&fixture, &[])?;
        drive_app(&fixture, scenario.size, &scenario.steps, &mut marks)
    }));
    match driven {
        Ok(result) => result?,
        Err(payload) => return Err(format!("panicked: {}", panic_text(payload.as_ref()))),
    }
    (scenario.expect)(&fixture, &marks)?;
    fsck_clean(fixture.repo())
}

/// Set a scenario up for a pty capture: the fixture, its setup and the
/// capture's headless steps. Hands back the fixture (alive while the
/// captured run lasts) and the directory to open.
pub fn prepare_capture(scenario: &Scenario) -> Result<(Fixture, PathBuf), String> {
    let capture = scenario
        .capture
        .as_ref()
        .ok_or_else(|| format!("{} has no capture", scenario.name))?;
    let fixture = Fixture::build(scenario.shape, scenario.remote)?;
    (scenario.setup)(&fixture)?;
    if !capture.headless.is_empty() {
        drive_app(
            &fixture,
            scenario.size,
            &capture.headless,
            &mut Marks::default(),
        )?;
    }
    let repo = fixture.repo().to_path_buf();
    Ok((fixture, repo))
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_else(|| "(no message)".to_owned())
}

/// Open the app on the repository at `size`, run the steps, and always
/// close the pool. A [`Step::Relaunch`] closes the app and opens it again
/// on the same repository for the steps after it.
fn drive_app(
    fixture: &Fixture,
    size: (u16, u16),
    steps: &[Step],
    marks: &mut Marks,
) -> Result<(), String> {
    let mut statuses = Vec::new();
    for chunk in steps.split(|step| matches!(step, Step::Relaunch)) {
        let (app, state) = App::open(fixture.repo());
        let state = app.resize(&state, size.0, size.1);
        let outcome = {
            let mut walk = Walk {
                app: &app,
                fixture,
                columns: size.0,
                statuses: std::mem::take(&mut statuses),
            };
            let outcome = catch_unwind(AssertUnwindSafe(|| walk.drive(state, chunk, marks)));
            statuses = walk.statuses;
            outcome
        };
        app.close();
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(why)) => return Err(why),
            Err(payload) => return Err(format!("panicked: {}", panic_text(payload.as_ref()))),
        }
    }
    Ok(())
}

pub(super) struct Walk<'a> {
    pub(super) app: &'a App,
    pub(super) fixture: &'a Fixture,
    pub(super) columns: u16,
    /// One line per distinct status the loop drew, in order.
    pub(super) statuses: Vec<String>,
}

impl Walk<'_> {
    fn drive(
        &mut self,
        mut state: AppState,
        steps: &[Step],
        marks: &mut Marks,
    ) -> Result<(), String> {
        for step in steps {
            state = match step {
                Step::Key(key) => self.app.press(&state, *key),
                Step::WaitFor(wait) => self.wait_for(state, *wait)?,
                Step::CursorTo(badge) => self.cursor_to(&state, |state| on_badge(state, badge))?,
                Step::CursorToSubject(subject) => {
                    self.cursor_to(&state, |state| on_subject(state, subject))?
                }
                Step::MenuPick(prefix) => self.menu_pick(state, prefix, true)?,
                Step::MenuPickOpen(prefix) => self.menu_pick(state, prefix, false)?,
                Step::DragBadgeOnto(badge) => self.drag_badge_onto(&state, badge)?,
                Step::Claim => {
                    marks.claim = Some(claim(&state)?);
                    state
                }
                Step::Confirm => {
                    check(state.preview.is_some() || state.confirm.is_some(), || {
                        format!("no box to confirm; status {:?}", state.status)
                    })?;
                    let state = self.app.key(&state, KeyCode::Right);
                    self.app.key(&state, KeyCode::Enter)
                }
                Step::Write(what) => self.settle_write(state, what)?,
                Step::Settle(what) => self.pump(state, what, |state| state.operating.is_none())?,
                Step::Stage(name) => self.stage(&state, name)?,
                Step::Shell(fix) => {
                    fix(self.fixture)?;
                    state
                }
                Step::Mark(name) => {
                    marks.at.insert(name, truth(self.fixture, &self.statuses)?);
                    state
                }
                Step::See(what, see) => {
                    see(&state).map_err(|why| format!("{what}: {why}"))?;
                    state
                }
                Step::Relaunch => unreachable!("drive_app splits the steps at a relaunch"),
            };
            self.note(&state);
        }
        Ok(())
    }

    fn note(&mut self, state: &AppState) {
        let line = status_text(state);
        if self.statuses.last() != Some(&line) {
            self.statuses.push(line);
        }
    }

    /// Feed job answers through `update` until `done`, recording every
    /// status line on the way. A silent pool is a named failure.
    pub(super) fn pump(
        &mut self,
        mut state: AppState,
        what: &str,
        done: impl Fn(&AppState) -> bool,
    ) -> Result<AppState, String> {
        loop {
            if done(&state) {
                return Ok(state);
            }
            let event = self
                .app
                .events
                .recv_timeout(PATIENCE)
                .map_err(|why| format!("waiting for {what}: {why}"))?;
            if std::env::var_os("GRAFT_TEST_TRACE").is_some() {
                eprintln!("[{what}] {event:?} -> operating {:?}", state.operating);
            }
            let (next, commands) = update(&state, event);
            state = next;
            self.note(&state);
            for command in commands {
                self.app.jobs.dispatch(command);
            }
        }
    }

    fn wait_for(&mut self, state: AppState, wait: Wait) -> Result<AppState, String> {
        match wait {
            Wait::Ready => self.pump(state, "rows, refs, the tree and the journal", ready),
            Wait::PreviewReady => self.pump(state, "the preview's plan", |state| {
                state.preview.as_ref().is_some_and(|preview| {
                    !matches!(preview.plan, Loading::Pending(_) | Loading::Idle)
                })
            }),
            Wait::BannerFocused => self.pump(state, "the stop on the conflict", |state| {
                state.operating.is_none()
                    && state.in_progress.is_some()
                    && !graft::update::banner::unmerged_paths(state).is_empty()
                    && state.banner_focus
                    && state.journal.reading.is_none()
            }),
            Wait::JournalHas(kind) => self.pump(state, "the journal re-read", move |state| {
                matches!(&state.journal.entries, Loading::Ready(read)
                    if read.entries.iter().any(|entry| entry.kind == kind))
                    && state.journal.reading.is_none()
            }),
            Wait::RefsCaughtUp => {
                let now = super::truth::refs_of(self.fixture.repo())?;
                self.pump(state, "the refs re-read", move |state| {
                    state
                        .refs
                        .as_deref()
                        .is_some_and(|refs| badges_match(refs, &now))
                })
            }
            Wait::Journal => self.pump(state, "the journal browser", |state| {
                state.journal.shown
                    && matches!(state.journal.entries, Loading::Ready(_))
                    && state.journal.reading.is_none()
            }),
        }
    }

    pub(super) fn settle_write(&mut self, state: AppState, what: &str) -> Result<AppState, String> {
        check(state.operating.is_some(), || {
            format!("{what}: no write was queued; status {:?}", state.status)
        })?;
        let state = self.pump(state, what, |state| state.operating.is_none())?;
        match &state.status {
            Status::Done(_) => Ok(state),
            other => Err(format!("{what} did not succeed: {other:?}")),
        }
    }
}

/// The plan behind the open preview, as the claim.
fn claim(state: &AppState) -> Result<Claim, String> {
    let preview = state
        .preview
        .as_ref()
        .ok_or_else(|| format!("no preview is open; status {:?}", state.status))?;
    match &preview.plan {
        Loading::Ready(planned) => Ok(Claim {
            plan: planned.clone(),
        }),
        other => Err(format!("the preview has no plan to claim: {other:?}")),
    }
}

/// Everything a history key decides from has landed.
fn ready(state: &AppState) -> bool {
    state.working_tree.is_some()
        && state.refs.is_some()
        && matches!(state.remotes, Loading::Ready(_))
        && state.operating.is_none()
        && state.journal.reading.is_none()
        && state
            .history
            .as_ref()
            .and_then(|history| history.window.as_ref())
            .is_some_and(|window| !window.rows.is_empty())
}

/// Every branch and remote badge points where git's ref does.
fn badges_match(
    refs: &graft::refs::RefBadges,
    now: &std::collections::BTreeMap<String, String>,
) -> bool {
    let branches = refs
        .branches()
        .map(|(oid, name, _, _)| (format!("refs/heads/{}", String::from_utf8_lossy(name)), oid));
    let remotes = refs.remotes().map(|(oid, name)| {
        (
            format!("refs/remotes/{}", String::from_utf8_lossy(name)),
            oid,
        )
    });
    branches
        .chain(remotes)
        .all(|(name, oid)| now.get(&name).is_some_and(|real| *real == oid.to_string()))
}

/// The status line's words, whatever its kind.
pub(super) fn status_text(state: &AppState) -> String {
    match &state.status {
        Status::Ready => String::new(),
        Status::Copied { short } => format!("copied {short}"),
        Status::Failed(text) | Status::Busy(text) | Status::Done(text) => text.clone(),
        Status::Sticky { reason, keys: None } => reason.clone(),
        Status::Sticky {
            reason,
            keys: Some(keys),
        } => format!("{reason} · {keys}"),
    }
}
