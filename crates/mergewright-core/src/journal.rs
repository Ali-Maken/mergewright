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

use crate::oid::Oid;

mod entry_id;
mod merge_state;

pub use entry_id::EntryId;
pub use merge_state::MergeState;

/// What kind of operation an entry records.
///
/// `Undo` and `Redo` are ordinary entries with `reverses` set (DESIGN.md
/// §6.5): `U` takes an `Undo` back by journaling a `Redo` against it, so
/// redo is not a second data structure.
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
    /// `git merge`: a merge commit or a fast-forward onto the current branch.
    /// One entry from start to finish, `Pending` while conflicts are open.
    Merge,
    /// `git rebase`: the current branch replayed onto another commit. One
    /// entry across every conflict stop.
    Rebase,
    /// A history rewrite of the current branch — squash, fixup, reword,
    /// reorder, drop, edit, or staged changes put into an older commit — run
    /// by git's own `rebase -i` on a todo Mergewright wrote, with every branch
    /// built on it following (ADR-0047). One entry across every stop; its
    /// inverse is a rebase's: every recorded branch back, then the index and
    /// the working tree.
    Rewrite,
    /// `git cherry-pick`: commits replayed onto `HEAD`.
    CherryPick,
    /// `git revert`: a commit's reverse patch committed onto `HEAD`.
    Revert,
    /// `git reset` of `HEAD`'s branch, or `update-ref` of another branch.
    Reset,
    /// `git switch`: `HEAD` moved to a branch or a commit.
    Checkout,
    /// `git branch <name> <commit>`, optionally checked out.
    BranchCreate,
    /// A branch deleted by compare-and-swap.
    BranchDelete,
    /// `git tag`, lightweight or annotated.
    TagCreate,
    /// A tag deleted by compare-and-swap.
    TagDelete,
    /// `git stash branch`: a branch at the stash's base with the stash
    /// applied and dropped.
    BranchFromStash,
    /// A conflicted path resolved whole, one side taken: the shapes with no
    /// markers to take from (`git checkout --ours|--theirs` then `add`, or
    /// `add`/`rm` on their own). Folded into the live operation's entry when
    /// one is open; its own entry when nothing is (m6-conflicts/03).
    ResolveConflict,
    /// A conflicted path put back the way git left it: `git checkout
    /// --merge`, the resolver's "start this file over" (m6-conflicts/03).
    RestartConflict,
    /// A merge **begun in the shell** finished from Mergewright: `git merge
    /// --continue`, or `git commit -F -` with the message the user edited.
    /// Always its own entry — there is no entry of Mergewright's to close — and
    /// its `u` puts the merge back in progress (m6-conflicts/20).
    FinishMerge,
    /// A merge **begun in the shell** aborted from Mergewright: `git merge
    /// --abort`. Its own entry with a full backup, so `u` puts the merge
    /// back as it stood — the stages, the hand edits, git's merge files
    /// (m6-conflicts/57).
    AbortMerge,
    /// A put-back from the journal's list (ADR-0046 §3, hard choice 5 of the
    /// M7 note): a branch moved back to where the shell's step found it, a
    /// deleted branch made again at its lost tip, or an edit an undo
    /// replaced put back from that undo's backup. Its own kind so the
    /// journal says what he did (`put main back (the shell's reset)`); its
    /// `u` is the generic inverse from the recorded refs and trees
    /// (m7-power-tools/22; the operation is recover-op's).
    Recover,
    /// The journal's time machine: every in-force entry back to a target,
    /// reversed one `Undo` entry at a time (DESIGN.md §6.6).
    ///
    /// **Nothing writes this, and on the design as it stands nothing will.**
    /// §6.6 decided the time machine journals one `Undo` entry *per step*,
    /// so that `U` walks the revert back a single operation at a time and
    /// the journal stays a record of what happened rather than a summary of
    /// it. A `RevertTo` entry would be exactly the summary that decision
    /// rejected. `mergewright_ops::undo::revert_to` therefore writes `Undo` lines
    /// and no entry ever carries this kind — which also makes
    /// `apply`'s `"revert to"` label for it unreachable.
    ///
    /// It is kept, not deleted, because the M5 design note names it and the
    /// call is the conductor's rather than a builder's. Note that the app's
    /// own `PreviewSubject::RevertTo` (ticket m5/08) is a different type
    /// with the same name: it identifies what a preview is *of*, and its
    /// existence is not a user of this variant (m5/07 job 2).
    RevertTo,
    /// The inverse of an earlier entry.
    Undo,
    /// The inverse of an `Undo` entry.
    Redo,
    /// A rename move an earlier build journaled; kept so those journals still
    /// read; never written, never undone. Transparent to `u`/`U` like a fetch.
    Migrate,
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

