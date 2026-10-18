//! The resolver's file list (m6-conflicts/13, ADR-0043 §1): which files
//! this step conflicted on, in which order, each in which state — derived
//! here, once, and read by the view, the mouse and the landing after a file
//! is done.
//!
//! `mergewright_ops::conflict::ConflictSet` is order only; every row's state comes
//! from the working-tree status and the resolver's own document, so it
//! cannot disagree with what the rest of the app shows.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mergewright_core::job::JobId;
use mergewright_git::status::UnmergedHow;

use super::{ResolverState, reopen};
use crate::update::detail::Loading;
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// The conflict set's job's answer (m6-conflicts/13, ease-pass/11b).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetRead {
    /// The files the operation conflicted on, or why not.
    pub set: Result<Arc<mergewright_ops::conflict::ConflictSet>, String>,
    /// A merge's files git combined on its own, or why not; `None` when the
    /// operation is not a merge.
    pub combined: Option<Result<Vec<PathBuf>, String>>,
}

impl From<Result<Arc<mergewright_ops::conflict::ConflictSet>, String>> for SetRead {
    /// A set read alone, with nothing combined: a non-merge's answer.
    fn from(set: Result<Arc<mergewright_ops::conflict::ConflictSet>, String>) -> Self {
        SetRead {
            set,
            combined: None,
        }
    }
}

/// Where one listed file stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowState {
    /// A document with `open` of `of` blocks still open.
    Open {
        /// Blocks still open.
        open: usize,
        /// Blocks in the file.
        of: usize,
    },
    /// Every block taken; `⏎` has not marked it done yet.
    /// In a merge, a merge's solved file (CONTEXT `Solved (file)`).
    AllTaken,
    /// git's status no longer calls it unmerged.
    Done,
    /// A whole-file shape (modify/delete, …), resolved by `o`/`t`.
    WholeFile {
        /// The shape git's status gave it.
        how: UnmergedHow,
    },
    /// A binary file, kept whole from one side.
    Binary,
    /// Not read yet.
    Unread,
}

/// One row of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedFile {
    /// The path, as git names it.
    pub path: PathBuf,
    /// Where it stands.
    pub state: RowState,
}

/// **The one source of the list's order and states.**
///
/// Rows: the conflict set's paths once it has arrived, and every unmerged
/// path it does not name; before it arrives (or when it could not be read)
/// the unmerged paths alone. The open file is listed even when neither
/// names it yet — the status may not have arrived — so the cursor always
/// has a row. **Order: git's path order** (byte-wise), over all of them, so
/// a file becoming done keeps its row (m6-conflicts/25).
///
/// State, first match wins: status not arrived → `Unread` (but for the open
/// file); not unmerged → `Done`; the open file → what its document or card
/// says; otherwise what it last was while open here ([`ResolverState::seen`]),
/// else `AllTaken` when the set read found it solved on disk, else `Unread`.
#[must_use]
pub fn listed_files(state: &AppState) -> Vec<ListedFile> {
    let Some(open) = state.resolver.as_ref() else {
        return Vec::new();
    };
    let unmerged = crate::update::banner::unmerged_paths(state);
    let mut paths: Vec<PathBuf> = match &open.files {
        Loading::Ready(set) => set.paths.clone(),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => Vec::new(),
    };
    for path in unmerged.iter().chain(std::iter::once(&open.path)) {
        if !paths.contains(path) {
            paths.push(path.clone());
        }
    }
    // git's path order, over the whole union, so a row never moves when its
    // file becomes done (m6-conflicts/25 D1): the set lists unmerged paths
    // first, and a file staged would otherwise jump to the end.
    paths.sort_by(|a, b| git_order(a, b));
    let arrived = state.working_tree.is_some();
    paths
        .into_iter()
        .map(|path| {
            let is_open = path == open.path;
            let state = if !arrived && !is_open {
                RowState::Unread
            } else if arrived && !unmerged.contains(&path) {
                RowState::Done
            } else if let Some(now) = is_open.then(|| open_state(open)).flatten() {
                now
            } else if let Some(seen) = open.seen.get(&path) {
                seen.clone()
            } else if solved_on_disk(open, &path) {
                RowState::AllTaken
            } else {
                RowState::Unread
            };
            ListedFile { path, state }
        })
        .collect()
}

/// The set read found `path` solved on disk — every clash picked, not yet
/// staged (`ConflictSet::solved`, the resolver's own rule): so a file solved
/// before a pause or a relaunch reads as solved unopened (ease-pass/05c).
fn solved_on_disk(open: &ResolverState, path: &Path) -> bool {
    match &open.files {
        Loading::Ready(set) => set.solved.iter().any(|solved| solved == path),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => false,
    }
}

/// git's own path order — the index's and `status`'s: byte-wise over the
/// path's bytes. **Not** `Path`'s `Ord`, which compares by component and so
/// puts `a/b` before `a.b`.
fn git_order(a: &Path, b: &Path) -> std::cmp::Ordering {
    a.as_os_str()
        .as_encoded_bytes()
        .cmp(b.as_os_str().as_encoded_bytes())
}

