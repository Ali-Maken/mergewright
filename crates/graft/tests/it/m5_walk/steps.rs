//! What a scenario is made of: the steps that press, wait, mark, claim and
//! shell out, the capture a pty run replays, and the [`Scenario`] that binds
//! them to a fixture and a verdict.

use graft::update::{AppState, Command};
use graft_core::event::KeyPress;
use graft_core::journal::OperationKind;
use graft_fixtures::Shape;

use super::fixture::{Fixture, Remote};
use super::truth::Marks;

/// A scenario's `setup` and a `Shell` step: git and plain files against the
/// fixture, as a teammate or an editor would change them.
pub type Fix = fn(&Fixture) -> Result<(), String>;
/// A fact only the UI state holds (a box's title, a line in the preview).
pub type See = fn(&AppState) -> Result<(), String>;
/// A fact about the commands a key asked for, for a command the headless
/// walk must **not** run — the hand-over to `$EDITOR` is the only one.
pub type SeeCommands = fn(&[Command]) -> Result<(), String>;
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
    /// Git stopped on a conflict and the resolver the stop lands in
    /// (m6-conflicts/18) has read its first file.
    ResolverLanded,
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
    /// The Mistake Radar's default-branch context has landed. `.` builds a
    /// row's verbs from the findings it can see *at that moment*
    /// (`context_menu::open_selected` → `view::radar::findings_for`), and
    /// that function answers "no findings" while the context is still
    /// `Pending` — so the `Fix:` rows are simply absent if the key is
    /// pressed too early, with nothing on screen saying why. `Ready`'s own
    /// predicate never looks at the radar.
    RadarReady,
    /// The app has re-read git's in-progress state and found none: the
    /// operation the banner was about is over as far as the app knows.
    /// `Ready`'s own predicate never looks at `in_progress`, and the read
    /// that clears it lands after the write's own answer — so `u` straight
    /// after a `--continue` that finished is refused with `finish or abort
    /// the merge first` for an operation git has already finished.
    NotInProgress,
    /// `c` on a resolved merge opened the finish summary and its read has
    /// landed (m6-conflicts/19): `⏎` commits from here.
    FinishReady,
    /// The working tree, re-read, has no unmerged path left: what `c`
    /// checks before it will resume (`banner::continue_refusal`). Marking a
    /// file resolved is a write like any other, and the banner decides from
    /// the last read, not from the write.
    Resolved,
    /// The journal's own entries have landed — `Loading::Ready`, not merely
    /// "no re-read in flight". `u` after a push reads them to decide
    /// whether to explain itself (`update::remote::undo_after_push` returns
    /// `None` outright while they are still `Pending`), so a `u` pressed
    /// before they land is an ordinary, unasked undo.
    JournalRead,
    /// Git's credential question is open: the masked field has the
    /// keyboard.
    CredentialPrompt,
    /// The changes pane's file list has landed — the pane is open and has
    /// something to hint keys for.
    ChangesPane,
    /// The app's **remote facts** have caught up with git: the upstream's
    /// `tracking_tip` is where `refs/remotes/origin/…` really points now.
    /// `Wait::Ready` only asks whether `state.remotes` is `Ready`, which a
    /// stale reading also is — and a force-with-lease built from a stale
    /// tip is refused by git, not by the app.
    RemotesCaughtUp,
    /// The resolver is open and idle on a file: its read or its last take
    /// has landed (`job` is `None`), it holds a document **or** a
    /// whole-file card, and no write is running (m6-conflicts/07). A
    /// whole-file shape never gets a document, so waiting for `file` alone
    /// would hang on a card.
    ConflictOpened,
    /// The app's re-read no longer lists this path as unmerged, nothing is
    /// running — and git agrees: `ls-files -u -- <path>` prints nothing
    /// (m6-conflicts/07). Named `ResolvedPath` rather than the ticket's
    /// `Resolved(path)` because [`Wait::Resolved`] (every path) is M5's.
    ResolvedPath(&'static str),
    /// The inverse: the app's re-read lists this path as unmerged again,
    /// nothing is running — and git agrees, `ls-files -u -- <path>` prints
    /// its stages (m6-conflicts/07, an undone resolution).
    ConflictedPath(&'static str),
    /// The resolver has closed by itself — a re-read found the file no
    /// longer conflicted — and nothing is running (m6-conflicts/07).
    ResolverClosed,
    /// A finished operation's landing has happened: nothing in progress,
    /// nothing running, and the graph cursor moved to `HEAD`
    /// (`land_on_head` cleared; m6-conflicts/21, as
    /// `it::finish::done_lands_on_the_merge_commit` waits).
    LandedOnHead,
    /// The launch landed on an operation git is part-way through, whether or
    /// not any path is still unmerged: the banner has the keyboard
    /// (m6-conflicts/21; [`Wait::BannerFocused`] needs a conflict left).
    OnTheBanner,
    /// `u` put a finished operation back: nothing running, git in progress
    /// again, and the status re-read landed (m6-conflicts/21, as ticket
    /// 41's `u_landing` waits).
    BackInProgress,
    /// The frame drawn at the app's size has nothing still loading on it —
    /// no `reading…`, `sorting…` or `reading the diff…` (m6-conflicts/21):
    /// the frame a landing is judged on.
    FrameSettled,
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
    /// Type this text into the credential prompt's masked field. Never
    /// printed and never named by a step's own label: the walk proves the
    /// bytes reached no event, status line, prompt dump, journal, askpass
    /// argument or file (`run::no_secret_leaked`).
    TypeSecret(&'static str),
    /// The open preview or confirm box: `→` off `Cancel`, `Enter`.
    Confirm,
    /// The open box's choice at this index: `→` that many times from
    /// `Cancel`, then `Enter`. [`Step::Confirm`] is `Choose(1)` with the
    /// box's own existence checked; this is for the third choice of the
    /// undo-after-push box.
    Choose(usize),
    /// Press one key for its **commands only**, without dispatching them —
    /// the one step that looks at what a key asked for rather than at what
    /// the app did with it. `e` hands a conflicted file to `$EDITOR`, which
    /// a headless walk must not start.
    KeyFor(&'static str, KeyPress, SeeCommands),
    /// `Esc` on the write in flight, then its answer: the real `CancelJob`
    /// path. The status must say `cancelling` the moment the key lands.
    Cancel(&'static str),
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
    /// Drop the app **without shutting it down** — no `cancel_all`, no
    /// join, no outcome line — and open it again on the same repository
    /// for the steps after it: on disk, what `kill -9` leaves
    /// (m6-conflicts/07, design note Decision 2 (b)). Refused unless no
    /// write is in flight, so nothing that finishes after the "kill" can
    /// write the journal; a read still running (a status re-read after a
    /// take) is left to finish and find nobody listening. The truth
    /// assertion runs between the two apps: `rebase-merge/` or
    /// `MERGE_HEAD` present, the journal's last line `Pending`.
    Kill,
    /// Open the resolver on this path from the changes pane: off the
    /// banner, into the pane, the cursor to the path's row, `⏎`, and wait
    /// for [`Wait::ConflictOpened`]. A resolver already open on another
    /// file is closed with `Esc` first (m6-conflicts/07).
    Resolve(&'static str),
    /// Press a key whose commands hand the terminal over — `e`
    /// (`OpenExternalEditor`) or `M` (`OpenMergetool`) — and **carry the
    /// hand-over out**, as the frame loop does, with the loop's own command
    /// builders (`event_loop::outside::{editor_command, mergetool_command}`)
    /// and git's own editor lookup; then feed the answer back through
    /// `update` (m6-conflicts/07). What it cannot do headless is suspend and
    /// re-enter a terminal, so the exit-status words are this step's, copied
    /// from `Handover::{open, mergetool}` (`event_loop/outside.rs`).
    HandOver(&'static str, KeyPress),
    /// A journey's counted route starts (`true`) or ends (`false`) here
    /// (m6-conflicts/21). Between the two, every step that sends input
    /// counts as UX 1.3 counts it — a key, a chord, a click, a `Type` is one
    /// each — and `run_scenario` fails the journey when the count is over
    /// the note's `journey-counts` row. Outside them nothing is counted: a
    /// journey's start and its truth checks after the landing are not steps
    /// the user takes.
    Route(bool),
    /// A left click on the resolver's file list, on the row naming this
    /// path, found in the **drawn** frame: the strip at under 180 columns,
    /// the list column at 180 and over (m6-conflicts/21).
    Click(&'static str),
    /// Facts read from the frame as the view draws it now, at the app's own
    /// size — what the user sees, not what `AppState` holds
    /// (m6-conflicts/21: a landing is read from the drawn frame).
    Frame(&'static str, &'static [Sees]),
}

/// One fact about a drawn frame ([`Step::Frame`]). Rows are the frame's
/// rows with trailing blanks trimmed.
#[derive(Clone, Copy, Debug)]
pub enum Sees {
    /// Some row contains this text.
    Has(&'static str),
    /// No row contains this text.
    Lacks(&'static str),
    /// The bottom row (the last one with anything on it), leading blanks
    /// trimmed, starts with this text.
    BottomStarts(&'static str),
    /// Some row contains both texts, the first before the second.
    Row(&'static str, &'static str),
    /// The first row containing the first text is above the first row
    /// containing the second, and both exist.
    Above(&'static str, &'static str),
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
    /// What `origin` is: nothing, a bare over `file://` with a teammate's
    /// clone, or that same bare over Basic-auth HTTP. For either remote the
    /// app opens the work clone.
    pub remote: Remote,
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
