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
    state.lands_on_head = None;
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
    why: &'static str,
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
    // from its answer (m7-power-tools/03).
    next.lands_on_head = op.moves_head().then_some(id);
    next.status = Status::Busy(super::remote::busy_text(&op));
    super::remote::launched(
        next,
        id,
        op.describe(),
        Some(op.clone()),
        stoppable,
        opened_by,
    );
    commands.push(Command::Operate(id, Box::new(op)));
    Ok(())
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

/// What the diff region's line cursor names: a hunk of the file on show, and
/// how much of it.
struct DiffUnit {
    /// Index into the raw [`FileDiff`]'s hunks — the same index the drawn
    /// [`graft_ui::detail::DiffView`] uses.
    hunk: usize,
    /// The whole hunk, or the one changed line under the cursor.
    selection: Selection,
    /// `true` when the cursor named one line rather than the hunk; only the
    /// wording of the status line and the confirm box differs.
    one_line: bool,
}

/// The unit under the diff cursor: the whole hunk on a `@@` heading or on a
/// context line, that one line on a `+`/`−` line. `None` off the changes
/// pane, on the truncation footer, or before a diff has arrived.
///
/// The line index it builds is into the **raw** hunk: `assemble_hunk` puts
/// the heading at `HunkView::lines[0]` and copies git's lines after it
/// one for one, so a drawn line `i` is raw line `i - 1`.
fn diff_unit(state: &AppState) -> Option<DiffUnit> {
    // Every diff has a line cursor since m7-power-tools/20b; only the
    // changes pane's is a unit of a write.
    if state.detail.spec != Some(DetailSpec::WorkingTree) {
        return None;
    }
    let cursor = state.detail.body_cursor()?;
    let Loading::Ready(view) = &state.detail.diff else {
        return None;
    };
    match view.body_row(cursor)? {
        BodyRow::Heading { hunk } => Some(DiffUnit {
            hunk,
            selection: Selection::WholeHunk,
            one_line: false,
        }),
        BodyRow::Line { hunk, line } => {
            let kind = view.hunks.get(hunk)?.lines.get(line)?.kind;
            let whole = DiffUnit {
                hunk,
                selection: Selection::WholeHunk,
                one_line: false,
            };
            match kind {
                // A context line belongs to its hunk and to nothing smaller:
                // `Space` on one means the hunk around it.
                LineKindView::Context | LineKindView::HunkHeading => Some(whole),
                LineKindView::Added | LineKindView::Removed => Some(DiffUnit {
                    hunk,
                    selection: Selection::Lines(vec![line.checked_sub(1)?]),
                    one_line: true,
                }),
            }
        }
        BodyRow::Truncation => None,
    }
}

/// What `e` opens: the file the changes-pane cursor names, and the line of it
/// under the cursor when the diff region has focus
/// (m3-working-tree-undo-and-stash/05).
///
/// The **new** side's line number, because that is the file on disk — the old
/// side's numbering belongs to a version the editor would not be opening. A
/// removed line has no new number, so the cursor lands at the top of the file
/// rather than at a line that is not there any more.
pub fn edit_target(state: &AppState) -> Option<(PathBuf, Option<u32>)> {
    if !(state.detail.shown && state.detail.spec == Some(DetailSpec::WorkingTree)) {
        return None;
    }
    let path = selected_path(state)?;
    Some((path, cursor_line(state)))
}

/// The new-side line number under the diff cursor, or `None` off the diff
/// region, on a heading, or on a removed line.
fn cursor_line(state: &AppState) -> Option<u32> {
    if state.detail.focus != DetailFocus::Diff || state.detail.spec != Some(DetailSpec::WorkingTree)
    {
        return None;
    }
    let cursor = state.detail.body_cursor()?;
    let Loading::Ready(view) = &state.detail.diff else {
        return None;
    };
    match view.body_row(cursor)? {
        BodyRow::Line { hunk, line } => view.hunks.get(hunk)?.lines.get(line)?.new_no,
        BodyRow::Heading { .. } | BodyRow::Truncation => None,
    }
}

/// Which way `graft_ops` will apply a patch cut from `section`, and which
/// diff it was cut from. Staging reads the worktree side forward; unstaging
/// and discarding both reverse what is already there.
///
/// `None` for a conflicted path: a file with conflict markers in it has no
/// clean side to cut a hunk against, so there is no patch to build and the
/// caller says so rather than guessing a side.
const fn patch_direction(section: WorkingTreeSection) -> Option<(PatchDirection, PatchSide)> {
    match section {
        WorkingTreeSection::Staged => Some((PatchDirection::Reverse, PatchSide::Index)),
        WorkingTreeSection::Unstaged | WorkingTreeSection::Untracked => {
            Some((PatchDirection::Forward, PatchSide::Worktree))
        }
        WorkingTreeSection::Unmerged => None,
    }
}

/// Why a key in the changes pane will not act on a conflicted path, in the
/// same shape as [`side_by_side_refusal`]: the key is answered and the
/// status line carries the reason, because a key that silently does nothing
/// reads as broken.
fn conflict_refusal(verb: &str) -> String {
    format!("{verb} does not act on a conflicted file \u{2014} resolve the conflict first")
}

/// Why a patch could not be built, in words that say what to do instead.
/// Never a silent no-op: the key is answered and the status line carries the
/// reason (design note, "Changes pane").
fn patch_reason(verb: &str, error: &PatchError) -> String {
    match error {
        // The one refusal a user meets in ordinary work: the last line of a
        // file with no trailing newline cannot be moved on its own without
        // stranding git's `\ No newline at end of file` marker.
        PatchError::NoNewlineBoundary { .. } => format!(
            "{verb}: this file has no newline at the end, so that line cannot go on its own — put the cursor on the hunk's @@ line to {verb} the whole hunk"
        ),
        PatchError::Binary => {
            format!("{verb}: a binary file has no hunks — {verb} the whole file from the list")
        }
        PatchError::NotAChangedLine { .. } => {
            format!("{verb}: that line is unchanged; there is nothing on it to {verb}")
        }
        PatchError::NothingSelected => format!("{verb}: nothing under the cursor to {verb}"),
        PatchError::NoSuchHunk { .. } | PatchError::NoSuchLine { .. } => {
            format!("{verb}: the diff on screen is out of date — Ctrl+R re-reads it")
        }
    }
}

/// The patch bytes for the unit under the diff cursor, or the reason there
/// are none, already worded for the status line.
fn patch_for(
    state: &AppState,
    unit: &DiffUnit,
    direction: PatchDirection,
    verb: &str,
) -> Result<Vec<u8>, String> {
    let raw = state
        .detail
        .diff_raw
        .as_ref()
        .ok_or_else(|| format!("{verb}: the diff is still loading"))?;
    graft_git::diff::hunk_patch(raw, unit.hunk, &unit.selection, direction)
        .map_err(|error| patch_reason(verb, &error))
}

/// Why a hunk-level write is refused in the side-by-side view, or `None` in
/// the unified view.
///
/// `graft_ui::detail_pane::side_by_side` draws no line cursor — there is no
/// single row to put one on once old and new sit in two columns. Acting on
/// the cursor anyway would write a hunk the user cannot see, which is the
/// same fault as review C1. Refusing in words and naming the way out beats
/// both acting unseen and a key that does nothing (m3/04 review M5).
fn side_by_side_refusal(state: &AppState, verb: &str) -> Option<String> {
    if state.detail.mode == graft_ui::detail::DiffMode::SideBySide {
        Some(format!(
            "side by side has no line cursor \u{2014} press v for the unified view to {verb} a hunk"
        ))
    } else {
        None
    }
}

/// `Space`/`s` with the diff region focused: stage or unstage the hunk or
/// line under the line cursor. Direction comes from the file's section, the
/// same rule the file list follows.
///
/// Untracked files stage whole (ADR-0034): git has no index side to cut a
/// patch against, so `Space` there means the file.
///
/// `false` only when there is nothing to act on at all; a patch that cannot
/// be built answers the key and says why in the status line, because a key
/// that silently does nothing is worse than a refusal.
pub fn stage_at_diff_cursor(
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
    if section == WorkingTreeSection::Untracked {
        return stage_selected(next, commands, next_id);
    }
    if let Some(reason) = side_by_side_refusal(next, "stage") {
        next.status = Status::sticky(reason);
        return true;
    }
    let Some(path) = selected_path(next) else {
        return false;
    };
    let Some(unit) = diff_unit(next) else {
        return false;
    };
    let verb = if section == WorkingTreeSection::Staged {
        "unstage"
    } else {
        "stage"
    };
    let Some((direction, side)) = patch_direction(section) else {
        next.status = Status::sticky(conflict_refusal(verb));
        return true;
    };
    let patch_bytes = match patch_for(next, &unit, direction, verb) {
        Ok(bytes) => bytes,
        Err(reason) => {
            next.status = Status::sticky(reason);
            return true;
        }
    };
    let target = Target::Patch {
        path,
        patch: patch_bytes,
        side,
    };
    let op = if section == WorkingTreeSection::Staged {
        Operation::Unstage(target)
    } else {
        Operation::Stage(target)
    };
    let launched = launch(next, commands, op, next_id);
    say_if_refused(next, launched);
    true
}

/// `d` with the diff region focused: the confirm box for discarding the hunk
/// or line under the line cursor. An untracked file has no hunks to discard
/// one of, so it falls back to the whole-file box (ADR-0034).
pub fn request_discard_at_diff_cursor(next: &mut AppState) -> bool {
    if operation_in_flight(next) {
        return false;
    }
    let Some(section) = next.detail.selected_section() else {
        return false;
    };
    if section == WorkingTreeSection::Untracked {
        return request_discard(next);
    }
    if let Some(reason) = side_by_side_refusal(next, "discard") {
        next.status = Status::sticky(reason);
        return true;
    }
    let Some(path) = selected_path(next) else {
        return false;
    };
    let Some(unit) = diff_unit(next) else {
        return false;
    };
    // A discard always reverses what is on disk, whichever side it came from.
    let Some((_, side)) = patch_direction(section) else {
        next.status = Status::sticky(conflict_refusal("discard"));
        return true;
    };
    let patch_bytes = match patch_for(next, &unit, PatchDirection::Reverse, "discard") {
        Ok(bytes) => bytes,
        Err(reason) => {
            next.status = Status::sticky(reason);
            return true;
        }
    };
    let counts = patch_counts(&patch_bytes);
    let unit_word = if unit.one_line { "line" } else { "hunk" };
    let shown = path.display();
    let view = graft_ui::confirm::ConfirmView::new(
        sanitize_line(&format!("Discard this {unit_word} of {shown}?")),
        vec![
            sanitize_line(&format!(
                "{} added / {} removed lines go back to how they were.",
                counts.0, counts.1
            )),
            sanitize_line("Graft keeps a backup first, so `u` can undo it afterwards."),
        ],
        sanitize_line(&discard_patch_command(side)),
    )
    // The question and the button name the same act (m3/04 review M6).
    // This box asks "Discard this hunk of …?"; a button reading `Confirm`
    // leaves the user to remember which act they are confirming.
    .with_labels("Cancel", "Discard");
    next.confirm = Some(PendingConfirm {
        view,
        then: ConfirmThen::Operate(Operation::Discard(Target::Patch {
            path,
            patch: patch_bytes,
            side,
        })),
    });
    true
}

