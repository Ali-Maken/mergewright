//! Ghost Preview: **you see the result before you confirm.**
//!
//! Every history-changing key — `m`, `r`, the `.` menu's verbs, the
//! journal's time machine — opens this instead of running anything. A job
//! asks `graft_ops::preview` what the operation *would* do; the answer is
//! drawn as ghost rows in the graph and as a box docked above the footer
//! naming the refs that move, the working tree's fate, a conflict forecast
//! and the exact git commands. `Enter` on `Confirm` then runs **the same
//! [`Operation`] value** that was previewed (design note, hard choice 3):
//! the picture and the act are the same act, and a tip that moved in
//! between is refused by `check` rather than quietly re-planned.
//!
//! Modal, like the confirm box and the context menu: while it is open
//! [`preview_key`] claims every key but `Ctrl+C`, so a `u` typed over a
//! preview does nothing at all. It sits between the confirm box and the
//! context menu in [`super::keys::key_event`] — a confirm box may open over
//! a preview, never the reverse, and the menu closes before a preview opens.
//!
//! Seam rule (ARCHITECTURE.md): this module builds an [`Operation`] and
//! reads a [`PreviewPlan`]. **Every word in the box comes from the plan** —
//! there is no hand-written `git …` line anywhere here (`KNOWN_GAPS` row
//! 229), which is what keeps the box and the Command Log from describing
//! one operation two ways.

mod line_rows;
pub mod slice;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use graft_core::commit::Commit;
use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;
use graft_core::journal::EntryId;
use graft_core::oid::Oid;
use graft_graph::row::Row;
use graft_ops::operation::{Operation, Source, Tip};
use graft_ops::preview::{
    ConflictForecast, GhostCommit, HeadMove, PreviewPlan, RefMove, Target, UndoNote, WorktreeEffect,
};
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::preview_box::{PREVIEW_MIN, PreviewBox, preview_rows};

/// The shortest a box can be drawn and still be a box: two borders, one
/// line, one choices row. Mirrors `graft_ui::preview_box`'s own constant,
/// which is crate-private there.
const MIN_BOX_ROWS: u16 = 4;

/// Where the preview box goes on screen, in plain screen coordinates —
/// `update` draws nothing and names no drawing type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoxRect {
    /// Leftmost column.
    pub x: u16,
    /// Topmost row.
    pub y: u16,
    /// How wide.
    pub width: u16,
    /// How tall.
    pub height: u16,
}

impl BoxRect {
    /// One past the last row.
    #[must_use]
    pub fn bottom(self) -> u16 {
        self.y.saturating_add(self.height)
    }
}

pub use self::slice::{BadgeOverlay, PREVIEW_ANCHOR_CAP};

use self::slice::{SliceError, badges_for_preview, build_slice};
use super::detail::Loading;
use super::events::Command;
use super::state::{AppState, Status};
use crate::refs::{RefBadges, RefLabel};

/// Why `Esc` will not stop a running history operation.
///
/// Rendered after the operation's own description
/// (`remote::stop_write`: `"{description} {why}"`), so it reads
/// `rebase feature/auth onto main runs to the end · a conflict stops it on
/// the banner, where a aborts`.
///
/// A merge or a rebase is not one git write. Killing git mid-rebase leaves
/// `rebase-merge/` on disk with the journal entry `Pending` and no outcome
/// line — a state `graft_ops` does not model as `Aborted`, and which the
/// banner would then have to adopt as if the user had asked for it. The
/// honest answer is not to offer the cancel: a conflict stops the operation
/// by itself, on the banner, where `a` aborts it properly.
pub const HISTORY_CANNOT_STOP: &str =
    "runs to the end · a conflict stops it on the banner, where a aborts";

/// What a preview is a preview of.
///
/// The `Operation` is boxed and carried whole rather than rebuilt at
/// confirm time: the value that runs must be the value that was drawn, by
/// `PartialEq` (`m_opens_preview_and_enter_runs_the_pinned_op`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewSubject {
    /// One history operation, pinned to the tips it was drawn against.
    Operation {
        /// What `Confirm` runs.
        op: Box<Operation>,
        /// Which key opened it, so a `Moved` refusal can say which key to
        /// press again (`m again`, `r again`, `. again`).
        opened_by: &'static str,
    },
    /// The journal's time machine: revert to just before this entry.
    RevertTo(EntryId),
}

impl PreviewSubject {
    /// The operation this subject would run, when it has one.
    #[must_use]
    pub fn operation(&self) -> Option<&Operation> {
        match self {
            PreviewSubject::Operation { op, .. } => Some(op),
            PreviewSubject::RevertTo(_) => None,
        }
    }
}

/// What the preview job asks for: the subject, and the commits the app
/// already has on screen so the job only reads the ones it does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewRequest {
    /// What to plan.
    pub subject: PreviewSubject,
    /// Every commit id the app can draw without reading anything.
    pub resident: HashSet<Oid>,
}

/// Why the preview job drew nothing.
///
/// Two answers, routed apart (m6-conflicts/67): a refusal is not a failure,
/// and must not read as one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewFailure {
    /// `graft_ops::preview::plan` refused the operation — the same refusal
    /// the write would give. No box: the status line says why.
    Refused(graft_ops::operation::Refusal),
    /// A read failed. The line the `preview failed:` box prints, written
    /// for the user.
    Failed(String),
}

/// What the job came back with: the plan, and the commits it had to read to
/// anchor ghosts whose parents were not on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Planned {
    /// What the operation would do.
    pub plan: PreviewPlan,
    /// Commits read to anchor the picture, at most [`PREVIEW_ANCHOR_CAP`].
    pub anchors: Vec<Commit>,
    /// The operation as `y` will run it (`graft_ops::preview::plan_as_run`):
    /// a rebase completed with its line and drops, anything else as asked.
    /// `preview_produced` moves it into the subject, so the op `y` confirms
    /// is the one the picture was drawn from. `None` for the time machine.
    pub runs: Option<Box<Operation>>,
}

