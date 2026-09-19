//! The M4 acceptance walk (m4-remotes/07): one scenario per promise the
//! milestone makes — *you never leave Graft to fetch, pull or push, not for
//! a password, a rejection or a hang* — each against a **real remote**: a
//! bare repository over `file://` ([`LocalRemote`]), smart HTTP with Basic
//! auth ([`HttpRemote`]) or a port that never answers ([`HungRemote`]). The
//! real app is driven by keys over the real job pool and real `git`, with
//! `graft` itself as git's askpass helper, and then **repository truth is
//! read back by running git on both the work clone and the bare** — never
//! from `AppState`. `examples/m4_walk.rs` includes this file by `#[path]`
//! and prints one line per scenario; `every_scenario` below runs the same
//! list under `cargo test`, one test per scenario.
//!
//! Same shape as `m3_walk`: [`Step`]s that press, wait, mark and shell out,
//! and a [`Capture`] naming the keys a pty run replays as the harness
//! prelude. What is new: the fixture is a remote, [`Truth`] carries the bare's
//! refs, the journal, every status line the loop drew and the remote's own
//! counters, and every scenario ends by proving the walk's passwords reached
//! neither the journal on disk nor any event or status line the loop saw.
#![allow(
    clippy::expect_used,
    reason = "a walk step that cannot proceed has nothing left to prove; the panic names why"
)]

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use graft::jobs::Dispatch as _;
use graft::update::detail::Loading;
use graft::update::{AppState, Status, update};
use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::journal::{EntryStatus, JournalEntry, OperationKind};
use graft_fixtures::Shape;
use graft_fixtures::remote::{HttpRemote, HungRemote, LocalRemote};
use graft_ui::journal_screen::RowMark;

use crate::app_harness::{App, PATIENCE};
use crate::m3_walk::{
    RepoTruth, diff_truth as diff_work, fsck_clean, git_truth, keys_label,
    neutralise_global_config, run_git,
};

/// The HTTP remote's user; asked in the open when the URL does not name it.
pub const USER: &str = "tester";
/// The HTTP remote's password. Typed into the masked prompt, never echoed:
/// [`no_secret_leaked`] proves it, and `pty-walk-m4.py` greps every capture
/// for it.
pub const PASSWORD: &str = "correct-horse-battery-staple";
/// The password git is given first in `wrong-password-then-right`.
pub const WRONG: &str = "not-the-password";
/// Every string that must appear in no capture, status line, event or
/// journal — the pty script keeps the same list.
pub const SECRETS: [&str; 2] = [PASSWORD, WRONG];

/// A scenario's `setup` and a `Shell` step: git and plain files, against the
/// fixture (work clone, bare, or a teammate's clone).
pub type Fix = fn(&Fixture) -> Result<(), String>;
/// A fact only the UI state holds (a box's title, a prompt's mask).
pub type See = fn(&AppState) -> Result<(), String>;
/// The verdict: from git on both repositories and the marks the steps took.
pub type Expect = fn(&Fixture, &Marks) -> Result<(), String>;

/// Which remote the work clone's `origin` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteKind {
    /// The bare over `file://` — the transport `LocalRemote` gives.
    Local,
    /// The same bare over smart HTTP with Basic auth (`USER`/`PASSWORD`);
    /// `anonymous` leaves the user out of the URL so git asks for it first.
    Http { anonymous: bool },
    /// A loopback port that accepts and never answers.
    Hung,
}

/// The remote a scenario runs against, alive for as long as it runs.
pub struct Fixture {
    /// The bare, the work clone and a teammate's clone.
    pub local: LocalRemote,
    /// Serving the bare, when the scenario is over HTTP.
    pub http: Option<HttpRemote>,
    /// Never answering, when the scenario is about a hang.
    pub hung: Option<HungRemote>,
}

impl Fixture {
    /// Build `shape` as a remote of `kind`, and put the machine's own git
    /// config out of the app's reach in the work clone.
    pub fn build(shape: Shape, kind: RemoteKind) -> Result<Self, String> {
        let local = LocalRemote::new(shape).map_err(|why| format!("fixture: {why:?}"))?;
        // The app under test runs in this process with the user's real git
        // config; the work clone overrides what would change a write's
        // meaning (m3_walk), and blanks any credential helper: only the keys
        // this walk presses may answer git (credential_by_key).
        neutralise_global_config(local.work())?;
        run_git(
            local.work(),
            &["config", "--local", "credential.helper", ""],
        )?;
        let mut fixture = Fixture {
            local,
            http: None,
            hung: None,
        };
        match kind {
            RemoteKind::Local => {}
            RemoteKind::Http { anonymous } => {
                let http = HttpRemote::serve(fixture.local.bare(), USER, PASSWORD)
                    .map_err(|why| format!("the HTTP remote: {why}"))?;
                let url = if anonymous {
                    http.url_anonymous()
                } else {
                    http.url()
                };
                fixture.git(fixture.work(), &["remote", "set-url", "origin", &url])?;
                fixture.http = Some(http);
            }
            RemoteKind::Hung => {
                let hung = HungRemote::listen().map_err(|why| format!("the hung remote: {why}"))?;
                fixture.git(
                    fixture.work(),
                    &["remote", "set-url", "origin", &hung.url()],
                )?;
                fixture.hung = Some(hung);
            }
        }
        Ok(fixture)
    }

    /// The clone the app opens.
    pub fn work(&self) -> &Path {
        self.local.work()
    }

    /// The remote itself.
    pub fn bare(&self) -> &Path {
        self.local.bare()
    }

    /// The scratch root: the bare, the work clone and the teammate's clone
    /// are its children, and so is any clone a scenario adds.
    pub fn root(&self) -> &Path {
        self.work()
            .parent()
            .expect("the work clone lives inside the fixture's scratch")
    }

    /// The default branch's short name (`main` for the corpus shapes).
    pub fn branch(&self) -> &str {
        self.local.default_branch()
    }

    /// `refs/heads/<branch>`.
    pub fn main_ref(&self) -> String {
        format!("refs/heads/{}", self.branch())
    }

    /// `refs/remotes/origin/<branch>`.
    pub fn tracking_ref(&self) -> String {
        format!("refs/remotes/origin/{}", self.branch())
    }

    /// Hermetic `git` in `dir` (the fixture's own runner), trimmed stdout.
    pub fn git(&self, dir: &Path, args: &[&str]) -> Result<String, String> {
        self.local
            .git(dir, args)
            .map_err(|why| format!("git {args:?} in {}: {why}", dir.display()))
    }

    /// A teammate's commit that **changes** `file` (a tracked file of the
    /// shape) to `content` and pushes it, so the work clone falls behind on
    /// a path it has. `LocalRemote::commit_on_remote` adds a new file, which
    /// is not what a dirty-file refusal is about.
    pub fn change_on_remote(&self, file: &str, content: &str) -> Result<String, String> {
        let mate = self.root().join("mate-change");
        if !mate.exists() {
            let bare = self.bare().to_string_lossy().into_owned();
            let mate_arg = mate.to_string_lossy().into_owned();
            self.git(self.root(), &["clone", "-q", &bare, &mate_arg])?;
        }
        self.git(&mate, &["pull", "-q", "--ff-only", "origin", self.branch()])?;
        std::fs::write(mate.join(file), content).map_err(|why| format!("{file}: {why}"))?;
        self.git(&mate, &["add", "--", file])?;
        self.git(&mate, &["commit", "-q", "-m", &format!("change {file}")])?;
        self.git(
            &mate,
            &["push", "-q", "origin", &format!("HEAD:{}", self.main_ref())],
        )?;
        self.git(&mate, &["rev-parse", "HEAD"])
    }

