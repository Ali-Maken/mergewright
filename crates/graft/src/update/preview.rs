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

/// What the job came back with: the plan, and the commits it had to read to
/// anchor ghosts whose parents were not on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Planned {
    /// What the operation would do.
    pub plan: PreviewPlan,
    /// Commits read to anchor the picture, at most [`PREVIEW_ANCHOR_CAP`].
    pub anchors: Vec<Commit>,
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
    if state.operating.is_some() {
        let running = state
            .remote
            .running
            .as_ref()
            .map_or_else(|| "the write".to_owned(), |it| it.description.clone());
        state.status = Status::Sticky {
            reason: format!("wait for {running} to finish"),
            keys: None,
        };
        return false;
    }
    if let Some(kind) = state.in_progress.as_ref().map(|detail| detail.kind) {
        state.status = Status::Sticky {
            reason: format!("finish or abort the {} first", kind_word(kind)),
            keys: None,
        };
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
        ghost_index: HashMap::new(),
        clipped: 0,
        undrawable: None,
        cursor: 0,
        return_to_journal,
    });
    commands.push(Command::PlanPreview(id, Box::new(request)));
    true
}

/// git's word for the operation it is part-way through, for the refusal
/// line. `graft_git`'s own `Display` is what the commit editor's refusal
/// already says, so the two cannot drift.
fn kind_word(kind: graft_git::state::InProgress) -> String {
    kind.name().to_owned()
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
    let choices = choice_count(state);
    let Some(preview) = state.preview.as_mut() else {
        unreachable!("checked just above")
    };
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

/// The plan for an open preview landed (or failed).
///
/// Lays the slice out **once**, here, rather than per frame: `layout` is a
/// pure function over a few hundred commits and the result is what the
/// graph draws until the box closes.
pub fn preview_produced(state: &mut AppState, id: JobId, result: Result<Arc<Planned>, String>) {
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
        Err(error) => {
            preview.plan = Loading::Failed(error);
            preview.cursor = 0;
            return;
        }
    };
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
            preview.rows = graft_graph::layout(&slice.commits, seeds);
        }
        Err(SliceError::MissingParent(oid)) => {
            preview.rows = Vec::new();
            preview.ghost_index = HashMap::new();
            preview.clipped = 0;
            preview.undrawable = Some(oid);
        }
    }
    preview.plan = Loading::Ready(planned);
    preview.cursor = 0;
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
    let description = preview
        .planned()
        .map_or_else(|| "revert to".to_owned(), |it| it.plan.description.clone());
    match preview.subject {
        PreviewSubject::Operation { op, opened_by } => {
            // The same value, not a rebuilt one: `check` refuses `Moved` if
            // a tip has shifted since the picture was drawn, which is the
            // whole of hard choice 3. `opened_by` rides along so that
            // refusal can name the key to press again.
            super::operate::launch_unstoppable(
                state,
                commands,
                *op,
                HISTORY_CANNOT_STOP,
                Some(opened_by),
                next_id,
            );
        }
        PreviewSubject::RevertTo(entry) => {
            let id = next_id();
            state.operating = Some(id);
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
            } else {
                ready_box(&planned.plan, preview.clipped)
            }
        }
    };
    Some(with_cursor(view, preview.cursor))
}

/// A box with nothing to confirm: one line saying why, and `Close`.
fn closing_box(line: &str, subject: &PreviewSubject) -> PreviewBox {
    let mut view = PreviewBox::new(sanitize_line(&planning_title(subject)))
        .with_choices(vec![sanitize_line("Close")]);
    view.lines = vec![sanitize_line(line)];
    view
}

/// The box for a plan that landed.
fn ready_box(plan: &PreviewPlan, clipped: usize) -> PreviewBox {
    let title = sanitize_line(&format!("Preview: {}", plan.description));
    let mut view = if nothing_to_do(plan) {
        PreviewBox::new(title).with_choices(vec![sanitize_line("Close")])
    } else {
        PreviewBox::new(title)
    };
    view.lines = plan_lines(plan, clipped);
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

/// The wording table: ghosts, refs, `HEAD`, the working tree, the forecast,
/// the undo note, and the slice's own edge.
fn plan_lines(plan: &PreviewPlan, clipped: usize) -> Vec<SanitizedText> {
    let mut lines: Vec<String> = Vec::new();
    lines.extend(ghost_line(&plan.ghosts));
    lines.extend(plan.ref_moves.iter().map(ref_line));
    lines.extend(head_line(plan));
    lines.push(worktree_line(&plan.worktree));
    lines.push(conflict_line(&plan.conflicts));
    lines.push(undo_line(&plan.undo));
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

/// One ref move: `feature 7f6e5d4 → ghost`, `old a1b2c3d → deleted`,
/// `topic (new) → 3c2b1a0`.
fn ref_line(moved: &RefMove) -> String {
    let name = String::from_utf8_lossy(&slice::short_of(&moved.name)).into_owned();
    let from = moved.from.map_or_else(|| "(new)".to_owned(), short);
    let to = match moved.to {
        None => "deleted".to_owned(),
        Some(Target::Real(oid)) => short(oid),
        Some(Target::Ghost(_)) => "ghost".to_owned(),
    };
    format!("{name} {from} \u{2192} {to}")
}

/// Where `HEAD` goes, when the plan moves it.
fn head_line(plan: &PreviewPlan) -> Option<String> {
    match plan.head.as_ref()? {
        HeadMove::ToBranch(name) => Some(format!(
            "HEAD \u{2192} {}",
            String::from_utf8_lossy(&slice::short_of(name))
        )),
        HeadMove::Detached(Target::Real(oid)) => Some(format!("HEAD detaches at {}", short(*oid))),
        HeadMove::Detached(Target::Ghost(_)) => Some("HEAD detaches at ghost".to_owned()),
    }
}

/// What happens to the files on disk. `LOSES` is the word the user must
/// read first; the backup that makes it recoverable comes after it.
fn worktree_line(effect: &WorktreeEffect) -> String {
    match effect {
        WorktreeEffect::Untouched => "working tree: untouched".to_owned(),
        WorktreeEffect::Updated { paths, more } => format!(
            "working tree: {} files updated{}",
            paths.len(),
            if *more { ", more" } else { "" }
        ),
        WorktreeEffect::Lost { paths } => {
            format!("working tree: LOSES {} (backup saved)", list(paths, 2))
        }
        WorktreeEffect::RestoredFromBackup => {
            "index and working tree restored from backup".to_owned()
        }
        // Not `Untouched` and not an empty `Updated`: both read as "nothing
        // happens to your files", which is the one thing this does not mean.
        WorktreeEffect::NotShown { why } => format!("working tree: changes not listed ({why})"),
    }
}

/// The forecast, in the only terms `merge-tree` can support: `may`, never
/// `will`, and a hole named as a hole.
fn conflict_line(forecast: &ConflictForecast) -> String {
    match forecast {
        ConflictForecast::Clean => "no conflicts expected (forecast)".to_owned(),
        ConflictForecast::Likely { paths } => {
            format!("may conflict (forecast): {}", list(paths, 3))
        }
        ConflictForecast::NotChecked(why) => format!("conflicts: not checked ({why})"),
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
mod tests;

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
        HistoryKey::Rebase => Operation::Rebase(graft_ops::operation::RebaseParams {
            branch: current,
            onto: source,
            onto_tip: tip,
        }),
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
