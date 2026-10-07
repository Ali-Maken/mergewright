//! Working-tree writes from the changes pane: `Space`/`s` stage or unstage
//! the file under the cursor, `Ctrl+A` stages every changed and untracked
//! path, `d` opens the discard confirm box
//! (m3-working-tree-undo-and-stash/04, design note "Changes pane").
//!
//! One write in flight at a time (`AppState::operating`), enforced where it
//! is set: [`claim_write`] is its only setter and refuses while a write runs
//! (m6-conflicts/78, Major 3).
//!
//! Seam rule (ARCHITECTURE.md, `crates/graft/tests/it/seam_rule.rs`): this
//! module only builds an [`Operation`] and a [`Command::Operate`] from it —
//! `graft_ops` decides what the write does, never `app`.

use std::path::{Path, PathBuf};

use graft_core::job::JobId;
use graft_git::diff::{PatchDirection, PatchError, Selection};
use graft_git::state::InProgress;
use graft_git::status::StatusEntry;
use graft_ops::apply::Refresh;
use graft_ops::operation::{Operation, PatchSide, Refusal, Target};
use graft_term::sanitize_line;
use graft_ui::detail::{BodyRow, DetailFocus, LineKindView, WorkingTreeSection};

use super::confirm::{ConfirmThen, PendingConfirm};
use super::detail::{DetailSpec, Loading};
use super::events::Command;
use super::state::{AppState, Status};
use crate::jobs::operate::Operated;

mod diff_cursor;
mod failure;
mod outcome;
mod stage_discard;
use diff_cursor::conflict_refusal;
pub use diff_cursor::{edit_target, request_discard_at_diff_cursor, stage_at_diff_cursor};
pub(super) use failure::refresh_scope;
pub(crate) use failure::{edits_clause, edits_status, refusal_status};
use failure::{operation_failed, stalled_line};
pub use outcome::operated;
pub(crate) use outcome::staging_not_kept_status;
pub use stage_discard::request_discard;
pub(crate) use stage_discard::stage_all;

/// A job ended without a `Product::Operated` — it panicked, the pool
/// stopped it, or it was cancelled. Nothing else clears `AppState::operating`,
/// so without this every write key stays a no-op for the rest of the session
/// and the only way out is quitting (m3/04 review M4).
///
/// The working tree is re-read as well: a job that died part-way through a
/// plan may have written before it did.
pub(super) fn write_job_ended(
    state: &mut AppState,
    id: JobId,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.operating != Some(id) {
        return Vec::new();
    }
    state.operating = None;
    // A write that died without answering owes no landing.
    state.lands = None;
    // A resume that died without answering: nothing will ever take it.
    if state
        .resuming
        .as_ref()
        .is_some_and(|(resume, _)| *resume == id)
    {
        state.resuming = None;
    }
    let tree = next_id();
    let mut commands = vec![super::status_read::ask(state, tree)];
    commands.extend(state.detail.refresh(next_id));
    commands
}

/// Queue one write, and say so.
///
/// One at a time (`AppState::operating`), and the status line carries
/// `<description>` from the moment the key is pressed rather than from
/// whenever the job thread gets going. Without it `Space` gives no
/// acknowledgement at all and a second `Space` looks like a dead key rather
/// than a write already running (m3/04 review M3).
///
/// `Err` while another write runs: nothing is written, nothing queued. Every
/// caller words it (`#[must_use]` lists them), so a second write can never
/// start beside the first and take `operating` from it (m6-conflicts/78).
#[must_use = "a refused launch is worded: `Status::Failed(busy.words())` or the modal's own refusal"]
pub(super) fn launch(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    op: Operation,
    next_id: &mut impl FnMut() -> JobId,
) -> Result<(), InFlight> {
    launch_with(
        next,
        commands,
        op,
        super::remote::Stoppable::Yes,
        None,
        next_id,
    )
}

/// Queue one write `Esc` must not stop, saying why in `why`.
///
/// A merge, rebase, cherry-pick, revert or time-machine jump is not one git
/// write: killing it mid-flight leaves git's own state on disk with the
/// journal entry still `Pending` and no outcome line — a shape `graft_ops`
/// does not model ([`crate::update::preview::HISTORY_CANNOT_STOP`], which
/// is the line `Esc` then shows). The refusal is words, never a key that
/// looks ignored.
///
/// `Err` while another write runs, as [`launch`].
#[must_use = "a refused launch is worded: `Status::Failed(busy.words())`"]
pub(super) fn launch_unstoppable(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    op: Operation,
    why: super::preview::CannotStop,
    opened_by: Option<&'static str>,
    next_id: &mut impl FnMut() -> JobId,
) -> Result<(), InFlight> {
    launch_with(
        next,
        commands,
        op,
        super::remote::Stoppable::No(why),
        opened_by,
        next_id,
    )
}

