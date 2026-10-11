//! The undo journal's state: what `J` shows, and the read that keeps it
//! current (m3-working-tree-undo-and-stash/06).
//!
//! Pure, like `update::map`, whose open/close/key/event shape this copies.
//! The journal itself is a file, so reading it is a job; what lives here is
//! which row the cursor is on, which one is expanded, and whether the
//! interrupted-entry notice has been looked at.
//!
//! Since m7-power-tools/24 the rows are `graft_ops::journal_list`'s list —
//! Graft's own steps (`●`), the shell's (`$`) and what nothing holds any
//! more (`✗`) — built again each time one of its three reads lands
//! ([`rebuild`]). The cursor is a [`RowKey`], not an index, so it stays on
//! its row while rows arrive above it.

use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::journal::{EntryId, OperationKind};
use graft_core::oid::Oid;
use graft_ops::journal_list::{JournalList, ListRow, LostScan, OutsideRecord, Recover};
use graft_ops::operation::Operation;

use super::detail::Loading;
use super::events::{Command, Product};
use super::state::{AppState, Status};
use crate::jobs::journal_list::LostFound;
use crate::jobs::undo::JournalRead;

pub(crate) mod answers;
pub(crate) mod find;
pub(crate) mod keys;
mod reading;
pub(super) use answers::journal_event;
pub(crate) use answers::{lost_landed, outside_landed, recover_answered};
#[cfg(test)]
pub(crate) use keys::journal_enter;
pub(crate) use keys::undo_from_j;
pub(super) use keys::{close, journal_key};
pub(crate) use reading::{not_undone_kind, switched_from};

/// A row of J's list by what it stands for, so the cursor keeps its row
/// when the list is built again with more rows in it (note J1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RowKey {
    /// A journal entry.
    Graft(EntryId),
    /// A step the shell took: its reflog record's own stable fields (V5).
    /// `old` is in it because two records of one ref in one second can
    /// share `new`.
    Outside {
        /// The ref the record is in.
        refname: String,
        /// Its old value.
        old: Option<Oid>,
        /// Its new value.
        new: Option<Oid>,
        /// When, in seconds.
        at: i64,
    },
    /// A lost run, by its tip.
    Lost(Oid),
    /// A lost edit, by the undo that replaced it.
    LostEdit(EntryId),
}

/// The key of `list.rows[index]`.
pub(crate) fn row_key(list: &JournalList, index: usize) -> Option<RowKey> {
    Some(match list.rows.get(index)? {
        ListRow::Graft(id) => RowKey::Graft(id.clone()),
        ListRow::Outside(step) => RowKey::Outside {
            refname: step.record.refname.clone(),
            old: step.record.old,
            new: step.record.new,
            at: step.record.at,
        },
        ListRow::Lost { commit, .. } => RowKey::Lost(commit.run.tip),
        ListRow::LostEdit { edit, .. } => RowKey::LostEdit(edit.entry.clone()),
    })
}

/// Where the row `key` stands in `list`, if it is there.
pub(crate) fn index_of(list: &JournalList, key: &RowKey) -> Option<usize> {
    (0..list.rows.len()).find(|&index| row_key(list, index).as_ref() == Some(key))
}

/// The commit `b` names on a row: ✗ its tip; $ `new`, else `old`; ● and a
/// lost edit none (the list alone names no commit for a journal entry).
pub(crate) fn row_commit(list: &JournalList, index: usize) -> Option<Oid> {
    match list.rows.get(index)? {
        ListRow::Lost { commit, .. } => Some(commit.run.tip),
        ListRow::Outside(step) => step.record.new.or(step.record.old),
        ListRow::Graft(_) | ListRow::LostEdit { .. } => None,
    }
}

