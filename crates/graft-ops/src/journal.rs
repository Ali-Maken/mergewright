//! The undo journal: its line format, the undo/redo cursor over its entries,
//! and ADR-0031's point-in-time query. Pure: the `Journal` handle that opens
//! and appends the file is `ops-execution`'s.
//!
//! The journal is append-only JSONL, two lines per entry (`lines`). Undo and
//! redo are not a stack kept beside it: they are read off the entries
//! themselves (DESIGN.md §6.5). An entry is *in force* when it changed the
//! repository and nothing in force since has reversed it; `next_undo` is the
//! newest in-force operation, `next_redo` the newest in-force undo that no
//! newer operation has moved past.

mod lines;

use std::collections::{BTreeMap, BTreeSet, HashSet};

use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind};
use graft_core::oid::Oid;

pub use lines::{Line, OutcomeLine, PendingLine};

/// Why the journal's bytes could not be read as entries.
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    /// A line that is not one of the two records.
    #[error("journal line {line}: {what}")]
    Json {
        /// 1-based line number.
        line: usize,
        /// serde's own message.
        what: String,
    },
    /// A truncated record somewhere other than the end: the file was torn
    /// and then written past, which no crash produces.
    #[error("journal line {line} is torn but not the last line")]
    TornTail {
        /// 1-based line number.
        line: usize,
    },
    /// Two `Pending` lines with one id.
    #[error("journal entry {0} opened twice")]
    DuplicatePending(EntryId),
    /// An outcome line whose id no `Pending` line opened.
    #[error("journal entry {0} has an outcome but was never opened")]
    OutcomeWithoutPending(EntryId),
    /// Two outcome lines with one id.
    #[error("journal entry {0} has two outcomes")]
    DuplicateOutcome(EntryId),
    /// An outcome line with `status: pending`, or the reverse.
    #[error("journal entry {0} cannot be rendered as an outcome while still pending")]
    StillPending(EntryId),
    /// Reading or writing the file: `ops-execution`'s handle wraps its I/O in
    /// this same error.
    #[error("journal file: {0}")]
    Io(#[from] std::io::Error),
}

/// What `parse` read: the entries, and whether the file ends in a torn line.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    /// Every opened entry, in the order its `Pending` line appears.
    pub entries: Vec<JournalEntry>,
    /// The 1-based number of a truncated last line — a crash mid-append. It
    /// carried nothing the entries need: a torn `Pending` line never ran, a
    /// torn outcome leaves its entry `Pending`, which the app surfaces.
    pub torn_tail: Option<usize>,
}

/// Reads a journal file's bytes. Pure.
///
/// # Errors
/// [`JournalError`] for a malformed line, a torn line that is not the last,
/// an outcome without its `Pending` line, or a duplicated line.
pub fn parse(bytes: &[u8]) -> Result<Parsed, JournalError> {
    let text = String::from_utf8_lossy(bytes);
    let raw_lines: Vec<&str> = text.split('\n').collect();
    let last = raw_lines.len();
    let mut entries: Vec<JournalEntry> = Vec::new();
    let mut by_id: BTreeMap<EntryId, usize> = BTreeMap::new();
    let mut closed: HashSet<EntryId> = HashSet::new();
    let mut torn_tail = None;
    for (number, raw) in (1..).zip(raw_lines) {
        if raw.trim().is_empty() {
            continue;
        }
        let line = match Line::from_json(raw) {
            Ok(line) => line,
            Err(error) if error.is_eof() && number == last => {
                torn_tail = Some(number);
                break;
            }
            Err(error) if error.is_eof() => return Err(JournalError::TornTail { line: number }),
            Err(error) => {
                return Err(JournalError::Json {
                    line: number,
                    what: error.to_string(),
                });
            }
        };
        match line {
            Line::Pending(pending) => {
                if by_id.contains_key(&pending.id) {
                    return Err(JournalError::DuplicatePending(pending.id));
                }
                by_id.insert(pending.id.clone(), entries.len());
                entries.push(pending.into_entry());
            }
            Line::Outcome(outcome) => {
                let Some(&index) = by_id.get(&outcome.id) else {
                    return Err(JournalError::OutcomeWithoutPending(outcome.id));
                };
                if !closed.insert(outcome.id.clone()) {
                    return Err(JournalError::DuplicateOutcome(outcome.id));
                }
                outcome.apply_to(&mut entries[index]);
            }
        }
    }
    Ok(Parsed { entries, torn_tail })
}

/// The `Pending` line for an entry, newline included: the bytes to append
/// before the operation runs.
///
/// # Errors
/// [`JournalError::Json`] if serde cannot encode it (it always can: the
/// shape has no map keys but strings).
pub fn render_pending(entry: &JournalEntry) -> Result<Vec<u8>, JournalError> {
    render(&PendingLine::of(entry))
}

/// The outcome line for a `Committed` or `Aborted` entry, newline included.
///
/// # Errors
/// [`JournalError::StillPending`] for a `Pending` entry; `Json` as above.
pub fn render_outcome(entry: &JournalEntry) -> Result<Vec<u8>, JournalError> {
    if entry.status == EntryStatus::Pending {
        return Err(JournalError::StillPending(entry.id.clone()));
    }
    render(&OutcomeLine::of(entry))
}

