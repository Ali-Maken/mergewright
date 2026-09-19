//! The two JSONL records an entry is written as (DESIGN.md §6.2).
//!
//! A `Pending` line carries everything known before the operation runs; the
//! outcome line, appended afterwards under the same `id`, carries the
//! after-state. The log is append-only: an outcome is a second line, never
//! an edit, so a crash between the two leaves a `Pending` line alone, which
//! `parse` keeps as a `Pending` entry for the app to surface.

use std::collections::BTreeMap;

use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches};
use graft_core::oid::Oid;
use serde::{Deserialize, Serialize};

/// One record of the journal file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// The entry's opening: its before-state.
    Pending(PendingLine),
    /// The entry's outcome: `Committed` or `Aborted`, with its after-state.
    Outcome(OutcomeLine),
}

/// The before-state, written before the operation starts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingLine {
    /// The entry's id.
    pub id: EntryId,
    /// Always `Pending`; kept in the record so every line reads the same.
    pub status: EntryStatus,
    /// When the entry was opened.
    pub at_unix_ms: i64,
    /// What kind of operation.
    pub kind: OperationKind,
    /// The browser's text.
    pub description: String,
    /// What the operation was going to change.
    pub touches: Touches,
    /// `HEAD` before.
    pub head_before: HeadRef,
    /// Touched refs before.
    pub refs_before: BTreeMap<String, Option<Oid>>,
    /// Index tree before.
    pub index_before: Option<Oid>,
    /// Working-tree tree before.
    pub worktree_before: Option<Oid>,
    /// The backup ref, taken before the operation.
    pub backup_ref: Option<String>,
    /// Commits the inverse needs alive, known before the operation runs.
    pub pins: Vec<Oid>,
    /// The entry this one reverses, for `Undo`/`Redo`.
    pub reverses: Option<EntryId>,
}

/// The after-state, written once the operation finished or failed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeLine {
    /// The entry's id.
    pub id: EntryId,
    /// `Committed` or `Aborted { reason }`.
    pub status: EntryStatus,
    /// `HEAD` after.
    pub head_after: Option<HeadRef>,
    /// Touched refs after.
    pub refs_after: BTreeMap<String, Option<Oid>>,
    /// Index tree after.
    pub index_after: Option<Oid>,
    /// Working-tree tree after.
    pub worktree_after: Option<Oid>,
    /// The Command Log.
    pub commands: Vec<String>,
}

/// Just enough of any line to know which record it is.
#[derive(Deserialize)]
struct Head {
    status: EntryStatus,
}

impl Line {
    /// Parses one record; the status field decides which shape is read.
    pub(super) fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        let head: Head = serde_json::from_str(text)?;
        Ok(match head.status {
            EntryStatus::Pending => Self::Pending(serde_json::from_str(text)?),
            EntryStatus::Committed | EntryStatus::Aborted { .. } => {
                Self::Outcome(serde_json::from_str(text)?)
            }
        })
    }

    /// The record's id.
    #[must_use]
    pub fn id(&self) -> &EntryId {
        match self {
            Self::Pending(line) => &line.id,
            Self::Outcome(line) => &line.id,
        }
    }
}

impl PendingLine {
    pub(super) fn of(entry: &JournalEntry) -> Self {
        Self {
            id: entry.id.clone(),
            status: EntryStatus::Pending,
            at_unix_ms: entry.at_unix_ms,
            kind: entry.kind,
            description: entry.description.clone(),
            touches: entry.touches.clone(),
            head_before: entry.head_before.clone(),
            refs_before: entry.refs_before.clone(),
            index_before: entry.index_before,
            worktree_before: entry.worktree_before,
            backup_ref: entry.backup_ref.clone(),
            pins: entry.pins.clone(),
            reverses: entry.reverses.clone(),
        }
    }

    /// The entry as it stands with no outcome yet.
    pub(super) fn into_entry(self) -> JournalEntry {
        JournalEntry {
            id: self.id,
            at_unix_ms: self.at_unix_ms,
            kind: self.kind,
            description: self.description,
            status: EntryStatus::Pending,
            touches: self.touches,
            head_before: self.head_before,
            head_after: None,
            refs_before: self.refs_before,
            refs_after: BTreeMap::new(),
            index_before: self.index_before,
            index_after: None,
            worktree_before: self.worktree_before,
            worktree_after: None,
            backup_ref: self.backup_ref,
            pins: self.pins,
            commands: Vec::new(),
            reverses: self.reverses,
        }
    }
}

impl OutcomeLine {
    pub(super) fn of(entry: &JournalEntry) -> Self {
        Self {
            id: entry.id.clone(),
            status: entry.status.clone(),
            head_after: entry.head_after.clone(),
            refs_after: entry.refs_after.clone(),
            index_after: entry.index_after,
            worktree_after: entry.worktree_after,
            commands: entry.commands.clone(),
        }
    }

    /// Fills the after-fields of a `Pending` entry.
    pub(super) fn apply_to(self, entry: &mut JournalEntry) {
        entry.status = self.status;
        entry.head_after = self.head_after;
        entry.refs_after = self.refs_after;
        entry.index_after = self.index_after;
        entry.worktree_after = self.worktree_after;
        entry.commands = self.commands;
    }
}
