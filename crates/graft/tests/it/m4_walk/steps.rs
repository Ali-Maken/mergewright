//! What a scenario is made of: the steps that press, wait, mark and shell
//! out, the capture a pty run replays, and the [`Scenario`] that binds them
//! to a fixture and a verdict.

use graft::update::AppState;
use graft_core::event::KeyPress;
use graft_core::journal::OperationKind;
use graft_fixtures::Shape;

use super::fixture::{Fixture, RemoteKind};
use super::truth::Marks;

/// A scenario's `setup` and a `Shell` step: git and plain files, against the
/// fixture (work clone, bare, or a teammate's clone).
pub type Fix = fn(&Fixture) -> Result<(), String>;
/// A fact only the UI state holds (a box's title, a prompt's mask).
pub type See = fn(&AppState) -> Result<(), String>;
/// The verdict: from git on both repositories and the marks the steps took.
pub type Expect = fn(&Fixture, &Marks) -> Result<(), String>;

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
