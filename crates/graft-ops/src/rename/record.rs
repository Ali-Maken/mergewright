//! The move's one journal entry: a `Migrate` record in the opening
//! worktree's new journal, so J says when and what moved. Not an operation
//! `u` can see (`journal::undoable_kind`).

use std::collections::BTreeMap;
use std::hash::{BuildHasher, RandomState};
use std::time::{SystemTime, UNIX_EPOCH};

use graft_core::journal::{EntryId, EntryStatus, JournalEntry, OperationKind, Touches};
use graft_core::on_disk::OnDiskNames;
use graft_git::repo::RepoHandle;

use crate::facts;
use crate::journal::{Journal, journal_path_in};

/// The entry's words, which J shows as its row:
/// `rename to Tributary (3 refs, 12 files)`.
#[must_use]
pub fn description(refs: usize, files: usize) -> String {
    let plural =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    format!(
        "rename to Tributary ({}, {})",
        plural(refs, "ref", "refs"),
        plural(files, "file", "files")
    )
}

/// Append the entry, both lines. `Err` is why it could not be, in words;
/// the move stands either way.
pub(super) fn append(
    repo: &RepoHandle,
    refs: usize,
    files: usize,
    command: &str,
) -> Result<(), String> {
    let path = journal_path_in(repo, OnDiskNames::New).map_err(|error| error.to_string())?;
    let mut journal = Journal::open(&path).map_err(|error| error.to_string())?;
    let head = facts::head_ref(repo).map_err(|error| error.to_string())?;
    let now_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("the clock reads before 1970: {error}"))
        .and_then(|since| i64::try_from(since.as_millis()).map_err(|error| error.to_string()))?;
    let mut entry = JournalEntry {
        id: EntryId::new(now_unix_ms, random_tail()),
        at_unix_ms: now_unix_ms,
        kind: OperationKind::Migrate,
        description: description(refs, files),
        status: EntryStatus::Pending,
        touches: Touches::default(),
        head_after: Some(head.clone()),
        head_before: head,
        refs_before: BTreeMap::new(),
        refs_after: BTreeMap::new(),
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        worktree_raw_before: None,
        backup_ref: None,
        pins: Vec::new(),
        commands: Vec::new(),
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: BTreeMap::new(),
        config_after: BTreeMap::new(),
        put_back: None,
        fetched_refs: None,
    };
    journal
        .append_pending(&entry)
        .map_err(|error| error.to_string())?;
    entry.status = EntryStatus::Committed;
    if !command.is_empty() {
        entry.commands = vec![command.to_owned()];
    }
    journal
        .append_outcome(&entry)
        .map_err(|error| error.to_string())
}

/// Ten bytes for the id's random half. std's `RandomState` is seeded from
/// the operating system's randomness; this crate has no other source and
/// takes no new dependency for one id.
fn random_tail() -> [u8; 10] {
    let state = RandomState::new();
    let high = state.hash_one(std::process::id()).to_le_bytes();
    let low = state.hash_one(high).to_le_bytes();
    let mut tail = [0u8; 10];
    tail[..8].copy_from_slice(&high);
    tail[8..].copy_from_slice(&low[..2]);
    tail
}
