//! What goes in (C5–C9): the files chosen on open, `space` moving one in or
//! out, the cursor over them, their line counts and the diff of the one
//! under the cursor.

use std::path::Path;
use std::sync::Arc;

use graft_git::diff::{FileEntry, FileList, FileStatus};
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
use graft_ops::operation::{Chosen, FileChoice, Include, OutWhy, first_choice};
use graft_ui::detail::DiffView;

use super::{AppState, Command, CommitEditor, JobId};
use crate::update::detail::{DetailSpec, DiffKey};

/// The diff of the file under the cursor: asked for each time the cursor
/// lands on a file, only the newest answer taken.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileDiffState {
    /// The read in flight.
    pub job: Option<JobId>,
    /// The file it is for.
    pub path: Option<std::path::PathBuf>,
    /// What came back: the diff, or why it could not be read.
    pub view: Option<Result<Arc<DiffView>, String>>,
}

/// Untouched amend → the index as it is ([`Include::Staged`]): a
/// message-only amend stays possible. Otherwise the files going in, each
/// whole or as staged ([`Include::Chosen`]).
///
/// `touched` is "he chose": `space` was pressed, or this is a plain commit,
/// which always takes what the screen shows going in.
#[must_use]
pub fn include_of(choices: &[FileChoice], touched: bool) -> Include {
    if !touched {
        return Include::Staged;
    }
    Include::Chosen(
        choices
            .iter()
            .filter(|choice| choice.goes_in)
            .map(|choice| Chosen {
                path: choice.path.clone(),
                part: choice.part,
            })
            .collect(),
    )
}

/// The first choice (C6): every change but a secrets file, or only what is
/// staged — and for an amend, only what is staged, so nothing staged means
/// only the message changes.
fn first_for(editor: &CommitEditor, status: &WorkingTreeStatus) -> Vec<FileChoice> {
    let mut choices = first_choice(status);
    if editor.amend && status.staged().next().is_none() {
        for choice in &mut choices {
            choice.goes_in = false;
            choice.why_out = Some(OutWhy::NotStaged);
        }
    }
    choices
}

/// Choose fresh on open (C6). Before the first working-tree read there is
/// nothing to choose from; [`tree_read`] chooses once it lands.
pub(super) fn choose(editor: &mut CommitEditor, status: Option<&WorkingTreeStatus>) {
    editor.files = status.map_or_else(Vec::new, |status| first_for(editor, status));
    editor.touched = false;
    editor.file_cursor = display_order(&editor.files).first().copied().unwrap_or(0);
}

/// A working-tree read landed while the screen is open: a file that came
/// or went since is added or dropped. What he chose for a file still there
/// stands; an untouched choice follows the new read whole. The counts and
/// the diff are asked for again, since the files changed under them.
pub fn tree_read(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(status) = state.working_tree.clone() else {
        return;
    };
    let Some(editor) = state.commit_editor.as_mut() else {
        return;
    };
    let cursor_path = editor.files.get(editor.file_cursor).map(|f| f.path.clone());
    let fresh = first_for(editor, &status);
    let files = if editor.touched {
        fresh
            .into_iter()
            .map(|choice| {
                editor
                    .files
                    .iter()
                    .find(|old| old.path == choice.path)
                    .cloned()
                    .unwrap_or(choice)
            })
            .collect()
    } else {
        fresh
    };
    if files == editor.files {
        return;
    }
    editor.files = files;
    editor.file_cursor = cursor_path
        .and_then(|path| editor.files.iter().position(|file| file.path == path))
        .or_else(|| display_order(&editor.files).first().copied())
        .unwrap_or(0);
    let id = next_id();
    editor.counts_job = Some(id);
    commands.push(Command::ReadCommitFiles(id));
    editor.diff = FileDiffState::default();
    ask_diff(state, commands, next_id);
}

/// The files as the list draws them: going in first, then staying out,
/// each in the status' order. Indexes into `files`.
#[must_use]
pub(crate) fn display_order(files: &[FileChoice]) -> Vec<usize> {
    let going_in = (0..files.len()).filter(|at| files[*at].goes_in);
    let staying_out = (0..files.len()).filter(|at| !files[*at].goes_in);
    going_in.chain(staying_out).collect()
}

/// Move the file cursor one row down (`forward`) or up the list as drawn,
/// stopping at either end, and ask for the new file's diff.
pub(super) fn move_cursor(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    forward: bool,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(editor) = state.commit_editor.as_mut() else {
        return;
    };
    let order = display_order(&editor.files);
    let Some(at) = order.iter().position(|index| *index == editor.file_cursor) else {
        return;
    };
    let to = if forward {
        (at + 1).min(order.len().saturating_sub(1))
    } else {
        at.saturating_sub(1)
    };
    if order[to] == editor.file_cursor {
        return;
    }
    editor.file_cursor = order[to];
    ask_diff(state, commands, next_id);
}