/// The index as an entry records it: one tree, and what shape it has.
///
/// A merged index writes straight out with `git write-tree`. An unmerged one
/// has no tree at all — `write-tree` refuses it — so it is recorded as the
/// **stage tree**: every index entry at `<stage>/<path>`, stage 0, so a
/// conflict's three sides are one object id like any other state
/// (m6-conflicts/08, ADR-0031: the journal answers whole-repo state at a
/// time, and an unmerged index is a state like any other).
///
/// The shape travels with the id because the two trees are not
/// interchangeable: handing a stage tree to `git restore --source` would
/// write `0/`, `1/`, `2/` and `3/` directories into the working tree.
///
/// On disk a `Merged` state is the bare oid string every entry written
/// before M6 holds, so those lines parse and render unchanged; `Unmerged` is
/// `{"unmerged":"<oid>"}`.
///
/// ```
/// use mergewright_core::journal::IndexState;
/// use mergewright_core::oid::Oid;
///
/// let id = Oid::from_hex("acf15d58a3b48e5878fe281a624fb5c688dcaee1").expect("a hex oid");
/// let merged = IndexState::Merged(id);
/// assert_eq!(merged.tree(), id);
/// assert!(!merged.is_unmerged());
/// assert_eq!(
///     serde_json::to_string(&merged).expect("it serialises"),
///     "\"acf15d58a3b48e5878fe281a624fb5c688dcaee1\""
/// );
///
/// let unmerged = IndexState::Unmerged { unmerged: id };
/// assert!(unmerged.is_unmerged());
/// assert_eq!(
///     serde_json::to_string(&unmerged).expect("it serialises"),
///     "{\"unmerged\":\"acf15d58a3b48e5878fe281a624fb5c688dcaee1\"}"
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IndexState {
    /// The plain tree `git write-tree` gives. Serialised as the bare oid
    /// string, which is exactly what every pre-M6 line holds.
    Merged(Oid),
    /// The stage tree `{ 0/<path>…, 1/<path>…, 2/<path>…, 3/<path>… }`, every
    /// entry at stage 0 so the tree is writable at all.
    Unmerged {
        /// The stage tree's id.
        unmerged: Oid,
    },
}

impl IndexState {
    /// The tree id, whichever shape this is. Read it only together with
    /// [`IndexState::is_unmerged`] — the two trees mean different things.
    #[must_use]
    pub fn tree(&self) -> Oid {
        match self {
            IndexState::Merged(tree) | IndexState::Unmerged { unmerged: tree } => *tree,
        }
    }