/// Both launches: one job id, one `Busy` line from the keystroke onwards,
/// one `Running` record, one [`Command::Operate`].
fn launch_with(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    op: Operation,
    stoppable: super::remote::Stoppable,
    opened_by: Option<&'static str>,
    next_id: &mut impl FnMut() -> JobId,
) -> Result<(), InFlight> {
    // No branch check here (gap row 550, m6-06 review Major 1): the journal
    // names a push's branch by `head_before`, so a push must send `HEAD`'s
    // branch — and `graft_ops::apply` holds exactly that, against a fresh
    // read of `HEAD`, before the `Pending` line (`check_push` refuses with
    // `Refusal::BranchChanged`). A check here could only compare against the
    // in-memory remote facts, which a re-read can replace under an open push
    // box (`update::remote::tests::a_push_boxed_before_a_reread_launches_and_ops_decides`).
    let id = next_id();
    claim_write(next, id)?;
    // Decided here, from what was asked, so `Done` never guesses the kind
    // from its answer (m7-power-tools/03, /24).
    next.lands = landing_of(&op).map(|on| (id, on));
    next.status = Status::Busy(super::remote::busy_text(&op, &next.keymap));
    commands.extend(super::remote::launched(
        next,
        id,
        op.describe(),
        Some(op.clone()),
        stoppable,
        opened_by,
    ));
    commands.push(Command::Operate(
        id,
        Box::new(op),
        std::sync::Arc::clone(&next.keymap),
    ));
    Ok(())
}

/// Where a finished write puts him (m7-power-tools/03, /24): the graph on
/// `HEAD`, the graph on one commit, or the changes pane on one file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum Landing {
    /// The graph, cursor on the new `HEAD`, as a `--continue` lands.
    #[default]
    Head,
    /// The graph, cursor on this commit: a put-back's restored commit, a new
    /// branch's tip.
    Commit(graft_core::oid::Oid),
    /// The changes pane, cursor on this file: an edit put back.
    Changes(PathBuf),
}

/// The landing `op`'s `Done` owes, decided from what was asked.
///
/// `Head` iff it moves `HEAD` ([`Operation::moves_head`]); a put-back on
/// what it restores; a branch made without switching on its tip; nothing
/// otherwise. Every kind is named, so a new one has to choose.
pub(crate) fn landing_of(op: &Operation) -> Option<Landing> {
    use graft_ops::operation::RecoverTarget;
    if op.moves_head() {
        return Some(Landing::Head);
    }
    match op {
        Operation::Recover(params) => match &params.target {
            RecoverTarget::MoveRef { to, .. } => Some(Landing::Commit(*to)),
            RecoverTarget::Recreate { at, .. } => Some(Landing::Commit(*at)),
            RecoverTarget::Files { paths, .. } => paths.first().cloned().map(Landing::Changes),
        },
        // `checkout: true` moved `HEAD` and returned above.
        Operation::BranchCreate(params) => Some(Landing::Commit(params.at)),
        Operation::Commit(_)
        | Operation::Amend(_)
        | Operation::Merge(_)
        | Operation::Rebase(_)
        | Operation::Rewrite(_)
        | Operation::CherryPick(_)
        | Operation::Revert(_)
        | Operation::Pull(_)
        | Operation::Checkout(_)
        | Operation::BranchFromStash(_)
        | Operation::FinishMerge(_)
        | Operation::Reset(_)
        | Operation::Stage(_)
        | Operation::Unstage(_)
        | Operation::Discard(_)
        | Operation::StashPush(_)
        | Operation::StashApply { .. }
        | Operation::StashPop { .. }
        | Operation::StashDrop { .. }
        | Operation::Fetch(_)
        | Operation::Push(_)
        | Operation::BranchDelete(_)
        | Operation::TagCreate(_)
        | Operation::TagDelete(_)
        | Operation::ResolveWholeFile(_)
        | Operation::RestartConflict(_)
        | Operation::AbortMerge(_)
        | Operation::DropPutBack { .. } => None,
    }
}

/// A write is already running: every other write key is refused
/// ([`claim_write`]).
#[must_use]
pub fn operation_in_flight(state: &AppState) -> bool {
    state.operating.is_some()
}

/// Why a write cannot start now: another one is running (m6-conflicts/78).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InFlight {
    /// The running write's own description, the words its `Busy` line draws
    /// (`fetch all remotes`), or `the write` when none is on record.
    pub(crate) running: String,
}

impl InFlight {
    /// `wait for <running> to finish`: the one wording of "a write is
    /// already running". `f`, the Ghost Preview's refusal, the name prompt
    /// and every refused launch say this, so none can drift.
    #[must_use]
    pub(crate) fn words(&self) -> String {
        format!("wait for {} to finish", self.running)
    }
}