/// Added and removed line counts of a built patch, read back off its own
/// body so the confirm box can only ever name what the patch actually
/// carries. The `+++`/`---` header lines are skipped.
fn patch_counts(patch: &[u8]) -> (usize, usize) {
    let mut added = 0;
    let mut removed = 0;
    let mut in_hunk = false;
    for line in patch.split(|byte| *byte == b'\n') {
        if line.starts_with(b"@@") {
            in_hunk = true;
            continue;
        }
        if !in_hunk {
            continue;
        }
        match line.first() {
            Some(b'+') => added += 1,
            Some(b'-') => removed += 1,
            _ => {}
        }
    }
    (added, removed)
}

/// The command the confirm box shows for a patch discard: exactly what
/// `graft_ops::operation::stage::plan_discard` runs — the working tree first,
/// then the index when the hunk came from the index.
fn discard_patch_command(side: PatchSide) -> String {
    match side {
        PatchSide::Worktree => "git apply -R -".to_string(),
        PatchSide::Index => "git apply -R -  &&  git apply --cached -R -".to_string(),
    }
}

/// Whether `Ctrl+A` stages this entry.
///
/// Not every dirty path: `git add` on a path whose only change is a **staged
/// deletion** — or a staged rename's vanished old name, which
/// `graft_ops::operation::stage::pathspec` adds alongside the new one — is
/// `fatal: pathspec '<p>' did not match any files`, exit 128, nothing staged
/// at all. One such entry would make `Ctrl+A` fail outright rather than stage
/// the rest (m3/04 review M2, verified against git 2.54).
///
/// An unmerged path is skipped for a different reason: `git add` on a
/// conflicted file marks it resolved with the conflict markers still inside.
/// Resolving a conflict is M6's, and it is not something `Ctrl+A` should do
/// by accident.
fn staged_by_stage_all(entry: &StatusEntry) -> bool {
    !entry.unmerged && (entry.unstaged.is_some() || entry.untracked)
}

/// `Ctrl+A`: stage every unstaged and untracked path
/// `AppState::working_tree` lists. `false` while a write is running, or when
/// nothing there is stageable.
pub fn stage_all(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if operation_in_flight(next) {
        return false;
    }
    let Some(status) = next.working_tree.as_ref() else {
        return false;
    };
    if status.is_clean() {
        return false;
    }
    let paths: Vec<PathBuf> = status
        .entries
        .iter()
        .filter(|entry| staged_by_stage_all(entry))
        .map(|entry| entry.path.clone())
        .collect();
    if paths.is_empty() {
        return false;
    }
    // Unreachable `Err`: `operation_in_flight` returned above.
    let launched = launch(
        next,
        commands,
        Operation::Stage(Target::Paths(paths)),
        next_id,
    );
    say_if_refused(next, launched);
    true
}

/// `d`: opens the confirm box for discarding the file under the cursor —
/// the consequence in plain words, the exact command, `Cancel` under the
/// cursor. `false` under the same conditions as [`stage_selected`], and the
/// key is free to mean something else.
pub fn request_discard(next: &mut AppState) -> bool {
    if operation_in_flight(next) {
        return false;
    }
    let Some(path) = selected_path(next) else {
        return false;
    };
    // A conflicted path has no "before" to discard to. Its entry is both
    // staged and unstaged, so the box below would offer a reset to HEAD and
    // throw away the resolution in progress; `M`/`e`/`space resolved` are
    // the conflict keys (design note). Refused in words, like `space` and
    // hunk-`d` above — never silently.
    if next.detail.selected_section() == Some(WorkingTreeSection::Unmerged) {
        next.status = Status::sticky(conflict_refusal("discard"));
        return true;
    }
    let Some(entry) = status_entry(next, &path) else {
        return false;
    };
    let command = discard_command(&path, entry);
    let counts = selected_counts(next);
    let entry = entry.clone();
    let shown = path.display();
    // The question and the button must name the same act. An untracked
    // file is deleted, not discarded, and a box reading "Delete notes.md?"
    // over a button saying `Discard` was the one box whose whole job is to
    // be unambiguous before destroying something (m3/04 review round,
    // closed here).
    let (question, verb) = if entry.untracked {
        (format!("Delete {shown}?"), "Delete")
    } else {
        (format!("Discard changes to {shown}?"), "Discard")
    };
    let mut consequence = discard_consequence(&entry, counts);
    consequence.push("Graft keeps a backup first, so `u` can undo it afterwards.".to_string());
    let view = graft_ui::confirm::ConfirmView::new(
        sanitize_line(&question),
        consequence.iter().map(|line| sanitize_line(line)).collect(),
        sanitize_line(&command),
    )
    .with_labels("Cancel", verb);
    next.confirm = Some(PendingConfirm {
        view,
        then: ConfirmThen::Operate(Operation::Discard(Target::Paths(vec![path]))),
    });
    true
}

/// The line counts of the file under the files cursor, as the pane's own file
/// list reports them: `(added, removed)`, or `None` for a binary file or
/// before the list has arrived.
fn selected_counts(state: &AppState) -> Option<(u32, u32)> {
    let list = state.detail.list.as_ref()?;
    let entry = state.detail.selected_entry()?;
    let file = list.files.get(usize::try_from(entry).ok()?)?;
    Some((file.added?, file.removed?))
}

/// What a whole-file discard actually does to this path, in plain words.
///
/// Three classes, three different consequences, and the one boilerplate line
/// this used to carry was wrong for two of them (m3/04 review M6):
///
/// - **untracked** - `git clean -f` *deletes* the file. A box that said
///   "throws away the change" never used the word the user needed.
/// - **staged and unstaged** - `plan_discard` runs
///   `restore --staged --worktree --source=HEAD`, so *both* changes go, not
///   just the one the UNSTAGED row is showing. "and the index, if staged"
///   hedged about something we already know.
/// - **staged only / unstaged only** - the one change goes, and the box says
///   which side it comes back from.
///
/// PROMPT.md section 3 asks for the consequence named. The exact command is
/// the line below this one, but this is the line that gets read.
/// ` — 1 line` / ` — 7 lines`: "1 lines" reads as a bug in the counter.
fn untracked_size(added: u32) -> String {
    if added == 1 {
        " \u{2014} 1 line".to_owned()
    } else {
        format!(" \u{2014} {added} lines")
    }
}

fn discard_consequence(entry: &StatusEntry, counts: Option<(u32, u32)>) -> Vec<String> {
    let lines = match counts {
        Some((added, removed)) => format!(" (+{added} \u{2212}{removed})"),
        None => String::new(),
    };
    if entry.untracked {
        // An untracked file has no diff against the index, so the list often
        // reports no lines at all. "0 lines" would read as a fact; silence is
        // the truth.
        let size = match counts {
            Some((added, _)) if added > 0 => untracked_size(added),
            Some(_) | None => String::new(),
        };
        return vec![format!("Deletes the file from the working tree{size}.")];
    }
    if entry.is_staged() && entry.is_unstaged() {
        return vec![format!(
            "Both the staged and the unstaged change go back to HEAD{lines}."
        )];
    }
    if entry.is_staged() {
        vec![format!(
            "The staged change goes back to how HEAD has it{lines}."
        )]
    } else {
        vec![format!(
            "The change in the working tree goes back to how the index has it{lines}."
        )]
    }
}

/// The command the confirm box shows: what `graft_ops::operation::stage`
/// actually runs for a whole-file discard of this path's class
/// (`crates/graft-ops/src/operation/stage.rs::plan_discard`).
fn discard_command(path: &Path, entry: &StatusEntry) -> String {
    let shown = path.display();
    if entry.untracked {
        format!("git clean -f -- {shown}")
    } else if entry.is_staged() {
        format!("git restore --staged --worktree --source=HEAD -- {shown}")
    } else {
        format!("git restore --worktree -- {shown}")
    }
}

/// `operated`'s `Incomplete`: the write ran, only part of its after-state
/// could not be read back.
fn incomplete(
    state: &mut AppState,
    applied: &graft_ops::apply::Applied,
    error: &str,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    state.status = Status::sticky(format!(
        "{} \u{2014} could not confirm it fully: {error}",
        applied.entry.description
    ));
    // It ran: the same reasoning as `Done`, and leaving the editor open would
    // invite a second commit of a message already committed.
    super::commit_editor::committed(state);
    refresh_scope(state, applied.refresh, next_id)
}

/// The line a clean write leaves: a clash's drop words the end of the clash;
/// a remote op its own words; else the entry's description — then, after the
/// tick, `· your edits are back` when the write set edits aside and they came
/// back clean (m6-conflicts/74).
fn done_status(
    state: &mut AppState,
    remote_op: Option<&Operation>,
    clash_drop: Option<(Option<usize>, graft_core::journal::PutBackWhy)>,
    applied: &graft_ops::apply::Applied,
) -> Status {
    if let Some((index, why)) = clash_drop {
        return super::clash::dropped_status(index, why);
    }
    Status::Done {
        what: done_what(state, remote_op, applied),
        also: super::clash::edits_back(&applied.entry).map(str::to_owned),
        keys: None,
    }
}

/// What a `Done` puts before its tick: a remote op's own words, else the
/// entry's description.
fn done_what(
    state: &mut AppState,
    remote_op: Option<&Operation>,
    applied: &graft_ops::apply::Applied,
) -> String {
    remote_op
        .and_then(|op| super::remote::remote_done(state, op))
        .unwrap_or_else(|| applied.entry.description.clone())
}

/// The key half of every [`staging_not_kept_rungs`] wording.
const STAGING_UNDO: &str = "u undoes it, staging included";

