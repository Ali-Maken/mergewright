//! A lost edit put back from the journal's list (`RecoverTarget::Files`,
//! m7-power-tools/30w): his own bytes over git's checkout, where the backup
//! kept them (ADR-0052).
//!
//! The put-back's plan is `git restore --source=<backup_tree> --worktree`,
//! and git smudges what it writes: an LF file under `core.autocrlf=true`
//! comes back CRLF, a line a one-way clean filter strips stays stripped. The
//! entry that took that backup — the undo or redo whose row this is — also
//! took its `raw/` tree: his bytes for every path git's checkout would not
//! give back byte-exact. This writes them over the restore, through the same
//! [`write_raw`] `u` uses.
//!
//! A path the backup kept no raw bytes for is left as git wrote it: git's
//! checkout gave it back byte-exact under the config in force when the backup
//! was taken (ADR-0052 §2). If he has changed `core.autocrlf` since, it comes
//! back as every other checkout in his repo now does.

use std::path::PathBuf;

use graft_core::journal::{EntryId, JournalEntry};
use graft_core::oid::Oid;
use graft_git::run::Stop;

use crate::operation::{Operation, RecoverParams, RecoverTarget};
use crate::restore::{RestoreError, write_raw};

use super::{AfterState, Context};

/// The raw step for `op`, only when its plan `went_through` (exited 0):
/// `None` for anything but a file put-back, and for one whose backup kept
/// nothing raw (0 spawns). `Some` is the step's failure, for `unread[0]`.
pub(super) fn after_put_back(
    ctx: &mut Context<'_>,
    op: &Operation,
    went_through: bool,
    commands: &mut Vec<String>,
) -> Option<AfterState> {
    if !went_through {
        return None;
    }
    let Operation::Recover(RecoverParams {
        target:
            RecoverTarget::Files {
                entry,
                backup_tree,
                paths,
            },
        ..
    }) = op
    else {
        return None;
    };
    let raw = his_bytes(ctx.journal.entries(), entry, *backup_tree)?;
    write_back(ctx, raw, paths, commands).err()
}

/// The `raw/` tree holding his own bytes for a lost edit put back from
/// `backup_tree`: the entry's `worktree_raw_before`, only when its
/// `worktree_before` is `backup_tree` — a `raw/` belongs to the tree it was
/// taken beside and no other. `None` → nothing to write (+0 spawns).
pub(super) fn his_bytes(
    entries: &[JournalEntry],
    entry: &EntryId,
    backup_tree: Oid,
) -> Option<Oid> {
    entries
        .iter()
        .find(|recorded| &recorded.id == entry)
        .filter(|recorded| recorded.worktree_before == Some(backup_tree))
        .and_then(|recorded| recorded.worktree_raw_before)
}

/// After the put-back's `restore --source` exited 0: his raw bytes over each
/// of `paths` that `raw` holds, through [`write_raw`]. No check of what is on
/// disk first (30s's `same_content_paths`): the restore has just written the
/// backup's blob at exactly these paths, so each is his content.
///
/// Never stopped: the restore has run, and a stop here would leave git's
/// encoding with no record of it, so `ctx`'s stop is swapped for one that
/// never fires and handed back on the way out. Every spawn is pushed to
/// `commands`, the entry's Command Log.
///
/// Err is [`AfterState::NotAsTheyWere`]: `files` is `paths.len()` when a
/// write failed ([`RestoreError::WriteRaw`]), 0 when the read before it did.
pub(super) fn write_back(
    ctx: &mut Context<'_>,
    raw: Oid,
    paths: &[PathBuf],
    commands: &mut Vec<String>,
) -> Result<(), AfterState> {
    // Bytes as git names them: `write_raw` matches them against `ls-tree`'s.
    let named: Vec<Vec<u8>> = paths
        .iter()
        .map(|path| path.as_os_str().as_encoded_bytes().to_vec())
        .collect();
    let never: &dyn Stop = &();
    let asked = std::mem::replace(&mut ctx.stop, never);
    let written = write_raw(ctx, raw, &named, commands);
    ctx.stop = asked;
    written.map_err(|error| {
        let files = match error {
            RestoreError::WriteRaw { .. } => paths.len(),
            _ => 0,
        };
        AfterState::NotAsTheyWere { error, files }
    })
}

#[cfg(test)]
mod tests {
    use graft_core::journal::OperationKind;

    use super::*;
    use crate::journal::tests::{committed, id, oid};

    /// An undo at `at` whose before-tree is `before` and `raw/` tree `raw`.
    fn undo(at: i64, before: u8, raw: Option<u8>) -> JournalEntry {
        JournalEntry {
            worktree_before: Some(oid(before)),
            worktree_raw_before: raw.map(oid),
            ..committed(at, OperationKind::Undo, 0, 0)
        }
    }

    #[test]
    fn his_bytes_is_the_entrys_raw_tree_when_its_before_tree_is_the_backup() {
        let entries = [undo(1, 1, Some(9)), undo(2, 2, Some(8))];
        assert_eq!(his_bytes(&entries, &id(2), oid(2)), Some(oid(8)));
    }

    #[test]
    fn his_bytes_is_none_for_another_tree_no_raw_or_no_such_entry() {
        let kept = [undo(1, 1, Some(9))];
        assert_eq!(his_bytes(&kept, &id(1), oid(2)), None, "another tree");
        let none = [undo(1, 1, None)];
        assert_eq!(his_bytes(&none, &id(1), oid(1)), None, "nothing kept raw");
        assert_eq!(his_bytes(&kept, &id(2), oid(1)), None, "no such entry");
    }
}