/// One open Ghost Preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewState {
    /// What is being previewed.
    pub subject: PreviewSubject,
    /// The rows the graph had on hand when the preview opened, in drawing
    /// order — the slice the picture is laid out from.
    ///
    /// The commits themselves, not only their ids (which is what the job
    /// gets): the picture must not change under the user because a refresh
    /// landed while they were reading it, and a snapshot is the cheapest way
    /// to promise that.
    pub resident: Vec<Commit>,
    /// The absolute history row number of `resident[0]`.
    pub slice_top: usize,
    /// The planning job. Only this id's answer is wanted.
    pub job: JobId,
    /// The plan, once it lands.
    pub plan: Loading<Arc<Planned>>,
    /// The laid-out rows the graph draws while the plan is `Ready`.
    pub rows: Vec<Row>,
    /// Where the user's screen begins inside [`PreviewState::rows`].
    ///
    /// The slice is three screens wide so a ghost has somewhere to hang,
    /// but only one of those screens is the user's. Drawing from `rows[0]`
    /// shows them the screen *above* theirs the moment they are scrolled
    /// (ticket 17 F1), so the picture starts here instead: the index of the
    /// row that was at the top of the viewport when the preview opened.
    pub view_start: usize,
    /// The commit the cursor was on, as an index into
    /// [`PreviewState::rows`] — what the picture highlights.
    ///
    /// Not [`PreviewState::cursor`], which is the *choice* cursor inside
    /// the box (`0` Cancel, `1` Confirm) and has nothing to do with a row.
    pub selected_row: Option<usize>,
    /// Which of [`PreviewState::rows`] are ghosts, by their synthetic id,
    /// mapped to their index in the plan's ghost list.
    pub ghost_index: HashMap<Oid, usize>,
    /// How many parent links the slice had to drop at its edge.
    pub clipped: usize,
    /// A ghost parent that is neither resident nor an anchor: the picture
    /// cannot be drawn in the right place, so it is not drawn at all and the
    /// box says which commit is missing.
    pub undrawable: Option<Oid>,
    /// Which choice the cursor is on. `0` is always the safe one.
    pub cursor: usize,
    /// The journal browser was open and asked for this; closing the preview
    /// puts it back at the same row.
    pub return_to_journal: bool,
    /// The plan was refused and the box stays up saying why: a stash
    /// put-back's preview (m6-conflicts/86), whose words — `your unsaved
    /// edits to notes.md are in the way` — are about the list under it.
    /// [`PreviewState::plan`] is then `Failed(<the refusal's words>)`,
    /// drawn without the `preview failed:` prefix.
    pub refused: bool,
}

impl PreviewState {
    /// The plan, if it has landed.
    fn planned(&self) -> Option<&Planned> {
        match &self.plan {
            Loading::Ready(planned) => Some(planned),
            Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
        }
    }

    /// Is this preview still waiting for its plan?
    fn planning(&self) -> bool {
        matches!(self.plan, Loading::Idle | Loading::Pending(_))
    }

    /// Can `Enter` act at all? Only a plan that landed, that does
    /// something, and that could be drawn offers `Confirm`.
    fn acts(&self) -> bool {
        self.undrawable.is_none() && self.planned().is_some_and(|it| !nothing_to_do(&it.plan))
    }

    /// What to call the predicted block on screen, in the user's words:
    /// `after rebase`, `after merge`, `after cherry-pick`, `after revert`,
    /// else `the result`.
    ///
    /// Drawn on a row of its own above the first predicted row
    /// (`view::preview::label_entry`). The label is the separator whose
    /// absence was the complaint: without it the predicted subjects are
    /// word for word the real ones directly above them, and the first
    /// available reading is *these commits are in my history twice*
    /// (m5/20 item 4). **Never says `ghost`** — that is our word, a
    /// `CONTEXT.md` glossary term the interface never defines.
    #[must_use]
    pub fn outcome_label(&self) -> String {
        self.subject
            .operation()
            .map(graft_ops::operation::Operation::kind)
            .and_then(outcome_verb)
            .map_or_else(|| "the result".to_owned(), |verb| format!("after {verb}"))
    }
}

/// The word for what the operation does, for [`PreviewState::outcome_label`]
/// — the verb the user pressed a key for, not the type's name. `None` for
/// the kinds a preview never opens over, which fall back to `the result`
/// rather than inventing a verb for them.
fn outcome_verb(kind: graft_core::journal::OperationKind) -> Option<&'static str> {
    use graft_core::journal::OperationKind;
    match kind {
        OperationKind::Merge => Some("merge"),
        OperationKind::Rebase => Some("rebase"),
        OperationKind::CherryPick => Some("cherry-pick"),
        OperationKind::Revert => Some("revert"),
        _ => None,
    }
}

/// A plan that would change nothing: a merge already up to date, a reset to
/// where the branch already is. There is nothing to confirm, so the box
/// offers only `Close` — an enabled `Confirm` that does nothing is how a
/// user learns to stop reading the box.
fn nothing_to_do(plan: &PreviewPlan) -> bool {
    plan.ref_moves.is_empty()
        && plan.head.is_none()
        && plan.ghosts.is_empty()
        && matches!(plan.worktree, WorktreeEffect::Untouched)
}

/// Why a preview cannot open right now, as the words to show: a write is
/// already running, or git is mid-operation. `None` when it can.
///
/// Split out of [`open_preview`] so a caller whose own modal stays open can
/// show the refusal inside that modal instead of on the status line under
/// it: branch-from-stash's name prompt (m6-conflicts/69, conductor
/// 2026-09-25).
pub(crate) fn preview_refusal(state: &AppState) -> Option<String> {
    if let Some(busy) = super::operate::in_flight(state) {
        return Some(busy.words());
    }
    // Every live operation refuses, a merge begun in the shell included:
    // keymap §2.6a's exception covers `u`/`U` only, and this opens a Ghost
    // Preview or the journal's revert. No keys: the preview and the journal
    // overlays hide the banner, so `⇥ banner` would point at nothing.
    let kind = state.in_progress.as_ref()?.kind;
    Some(super::undo::live_refusal_words(kind))
}

/// Opens a preview of `subject`, or refuses in words.
///
/// Refuses — with no job started — while another write is running, while a
/// preview is already open, and while git has an operation in progress:
/// none of the three can be previewed honestly, and a preview drawn against
/// a repository that is about to change is a picture of nothing.
///
/// `true` when a preview opened.
pub fn open_preview(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    subject: PreviewSubject,
    return_to_journal: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    // A preview is already on screen. It is modal and it is the answer to
    // this key, so there is nothing to say that the screen is not already
    // saying.
    if state.preview.is_some() {
        return false;
    }
    if let Some(reason) = preview_refusal(state) {
        state.status = Status::Sticky { reason, keys: None };
        return false;
    }

    // The slice: every resident row in [top − graph_rows, top + 2·graph_rows),
    // snapshotted now. Taken **before** `state.preview` is set, so the box's
    // own rows have not yet shrunk the window it is measured in.
    let height = state.graph_rows();
    let (slice_top, resident) = resident_slice(state, height);
    let id = next_id();
    let request = PreviewRequest {
        subject: subject.clone(),
        resident: resident.iter().map(|commit| commit.id).collect(),
    };
    state.preview = Some(PreviewState {
        subject,
        resident,
        slice_top,
        job: id,
        plan: Loading::Pending(id),
        rows: Vec::new(),
        view_start: 0,
        selected_row: None,
        ghost_index: HashMap::new(),
        clipped: 0,
        undrawable: None,
        cursor: 0,
        return_to_journal,
        refused: false,
    });
    commands.push(Command::PlanPreview(id, Box::new(request)));
    true
}

/// The rows the picture is laid out from: everything the window holds in
/// `[top − height, top + 2·height)`, in drawing order, with the absolute
/// row number of the first.
fn resident_slice(state: &AppState, height: usize) -> (usize, Vec<Commit>) {
    let Some(history) = state.history.as_ref() else {
        return (0, Vec::new());
    };
    let first = history.top.saturating_sub(height);
    let last = history.top.saturating_add(2 * height);
    let mut commits = Vec::new();
    let mut slice_top = first;
    for row in first..last {
        match history.row_at(row) {
            Some(found) => {
                if commits.is_empty() {
                    slice_top = row;
                }
                commits.push(found.commit.clone());
            }
            // A gap in the window: the rows on hand are contiguous, so this
            // only happens before the first one or after the last.
            None if commits.is_empty() => {}
            None => break,
        }
    }
    (slice_top, commits)
}