    /// A teammate's commit of `count` distinct files of `size` random-ish
    /// bytes — enough objects that git reports progress while the work
    /// clone receives them.
    pub fn many_objects_on_remote(&self, count: usize, size: usize) -> Result<String, String> {
        let mate = self.root().join("mate-many");
        let bare = self.bare().to_string_lossy().into_owned();
        let mate_arg = mate.to_string_lossy().into_owned();
        self.git(self.root(), &["clone", "-q", &bare, &mate_arg])?;
        let dir = mate.join("bulk");
        std::fs::create_dir_all(&dir).map_err(|why| format!("bulk: {why}"))?;
        // A cheap generator with no repetition git could delta away.
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        for n in 0..count {
            let mut bytes = Vec::with_capacity(size);
            while bytes.len() < size {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                bytes.extend_from_slice(&seed.to_le_bytes());
            }
            std::fs::write(dir.join(format!("{n}.bin")), &bytes)
                .map_err(|why| format!("bulk/{n}.bin: {why}"))?;
        }
        self.git(&mate, &["add", "--", "bulk"])?;
        self.git(&mate, &["commit", "-q", "-m", "bulk"])?;
        self.git(
            &mate,
            &["push", "-q", "origin", &format!("HEAD:{}", self.main_ref())],
        )?;
        self.git(&mate, &["rev-parse", "HEAD"])
    }

    /// The first tracked file of the shape, the one a teammate's change
    /// will touch.
    pub fn a_tracked_file(&self) -> Result<String, String> {
        run_git(self.work(), &["ls-files"])?
            .lines()
            .next()
            .map(str::to_owned)
            .ok_or_else(|| "the shape has no tracked file".to_owned())
    }
}

/// Repository truth on both sides, plus what the loop drew, entirely from
/// git, the file system and the remote's own counters.
#[derive(Clone, Debug, Default)]
pub struct Truth {
    /// The work clone (`m3_walk::git_truth`).
    pub work: RepoTruth,
    /// The bare's `for-each-ref`: name → object.
    pub bare: BTreeMap<String, String>,
    /// The work clone's undo journal, as written; empty before any write.
    pub journal: Vec<JournalEntry>,
    /// Every distinct status line the loop drew so far, in order.
    pub statuses: Vec<String>,
    /// The HTTP remote's `(authorised, rejected)` request counts.
    pub http: Option<(usize, usize)>,
    /// The hung remote's connection count.
    pub hung_connections: Option<usize>,
}

/// Read both repositories now.
pub fn truth(fixture: &Fixture, statuses: &[String]) -> Result<Truth, String> {
    let work = git_truth(fixture.work())?;
    let bare = run_git(
        fixture.bare(),
        &["for-each-ref", "--format=%(refname) %(objectname)"],
    )?
    .lines()
    .filter_map(|line| {
        let (name, oid) = line.split_once(' ')?;
        Some((name.to_owned(), oid.to_owned()))
    })
    .collect();
    Ok(Truth {
        work,
        bare,
        journal: journal(fixture.work())?,
        statuses: statuses.to_vec(),
        http: fixture.http.as_ref().map(HttpRemote::requests),
        hung_connections: fixture.hung.as_ref().map(HungRemote::connections),
    })
}

