//! The undo journal's state: what `J` shows, and the read that keeps it
//! current (m3-working-tree-undo-and-stash/06).
//!
//! Pure, like `update::map`, whose open/close/key/event shape this copies.
//! The journal itself is a file, so reading it is a job; what lives here is
//! which entry the cursor is on, which one is expanded, and whether the
//! interrupted-entry notice has been looked at.

use graft_core::job::JobId;

use super::detail::Loading;
use super::events::{Command, Product};
use super::state::AppState;
use crate::jobs::undo::JournalRead;

/// The journal browser's own state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JournalState {
    /// `J` opened it.
    pub shown: bool,
    /// The row under the cursor, newest first.
    pub selected: usize,
    /// The row whose Command Log is open, if any. One at a time: the log is
    /// several lines and two open at once pushes the cursor off screen.
    pub expanded: Option<usize>,
    /// The interrupted-entry notice has been looked at, so the footer stops
    /// saying so. One press of `J` is enough.
    pub notice_seen: bool,
    /// The journal as it was last read.
    pub entries: Loading<std::sync::Arc<JournalRead>>,
}

impl JournalState {
    /// Ask for the journal, under a fresh id. Idempotent in the sense that
    /// matters: a second read simply replaces the first, and the older
    /// answer is dropped by id.
    pub(super) fn read(&mut self, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        let id = next_id();
        self.entries = Loading::Pending(id);
        vec![Command::ReadJournal(id)]
    }

    /// The entries the browser draws, newest first. Empty while loading or
    /// after a failure — [`JournalState::entries`] says which.
    #[must_use]
    pub fn newest_first(&self) -> Vec<&graft_core::journal::JournalEntry> {
        match &self.entries {
            Loading::Ready(read) => read.entries.iter().rev().collect(),
            Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => Vec::new(),
        }
    }
}

/// The launch read: asks once, and never again on its own. Called at the
/// tail of every `update` (`super::settle`), which is where the radar's own
/// idempotent requests live.
pub(super) fn request(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    // Nothing to read until the repository is open: the journal is a file
    // inside it, and `journal_path` asks git where. The radar's own request
    // waits on a fact about the repository for the same reason.
    if state.repository.is_some() && state.journal.entries == Loading::Idle {
        commands.extend(state.journal.read(next_id));
    }
}

/// `Product::Journal` landed. Stale answers are dropped by id, the same way
/// every other `Loading` field drops them.
pub(super) fn journal_event(state: &mut AppState, id: JobId, product: &Product) -> bool {
    let Product::Journal(result) = product else {
        return false;
    };
    if state.journal.entries != Loading::Pending(id) {
        return true;
    }
    state.journal.entries = match result {
        Ok(read) => Loading::Ready(std::sync::Arc::clone(read)),
        Err(why) => Loading::Failed(why.clone()),
    };
    // A shorter journal must not leave the cursor past its end.
    let rows = state.journal.newest_first().len();
    state.journal.selected = state.journal.selected.min(rows.saturating_sub(1));
    if state.journal.expanded.is_some_and(|row| row >= rows) {
        state.journal.expanded = None;
    }
    true
}