/// The open file's own state, from its document or card; `None` while its
/// read is still in flight.
pub(super) fn open_state(open: &ResolverState) -> Option<RowState> {
    if open.done.is_some() {
        return Some(RowState::Done);
    }
    if let Some(file) = open.file.as_ref() {
        let left = file.open();
        return Some(if left == 0 {
            RowState::AllTaken
        } else {
            RowState::Open {
                open: left,
                of: file.blocks().len(),
            }
        });
    }
    open.whole_file.as_ref().map(|whole| {
        if whole.binary {
            RowState::Binary
        } else {
            RowState::WholeFile { how: whole.how }
        }
    })
}

/// Records the open file's state as it now stands, so the list still says
/// so once another file is open.
pub(super) fn remember(open: &mut ResolverState) {
    if let Some(now) = open_state(open) {
        open.seen.insert(open.path.clone(), now);
    }
}

/// The open file's row in [`listed_files`]; 0 when it has none. A function
/// of the open path, not a field, so it cannot drift from it.
#[must_use]
pub fn file_cursor(state: &AppState) -> usize {
    let Some(open) = state.resolver.as_ref() else {
        return 0;
    };
    listed_files(state)
        .iter()
        .position(|file| file.path == open.path)
        .unwrap_or(0)
}

/// **Open the file at `index`** of [`listed_files`], keeping the list.
///
/// `true` when the click was answered — opening, refusing in words, or
/// already there (nothing sent); `false` only when `index` names no row.
pub fn select_file(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    index: usize,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(target) = listed_files(state).into_iter().nth(index) else {
        return false;
    };
    let Some(open) = state.resolver.as_ref() else {
        return false;
    };
    if target.path == open.path {
        return true;
    }
    if let Some(words) = switch_refusal(state) {
        state.status = Status::sticky(words);
        return true;
    }
    // A `Done` row opens too: the job's route reads it read-only
    // (m6-conflicts/25).
    let from = open.opened_from;
    // The file left keeps its row's state as it stands now.
    if let Some(open) = state.resolver.as_mut() {
        remember(open);
    }
    reopen(state, commands, target.path, from, next_id);
    true
}

/// `j`/`↓` (`forward`) or `k`/`↑`: open the next or previous row of
/// [`listed_files`] (m6-conflicts/17 D4). No wrap (§2.2): at either end
/// nothing is sent and the words stay — the list's `▸` on the end row says
/// so. Refusals are [`select_file`]'s own. `true` always: the key is spent.
pub(super) fn step_file(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    forward: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let at = file_cursor(state);
    let target = if forward {
        Some(at + 1).filter(|next| *next < listed_files(state).len())
    } else {
        at.checked_sub(1)
    };
    if let Some(index) = target {
        select_file(state, commands, index, next_id);
    }
    true
}

/// Why the open file cannot be left right now: a box is waiting for an
/// answer about it, or a read, a take, a base load or a write on it is in
/// flight, and its answer would land on the next.
fn switch_refusal(state: &AppState) -> Option<&'static str> {
    let open = state.resolver.as_ref()?;
    if super::box_open(state) {
        // `open_refusal`'s words: the box's question is about this file.
        return Some("finish the box first");
    }
    if let Some(words) = super::busy_refusal(state) {
        return Some(words);
    }
    if open.base_job.is_some() {
        return Some("still loading the base");
    }
    if open.staging.is_some() || open.restarting.is_some() {
        return Some("a write is already running");
    }
    None
}

/// Ask for the conflict set again, keeping the list on screen until it
/// answers (`JournalState::reading`'s pattern).
pub(super) fn reread_set(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.resolver.is_none() {
        return;
    }
    let id = next_id();
    if let Some(open) = state.resolver.as_mut() {
        open.set_job = Some(id);
        if matches!(open.files, Loading::Idle) {
            open.files = Loading::Pending(id);
        }
        if matches!(open.combined, Loading::Idle) {
            open.combined = Loading::Pending(id);
        }
    }
    commands.push(Command::ReadConflictSet(id));
}

/// The conflict set answered.
pub fn conflict_set_read(state: &mut AppState, id: JobId, read: SetRead) {
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    // Not the newest read: dropping it is safe because the newer one is
    // still coming and reads the same repository later, so it carries
    // everything this answer would.
    if open.set_job != Some(id) {
        return;
    }
    open.set_job = None;
    open.files = match read.set {
        Ok(set) => Loading::Ready(set),
        Err(why) => Loading::Failed(why),
    };
    open.combined = match read.combined {
        // Not a merge: nothing combined, and the view draws no section.
        None => Loading::Ready(Arc::new(Vec::new())),
        Some(Ok(paths)) => Loading::Ready(Arc::new(paths)),
        Some(Err(why)) => Loading::Failed(why),
    };
}