/// The fallback's status wordings, longest first (planner 30b, Decided 3).
/// `what` is what `Done` would put before its tick:
/// 1. `<what> ✓ · your edits are back · staging not kept: <paths> · u undoes it, staging included`
/// 2. `<what> ✓ · staging not kept: <paths> · u undoes it, staging included`
/// 3. `staging not kept: <paths> · u undoes it, staging included`
///
/// The cut drops `your edits are back` first: the files and the key are
/// what he must read. `<paths>` is `<first>[ +N more]`, as graft-ops'
/// `AfterState::StagingNotKept` words it (pinned by
/// `staging_not_kept_words_match_graft_ops`).
pub(crate) fn staging_not_kept_rungs(what: &str, paths: &[PathBuf]) -> Vec<String> {
    let first = paths
        .first()
        .map_or_else(String::new, |path| path.display().to_string());
    let named = match paths.len() {
        0 | 1 => first,
        n => format!("{first} +{} more", n - 1),
    };
    let lost = format!("staging not kept: {named}");
    let done_line = crate::view::status::done_line;
    vec![
        done_line(
            what,
            Some(&format!("your edits are back \u{b7} {lost}")),
            Some(STAGING_UNDO),
        ),
        done_line(what, Some(&lost), Some(STAGING_UNDO)),
        format!("{lost} \u{b7} {STAGING_UNDO}"),
    ]
}

/// The fallback's line for a status row `columns` wide: the longest rung
/// that fits the row's text (the row indents one cell), else the last —
/// `graft_ui::preview_box::fitting_line`'s rule, since the status line has
/// no ladder of its own (30b). **Sticky**, not `Done`: `Done` clears on the
/// next key, and this line is the only place that says the staging went.
/// The key half rides `keys`, so the footer's own fit keeps it whole.
pub(crate) fn staging_not_kept_status(what: &str, paths: &[PathBuf], columns: u16) -> Status {
    // `fitting_line` measures a box `width` wide whose text is 4 cells
    // narrower; the status row's text is `columns - 1` (it indents one
    // cell), so the same rule is asked with the box 3 cells wider.
    let rungs = staging_not_kept_rungs(what, paths);
    let picked = graft_ui::preview_box::fitting_line(&rungs, columns.saturating_add(3));
    let picked = picked.as_str();
    let suffix = format!(" \u{b7} {STAGING_UNDO}");
    Status::Sticky {
        reason: picked.strip_suffix(&suffix).unwrap_or(picked).to_owned(),
        keys: Some(STAGING_UNDO.to_owned()),
    }
}

/// What a write that ran leaves behind whatever its line says: the editor
/// closed, the scope re-read, and the cursor on the new `HEAD` when the
/// launch asked for it.
fn landed(
    state: &mut AppState,
    applied: &graft_ops::apply::Applied,
    lands_on_head: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    // The write the editor was open for landed, so the editor and
    // the draft behind it are about a commit that now exists
    // (m3-working-tree-undo-and-stash/05). A failure leaves both
    // alone: the message is what the user has to fix.
    super::commit_editor::committed(state);
    let commands = refresh_scope(state, applied.refresh, next_id);
    // A write that moved `HEAD` lands the cursor on its new row once
    // this refresh's refs and walk are in, exactly as a `--continue`
    // does (UX §3.1, m7-power-tools/03).
    if lands_on_head {
        super::banner::finished_lands_on_head(state, &commands);
    }
    commands
}

/// An operate job answered. Stale answers (a job that is not the one
/// `AppState::operating` is waiting for) are dropped. `Operated::Incomplete`
/// still refreshes — the write happened even though part of its after-state
/// could not be read (`ApplyError::AfterState`'s contract,
/// `.scratch/m3-working-tree-undo-and-stash/as-built.md`).
pub fn operated(
    state: &mut AppState,
    id: JobId,
    outcome: Operated,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.operating != Some(id) {
        return Vec::new();
    }
    state.operating = None;
    let lands_on_head = state.lands_on_head.take() == Some(id);
    // A write that finished before `Esc`'s stop reached it answers here as
    // usual; the cancel it outran is over.
    state.remote.cancelling = None;
    let launched_op = state
        .remote
        .running
        .as_ref()
        .filter(|running| running.id == id)
        .and_then(|running| running.op.clone());
    let remote_op = launched_op.clone().filter(Operation::is_remote);
    // The drop a stash clash's close launched (m6-conflicts/72b): its `Done`
    // is worded as the end of the clash.
    let clash_drop = state
        .clash_drop
        .take_if(|(drop, _, _)| *drop == id)
        .map(|(_, index, why)| (index, why));
    match outcome {
        Operated::Done(applied) => {
            state.status = done_status(state, remote_op.as_ref(), clash_drop, &applied);
            landed(state, &applied, lands_on_head, next_id)
        }
        // It ran and was read back; only the staging did not come back
        // (30b). Everything a `Done` does, with its own sticky line.
        Operated::StagingNotKept { applied, paths } => {
            let what = done_what(state, remote_op.as_ref(), &applied);
            state.status = staging_not_kept_status(&what, &paths, state.size.columns);
            landed(state, &applied, lands_on_head, next_id)
        }
        Operated::Incomplete { applied, error } => incomplete(state, &applied, &error, next_id),
        Operated::Conflicted {
            description,
            kind,
            paths,
            stalled,
            entry,
        } => {
            // The answer names the live entry; the journal re-read the
            // refresh asks for will say the same (gap row 505).
            state.live_entries = vec![entry];
            // A state, not a `Status` (m6-conflicts/12): git stopped and its
            // state is still there, so the stop has to outlive the refresh
            // it triggers, every line a later key puts on the status slot,
            // and an `Esc` that dismisses one of them. The footer words it
            // each frame (`banner::stop_reason`) beside the banner's keys —
            // never `esc dismiss`, which read as the way on and dropped the
            // banner's focus (`.scratch/m6-conflicts/scout-stuck-after-merge.md`).
            state.stopped = Some(super::state::Stop {
                origin: super::relaunch::LiveOrigin::ThisRun,
                again: false,
                kind,
                paths_at_stop: paths.len(),
            });
            // `Ready` also clears the launch's `Busy`, as the old line did.
            // Why git would not continue (`stalled`) is a real problem, not
            // the stop's state: it stays on the status line, dismissable.
            state.status = match stalled.as_deref() {
                Some(said) => Status::sticky(stalled_line(&description, kind, said)),
                None => Status::Ready,
            };
            // …and the banner takes the keyboard, so `c`/`s`/`a` are under
            // the user's fingers rather than something to go looking for
            // (m5-history-and-ghost-preview/09).
            state.banner_focus = super::banner::banner_focusable(kind);
            // The editor and its draft are left alone, as on a failure: no
            // commit was made, so clearing a message the user typed would be
            // throwing away their words over someone else's conflict.
            // Refs moved and the working tree moved. `Everything` is also
            // the only scope that re-reads `AppState::in_progress`, which is
            // what the commit editor's refusal names.
            let mut commands = refresh_scope(state, Refresh::Everything, next_id);
            // …and, with anything unmerged, lands in the resolver on the
            // first conflicted file (m6-conflicts/18).
            super::banner::land_in_resolver(state, &mut commands, &paths, next_id);
            commands
        }
        // A stash put back clashed: the resolver on the first clashed file,
        // no banner (m6-conflicts/72b).
        Operated::Clashed { entry, clash } => {
            super::clash::clashed(state, launched_op.as_ref(), entry, &clash, next_id)
        }
        failed @ Operated::Failed { .. } => {
            operation_failed(state, remote_op.as_ref(), failed, next_id)
        }
    }
}

/// A write that did not land: the line it leaves, and what it makes stale.
///
/// A failure is not proof the screen is still true — `ApplyError::Git` ran
/// part-way and journaled `Aborted` — so the job decides the scope, and
/// `None` is only the cases where nothing was written (m3/04 review M1, and
/// [`Operated::Failed`]'s own `refresh` field).
fn operation_failed(
    state: &mut AppState,
    remote_op: Option<&Operation>,
    failed: Operated,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let Operated::Failed {
        error,
        refresh: what,
        remote,
        refused,
        edits,
    } = failed
    else {
        // `operated` only ever hands this arm an `Operated::Failed`; the
        // match above is what guarantees it.
        return Vec::new();
    };
    state.status = match edits {
        Some(at) => edits_status(&error, at),
        None => failure_line(state, remote_op, remote.as_ref(), refused.as_ref(), error),
    };
    match what {
        Some(what) => refresh_scope(state, what, next_id),
        None => Vec::new(),
    }
}

/// The line for a set-aside that did not end in a clean put-back or a clash
/// (ADR-0045 §5 amended, m6-conflicts/84): sticky, the operation's own
/// failure first, then where `graft_ops` says the edits are. The clause is
/// the `keys` half, so at narrow widths the failure is cut and the clause
/// kept whole (`fit_status_line`). `S p` is named only when the edits are
/// not in the tree.
pub(crate) fn edits_status(failure: &str, at: graft_ops::EditsAt) -> Status {
    Status::Sticky {
        reason: failure.to_owned(),
        keys: Some(edits_clause(at)),
    }
}

/// Where a set-aside's edits are, in the app's words (keys included): the
/// clause of [`edits_status`], and of the relaunch notice when recovery
/// read where they were (`update::undo::interrupted_notice`, 30c). Same
/// state, same words.
pub(crate) fn edits_clause(at: graft_ops::EditsAt) -> String {
    use graft_ops::EditsAt;
    match at {
        EditsAt::Stash { n } => format!("your edits are in stash@{{{n}}} · S p puts them back"),
        EditsAt::Back { copy: Some(n) } => {
            format!("your edits are back · a copy stays in stash@{{{n}}}")
        }
        EditsAt::Back { copy: None } => "your edits are back · a copy may still be in S".to_owned(),
        EditsAt::Backup { copy: Some(n) } => {
            format!("u restores the backup · a copy is in stash@{{{n}}}")
        }
        EditsAt::Backup { copy: None } => "u restores the backup".to_owned(),
    }
}

/// The status line for a write that failed: the remote wording where the
/// failure came from a remote, the history wording where a precondition
/// refused it, and `graft_ops`' own sentence otherwise.
fn failure_line(
    state: &mut AppState,
    remote_op: Option<&Operation>,
    remote: Option<&graft_git::remote::RemoteFailure>,
    refused: Option<&Refusal>,
    error: String,
) -> Status {
    match (remote_op, remote) {
        (Some(op), Some(failure)) => super::remote::remote_failed(state, op, failure, &error),
        _ => history_refusal(state, refused).unwrap_or_else(|| Status::sticky(error)),
    }
}

