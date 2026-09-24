//! The resolver's file list (m6-conflicts/13, ADR-0043 §1): which files
//! this step conflicted on, in which order, each in which state — derived
//! here, once, and read by the view, the mouse and the landing after a file
//! is done.
//!
//! `graft_ops::conflict::ConflictSet` is order only; every row's state comes
//! from the working-tree status and the resolver's own document, so it
//! cannot disagree with what the rest of the app shows.

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::job::JobId;
use graft_git::status::UnmergedHow;

use super::{ResolverState, reopen};
use crate::update::detail::Loading;
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

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
/// Order: the conflict set's paths once it has arrived, then every unmerged
/// path it does not name, in git's order; before it arrives (or when it
/// could not be read) the unmerged paths alone. The open file is listed
/// even when neither names it yet — the status may not have arrived — so
/// the cursor always has a row.
///
/// State, first match wins: status not arrived → `Unread` (but for the open
/// file); not unmerged → `Done`; the open file → what its document or card
/// says; otherwise what it last was while open here ([`ResolverState::seen`]),
/// else `Unread`.
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
            } else {
                open.seen.get(&path).cloned().unwrap_or(RowState::Unread)
            };
            ListedFile { path, state }
        })
        .collect()
}

/// The open file's own state, from its document or card; `None` while its
/// read is still in flight.
pub(super) fn open_state(open: &ResolverState) -> Option<RowState> {
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
    if target.state == RowState::Done {
        // git no longer calls the path unmerged, so the opening read would
        // be refused (`graft_ops::conflict::open_conflict`'s `NotUnmerged`)
        // and close the resolver. Refused here, in words, instead — see the
        // ticket's NOTES: the spec's read-only open needs graft-ops.
        state.status = Status::Sticky {
            reason: format!("{} is already done", target.path.display()),
            keys: None,
        };
        return true;
    }
    let from = open.opened_from;
    // The file left keeps its row's state as it stands now.
    if let Some(open) = state.resolver.as_mut() {
        remember(open);
    }
    reopen(state, commands, target.path, from, next_id);
    true
}

/// Why the open file cannot be left right now: a read, a take, a base load
/// or a write on it is in flight, and its answer would land on the next.
fn switch_refusal(state: &AppState) -> Option<&'static str> {
    let open = state.resolver.as_ref()?;
    if let Some(words) = open.busy_words() {
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
    }
    commands.push(Command::ReadConflictSet(id));
}

/// The conflict set answered.
pub fn conflict_set_read(
    state: &mut AppState,
    id: JobId,
    result: Result<Arc<graft_ops::conflict::ConflictSet>, String>,
) {
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
    open.files = match result {
        Ok(set) => Loading::Ready(set),
        Err(why) => Loading::Failed(why),
    };
}
