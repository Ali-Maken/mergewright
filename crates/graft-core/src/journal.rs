//! The undo journal's record shape (DESIGN.md §6.1, ADR-0031).
//!
//! `ops` writes these, `app` reads them back for the journal browser, `ui`
//! renders the view model built from them — three crates that may not depend
//! on each other, so the shape lives here and carries no behaviour
//! (DESIGN.md §8, ADR-0007). The journal is *state-based*: an entry records
//! what the repository looked like before and after an operation — refs,
//! `HEAD`, the index tree, the working-tree tree — never the parameters to run
//! it again. One restore path then serves undo, redo, recovery from an
//! operation that died half way, and ADR-0031's "what did the repo look like
//! at 14:20", which nothing exposes before M5 but the shape must answer now.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::oid::Oid;

/// A journal entry's id: a ULID, sortable by creation time and safe inside a
/// ref name, which is where it also appears (`refs/graft/backup/<worktree>/
/// <id>`, ADR-0010).
///
/// Twenty-six Crockford base32 characters: 48 bits of Unix milliseconds, then
/// 80 random bits. Ordering is by string, which for ULIDs is by time.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EntryId(String);

/// Crockford's base32 alphabet: digits and letters without `I`, `L`, `O`, `U`.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// How many characters a ULID has.
const ULID_LEN: usize = 26;

impl EntryId {
    /// A new id for an entry created at `unix_ms` with 80 bits of `random`.
    ///
    /// The caller supplies both so this stays pure: the clock and the
    /// randomness are `ops`'s to obtain. Milliseconds outside the 48-bit
    /// range (negative, or past the year 10889) are clamped rather than
    /// wrapped, so an absurd clock still yields a sortable id.
    #[must_use]
    pub fn new(unix_ms: i64, random: [u8; 10]) -> Self {
        let millis = u128::from(u64::try_from(unix_ms).unwrap_or(0).min((1 << 48) - 1));
        let mut random_bits = 0u128;
        for byte in random {
            random_bits = (random_bits << 8) | u128::from(byte);
        }
        let value = (millis << 80) | random_bits;
        let text = (0..ULID_LEN)
            .map(|i| {
                let shift = 5 * (ULID_LEN - 1 - i);
                let index = ((value >> shift) & 0x1f) as usize;
                char::from(CROCKFORD[index])
            })
            .collect();
        Self(text)
    }

    /// Parses a ULID; lowercase letters are accepted and stored upper case.
    ///
    /// # Errors
    /// [`Error::EntryId`] when `s` is not 26 Crockford base32 characters
    /// with a first character of `0`–`7` (a 128-bit value has 130 bits of
    /// room in 26 characters; the top two must be zero).
    pub fn parse(s: &str) -> Result<Self, Error> {
        let invalid = || Error::EntryId {
            text: s.escape_debug().to_string(),
        };
        if s.len() != ULID_LEN {
            return Err(invalid());
        }
        let upper = s.to_ascii_uppercase();
        let well_formed = upper.bytes().all(|b| CROCKFORD.contains(&b));
        let first_in_range = upper.as_bytes()[0] <= b'7';
        if well_formed && first_in_range {
            Ok(Self(upper))
        } else {
            Err(invalid())
        }
    }

    /// The id's text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for EntryId {
    type Error = Error;

    fn try_from(text: String) -> Result<Self, Error> {
        Self::parse(&text)
    }
}

impl From<EntryId> for String {
    fn from(id: EntryId) -> Self {
        id.0
    }
}

impl std::fmt::Display for EntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What kind of operation an entry records.
///
/// `Undo` and `Redo` are ordinary entries with `reverses` set (DESIGN.md
/// §6.5): undo is itself undoable, and redo is not a second data structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    /// `git add`: paths or a patch into the index.
    Stage,
    /// The reverse of `Stage`.
    Unstage,
    /// Drop working-tree and index changes to paths or a patch.
    Discard,
    /// A new commit from the index.
    Commit,
    /// `HEAD` rewritten from the index and a new message.
    Amend,
    /// `git stash push`.
    StashPush,
    /// `git stash apply`.
    StashApply,
    /// `git stash pop`.
    StashPop,
    /// `git stash drop`.
    StashDrop,
    /// `git fetch`: moves remote-tracking refs only. Journaled, never undone.
    Fetch,
    /// A fast-forward pull: `fetch` then `merge --ff-only`. Undoable locally.
    Pull,
    /// `git push`: changes the remote. Journaled, never undone.
    Push,
    /// The inverse of an earlier entry.
    Undo,
    /// The inverse of an `Undo` entry.
    Redo,
}

/// Where an entry is in the recording protocol (DESIGN.md §6.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryStatus {
    /// The before-state is recorded and the operation may have started. A
    /// `Pending` entry with no outcome after a restart is surfaced, never
    /// assumed to have succeeded.
    Pending,
    /// The operation finished and the after-state is recorded.
    Committed,
    /// The operation failed or was stopped. With a backup taken it is still
    /// undoable: a `stash pop` that dies half way leaves a half-applied tree
    /// (design note, Proposed change 2).
    Aborted {
        /// Why, in one line for the journal browser.
        reason: String,
    },
}

/// What an operation changes: the refs by full name, the index, the working
/// tree. Decides the snapshot's scope and the inverse's generic steps.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Touches {
    /// Full ref names (`refs/heads/main`, `refs/stash`), or `HEAD` when
    /// detached.
    pub refs: Vec<String>,
    /// The index is written.
    pub index: bool,
    /// Files in the working tree are written or removed.
    pub worktree: bool,
}