/// The two refusals a confirmed Ghost Preview can come back with, each with
/// the key that answers it (design note, hard choices 3 and 4).
///
/// `None` for every other refusal: those already read as one sentence from
/// `Refusal::explain`, and inventing a key hint for them would be a promise
/// the app cannot keep.
///
/// The verb is the first word of `Operation::describe`, which every
/// `describe_*` in `graft_ops::operation::history` begins with (`rebase …`,
/// `merge …`, `cherry-pick …`). Taking it from there rather than writing a
/// table here is what keeps this line and the Command Log describing the
/// same operation (`KNOWN_GAPS` row 229).
fn history_refusal(state: &AppState, refused: Option<&Refusal>) -> Option<Status> {
    let running = state.remote.running.as_ref();
    match refused? {
        // The picture was drawn against a tip that has since moved, so the
        // operation the user confirmed is not the one that would run. The
        // answer is to look again — at the same key.
        Refusal::Moved { name } => Some(Status::Sticky {
            reason: format!("{} moved since the preview", short_ref(name)),
            keys: running
                .and_then(|running| running.opened_by)
                .map(|key| format!("{key} again")),
        }),
        // Hard choice 4: refuse before git runs rather than `--autostash`,
        // whose pop can conflict outside any journal entry. The refusal
        // costs one key, and that key is named.
        // The op it launched names the verb, so these words are the ones a
        // preview refused with before the confirm (m6-conflicts/67).
        refused @ Refusal::DirtyTree { .. } => running
            .and_then(|running| running.op.as_ref())
            .map(|op| refusal_status(op, refused)),
        // Marking a half-resolved file done is the one mistake that loses
        // work quietly: the markers go into the commit. The reason names
        // the file; the keys are the two ways out of it, which is exactly
        // what the user needs next.
        Refusal::MarkersRemain { path } => Some(Status::Sticky {
            reason: format!("{} still has conflict markers", path.display()),
            keys: Some("e edit  M mergetool".to_owned()),
        }),
        _ => None,
    }
}

/// `refs/heads/main` → `main`; anything else as it stands (`HEAD`).
fn short_ref(full: &str) -> &str {
    full.rsplit_once('/').map_or(full, |(_, last)| last)
}

/// The one wording of a refusal of an operation he asked for, whether the
/// preview refused it before the box opened or the write refused it after
/// the confirm.
///
/// `DirtyTree` names the verb and the key that answers it (hard choice 4:
/// refuse before git runs rather than `--autostash`, whose pop can conflict
/// outside any journal entry; the refusal costs one key, and that key is
/// named). Every other refusal is `Refusal::explain`'s one sentence.
pub(crate) fn refusal_status(op: &Operation, refusal: &Refusal) -> Status {
    match refusal {
        Refusal::DirtyTree { count } => Status::Sticky {
            reason: format!("{} needs a clean tree", verb_of(&op.describe())),
            keys: Some(format!(
                "S stash {count} changed {}",
                if *count == 1 { "file" } else { "files" }
            )),
        },
        other => Status::sticky(other.explain()),
    }
}

/// `rebase feature onto main` → `rebase`.
fn verb_of(description: &str) -> &str {
    description.split_whitespace().next().unwrap_or(description)
}

/// The status line for an operation git stopped part-way through **and
/// would not carry on**: `git <kind> --continue` said why, in its own words.
///
/// Only that case is a line now. A plain conflict stop is
/// [`AppState::stopped`] and the footer words it beside the banner's keys
/// (m6-conflicts/12); git's refusal is a real problem on top of the stop,
/// and a sticky line is what keeps its words on screen until `Esc`.
fn stalled_line(description: &str, kind: InProgress, said: &str) -> String {
    let what = kind.name();
    format!(
        "{description} \u{2014} git would not continue: {said}; the {what} is still in progress"
    )
}