/// The row the cursor starts on once the lost commits are in, when no key
/// has been pressed since `J` (planner's ruling 2026-09-26): the newest `✗`
/// row under a `$` row, when that `$` row is newer than every `●` row but a
/// fetch. `None` leaves the cursor on the newest row.
///
/// A lost edit (always under a `●`) and a commit lost under a `●` are never
/// chosen; `n` reaches them. Pure: no state, no terminal.
pub(crate) fn preselect(list: &JournalList, is_fetch: impl Fn(EntryId) -> bool) -> Option<usize> {
    for (index, row) in list.rows.iter().enumerate() {
        match row {
            // Graft worked since: the shell's loss is not the news.
            ListRow::Graft(id) if !is_fetch(id.clone()) => return None,
            ListRow::Lost { event, .. }
                if matches!(list.rows.get(*event), Some(ListRow::Outside(_))) =>
            {
                return Some(index);
            }
            ListRow::Graft(_)
            | ListRow::Outside(_)
            | ListRow::Lost { .. }
            | ListRow::LostEdit { .. } => {}
        }
    }
    None
}

/// The journal browser's own state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JournalState {
    /// `J` opened it.
    pub shown: bool,
    /// The row under the cursor; `None` is the newest row.
    pub(crate) cursor: Option<RowKey>,
    /// The row whose Command Log is open, if any. One at a time: the log is
    /// several lines and two open at once pushes the cursor off screen.
    pub(crate) expanded: Option<RowKey>,
    /// A key was pressed since `J`: the lost rows' preselect is off.
    pub(crate) touched: bool,
    /// The interrupted-entry notice has been looked at, so the footer stops
    /// saying so. One press of `J` is enough.
    pub notice_seen: bool,
    /// The journal as it was last read.
    pub entries: Loading<Arc<JournalRead>>,
    /// The read in flight, if any. Held apart from [`JournalState::entries`]
    /// so a re-read does not blank the list it is refreshing — the marks and
    /// the interrupted notice are drawn from that list, and they must not
    /// blink off for the duration of every `Ctrl+R` (m3/06 review, Major 1).
    pub reading: Option<JobId>,
    /// Interrupted entries whose after-state could not be read, and why,
    /// **accumulated across reads**. Only the read that recovered an entry
    /// can report it, so a later read that recovers nothing must not erase
    /// the warning.
    pub unreadable: Vec<String>,
    /// Stage 2: git's reflog records, read once per `J`.
    pub(crate) outside: Loading<Arc<Vec<OutsideRecord>>>,
    /// Stage 3: the lost commits and edits, found after stage 2.
    pub(crate) lost: Loading<Arc<LostFound>>,
    /// The list as last built from what has landed ([`rebuild`]).
    pub(crate) list: Option<Arc<JournalList>>,
    /// The name the last `⏎` offered on a row (`OfferBranch`): `b` on that
    /// row prefills it.
    pub(crate) offered: Option<(RowKey, String)>,
    /// A `⏎` asking what a row puts back, and the row it asked for. An
    /// answer under any other id is stale.
    pub(crate) recovering: Option<(JobId, RowKey)>,
    /// The `/` find: its prompt while he types, and the find kept
    /// (m7-power-tools/30o-c, [`find`]).
    pub(crate) find: find::JournalFind,
}