/// The preview's keys, while it is open.
///
/// Modal: **every** key but `Ctrl+C` is claimed, so nothing else — `u`
/// least of all — acts over a picture the user is still reading. `←`/`→`
/// and `Tab`/`BackTab` move between the choices, clamped and without
/// wrapping (ADR-0038 §1: walking left off `Cancel` must not arrive at
/// `Confirm`); `Enter` acts on the choice under the cursor; `Esc` closes,
/// cancelling a plan still in flight. Everything else is swallowed.
///
/// `true` whenever a preview was open.
pub fn preview_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if state.preview.is_none() {
        return false;
    }
    // `Ctrl+C` quits from anywhere, including here. It is the one key no
    // modal in this app claims.
    if key.code == KeyCode::Char('c') && key.modifiers == Modifiers::CONTROL {
        return false;
    }
    // Counted before the borrow, since it reads the whole state; the `else`
    // then covers the same case the check above did, without a panic
    // standing between the two (review 08 Minor 3).
    let choices = choice_count(state);
    let Some(preview) = state.preview.as_mut() else {
        return false;
    };
    if let PreviewSubject::Operation { opened_by, .. } = &preview.subject
        && confirm_key(key, opened_by)
    {
        confirm_by_key(state, commands, next_id);
        return true;
    }
    match key.code {
        KeyCode::Left => preview.cursor = preview.cursor.saturating_sub(1),
        KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
            preview.cursor = preview
                .cursor
                .saturating_add(1)
                .min(choices.saturating_sub(1));
        }
        KeyCode::Enter => enter(state, commands, next_id),
        KeyCode::Escape => close_preview(state, commands),
        // Swallowed: the picture keeps the keyboard until it is answered.
        _ => {}
    }
    true
}

/// `y` in any operation preview; the opening verb again in an `m`- or
/// `r`-opened one. Nothing else — not `b` in a name-prompt preview, not `.`
/// in a context-menu one, not the other verb (user ruling, 2026-09-23).
fn confirm_key(key: KeyPress, opened_by: &str) -> bool {
    if key.code == KeyCode::Char('y') && !key.modifiers.control && !key.modifiers.alt {
        return true;
    }
    let again = match is_history_key(key) {
        Some(HistoryKey::Merge) => "m",
        Some(HistoryKey::Rebase) => "r",
        None => return false,
    };
    again == opened_by
}

/// A confirm key: `Confirm` wherever the cursor is, but only over a picture
/// that is on screen, landed and able to act. Otherwise swallowed, and the
/// box stays open — unlike `Enter` on `Cancel`, a confirm key never closes
/// it. Not queued while planning: the user sees the picture before he
/// confirms (M5's promise), and the box already reads `predicting…`.
fn confirm_by_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if !fits(state) {
        return;
    }
    let Some(preview) = state.preview.as_ref() else {
        return;
    };
    if preview.planning() || !preview.acts() {
        return;
    }
    confirm_preview(state, commands, next_id);
}

/// The footer keys a box for `preview` names: `y confirm` for an operation
/// preview, and the opening verb when it confirms again — but only where
/// [`confirm_by_key`] answers them: a plan that can act, or one still
/// planning, where the key is waiting for the picture rather than dead. A
/// closing box and a can't-act plan name neither (`02-keymap.md:111`,
/// review m6-amend-11-12 Minor 1).
fn footer_keys(mut view: PreviewBox, preview: &PreviewState) -> PreviewBox {
    if !(preview.acts() || preview.planning()) {
        return view;
    }
    // A stash put-back names its confirm by its verb, `y apply` / `y pop`
    // (m6-conflicts/86), through the same footer slot `m merge` uses. `y`
    // is the key either way (`confirm_key`); the opening key is never a
    // confirm here — no `p p`.
    if let Some((_, pop)) = stash_put_back(&preview.subject) {
        view.again = Some((sanitize_line("y"), sanitize_line(stash_verb(pop))));
        return view;
    }
    if let PreviewSubject::Operation { opened_by, .. } = &preview.subject {
        view.confirm_key = true;
        view.again = match *opened_by {
            "m" => Some((sanitize_line("m"), sanitize_line("merge"))),
            "r" => Some((sanitize_line("r"), sanitize_line("rebase"))),
            _ => None,
        };
    }
    view
}

/// `Enter`: act on the choice under the cursor.
fn enter(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId) {
    // A picture that cannot be seen is never confirmed (design note, "New
    // UI"): below `PREVIEW_MIN` the box is one line of text and the plan is
    // not on screen at all, so `Enter` does nothing and `Esc` is the way
    // out.
    if !fits(state) {
        return;
    }
    let Some(preview) = state.preview.as_ref() else {
        return;
    };
    // Index 0 is always the safe choice, and a box with nothing to confirm
    // has only that one.
    if preview.cursor == 0 || !preview.acts() {
        close_preview(state, commands);
        return;
    }
    confirm_preview(state, commands, next_id);
}

/// Is the screen big enough to show a preview at all?
///
/// Measured against the terminal, which is what the design note's "< 40
/// cols or < 12 rows" means and what [`preview_box_rect`] measures too, so
/// the key and the picture cannot disagree about whether there is a picture.
#[must_use]
pub fn fits(state: &AppState) -> bool {
    state.size.columns >= PREVIEW_MIN.0 && state.size.rows >= PREVIEW_MIN.1
}

/// How many choices the open box offers; `0` when none is open.
fn choice_count(state: &AppState) -> usize {
    preview_box(state).map_or(0, |view| view.choices().len())
}

/// A click while the preview is open.
///
/// A click on one of the box's buttons is that choice, acted on at once.
/// Every other click — and the wheel, which never reaches here — is
/// ignored: the rows behind a modal picture are not selectable, and a
/// stray click must never dismiss a box the user is reading.
///
/// `true` whenever a preview was open.
pub fn preview_click(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    column: u16,
    row: u16,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(view) = preview_box(state) else {
        return false;
    };
    let Some(area) = preview_box_rect(state, &view) else {
        return true;
    };
    if let Some(choice) = choice_at(&view, area, column, row) {
        if let Some(preview) = state.preview.as_mut() {
            preview.cursor = choice;
        }
        enter(state, commands, next_id);
    }
    true
}

/// The open preview's plan failed: a refusal closes the box with a status
/// line (m6-conflicts/67) — except a stash put-back's, which keeps its box:
/// the refusal is about the list the box sits over, and `esc` is back on it
/// (m6-conflicts/86; the design note's "Refused: the box's refusal state").
/// A failed read shows in the box.
fn plan_failed(state: &mut AppState, failure: PreviewFailure) {
    let Some(preview) = state.preview.as_mut() else {
        return;
    };
    match failure {
        PreviewFailure::Refused(refusal) if stash_put_back(&preview.subject).is_some() => {
            preview.plan = Loading::Failed(refusal.explain());
            preview.refused = true;
            preview.cursor = 0;
        }
        PreviewFailure::Refused(refusal) => refused(state, &refusal),
        PreviewFailure::Failed(error) => {
            preview.plan = Loading::Failed(error);
            preview.cursor = 0;
        }
    }
}