fn render<T: serde::Serialize>(line: &T) -> Result<Vec<u8>, JournalError> {
    let mut bytes = serde_json::to_vec(line).map_err(|error| JournalError::Json {
        line: 0,
        what: error.to_string(),
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Whether an entry changed the repository: `Committed`, or `Aborted` with a
/// backup (it may have got half way, design note Proposed change 2).
fn took_effect(entry: &JournalEntry) -> bool {
    match entry.status {
        EntryStatus::Committed => true,
        EntryStatus::Aborted { .. } => entry.worktree_before.is_some(),
        EntryStatus::Pending => false,
    }
}

/// One flag per entry: it took effect and no later in-force entry reverses
/// it. Walks backwards so each reverser's own flag is known first.
fn in_force_flags(entries: &[JournalEntry]) -> Vec<bool> {
    let mut flags = vec![false; entries.len()];
    let mut reversed: HashSet<&EntryId> = HashSet::new();
    for (index, entry) in entries.iter().enumerate().rev() {
        let in_force = took_effect(entry) && !reversed.contains(&entry.id);
        flags[index] = in_force;
        if let Some(target) = entry.reverses.as_ref().filter(|_| in_force) {
            reversed.insert(target);
        }
    }
    flags
}

fn is_operation(entry: &JournalEntry) -> bool {
    !matches!(entry.kind, OperationKind::Undo | OperationKind::Redo)
}

/// Whether the entry with `id` is in force: it took effect and no later
/// in-force entry has `reverses == Some(id)`.
#[must_use]
pub fn in_force(entries: &[JournalEntry], id: &EntryId) -> bool {
    let flags = in_force_flags(entries);
    entries
        .iter()
        .zip(flags)
        .any(|(entry, flag)| flag && entry.id == *id)
}

/// What `u` undoes next: the newest in-force operation (never an `Undo` or
/// `Redo` entry — those are stepped over through `next_redo` and by undoing
/// what they put back).
#[must_use]
pub fn next_undo(entries: &[JournalEntry]) -> Option<&JournalEntry> {
    let flags = in_force_flags(entries);
    entries
        .iter()
        .zip(flags)
        .rev()
        .find(|(entry, flag)| *flag && is_operation(entry))
        .map(|(entry, _)| entry)
}

/// What `U` redoes next: the newest in-force `Undo` entry, unless an in-force
/// operation is newer than it — a new operation after an undo moves the
/// cursor past that branch of history (§6.5).
#[must_use]
pub fn next_redo(entries: &[JournalEntry]) -> Option<&JournalEntry> {
    let flags = in_force_flags(entries);
    for (entry, flag) in entries.iter().zip(flags).rev() {
        if !flag {
            continue;
        }
        if is_operation(entry) {
            return None;
        }
        if entry.kind == OperationKind::Undo {
            return Some(entry);
        }
    }
    None
}

/// The entry `entry.reverses` names, followed through `Undo`/`Redo` chains
/// down to the operation at the root; `None` for an operation entry itself
/// or a dangling `reverses`.
#[must_use]
pub fn root_operation<'a>(
    entries: &'a [JournalEntry],
    entry: &JournalEntry,
) -> Option<&'a JournalEntry> {
    let mut current = entry.reverses.as_ref()?;
    let mut hops = 0;
    loop {
        let target = entries.iter().find(|candidate| candidate.id == *current)?;
        if is_operation(target) {
            return Some(target);
        }
        current = target.reverses.as_ref()?;
        hops += 1;
        if hops > entries.len() {
            // A cycle: two entries reversing each other. Never written by
            // Graft; treated as no root rather than looping.
            return None;
        }
    }
}

/// The repository's state at one moment, as far as the journal knows it: only
/// the refs some entry touched, and only when an entry is at or before `at`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PointInTime {
    /// `HEAD`, when any entry recorded it.
    pub head: Option<HeadRef>,
    /// Every ref any entry touched, with its value at `at`.
    pub refs: BTreeMap<String, Option<Oid>>,
    /// The index tree.
    pub index: Option<Oid>,
    /// The working-tree tree.
    pub worktree: Option<Oid>,
}

/// ADR-0031's query: what the repository looked like at `at_unix_ms`. For
/// each thing the journal tracks, the after-value of the newest entry at or
/// before `at` that recorded it, else the before-value of the oldest entry
/// after `at` that did. `None` when no entry is at or before `at`: before
/// the journal started watching, it cannot say. Nothing calls this in M3;
/// it proves the shape answers.
#[must_use]
pub fn state_at(entries: &[JournalEntry], at_unix_ms: i64) -> Option<PointInTime> {
    let (before, after): (Vec<_>, Vec<_>) = entries
        .iter()
        .partition(|entry| entry.at_unix_ms <= at_unix_ms);
    if before.is_empty() {
        return None;
    }
    // Newest first for the past, oldest first for the future.
    let mut past = before;
    past.sort_by_key(|entry| std::cmp::Reverse(entry.at_unix_ms));
    let mut future = after;
    future.sort_by_key(|entry| entry.at_unix_ms);

    let names: BTreeSet<&String> = entries
        .iter()
        .flat_map(|entry| entry.refs_before.keys().chain(entry.refs_after.keys()))
        .collect();
    let refs = names
        .into_iter()
        .map(|name| {
            let value = past
                .iter()
                .find_map(|entry| entry.refs_after.get(name).copied())
                .or_else(|| {
                    future
                        .iter()
                        .find_map(|entry| entry.refs_before.get(name).copied())
                })
                .flatten();
            (name.clone(), value)
        })
        .collect();
    let head = past
        .iter()
        .find_map(|entry| entry.head_after.clone())
        .or_else(|| future.first().map(|entry| entry.head_before.clone()));
    let index = past
        .iter()
        .find_map(|entry| entry.index_after)
        .or_else(|| future.iter().find_map(|entry| entry.index_before));
    let worktree = past
        .iter()
        .find_map(|entry| entry.worktree_after)
        .or_else(|| future.iter().find_map(|entry| entry.worktree_before));
    Some(PointInTime {
        head,
        refs,
        index,
        worktree,
    })
}

#[cfg(test)]
pub(crate) mod tests;