/// The journal's entries, or none when no write has happened yet.
pub fn journal(work: &Path) -> Result<Vec<JournalEntry>, String> {
    let path = work.join(".git/graft/undo-journal.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    graft_ops::journal::Journal::open(&path)
        .map(|journal| journal.entries().to_vec())
        .map_err(|why| format!("the journal: {why}"))
}

/// What moved between two truths, 20 lines at most: the work clone's
/// differences first (`m3_walk::diff_truth`), then the bare's refs, then
/// the journal's length.
#[must_use]
pub fn diff_truth(before: &Truth, after: &Truth) -> Vec<String> {
    let mut out = diff_work(&before.work, &after.work);
    for (name, was) in &before.bare {
        match after.bare.get(name) {
            Some(now) if now != was => out.push(format!("bare {name}: {was} → {now}")),
            Some(_) => {}
            None => out.push(format!("bare {name}: gone (was {was})")),
        }
    }
    for (name, now) in &after.bare {
        if !before.bare.contains_key(name) {
            out.push(format!("bare {name}: new {now}"));
        }
    }
    if before.journal.len() != after.journal.len() {
        out.push(format!(
            "journal: {} → {} entries",
            before.journal.len(),
            after.journal.len()
        ));
    }
    out.truncate(20);
    out
}

/// The truths a run recorded: after setup, and at every `Mark`.
#[derive(Default)]
pub struct Marks {
    /// After `setup`, before the app opened.
    pub before: Truth,
    /// At each `Step::Mark`.
    pub at: BTreeMap<&'static str, Truth>,
}

impl Marks {
    /// The truth recorded under `name`.
    pub fn get(&self, name: &str) -> Result<&Truth, String> {
        self.at
            .get(name)
            .ok_or_else(|| format!("no mark named {name}"))
    }
}

/// Something the walk waits for before the next key.
#[derive(Clone, Copy, Debug)]
pub enum Wait {
    /// Rows, refs, the working tree and the remote facts have landed and no
    /// re-read is in flight — everything `f`/`p`/`P` decide from.
    Ready,
    /// Git's credential question is open.
    CredentialPrompt,
    /// The hung remote has git's connection.
    Connected,
    /// The journal on screen has an entry of this kind and no re-read is in
    /// flight (`u`'s wording reads it).
    JournalHas(OperationKind),
}

/// One thing the walk does to the running app, in order.
pub enum Step {
    /// Press one key.
    Key(KeyPress),
    /// Type text, one character key at a time — a subject, a username.
    Type(&'static str),
    /// Type a password into the credential prompt. Never echoed: the step
    /// is not printed, and [`no_secret_leaked`] proves the bytes reached no
    /// event, status line or journal.
    TypeSecret(&'static str),
    /// Feed job answers until the condition holds.
    WaitFor(Wait),
    /// A write was queued by the last key: run it to its answer and insist
    /// it succeeded (`Status::Done`).
    Write(&'static str),
    /// Run whatever is in flight to its answer, whatever it was — for a
    /// refusal or a rejection.
    Settle(&'static str),
    /// The box `P` opened: `→` off `Cancel`, `Enter`.
    Confirm,
    /// `Tab` until `[ Commit ]` has focus, then `Enter` (ADR-0034's path).
    CommitButton,
    /// `Esc` on the running write, then its answer; it must clear within
    /// 5 s of the key.
    Cancel(&'static str),
    /// `u` until the line says `nothing to undo` (ten at most).
    UndoAll,
    /// Change a repository out of band, as a teammate or an editor would.
    Shell(Fix),
    /// Record truth under this name for `expect`.
    Mark(&'static str),
    /// Check a fact only the UI state holds.
    See(&'static str, See),
}

/// When the pty script writes the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    /// Once the screen has stopped changing.
    Settled,
    /// The moment the expected text appears — for a footer that is gone a
    /// second later.
    Appears,
}

/// A pty capture of this scenario (what the user judges by eye).
pub struct Capture {
    /// `m4-<nn>-<what>`, the file's stem under `docs/shots/`.
    pub stem: &'static str,
    /// Driven headless first, so the captured run starts from their result.
    pub headless: Vec<Step>,
    /// The captured run's `Harness::prelude`.
    pub keys: Vec<KeyPress>,
    /// Typed after `keys`, once the credential prompt is open — appended to
    /// the prelude by `--capture`, printed by nothing.
    pub secret: Option<&'static str>,
    /// Text the frame must contain, or the capture says it did not.
    pub on_screen: &'static str,
    /// When to take the frame.
    pub when: When,
}

/// One scenario of the walk.
pub struct Scenario {
    /// Its name on the command line and in `TESTING.md`.
    pub name: &'static str,
    /// The corpus shape the bare is cloned from.
    pub shape: Shape,
    /// What `origin` is.
    pub remote: RemoteKind,
    /// The state before the app opens, scripted with git and plain files.
    pub setup: Fix,
    /// What the walk does.
    pub steps: Vec<Step>,
    /// What must then be true, by git on both sides.
    pub expect: Expect,
    /// Its capture, when it has a visual result.
    pub capture: Option<Capture>,
}

// ───────────────────────────── running one ─────────────────────────────

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

/// Neither password is in the journal on disk, nor in any event or status
/// line the loop saw. Checked after every scenario, not only the HTTP ones:
/// the rule is about the whole app.
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

fn check(condition: bool, what: impl FnOnce() -> String) -> Result<(), String> {
    if condition { Ok(()) } else { Err(what()) }
}

/// Everything the loop saw, so a scenario can prove what never appeared.
#[derive(Default)]
pub struct Recorded {
    /// One `{event:?}` per event, progress texts and prompts included.
    pub events: Vec<String>,
    /// One line per distinct status the loop drew, in order.
    pub statuses: Vec<String>,
}

impl Recorded {
    fn note(&mut self, state: &AppState) {
        let line = status_text(state);
        if self.statuses.last() != Some(&line) {
            self.statuses.push(line);
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
        loop {
            if done(&state) {
                return Ok(state);
            }
            let event = self
                .app
                .events
                .recv_timeout(PATIENCE)
                .map_err(|why| format!("waiting for {what}: {why}"))?;
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
            Status::Done(_) => Ok(state),
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
        && matches!(state.remotes, Loading::Ready(_))
        && state.remote.reading.is_none()
        && state.operating.is_none()
        && state
            .history
            .as_ref()
            .and_then(|history| history.window.as_ref())
            .is_some_and(|window| !window.rows.is_empty())
}

/// The status line's words, whatever its kind.
fn status_text(state: &AppState) -> String {
    match &state.status {
        Status::Ready => String::new(),
        Status::Copied { short } => format!("copied {short}"),
        Status::Failed(text) | Status::Sticky(text) | Status::Busy(text) | Status::Done(text) => {
            text.clone()
        }
    }
}

// ───────────────────────────── step vocabulary ─────────────────────────────

fn key(code: KeyCode) -> Step {
    Step::Key(KeyPress::plain(code))
}

fn ch(c: char) -> KeyCode {
    KeyCode::Char(c)
}

fn plain(codes: &[KeyCode]) -> Vec<KeyPress> {
    codes.iter().map(|code| KeyPress::plain(*code)).collect()
}

fn ctrl(c: char) -> KeyPress {
    KeyPress {
        code: KeyCode::Char(c),
        modifiers: Modifiers {
            control: true,
            ..Modifiers::default()
        },
    }
}

fn chain(parts: Vec<Vec<Step>>) -> Vec<Step> {
    parts.into_iter().flatten().collect()
}

/// `c`, a subject, `[ Commit ]`, the write, then everything re-read.
fn commit_by_key(subject: &'static str) -> Vec<Step> {
    vec![
        key(ch('c')),
        Step::Type(subject),
        Step::CommitButton,
        Step::Write("the commit"),
        Step::WaitFor(Wait::Ready),
    ]
}

/// `P`, through the box, to a successful push.
fn push_by_key() -> Vec<Step> {
    vec![key(ch('P')), Step::Confirm, Step::Write("the push")]
}

/// Stage a new file before the app opens, so its first status read has it
/// and `c` has something to commit.
fn stage_pushed_txt(fixture: &Fixture) -> Result<(), String> {
    std::fs::write(fixture.work().join("pushed.txt"), "pushed\n")
        .map_err(|why| format!("pushed.txt: {why}"))?;
    run_git(fixture.work(), &["add", "pushed.txt"])?;
    Ok(())
}

fn teammate_commits(fixture: &Fixture) -> Result<(), String> {
    fixture
        .local
        .commit_on_remote("theirs")
        .map(drop)
        .map_err(|why| format!("the teammate's commit: {why}"))
}

fn ours_locally(fixture: &Fixture) -> Result<(), String> {
    fixture
        .local
        .commit_locally("ours")
        .map(drop)
        .map_err(|why| format!("the local commit: {why}"))
}

fn amend_locally(fixture: &Fixture) -> Result<(), String> {
    fixture
        .git(
            fixture.work(),
            &["commit", "--amend", "-q", "-m", "amended"],
        )
        .map(drop)
}

fn sticky_containing(state: &AppState, texts: &[&str]) -> Result<(), String> {
    let Status::Sticky(line) = &state.status else {
        return Err(format!("not a sticky failure: {:?}", state.status));
    };
    for text in texts {
        check(line.contains(text), || {
            format!("the line lacks `{text}`: {line}")
        })?;
    }
    Ok(())
}

fn done_containing(state: &AppState, text: &str) -> Result<(), String> {
    let Status::Done(line) = &state.status else {
        return Err(format!("not a Done line: {:?}", state.status));
    };
    check(line.contains(text), || {
        format!("the line lacks `{text}`: {line}")
    })
}

fn box_title_starts(state: &AppState, text: &str) -> Result<(), String> {
    let title = state
        .confirm
        .as_ref()
        .map(|pending| pending.view.title.clone())
        .ok_or_else(|| format!("no box is open; status {:?}", state.status))?;
    check(title.starts_with(text), || {
        format!("the box is titled `{title}`, not `{text}…`")
    })
}

fn no_box(state: &AppState) -> Result<(), String> {
    check(state.confirm.is_none(), || {
        format!(
            "a box opened: {:?}",
            state.confirm.as_ref().map(|pending| &pending.view.title)
        )
    })
}

fn prompt_is(state: &AppState, masked: bool, naming: &str) -> Result<(), String> {
    let prompt = state
        .credential
        .as_ref()
        .ok_or_else(|| format!("no credential prompt is open; status {:?}", state.status))?;
    check(prompt.masked == masked, || {
        format!("the prompt's mask is {}, wanted {masked}", prompt.masked)
    })?;
    let text = prompt.prompt.as_str();
    check(text.contains(naming), || {
        format!("the prompt does not name `{naming}`: {text}")
    })
}

fn no_prompt(state: &AppState) -> Result<(), String> {
    check(state.credential.is_none(), || {
        "the credential prompt is still open".to_owned()
    })
}

fn same_bare(what: &str, a: &Truth, b: &Truth) -> Result<(), String> {
    check(a.bare == b.bare, || {
        let mut lines = Vec::new();
        for (name, was) in &a.bare {
            if b.bare.get(name) != Some(was) {
                lines.push(format!("{name}: {was} → {:?}", b.bare.get(name)));
            }
        }
        for name in b.bare.keys() {
            if !a.bare.contains_key(name) {
                lines.push(format!("{name}: new"));
            }
        }
        format!("{what}: the bare moved: {}", lines.join(" | "))
    })
}

fn bare_ref<'a>(truth: &'a Truth, name: &str) -> Result<&'a str, String> {
    truth
        .bare
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("the bare has no {name}"))
}

fn work_ref<'a>(truth: &'a Truth, name: &str) -> Result<&'a str, String> {
    truth
        .work
        .refs
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("the work clone has no {name}"))
}

fn head(truth: &Truth) -> Result<&str, String> {
    truth
        .work
        .head
        .as_deref()
        .ok_or_else(|| "the work clone has no HEAD".to_owned())
}

fn last_entry(truth: &Truth) -> Result<&JournalEntry, String> {
    truth
        .journal
        .last()
        .ok_or_else(|| "the journal is empty".to_owned())
}

fn aborted_cancelled(entry: &JournalEntry) -> Result<(), String> {
    check(
        matches!(&entry.status, EntryStatus::Aborted { reason } if reason.contains("cancelled")),
        || format!("the entry is {:?}, not Aborted(cancelled)", entry.status),
    )
}

/// Every `*.lock` under `.git`.
fn locks(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(locks(&path));
        } else if path.extension().is_some_and(|ext| ext == "lock") {
            found.push(path);
        }
    }
    found
}

// ───────────────────────────── the scenarios ─────────────────────────────

/// Every scenario, in the order `TESTING.md` lists them.
#[must_use]
pub fn scenarios() -> Vec<Scenario> {
    let mut all = fetching();
    all.extend(pulling());
    all.extend(pushing());
    all.extend(rejections());
    all.extend(credentials());
    all.extend(hangs_progress_and_undo());
    all
}

fn fetching() -> Vec<Scenario> {
    vec![fetch_brings_new_commits(), fetch_prunes_deleted_branch()]
}

fn fetch_brings_new_commits() -> Scenario {
    Scenario {
        name: "fetch-brings-new-commits",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: teammate_commits,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('f')),
            Step::Write("the fetch"),
            Step::See("f says fetched", |state| done_containing(state, "fetched")),
            Step::Mark("fetched"),
            key(ch('u')),
            Step::Write("the undo"),
            Step::See("u says nothing to undo", |state| {
                done_containing(state, "nothing to undo")
            }),
            Step::Mark("after-u"),
        ],
        expect: |fixture, marks| {
            let fetched = marks.get("fetched")?;
            let theirs = bare_ref(&marks.before, &fixture.main_ref())?;
            check(
                work_ref(fetched, &fixture.tracking_ref())? == theirs,
                || {
                    format!(
                        "the tracking ref is {:?}, the bare's tip {theirs}",
                        fetched.work.refs.get(&fixture.tracking_ref())
                    )
                },
            )?;
            check(head(fetched)? == head(&marks.before)?, || {
                "a fetch moved HEAD".to_owned()
            })?;
            let entry = last_entry(fetched)?;
            check(
                entry.kind == OperationKind::Fetch && entry.status == EntryStatus::Committed,
                || format!("the last entry is {:?} {:?}", entry.kind, entry.status),
            )?;
            let after = marks.get("after-u")?;
            check(after.work.refs == fetched.work.refs, || {
                format!("u moved a ref: {:?}", diff_work(&fetched.work, &after.work))
            })?;
            same_bare("u after a fetch", fetched, after)
        },
        capture: None,
    }
}

