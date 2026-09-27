//! Running one scenario: the fixture, its setup, the app driven headless
//! over the real job pool with this binary as git's askpass helper, the
//! verdict, `fsck`, and the proof that no password leaked.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use graft::jobs::Dispatch as _;
use graft::update::detail::Loading;
use graft::update::{AppState, RefsRead, Status, update};
use graft_core::event::KeyCode;

use super::fixture::{Fixture, SECRETS};
use super::steps::{Scenario, Step, Wait};
use super::truth::{Marks, truth};
use crate::app_harness::{App, PATIENCE};
use crate::repo_truth::{check, fsck_clean};

/// Run one scenario start to finish: the fixture, its setup, the app, the
/// verdict, `fsck`, and the proof that no password leaked.
pub fn run_scenario(scenario: &Scenario) -> Result<(), String> {
    let fixture = Fixture::build(scenario.shape, scenario.remote)?;
    let mut marks = Marks::default();
    let driven = catch_unwind(AssertUnwindSafe(|| -> Result<Recorded, String> {
        (scenario.setup)(&fixture)?;
        marks.before = truth(&fixture, &[])?;
        drive_app(&fixture, &scenario.steps, &mut marks)
    }));
    let seen = match driven {
        Ok(result) => result?,
        Err(payload) => return Err(format!("panicked: {}", panic_text(payload.as_ref()))),
    };
    (scenario.expect)(&fixture, &marks)?;
    fsck_clean(fixture.work())?;
    // A scenario that types a secret must have left prompt dumps for
    // `no_secret_leaked` to read; otherwise that half of the scan passes by
    // never running.
    check(
        !scenario
            .steps
            .iter()
            .any(|step| matches!(step, Step::TypeSecret(_)))
            || !seen.prompts.is_empty(),
        || "a secret was typed but no open-prompt state was recorded".to_owned(),
    )?;
    no_secret_leaked(&fixture, &seen)
}

/// Set a scenario up for a pty capture: the fixture, its setup and the
/// capture's headless steps. Hands back the fixture (alive while the
/// captured run lasts — the HTTP remote must keep serving) and the
/// directory to open.
pub fn prepare_capture(scenario: &Scenario) -> Result<(Fixture, PathBuf), String> {
    let capture = scenario
        .capture
        .as_ref()
        .ok_or_else(|| format!("{} has no capture", scenario.name))?;
    let fixture = Fixture::build(scenario.shape, scenario.remote)?;
    (scenario.setup)(&fixture)?;
    if !capture.headless.is_empty() {
        drive_app(&fixture, &capture.headless, &mut Marks::default())?;
    }
    let work = fixture.work().to_path_buf();
    Ok((fixture, work))
}

/// The binary git runs as its askpass helper: `graft` itself under `cargo
/// test`, or this very example, which answers askpass mode the same way.
pub fn askpass_helper() -> Result<PathBuf, String> {
    match option_env!("CARGO_BIN_EXE_graft") {
        Some(graft) => Ok(PathBuf::from(graft)),
        None => std::env::current_exe().map_err(|why| format!("current_exe: {why}")),
    }
}

/// Neither password is in the journal on disk, nor in any event, status
/// line or open-prompt `{state:?}` the loop saw. Checked after every
/// scenario, not only the HTTP ones: the rule is about the whole app.
pub fn no_secret_leaked(fixture: &Fixture, seen: &Recorded) -> Result<(), String> {
    let journal_path = fixture.work().join(".git/graft/undo-journal.jsonl");
    let journal_bytes = match std::fs::read(&journal_path) {
        Ok(bytes) => bytes,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(why) => return Err(format!("{}: {why}", journal_path.display())),
    };
    for secret in SECRETS {
        check(
            !journal_bytes
                .windows(secret.len())
                .any(|window| window == secret.as_bytes()),
            || "a password reached the undo journal".to_owned(),
        )?;
        for line in seen.events.iter().chain(&seen.statuses) {
            check(!line.contains(secret), || {
                // The line itself is not printed: it holds the secret.
                "a password reached an event or status line the loop saw".to_owned()
            })?;
        }
        for dump in &seen.prompts {
            check(!dump.contains(secret), || {
                // Not printed, for the same reason.
                "a password reached the app state's Debug while a prompt was open".to_owned()
            })?;
        }
    }
    Ok(())
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_else(|| "(no message)".to_owned())
}