/// The plan for an open preview landed (or failed).
///
/// Lays the slice out **once**, here, rather than per frame: `layout` is a
/// pure function over a few hundred commits and the result is what the
/// graph draws until the box closes.
pub fn preview_produced(
    state: &mut AppState,
    id: JobId,
    result: Result<Arc<Planned>, PreviewFailure>,
) {
    // The same seeds the live graph was laid out with, rebuilt from the
    // walk's own facts and the badges: the ghosts have to sit in the same
    // lanes and carry the same colours as the real rows around them, or the
    // before and after read as two graphs rather than one picture.
    let seeds = state
        .refs
        .as_deref()
        .map_or_else(graft_graph::Seeds::default, |refs| {
            crate::history::preview_seeds(
                state.history.as_ref().and_then(|it| it.walk.as_ref()),
                refs,
            )
        });
    // Which absolute history row the user's screen starts at, and which
    // their cursor is on. Read before the preview is borrowed, and read
    // once: a preview is modal, so neither can move while it is open.
    let viewport = state.history.as_ref().map(|it| (it.top, it.cursor));
    let Some(preview) = state.preview.as_mut() else {
        // The preview closed while the job was in flight. Nothing to do —
        // `close_preview` has already cancelled it.
        return;
    };
    if preview.job != id {
        // A stale answer, from a preview that has been closed and reopened.
        return;
    }
    let planned = match result {
        Ok(planned) => planned,
        Err(failure) => {
            plan_failed(state, failure);
            return;
        }
    };
    // The op the job completed (05b: a rebase's line and drops) becomes the
    // one `y` runs — `confirm_preview` moves the subject's op across. A
    // copy out of the shared answer; the key's own op is let go.
    if let (Some(runs), PreviewSubject::Operation { op, .. }) =
        (&planned.runs, &mut preview.subject)
    {
        op.clone_from(runs);
    }
    match build_slice(&preview.resident, &planned.anchors, &planned.plan) {
        Ok(slice) => {
            preview.ghost_index = slice
                .ghost_at
                .iter()
                .enumerate()
                .filter_map(|(index, &at)| Some((slice.commits.get(at)?.id, index)))
                .collect();
            preview.clipped = slice.clipped;
            preview.undrawable = None;
            let rows = graft_graph::layout(&slice.commits, seeds);
            let (top, cursor) = viewport.unwrap_or((0, 0));
            // `top` is always resident — it is the row the user is looking
            // at — so the fallback only fires when there is no history at
            // all, where `rows` is empty anyway.
            preview.view_start = row_of(preview, &rows, top).unwrap_or(0);
            preview.selected_row = row_of(preview, &rows, cursor);
            preview.rows = rows;
        }
        Err(SliceError::MissingParent(oid)) => {
            preview.rows = Vec::new();
            preview.view_start = 0;
            preview.selected_row = None;
            preview.ghost_index = HashMap::new();
            preview.clipped = 0;
            preview.undrawable = Some(oid);
        }
    }
    preview.plan = Loading::Ready(planned);
    preview.cursor = 0;
}

/// The preview's operation was refused: close the box, and say why on the
/// status line, in the words the write would use.
///
/// No box: the preview is modal, so the key the words name (`S`) would need
/// `esc` first (UX §1.3), and a picture of a write that cannot run is not
/// the result before you confirm. Focus goes back where the key was
/// pressed — nothing but the box moved it; the journal comes back if the
/// preview was opened from there, as `close_preview` does.
fn refused(state: &mut AppState, refusal: &graft_ops::operation::Refusal) {
    let Some(preview) = state.preview.take() else {
        return;
    };
    if preview.return_to_journal {
        state.journal.shown = true;
    }
    state.status = match &preview.subject {
        PreviewSubject::Operation { op, .. } => super::operate::refusal_status(op, refusal),
        // The time machine is planned by `plan_revert_to`, which the job
        // never maps to `Refused`; if it ever does, its sentence stands.
        PreviewSubject::RevertTo(_) => Status::sticky(refusal.explain()),
    };
}

/// Where the commit on absolute history row `row` ended up in the laid-out
/// slice, if the slice holds it at all.
///
/// By id rather than by arithmetic: the slice has the plan's ghosts woven
/// into it, so a resident row's index in `rows` is its index in the
/// snapshot plus however many ghosts were inserted above it.
fn row_of(preview: &PreviewState, rows: &[Row], row: usize) -> Option<usize> {
    let id = preview
        .resident
        .get(row.checked_sub(preview.slice_top)?)?
        .id;
    rows.iter().position(|it| it.commit.id == id)
}

/// `Confirm`: run the very operation that was drawn.
///
/// The preview closes on the same event — the picture has been answered —
/// and the status line carries the operation's description from this
/// keystroke rather than from whenever the job thread gets going.
pub fn confirm_preview(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(preview) = state.preview.take() else {
        return;
    };
    let Some(description) = preview.planned().map(|it| it.plan.description.clone()) else {
        // `Confirm` is only offered over a plan that landed
        // (`PreviewState::acts`), so this cannot fire. If it ever did, the
        // honest answer is to leave the picture up: launching an operation
        // under a description this function had to invent is how the box
        // and the Command Log come to describe one act two ways
        // (review 08 Minor 2).
        state.preview = Some(preview);
        return;
    };
    match preview.subject {
        PreviewSubject::Operation { op, opened_by } => {
            // The same value, not a rebuilt one: `check` refuses `Moved` if
            // a tip has shifted since the picture was drawn, which is the
            // whole of hard choice 3. `opened_by` rides along so that
            // refusal can name the key to press again.
            //
            // The preview has closed on this key, so a refusal (a write
            // began while it was up; `preview_refusal` only runs at open)
            // is said on the row it gave back (m6-conflicts/78).
            //
            // A stash put-back runs as the list's key ran it before the
            // preview (m6-conflicts/86): one git write, which `Esc` may
            // stop, so `launch` — not the history ops' `launch_unstoppable`,
            // whose `Esc` words name a rebase's banner.
            let launched = if stash_put_back_op(&op).is_some() {
                super::operate::launch(state, commands, *op, next_id)
            } else {
                super::operate::launch_unstoppable(
                    state,
                    commands,
                    *op,
                    HISTORY_CANNOT_STOP,
                    Some(opened_by),
                    next_id,
                )
            };
            super::operate::say_if_refused(state, launched);
        }
        PreviewSubject::RevertTo(entry) => {
            let id = next_id();
            let claimed = super::operate::claim_write(state, id);
            if claimed.is_err() {
                super::operate::say_if_refused(state, claimed);
                return;
            }
            state.status = Status::Busy(description.clone());
            super::remote::launched(
                state,
                id,
                description,
                None,
                super::remote::Stoppable::No(HISTORY_CANNOT_STOP),
                // No key to press again: the time machine is aimed from a
                // journal row, not from a key over the graph.
                None,
            );
            commands.push(Command::RevertTo(id, entry, false));
        }
    }
}

