//! Working-tree writes from the changes pane: `Space`/`s` stage or unstage
//! the file under the cursor, `Ctrl+A` stages every changed and untracked
//! path, `d` opens the discard confirm box
//! (m3-working-tree-undo-and-stash/04, design note "Changes pane").
//!
//! One write in flight at a time (`AppState::operating`); every other write
//! key is a no-op while it is `Some`.
//!
//! Seam rule (ARCHITECTURE.md, `crates/graft/tests/it/seam_rule.rs`): this
//! module only builds an [`Operation`] and a [`Command::Operate`] from it —
//! `graft_ops` decides what the write does, never `app`.

use std::path::{Path, PathBuf};

use graft_core::job::JobId;
use graft_git::diff::{PatchDirection, PatchError, Selection};
use graft_git::status::StatusEntry;
use graft_ops::apply::Refresh;
use graft_ops::operation::{Operation, PatchSide, Target};
use graft_ui::detail::{BodyRow, DetailFocus, LineKindView, WorkingTreeSection};

use super::confirm::PendingConfirm;
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
pub(super) fn write_job_ended(state: &mut AppState, id: JobId) -> Vec<Command> {
    if state.operating != Some(id) {
        return Vec::new();
    }
    state.operating = None;
    let mut commands = vec![Command::ReadWorkingTree];
    let AppState {
        detail, local_jobs, ..
    } = &mut *state;
    commands.extend(detail.refresh(&mut || local_jobs.next_id()));
    commands
}

/// Queue one write, and say so.
///
/// One at a time (`AppState::operating`), and the status line carries
/// `<description>` from the moment the key is pressed rather than from
/// whenever the job thread gets going. Without it `Space` gives no
/// acknowledgement at all and a second `Space` looks like a dead key rather
/// than a write already running (m3/04 review M3).
pub(super) fn launch(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    op: Operation,
    next_id: &mut impl FnMut() -> JobId,
) {
    let id = next_id();
    next.operating = Some(id);
    next.status = Status::Busy(op.describe());
    commands.push(Command::Operate(id, Box::new(op)));
}