fn fetch_prunes_deleted_branch() -> Scenario {
    Scenario {
        name: "fetch-prunes-deleted-branch",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            fixture.git(fixture.bare(), &["branch", "gone"])?;
            fixture.git(fixture.work(), &["fetch", "-q", "origin"])?;
            fixture.git(fixture.bare(), &["branch", "-D", "gone"])?;
            Ok(())
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('f')),
            Step::Write("the fetch"),
            Step::Mark("fetched"),
        ],
        expect: |_, marks| {
            check(
                marks
                    .before
                    .work
                    .refs
                    .contains_key("refs/remotes/origin/gone"),
                || "setup: the work clone never had origin/gone".to_owned(),
            )?;
            let fetched = marks.get("fetched")?;
            check(
                !fetched.work.refs.contains_key("refs/remotes/origin/gone"),
                || "origin/gone survived the fetch: not pruned".to_owned(),
            )
        },
        capture: None,
    }
}

fn pulling() -> Vec<Scenario> {
    vec![
        pull_fast_forward(),
        pull_diverged_refused(),
        pull_dirty_file_refused(),
        pull_no_upstream_refused(),
    ]
}

/// `pull-fast-forward`'s verdict.
fn pull_fast_forward_expect(fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let before = &marks.before;
    let theirs = bare_ref(before, &fixture.main_ref())?.to_owned();
    let old = head(before)?.to_owned();
    let pulled = marks.get("pulled")?;
    check(head(pulled)? == theirs, || {
        format!("main is {:?}, not the bare's {theirs}", pulled.work.head)
    })?;
    let changed = run_git(fixture.bare(), &["diff", "--name-only", &old, &theirs])?;
    let file = changed
        .lines()
        .next()
        .ok_or("the teammate's commit changed no file")?
        .to_owned();
    let their_bytes = run_git(fixture.bare(), &["show", &format!("{theirs}:{file}")])?;
    check(
        pulled.work.files.get(Path::new(&file)).map(Vec::as_slice) == Some(their_bytes.as_bytes()),
        || format!("{file}'s bytes are not the teammate's"),
    )?;
    let undone = marks.get("undone")?;
    check(head(undone)? == old, || {
        format!("u left main at {:?}, not {old}", undone.work.head)
    })?;
    check(undone.work.files == before.work.files, || {
        format!(
            "u did not restore the files: {:?}",
            diff_work(&before.work, &undone.work)
        )
    })?;
    check(work_ref(undone, &fixture.tracking_ref())? == theirs, || {
        "u moved the tracking ref".to_owned()
    })?;
    same_bare("u after a pull", pulled, undone)?;
    let redone = marks.get("redone")?;
    check(head(redone)? == theirs, || {
        format!("U left main at {:?}, not {theirs}", redone.work.head)
    })
}

fn pull_fast_forward() -> Scenario {
    Scenario {
        name: "pull-fast-forward",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: teammate_commits,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('p')),
            Step::Write("the pull"),
            Step::See("p says pulled", |state| {
                done_containing(state, "pulled origin/")
            }),
            Step::Mark("pulled"),
            key(ch('u')),
            Step::Write("the undo"),
            Step::Mark("undone"),
            key(ch('U')),
            Step::Write("the redo"),
            Step::Mark("redone"),
        ],
        expect: pull_fast_forward_expect,
        capture: None,
    }
}

fn pull_diverged_refused() -> Scenario {
    Scenario {
        name: "pull-diverged-refused",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            teammate_commits(fixture)?;
            ours_locally(fixture)
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('p')),
            Step::Settle("the refused pull"),
            Step::See("the line names merge/rebase and the command", |state| {
                sticky_containing(
                    state,
                    &[
                        "pull stopped:",
                        "diverged",
                        "merge or rebase",
                        "git pull --rebase",
                    ],
                )
            }),
            Step::Mark("refused"),
        ],
        expect: |fixture, marks| {
            let refused = marks.get("refused")?;
            check(head(refused)? == head(&marks.before)?, || {
                "main moved".to_owned()
            })?;
            let theirs = bare_ref(&marks.before, &fixture.main_ref())?;
            check(
                work_ref(refused, &fixture.tracking_ref())? == theirs,
                || "the tracking ref did not advance to the teammate's tip".to_owned(),
            )?;
            same_bare("a refused pull", &marks.before, refused)
        },
        capture: None,
    }
}