    /// Is this the stage tree of a half-finished merge?
    #[must_use]
    pub fn is_unmerged(&self) -> bool {
        matches!(self, IndexState::Unmerged { .. })
    }
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
    /// The index before, when the operation touches the index. `None` means
    /// only *never recorded* — an unmerged index has a state of its own
    /// ([`IndexState::Unmerged`]), not an absence.
    pub index_before: Option<IndexState>,
    /// The index after.
    pub index_after: Option<IndexState>,
    /// The working tree — tracked content and untracked files — as a tree
    /// id before, when the operation touches the working tree (§6.3).
    pub worktree_before: Option<Oid>,
    /// The working-tree tree after.
    pub worktree_after: Option<Oid>,
    /// The bytes on disk before, of the uncommitted files git's own checkout
    /// of `worktree_before` would not give back (ADR-0052): the backup
    /// commit's `raw/` tree. `None` = nothing kept, or a line written before
    /// the field existed; absent from its JSON either way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_raw_before: Option<Oid>,
    /// `refs/graft/backup/<worktree>/<id>`: the commit keeping the before
    /// trees and `pins` reachable (ADR-0010).
    pub backup_ref: Option<String>,
    /// Commits the inverse needs alive: a dropped or popped stash.
    pub pins: Vec<Oid>,
    /// The Command Log: every `git` line the operation ran, in order.
    pub commands: Vec<String>,
    /// For `Undo`/`Redo`: the entry this one reverses.
    pub reverses: Option<EntryId>,
    /// The merge git had in progress before the operation, recorded on
    /// **every** entry (m6-conflicts/20). `None` = not merging, or a line
    /// written before this field existed; the inverse acts only where the
    /// two sides differ, so an old line is safe.
    ///
    /// It is also the bound a later fix needs for `d` on a done file of a
    /// merge begun in the shell (`KNOWN_GAPS` M6/25): this merge's entries are
    /// the ones whose `merge_before.merge_head` is the current `MERGE_HEAD`
    /// and whose `head_before` is `HEAD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_before: Option<MergeState>,
    /// The merge git had in progress after it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_after: Option<MergeState>,
    /// Every `branch.<name>.*` key in the repository's own config, for each
    /// branch in `touches.refs`, before the operation: key → values (git's
    /// canonical key spelling; a key with several values keeps them in
    /// order). An absent key is simply not in the map. A tracking checkout
    /// writes `branch.<b>.remote/.merge`, and the inverse puts back exactly
    /// what differs from `config_after` (review m7-16b..21b, Major 1). Empty
    /// on a line written before the field existed, and absent from its JSON.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub config_before: BTreeMap<String, Vec<String>>,
    /// The same keys after the operation.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub config_after: BTreeMap<String, Vec<String>>,
    /// The stash this operation put back into the working tree, and why
    /// (ADR-0045 §1). Written on the `Pending` line by a pop or apply (the
    /// entry's own stash commit) and by a set-aside. `None` = no put-back, or
    /// a line written before the field existed; absent from the JSON then, so
    /// old lines re-render byte-identically. `mergewright_ops::live_clash` reads it
    /// to tell a stash clash from any other unmerged index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub put_back: Option<PutBack>,
    /// Every ref under `refs/` (Mergewright's own `refs/graft/` aside) whose value
    /// a fetch changed — a fetch's, or a pull's fetch line — read before git
    /// ran and again after it exited, success, failure or stop (ADR-0062).
    /// Written on the outcome line. **Outside the undo record**: nothing `u`
    /// reads looks at it, and `u` never moves these refs back.
    ///
    /// `Some(empty)` = read, nothing moved. `None` = not recorded: every
    /// other kind, a line written before the field existed, an interrupted
    /// entry, an after-read that failed. Absent from the JSON then, so old
    /// lines re-render byte-identically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_refs: Option<BTreeMap<String, RefMove>>,
}

/// One ref a fetch moved (ADR-0062): its value before and after. `None` is
/// absent — `before: None` a new ref, `after: None` one the fetch pruned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefMove {
    /// The ref's value before the fetch ran.
    pub before: Option<Oid>,
    /// Its value after git exited.
    pub after: Option<Oid>,
}

/// A stash Mergewright put back into the working tree (ADR-0045 §1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutBack {
    /// The stash's commit — found in the list by this id, never by position.
    pub stash: Oid,
    /// Which operation put it back.
    pub why: PutBackWhy,
}

