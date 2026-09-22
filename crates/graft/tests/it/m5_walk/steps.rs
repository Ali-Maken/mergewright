//! What a scenario is made of: the steps that press, wait, mark, claim and
//! shell out, the capture a pty run replays, and the [`Scenario`] that binds
//! them to a fixture and a verdict.

use graft::update::AppState;
use graft_core::event::KeyPress;
use graft_core::journal::OperationKind;
use graft_fixtures::Shape;

use super::fixture::Fixture;
use super::truth::Marks;

/// A scenario's `setup` and a `Shell` step: git and plain files against the
/// fixture, as a teammate or an editor would change them.
pub type Fix = fn(&Fixture) -> Result<(), String>;
/// A fact only the UI state holds (a box's title, a line in the preview).
pub type See = fn(&AppState) -> Result<(), String>;
/// The verdict: from git, the marks the steps took and the claim the
/// preview made.
pub type Expect = fn(&Fixture, &Marks) -> Result<(), String>;

/// Something the walk waits for before the next key.
#[derive(Clone, Copy, Debug)]
pub enum Wait {
    /// Rows, refs, the working tree and the in-progress state have landed
    /// and nothing is running — everything a history key decides from.
    Ready,
    /// The open preview's plan has landed (ready or failed), and the ghost
    /// rows with it.
    PreviewReady,
    /// Git stopped on a conflict, the working tree re-read names the
    /// unmerged paths, and the banner has the keyboard.
    BannerFocused,
    /// The journal on screen has an entry of this kind and no re-read is in
    /// flight.
    JournalHas(OperationKind),
    /// Every branch and remote badge on screen points where git's ref
    /// does now — the re-read after a write or a fetch has landed.
    RefsCaughtUp,
    /// The journal browser is open with its entries on hand.
    Journal,
    /// The name prompt (`b`, `t`, or the stash overlay's `b`) is open, its
    /// field ready to type into. Opening it is synchronous — nothing to
    /// pump — but every scenario that types a name waits for it
    /// explicitly rather than assuming the key that opens it always will.
    Prompt,
    /// The stash overlay's list has landed: `S` opened it and dispatched
    /// `Command::ReadStashes`, and no existing `Wait` pumps for that read —
    /// `Ready`'s own predicate never looks at `AppState::stash`. Needed for
    /// `stash-b-branches-from-stash`, the one scenario that opens the
    /// overlay before typing a name into it.
    StashReady,
}

/// One thing the walk does to the running app, in order.
pub enum Step {
    /// Press one key.
    Key(KeyPress),
    /// Type this text into the field the name prompt has open, one key per
    /// character — what a person typing a branch or tag name does.
    Type(&'static str),
    /// Feed job answers until the condition holds.
    WaitFor(Wait),
    /// `j` from the top until the cursor's row carries this branch or tag
    /// badge.
    CursorTo(&'static str),
    /// `j` from the top until the cursor's row has this subject.
    CursorToSubject(&'static str),
    /// `.` on the cursor's row, `j` to the verb whose label starts with
    /// this text, `Enter`.
    MenuPick(&'static str),
    /// The same on a menu already open (the drop menu after a release):
    /// `j` to the verb, `Enter`, no `.`.
    MenuPickOpen(&'static str),
    /// Pick a branch badge up with the mouse, drag it onto the row carrying
    /// this badge and release: the drop menu opens.
    DragBadgeOnto(&'static str),
    /// Take the preview's plan as the claim `expect` judges git against.
    Claim,
    /// The open preview or confirm box: `→` off `Cancel`, `Enter`.
    Confirm,
    /// A write was queued by the last key: run it to its answer and insist
    /// it succeeded (`Status::Done`).
    Write(&'static str),
    /// Run whatever is in flight to its answer, whatever it was — for a
    /// refusal or a stop on a conflict.
    Settle(&'static str),
    /// Stage one file through the changes pane and come back to the graph.
    Stage(&'static str),
    /// Change the repository out of band, as a teammate or an editor would.
    Shell(Fix),
    /// Record truth under this name for `expect`.
    Mark(&'static str),
    /// Check a fact only the UI state holds.
    See(&'static str, See),
    /// Close the app and open it again on the same repository — what a
    /// relaunch mid-conflict is.
    Relaunch,
}

/// When the pty script writes the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    /// Once the screen has stopped changing.
    Settled,
    /// The moment the expected text appears.
    Appears,
}

/// A pty capture of this scenario (what the user judges by eye).
pub struct Capture {
    /// `m5-<nn>-<what>`, the file's stem under `docs/shots/`.
    pub stem: &'static str,
    /// Driven headless first, so the captured run starts from their result.
    pub headless: Vec<Step>,
    /// The captured run's `Harness::prelude`.
    pub keys: Vec<KeyPress>,
    /// Text the frame must contain, or the capture says it did not.
    pub on_screen: &'static str,
    /// When to take the frame.
    pub when: When,
}

/// One scenario of the walk.
pub struct Scenario {
    /// Its name on the command line and in `TESTING.md`.
    pub name: &'static str,
    /// The corpus shape the scratch (or the bare) is copied from.
    pub shape: Shape,
    /// `origin` is a bare over `file://` with a teammate's clone; the app
    /// opens the work clone.
    pub remote: bool,
    /// The terminal the app believes it has; the preview's size rule reads
    /// it. `(80, 24)` unless the scenario is about the size.
    pub size: (u16, u16),
    /// The state before the app opens, scripted with git and plain files.
    pub setup: Fix,
    /// What the walk does.
    pub steps: Vec<Step>,
    /// What must then be true, by git.
    pub expect: Expect,
    /// Its capture, when it has a visual result.
    pub capture: Option<Capture>,
}