/// Closes the preview without running anything.
///
/// A plan still in flight is cancelled: it is a read, but it can be several
/// `merge-tree` spawns, and nobody is waiting for the answer any more. The
/// journal browser reopens if it was what asked.
pub fn close_preview(state: &mut AppState, commands: &mut Vec<Command>) {
    let Some(preview) = state.preview.take() else {
        return;
    };
    // A put-in's words go with its picture (m7-power-tools/09).
    super::amend_into::forget(state);
    if preview.planning() {
        commands.push(Command::CancelJob(preview.job));
    }
    if preview.return_to_journal {
        state.journal.shown = true;
    }
}

/// The badges the graph draws while the preview is open, or `None` when
/// there is no plan to overlay.
#[must_use]
pub fn preview_badges(state: &AppState) -> Option<BadgeOverlay<'_>> {
    let preview = state.preview.as_ref()?;
    let planned = preview.planned()?;
    let badges = state.refs.as_deref()?;
    Some(badges_for_preview(badges, &planned.plan))
}

// --- the box ------------------------------------------------------------

/// Everything the Ghost Preview box draws, or `None` when none is open.
///
/// Pure over [`AppState`], and **every line comes from the
/// [`PreviewPlan`]**: the refs it moves, what it does to the working tree,
/// what the forecast says, what `u` would take back, and `plan.commands`
/// verbatim. No `git …` string is built here (`KNOWN_GAPS` row 229) — the
/// plan's own lines are the ones that run.
#[must_use]
pub fn preview_box(state: &AppState) -> Option<PreviewBox> {
    let preview = state.preview.as_ref()?;
    let view = match &preview.plan {
        Loading::Idle | Loading::Pending(_) => {
            let mut view = PreviewBox::new(sanitize_line(&planning_title(&preview.subject)));
            view.planning = true;
            view
        }
        Loading::Failed(words) if preview.refused => closing_box(words, &preview.subject),
        Loading::Failed(error) => {
            closing_box(&format!("preview failed: {error}"), &preview.subject)
        }
        Loading::Ready(planned) => {
            if let Some(missing) = preview.undrawable {
                closing_box(
                    &format!(
                        "preview cannot be drawn here: {} is not loaded · esc",
                        short(missing)
                    ),
                    &preview.subject,
                )
            } else if let Some((index, pop)) = stash_put_back(&preview.subject) {
                stash_box(&planned.plan, index, pop)
            } else {
                ready_box(
                    &planned.plan,
                    preview.subject.operation(),
                    preview.clipped,
                    state.size.columns,
                )
            }
        }
    };
    let view = put_in_words(footer_keys(view, preview), state, preview);
    Some(with_cursor(view, preview.cursor))
}

/// A put-in's own words over the rewrite's box (m7-power-tools/09, note
/// R2): the title names what goes where, the rows above the plan's say
/// what each commit gains, and the footer names the confirm by its verb
/// (`y put it in`), as a stash put-back's does. Only for the operation the
/// put-in planned — an `R`-editor rewrite keeps the rewrite's words.
fn put_in_words(mut view: PreviewBox, state: &AppState, preview: &PreviewState) -> PreviewBox {
    let Some(shown) = preview
        .subject
        .operation()
        .and_then(|op| state.amend_into.shown_for(op))
    else {
        return view;
    };
    view.title = sanitize_line(&format!("Preview: {}", shown.title));
    if preview.planned().is_some() && preview.undrawable.is_none() {
        let mut lines: Vec<_> = shown.rows.iter().map(|row| sanitize_line(row)).collect();
        lines.append(&mut view.lines);
        view.lines = lines;
    }
    if preview.acts() || preview.planning() {
        view.confirm_key = false;
        view.again = Some((sanitize_line("y"), sanitize_line(shown.confirm)));
    }
    view
}

/// A box with nothing to confirm: one line saying why, and `Close`.
fn closing_box(line: &str, subject: &PreviewSubject) -> PreviewBox {
    let mut view = PreviewBox::new(sanitize_line(&planning_title(subject)))
        .with_choices(vec![sanitize_line("Close")]);
    view.lines = vec![sanitize_line(line)];
    // The stash note's refusal state: keys `esc back` (review 86, m3).
    // Other previews keep their footer; see the ticket's NOTES.
    view.back_only = stash_put_back(subject).is_some();
    view
}

/// The box for a plan that landed, for a frame `columns` wide. `op` is the
/// operation it was planned for, when it is one: a rebase or rewrite draws
/// its line (m7/05).
fn ready_box(
    plan: &PreviewPlan,
    op: Option<&Operation>,
    clipped: usize,
    columns: u16,
) -> PreviewBox {
    let title = sanitize_line(&format!("Preview: {}", plan.description));
    let mut view = if nothing_to_do(plan) {
        PreviewBox::new(title).with_choices(vec![sanitize_line("Close")])
    } else {
        PreviewBox::new(title)
    };
    view.lines = plan_lines(plan, op, clipped, columns);
    view.commands = plan
        .commands
        .iter()
        .map(|line| sanitize_line(line))
        .collect();
    view
}

/// Which stash a put-back preview is about, and whether it pops: `None` for
/// every other subject.
fn stash_put_back(subject: &PreviewSubject) -> Option<(usize, bool)> {
    stash_put_back_op(subject.operation()?)
}

/// [`stash_put_back`] for an operation on its own.
fn stash_put_back_op(op: &Operation) -> Option<(usize, bool)> {
    match op {
        Operation::StashApply { index, .. } => Some((*index, false)),
        Operation::StashPop { index, .. } => Some((*index, true)),
        _ => None,
    }
}

/// The verb on a stash put-back's confirm button and footer.
fn stash_verb(pop: bool) -> &'static str {
    if pop { "pop" } else { "apply" }
}

/// The box for a stash put-back (m6-conflicts/86): the files it writes,
/// listed as the conflict line lists its paths; the forecast in the
/// preview's own words; one row for the stash's fate, kept whole; and
/// `Cancel` / `Apply` (or `Pop`).
fn stash_box(plan: &PreviewPlan, index: usize, pop: bool) -> PreviewBox {
    let name = format!("stash@{{{index}}}");
    let title = sanitize_line(&format!("Preview: {}", plan.description));
    let label = if pop { "Pop" } else { "Apply" };
    let mut view =
        PreviewBox::new(title).with_choices(vec![sanitize_line("Cancel"), sanitize_line(label)]);
    let files = match &plan.worktree {
        WorktreeEffect::Updated { paths, more } => {
            format!("working tree: {}", capped_list(paths, *more))
        }
        // Only the index changes (review 86, m1): its paths, never an empty
        // `working tree:` line.
        WorktreeEffect::IndexOnly { paths, more } => {
            format!("index only: {}", capped_list(paths, *more))
        }
        other => worktree_line(other),
    };
    let mut lines = vec![files, conflict_line(&plan.conflicts, None)];
    if pop {
        lines.push(format!("{name} is dropped \u{b7} u brings it back"));
    } else {
        lines.push(format!("{name} stays in the list"));
        lines.push(undo_line(&plan.undo));
    }
    view.lines = lines.iter().map(|line| sanitize_line(line)).collect();
    view.commands = plan
        .commands
        .iter()
        .map(|line| sanitize_line(line))
        .collect();
    view
}