/// Why a stash was put back: it decides whether finishing a clash drops it
/// (`Pop`, `SetAside`) or keeps it (`Apply`), ADR-0045 §3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PutBackWhy {
    /// `git stash pop`.
    Pop,
    /// `git stash apply`: the stash stays in the list whatever happens.
    Apply,
    /// The user's edits, set aside around a rebase or switch and put back
    /// after it (ADR-0045 §4).
    SetAside,
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

    /// The journal is a file format, and M6 changed the type of two of its
    /// fields. Every line M3–M5 wrote holds a **bare oid string** there, so
    /// the untagged `Merged` arm has to read one, `null` has to stay `None`,
    /// and the new `Unmerged` shape has to round-trip beside them.
    #[test]
    fn index_state_serde_is_backward_compatible() {
        #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
        struct Holder {
            index_before: Option<IndexState>,
        }
        let id = Oid::from_hex("acf15d58a3b48e5878fe281a624fb5c688dcaee1").expect("a hex oid");

        // A bare oid — what every pre-M6 line holds — is a merged index.
        let old: Holder =
            serde_json::from_str(r#"{"index_before":"acf15d58a3b48e5878fe281a624fb5c688dcaee1"}"#)
                .expect("a pre-M6 line parses");
        assert_eq!(old.index_before, Some(IndexState::Merged(id)));
        // And renders back byte-identically: no field renamed, no map where
        // a string was.
        assert_eq!(
            serde_json::to_string(&old).expect("it renders"),
            r#"{"index_before":"acf15d58a3b48e5878fe281a624fb5c688dcaee1"}"#
        );

        // `null` is still "never recorded", not "unmerged".
        let absent: Holder =
            serde_json::from_str(r#"{"index_before":null}"#).expect("a null line parses");
        assert_eq!(absent.index_before, None);

        // The new shape round-trips.
        let unmerged = Holder {
            index_before: Some(IndexState::Unmerged { unmerged: id }),
        };
        let text = serde_json::to_string(&unmerged).expect("it renders");
        assert_eq!(
            text,
            r#"{"index_before":{"unmerged":"acf15d58a3b48e5878fe281a624fb5c688dcaee1"}}"#
        );
        assert_eq!(
            serde_json::from_str::<Holder>(&text).expect("it parses back"),
            unmerged
        );
    }

    /// A line as every entry before m6-conflicts/72 wrote it: no `put_back`.
    const OLD_LINE: &str = r#"{"id":"01J8Z3K4M5N6P7Q8R9S0T1V2W3","at_unix_ms":1,"kind":"stash_pop","description":"pop stash@{0}","status":"committed","touches":{"refs":["refs/stash"],"index":true,"worktree":true},"head_before":{"branch":"refs/heads/main"},"head_after":null,"refs_before":{},"refs_after":{},"index_before":null,"index_after":null,"worktree_before":null,"worktree_after":null,"backup_ref":null,"pins":[],"commands":[],"reverses":null}"#;

    /// ADR-0045 §1: the field is additive. An old line reads as `None` and
    /// renders back byte for byte, so nothing already on disk changes.
    #[test]
    fn a_journal_line_without_put_back_re_renders_byte_identically() {
        let entry: JournalEntry = serde_json::from_str(OLD_LINE).expect("an old line parses");
        assert_eq!(entry.put_back, None);
        assert_eq!(serde_json::to_string(&entry).expect("it renders"), OLD_LINE);
    }

    /// ADR-0047: a rewrite is a kind of its own on disk, `"rewrite"`, and a
    /// line written with it reads back to the same entry. Old lines never
    /// carry it, so `a_journal_line_without_put_back_re_renders_byte_identically`
    /// still holds for them.
    #[test]
    fn a_rewrite_entry_round_trips() {
        let mut entry: JournalEntry = serde_json::from_str(OLD_LINE).expect("an old line parses");
        entry.kind = OperationKind::Rewrite;
        let text = serde_json::to_string(&entry).expect("it renders");
        assert!(text.contains(r#""kind":"rewrite""#), "{text}");
        assert_eq!(
            serde_json::from_str::<JournalEntry>(&text).expect("it parses back"),
            entry
        );
    }

    /// ADR-0046 amendment (hard choice 5): a put-back from the journal's
    /// list is a kind of its own on disk, `"recover"`, written for good.
    /// Old lines never carry it and read as before.
    #[test]
    fn a_recover_entry_round_trips() {
        let mut entry: JournalEntry = serde_json::from_str(OLD_LINE).expect("an old line parses");
        entry.kind = OperationKind::Recover;
        let text = serde_json::to_string(&entry).expect("it renders");
        assert!(text.contains(r#""kind":"recover""#), "{text}");
        assert_eq!(
            serde_json::from_str::<JournalEntry>(&text).expect("it parses back"),
            entry
        );
    }

    #[test]
    fn put_back_round_trips() {
        let mut entry: JournalEntry = serde_json::from_str(OLD_LINE).expect("an old line parses");
        let stash = Oid::from_hex("acf15d58a3b48e5878fe281a624fb5c688dcaee1").expect("a hex oid");
        for why in [PutBackWhy::Pop, PutBackWhy::Apply, PutBackWhy::SetAside] {
            entry.put_back = Some(PutBack { stash, why });
            let text = serde_json::to_string(&entry).expect("it renders");
            assert_eq!(
                serde_json::from_str::<JournalEntry>(&text).expect("it parses back"),
                entry
            );
        }
        entry.put_back = Some(PutBack {
            stash,
            why: PutBackWhy::SetAside,
        });
        let text = serde_json::to_string(&entry).expect("it renders");
        assert!(
            text.ends_with(
                r#","put_back":{"stash":"acf15d58a3b48e5878fe281a624fb5c688dcaee1","why":"set_aside"}}"#
            ),
            "{text}"
        );
    }
}
