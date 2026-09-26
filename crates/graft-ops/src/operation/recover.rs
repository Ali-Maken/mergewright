//! What a put-back from the journal's list aims at (ADR-0046 §3). The types
//! only: `journal_list::recover_for` chooses one; the operation that runs it
//! — its check, plan, touches and preview — is recover-op's
//! (m7-power-tools/22 narrowed the note's `Recover::Run(Operation)` to the
//! params, so nothing here is a stub).

use std::path::PathBuf;

use graft_core::journal::EntryId;
use graft_core::oid::Oid;

/// The thing a put-back changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoverTarget {
    /// Move an existing branch back: `name` (short) to `to`, but only while
    /// it still sits at `expected` — a compare-and-swap, so a branch that
    /// moved on since is never moved over.
    MoveRef {
        /// The branch, short name (`main`).
        name: String,
        /// Where it goes back to.
        to: Oid,
        /// Where it must be now.
        expected: Oid,
    },
    /// Make a deleted branch again at its lost tip.
    Recreate {
        /// The branch, short name (`spike`).
        name: String,
        /// Its tip.
        at: Oid,
    },
    /// Put back an edit an undo (or redo) replaced, from that entry's
    /// backup tree. The index is untouched.
    Files {
        /// The undo or redo that replaced it.
        entry: EntryId,
        /// Its backup's working-tree tree, which holds the edit.
        backup_tree: Oid,
        /// The files to put back.
        paths: Vec<PathBuf>,
    },
}

/// A put-back, and the event it undoes in words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoverParams {
    /// What changes.
    pub target: RecoverTarget,
    /// The event it puts back, as the journal's description ends:
    /// `the shell's reset`, `the shell's branch -D`, `undo of "commit x"`.
    pub because: String,
}

/// Why a row of the journal's list has no put-back of its own
/// ([`super::Refusal::NoPutBack`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoPutBack {
    /// A row Graft journaled: `u` undoes it, and `⏎` on it is the time
    /// machine's revert-to (ADR-0044 §2), never a put-back.
    Graft,
    /// The shell's switch: it moved no branch, so there is nothing to put
    /// back. The app's switch goes back instead.
    Switch {
        /// The branch (or commit) `HEAD` was on before, as git named it.
        back_to: String,
    },
    /// The step made the ref: before it there was nothing to go back to.
    Created {
        /// The branch, short name.
        name: String,
    },
    /// No such row: the list changed under the cursor.
    Nothing,
}