impl JournalState {
    /// Ask for the journal, under a fresh id. Idempotent in the sense that
    /// matters: a second read simply replaces the first, and the older
    /// answer is dropped by id.
    pub(super) fn read(&mut self, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        let id = next_id();
        self.reading = Some(id);
        // A list already on hand stays on screen while the fresh one is
        // fetched: the browser's marks and the interrupted notice come from
        // it, and a journal read happens after every write.
        if !matches!(self.entries, Loading::Ready(_)) {
            self.entries = Loading::Pending(id);
        }
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

    /// The cursor's index in the list; 0 (the newest) when it names none.
    #[must_use]
    pub(crate) fn selected(&self) -> usize {
        match (&self.list, &self.cursor) {
            (Some(list), Some(key)) => index_of(list, key).unwrap_or(0),
            _ => 0,
        }
    }

    /// The cursor as an index into [`JournalState::newest_first`]: the
    /// cursor's `●` row, or the nearest `●` above it. What the browser's
    /// drawing of the `●` rows alone selects (m3/06; 24-b draws the rest).
    #[must_use]
    pub fn selected_graft_row(&self) -> usize {
        let Some(list) = &self.list else {
            return 0;
        };
        let upto = self.selected().min(list.rows.len().saturating_sub(1));
        list.rows
            .iter()
            .take(upto + 1)
            .filter(|row| matches!(row, ListRow::Graft(_)))
            .count()
            .saturating_sub(1)
    }

    /// Whether entry `id`'s Command Log is open.
    #[must_use]
    pub fn is_expanded(&self, id: &EntryId) -> bool {
        matches!(&self.expanded, Some(RowKey::Graft(open)) if open == id)
    }

    /// Stage 2 or stage 3 is still out: the title says `finding lost
    /// commits…` while this holds (J1).
    #[must_use]
    pub fn finding_lost(&self) -> bool {
        matches!(self.outside, Loading::Pending(_)) || matches!(self.lost, Loading::Pending(_))
    }

    fn rows(&self) -> usize {
        self.list.as_ref().map_or(0, |list| list.rows.len())
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

/// The local branch names on hand. No refs read yet is no branch at all,
/// which reads every branch as gone; the lost rows need two reads that land
/// after the launch's refs read, so that case is not reached on the way
/// through `J`.
fn local_branches(state: &AppState) -> std::collections::HashSet<Vec<u8>> {
    state
        .refs
        .as_deref()
        .map(|refs| {
            refs.branches()
                .map(|(_, name, _, _)| name.to_vec())
                .collect()
        })
        .unwrap_or_default()
}

/// Builds J's list again from what has landed: the journal (stage 1), the
/// reflog records (stage 2), the lost commits and edits (stage 3).
///
/// The cursor keeps its [`RowKey`]; a key that is gone falls to the newest
/// row. Once stage 3 is in, and no key was pressed since `J`, the cursor
/// goes where [`preselect`] says.
pub(crate) fn rebuild(state: &mut AppState) {
    let Loading::Ready(read) = &state.journal.entries else {
        state.journal.list = None;
        find::refresh_hits(&mut state.journal);
        return;
    };
    let outside: &[OutsideRecord] = match &state.journal.outside {
        Loading::Ready(records) => records,
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => &[],
    };
    let found = match &state.journal.lost {
        Loading::Ready(found) => Some(Arc::clone(found)),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
    };
    let scan = match (&state.journal.lost, &found) {
        (_, Some(found)) => found.scan.clone(),
        (Loading::Failed(why), None) => LostScan::Failed(why.clone()),
        // Not asked yet, or still out: the list has no lost rows so far.
        _ => LostScan::Running,
    };
    // Whether a lost commit's branch is gone is asked of the refs on hand.
    let branches = local_branches(state);
    let subject_of = |tip: Oid| found.as_ref().and_then(|found| found.subject_of(tip));
    let branch_exists = |name: &str| branches.contains(name.as_bytes());
    let list = graft_ops::journal_list::build(
        &read.entries,
        outside,
        found.as_ref().map(|found| &found.unreached),
        &subject_of,
        &branch_exists,
        found
            .as_ref()
            .map_or(&[][..], |found| found.edits.as_slice()),
        scan,
    );
    let preselected = if found.is_some() && !state.journal.touched {
        let entries = &read.entries;
        preselect(&list, |id| {
            entries
                .iter()
                .any(|entry| entry.id == id && entry.kind == OperationKind::Fetch)
        })
    } else {
        None
    };
    let journal = &mut state.journal;
    if let Some(index) = preselected {
        journal.cursor = row_key(&list, index);
    } else if let Some(key) = &journal.cursor
        && index_of(&list, key).is_none()
    {
        journal.cursor = row_key(&list, 0);
    }
    if let Some(key) = &journal.expanded
        && index_of(&list, key).is_none()
    {
        journal.expanded = None;
    }
    journal.list = Some(Arc::new(list));
    // The kept find's hits are list indices: the new list moves them.
    find::refresh_hits(journal);
}

#[cfg(test)]
mod tests;