/// `space`: the file under the cursor goes in, or stays out. The cursor
/// stays on it, under its new heading. Once pressed, the screen's choice is
/// what an amend takes ([`include_of`]).
pub(super) fn toggle(state: &mut AppState) {
    let Some(editor) = state.commit_editor.as_mut() else {
        return;
    };
    let Some(file) = editor.files.get_mut(editor.file_cursor) else {
        return;
    };
    file.goes_in = !file.goes_in;
    editor.touched = true;
}

/// Ask for the diff of the file under the cursor, unless it is already
/// shown or on its way.
pub(super) fn ask_diff(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let options = state.detail.options;
    let max_lines = crate::update::detail::MAX_LINES;
    let status = state.working_tree.clone();
    let Some(editor) = state.commit_editor.as_mut() else {
        return;
    };
    let Some(file) = editor.files.get(editor.file_cursor) else {
        editor.diff = FileDiffState::default();
        return;
    };
    if editor.diff.path.as_ref() == Some(&file.path) {
        return;
    }
    let entry = entry_for(&file.path, editor.counts.as_deref(), status.as_deref());
    let id = next_id();
    editor.diff = FileDiffState {
        job: Some(id),
        path: Some(file.path.clone()),
        view: None,
    };
    let key = DiffKey {
        spec: DetailSpec::WorkingTree,
        entry: 0,
        // No section: the working tree against `HEAD`, both halves at once
        // (`jobs::detail::diff_spec_for`).
        section: None,
        options,
        max_lines,
    };
    commands.push(Command::ReadFileDiff(id, key, entry));
}

/// The diff read's entry for `path`: the line-count read's own when it has
/// landed (it knows a rename's old path), else one built from the status.
fn entry_for(
    path: &Path,
    counts: Option<&FileList>,
    status: Option<&WorkingTreeStatus>,
) -> FileEntry {
    if let Some(entry) = counts.and_then(|list| counted(list, path)) {
        return entry.clone();
    }
    let found = status.and_then(|status| status.entries.iter().find(|entry| entry.path == path));
    FileEntry {
        path: bytes(path).to_vec(),
        old_path: found
            .and_then(|entry| entry.original_path.as_deref())
            .map(|old| bytes(old).to_vec()),
        status: if found.is_some_and(|entry| entry.untracked) {
            FileStatus::Untracked
        } else {
            FileStatus::Modified
        },
        added: None,
        removed: None,
    }
}

/// A path's bytes, as git's lists carry them.
fn bytes(path: &Path) -> &[u8] {
    path.as_os_str().as_encoded_bytes()
}

/// `path`'s entry in the line-count read.
pub(crate) fn counted<'a>(list: &'a FileList, path: &Path) -> Option<&'a FileEntry> {
    list.files.iter().find(|entry| entry.path == bytes(path))
}

/// The diff read `id` is the screen's own: the answer router asks before
/// the detail pane takes it.
#[must_use]
pub(crate) fn claims_diff(state: &AppState, id: JobId) -> bool {
    state
        .commit_editor
        .as_ref()
        .is_some_and(|editor| editor.diff.job == Some(id))
}

/// The diff of the file under the cursor came back.
pub fn diff_arrived(state: &mut AppState, id: JobId, result: Result<Arc<DiffView>, String>) {
    let Some(editor) = state.commit_editor.as_mut() else {
        return;
    };
    if editor.diff.job != Some(id) {
        return;
    }
    editor.diff.job = None;
    editor.diff.view = Some(result);
}

/// The line counts came back. A failure leaves the rows without counts and
/// the summary without its `+a −d`; the rest of the screen works, so it is
/// said on the status row rather than dropped.
pub fn commit_files_arrived(
    state: &mut AppState,
    id: JobId,
    result: Result<Arc<FileList>, String>,
) {
    let Some(editor) = state.commit_editor.as_mut() else {
        return;
    };
    if editor.counts_job != Some(id) {
        return;
    }
    editor.counts_job = None;
    match result {
        Ok(list) => editor.counts = Some(list),
        Err(why) => {
            state.status = crate::update::Status::Failed(format!("line counts not read: {why}"));
        }
    }
}

/// The one-letter change the list shows (`M`, `A`, `D`, `R`).
#[must_use]
pub(crate) fn kind_of(entry: &StatusEntry) -> char {
    if entry.untracked {
        return 'A';
    }
    match entry.staged.or(entry.unstaged) {
        Some(Change::Added) => 'A',
        Some(Change::Deleted) => 'D',
        Some(Change::Renamed) => 'R',
        Some(Change::Copied) => 'C',
        Some(Change::TypeChanged) => 'T',
        Some(Change::Modified) | None => 'M',
    }
}