/// The box's title before the plan lands, where `plan.description` is not
/// known yet: the subject's own words, which is all the app has.
fn planning_title(subject: &PreviewSubject) -> String {
    match subject {
        PreviewSubject::Operation { op, .. } => format!("Preview: {}", op.describe()),
        PreviewSubject::RevertTo(_) => "Preview: revert to before this".to_owned(),
    }
}

/// The wording table: ghosts, refs, `HEAD`, the working tree and the edits
/// set aside around it, the forecast, the undo note, and the slice's own
/// edge. `columns` picks the set-aside row's wording; every other row is
/// one wording, cut by the box.
///
/// A rebase or rewrite moving a line of branches draws them under a
/// `branches:` label, the squash-merge suggestion under them, `on origin:`
/// above the forecast, and a stop forecast by its own cut ladder
/// (`line_rows`, m7/05). A plain one-branch rebase draws as before.
fn plan_lines(
    plan: &PreviewPlan,
    op: Option<&Operation>,
    clipped: usize,
    columns: u16,
) -> Vec<SanitizedText> {
    let mut lines: Vec<String> = Vec::new();
    lines.extend(ghost_line(&plan.ghosts));
    if line_rows::is_line(plan, op) {
        lines.extend(line_rows::ref_rows(plan));
    } else {
        lines.extend(plan.ref_moves.iter().map(|moved| ref_line(plan, moved)));
    }
    lines.extend(line_rows::already_in_row(plan, op));
    lines.extend(head_line(plan));
    lines.push(worktree_line(&plan.worktree));
    // The edits a rebase or checkout sets aside and puts back after
    // (ADR-0045 §4, m6-conflicts/74): under the tree line it qualifies.
    // No staging words: every put-back keeps the staging (ADR-0045
    // amendment b), and `SetAside` carries only the file count (30c).
    if let Some(aside) = plan.set_aside {
        lines.push(graft_ui::preview_box::fitting_line(
            &graft_ui::preview_box::set_aside_row(aside.files),
            columns,
        ));
    }
    lines.extend(line_rows::published_row(plan));
    // A stop forecast cuts its paths, then its subject, then `· not
    // checked past it`, never the step (note R3).
    lines.push(match line_rows::forecast_rungs(plan) {
        Some(wordings) => {
            let wordings: Vec<String> = wordings
                .into_iter()
                .map(|wording| with_put_back(wording, plan.put_back.as_ref()))
                .collect();
            graft_ui::preview_box::fitting_line(&wordings, columns)
        }
        None => conflict_line(&plan.conflicts, plan.put_back.as_ref()),
    });
    // A plan that changes nothing has nothing to take back, and offering
    // `u` over it teaches the box says things it does not mean
    // (review 08 Minor 7).
    if !nothing_to_do(plan) {
        lines.push(undo_line(&plan.undo));
    }
    // The slice's edge, named rather than left for the eye to misread: a
    // lane that simply stops is the accepted cost of a bounded picture
    // (hard choice 1), but the user is told it is happening.
    if clipped > 0 {
        lines.push(format!(
            "{clipped} {} continue outside this view",
            if clipped == 1 { "lane" } else { "lanes" }
        ));
    }
    if nothing_to_do(plan) {
        lines.push("nothing to do".to_owned());
    }
    lines.iter().map(|line| sanitize_line(line)).collect()
}

/// `2 commits replayed`, `1 merge commit`, `1 new commit` — one line per
/// kind of ghost the plan draws, in that order. Nothing when there are no
/// ghosts.
fn ghost_line(ghosts: &[GhostCommit]) -> Option<String> {
    if ghosts.is_empty() {
        return None;
    }
    let replayed = ghosts.iter().filter(|it| it.replays.is_some()).count();
    let merges = ghosts
        .iter()
        .filter(|it| it.replays.is_none() && it.parents.len() > 1)
        .count();
    let fresh = ghosts.len() - replayed - merges;
    let mut parts = Vec::new();
    if replayed > 0 {
        parts.push(format!(
            "{replayed} {} replayed",
            if replayed == 1 { "commit" } else { "commits" }
        ));
    }
    if merges > 0 {
        parts.push(format!(
            "{merges} merge {}",
            if merges == 1 { "commit" } else { "commits" }
        ));
    }
    if fresh > 0 {
        parts.push(format!(
            "{fresh} new {}",
            if fresh == 1 { "commit" } else { "commits" }
        ));
    }
    Some(parts.join(" · "))
}

/// One ref move: `topic e7e39a7 → 5 new commits`, `old a1b2c3d → deleted`,
/// `topic (new) → 3c2b1a0`.
///
/// A ref landing on a predicted commit says **how many new commits** it
/// would have, not `ghost`. `ghost` is our word: it appears nowhere else on
/// screen, and the user is left to connect `○` and `—` to a noun the
/// interface never defines (m5/20 item 4).
fn ref_line(plan: &PreviewPlan, moved: &RefMove) -> String {
    let name = String::from_utf8_lossy(&slice::short_of(&moved.name)).into_owned();
    let from = moved.from.map_or_else(|| "(new)".to_owned(), short);
    let to = match moved.to {
        None => "deleted".to_owned(),
        Some(Target::Real(oid)) => short(oid),
        Some(Target::Ghost(at)) => new_commits(new_commits_at(plan, at)),
    };
    format!("{name} {from} \u{2192} {to}")
}

/// `5 new commits`, singular at one.
fn new_commits(count: usize) -> String {
    if count == 1 {
        "1 new commit".to_owned()
    } else {
        format!("{count} new commits")
    }
}

/// How many predicted commits the ref would gain: the ghost it lands on and
/// every ghost below it in the same chain.
///
/// Walked through `parents`, not counted as `at + 1`: the plan's ghost list
/// is one flat vector, and a plan with two chains in it would otherwise
/// count the other chain's commits into this ref's total.
fn new_commits_at(plan: &PreviewPlan, at: usize) -> usize {
    let mut count = 0;
    let mut next = Some(at);
    while let Some(index) = next {
        let Some(ghost) = plan.ghosts.get(index) else {
            break;
        };
        count += 1;
        next = ghost.parents.iter().find_map(|parent| match parent {
            Target::Ghost(parent) => Some(*parent),
            Target::Real(_) => None,
        });
    }
    count
}

/// Where `HEAD` goes, when the plan moves it.
fn head_line(plan: &PreviewPlan) -> Option<String> {
    match plan.head.as_ref()? {
        HeadMove::ToBranch(name) => Some(format!(
            "HEAD \u{2192} {}",
            String::from_utf8_lossy(&slice::short_of(name))
        )),
        HeadMove::Detached(Target::Real(oid)) => Some(format!("HEAD detaches at {}", short(*oid))),
        HeadMove::Detached(Target::Ghost(at)) => Some(format!(
            "HEAD detaches at the newest of {}",
            new_commits(new_commits_at(plan, *at))
        )),
    }
}

