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

mod fetched;
mod file;
mod lines;

use std::collections::{BTreeMap, BTreeSet, HashSet};

use mergewright_core::journal::{EntryId, EntryStatus, HeadRef, IndexState, JournalEntry, OperationKind};
use mergewright_core::oid::Oid;

pub use fetched::{moved_lines, moved_words, wrote_anything};
pub use file::{Alone, Journal, journal_path, journal_path_in};
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
    /// [`Journal::alone`] asked of a `Journal` holding the writer's lock: a
    /// caller's mistake, refused rather than done (58r).
    #[error("the journal was asked to recover while it is still writing an entry")]
    StillWriting,
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

/// Whether an entry changed the repository. `Committed` always did: it is
/// what the user did, even when git found nothing to do. `Aborted` only when
/// it has a backup and got some way — its recorded after-state differs from
/// its before-state somewhere, or a touched field was never recorded after
/// (design note Proposed change 2). A failure that changed nothing — a hook
/// that rejected the commit, a stash push with nothing staged, an undo whose
/// compare-and-swap refused — never happened as far as the cursor is
/// concerned: it must not shadow the real `next_undo`, and an aborted undo
/// must not mark its target reversed while it is still fully in effect.
fn took_effect(entry: &JournalEntry) -> bool {
    match entry.status {
        EntryStatus::Committed => true,
        EntryStatus::Aborted { .. } => entry.worktree_before.is_some() && changed_anything(entry),
        EntryStatus::Pending => false,
    }
}

/// Whether any recorded after-field differs from its before-field. A touched
/// field with no after-value counts as changed: unknown is not "unchanged"
/// (`JournalEntry`'s own rule), and the inverse then restores all of it.
fn changed_anything(entry: &JournalEntry) -> bool {
    let refs = entry
        .refs_before
        .iter()
        .any(|(name, before)| entry.refs_after.get(name) != Some(before));
    let head = entry.head_after.as_ref() != Some(&entry.head_before);
    let index = entry.touches.index && entry.index_after != entry.index_before;
    let worktree = entry.touches.worktree && entry.worktree_after != entry.worktree_before;
    // git's merge files are state like any other: an undo of a finished
    // merge that failed after putting them back changed the repository.
    let merge = entry.merge_after != entry.merge_before;
    refs || head || index || worktree || merge
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

/// One flag per entry, in the journal's own order: the browser marks every
/// row at once, and asking [`in_force`] per row would walk the journal once
/// per row (m3-working-tree-undo-and-stash/06).
#[must_use]
pub fn in_force_all(entries: &[JournalEntry]) -> Vec<bool> {
    in_force_flags(entries)
}

/// Whether the entry is an operation of the user's rather than a movement
/// along the undo stack. Inverting an `Undo` entry *re-applies* what it
/// reversed, so anything that walks the journal backwards — `u`
/// ([`next_undo`]) and the time machine ([`crate::undo::plan_revert_to`])
/// alike — must step over `Undo` and `Redo` entries rather than invert them.
pub(crate) fn is_operation(entry: &JournalEntry) -> bool {
    !matches!(entry.kind, OperationKind::Undo | OperationKind::Redo)
}

/// Whether `u` and `U` can see an entry of this kind. Fetch and push are
/// journaled — the Command Log and the browser show them — but transparent
/// to undo: `u` never talks to a remote, so it steps past them to the local
/// operation before (M4 design note, hard choice 1). Pull is undoable: it
/// moved only the local branch and tree. The rename's `Migrate` entry
/// (ticket 26) is a record of a one-time move the user ruled can never be
/// moved back, so it is transparent the same way.
///
/// A negative list: a kind added later is undoable unless named here.
#[must_use]
pub fn undoable_kind(kind: OperationKind) -> bool {
    !matches!(
        kind,
        OperationKind::Fetch | OperationKind::Push | OperationKind::Migrate
    )
}

/// Whether `u` and `U` can see this entry: its kind is undoable, and it is
/// not a pull that found nothing to do. `merge --ff-only` on an up-to-date
/// branch exits 0, so such a pull is `Committed` with every after-field equal
/// to its before-field; `u` on it would change nothing and cost the user a
/// second keystroke. It stays in the journal — its fetch did run, and may
/// have moved tracking refs the browser should show — but is transparent to
/// undo, like a fetch (m4/04 review, M3). A pull that did move something is
/// undoable as before.
#[must_use]
pub fn undoable(entry: &JournalEntry) -> bool {
    undoable_kind(entry.kind) && !is_no_op_pull(entry)
}

fn is_no_op_pull(entry: &JournalEntry) -> bool {
    entry.kind == OperationKind::Pull
        && entry.status == EntryStatus::Committed
        && !changed_anything(entry)
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
/// what they put back — and never a fetch, a push or a pull that moved
/// nothing, see [`undoable`]).
#[must_use]
pub fn next_undo(entries: &[JournalEntry]) -> Option<&JournalEntry> {
    let flags = in_force_flags(entries);
    entries
        .iter()
        .zip(flags)
        .rev()
        .find(|(entry, flag)| *flag && is_operation(entry) && undoable(entry))
        .map(|(entry, _)| entry)
}

/// What `U` redoes next: the newest in-force `Undo` entry, unless an in-force
/// operation is newer than it — a new operation after an undo moves the
/// cursor past that branch of history (§6.5). A fetch, a push or a pull that
/// moved nothing after the undo does not: none is undoable, so none ends the
/// redo branch.
#[must_use]
pub fn next_redo(entries: &[JournalEntry]) -> Option<&JournalEntry> {
    let flags = in_force_flags(entries);
    for (entry, flag) in entries.iter().zip(flags).rev() {
        if !flag || !undoable(entry) {
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
            // Mergewright; treated as no root rather than looping.
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
    pub index: Option<IndexState>,
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