fn pull_dirty_file_refused() -> Scenario {
    Scenario {
        name: "pull-dirty-file-refused",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            let file = fixture.a_tracked_file()?;
            fixture.change_on_remote(&file, "theirs\n")?;
            std::fs::write(fixture.work().join(&file), "mine, uncommitted\n")
                .map_err(|why| format!("{file}: {why}"))?;
            Ok(())
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('p')),
            Step::Settle("the refused pull"),
            Step::See("the line names the file", |state| {
                sticky_containing(
                    state,
                    &[
                        "pull refused: would overwrite",
                        "(uncommitted)",
                        "commit or stash first",
                    ],
                )
            }),
            Step::Mark("refused"),
        ],
        expect: |fixture, marks| {
            let refused = marks.get("refused")?;
            let file = fixture.a_tracked_file()?;
            check(
                refused.work.files.get(Path::new(&file)).map(Vec::as_slice)
                    == Some(b"mine, uncommitted\n".as_slice()),
                || format!("{file} is not byte for byte the uncommitted edit"),
            )?;
            check(refused.work.files == marks.before.work.files, || {
                format!(
                    "a file changed: {:?}",
                    diff_work(&marks.before.work, &refused.work)
                )
            })?;
            check(head(refused)? == head(&marks.before)?, || {
                "main moved".to_owned()
            })
        },
        capture: None,
    }
}

fn pull_no_upstream_refused() -> Scenario {
    Scenario {
        name: "pull-no-upstream-refused",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            fixture
                .git(fixture.work(), &["branch", "--unset-upstream"])
                .map(drop)
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('p')),
            Step::See("p is refused in words", |state| {
                check(status_text(state).contains("has no upstream"), || {
                    format!("the line: {:?}", state.status)
                })?;
                check(state.operating.is_none(), || {
                    "a write was queued".to_owned()
                })
            }),
            Step::Mark("refused"),
        ],
        expect: |_, marks| {
            let refused = marks.get("refused")?;
            let moved = diff_truth(&marks.before, refused);
            check(moved.is_empty(), || format!("something changed: {moved:?}"))?;
            check(refused.journal.is_empty(), || {
                "a refusal was journaled".to_owned()
            })
        },
        capture: None,
    }
}

fn pushing() -> Vec<Scenario> {
    vec![push_plain(), push_sets_upstream(), push_detached_refused()]
}

/// `push-plain`'s verdict.
fn push_plain_expect(fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let before = &marks.before;
    let old = head(before)?.to_owned();
    let pushed = marks.get("pushed")?;
    let committed = head(pushed)?.to_owned();
    check(committed != old, || "the commit did not land".to_owned())?;
    check(bare_ref(pushed, &fixture.main_ref())? == committed, || {
        format!(
            "the bare's main is {:?}, not {committed}",
            pushed.bare.get(&fixture.main_ref())
        )
    })?;
    let entry = pushed
        .journal
        .iter()
        .find(|entry| entry.kind == OperationKind::Push)
        .ok_or("no Push entry in the journal")?;
    let tracking = fixture.tracking_ref();
    let before_ref = entry.refs_before.get(&tracking).copied().flatten();
    let after_ref = entry.refs_after.get(&tracking).copied().flatten();
    check(
        before_ref.map(|oid| oid.to_string()) == Some(old.clone())
            && after_ref.map(|oid| oid.to_string()) == Some(committed.clone()),
        || format!("the Push entry's {tracking}: {before_ref:?} → {after_ref:?}"),
    )?;
    let undone = marks.get("undone")?;
    check(head(undone)? == old, || {
        format!("u left main at {:?}, not {old}", undone.work.head)
    })?;
    same_bare("u after a push", pushed, undone)?;
    let redone = marks.get("redone")?;
    check(head(redone)? == committed, || {
        format!("U left main at {:?}, not {committed}", redone.work.head)
    })
}

fn push_plain() -> Scenario {
    Scenario {
        name: "push-plain",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: stage_pushed_txt,
        steps: chain(vec![
            vec![Step::WaitFor(Wait::Ready)],
            commit_by_key("to push"),
            vec![
                key(ch('P')),
                Step::See("the plain push box", |state| {
                    box_title_starts(state, "Push ")
                }),
                Step::Confirm,
                Step::Write("the push"),
                Step::See("P says pushed", |state| done_containing(state, "pushed ")),
                Step::Mark("pushed"),
                Step::WaitFor(Wait::JournalHas(OperationKind::Push)),
                key(ch('u')),
                Step::Write("the undo"),
                Step::See("u says origin still has it", |state| {
                    done_containing(state, " still has it")
                }),
                Step::Mark("undone"),
                key(ch('U')),
                Step::Write("the redo"),
                Step::Mark("redone"),
            ],
        ]),
        expect: push_plain_expect,
        capture: Some(Capture {
            stem: "m4-01-push-box-plain",
            headless: chain(vec![
                vec![Step::WaitFor(Wait::Ready)],
                commit_by_key("to push"),
            ]),
            keys: plain(&[ch('P')]),
            secret: None,
            on_screen: "→ origin/",
            when: When::Settled,
        }),
    }
}

fn push_sets_upstream() -> Scenario {
    Scenario {
        name: "push-sets-upstream",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            fixture
                .git(fixture.work(), &["checkout", "-q", "-b", "feature"])
                .map(drop)
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::See("the box says it sets the upstream", |state| {
                let view = &state.confirm.as_ref().ok_or("no box")?.view;
                check(
                    view.message
                        .iter()
                        .any(|line| line.contains("sets upstream")),
                    || format!("the box: {:?} / {:?}", view.title, view.message),
                )
            }),
            Step::Confirm,
            Step::Write("the push"),
            Step::Mark("pushed"),
        ],
        expect: |fixture, marks| {
            let pushed = marks.get("pushed")?;
            let tip = head(pushed)?;
            check(bare_ref(pushed, "refs/heads/feature")? == tip, || {
                "the bare's feature is not the local tip".to_owned()
            })?;
            let remote = run_git(fixture.work(), &["config", "branch.feature.remote"])?;
            let merge = run_git(fixture.work(), &["config", "branch.feature.merge"])?;
            check(
                remote.trim() == "origin" && merge.trim() == "refs/heads/feature",
                || format!("branch.feature.remote={remote:?} merge={merge:?}"),
            )
        },
        capture: None,
    }
}

fn push_detached_refused() -> Scenario {
    Scenario {
        name: "push-detached-refused",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            fixture
                .git(fixture.work(), &["checkout", "-q", "--detach"])
                .map(drop)
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::See("P is refused in words, no box", |state| {
                no_box(state)?;
                check(status_text(state).contains("detached"), || {
                    format!("the line: {:?}", state.status)
                })
            }),
            Step::Mark("refused"),
        ],
        expect: |_, marks| {
            let refused = marks.get("refused")?;
            same_bare("a refused push", &marks.before, refused)?;
            check(refused.journal.is_empty(), || {
                "a refusal was journaled".to_owned()
            })
        },
        capture: None,
    }
}

fn rejections() -> Vec<Scenario> {
    vec![push_rejected_then_lease(), push_lease_refused_stale()]
}

fn push_rejected_then_lease() -> Scenario {
    Scenario {
        name: "push-rejected-then-lease",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: amend_locally,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::Settle("the rejected push"),
            Step::See("the rejection stays, with the hint", |state| {
                sticky_containing(
                    state,
                    &[
                        "push rejected: origin/",
                        "has commits you don't",
                        "p to pull first, or P to force with lease",
                    ],
                )
            }),
            Step::Mark("rejected"),
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::See("the lease box", |state| {
                box_title_starts(state, "Force-push ")
            }),
            Step::Confirm,
            Step::Write("the lease push"),
            Step::Mark("forced"),
        ],
        expect: |fixture, marks| {
            let rejected = marks.get("rejected")?;
            same_bare("a rejected push", &marks.before, rejected)?;
            let amended = head(&marks.before)?;
            let forced = marks.get("forced")?;
            check(bare_ref(forced, &fixture.main_ref())? == amended, || {
                "the lease push did not land".to_owned()
            })
        },
        // The rejection `P` remembers lives in the running app, so the
        // captured run must be rejected itself: `P → ⏎` (rejected), then
        // `P` again opens the lease box.
        capture: Some(Capture {
            stem: "m4-02-push-box-lease",
            headless: Vec::new(),
            keys: plain(&[ch('P'), KeyCode::Right, KeyCode::Enter, ch('P')]),
            secret: None,
            on_screen: "Force-push",
            when: When::Settled,
        }),
    }
}