/// What happens to the files on disk. `LOSES` is the word the user must
/// read first; the backup that makes it recoverable comes after it.
fn worktree_line(effect: &WorktreeEffect) -> String {
    match effect {
        WorktreeEffect::Untouched => "working tree: untouched".to_owned(),
        // `files_label`, not `{n} files`: "1 files updated" reads as a bug
        // in the counter, and every other count in this box already agrees
        // with its noun.
        WorktreeEffect::Updated { paths, more } => format!(
            "working tree: {} updated{}",
            graft_ui::stash_screen::files_label(paths.len()),
            if *more { ", more" } else { "" }
        ),
        // Only the stash preview draws this, through `stash_box`'s own line;
        // here for any other box that one day carries it.
        WorktreeEffect::IndexOnly { paths, more } => format!(
            "index only: {} staged{}",
            graft_ui::stash_screen::files_label(paths.len()),
            if *more { ", more" } else { "" }
        ),
        WorktreeEffect::Lost { paths } => {
            format!("working tree: LOSES {} (backup saved)", list(paths, 2))
        }
        WorktreeEffect::RestoredFromBackup => {
            "index and working tree restored from backup".to_owned()
        }
        // A new branch switched to where `HEAD` is: nothing set aside,
        // nothing rewritten (10b).
        WorktreeEffect::EditsStay => "your edits stay as they are".to_owned(),
        // Not `Untouched` and not an empty `Updated`: both read as "nothing
        // happens to your files", which is the one thing this does not mean.
        WorktreeEffect::NotShown { why } => format!("working tree: changes not listed ({why})"),
    }
}

/// The forecast, in the only terms `merge-tree` can support: `may`, never
/// `will`, and a hole named as a hole.
///
/// No `(forecast)` on the end of either line. `02-keymap.md` §2.1 (extended
/// 2026-09-21): a trailing parenthesis carries a **key**, not a word,
/// anywhere the user might press what is inside it — and `forecast` is our
/// word, not theirs (CONTEXT.md, "These are our words"). `expected` and
/// `may` already say the line is a prediction, which is all the
/// parenthetical was adding.
///
/// With edits set aside the line folds in the put-back's own forecast
/// (`PreviewPlan::put_back`, review m6-final-fixes-74 M2; the note's
/// "Preview: the conflict line folds the put-back" table): a clean operation
/// says what putting the edits back may do; an operation that may stop
/// itself was never followed to the put-back, and says so after its own
/// words, where a cut at the tail takes it first.
fn conflict_line(forecast: &ConflictForecast, put_back: Option<&ConflictForecast>) -> String {
    match (forecast, put_back) {
        (ConflictForecast::Clean, None | Some(ConflictForecast::Clean)) => {
            "no conflicts expected".to_owned()
        }
        // A put-back is one merge, never a chain: it is never `StopsAt`,
        // and would read as its paths if it were.
        (
            ConflictForecast::Clean,
            Some(ConflictForecast::Likely { paths } | ConflictForecast::StopsAt { paths, .. }),
        ) => {
            format!("your edits may clash when put back: {}", list(paths, 3))
        }
        (ConflictForecast::Clean, Some(ConflictForecast::NotChecked(why))) => {
            format!("your edits' put-back: not checked ({why})")
        }
        (ConflictForecast::Likely { paths }, put_back) => {
            with_put_back(format!("may conflict: {}", list(paths, 3)), put_back)
        }
        (ConflictForecast::NotChecked(why), put_back) => {
            with_put_back(format!("conflicts: not checked ({why})"), put_back)
        }
        // `plan_lines` draws a stop by `line_rows::forecast_rungs`, which
        // knows the ghost's subject; this is the bare wording for any other
        // caller.
        (
            ConflictForecast::StopsAt {
                step,
                of,
                commit,
                paths,
            },
            put_back,
        ) => with_put_back(
            format!(
                "may stop at step {step} of {of} ({}): {} · not checked past it",
                short(*commit),
                list(paths, 3)
            ),
            put_back,
        ),
    }
}

/// The operation's own forecast line, then ` · put-back not checked` when
/// edits are set aside: an operation that may stop is not followed further.
fn with_put_back(line: String, put_back: Option<&ConflictForecast>) -> String {
    match put_back {
        Some(_) => format!("{line} · put-back not checked"),
        None => line,
    }
}

/// What `u` would take back afterwards.
fn undo_line(note: &UndoNote) -> String {
    match note {
        UndoNote::Undoable => "undo: u takes this back".to_owned(),
        UndoNote::NotUndoable(why) => format!("not undoable: {why}"),
    }
}

/// The first `cap` paths, with `(+n)` for the rest.
fn list(paths: &[std::path::PathBuf], cap: usize) -> String {
    let shown: Vec<String> = paths
        .iter()
        .take(cap)
        .map(|path| path.display().to_string())
        .collect();
    let rest = paths.len().saturating_sub(cap);
    if rest == 0 {
        shown.join(", ")
    } else {
        format!("{} (+{rest})", shown.join(", "))
    }
}

/// Three of `paths`, then the rest: `(+n)` when the list is whole, `and
/// more` when it was cut at `PATHS_CAP` — the true count is not known then,
/// so a number would be wrong (review 86, m2).
fn capped_list(paths: &[std::path::PathBuf], more: bool) -> String {
    if more {
        format!("{} and more", list(&paths[..paths.len().min(3)], 3))
    } else {
        list(paths, 3)
    }
}

/// A commit id's first seven hex digits, the width the graph's SHA column
/// uses.
fn short(oid: Oid) -> String {
    let hex = oid.to_string();
    hex[..7.min(hex.len())].to_owned()
}

/// The box with its cursor put back where the state says it is.
///
/// [`PreviewBox`]'s cursor is private and starts on the safe choice by
/// construction (ADR-0038 §1), so it is walked rather than assigned: there
/// is deliberately no way to build a box whose acting choice is where the
/// cursor rests.
fn with_cursor(mut view: PreviewBox, cursor: usize) -> PreviewBox {
    for _ in 0..cursor {
        view.right();
    }
    view
}

// --- geometry -----------------------------------------------------------

/// Rows the graph gives up to the box this frame, footer excluded.
///
/// The box takes rows the same way the uncommitted-changes row and the
/// footer do — out of [`AppState::graph_rows`] — so the rows it covers are
/// rows the graph never drew, rather than rows drawn and then painted over.
#[must_use]
pub fn preview_rows_taken(state: &AppState) -> u16 {
    let Some(view) = preview_box(state) else {
        return 0;
    };
    if fits(state) {
        preview_rows(&view, state.size.columns)
    } else {
        // Below `PREVIEW_MIN` the widget draws one line of text, not a box.
        1
    }
}