/// A write is already running: every other write key is a no-op.
#[must_use]
pub fn operation_in_flight(state: &AppState) -> bool {
    state.operating.is_some()
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
        WorkingTreeSection::Unstaged | WorkingTreeSection::Untracked => Operation::Stage(target),
    };
    launch(next, commands, op, next_id);
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
    if state.detail.focus != DetailFocus::Diff {
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
const fn patch_direction(section: WorkingTreeSection) -> (PatchDirection, PatchSide) {
    match section {
        WorkingTreeSection::Staged => (PatchDirection::Reverse, PatchSide::Index),
        WorkingTreeSection::Unstaged | WorkingTreeSection::Untracked => {
            (PatchDirection::Forward, PatchSide::Worktree)
        }
    }
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
        next.status = Status::Sticky(reason);
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
    let (direction, side) = patch_direction(section);
    let patch_bytes = match patch_for(next, &unit, direction, verb) {
        Ok(bytes) => bytes,
        Err(reason) => {
            next.status = Status::Sticky(reason);
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
    launch(next, commands, op, next_id);
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
        next.status = Status::Sticky(reason);
        return true;
    }
    let Some(path) = selected_path(next) else {
        return false;
    };
    let Some(unit) = diff_unit(next) else {
        return false;
    };
    // A discard always reverses what is on disk, whichever side it came from.
    let (_, side) = patch_direction(section);
    let patch_bytes = match patch_for(next, &unit, PatchDirection::Reverse, "discard") {
        Ok(bytes) => bytes,
        Err(reason) => {
            next.status = Status::Sticky(reason);
            return true;
        }
    };
    let counts = patch_counts(&patch_bytes);
    let unit_word = if unit.one_line { "line" } else { "hunk" };
    let shown = path.display();
    let view = graft_ui::confirm::ConfirmView::new(
        format!("Discard this {unit_word} of {shown}?"),
        vec![
            format!(
                "{} added / {} removed lines go back to how they were.",
                counts.0, counts.1
            ),
            "Graft keeps a backup first, so `u` can undo it afterwards.".to_string(),
        ],
        discard_patch_command(side),
    );
    next.confirm = Some(PendingConfirm {
        view,
        then: Operation::Discard(Target::Patch {
            path,
            patch: patch_bytes,
            side,
        }),
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
    launch(
        next,
        commands,
        Operation::Stage(Target::Paths(paths)),
        next_id,
    );
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
    let Some(entry) = status_entry(next, &path) else {
        return false;
    };
    let command = discard_command(&path, entry);
    let counts = selected_counts(next);
    let entry = entry.clone();
    let shown = path.display();
    let question = if entry.untracked {
        format!("Delete {shown}?")
    } else {
        format!("Discard changes to {shown}?")
    };
    let mut consequence = discard_consequence(&entry, counts);
    consequence.push("Graft keeps a backup first, so `u` can undo it afterwards.".to_string());
    let view = graft_ui::confirm::ConfirmView::new(question, consequence, command);
    next.confirm = Some(PendingConfirm {
        view,
        then: Operation::Discard(Target::Paths(vec![path])),
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
            Some((added, _)) if added > 0 => format!(" \u{2014} {added} lines"),
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
    match outcome {
        Operated::Done(applied) => {
            state.status = Status::Done(applied.entry.description.clone());
            // The write the editor was open for landed, so the editor and
            // the draft behind it are about a commit that now exists
            // (m3-working-tree-undo-and-stash/05). A failure leaves both
            // alone: the message is what the user has to fix.
            super::commit_editor::committed(state);
            refresh(state, applied.refresh, next_id)
        }
        Operated::Incomplete { applied, error } => {
            state.status = Status::Sticky(format!(
                "{} \u{2014} could not confirm it fully: {error}",
                applied.entry.description
            ));
            // It ran: the same reasoning as `Done`, and leaving the editor
            // open would invite a second commit of a message already
            // committed.
            super::commit_editor::committed(state);
            refresh(state, applied.refresh, next_id)
        }
        Operated::Failed {
            error,
            refresh: what,
        } => {
            state.status = Status::Sticky(error);
            // A failure is not proof the screen is still true: `ApplyError::Git`
            // means git ran part-way and journaled the entry `Aborted`, so the
            // file list, the diff bytes and the header counts describe a
            // repository that has already moved. The job decides how wide the
            // re-read is; `None` is only the cases where nothing was written
            // (m3/04 review M1).
            match what {
                Some(what) => refresh(state, what, next_id),
                None => Vec::new(),
            }
        }
    }
}

/// Re-read what the write changed.
///
/// [`Refresh::Everything`] means refs moved, which is exactly the state
/// `Ctrl+R` rebuilds — so it runs `Ctrl+R`'s own path rather than a
/// hand-written subset of it. The subset was missing the walk cancellation
/// (a second history walk started while the first still ran), the repository
/// re-open and the Mistake Radar's default-branch reset, which goes stale the
/// moment a commit moves the tip (m3/04 review M8).
fn refresh(
    state: &mut AppState,
    what: Refresh,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match what {
        Refresh::WorkingTree => {
            let mut commands = vec![Command::ReadWorkingTree];
            commands.extend(state.detail.refresh(next_id));
            commands
        }
        Refresh::Everything => {
            let mut commands = Vec::new();
            super::keys::refresh_key(state, &mut commands);
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
            },
            refresh,
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
            submodule: false,
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
            submodule: false,
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
            submodule: false,
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
        let Status::Sticky(text) = &state.status else {
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
        assert_eq!(pending.view.cursor, graft_ui::confirm::Choice::Cancel);
        assert!(
            pending.view.title.as_str().contains("hunk of src/a.rs"),
            "the question names what goes: {:?}",
            pending.view.title
        );
        assert_eq!(pending.view.command, "git apply -R -");
        assert!(
            matches!(
                pending.then,
                Operation::Discard(Target::Patch {
                    side: PatchSide::Worktree,
                    ..
                })
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
            submodule: false,
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
        assert_eq!(pending.view.command, "git clean -f -- new.txt");
        assert_eq!(
            pending.then,
            Operation::Discard(Target::Paths(vec!["new.txt".into()]))
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
            pending.view.command,
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
        assert_eq!(pending.view.cursor, graft_ui::confirm::Choice::Cancel);
    }

    fn deleted_and_staged(path: &str) -> StatusEntry {
        StatusEntry {
            path: path.into(),
            original_path: None,
            staged: Some(Change::Deleted),
            unstaged: None,
            untracked: false,
            unmerged: false,
            submodule: false,
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
            submodule: false,
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
            let Status::Sticky(said) = &state.status else {
                panic!("expected a refusal, got {:?}", state.status);
            };
            assert!(said.contains("press v"), "{said}");

            let mut state = state_on_diff(section, one_pair_diff(), 1);
            state.detail.mode = graft_ui::detail::DiffMode::SideBySide;
            assert!(request_discard_at_diff_cursor(&mut state));
            assert!(state.confirm.is_none());
            let Status::Sticky(said) = &state.status else {
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
        assert_eq!(pending.view.title, "Delete new.txt?");
        let said = pending.view.message.join(" ");
        assert!(said.contains("Deletes the file"), "{said}");
        assert!(said.contains("backup"), "{said}");
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
            submodule: false,
        };
        let mut state = state_on("src/a.rs", both, WorkingTreeSection::Unstaged);
        assert!(request_discard(&mut state));
        let pending = state.confirm.as_ref().expect("a box opened");
        let said = pending.view.message.join(" ");
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
            let commands = write_job_ended(&mut state, if ended { id } else { JobId::new(9) });
            if ended {
                assert_eq!(state.operating, None);
                assert!(commands.contains(&Command::ReadWorkingTree), "{commands:?}");
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
            },
            &mut ids(),
        );
        assert!(commands.is_empty());
        assert_eq!(state.operating, Some(JobId::new(1)));
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
            },
            &mut ids(),
        );
        assert!(commands.contains(&Command::ReadWorkingTree), "{commands:?}");
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::ReadHistory(_))),
            "a working-tree failure must not restart the walk: {commands:?}"
        );
        assert_eq!(state.operating, None);
        assert_eq!(state.status, Status::Sticky("boom".into()));
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
            },
            &mut ids(),
        );
        assert!(commands.is_empty(), "{commands:?}");
        assert_eq!(state.operating, None);
        assert_eq!(state.status, Status::Sticky("nothing to do".into()));
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
        assert_eq!(state.status, Status::Done("stage src/a.rs".into()));
        assert!(commands.contains(&Command::ReadWorkingTree), "{commands:?}");
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::ReadHistory(_))),
            "{commands:?}"
        );
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
        crate::update::keys::refresh_key(&mut fresh, &mut after_ctrl_r);

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
}