fn push_lease_refused_stale() -> Scenario {
    Scenario {
        name: "push-lease-refused-stale",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            teammate_commits(fixture)?;
            ours_locally(fixture)
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::Settle("the rejected push"),
            Step::See("rejected", |state| {
                sticky_containing(state, &["push rejected"])
            }),
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::See("the lease box", |state| {
                box_title_starts(state, "Force-push ")
            }),
            Step::Confirm,
            Step::Settle("the stale lease push"),
            Step::See("the line names the stale lease", |state| {
                sticky_containing(state, &["push refused:", "stale lease", "f to fetch"])
            }),
            Step::Mark("refused"),
        ],
        expect: |_, marks| same_bare("a stale lease", &marks.before, marks.get("refused")?),
        capture: Some(Capture {
            stem: "m4-06-rejected-sticky",
            headless: Vec::new(),
            keys: plain(&[ch('P'), KeyCode::Right, KeyCode::Enter]),
            secret: None,
            on_screen: "push rejected",
            when: When::Settled,
        }),
    }
}

fn credentials() -> Vec<Scenario> {
    vec![
        wrong_password_then_right(),
        password_prompt_esc_cancels(),
        username_then_password(),
    ]
}

fn wrong_password_then_right() -> Scenario {
    Scenario {
        name: "wrong-password-then-right",
        shape: Shape::Linear,
        remote: RemoteKind::Http { anonymous: false },
        setup: ours_locally,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::WaitFor(Wait::CredentialPrompt),
            Step::See("the prompt is masked", |state| {
                prompt_is(state, true, "assword")
            }),
            Step::TypeSecret(WRONG),
            key(KeyCode::Enter),
            Step::Settle("the refused push"),
            // The target git names is the whole URL
            // (`http://127.0.0.1:<port>/repo.git/`), not the `host:port` the
            // design note pictured — recorded in the ticket, asserted as
            // built.
            Step::See("authentication failed, with the hint", |state| {
                sticky_containing(
                    state,
                    &["authentication failed for ", "127.0.0.1:", "P to try again"],
                )
            }),
            Step::Mark("refused"),
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::WaitFor(Wait::CredentialPrompt),
            Step::TypeSecret(PASSWORD),
            key(KeyCode::Enter),
            Step::Write("the push"),
            Step::Mark("pushed"),
        ],
        expect: |fixture, marks| {
            let refused = marks.get("refused")?;
            same_bare("a wrong password", &marks.before, refused)?;
            let (ok, rejected) = refused.http.ok_or("no HTTP counters")?;
            check(ok == 0 && rejected >= 1, || {
                format!("with the wrong password: {ok} authorised, {rejected} rejected")
            })?;
            let pushed = marks.get("pushed")?;
            let tip = head(&marks.before)?;
            check(bare_ref(pushed, &fixture.main_ref())? == tip, || {
                "the right password did not push".to_owned()
            })?;
            let (ok, rejected) = pushed.http.ok_or("no HTTP counters")?;
            check(ok >= 1 && rejected >= 1, || {
                format!("in all: {ok} authorised, {rejected} rejected")
            })
        },
        capture: Some(Capture {
            stem: "m4-03-credential-masked",
            headless: Vec::new(),
            keys: plain(&[ch('P'), KeyCode::Right, KeyCode::Enter]),
            secret: Some(PASSWORD),
            on_screen: "Password for",
            when: When::Settled,
        }),
    }
}

fn password_prompt_esc_cancels() -> Scenario {
    Scenario {
        name: "password-prompt-esc-cancels",
        shape: Shape::Linear,
        remote: RemoteKind::Http { anonymous: false },
        setup: ours_locally,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::WaitFor(Wait::CredentialPrompt),
            key(KeyCode::Escape),
            Step::See("Esc closed the prompt", no_prompt),
            Step::Settle("the cancelled push"),
            Step::Mark("cancelled"),
        ],
        expect: |_, marks| {
            let cancelled = marks.get("cancelled")?;
            same_bare("a cancelled push", &marks.before, cancelled)?;
            let entry = last_entry(cancelled)?;
            check(entry.kind == OperationKind::Push, || {
                format!("the last entry is {:?}", entry.kind)
            })?;
            aborted_cancelled(entry)?;
            let (ok, _) = cancelled.http.ok_or("no HTTP counters")?;
            check(ok == 0, || format!("{ok} requests were authorised"))
        },
        capture: None,
    }
}

fn username_then_password() -> Scenario {
    Scenario {
        name: "username-then-password",
        shape: Shape::Linear,
        remote: RemoteKind::Http { anonymous: true },
        setup: ours_locally,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::WaitFor(Wait::CredentialPrompt),
            Step::See("the username is asked in the open", |state| {
                prompt_is(state, false, "sername")
            }),
            Step::Type(USER),
            key(KeyCode::Enter),
            Step::WaitFor(Wait::CredentialPrompt),
            Step::See("the password is asked masked", |state| {
                prompt_is(state, true, "assword")
            }),
            Step::TypeSecret(PASSWORD),
            key(KeyCode::Enter),
            Step::Write("the push"),
            Step::Mark("pushed"),
        ],
        expect: |fixture, marks| {
            let pushed = marks.get("pushed")?;
            let tip = head(&marks.before)?;
            check(bare_ref(pushed, &fixture.main_ref())? == tip, || {
                "the push did not land".to_owned()
            })
        },
        capture: Some(Capture {
            stem: "m4-04-credential-username",
            headless: Vec::new(),
            keys: plain(&[
                ch('P'),
                KeyCode::Right,
                KeyCode::Enter,
                ch('t'),
                ch('e'),
                ch('s'),
                ch('t'),
                ch('e'),
                ch('r'),
            ]),
            secret: None,
            on_screen: "Username for",
            when: When::Settled,
        }),
    }
}

fn hangs_progress_and_undo() -> Vec<Scenario> {
    vec![
        hung_remote_esc_cancels(),
        progress_reaches_the_footer(),
        u_never_touches_the_remote(),
        journal_rows(),
        no_index_write_on_fetch(),
    ]
}

fn hung_remote_esc_cancels() -> Scenario {
    Scenario {
        name: "hung-remote-esc-cancels",
        shape: Shape::Linear,
        remote: RemoteKind::Hung,
        setup: |_| Ok(()),
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('f')),
            Step::See("the fetch is running", |state| {
                check(state.operating.is_some(), || "f queued nothing".to_owned())
            }),
            Step::WaitFor(Wait::Connected),
            Step::Cancel("the cancelled fetch"),
            Step::See("the line says cancelled", |state| {
                done_containing(state, "fetch all remotes cancelled")
            }),
            Step::Mark("cancelled"),
        ],
        expect: |fixture, marks| {
            let cancelled = marks.get("cancelled")?;
            let entry = last_entry(cancelled)?;
            check(entry.kind == OperationKind::Fetch, || {
                format!("the last entry is {:?}", entry.kind)
            })?;
            aborted_cancelled(entry)?;
            let left = locks(&fixture.work().join(".git"));
            check(left.is_empty(), || format!("locks left: {left:?}"))?;
            check(cancelled.hung_connections == Some(1), || {
                format!("{:?} connections", cancelled.hung_connections)
            })
        },
        capture: None,
    }
}