/// Where the box is drawn: docked to the bottom of the graph area, above
/// the footer row. `None` when the area cannot hold it at all.
#[must_use]
pub fn preview_box_rect(state: &AppState, view: &PreviewBox) -> Option<BoxRect> {
    let (width, height) = (state.size.columns, state.size.rows);
    if width == 0 || height == 0 {
        return None;
    }
    // `graft_ui::preview_box::preview_box_area`'s arithmetic, in plain
    // numbers: `update` never names a drawing type (`update::tests::    // update_cannot_reach_the_world`), so the two are kept in step by
    // `tests::a_click_lands_on_the_button_the_widget_drew`, which renders
    // the real widget and reads the buttons back off the buffer.
    let footer_row = u16::from(height > 1);
    let usable = height.saturating_sub(footer_row);
    let rows = if width < PREVIEW_MIN.0 || height < PREVIEW_MIN.1 {
        usable.min(1)
    } else {
        preview_rows(view, width).max(MIN_BOX_ROWS).min(usable)
    };
    Some(BoxRect {
        x: 0,
        y: usable.saturating_sub(rows),
        width,
        height: rows,
    })
}

/// Which choice a click at `(column, row)` lands on, if any.
///
/// Mirrors `graft_ui`'s `buttons_row` layout — every choice bracketed, the
/// cursor's prefixed `▸` and the rest a space, two cells between them, the
/// whole row inset by the border and a space. `graft_ui` keeps that
/// arithmetic private, so
/// `the_click_targets_match_what_the_widget_draws` renders the real widget
/// and checks the two agree rather than trusting this copy.
fn choice_at(view: &PreviewBox, area: BoxRect, column: u16, row: u16) -> Option<usize> {
    if area.height < 2 || row.saturating_add(2) != area.bottom() {
        return None;
    }
    let mut at = area.x.saturating_add(2);
    for (index, label) in view.choices().iter().enumerate() {
        if index > 0 {
            at = at.saturating_add(2);
        }
        // `▸[ ` + label + ` ]`
        let width =
            u16::try_from(label.as_str().chars().count().saturating_add(5)).unwrap_or(u16::MAX);
        if column >= at && column < at.saturating_add(width) {
            return Some(index);
        }
        at = at.saturating_add(width);
    }
    None
}

#[cfg(test)]
pub(super) mod tests;

/// The two history keys the graph pane answers directly
/// (`docs/design/02-keymap.md` §2.3: `m` merge selected branch into current,
/// `r` rebase current onto selection).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryKey {
    /// `m`.
    Merge,
    /// `r`.
    Rebase,
}

/// Is this `m` or `r`?
#[must_use]
pub fn is_history_key(key: KeyPress) -> Option<HistoryKey> {
    if key.modifiers.control || key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char('m') => Some(HistoryKey::Merge),
        KeyCode::Char('r') => Some(HistoryKey::Rebase),
        _ => None,
    }
}

/// `m` or `r`: build the operation the selection names and open its preview.
///
/// Nothing runs here. Both keys end in a picture, and only `Confirm` in that
/// picture runs anything (PROMPT.md §1.7) — which is also why every refusal
/// below is a sentence rather than a beep: the user pressed a key that does
/// something, and is owed the reason it did not.
pub fn history_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    which: HistoryKey,
    next_id: &mut impl FnMut() -> JobId,
) {
    let verb = match which {
        HistoryKey::Merge => "m",
        HistoryKey::Rebase => "r",
    };
    match history_operation(state, which, verb) {
        Ok(op) => {
            open_preview(
                state,
                commands,
                PreviewSubject::Operation {
                    op: Box::new(op),
                    opened_by: verb,
                },
                false,
                next_id,
            );
        }
        Err(reason) => {
            state.status = Status::Sticky { reason, keys: None };
        }
    }
}

/// The operation `m` or `r` names from where the cursor is, or the sentence
/// saying what is missing.
fn history_operation(state: &AppState, which: HistoryKey, verb: &str) -> Result<Operation, String> {
    // A `p` that could not fast-forward offered these two keys and named
    // the upstream they act on. While that line is on screen they mean
    // *that* branch, wherever the cursor is — the user is answering the
    // sentence they are reading, not pointing at a row (piece 9). It comes
    // first for exactly that reason: no row is read, so no row can refuse.
    if let Some((tracking, tip)) = state.remote.diverged_upstream.as_ref() {
        let refs = state
            .refs
            .as_deref()
            .ok_or_else(|| format!("{verb} is waiting for the refs"))?;
        let current =
            current_branch(refs).ok_or_else(|| format!("{verb} needs a branch checked out"))?;
        let source = Source::Branch(super::remote::short_tracking(tracking).to_owned());
        return Ok(build(which, current, source, *tip));
    }

    if state.on_working_tree {
        return Err(format!("{verb} needs a commit row"));
    }
    let refs = state
        .refs
        .as_deref()
        .ok_or_else(|| format!("{verb} is waiting for the refs"))?;
    let selected = state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .ok_or_else(|| format!("{verb} needs a commit row"))?
        .commit
        .id;
    let current =
        current_branch(refs).ok_or_else(|| format!("{verb} needs a branch checked out"))?;
    let source = source_at(refs, selected);
    if current.oid == selected
        && matches!(source, Source::Branch(ref name) if *name == short_name_of(&current.name))
    {
        return Err("already on it".to_owned());
    }
    Ok(build(which, current, source, selected))
}

/// The operation itself, once the source and the tip it is pinned to are
/// settled — the same shape whether they came from the cursor's row or from
/// the diverged upstream a `p` named.
fn build(which: HistoryKey, current: Tip, source: Source, tip: Oid) -> Operation {
    match which {
        HistoryKey::Merge => Operation::Merge(graft_ops::operation::MergeParams {
            source,
            into: current,
            source_tip: tip,
            ff: graft_ops::operation::FastForward::Default,
        }),
        HistoryKey::Rebase => Operation::Rebase(graft_ops::operation::RebaseParams::new(
            current, source, tip,
        )),
    }
}

/// The branch `HEAD` is on, as a [`Tip`]: full ref name and the oid the
/// preview is pinned to. `None` when `HEAD` is detached or unborn — the
/// state both keys refuse in words.
pub(super) fn current_branch(refs: &RefBadges) -> Option<Tip> {
    let head = refs.head()?;
    refs.on(head).iter().find_map(|label| match label {
        RefLabel::Branch {
            name,
            current: true,
            ..
        } => Some(Tip {
            name: format!("refs/heads/{}", String::from_utf8_lossy(name)),
            oid: head,
        }),
        _ => None,
    })
}

/// What the cursor's row names on a command line: the first local branch on
/// it, else the first remote-tracking branch, else the commit itself.
///
/// A name beats an id because a name is what the user sees on the row and
/// what a reflog will record; the id is the honest answer when the row
/// carries no branch at all.
pub(super) fn source_at(refs: &RefBadges, oid: Oid) -> Source {
    let labels = refs.on(oid);
    let local = labels.iter().find_map(|label| match label {
        RefLabel::Branch { name, .. } => Some(String::from_utf8_lossy(name).into_owned()),
        _ => None,
    });
    if let Some(name) = local {
        return Source::Branch(name);
    }
    let remote = labels.iter().find_map(|label| match label {
        RefLabel::Remote(name) => Some(String::from_utf8_lossy(name).into_owned()),
        _ => None,
    });
    remote.map_or(Source::Commit(oid), Source::Branch)
}

/// `refs/heads/main` → `main`.
pub(super) fn short_name_of(full: &str) -> String {
    String::from_utf8_lossy(&slice::short_of(full)).into_owned()
}
