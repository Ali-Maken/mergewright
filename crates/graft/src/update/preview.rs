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
pub(crate) use line_rows::{ForkTone, fork_rows, onto_label};
pub mod slice;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use graft_core::commit::Commit;
use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;
use graft_core::journal::EntryId;
use graft_core::oid::Oid;
use graft_graph::row::Row;
use graft_ops::operation::{Operation, PickParams, RebaseParams, RecoverParams, Source, Tip};
use graft_ops::preview::{
    ConflictForecast, DropSuggestion, GhostCommit, HeadMove, PreviewPlan, RefMove, Target,
    UndoNote, WorktreeEffect,
};
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::preview_box::{Again, PREVIEW_MIN, PreviewBox, preview_rows};

mod answer;
pub(crate) mod boxes;
mod history;
mod keys;
mod plan_lines;
mod rect;
mod take;
pub use answer::{
    close_preview, confirm_preview, preview_badges, preview_box, preview_click, preview_produced,
};
use boxes::{
    closing_box, planning_title, put_back_box, put_in_words, ready_box, stash_box, stash_put_back,
    stash_put_back_op, stash_verb,
};
pub use history::{HistoryKey, history_key, is_history_key};
pub(super) use history::{current_branch, short_name_of, source_at};
#[cfg(test)]
use keys::confirm_key;
use keys::{enter, footer_keys, keeps_drops_here};
pub use keys::{fits, preview_key};
use plan_lines::{
    capped_list, conflict_line, list, plan_lines, ref_line, short, undo_line, with_cursor,
    worktree_line,
};
use rect::choice_at;
pub use rect::{preview_box_rect, preview_rows_taken};
use take::take_title_of;
pub(crate) use take::{is_take_key, take_key};

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

/// Why `Esc` will not stop a running put-back from J (m7/30o Minor 7):
/// `put main back to 9a8b7c6 runs to the end · u undoes it after`. A
/// put-back has no conflict and no banner, so [`HISTORY_CANNOT_STOP`]'s
/// words would name a place he never lands on; `u` is what takes it back.
pub const PUT_BACK_CANNOT_STOP: &str = "runs to the end · u undoes it after";

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

#[cfg(test)]
pub(super) mod tests;