/// Re-read what the write changed.
///
/// [`Refresh::Everything`] means refs moved, which is exactly the state
/// `Ctrl+R` rebuilds — so it runs `Ctrl+R`'s own path rather than a
/// hand-written subset of it. The subset was missing the walk cancellation
/// (a second history walk started while the first still ran), the repository
/// re-open and the Mistake Radar's default-branch reset, which goes stale the
/// moment a commit moves the tip (m3/04 review M8).
pub(super) fn refresh_scope(
    state: &mut AppState,
    what: Refresh,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match what {
        Refresh::WorkingTree => {
            // Every write journals an entry — an aborted one too — so the
            // browser and the interrupted notice are stale the moment one
            // finishes (m3-working-tree-undo-and-stash/06). The
            // `Everything` arm gets the same read through `refresh_key`.
            let mut commands = state.journal.read(next_id);
            // A stash push, apply, pop or drop moves `refs/stash`, and the
            // overlay is what the user is looking at when they press one
            // (m3-working-tree-undo-and-stash/06).
            super::stash::refresh(state, &mut commands, next_id);
            let tree = next_id();
            commands.push(super::status_read::ask(state, tree));
            commands.extend(state.detail.refresh(next_id));
            commands
        }
        Refresh::Everything => {
            let mut commands = Vec::new();
            super::keys::refresh_key(state, &mut commands, next_id);
            commands
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use graft_core::job::JobId;
    use graft_git::diff::{DiffLine, FileDiff, FileEntry, FileList, FileStatus, Hunk, LineKind};
    use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
    use graft_ui::detail::{FileListView, FileRow, FileRowKind, FileStatusView};

    use super::*;
    use crate::update::detail::DetailSpec;

    /// An `Applied` naming `description`, with `refresh` as its scope. Only
    /// those two fields are read by `operated`; the rest is a plausible
    /// journal entry.
    fn applied(description: &str, refresh: Refresh) -> graft_ops::apply::Applied {
        use graft_core::journal::{
            EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches,
        };
        graft_ops::apply::Applied {
            entry: JournalEntry {
                id: EntryId::new(1, [0; 10]),
                at_unix_ms: 1,
                kind: OperationKind::Stage,
                description: description.to_owned(),
                status: EntryStatus::Committed,
                touches: Touches::default(),
                head_before: HeadRef::Branch("refs/heads/main".to_owned()),
                head_after: None,
                refs_before: std::collections::BTreeMap::new(),
                refs_after: std::collections::BTreeMap::new(),
                index_before: None,
                index_after: None,
                worktree_before: None,
                worktree_after: None,
                backup_ref: None,
                pins: Vec::new(),
                commands: Vec::new(),
                reverses: None,
                merge_before: None,
                merge_after: None,
                put_back: None,
            },
            refresh,
            step_backup: None,
            replaced: graft_ops::undo::Replaced::default(),
        }
    }

    /// A changes pane holding one section with one file row under it, the
    /// cursor on the file row, and `entry` behind `AppState::working_tree` —
    /// the shape `selected_paths`/`status_entry` read.
    fn state_on(path: &str, entry: StatusEntry, section: WorkingTreeSection) -> AppState {
        // Row 0 is the section heading, row 1 the file.
        state_with(&[(path.to_string(), section)], vec![entry], 1)
    }

    /// A changes pane built row by row: `rows` is `(path, section)` in draw
    /// order, a section heading inserted whenever the section changes.
    /// `cursor` is the row the pane's cursor sits on.
    fn state_with(
        rows: &[(String, WorkingTreeSection)],
        entries: Vec<StatusEntry>,
        cursor: usize,
    ) -> AppState {
        let (files, view_rows) = sectioned_rows(rows);
        let count = u32::try_from(files.len()).expect("a test list is short");
        let list = FileList {
            files,
            added: 1,
            removed: 0,
        };
        let view = FileListView {
            rows: view_rows,
            cursor,
            top: 0,
            files: count,
            added: 1,
            removed: 0,
            message: Vec::new(),
            compare: None,
        };
        let mut state = AppState {
            working_tree: Some(Arc::new(WorkingTreeStatus { entries })),
            ..AppState::default()
        };
        state.detail.spec = Some(DetailSpec::WorkingTree);
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Files;
        state.detail.files = crate::update::detail::Loading::Ready(Arc::new(view));
        state.detail.list = Some(Arc::new(list));
        state
    }

    /// The git file list and the drawn rows for `rows`, a section heading
    /// inserted wherever the section changes.
    fn sectioned_rows(rows: &[(String, WorkingTreeSection)]) -> (Vec<FileEntry>, Vec<FileRow>) {
        let mut files = Vec::new();
        let mut view_rows = Vec::new();
        let mut current: Option<WorkingTreeSection> = None;
        for (path, section) in rows {
            if current != Some(*section) {
                view_rows.push(FileRow {
                    depth: 0,
                    name: graft_term::sanitize::sanitize_line(section.title()),
                    kind: FileRowKind::Section {
                        section: *section,
                        count: rows.iter().filter(|(_, s)| s == section).count(),
                    },
                    entry: None,
                });
                current = Some(*section);
            }
            let index = u32::try_from(files.len()).expect("a test list is short");
            files.push(FileEntry {
                path: path.as_bytes().to_vec(),
                old_path: None,
                status: FileStatus::Modified,
                added: Some(1),
                removed: Some(0),
            });
            view_rows.push(FileRow {
                depth: 1,
                name: graft_term::sanitize::sanitize_line(path),
                kind: FileRowKind::File {
                    status: FileStatusView::Modified,
                    added: Some(1),
                    removed: Some(0),
                },
                entry: Some(index),
            });
        }
        (files, view_rows)
    }

    fn unstaged_entry(path: &str) -> StatusEntry {
        StatusEntry {
            path: path.into(),
            original_path: None,
            staged: None,
            unstaged: Some(Change::Modified),
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }
    }

    fn staged_entry(path: &str) -> StatusEntry {
        StatusEntry {
            path: path.into(),
            original_path: None,
            staged: Some(Change::Modified),
            unstaged: None,
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }
    }

    fn untracked_entry(path: &str) -> StatusEntry {
        StatusEntry {
            path: path.into(),
            original_path: None,
            staged: None,
            unstaged: None,
            untracked: true,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }
    }

    fn ids() -> impl FnMut() -> JobId {
        let mut next = 1u64;
        move || {
            let id = JobId::new(next);
            next += 1;
            id
        }
    }

    fn line(kind: LineKind, text: &str, old_no: Option<u32>, new_no: Option<u32>) -> DiffLine {
        DiffLine {
            kind,
            text: text.as_bytes().to_vec(),
            old_no,
            new_no,
            no_newline_at_eof: false,
        }
    }

    /// `src/a.rs` with one hunk: a context line, a `-`/`+` pair, a context
    /// line. Body rows: `0` the `@@` heading, `1` context, `2` the removed
    /// line, `3` the added line, `4` context.
    fn one_pair_diff() -> FileDiff {
        FileDiff {
            entry: FileEntry {
                path: b"src/a.rs".to_vec(),
                old_path: None,
                status: FileStatus::Modified,
                added: Some(1),
                removed: Some(1),
            },
            hunks: vec![Hunk {
                old_start: 1,
                old_lines: 3,
                new_start: 1,
                new_lines: 3,
                heading: Vec::new(),
                lines: vec![
                    line(LineKind::Context, "one", Some(1), Some(1)),
                    line(LineKind::Removed, "old", Some(2), None),
                    line(LineKind::Added, "new", None, Some(2)),
                    line(LineKind::Context, "two", Some(3), Some(3)),
                ],
            }],
            binary: None,
            truncated: None,
            old_mode: None,
            new_mode: None,
        }
    }

    /// `src/a.rs` whose only line changed and whose file ends without a
    /// newline on both sides — the shape `hunk_patch` refuses a single-line
    /// selection on.
    fn no_trailing_newline_diff() -> FileDiff {
        let mut removed = line(LineKind::Removed, "old", Some(1), None);
        removed.no_newline_at_eof = true;
        let mut added = line(LineKind::Added, "new", None, Some(1));
        added.no_newline_at_eof = true;
        FileDiff {
            hunks: vec![Hunk {
                old_start: 1,
                old_lines: 1,
                new_start: 1,
                new_lines: 1,
                heading: Vec::new(),
                lines: vec![removed, added],
            }],
            ..one_pair_diff()
        }
    }

    /// The changes pane with the diff region focused, `diff` on show and the
    /// line cursor on body row `cursor`.
    fn state_on_diff(section: WorkingTreeSection, diff: FileDiff, cursor: usize) -> AppState {
        let path = String::from_utf8(diff.entry.path.clone()).expect("a test path is utf-8");
        let entry = match section {
            WorkingTreeSection::Staged => staged_entry(&path),
            WorkingTreeSection::Unstaged => unstaged_entry(&path),
            WorkingTreeSection::Untracked => untracked_entry(&path),
            WorkingTreeSection::Unmerged => unmerged_entry(&path),
        };
        let mut state = state_on(&path, entry, section);
        state.detail.focus = DetailFocus::Diff;
        let view = crate::jobs::detail::assemble_diff(&diff, false);
        state.detail.diff = Loading::Ready(Arc::new(view));
        state.detail.diff_raw = Some(Arc::new(diff));
        state.detail.body_cursor = cursor;
        state
    }

    /// The `Target::Patch` the one queued command carries.
    fn queued_patch(commands: &[Command]) -> (&PathBuf, &Vec<u8>, PatchSide) {
        let [Command::Operate(_, op)] = commands else {
            panic!("expected exactly one Operate, got {commands:?}");
        };
        let (Operation::Stage(target) | Operation::Unstage(target) | Operation::Discard(target)) =
            &**op
        else {
            panic!("expected a stage/unstage/discard");
        };
        let Target::Patch { path, patch, side } = target else {
            panic!("expected a patch target, got {target:?}");
        };
        (path, patch, *side)
    }

    /// `Space` on a conflicted row **marks it resolved**: staging an
    /// unmerged path is exactly how git records a resolution, and that is
    /// the key the design note gives it ("`space` marks resolved",
    /// m5-history-and-ghost-preview/09).
    ///
    /// Group 04 left this refusing, because the operation that makes it safe
    /// did not exist yet. It does now: `graft_ops`' `check` admits `Stage`
    /// while an operation is in progress and refuses `MarkersRemain` while
    /// `<<<<<<< ` is still in the file, so the refusal moved from "this key
    /// is not for that" to "that file is not finished" — a fact about the
    /// file rather than about the key.
    #[test]
    fn space_on_a_conflicted_file_stages_it_as_the_resolution() {
        let mut state = state_on(
            "src/auth.rs",
            unmerged_entry("src/auth.rs"),
            WorkingTreeSection::Unmerged,
        );
        let mut commands = Vec::new();
        assert!(
            stage_selected(&mut state, &mut commands, &mut ids()),
            "the key is answered, not ignored"
        );
        let [Command::Operate(_, op)] = commands.as_slice() else {
            panic!("expected one staging operation: {commands:?}");
        };
        assert!(
            matches!(&**op, Operation::Stage(Target::Paths(paths))
                if paths == &[PathBuf::from("src/auth.rs")]),
            "{op:?}"
        );
        assert!(state.operating.is_some(), "the write is in flight");
    }

    /// `d` on a conflicted **file row** is refused too. A `UU` entry is both
    /// staged and unstaged, so the discard box would otherwise offer "both
    /// go back to HEAD" and `git restore --staged --worktree
    /// --source=HEAD` — throwing away the resolution in progress. The key
    /// is answered and says why (M5 group 04 review, Major 3).
    #[test]
    fn d_on_a_conflicted_file_row_says_why_instead_of_offering_a_reset_to_head() {
        let mut state = state_on(
            "src/auth.rs",
            unmerged_entry("src/auth.rs"),
            WorkingTreeSection::Unmerged,
        );
        assert!(
            request_discard(&mut state),
            "the key is answered, not ignored"
        );
        assert!(state.confirm.is_none(), "no discard box opened");
        let Status::Sticky { reason, .. } = &state.status else {
            panic!("expected a sticky reason, got {:?}", state.status);
        };
        assert!(reason.contains("conflicted file"), "{reason}");
    }

    /// The same for `d` on a hunk of a conflicted file: there is no clean
    /// side to cut a patch against, so the box never opens and the status
    /// line says so.
    #[test]
    fn d_in_the_diff_region_of_a_conflicted_file_says_why_instead_of_asking() {
        let mut state = state_on_diff(WorkingTreeSection::Unmerged, one_pair_diff(), 0);
        assert!(
            request_discard_at_diff_cursor(&mut state),
            "the key is answered, not ignored"
        );
        assert!(state.confirm.is_none(), "no box opened");
        let Status::Sticky { reason, .. } = &state.status else {
            panic!("expected a sticky reason, got {:?}", state.status);
        };
        assert!(reason.contains("conflicted file"), "{reason}");
    }

    #[test]
    fn space_on_a_hunk_heading_stages_the_whole_hunk() {
        let diff = one_pair_diff();
        let mut state = state_on_diff(WorkingTreeSection::Unstaged, diff.clone(), 0);
        let mut commands = Vec::new();
        assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
        let (target_path, patch, side) = queued_patch(&commands);
        assert_eq!(target_path, &PathBuf::from("src/a.rs"));
        assert_eq!(side, PatchSide::Worktree);
        assert_eq!(
            patch,
            &graft_git::diff::hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward)
                .expect("the whole hunk always builds")
        );
    }

    #[test]
    fn space_on_an_added_line_stages_that_line_alone() {
        let diff = one_pair_diff();
        // Body row 3 is the `+` line; raw hunk line 2.
        let mut state = state_on_diff(WorkingTreeSection::Unstaged, diff.clone(), 3);
        let mut commands = Vec::new();
        assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
        let (_, patch, _) = queued_patch(&commands);
        assert_eq!(
            patch,
            &graft_git::diff::hunk_patch(
                &diff,
                0,
                &Selection::Lines(vec![2]),
                PatchDirection::Forward
            )
            .expect("one added line builds")
        );
    }

    #[test]
    fn space_on_a_context_line_stages_the_hunk_around_it() {
        let diff = one_pair_diff();
        // Body row 1 is the leading context line.
        let mut state = state_on_diff(WorkingTreeSection::Unstaged, diff.clone(), 1);
        let mut commands = Vec::new();
        assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
        let (_, patch, _) = queued_patch(&commands);
        assert_eq!(
            patch,
            &graft_git::diff::hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward)
                .expect("the whole hunk always builds")
        );
    }

    #[test]
    fn a_hunk_under_staged_unstages_in_reverse_against_the_index() {
        let diff = one_pair_diff();
        let mut state = state_on_diff(WorkingTreeSection::Staged, diff.clone(), 0);
        let mut commands = Vec::new();
        assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
        let [Command::Operate(_, op)] = commands.as_slice() else {
            panic!("expected one Operate");
        };
        assert!(
            matches!(&**op, Operation::Unstage(_)),
            "the STAGED section unstages: {op:?}"
        );
        let (_, patch, side) = queued_patch(&commands);
        assert_eq!(side, PatchSide::Index);
        assert_eq!(
            patch,
            &graft_git::diff::hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Reverse)
                .expect("the whole hunk always builds")
        );
    }

    /// The one refusal a user meets in ordinary work. It must read as a
    /// refusal with a way out, never as a key that did nothing.
    #[test]
    fn a_single_line_at_a_missing_final_newline_is_refused_in_words() {
        // Body row 2 is the `+` line of a file with no trailing newline.
        let mut state = state_on_diff(WorkingTreeSection::Unstaged, no_trailing_newline_diff(), 2);
        let mut commands = Vec::new();
        assert!(
            stage_at_diff_cursor(&mut state, &mut commands, &mut ids()),
            "the key is answered, not ignored"
        );
        assert!(commands.is_empty(), "nothing is written");
        assert_eq!(state.operating, None);
        let Status::Sticky { reason: text, .. } = &state.status else {
            panic!(
                "expected a failure on the status line, got {:?}",
                state.status
            );
        };
        assert!(
            text.contains("no newline at the end") && text.contains("whole hunk"),
            "the reason names the cause and the way out: {text}"
        );
    }

    #[test]
    fn the_whole_hunk_still_stages_where_a_single_line_cannot() {
        let mut state = state_on_diff(WorkingTreeSection::Unstaged, no_trailing_newline_diff(), 0);
        let mut commands = Vec::new();
        assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
        assert_eq!(commands.len(), 1, "the heading row still writes");
    }

    #[test]
    fn d_in_the_diff_region_confirms_before_discarding_the_hunk() {
        let mut state = state_on_diff(WorkingTreeSection::Unstaged, one_pair_diff(), 0);
        assert!(request_discard_at_diff_cursor(&mut state));
        let pending = state.confirm.expect("confirm box opened");
        assert_eq!(pending.view.selected(), 0);
        assert!(
            pending.view.title.as_str().contains("hunk of src/a.rs"),
            "the question names what goes: {:?}",
            pending.view.title
        );
        assert_eq!(pending.view.command.as_str(), "git apply -R -");
        // The question and the button name the same act (m3/04 review M6):
        // "Discard this hunk of …?" over a `Confirm` button leaves the user
        // to remember what they are confirming.
        assert_eq!(pending.view.choices[0].as_str(), "Cancel");
        assert_eq!(pending.view.choices[1].as_str(), "Discard");
        assert!(
            matches!(
                pending.then,
                ConfirmThen::Operate(Operation::Discard(Target::Patch {
                    side: PatchSide::Worktree,
                    ..
                }))
            ),
            "the working tree's own side is reversed: {:?}",
            pending.then
        );
    }

    #[test]
    fn d_on_one_line_names_the_line_and_counts_it() {
        // Body row 3, the `+` line.
        let mut state = state_on_diff(WorkingTreeSection::Unstaged, one_pair_diff(), 3);
        assert!(request_discard_at_diff_cursor(&mut state));
        let pending = state.confirm.expect("confirm box opened");
        assert!(
            pending.view.title.as_str().contains("line of src/a.rs"),
            "{:?}",
            pending.view.title
        );
        let said: Vec<String> = pending
            .view
            .message
            .iter()
            .map(|line| line.as_str().to_string())
            .collect();
        assert!(
            said.iter()
                .any(|line| line.starts_with("1 added / 0 removed")),
            "the consequence counts the lines actually in the patch: {said:?}"
        );
    }

    /// ADR-0034: untracked files have no index side to cut a hunk against,
    /// so the diff region's `Space` means the whole file there.
    #[test]
    fn space_in_an_untracked_files_diff_stages_the_whole_file() {
        let mut state = state_on_diff(WorkingTreeSection::Untracked, one_pair_diff(), 0);
        let mut commands = Vec::new();
        assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
        let [Command::Operate(_, op)] = commands.as_slice() else {
            panic!("expected one Operate");
        };
        assert_eq!(
            **op,
            Operation::Stage(Target::Paths(vec!["src/a.rs".into()]))
        );
    }

    #[test]
    fn stage_selected_stages_an_unstaged_file() {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let mut commands = Vec::new();
        assert!(stage_selected(&mut state, &mut commands, &mut ids()));
        assert!(state.operating.is_some());
        let Command::Operate(_, op) = &commands[0] else {
            panic!("expected Operate");
        };
        assert_eq!(
            **op,
            Operation::Stage(Target::Paths(vec!["src/a.rs".into()]))
        );
    }

    #[test]
    fn stage_selected_unstages_a_row_under_the_staged_section() {
        let mut state = state_on(
            "src/a.rs",
            staged_entry("src/a.rs"),
            WorkingTreeSection::Staged,
        );
        let mut commands = Vec::new();
        assert!(stage_selected(&mut state, &mut commands, &mut ids()));
        let Command::Operate(_, op) = &commands[0] else {
            panic!("expected Operate");
        };
        assert_eq!(
            **op,
            Operation::Unstage(Target::Paths(vec!["src/a.rs".into()]))
        );
    }

    /// A path staged *and* unstaged draws a row under both sections. The
    /// status flags are the same for both rows, so only the section can say
    /// which way the write goes.
    #[test]
    fn section_not_status_decides_direction_for_a_file_in_both() {
        let both = StatusEntry {
            path: "src/a.rs".into(),
            original_path: None,
            staged: Some(Change::Modified),
            unstaged: Some(Change::Modified),
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        };
        let rows = vec![
            ("src/a.rs".to_string(), WorkingTreeSection::Staged),
            ("src/a.rs".to_string(), WorkingTreeSection::Unstaged),
        ];

        // Row 1 is the file under STAGED, row 3 the same file under UNSTAGED.
        let mut staged_row = state_with(&rows, vec![both.clone()], 1);
        let mut commands = Vec::new();
        assert!(stage_selected(&mut staged_row, &mut commands, &mut ids()));
        let Command::Operate(_, op) = &commands[0] else {
            panic!("expected Operate");
        };
        assert_eq!(
            **op,
            Operation::Unstage(Target::Paths(vec!["src/a.rs".into()])),
            "the STAGED row unstages"
        );

        let mut unstaged_row = state_with(&rows, vec![both], 3);
        let mut commands = Vec::new();
        assert!(stage_selected(&mut unstaged_row, &mut commands, &mut ids()));
        let Command::Operate(_, op) = &commands[0] else {
            panic!("expected Operate");
        };
        assert_eq!(
            **op,
            Operation::Stage(Target::Paths(vec!["src/a.rs".into()])),
            "the UNSTAGED row stages"
        );
    }

    #[test]
    fn stage_selected_on_a_section_row_takes_every_file_under_it() {
        let rows = vec![
            ("src/a.rs".to_string(), WorkingTreeSection::Unstaged),
            ("src/b.rs".to_string(), WorkingTreeSection::Unstaged),
            ("new.txt".to_string(), WorkingTreeSection::Untracked),
        ];
        let entries = vec![
            unstaged_entry("src/a.rs"),
            unstaged_entry("src/b.rs"),
            untracked_entry("new.txt"),
        ];
        // Row 0 is the UNSTAGED heading; its two files follow.
        let mut state = state_with(&rows, entries, 0);
        let mut commands = Vec::new();
        assert!(stage_selected(&mut state, &mut commands, &mut ids()));
        let Command::Operate(_, op) = &commands[0] else {
            panic!("expected Operate");
        };
        assert_eq!(
            **op,
            Operation::Stage(Target::Paths(vec!["src/a.rs".into(), "src/b.rs".into()])),
            "the untracked file below the next heading is not swept in"
        );
    }

    #[test]
    fn stage_selected_is_a_noop_while_a_write_is_already_running() {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        // The first `Space` launched the write and said so; the second must
        // change nothing, and the line it left standing is what tells the
        // user why (m3/04 review M3).
        let mut commands = Vec::new();
        assert!(stage_selected(&mut state, &mut commands, &mut ids()));
        assert_eq!(state.status, Status::Busy("stage src/a.rs".into()));

        let mut again = Vec::new();
        assert!(!stage_selected(&mut state, &mut again, &mut ids()));
        assert!(again.is_empty());
        assert_eq!(state.status, Status::Busy("stage src/a.rs".into()));
    }

    #[test]
    fn stage_all_stages_every_path_in_status() {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let mut commands = Vec::new();
        assert!(stage_all(&mut state, &mut commands, &mut ids()));
        let Command::Operate(_, op) = &commands[0] else {
            panic!("expected Operate");
        };
        assert_eq!(
            **op,
            Operation::Stage(Target::Paths(vec!["src/a.rs".into()]))
        );
    }

    #[test]
    fn stage_all_is_a_noop_on_a_clean_tree() {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        state.working_tree = Some(Arc::new(WorkingTreeStatus::default()));
        let mut commands = Vec::new();
        assert!(!stage_all(&mut state, &mut commands, &mut ids()));
    }

    #[test]
    fn request_discard_names_clean_for_an_untracked_file() {
        let mut state = state_on(
            "new.txt",
            untracked_entry("new.txt"),
            WorkingTreeSection::Untracked,
        );
        assert!(request_discard(&mut state));
        let pending = state.confirm.expect("confirm box opened");
        assert_eq!(pending.view.command.as_str(), "git clean -f -- new.txt");
        assert_eq!(
            pending.then,
            ConfirmThen::Operate(Operation::Discard(Target::Paths(vec!["new.txt".into()])))
        );
    }

    #[test]
    fn request_discard_names_restore_staged_for_a_staged_file() {
        let mut state = state_on(
            "src/a.rs",
            staged_entry("src/a.rs"),
            WorkingTreeSection::Staged,
        );
        assert!(request_discard(&mut state));
        let pending = state.confirm.expect("confirm box opened");
        assert_eq!(
            pending.view.command.as_str(),
            "git restore --staged --worktree --source=HEAD -- src/a.rs"
        );
    }

    #[test]
    fn request_discard_opens_with_cancel_under_the_cursor() {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        assert!(request_discard(&mut state));
        let pending = state.confirm.expect("confirm box opened");
        assert_eq!(pending.view.selected(), 0);
    }

    fn deleted_and_staged(path: &str) -> StatusEntry {
        StatusEntry {
            path: path.into(),
            original_path: None,
            staged: Some(Change::Deleted),
            unstaged: None,
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }
    }

    fn unmerged_entry(path: &str) -> StatusEntry {
        StatusEntry {
            path: path.into(),
            original_path: None,
            staged: None,
            unstaged: None,
            untracked: false,
            unmerged: true,
            unmerged_how: None,
            submodule: None,
        }
    }

    #[test]
    fn stage_all_skips_the_paths_git_add_would_refuse() {
        // m3/04 review M2: `git add` on a staged deletion is
        // `fatal: pathspec ... did not match any files`, exit 128, nothing
        // staged — so one such entry used to make the whole `Ctrl+A` fail.
        // An unmerged path is skipped too: `git add` there marks the conflict
        // resolved with the markers still in the file.
        let mut state = state_with(
            &[("src/a.rs".into(), WorkingTreeSection::Unstaged)],
            vec![
                deleted_and_staged("gone.rs"),
                unmerged_entry("conflict.rs"),
                unstaged_entry("src/a.rs"),
                untracked_entry("new.txt"),
            ],
            1,
        );
        let mut commands = Vec::new();
        assert!(stage_all(&mut state, &mut commands, &mut ids()));
        let Command::Operate(_, op) = &commands[0] else {
            panic!("expected Operate");
        };
        assert_eq!(
            **op,
            Operation::Stage(Target::Paths(vec!["src/a.rs".into(), "new.txt".into()]))
        );
    }

    #[test]
    fn stage_all_refuses_when_nothing_left_is_stageable() {
        let mut state = state_with(
            &[("gone.rs".into(), WorkingTreeSection::Staged)],
            vec![deleted_and_staged("gone.rs")],
            1,
        );
        let mut commands = Vec::new();
        assert!(!stage_all(&mut state, &mut commands, &mut ids()));
        assert!(commands.is_empty());
        assert_eq!(state.operating, None);
    }

    #[test]
    fn side_by_side_refuses_a_hunk_write_instead_of_acting_unseen() {
        // m3/04 review M5: the side-by-side view draws no line cursor, so a
        // `Space` there would stage a hunk the user cannot see.
        for section in [WorkingTreeSection::Unstaged, WorkingTreeSection::Staged] {
            let mut state = state_on_diff(section, one_pair_diff(), 1);
            state.detail.mode = graft_ui::detail::DiffMode::SideBySide;
            let mut commands = Vec::new();
            assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
            assert!(commands.is_empty(), "{commands:?}");
            assert_eq!(state.operating, None);
            let Status::Sticky { reason: said, .. } = &state.status else {
                panic!("expected a refusal, got {:?}", state.status);
            };
            assert!(said.contains("press v"), "{said}");

            let mut state = state_on_diff(section, one_pair_diff(), 1);
            state.detail.mode = graft_ui::detail::DiffMode::SideBySide;
            assert!(request_discard_at_diff_cursor(&mut state));
            assert!(state.confirm.is_none());
            let Status::Sticky { reason: said, .. } = &state.status else {
                panic!("expected a refusal, got {:?}", state.status);
            };
            assert!(said.contains("press v"), "{said}");
        }
    }

    #[test]
    fn the_discard_box_names_deletion_for_an_untracked_file() {
        // m3/04 review M6: `git clean -f` deletes it. The box never said so.
        let mut state = state_on(
            "new.txt",
            untracked_entry("new.txt"),
            WorkingTreeSection::Untracked,
        );
        assert!(request_discard(&mut state));
        let pending = state.confirm.as_ref().expect("a box opened");
        assert_eq!(pending.view.title.as_str(), "Delete new.txt?");
        // The question and the button say the same thing (m3/04 review
        // round; the box used to ask "Delete" over a `Discard` button).
        assert_eq!(pending.view.choices[1].as_str(), "Delete");
        assert_eq!(pending.view.choices[0].as_str(), "Cancel");
        let said = pending
            .view
            .message
            .iter()
            .map(graft_term::SanitizedText::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(said.contains("Deletes the file"), "{said}");
        assert!(said.contains("backup"), "{said}");
    }

    #[test]
    fn a_tracked_file_still_asks_to_discard_and_says_discard_on_the_button() {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        assert!(request_discard(&mut state));
        let pending = state.confirm.as_ref().expect("a box opened");
        assert_eq!(pending.view.title.as_str(), "Discard changes to src/a.rs?");
        assert_eq!(pending.view.choices[1].as_str(), "Discard");
    }

    #[test]
    fn the_discard_box_says_both_changes_go_for_a_file_staged_and_unstaged() {
        // m3/04 review M6: from the UNSTAGED row the pane shows only the
        // unstaged diff, but `plan_discard` runs
        // `restore --staged --worktree --source=HEAD` and both go.
        let both = StatusEntry {
            path: "src/a.rs".into(),
            original_path: None,
            staged: Some(Change::Modified),
            unstaged: Some(Change::Modified),
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        };
        let mut state = state_on("src/a.rs", both, WorkingTreeSection::Unstaged);
        assert!(request_discard(&mut state));
        let pending = state.confirm.as_ref().expect("a box opened");
        let said = pending
            .view
            .message
            .iter()
            .map(graft_term::SanitizedText::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            said.contains("Both the staged and the unstaged change"),
            "{said}"
        );
        // The counts come from the pane's own file list, not from prose.
        assert!(said.contains("(+1 \u{2212}0)"), "{said}");
    }

    #[test]
    fn a_dead_operate_job_does_not_take_the_write_keys_with_it() {
        // m3/04 review M4: nothing but `Product::Operated` cleared
        // `operating`, so one panicked or cancelled write left every write
        // key a no-op until the user quit.
        for ended in [true, false] {
            let mut state = state_on(
                "src/a.rs",
                unstaged_entry("src/a.rs"),
                WorkingTreeSection::Unstaged,
            );
            let id = JobId::new(3);
            state.operating = Some(id);
            let commands = write_job_ended(
                &mut state,
                if ended { id } else { JobId::new(9) },
                &mut ids(),
            );
            if ended {
                assert_eq!(state.operating, None);
                assert!(
                    commands
                        .iter()
                        .any(|command| matches!(command, Command::ReadWorkingTree(_))),
                    "{commands:?}"
                );
                // Every write key's first line is this check, so clearing it
                // is exactly what brings them back.
                assert!(!operation_in_flight(&state));
            } else {
                // A different job ending says nothing about the write.
                assert_eq!(state.operating, Some(id));
                assert!(commands.is_empty());
            }
        }
    }

    #[test]
    fn operated_drops_a_stale_answer() {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        state.operating = Some(JobId::new(1));
        let commands = operated(
            &mut state,
            JobId::new(2),
            Operated::Failed {
                error: "boom".into(),
                refresh: Some(Refresh::WorkingTree),
                remote: None,
                refused: None,
                edits: None,
            },
            &mut ids(),
        );
        assert!(commands.is_empty());
        assert_eq!(state.operating, Some(JobId::new(1)));
    }

    /// The two refusals a confirmed Ghost Preview can come back with are
    /// worded as a problem **and a key**, not as a bare sentence
    /// (m5-history-and-ghost-preview/08, piece 8; design note hard choices
    /// 3 and 4).
    ///
    /// `Moved` names the key that drew the picture, which is the key that
    /// redraws it — that is the whole point of carrying `opened_by` through
    /// the launch. `DirtyTree` names `S` and how many files it would take.
    #[test]
    fn a_refused_history_operation_says_which_key_answers_it() {
        let rebase = Operation::Rebase(graft_ops::operation::RebaseParams {
            branch: graft_ops::operation::Tip {
                name: "refs/heads/feature".to_owned(),
                oid: graft_core::oid::Oid::from_bytes(&[1; 20]).expect("20 bytes"),
            },
            onto: graft_ops::operation::Source::Branch("main".to_owned()),
            onto_tip: graft_core::oid::Oid::from_bytes(&[2; 20]).expect("20 bytes"),
        });
        let cases = [
            (
                Refusal::Moved {
                    name: "refs/heads/main".to_owned(),
                },
                Status::Sticky {
                    reason: "main moved since the preview".to_owned(),
                    keys: Some("r again".to_owned()),
                },
            ),
            (
                Refusal::DirtyTree { count: 3 },
                Status::Sticky {
                    reason: "rebase needs a clean tree".to_owned(),
                    keys: Some("S stash 3 changed files".to_owned()),
                },
            ),
        ];
        for (refusal, want) in cases {
            let mut state = AppState::default();
            let mut commands = Vec::new();
            launch_unstoppable(
                &mut state,
                &mut commands,
                rebase.clone(),
                crate::update::preview::HISTORY_CANNOT_STOP,
                Some("r"),
                &mut || JobId::new(9),
            )
            .expect("no write is running");
            let commands = operated(
                &mut state,
                JobId::new(9),
                Operated::Failed {
                    error: "the sentence nobody should see here".into(),
                    refresh: None,
                    remote: None,
                    refused: Some(refusal.clone()),
                    edits: None,
                },
                &mut ids(),
            );
            assert!(commands.is_empty(), "{refusal:?}: {commands:?}");
            assert_eq!(state.status, want, "{refusal:?}");
        }
    }

    /// Every other refusal keeps `graft_ops`' own sentence: the app invents
    /// no key hint it cannot honour.
    #[test]
    fn a_refusal_with_no_key_to_offer_keeps_the_plain_sentence() {
        let mut state = AppState::default();
        let id = JobId::new(4);
        state.operating = Some(id);
        let commands = operated(
            &mut state,
            id,
            Operated::Failed {
                error: "nothing staged".into(),
                refresh: None,
                remote: None,
                refused: Some(Refusal::NothingStaged),
                edits: None,
            },
            &mut ids(),
        );
        assert!(commands.is_empty());
        assert_eq!(state.status, Status::sticky("nothing staged"));
    }

    #[test]
    fn operated_failed_part_way_reports_the_error_and_still_refreshes() {
        // m3/04 review M1. `ApplyError::Git` is "git ran part-way, journaled
        // `Aborted`": the file list and the diff bytes on screen no longer
        // describe the repository, so the next `Space` would cut a patch from
        // stale bytes. The failure is shown *and* the pane re-reads.
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let id = JobId::new(5);
        state.operating = Some(id);
        let commands = operated(
            &mut state,
            id,
            Operated::Failed {
                error: "boom".into(),
                refresh: Some(Refresh::WorkingTree),
                remote: None,
                refused: None,
                edits: None,
            },
            &mut ids(),
        );
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::ReadWorkingTree(_))),
            "{commands:?}"
        );
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::ReadHistory(..))),
            "a working-tree failure must not restart the walk: {commands:?}"
        );
        assert_eq!(state.operating, None);
        assert_eq!(state.status, Status::sticky("boom"));
    }

    #[test]
    fn operated_failed_before_anything_ran_leaves_the_screen_alone() {
        // A refusal writes nothing, so there is nothing to re-read; the job
        // says so with `refresh: None` rather than the pane guessing.
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let id = JobId::new(5);
        state.operating = Some(id);
        let commands = operated(
            &mut state,
            id,
            Operated::Failed {
                error: "nothing to do".into(),
                refresh: None,
                remote: None,
                refused: None,
                edits: None,
            },
            &mut ids(),
        );
        assert!(commands.is_empty(), "{commands:?}");
        assert_eq!(state.operating, None);
        assert_eq!(state.status, Status::sticky("nothing to do"));
    }

    /// The state a conflicted merge leaves: one journaled `Pending` entry,
    /// git's own state live, two files unmerged.
    fn conflicted(paths: &[&str], stalled: Option<&str>) -> Operated {
        Operated::Conflicted {
            description: "merge feature into main".to_owned(),
            kind: InProgress::Merge,
            paths: paths.iter().map(PathBuf::from).collect(),
            stalled: stalled.map(ToOwned::to_owned),
            entry: graft_core::journal::EntryId::new(42, [4; 10]),
        }
    }

    /// **Gap row 505.** The conflict's answer names its journal entry, and
    /// that id reaches the state the banner is drawn from — `live_entries`
    /// — unchanged and at once, not only when the journal re-read lands.
    #[test]
    fn conflicted_entry_id_survives() {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let id = JobId::new(7);
        state.operating = Some(id);
        operated(&mut state, id, conflicted(&["src/a.rs"], None), &mut ids());
        assert_eq!(
            state.live_entries,
            vec![graft_core::journal::EntryId::new(42, [4; 10])]
        );
    }

    #[test]
    fn a_conflicted_merge_is_shown_as_a_state_and_re_reads_everything() {
        // m5/07 seam. `ApplyError::Conflicted` is not a failure: git stopped,
        // the entry is `Pending`, and the user is now *in* the merge. The
        // line has to say so, survive the refresh it triggers, and the
        // refresh has to be the wide one — it is what puts the unmerged
        // paths under CONFLICTS and fills `AppState::in_progress`.
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let id = JobId::new(7);
        state.operating = Some(id);
        let commands = operated(
            &mut state,
            id,
            conflicted(&["src/a.rs", "src/b.rs"], None),
            &mut ids(),
        );
        assert_eq!(state.operating, None);
        // m6-conflicts/12: the stop is state, not a status, so it outlives
        // its own refresh and anything else that takes the status slot.
        assert_eq!(state.status, Status::Ready);
        let stop = state.stopped.as_ref().expect("a live conflict is a stop");
        assert_eq!(stop.kind, InProgress::Merge);
        assert!(!stop.again);
        let reason = crate::update::banner::stop_reason(stop, None);
        assert_eq!(reason, "merge stopped \u{b7} 2 files conflicted");
        assert!(
            !reason.contains("failed") && !reason.contains("error"),
            "a conflict is a state, not an error: {reason}"
        );
        // The wide refresh, which is `Ctrl+R`'s own path: the working tree
        // (unmerged flags and `in_progress`) and the graph both.
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::ReadWorkingTree(_))),
            "{commands:?}"
        );
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::ReadHistory(..))),
            "refs moved: {commands:?}"
        );
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::ReadRefs(_))),
            "{commands:?}"
        );
    }

    #[test]
    fn a_conflict_in_one_file_counts_one_file() {
        let mut state = AppState::default();
        let id = JobId::new(7);
        state.operating = Some(id);
        operated(&mut state, id, conflicted(&["src/a.rs"], None), &mut ids());
        let stop = state.stopped.as_ref().expect("a stop");
        assert_eq!(
            crate::update::banner::stop_reason(stop, None),
            "merge stopped \u{b7} 1 file conflicted"
        );
    }

    #[test]
    fn a_stop_with_nothing_unmerged_does_not_claim_a_conflict() {
        // A rebase can stop with no unmerged path at all. Saying "0 files
        // conflict" would be a lie about why it stopped.
        let mut state = AppState::default();
        let id = JobId::new(7);
        state.operating = Some(id);
        operated(&mut state, id, conflicted(&[], None), &mut ids());
        let stop = state.stopped.as_ref().expect("a stop");
        let reason = crate::update::banner::stop_reason(stop, None);
        assert_eq!(reason, "merge stopped");
        assert!(!reason.contains("conflict"), "{reason}");
    }

    /// m6-conflicts/18 (P5): a stop with something unmerged lands in the
    /// resolver on the answer's first path — git's status order, the same
    /// source `first_conflicted` reads — opened from the banner.
    #[test]
    fn a_conflicted_merge_lands_in_the_resolver() {
        let mut state = AppState::default();
        let id = JobId::new(7);
        state.operating = Some(id);
        let commands = operated(
            &mut state,
            id,
            conflicted(&["src/a.rs", "src/b.rs"], None),
            &mut ids(),
        );
        let open = state
            .resolver
            .as_ref()
            .expect("the stop lands in the resolver");
        assert_eq!(open.path, PathBuf::from("src/a.rs"));
        assert_eq!(
            open.opened_from,
            crate::update::resolver::OpenedFrom::Banner
        );
        // Block 0 is set when the opening read lands (`answers.rs`), as for
        // any open; before it, no block is claimed.
        assert_eq!(open.cursor_block, None);
        assert!(state.banner_focus, "Esc goes back to the banner");
        assert!(state.stopped.is_some());
        assert_eq!(state.status, Status::Ready, "no refusal was worded");
        assert!(
            commands.iter().any(|command| matches!(
                command,
                Command::OpenConflict(_, path) if path == &PathBuf::from("src/a.rs")
            )),
            "{commands:?}"
        );
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::ReadWorkingTree(_))),
            "the wide refresh still runs: {commands:?}"
        );
        // Before the file list lands, `file_cursor` is 0 — the first row,
        // which is where `src/a.rs` will sit once it does.
        assert_eq!(crate::update::resolver::file_cursor(&state), 0);
    }

    /// m6-conflicts/29 (review-18 Major 1): `c` opens the commit editor
    /// while the merge runs; the stop then lands where it lands without a
    /// resolver — no refusal words, the box untouched.
    #[test]
    fn a_box_opened_mid_merge_keeps_the_stop_on_the_banner_without_words() {
        let mut state = AppState::default();
        let id = JobId::new(7);
        state.operating = Some(id);
        let mut editor_ids = ids();
        // The editor's template read is not under test here.
        let _template_read = crate::update::commit_editor::open(&mut state, false, &mut editor_ids);
        assert!(state.commit_editor.is_some(), "c opens the box mid-merge");
        let before = state.commit_editor.clone();
        operated(&mut state, id, conflicted(&["src/a.rs"], None), &mut ids());
        assert!(state.resolver.is_none(), "no resolver over the box");
        assert_eq!(state.status, Status::Ready, "no refusal was worded");
        assert!(state.stopped.is_some());
        assert_eq!(state.commit_editor, before, "the box is untouched");
    }

    #[test]
    fn an_edit_stop_with_nothing_unmerged_keeps_the_banner() {
        let mut state = AppState::default();
        let id = JobId::new(7);
        state.operating = Some(id);
        let commands = operated(&mut state, id, conflicted(&[], None), &mut ids());
        assert!(state.resolver.is_none(), "nothing to resolve");
        assert!(state.stopped.is_some());
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::OpenConflict(..))),
            "{commands:?}"
        );
    }

    #[test]
    fn a_continue_git_refused_repeats_gits_own_words() {
        // `ApplyError::ResumeFailed`: a hook said no, the state is still
        // live, and nothing is unmerged — git's line is the only thing that
        // can say what to do, so it is passed through rather than replaced.
        let mut state = AppState::default();
        let id = JobId::new(7);
        state.operating = Some(id);
        operated(
            &mut state,
            id,
            conflicted(&[], Some("pre-commit hook refused")),
            &mut ids(),
        );
        let Status::Sticky { reason, .. } = &state.status else {
            panic!("{:?}", state.status)
        };
        assert!(reason.contains("pre-commit hook refused"), "{reason}");
        assert!(
            reason.contains("the merge is still in progress"),
            "{reason}"
        );
    }

    #[test]
    fn operated_done_refreshes_the_working_tree_without_restarting_the_walk() {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let id = JobId::new(7);
        state.operating = Some(id);
        let commands = operated(
            &mut state,
            id,
            Operated::Done(applied("stage src/a.rs", Refresh::WorkingTree)),
            &mut ids(),
        );
        assert_eq!(state.status, Status::done("stage src/a.rs"));
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::ReadWorkingTree(_))),
            "{commands:?}"
        );
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::ReadHistory(..))),
            "{commands:?}"
        );
    }

    /// The id rewind that made `u` after a commit answer under another
    /// job's name (m3-working-tree-undo-and-stash/06): a caller borrows
    /// `local_jobs` out into a closure and writes its copy back when the
    /// callee returns, so any id the callee minted from the state's own
    /// counter is thrown away and the next mint reissues an id still in
    /// flight. Both refresh paths must mint from the closure only.
    #[test]
    fn a_refresh_never_mints_from_the_states_own_counter() {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let before = state.local_jobs;

        let mut commands = Vec::new();
        crate::update::keys::refresh_key(&mut state, &mut commands, &mut ids());
        assert!(!commands.is_empty(), "the refresh asked for something");
        assert_eq!(state.local_jobs, before, "Ctrl+R's path");

        let id = JobId::new(9);
        state.operating = Some(id);
        let commands = write_job_ended(&mut state, id, &mut ids());
        assert!(!commands.is_empty(), "the ended write asked for something");
        assert_eq!(state.local_jobs, before, "a write job that ended badly");
    }

    #[test]
    fn operated_done_on_everything_emits_the_same_commands_as_ctrl_r() {
        // m3/04 review M8: `Everything` used to be a hand-written subset that
        // had drifted from `Ctrl+R` — no walk cancellation, no repository
        // re-open, no Mistake Radar reset.
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let id = JobId::new(9);
        state.operating = Some(id);
        let after_write = operated(
            &mut state,
            id,
            Operated::Done(applied("commit", Refresh::Everything)),
            &mut ids(),
        );

        let mut fresh = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let mut after_ctrl_r = Vec::new();
        crate::update::keys::refresh_key(&mut fresh, &mut after_ctrl_r, &mut ids());

        assert_eq!(after_write, after_ctrl_r, "{after_write:?}");
        assert!(
            after_write.contains(&Command::OpenRepository),
            "{after_write:?}"
        );
    }
    /// Major 2 of the m3/05 review: `e` had no test at all, though the
    /// ticket's NOTES said it was tested.
    #[test]
    fn e_on_a_file_asks_for_that_files_porcelain_path() {
        let state = state_with(
            &[("src/a.rs".to_owned(), WorkingTreeSection::Unstaged)],
            vec![unstaged_entry("src/a.rs")],
            1,
        );

        let mut commands = Vec::new();
        let handled = crate::update::detail::detail_key(
            &mut state.clone(),
            &mut commands,
            graft_core::event::KeyPress {
                code: graft_core::event::KeyCode::Char('e'),
                modifiers: graft_core::event::Modifiers::default(),
            },
        );

        assert!(handled, "`e` is the changes pane's key");
        assert_eq!(
            commands,
            vec![Command::OpenExternalEditor {
                path: PathBuf::from("src/a.rs"),
                line: None,
            }],
            "the path is the porcelain's, relative to the worktree root — \
             `Handover` is what joins it onto that root"
        );
    }

    /// The cursor's line travels with it when the diff region has focus, so
    /// the editor opens where the user was reading.
    #[test]
    fn e_in_the_diff_carries_the_cursors_new_side_line() {
        let state = state_on_diff(WorkingTreeSection::Unstaged, one_pair_diff(), 3);

        let target = edit_target(&state).expect("a file is under the cursor");
        assert_eq!(target.0, PathBuf::from("src/a.rs"));
        assert!(
            target.1.is_some(),
            "the added line has a new-side number: {target:?}"
        );
    }

    /// m6-conflicts/46: an untracked file of one line says `1 line`.
    #[test]
    fn untracked_size_is_singular_at_one() {
        assert_eq!(untracked_size(1), " \u{2014} 1 line");
        assert_eq!(untracked_size(7), " \u{2014} 7 lines");
    }

    mod edits_words;
    mod lands_on_head;
    mod one_write;
    mod staging_not_kept;
}
