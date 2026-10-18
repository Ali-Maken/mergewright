//! The two JSONL records an entry is written as (DESIGN.md §6.2).
//!
//! A `Pending` line carries everything known before the operation runs; the
//! outcome line, appended afterwards under the same `id`, carries the
//! after-state. The log is append-only: an outcome is a second line, never
//! an edit, so a crash between the two leaves a `Pending` line alone, which
//! `parse` keeps as a `Pending` entry for the app to surface.

use std::collections::BTreeMap;

use mergewright_core::journal::{
    EntryId, EntryStatus, HeadRef, IndexState, JournalEntry, MergeState, OperationKind, PutBack,
    RefMove, Touches,
};
use mergewright_core::oid::Oid;
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
    /// The index before: a bare oid for a merged one, `{"unmerged":…}`
    /// for the stage tree of a half-finished merge.
    pub index_before: Option<IndexState>,
    /// Working-tree tree before.
    pub worktree_before: Option<Oid>,
    /// The raw bytes git's checkout of `worktree_before` would not give back
    /// (ADR-0052). Left out when nothing was kept, so such a line reads
    /// exactly as one written before the field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_raw_before: Option<Oid>,
    /// The backup ref, taken before the operation.
    pub backup_ref: Option<String>,
    /// Commits the inverse needs alive, known before the operation runs.
    pub pins: Vec<Oid>,
    /// The entry this one reverses, for `Undo`/`Redo`.
    pub reverses: Option<EntryId>,
    /// git's merge in progress before, when there was one. Left out of the
    /// line when there was none, so a line with nothing to say about a merge
    /// reads exactly as one written before the field existed
    /// (m6-conflicts/20).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_before: Option<MergeState>,
    /// `branch.<name>.*` of every touched branch before (review
    /// m7-16b..21b, Major 1). Left out when empty, the same way.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub config_before: BTreeMap<String, Vec<String>>,
    /// The stash a pop, apply or set-aside puts back (ADR-0045 §1). Left out
    /// when there is none, the same way (m6-conflicts/72).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub put_back: Option<PutBack>,
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
    /// The index after, the same two shapes.
    pub index_after: Option<IndexState>,
    /// Working-tree tree after.
    pub worktree_after: Option<Oid>,
    /// The Command Log.
    pub commands: Vec<String>,
    /// git's merge in progress after, the same way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_after: Option<MergeState>,
    /// The same config keys after, the same way.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub config_after: BTreeMap<String, Vec<String>>,
    /// The refs a fetch moved (ADR-0062), read around git's run, so known
    /// only now. Left out when not recorded — every other kind, every line
    /// written before the field existed — so such a line reads as before.
    /// `{}` is written: read, nothing moved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_refs: Option<BTreeMap<String, RefMove>>,
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
            worktree_raw_before: entry.worktree_raw_before,
            backup_ref: entry.backup_ref.clone(),
            pins: entry.pins.clone(),
            reverses: entry.reverses.clone(),
            merge_before: entry.merge_before.clone(),
            config_before: entry.config_before.clone(),
            put_back: entry.put_back.clone(),
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
            worktree_raw_before: self.worktree_raw_before,
            backup_ref: self.backup_ref,
            pins: self.pins,
            commands: Vec::new(),
            reverses: self.reverses,
            merge_before: self.merge_before,
            merge_after: None,
            config_before: self.config_before,
            config_after: BTreeMap::new(),
            put_back: self.put_back,
            fetched_refs: None,
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
            merge_after: entry.merge_after.clone(),
            config_after: entry.config_after.clone(),
            fetched_refs: entry.fetched_refs.clone(),
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
        entry.merge_after = self.merge_after;
        entry.config_after = self.config_after;
        entry.fetched_refs = self.fetched_refs;
    }
}