/// Everything the loop saw, so a scenario can prove what never appeared.
#[derive(Default)]
pub struct Recorded {
    /// One `{event:?}` per event, progress texts and prompts included.
    pub events: Vec<String>,
    /// One line per distinct status the loop drew, in order.
    pub statuses: Vec<String>,
    /// One `{state:?}` per state the loop passed through *while a
    /// credential prompt was open* — the whole state, so the prompt's own
    /// `Debug`, the field's and anything else that carries them are all in
    /// it. Scope item 5(d) of the secret-field ticket: the unit tests prove
    /// the redaction, this proves nothing walked around it on a real run.
    pub prompts: Vec<String>,
}

impl Recorded {
    fn note(&mut self, state: &AppState) {
        let line = status_text(state);
        if self.statuses.last() != Some(&line) {
            self.statuses.push(line);
        }
        if state.credential.is_some() {
            self.prompts.push(format!("{state:?}"));
        }
    }
}

/// Open the app on the work clone, run the steps, and always close the pool.
fn drive_app(fixture: &Fixture, steps: &[Step], marks: &mut Marks) -> Result<Recorded, String> {
    let (app, state) = App::open_with_askpass(fixture.work(), askpass_helper()?);
    let (outcome, seen) = {
        let mut walk = Walk {
            app: &app,
            fixture,
            seen: Recorded::default(),
        };
        let outcome = catch_unwind(AssertUnwindSafe(|| walk.drive(state, steps, marks)));
        (outcome, walk.seen)
    };
    app.close();
    match outcome {
        Ok(Ok(())) => Ok(seen),
        Ok(Err(why)) => Err(why),
        Err(payload) => Err(format!("panicked: {}", panic_text(payload.as_ref()))),
    }
}

struct Walk<'a> {
    app: &'a App,
    fixture: &'a Fixture,
    seen: Recorded,
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
                Step::Type(text) | Step::TypeSecret(text) => text
                    .chars()
                    .fold(state, |state, c| self.app.key(&state, KeyCode::Char(c))),
                Step::WaitFor(wait) => self.wait_for(state, *wait)?,
                Step::Write(what) => self.settle_write(state, what)?,
                Step::Settle(what) => self.pump(state, what, |state| state.operating.is_none())?,
                Step::Confirm => {
                    check(state.confirm.is_some(), || {
                        format!("no box to confirm; status {:?}", state.status)
                    })?;
                    let state = self.app.key(&state, KeyCode::Right);
                    self.app.key(&state, KeyCode::Enter)
                }
                Step::CommitButton => self.press_the_button(state)?,
                Step::Cancel(what) => self.cancel(&state, what)?,
                Step::UndoAll => self.undo_all(state)?,
                Step::Shell(fix) => {
                    fix(self.fixture)?;
                    state
                }
                Step::Mark(name) => {
                    marks
                        .at
                        .insert(name, truth(self.fixture, &self.seen.statuses)?);
                    state
                }
                Step::See(what, see) => {
                    see(&state).map_err(|why| format!("{what}: {why}"))?;
                    state
                }
            };
            self.seen.note(&state);
        }
        Ok(())
    }

    /// Feed job answers through `update` until `done`, recording every
    /// event and status line on the way. A silent pool is a named failure.
    fn pump(
        &mut self,
        mut state: AppState,
        what: &str,
        done: impl Fn(&AppState) -> bool,
    ) -> Result<AppState, String> {
        let mut deadline = crate::app_harness::Deadline::of(crate::app_harness::DEADLINE);
        loop {
            if done(&state) {
                return Ok(state);
            }
            let event = self
                .app
                .events
                .recv_timeout(PATIENCE)
                .map_err(|why| format!("waiting for {what}: {why}"))?;
            deadline.note(what, &event)?;
            self.seen.events.push(format!("{event:?}"));
            let (next, commands) = update(&state, event);
            state = next;
            self.seen.note(&state);
            if std::env::var_os("GRAFT_TEST_TRACE").is_some() {
                eprintln!(
                    "[{what}] {} -> operating {:?} remotes ready {} reading {:?} status {:?}",
                    self.seen.events.last().map_or("", String::as_str),
                    state.operating,
                    matches!(state.remotes, Loading::Ready(_)),
                    state.remote.reading,
                    state.status
                );
            }
            for command in commands {
                self.app.jobs.dispatch(command);
            }
        }
    }

    fn wait_for(&mut self, state: AppState, wait: Wait) -> Result<AppState, String> {
        match wait {
            Wait::Ready => self.pump(state, "rows, refs and the remote facts", ready),
            Wait::CredentialPrompt => self.pump(state, "git's credential question", |state| {
                state.credential.is_some()
            }),
            Wait::Connected => {
                let hung = self
                    .fixture
                    .hung
                    .as_ref()
                    .ok_or("Wait::Connected needs a hung remote")?;
                // The fixture's own event: its listener raises a Condvar as
                // it accepts. 30 s is the deadline for a connection that
                // never comes, not a poll interval.
                check(hung.wait_for_connection(Duration::from_secs(30)), || {
                    "git never connected to the hung remote".to_owned()
                })?;
                Ok(state)
            }
            Wait::PreviewPlanned => self.pump(state, "the preview's plan", |state| {
                state.preview.as_ref().is_some_and(|preview| {
                    !matches!(preview.plan, Loading::Pending(_) | Loading::Idle)
                })
            }),
            Wait::JournalHas(kind) => self.pump(state, "the journal re-read", move |state| {
                matches!(&state.journal.entries, Loading::Ready(read)
                    if read.entries.iter().any(|entry| entry.kind == kind))
                    && state.journal.reading.is_none()
            }),
        }
    }

    fn settle_write(&mut self, state: AppState, what: &str) -> Result<AppState, String> {
        check(state.operating.is_some(), || {
            format!("{what}: no write was queued; status {:?}", state.status)
        })?;
        let state = self.pump(state, what, |state| state.operating.is_none())?;
        match &state.status {
            Status::Done { .. } => Ok(state),
            other => Err(format!("{what} did not succeed: {other:?}")),
        }
    }

    fn press_the_button(&mut self, mut state: AppState) -> Result<AppState, String> {
        for _ in 0..4 {
            if state.commit_editor.as_ref().map(|editor| editor.focus)
                == Some(graft_ui::commit_editor::Field::CommitButton)
            {
                return Ok(self.app.key(&state, KeyCode::Enter));
            }
            state = self.app.key(&state, KeyCode::Tab);
        }
        Err("Tab never reached [ Commit ]".to_owned())
    }

    fn cancel(&mut self, state: &AppState, what: &str) -> Result<AppState, String> {
        check(state.operating.is_some(), || {
            format!(
                "{what}: nothing is running to cancel; status {:?}",
                state.status
            )
        })?;
        let pressed = Instant::now();
        let state = self.app.key(state, KeyCode::Escape);
        check(status_text(&state).ends_with("cancelling"), || {
            format!("Esc did not say cancelling: {:?}", state.status)
        })?;
        let state = self.pump(state, what, |state| state.operating.is_none())?;
        let took = pressed.elapsed();
        check(took < Duration::from_secs(5), || {
            format!("{what}: Esc took {took:?} to clear the write")
        })?;
        Ok(state)
    }

    fn undo_all(&mut self, mut state: AppState) -> Result<AppState, String> {
        for _ in 0..10 {
            state = self.app.key(&state, KeyCode::Char('u'));
            // A `u` that opens a box first — the drift rail, or the
            // undo-after-push box from M5 (ADR-0038 §4) — is answered with
            // its acting choice, which at index 1 is `[ Undo locally ]`.
            // "Undo everything" means everything, not "everything that did
            // not stop to ask".
            if state.confirm.is_some() {
                state = self.app.key(&state, KeyCode::Right);
                state = self.app.key(&state, KeyCode::Enter);
            }
            state = self.settle_write(state, "an undo")?;
            if status_text(&state).contains("nothing to undo") {
                return Ok(state);
            }
        }
        Err("ten undos and still something to undo".to_owned())
    }
}

