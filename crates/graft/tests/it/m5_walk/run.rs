//! Running one scenario: the fixture, its setup, the app driven headless
//! over the real job pool at the scenario's size, the verdict, and `fsck`.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use graft::jobs::Dispatch as _;
use graft::update::detail::Loading;
use graft::update::{AppState, Status, update};
use graft_core::event::KeyCode;

use super::fixture::{Fixture, Remote, SECRETS};
use super::gestures::{on_badge, on_subject};
use super::steps::{Scenario, Step, Wait};
use super::truth::{Claim, Marks, truth};
use crate::app_harness::{App, PATIENCE};
use crate::repo_truth::{check, fsck_clean};

/// Run one scenario start to finish: the fixture, its setup, the app, the
/// verdict, `fsck --no-dangling` clean, and the proof that no password
/// leaked.
pub fn run_scenario(scenario: &Scenario) -> Result<(), String> {
    let fixture = Fixture::build(scenario.shape, scenario.remote)?;
    let mut marks = Marks::default();
    let driven = catch_unwind(AssertUnwindSafe(|| -> Result<Seen, String> {
        (scenario.setup)(&fixture)?;
        marks.before = truth(&fixture, &[])?;
        drive_app(
            &fixture,
            scenario.size,
            scenario.remote,
            &scenario.steps,
            &mut marks,
        )
    }));
    let seen = match driven {
        Ok(result) => result?,
        Err(payload) => return Err(format!("panicked: {}", panic_text(payload.as_ref()))),
    };
    (scenario.expect)(&fixture, &marks)?;
    fsck_clean(fixture.repo())?;
    // A scenario that types a secret must have left prompt dumps and
    // askpass invocations for the scan below to read; without them that
    // half of it passes by never running, which is exactly what
    // m5-history-and-ghost-preview/11 recorded about an empty `SECRETS`.
    let typed = scenario
        .steps
        .iter()
        .any(|step| matches!(step, Step::TypeSecret(_)));
    if typed {
        check(!seen.prompts.is_empty(), || {
            "a secret was typed but no open-prompt state was recorded".to_owned()
        })?;
        check(!askpass_log(&fixture)?.is_empty(), || {
            "a secret was typed but git never ran the askpass helper".to_owned()
        })?;
    }
    no_secret_leaked(&fixture, &seen)
}

/// Everything the loop saw, so a scenario can prove what never appeared.
#[derive(Default)]
pub struct Seen {
    /// One `{event:?}` per event the loop was fed.
    pub events: Vec<String>,
    /// One line per distinct status the loop drew, in order.
    pub statuses: Vec<String>,
    /// One `{state:?}` per state the loop passed through **while the
    /// credential prompt was open** — the whole state, so the prompt's own
    /// `Debug`, the field's and anything else that carries the typed bytes
    /// are all in it.
    pub prompts: Vec<String>,
}

/// Where the askpass wrapper writes each invocation's arguments and
/// environment.
fn askpass_log_path(fixture: &Fixture) -> PathBuf {
    fixture.root().join("askpass-invocations.txt")
}

/// What git passed the askpass helper, every time it ran it: one block of
/// arguments and the whole environment per invocation. Empty before the
/// first.
fn askpass_log(fixture: &Fixture) -> Result<Vec<u8>, String> {
    let path = askpass_log_path(fixture);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(bytes),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(why) => Err(format!("{}: {why}", path.display())),
    }
}

/// The binary git runs as its askpass helper — `graft` itself under `cargo
/// test`, or this very example — wrapped in a script that records its own
/// arguments and environment before handing over.
///
/// The wrapper is how the walk can say anything about **argv and env from
/// outside the app**: `AppState` cannot show what a child process was
/// started with, and the one child that could carry a credential is this
/// one.
fn askpass_wrapper(fixture: &Fixture) -> Result<PathBuf, String> {
    let real = match option_env!("CARGO_BIN_EXE_graft") {
        Some(graft) => PathBuf::from(graft),
        None => std::env::current_exe().map_err(|why| format!("current_exe: {why}"))?,
    };
    let wrapper = fixture.root().join("askpass-wrapper.sh");
    let log = askpass_log_path(fixture);
    let script = format!(
        "#!/bin/sh\n\
         {{ echo '--- askpass'; for arg in \"$@\"; do echo \"argv: $arg\"; done; env; }} >> '{}'\n\
         exec '{}' \"$@\"\n",
        log.display(),
        real.display()
    );
    std::fs::write(&wrapper, script).map_err(|why| format!("{}: {why}", wrapper.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755))
            .map_err(|why| format!("{}: {why}", wrapper.display()))?;
    }
    Ok(wrapper)
}