fn progress_reaches_the_footer() -> Scenario {
    Scenario {
        name: "progress-reaches-the-footer",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| fixture.many_objects_on_remote(400, 4096).map(drop),
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('f')),
            Step::Write("the fetch"),
            Step::Mark("fetched"),
        ],
        expect: |_, marks| {
            let fetched = marks.get("fetched")?;
            check(
                fetched.statuses.iter().any(|line| {
                    line.starts_with("fetch all remotes · ") && line.contains("objects")
                }),
                || format!("no progress line among {:?}", fetched.statuses),
            )
        },
        capture: Some(Capture {
            stem: "m4-05-progress-footer",
            headless: Vec::new(),
            keys: plain(&[ch('f')]),
            secret: None,
            on_screen: "objects",
            when: When::Appears,
        }),
    }
}

/// `u-never-touches-the-remote`'s verdict.
fn u_never_touches_the_remote_expect(fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let pushed = marks.get("pushed")?;
    let undone = marks.get("undone")?;
    same_bare("undoing everything", pushed, undone)?;
    check(head(undone)? == head(&marks.before)?, || {
        format!(
            "undoing everything left main at {:?}, not where it started",
            undone.work.head
        )
    })?;
    let theirs = bare_ref(&marks.before, &fixture.main_ref())?;
    check(work_ref(undone, &fixture.tracking_ref())? != theirs, || {
        "the tracking ref should have moved to the pushed tip".to_owned()
    })?;
    check(
        work_ref(undone, &fixture.tracking_ref())? == head(pushed)?,
        || "the tracking ref is not the pushed tip".to_owned(),
    )
}

fn u_never_touches_the_remote() -> Scenario {
    Scenario {
        name: "u-never-touches-the-remote",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            teammate_commits(fixture)?;
            stage_pushed_txt(fixture)
        },
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                key(ch('f')),
                Step::Write("the fetch"),
                Step::WaitFor(Wait::Ready),
                key(ch('p')),
                Step::Write("the pull"),
                Step::WaitFor(Wait::Ready),
            ],
            commit_by_key("ours"),
            push_by_key(),
            vec![
                Step::Mark("pushed"),
                Step::WaitFor(Wait::JournalHas(OperationKind::Push)),
                Step::UndoAll,
                Step::Mark("undone"),
            ],
        ]),
        expect: u_never_touches_the_remote_expect,
        capture: Some(Capture {
            stem: "m4-08-undo-after-push",
            headless: chain(vec![
                vec![
                    Step::WaitFor(Wait::Ready),
                    key(ch('p')),
                    Step::Write("the pull"),
                    Step::WaitFor(Wait::Ready),
                ],
                commit_by_key("ours"),
                push_by_key(),
                vec![Step::WaitFor(Wait::JournalHas(OperationKind::Push))],
            ]),
            keys: plain(&[ch('u')]),
            secret: None,
            on_screen: "still has it",
            when: When::Settled,
        }),
    }
}

/// `f`, `p`, a commit out of band, `P`: fetch and pull are the app's, the
/// commit is not journaled, so the pull is the next undo.
fn fetch_pull_commit_push() -> Vec<Step> {
    vec![
        Step::WaitFor(Wait::Ready),
        key(ch('f')),
        Step::Write("the fetch"),
        Step::WaitFor(Wait::Ready),
        key(ch('p')),
        Step::Write("the pull"),
        // Let the pull's own refresh land before `Ctrl+R`, as the screen
        // settling would for a person: a refresh pressed before the previous
        // one's `Started` events arrive leaves that history job uncancelled,
        // and two resident history jobs fill the harness's two-worker pool
        // (recorded in the ticket).
        Step::WaitFor(Wait::Ready),
        Step::Shell(ours_locally),
        Step::Key(ctrl('r')),
        Step::WaitFor(Wait::Ready),
        key(ch('P')),
        Step::Confirm,
        Step::Write("the push"),
        Step::WaitFor(Wait::JournalHas(OperationKind::Push)),
    ]
}

/// `journal-rows`'s verdict.
fn journal_rows_expect(_fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let kinds: Vec<OperationKind> = marks
        .get("shown")?
        .journal
        .iter()
        .map(|entry| entry.kind)
        .collect();
    check(
        kinds
            == [
                OperationKind::Fetch,
                OperationKind::Pull,
                OperationKind::Push,
            ],
        || format!("the journal holds {kinds:?}"),
    )
}

fn journal_rows() -> Scenario {
    Scenario {
        name: "journal-rows",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: teammate_commits,
        steps: chain(vec![
            fetch_pull_commit_push(),
            vec![
                key(ch('J')),
                Step::See("the journal's remote rows", |state| {
                    check(state.journal.shown, || "J opened nothing".to_owned())?;
                    let now = state.harness.clock.now_seconds();
                    let view = graft::view::journal_view(state, now);
                    let row = |kind: &str| {
                        view.rows
                            .iter()
                            .find(|row| row.description.as_str().starts_with(kind))
                            .ok_or_else(|| {
                                format!(
                                    "no {kind} row among {:?}",
                                    view.rows
                                        .iter()
                                        .map(|row| row.description.as_str())
                                        .collect::<Vec<_>>()
                                )
                            })
                    };
                    let push = row("push")?;
                    check(
                        push.summary
                            .as_str()
                            .contains("not undoable \u{2014} origin has it"),
                        || format!("the push row: {}", push.summary.as_str()),
                    )?;
                    let fetch = row("fetch")?;
                    check(fetch.summary.as_str().contains("not undoable"), || {
                        format!("the fetch row: {}", fetch.summary.as_str())
                    })?;
                    let pull = row("pull")?;
                    check(pull.mark == RowMark::NextUndo, || {
                        format!("the pull row is not marked ← u: {:?}", pull.mark)
                    })
                }),
                Step::Mark("shown"),
            ],
        ]),
        expect: journal_rows_expect,
        capture: Some(Capture {
            stem: "m4-07-journal-remote-rows",
            headless: fetch_pull_commit_push(),
            keys: plain(&[ch('J')]),
            secret: None,
            on_screen: "not undoable",
            when: When::Settled,
        }),
    }
}

fn no_index_write_on_fetch() -> Scenario {
    Scenario {
        name: "no-index-write-on-fetch",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: teammate_commits,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('f')),
            Step::Write("the fetch"),
            Step::Mark("fetched"),
        ],
        expect: |_, marks| {
            let fetched = marks.get("fetched")?;
            check(
                fetched.work.index_bytes == marks.before.work.index_bytes,
                || ".git/index was rewritten by a fetch".to_owned(),
            )
        },
        capture: None,
    }
}

// ───────────────────────────── the coverage table ─────────────────────────────