/// The write running now, or `None` when one may start.
#[must_use]
pub(crate) fn in_flight(state: &AppState) -> Option<InFlight> {
    state.operating?;
    let running = state
        .remote
        .running
        .as_ref()
        .map_or_else(|| "the write".to_owned(), |it| it.description.clone());
    Some(InFlight { running })
}

/// **The only setter of `AppState::operating`** (m6-conflicts/78, Decided
/// 3): one write at a time is enforced where the slot is taken, not at each
/// key. Refuses, writing nothing, while a write runs.
pub(super) fn claim_write(state: &mut AppState, id: JobId) -> Result<(), InFlight> {
    if let Some(busy) = in_flight(state) {
        return Err(busy);
    }
    state.operating = Some(id);
    // A write starts: the `✎ changed` tags of the last put-in are over
    // (m7-power-tools/09) — undo, redo and a stage included.
    state.row_tags.clear();
    // …and so is a tag read still out: it would tag the old rows.
    super::amend_into::write_claimed(state);
    Ok(())
}

/// A refused launch, said on the status line as `f` says it
/// (`Status::Failed(InFlight::words())`), for a caller whose modal — if it
/// had one — has closed on the key. Most callers check
/// [`operation_in_flight`] first and so never see `Err`; this keeps the
/// refusal worded if that guard is ever lost, rather than silent.
pub(super) fn say_if_refused(state: &mut AppState, launched: Result<(), InFlight>) {
    if let Err(busy) = launched {
        state.status = Status::Failed(busy.words());
    }
}

/// The path under the changes-pane cursor, read from the file list the pane
/// already has — the same list its diff reads use
/// (`DetailState::selected_entry`).
fn selected_path(state: &AppState) -> Option<PathBuf> {
    let list = state.detail.list.as_ref()?;
    let entry = state.detail.selected_entry()?;
    let file = list.files.get(usize::try_from(entry).ok()?)?;
    Some(bytes_to_pathbuf(&file.path))
}

/// Every path the changes-pane cursor names: the one file of a file row,
/// every file of a section row, nothing for any other row. A section row's
/// files come out in the order the list draws them.
fn selected_paths(state: &AppState) -> Vec<PathBuf> {
    let Some(list) = state.detail.list.as_ref() else {
        return Vec::new();
    };
    let Loading::Ready(files) = &state.detail.files else {
        return Vec::new();
    };
    let path_of = |entry: u32| {
        usize::try_from(entry)
            .ok()
            .and_then(|index| list.files.get(index))
            .map(|file| bytes_to_pathbuf(&file.path))
    };
    match files.selected_entry() {
        Some(entry) => path_of(entry).into_iter().collect(),
        None => files
            .section_entries(files.cursor)
            .into_iter()
            .filter_map(path_of)
            .collect(),
    }
}

#[cfg(unix)]
fn bytes_to_pathbuf(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
fn bytes_to_pathbuf(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

/// `path`'s status entry, read from `AppState::working_tree` — the same read
/// that drives the uncommitted-changes row and the pane's header counts.
fn status_entry<'a>(state: &'a AppState, path: &Path) -> Option<&'a StatusEntry> {
    state.working_tree.as_ref()?.entry(path)
}

/// `Space`/`s` on the changes pane's file list: stage or unstage the file
/// under the cursor — or every file of the section row under it.
///
/// **Section membership decides direction**, not the file's status flags: a
/// path that is staged *and* unstaged has a row under both groups, and the
/// two rows mean opposite writes (design note, "Changes pane"). Reading the
/// flags instead would make the STAGED row of such a file stage it again.
///
/// A no-op — `false` — while a write is running, off the changes pane, or on
/// a row that names no file, so the caller can fall back to the key's other
/// meaning (folding a directory row).
pub fn stage_selected(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if operation_in_flight(next) {
        return false;
    }
    let Some(section) = next.detail.selected_section() else {
        return false;
    };
    let paths = selected_paths(next);
    if paths.is_empty() {
        return false;
    }
    let target = Target::Paths(paths);
    let op = match section {
        WorkingTreeSection::Staged => Operation::Unstage(target),
        // `Unmerged` is here on purpose: staging a conflicted path **is**
        // how git marks it resolved, and that is what `Space` on a
        // `CONFLICTS` row now means (m5-history-and-ghost-preview/09).
        // `graft_ops` is what makes it safe — `check` admits `Stage` while
        // an operation is in progress and refuses `MarkersRemain` while
        // `<<<<<<< ` is still in the file, so a half-resolved file can
        // never be marked done from here.
        WorkingTreeSection::Unstaged
        | WorkingTreeSection::Untracked
        | WorkingTreeSection::Unmerged => Operation::Stage(target),
    };
    // Unreachable `Err`: `operation_in_flight` returned above.
    let launched = launch(next, commands, op, next_id);
    say_if_refused(next, launched);
    true
}

#[cfg(test)]
mod tests;