/// Where `HEAD` pointed. Branch names are full (`refs/heads/main`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadRef {
    /// A symbolic `HEAD` on an existing branch.
    Branch(String),
    /// `HEAD` on a commit directly.
    Detached(Oid),
    /// A symbolic `HEAD` on a branch that has no commit yet.
    Unborn(String),
}

/// One journaled operation: its before- and after-state, and what it needs.
///
/// The before-fields are known when the `Pending` line is written; the
/// after-fields arrive with the outcome line. A field that was never
/// recorded is `None`/empty, which the inverse and the drift check treat as
/// "unknown", never as "unchanged".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    /// Unique, sortable; also the backup ref's last segment.
    pub id: EntryId,
    /// When the entry was opened, Unix milliseconds.
    pub at_unix_ms: i64,
    /// What kind of operation.
    pub kind: OperationKind,
    /// The journal browser's text: `stage src/a.rs`, `commit "fix"`.
    pub description: String,
    /// Where the entry is in the recording protocol.
    pub status: EntryStatus,
    /// What the operation was going to change.
    pub touches: Touches,
    /// `HEAD` before the operation.
    pub head_before: HeadRef,
    /// `HEAD` after; `None` until the outcome is recorded.
    pub head_after: Option<HeadRef>,
    /// Every touched ref's value before; `None` for a ref that did not exist.
    pub refs_before: BTreeMap<String, Option<Oid>>,
    /// The same refs after the operation.
    pub refs_after: BTreeMap<String, Option<Oid>>,
    /// The index as a tree id before, when the operation touches the index.
    pub index_before: Option<Oid>,
    /// The index tree after.
    pub index_after: Option<Oid>,
    /// The working tree — tracked content and untracked files — as a tree
    /// id before, when the operation touches the working tree (§6.3).
    pub worktree_before: Option<Oid>,
    /// The working-tree tree after.
    pub worktree_after: Option<Oid>,
    /// `refs/graft/backup/<worktree>/<id>`: the commit keeping the before
    /// trees and `pins` reachable (ADR-0010).
    pub backup_ref: Option<String>,
    /// Commits the inverse needs alive: a dropped or popped stash.
    pub pins: Vec<Oid>,
    /// The Command Log: every `git` line the operation ran, in order.
    pub commands: Vec<String>,
    /// For `Undo`/`Redo`: the entry this one reverses.
    pub reverses: Option<EntryId>,
}

/// The reason recovery writes on an entry a previous run left open.
///
/// Public because it is the *only* thing that distinguishes "the process
/// died" from "the operation failed" once recovery has rewritten the line,
/// and every reader that tells the user which of the two happened has to
/// agree on the word.
pub const INTERRUPTED: &str = "interrupted";

impl JournalEntry {
    /// Did a previous run die with this operation half-done?
    ///
    /// Two shapes say so, and a reader must accept both. `Pending` is what
    /// the *file* holds — an entry opened and never closed. `Aborted` with
    /// [`INTERRUPTED`] is what a reader is handed, because recovery closes
    /// such an entry (sealing its backup) before anyone reads the journal.
    /// Keying on `Pending` alone therefore looks right and never fires.
    #[must_use]
    pub fn is_interrupted(&self) -> bool {
        match &self.status {
            EntryStatus::Pending => true,
            EntryStatus::Aborted { reason } => reason == INTERRUPTED,
            EntryStatus::Committed => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ulid_encodes_time_then_random_in_26_crockford_characters() {
        let id = EntryId::new(1_700_000_000_000, [0xff; 10]);
        assert_eq!(id.as_str().len(), 26);
        // 1_700_000_000_000 ms is 0x18BCFE56800: the well-known ULID prefix.
        assert!(id.as_str().starts_with("01HF"), "{id}");
        assert!(id.as_str().ends_with("ZZZZZZZZZZZZZZZZ"), "{id}");
    }

    #[test]
    fn ulids_order_by_time_then_by_random() {
        let earlier = EntryId::new(1000, [0xff; 10]);
        let later = EntryId::new(1001, [0; 10]);
        assert!(earlier < later);
        let a = EntryId::new(1000, [0; 10]);
        let b = EntryId::new(1000, [1; 10]);
        assert!(a < b);
    }

    #[test]
    fn parse_round_trips_and_rejects_the_malformed() {
        let id = EntryId::new(42, [7; 10]);
        assert_eq!(EntryId::parse(id.as_str()).unwrap(), id);
        assert_eq!(
            EntryId::parse(&id.as_str().to_ascii_lowercase()).unwrap(),
            id
        );
        for bad in [
            "",
            "01HF",
            "8ZZZZZZZZZZZZZZZZZZZZZZZZZ",
            "01HFZZZZZZZZZZZZZZZZZZZZZU",
        ] {
            assert!(
                matches!(EntryId::parse(bad), Err(Error::EntryId { .. })),
                "{bad}"
            );
        }
    }

    #[test]
    fn absurd_clocks_clamp_instead_of_wrapping() {
        assert_eq!(EntryId::new(-5, [0; 10]), EntryId::new(0, [0; 10]));
        let max = EntryId::new(i64::MAX, [0; 10]);
        assert!(max.as_str().starts_with('7'), "{max}");
    }
}