/// The remote captures' table for `docs/shots/35-coverage-matrix.txt`: one
/// row per capture stem, a column per size the walk captures at.
#[must_use]
pub fn remotes_matrix(workspace: &Path) -> String {
    const SIZES: [&str; 2] = ["80x24", "160x48"];
    let stems: Vec<&str> = scenarios()
        .iter()
        .filter_map(|scenario| scenario.capture.as_ref().map(|capture| capture.stem))
        .collect();
    let width = stems
        .iter()
        .map(|stem| stem.len())
        .max()
        .unwrap_or(0)
        .max(7);
    let mut out = String::new();
    out.push_str("\nRemotes coverage: capture × size (m4-remotes/07)\n");
    out.push_str("Files docs/shots/<stem>-<WxH>.txt from .scratch/m4-remotes/pty-walk-m4.py;\n");
    out.push_str(
        "C the file exists, — it does not. Unicode tier only; colour is never captured.\n\n",
    );
    out.push_str(&format!("{:width$}", "capture"));
    for size in SIZES {
        out.push_str(&format!("  {size:<8}"));
    }
    out.push('\n');
    for stem in stems {
        out.push_str(&format!("{stem:width$}"));
        for size in SIZES {
            let exists = workspace
                .join(format!("docs/shots/{stem}-{size}.txt"))
                .is_file();
            out.push_str(&format!("  {:<8}", if exists { "C" } else { "—" }));
        }
        out.push('\n');
    }
    out
}

/// The `--stems` line for a capture: stem, scenario, expected text, key
/// label, when — tab-separated, and never the secret.
#[must_use]
pub fn stems_line(scenario: &Scenario, capture: &Capture) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}",
        capture.stem,
        scenario.name,
        capture.on_screen,
        keys_label(&capture.keys),
        match capture.when {
            When::Settled => "settled",
            When::Appears => "appears",
        }
    )
}

/// The prelude a `--capture` run feeds: the keys, then the secret's
/// characters, when there is one.
#[must_use]
pub fn capture_prelude(capture: &Capture) -> Vec<KeyPress> {
    let mut keys = capture.keys.clone();
    if let Some(secret) = capture.secret {
        keys.extend(secret.chars().map(|c| KeyPress::plain(KeyCode::Char(c))));
    }
    keys
}

// ───────────────────────────── under cargo test ─────────────────────────────

#[cfg(test)]
mod every_scenario {
    use super::{Scenario, run_scenario, scenarios};

    fn named(name: &str) -> Scenario {
        scenarios()
            .into_iter()
            .find(|scenario| scenario.name == name)
            .unwrap_or_else(|| panic!("no scenario named {name}"))
    }

    fn passes(name: &str) {
        if let Err(why) = run_scenario(&named(name)) {
            panic!("FAIL {name}: {why}");
        }
    }

    macro_rules! walk {
        ($($test:ident => $name:literal),* $(,)?) => {
            $(#[test] fn $test() { passes($name); })*
            /// Every scenario the walk lists has a test here, so none can
            /// be added to the walk and skipped by the fast tier.
            #[test]
            fn every_scenario_has_a_test() {
                let listed: Vec<&str> = scenarios().iter().map(|s| s.name).collect();
                let tested = [$($name),*];
                assert_eq!(listed, tested);
            }
        };
    }

    walk! {
        fetch_brings_new_commits => "fetch-brings-new-commits",
        fetch_prunes_deleted_branch => "fetch-prunes-deleted-branch",
        pull_fast_forward => "pull-fast-forward",
        pull_diverged_refused => "pull-diverged-refused",
        pull_dirty_file_refused => "pull-dirty-file-refused",
        pull_no_upstream_refused => "pull-no-upstream-refused",
        push_plain => "push-plain",
        push_sets_upstream => "push-sets-upstream",
        push_detached_refused => "push-detached-refused",
        push_rejected_then_lease => "push-rejected-then-lease",
        push_lease_refused_stale => "push-lease-refused-stale",
        wrong_password_then_right => "wrong-password-then-right",
        password_prompt_esc_cancels => "password-prompt-esc-cancels",
        username_then_password => "username-then-password",
        hung_remote_esc_cancels => "hung-remote-esc-cancels",
        progress_reaches_the_footer => "progress-reaches-the-footer",
        u_never_touches_the_remote => "u-never-touches-the-remote",
        journal_rows => "journal-rows",
        no_index_write_on_fetch => "no-index-write-on-fetch",
    }

    /// Every capture names a distinct `m4-<nn>-…` stem, presses keys, says
    /// what must be on screen and prints no secret; the coverage table
    /// lists each stem.
    #[test]
    fn captures_are_named_keyed_and_listed_in_the_matrix() {
        use super::{SECRETS, remotes_matrix, stems_line};
        let all = scenarios();
        let captures: Vec<_> = all
            .iter()
            .filter_map(|s| s.capture.as_ref().map(|c| (s, c)))
            .collect();
        assert_eq!(captures.len(), 8, "the ticket names eight visual results");
        let mut stems: Vec<&str> = captures.iter().map(|(_, c)| c.stem).collect();
        stems.sort_unstable();
        stems.dedup();
        assert_eq!(stems.len(), captures.len(), "stems repeat");
        let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let matrix = remotes_matrix(&workspace);
        for (scenario, capture) in &captures {
            assert!(capture.stem.starts_with("m4-"), "{}", capture.stem);
            assert!(!capture.keys.is_empty(), "{} presses nothing", capture.stem);
            assert!(
                !capture.on_screen.is_empty(),
                "{} expects nothing",
                capture.stem
            );
            assert!(
                matrix.contains(capture.stem),
                "{} missing from the matrix",
                capture.stem
            );
            let line = stems_line(scenario, capture);
            assert_eq!(line.split('\t').count(), 5, "{line}");
            for secret in SECRETS {
                assert!(!line.contains(secret), "--stems would print a secret");
            }
        }
    }

    /// `--capture` starts from the scenario's setup and its headless prefix:
    /// the work clone is open and behind its remote, and the fixture — the
    /// HTTP server included — stays alive until dropped. The prelude it
    /// feeds is the keys, then the secret's characters, which `--stems`
    /// never prints.
    #[test]
    fn prepare_capture_leaves_the_work_clone_open_and_the_prelude_types_the_secret() {
        use super::{PASSWORD, capture_prelude, prepare_capture};
        let scenario = named("wrong-password-then-right");
        let capture = scenario.capture.as_ref().expect("it has a capture");
        let prelude = capture_prelude(capture);
        assert_eq!(prelude.len(), capture.keys.len() + PASSWORD.len());
        assert!(capture.headless.is_empty());
        let (fixture, work) = prepare_capture(&scenario).unwrap();
        assert!(work.join(".git").is_dir());
        assert!(
            fixture.http.is_some(),
            "the HTTP remote serves the captured run"
        );
        drop(fixture);
        assert!(!work.exists(), "dropping the fixture deletes the clone");
    }

    /// The truth diff is what a failing scenario prints: it names the bare's
    /// moves after the work clone's and stays short.
    #[test]
    fn diff_truth_names_the_bare_and_stops_at_20_lines() {
        use super::{Truth, diff_truth};
        let before = Truth {
            bare: [("refs/heads/main".to_owned(), "aaa".to_owned())].into(),
            ..Truth::default()
        };
        let mut after = before.clone();
        after
            .bare
            .insert("refs/heads/main".to_owned(), "bbb".to_owned());
        after
            .bare
            .insert("refs/heads/feature".to_owned(), "ccc".to_owned());
        after.work.files = (0..40).map(|n| (format!("f{n}").into(), vec![n])).collect();
        let diff = diff_truth(&before, &after);
        assert_eq!(diff.len(), 20);
        assert!(diff[0].starts_with("f0: new, 1 bytes"), "{}", diff[0]);
        let small = diff_truth(
            &before,
            &Truth {
                work: before.work.clone(),
                ..after.clone()
            },
        );
        assert_eq!(
            small,
            [
                "bare refs/heads/main: aaa → bbb",
                "bare refs/heads/feature: new ccc"
            ]
        );
    }
}