/// Everything `f`/`p`/`P` decide from has landed.
fn ready(state: &AppState) -> bool {
    state.working_tree.is_some()
        && state.refs.is_some()
        // The refs answer the last read asked, not the ones before a write:
        // `P` refuses in words while a read is pending (m6-conflicts/49).
        && !matches!(state.refs_read, RefsRead::Pending(_))
        && matches!(state.remotes, Loading::Ready(_))
        && state.remote.reading.is_none()
        // What a lease would take off the remote: `P` says `still reading`
        // until it lands (m7-power-tools/16b).
        && !matches!(
            state.remote.last_rejected_push.as_ref().and_then(|rejected| rejected.leaves.as_ref()),
            Some((_, _, graft::update::remote::LeavesRead::Reading(_)))
        )
        && state.operating.is_none()
        && state
            .history
            .as_ref()
            .and_then(|history| history.window.as_ref())
            .is_some_and(|window| !window.rows.is_empty())
}

/// The status line's words, whatever its kind.
pub(super) fn status_text(state: &AppState) -> String {
    match &state.status {
        Status::Ready => String::new(),
        Status::Copied { short } => format!("copied {short}"),
        Status::Failed(text) | Status::Busy(text) | Status::Waiting(text) => text.clone(),
        Status::Done { what, also, keys } => [Some(what), also.as_ref(), keys.as_ref()]
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" · "),
        Status::Sticky { reason, keys: None } => reason.clone(),
        Status::Sticky {
            reason,
            keys: Some(keys),
        } => format!("{reason} · {keys}"),
    }
}