/// No password is in the journal on disk, in any event, status line or
/// open-prompt `{state:?}` the loop saw, in what git passed the askpass
/// helper (arguments **and** environment), or in any byte of any file the
/// fixture leaves behind. Run after **every** scenario, not only the one
/// that types one: the rule is about the whole app.
pub fn no_secret_leaked(fixture: &Fixture, seen: &Seen) -> Result<(), String> {
    let askpass = askpass_log(fixture)?;
    for secret in SECRETS {
        let needle = secret.as_bytes();
        check(!contains(&askpass, needle), || {
            // The block itself is not printed: it holds the secret.
            "a password reached the askpass helper's arguments or environment".to_owned()
        })?;
        for line in seen.events.iter().chain(&seen.statuses) {
            check(!line.contains(secret), || {
                "a password reached an event or a status line the loop saw".to_owned()
            })?;
        }
        for dump in &seen.prompts {
            check(!dump.contains(secret), || {
                "a password reached the app state's Debug while the prompt was open".to_owned()
            })?;
        }
        if let Some(path) = file_containing(&fixture.root(), needle)? {
            return Err(format!(
                "a password reached a file the run left behind: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack.len() >= needle.len()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

/// The first file under `root` whose bytes hold `needle` — the journal, a
/// git config, a log, a lock file, anything. Names the file rather than its
/// contents.
fn file_containing(root: &Path, needle: &[u8]) -> Result<Option<PathBuf>, String> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(format!("{}: {why}", root.display())),
    };
    for entry in entries {
        let entry = entry.map_err(|why| format!("{}: {why}", root.display()))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|why| format!("{}: {why}", path.display()))?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            if let Some(found) = file_containing(&path, needle)? {
                return Ok(Some(found));
            }
        } else if let Ok(bytes) = std::fs::read(&path) {
            // A file that cannot be read holds nothing this walk wrote; a
            // real read failure would already have failed the scenario.
            if contains(&bytes, needle) {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
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
            scenario.remote,
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
    remote: Remote,
    steps: &[Step],
    marks: &mut Marks,
) -> Result<Seen, String> {
    // Only an HTTP remote makes git ask for a credential, and only then is
    // the helper worth wrapping: `App::open` otherwise leaves the runner's
    // own default, which is what every other scenario runs under.
    let helper = match remote {
        Remote::Http => Some(askpass_wrapper(fixture)?),
        Remote::None | Remote::Local => None,
    };
    let mut seen = Seen::default();
    for chunk in steps.split(|step| matches!(step, Step::Relaunch)) {
        let (app, state) = match &helper {
            Some(helper) => App::open_with_askpass(fixture.repo(), helper.clone()),
            None => App::open(fixture.repo()),
        };
        let state = app.resize(&state, size.0, size.1);
        let outcome = {
            let mut walk = Walk {
                app: &app,
                fixture,
                columns: size.0,
                seen: std::mem::take(&mut seen),
            };
            let outcome = catch_unwind(AssertUnwindSafe(|| walk.drive(state, chunk, marks)));
            seen = walk.seen;
            outcome
        };
        app.close();
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(why)) => return Err(why),
            Err(payload) => return Err(format!("panicked: {}", panic_text(payload.as_ref()))),
        }
    }
    Ok(seen)
}

pub(super) struct Walk<'a> {
    pub(super) app: &'a App,
    pub(super) fixture: &'a Fixture,
    pub(super) columns: u16,
    /// Everything the loop drew, was fed, and passed through.
    pub(super) seen: Seen,
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
                Step::Type(text) | Step::TypeSecret(text) => {
                    text.chars().fold(state, |state, character| {
                        self.app.key(&state, KeyCode::Char(character))
                    })
                }
                Step::KeyFor(what, key, see) => {
                    let (next, commands) = crate::app_harness::press_for_commands(&state, *key);
                    see(&commands).map_err(|why| format!("{what}: {why}"))?;
                    next
                }
                Step::Cancel(what) => self.cancel(&state, what)?,
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
                Step::Confirm => self.choose(state, 1)?,
                Step::Choose(index) => self.choose(state, *index)?,
                Step::Write(what) => self.settle_write(state, what)?,
                Step::Settle(what) => self.pump(state, what, |state| state.operating.is_none())?,
                Step::Stage(name) => self.stage(&state, name)?,
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
                Step::Relaunch => unreachable!("drive_app splits the steps at a relaunch"),
            };
            self.note(&state);
        }
        Ok(())
    }

    /// The open box's choice at `index`: `→` that many times off `Cancel`,
    /// then `Enter`. [`Step::Confirm`] is this with `index` 1.
    fn choose(&mut self, state: AppState, index: usize) -> Result<AppState, String> {
        check(state.preview.is_some() || state.confirm.is_some(), || {
            format!("no box to answer; status {:?}", state.status)
        })?;
        let state = (0..index).fold(state, |state, _| self.app.key(&state, KeyCode::Right));
        Ok(self.app.key(&state, KeyCode::Enter))
    }

    fn note(&mut self, state: &AppState) {
        let line = status_text(state);
        if self.seen.statuses.last() != Some(&line) {
            self.seen.statuses.push(line);
        }
        if state.credential.is_some() {
            self.seen.prompts.push(format!("{state:?}"));
        }
    }

    /// `Esc` on the write in flight, then its answer. The key must be seen
    /// at once — `cancel_operating` sets `· cancelling` on the status line
    /// in the same `update` — and the write must clear within five seconds
    /// of it.
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
            self.seen.events.push(format!("{event:?}"));
            let (next, commands) = update(&state, event);
            state = next;
            self.note(&state);
            for command in commands {
                self.app.jobs.dispatch(command);
            }
        }
    }

    /// The two waits that must read git **before** they start pumping: the
    /// app has caught up when what it holds equals what the refs say *now*,
    /// and "now" has to be taken before the first answer is fed back.
    fn wait_for(&mut self, state: AppState, wait: Wait) -> Result<AppState, String> {
        match wait {
            Wait::RefsCaughtUp => {
                let now = super::truth::refs_of(self.fixture.repo())?;
                self.pump(state, "the refs re-read", move |state| {
                    state
                        .refs
                        .as_deref()
                        .is_some_and(|refs| badges_match(refs, &now))
                })
            }
            Wait::RemotesCaughtUp => {
                let now = super::truth::refs_of(self.fixture.repo())?;
                self.pump(state, "the remote facts re-read", move |state| {
                    let Loading::Ready(facts) = &state.remotes else {
                        return false;
                    };
                    facts.head_upstream.as_ref().is_some_and(|upstream| {
                        now.get(&upstream.tracking_ref).map(String::as_str)
                            == upstream.tracking_tip.map(|tip| tip.to_string()).as_deref()
                    })
                })
            }
            other => self.wait_for_state(state, other),
        }
    }

    /// Every other wait: a predicate over `AppState` alone.
    fn wait_for_state(&mut self, state: AppState, wait: Wait) -> Result<AppState, String> {
        match wait {
            Wait::RefsCaughtUp | Wait::RemotesCaughtUp => {
                unreachable!("wait_for answers the two that read git first")
            }
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
            Wait::Journal => self.pump(state, "the journal browser", |state| {
                state.journal.shown
                    && matches!(state.journal.entries, Loading::Ready(_))
                    && state.journal.reading.is_none()
            }),
            Wait::Prompt => self.pump(state, "the name prompt", |state| {
                state.name_prompt.is_some()
            }),
            Wait::StashReady => self.pump(state, "the stash list", |state| {
                matches!(state.stash.entries, Loading::Ready(_))
            }),
            Wait::RadarReady => self.pump(state, "the radar's context", |state| {
                matches!(state.radar.context, Loading::Ready(_))
            }),
            Wait::NotInProgress => self.pump(state, "the in-progress re-read", |state| {
                state.in_progress.is_none() && state.operating.is_none()
            }),
            Wait::Resolved => self.pump(state, "the resolved tree", |state| {
                state.operating.is_none() && graft::update::banner::unmerged_paths(state).is_empty()
            }),
            Wait::JournalRead => self.pump(state, "the journal's entries", |state| {
                matches!(state.journal.entries, Loading::Ready(_))
                    && state.journal.reading.is_none()
            }),
            Wait::CredentialPrompt => self.pump(state, "git's credential question", |state| {
                state.credential.is_some()
            }),
            Wait::ChangesPane => self.pump(state, "the changes pane's file list", |state| {
                matches!(state.detail.files, Loading::Ready(_)) && state.on_changes_pane()
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
