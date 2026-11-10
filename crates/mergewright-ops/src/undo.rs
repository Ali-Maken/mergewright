//! Property 4: the inverse, computed from a journal entry (DESIGN.md §6.4).
//!
//! Undo is not "run the reverse command": it restores the entry's recorded
//! before-state. The generic part is the same for every kind — refs back by
//! compare-and-swap, `HEAD` back, the index and working tree back from the
//! backup trees, scoped to what the operation changed — and the kind adds
//! only what state alone cannot express: `refs/stash`'s reflog *is* the
//! stash list, so a push, pop or drop is put back by a reflog step, never by
//! moving the ref. Redo is the inverse of an `Undo` entry, from that entry's
//! own before/after (§6.5): one path serves both directions.
//!
//! A plan is data; `ops-execution` runs the steps. Nothing in this file
//! spawns `git`: `execute` and `replaced` are the halves that read.

mod execute;
mod replaced;

pub(crate) use replaced::tree_diff;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use mergewright_core::journal::{
    EntryId, EntryStatus, HeadRef, IndexState, JournalEntry, MergeState, OperationKind, PutBack,
    PutBackWhy, Touches,
};
use mergewright_core::oid::Oid;
use mergewright_git::state::InProgress;

use crate::journal::{in_force_all, is_operation, next_redo, next_undo, root_operation, undoable};

pub use execute::{Direction, UndoError, redo, revert_to, undo};

/// The stash ref, whose reflog is the stash list; restored by
/// [`Step::StashStore`]/[`Step::StashDrop`], never by [`Step::UpdateRef`].
const STASH_REF: &str = "refs/stash";

/// One thing the executor does to put the repository back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// `git update-ref <name> <to> <expect>`: compare-and-swap, so a ref
    /// something else moved fails loudly instead of being clobbered. `None`
    /// on either side is "absent" (git's all-zero id).
    UpdateRef {
        /// Full ref name.
        name: String,
        /// What the ref must be now.
        expect: Option<Oid>,
        /// What it goes back to.
        to: Option<Oid>,
    },
    /// Point `HEAD` back: `symbolic-ref` for a branch (born or not),
    /// `update-ref --no-deref` when detached.
    SetHead(HeadRef),
    /// Put `paths` back as `index` recorded them, whatever shape it has: a
    /// merged path goes back at stage 0, a conflicted one gets its stages 1,
    /// 2 and 3 again. Always `update-index --index-info`, never `read-tree`.
    RestoreIndex {
        /// The recorded index state.
        index: IndexState,
        /// Which paths.
        paths: IndexPaths,
    },
    /// Write `paths` of `tree` into the working tree: a path in the tree is
    /// written, a path only in the scope's other side is removed. Then each
    /// of `paths` that `raw` holds gets his own bytes back (ADR-0052).
    RestoreWorktree {
        /// The recorded working-tree tree (tracked content and untracked
        /// files, §6.3).
        tree: Oid,
        /// Which paths.
        paths: PathScope,
        /// The same snapshot's `raw/` tree: the paths whose bytes git's
        /// checkout of `tree` would not give back, as they were on disk.
        /// Always from the entry whose before-snapshot `tree` is, never
        /// another's. `None`: nothing kept, or an entry older than 30q.
        raw: Option<Oid>,
    },
    /// Make `refs/stash`'s reflog contain `commit` again: `git stash store
    /// -m <the commit's subject>` (the subject is the message git wrote —
    /// verified byte for byte). Idempotent by the executor's doing, not
    /// git's: it scans the reflog first and stores only when `commit` is
    /// absent, so a pop that failed before dropping is not listed twice
    /// (`stash store` of a commit already at `stash@{1}` would add a
    /// duplicate on top — verified).
    StashStore {
        /// The stash commit, kept alive by the entry's backup ref or `pins`.
        commit: Oid,
    },
    /// Remove the reflog entry whose commit is `commit` (`git stash drop
    /// stash@{n}` at the index the executor finds it at); a no-op when the
    /// reflog does not have it. The plan's `pins` keep the commit alive.
    StashDrop {
        /// The stash commit.
        commit: Oid,
    },
    /// Make git's merge state this (m6-conflicts/20): `Some` writes
    /// `MERGE_MSG`, `MERGE_MODE` and `MERGE_HEAD` back — `u` of a finished
    /// merge, which is in progress again afterwards — and `None` removes
    /// them, which is its redo. One step both ways, like [`Step::SetHead`]:
    /// a roll-back of either direction is the same step aimed at the entry's
    /// own `merge_before`. Always the plan's **last** step, after the
    /// working tree, and checked afterwards with git's own `rev-parse -q
    /// --verify MERGE_HEAD`.
    SetMergeState(Option<MergeState>),
    /// Make one `branch.<name>.*` key of the local config hold exactly `to`
    /// (none: unset). Emitted for each key whose recorded before and after
    /// differ, after the refs: `u` of a tracking checkout removes the
    /// upstream it wrote and puts back one that was there before (review
    /// m7-16b..21b, Major 1).
    SetBranchConfig {
        /// The key, as the config spells it: `branch.sara/x.remote`.
        key: String,
        /// Its values afterwards, in order.
        to: Vec<String>,
    },
}

/// Which paths a restore touches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathScope {
    /// Only the paths that differ between the two trees: what the operation
    /// changed, nothing the user touched since.
    ChangedBetween {
        /// The recorded after-tree.
        from: Oid,
        /// The recorded before-tree, which the restore reads.
        to: Oid,
    },
    /// The whole tree: the after-state was never recorded (an operation that
    /// died before it could be read), so nothing narrower is known. The
    /// executor's own pre-undo snapshot (ADR-0009) keeps this safe.
    Everything,
}

/// Which paths an index restore touches.
///
/// Separate from [`PathScope`] because an index state is not a plain tree: a
/// stage tree's paths live one component down, and two states of different
/// shapes cannot be diffed against each other at all
/// ([`crate::snapshot::index_paths_changed`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndexPaths {
    /// Only the paths whose index entries differ between the two states:
    /// what the operation changed, nothing the user touched since.
    ChangedBetween {
        /// The recorded after-state.
        from: IndexState,
        /// The recorded before-state, which the restore reads.
        to: IndexState,
    },
    /// Exactly these paths and no others: what `d` restarting one conflicted
    /// file asks for (m6-conflicts/08).
    Exactly(Vec<PathBuf>),
    /// Every path either the recorded state or the index now holds: the
    /// after-state was never recorded, so nothing narrower is known.
    Everything,
}

/// What undoing (or redoing) an entry takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InversePlan {
    /// The entry this plan reverses: the new entry's `reverses`.
    pub reverses: EntryId,
    /// `Undo` or `Redo`: the new entry's kind.
    pub kind: OperationKind,
    /// `undo: <original description>` / `redo: …`.
    pub description: String,
    /// The union of what the steps touch: the new entry's snapshot scope.
    pub touches: Touches,
    /// In order: refs, `HEAD`, index, working tree, the stash reflog, then
    /// git's merge state.
    pub steps: Vec<Step>,
    /// Commits the steps make unreachable — every [`Step::StashDrop`]'s —
    /// which the new entry's own inverse will need alive: its `pins`, so the
    /// executor's backup ref points at them before the plan runs. Without
    /// this a redo after `gc.pruneExpire` would store a commit git has
    /// pruned.
    pub pins: Vec<Oid>,
}

/// Why an entry has no inverse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotInvertible {
    /// The operation may still be running, or died before recording an
    /// outcome; the app surfaces it instead.
    Pending,
    /// `Aborted` with no backup: nothing changed, nothing to put back.
    NoBackup,
    /// An entry `u` never reverses: a `Fetch` or `Push`, or a `Pull` that
    /// moved nothing (`journal::undoable`).
    NotUndoable(OperationKind),
    /// No in-force operation to undo.
    NothingToUndo,
    /// No in-force undo to redo.
    NothingToRedo,
    /// The entry lacks a recorded field the inverse needs; names it.
    Incomplete {
        /// Which field.
        field: &'static str,
    },
    /// git is part-way through an operation, and the entry was not recorded
    /// inside that same merge: an undo now would be a second history write
    /// over an unfinished first (keymap §2.6a). Refused by the executor
    /// before the drift rail, so nothing is written and no box asks
    /// (m6-conflicts/57).
    OperationLive {
        /// What git has in progress.
        kind: InProgress,
    },
}

/// [`NotInvertible::OperationLive`]'s words, `finish or abort the <kind>
/// first`: the one source for the executor's `explain` and the preview's
/// refusal (m6-conflicts/64, review m2). The app words its own refusal to
/// match (`mergewright`'s `operation_live_words_match_ops`).
#[must_use]
pub fn operation_live_words(kind: InProgress) -> String {
    format!("finish or abort the {} first", kind.name())
}

/// The inverse of `entry`. For an `Undo` or `Redo` entry, `original` is the
/// operation at the root of its `reverses` chain
/// ([`crate::journal::root_operation`]); an operation entry ignores it.
///
/// # Errors
/// [`NotInvertible`]: a `Pending` entry, an `Aborted` one without a backup,
/// or one missing a field the steps need.
pub fn inverse(
    entry: &JournalEntry,
    original: Option<&JournalEntry>,
) -> Result<InversePlan, NotInvertible> {
    // A fetch or push is journaled but never reversed from here: its only
    // local trace is a tracking ref, and moving that back would lie about the
    // remote (M4 design note, hard choice 1). A pull that moved nothing has
    // nothing to reverse, and is passed over the same way.
    if !crate::journal::undoable(entry) {
        return Err(NotInvertible::NotUndoable(entry.kind));
    }
    match &entry.status {
        EntryStatus::Pending => return Err(NotInvertible::Pending),
        EntryStatus::Aborted { .. } if entry.worktree_before.is_none() => {
            return Err(NotInvertible::NoBackup);
        }
        EntryStatus::Aborted { .. } | EntryStatus::Committed => {}
    }
    let (kind, root, forward) = match entry.kind {
        OperationKind::Undo => (OperationKind::Redo, original, true),
        OperationKind::Redo => (OperationKind::Undo, original, false),
        _ => (OperationKind::Undo, Some(entry), false),
    };
    let root = root.ok_or(NotInvertible::Incomplete {
        field: "reverses (no root operation)",
    })?;
    let mut steps = generic_steps(entry)?;
    steps.extend(stash_step(entry, root, forward)?);
    // Last, after the working tree: with the branch, the index and the files
    // back, the merge files turn the resolved tree into a merge in progress
    // again. Only where the entry changed git's merge state — an entry older
    // than the two fields reads `None` on both sides and gets no step.
    if entry.merge_before != entry.merge_after {
        steps.push(Step::SetMergeState(entry.merge_before.clone()));
    }
    let verb = if kind == OperationKind::Undo {
        "undo"
    } else {
        "redo"
    };
    // Redoing an undo that failed half way puts back the state the failed
    // undo started from, which is the root's after-state; the browser says
    // so, since "redo" alone reads as if the undo had finished.
    let after_failure = match entry.status {
        EntryStatus::Aborted { .. } if entry.kind == OperationKind::Undo => {
            " (after a failed undo)"
        }
        _ => "",
    };
    Ok(InversePlan {
        reverses: entry.id.clone(),
        kind,
        description: format!("{verb}: {}{after_failure}", root.description),
        touches: touches_of(&steps),
        pins: dropped_by(&steps),
        steps,
    })
}

/// The commits the plan's `StashDrop` steps leave unreachable.
fn dropped_by(steps: &[Step]) -> Vec<Oid> {
    steps
        .iter()
        .filter_map(|step| match step {
            Step::StashDrop { commit } => Some(*commit),
            _ => None,
        })
        .collect()
}

/// Refs back, `HEAD` back, trees back — from the entry's own before/after.
fn generic_steps(entry: &JournalEntry) -> Result<Vec<Step>, NotInvertible> {
    let mut steps = Vec::new();
    for (name, before) in &entry.refs_before {
        if name == STASH_REF {
            continue;
        }
        let expect = entry
            .refs_after
            .get(name)
            .copied()
            .ok_or(NotInvertible::Incomplete {
                field: "refs_after",
            })?;
        if expect != *before {
            steps.push(Step::UpdateRef {
                name: name.clone(),
                expect,
                to: *before,
            });
        }
    }
    steps.extend(config_steps(entry)?);
    if entry.head_after.as_ref() != Some(&entry.head_before) {
        steps.push(Step::SetHead(entry.head_before.clone()));
    }
    if entry.touches.index {
        // An unmerged before-index used to land here as `NoBackup`: it had no
        // tree, so `index_before` was `None` and every older entry became
        // unreachable behind it, undo being linear (m6-conflicts/08, the
        // defect this group exists for). It has a state of its own now.
        let index = entry.index_before.ok_or(NotInvertible::NoBackup)?;
        steps.push(Step::RestoreIndex {
            index,
            paths: match entry.index_after {
                Some(from) => IndexPaths::ChangedBetween { from, to: index },
                None => IndexPaths::Everything,
            },
        });
    }
    if entry.touches.worktree {
        let tree = entry.worktree_before.ok_or(NotInvertible::NoBackup)?;
        steps.push(Step::RestoreWorktree {
            tree,
            paths: scope(entry.worktree_after, tree),
            // The same entry's before-snapshot as `tree` (R3: only a
            // before-snapshot keeps `raw/`).
            raw: entry.worktree_raw_before,
        });
    }
    Ok(steps)
}

/// One [`Step::SetBranchConfig`] per `branch.<name>.*` key whose recorded
/// values differ before and after. A key missing from one side was not set
/// on that side.
fn config_steps(entry: &JournalEntry) -> Result<Vec<Step>, NotInvertible> {
    let unset = Vec::new();
    let keys: BTreeSet<&String> = entry
        .config_before
        .keys()
        .chain(entry.config_after.keys())
        .collect();
    let mut steps = Vec::new();
    for key in keys {
        // Only a branch's key is ever recorded; anything else is a line
        // edited by hand, and writing it back would reach past the branch.
        if crate::branch_config::branch_of(key).is_none() {
            return Err(NotInvertible::Incomplete {
                field: "config_before",
            });
        }
        let before = entry.config_before.get(key).unwrap_or(&unset);
        if entry.config_after.get(key).unwrap_or(&unset) != before {
            steps.push(Step::SetBranchConfig {
                key: key.clone(),
                to: before.clone(),
            });
        }
    }
    Ok(steps)
}

fn scope(after: Option<Oid>, before: Oid) -> PathScope {
    match after {
        Some(from) => PathScope::ChangedBetween { from, to: before },
        None => PathScope::Everything,
    }
}

/// The reflog step for the root operation's kind: its inverse when undoing,
/// its own effect again when redoing. `entry` is the entry being inverted —
/// when redoing, the `Undo` entry. A push that pushed nothing (`git stash
/// push` with nothing to save exits 0, so the entry is `Committed` with
/// `refs/stash` unmoved) has no reflog step either way: the commit at
/// `refs/stash` is the user's own earlier stash, and dropping it would
/// destroy something the operation never touched.
fn stash_step(
    entry: &JournalEntry,
    root: &JournalEntry,
    forward: bool,
) -> Result<Option<Step>, NotInvertible> {
    let pinned = || {
        root.pins
            .first()
            .copied()
            .ok_or(NotInvertible::Incomplete { field: "pins" })
    };
    // A rebase or checkout that set edits aside (ADR-0045 §6). A clean
    // put-back dropped `S`, so the root's `refs/stash` did not move and
    // there is nothing to do either way. A clash kept `S` in the list: the
    // undo drops it, since the backup puts the very same edits back, and a
    // redo stores it again only if that undo's own `refs/stash` moved (the
    // pop rule, m6-conflicts/07 F2).
    if let Some(PutBack {
        stash,
        why: PutBackWhy::SetAside,
    }) = root.put_back
    {
        return Ok(if forward {
            stash_ref_moved(entry).then_some(Step::StashStore { commit: stash })
        } else {
            stash_moved_to(root)?
                .filter(|kept| *kept == stash)
                .map(|commit| Step::StashDrop { commit })
        });
    }
    Ok(match (root.kind, forward) {
        (OperationKind::StashPush, false) => {
            stash_moved_to(root)?.map(|commit| Step::StashDrop { commit })
        }
        (OperationKind::StashPush, true) => {
            stash_moved_to(root)?.map(|commit| Step::StashStore { commit })
        }
        // `stash branch` applies the stash and drops it, so its inverse
        // stores the commit back exactly as a pop's does; the branch it
        // created is removed by the generic `UpdateRef` (m5/07).
        (
            OperationKind::StashPop | OperationKind::StashDrop | OperationKind::BranchFromStash,
            false,
        ) => Some(Step::StashStore { commit: pinned()? }),
        // The redo drops only what its undo stored back: a pop git refused
        // on a conflict kept its stash, its undo's store was a no-op, and
        // there is nothing to drop (m6-conflicts/07, F2). The undo's own
        // `refs/stash` says which; the root's cannot, since popping
        // `stash@{1}` leaves it unmoved whether git dropped it or not.
        (
            OperationKind::StashPop | OperationKind::StashDrop | OperationKind::BranchFromStash,
            true,
        ) => {
            let commit = pinned()?;
            // `refs/stash` moved to some other commit means something else
            // wrote the stash list while the undo ran. Dropping nothing is
            // the side that loses nothing: at worst the stash stays listed.
            stash_moved_to(entry)?
                .filter(|stored| *stored == commit)
                .map(|commit| Step::StashDrop { commit })
        }
        (OperationKind::Undo | OperationKind::Redo, _) => {
            return Err(NotInvertible::Incomplete {
                field: "reverses (root is not an operation)",
            });
        }
        _ => None,
    })
}

/// Where `entry` moved `refs/stash` to: its after-value, when that differs
/// from its before-value. For a push, the stash commit it created; for an
/// undo, the commit its `StashStore` put on top. Both sides must be recorded
/// — an unknown before-value cannot tell a new stash from the user's old one,
/// and an unknown after-value cannot tell a store from a no-op.
fn stash_moved_to(entry: &JournalEntry) -> Result<Option<Oid>, NotInvertible> {
    let before = entry
        .refs_before
        .get(STASH_REF)
        .copied()
        .ok_or(NotInvertible::Incomplete {
            field: "refs_before[refs/stash]",
        })?;
    let after = entry
        .refs_after
        .get(STASH_REF)
        .copied()
        .ok_or(NotInvertible::Incomplete {
            field: "refs_after[refs/stash]",
        })?;
    Ok(after.filter(|_| after != before))
}

/// Whether `entry` recorded `refs/stash` on both sides and it moved. An
/// entry whose plan had no stash step never recorded it, and moved nothing.
/// Unlike [`stash_moved_to`], a move to no stash at all counts: an undo that
/// dropped the only stash leaves `refs/stash` gone.
fn stash_ref_moved(entry: &JournalEntry) -> bool {
    match (
        entry.refs_before.get(STASH_REF),
        entry.refs_after.get(STASH_REF),
    ) {
        (Some(before), Some(after)) => before != after,
        _ => false,
    }
}

fn touches_of(steps: &[Step]) -> Touches {
    let mut touches = Touches::default();
    for step in steps {
        match step {
            Step::UpdateRef { name, .. } => touches.refs.push(name.clone()),
            // `HEAD` is deliberately **not** a touched ref, for the same
            // reason a checkout does not record one
            // (`history::touches_checkout`): a touched ref is snapshotted
            // into `refs_before`/`refs_after`, and inverting that snapshot
            // emits `update-ref HEAD <old> <new>`, which moves the branch
            // `HEAD` points at rather than `HEAD` itself. The move is
            // recorded by the entry's `head_before`/`head_after` and put
            // back by `SetHead`, which is symbolic-aware.
            //
            // git's merge files are no ref, index or tree either; every
            // entry records them before and after, whatever it touches.
            Step::SetHead(_) | Step::SetMergeState(_) => {}
            // The branch's ref is named so the new entry records this key's
            // section before and after: that is what its own inverse (the
            // redo) writes back from.
            Step::SetBranchConfig { key, .. } => {
                if let Some(branch) = crate::branch_config::branch_of(key)
                    && !touches.refs.contains(&branch)
                {
                    touches.refs.push(branch);
                }
            }
            Step::RestoreIndex { .. } => touches.index = true,
            Step::RestoreWorktree { .. } => touches.worktree = true,
            Step::StashStore { .. } | Step::StashDrop { .. } => {
                touches.refs.push(STASH_REF.to_owned());
            }
        }
    }
    touches.refs.dedup();
    touches
}

/// What `u` does: the inverse of [`next_undo`]'s entry.
///
/// # Errors
/// [`NotInvertible::NothingToUndo`], or the entry's own reason.
pub fn plan_undo(entries: &[JournalEntry]) -> Result<InversePlan, NotInvertible> {
    let entry = next_undo(entries).ok_or(NotInvertible::NothingToUndo)?;
    inverse(entry, root_operation(entries, entry))
}

/// What `U` does: the inverse of [`next_redo`]'s `Undo` entry.
///
/// # Errors
/// [`NotInvertible::NothingToRedo`], or the entry's own reason.
pub fn plan_redo(entries: &[JournalEntry]) -> Result<InversePlan, NotInvertible> {
    let entry = next_redo(entries).ok_or(NotInvertible::NothingToRedo)?;
    inverse(entry, root_operation(entries, entry))
}

/// The time machine's plans: every in-force entry from the newest back to
/// `target` inclusive, each inverted, newest first (§6.6).
///
/// Reverting *to* an entry means reverting to just before it ran, so the
/// target itself is reversed along with everything after it. The plans are
/// computed against the journal as it stands, and running them in this order
/// keeps every compare-and-swap true: each step's `expect` is the value the
/// step before it just restored.
///
/// An entry `u` cannot see is skipped, by exactly the rules `u` uses. An
/// `Undo` or `Redo` entry is stepped over (`journal::is_operation`) — inverting one
/// re-applies what the user undid, so a journal of `[pick, branch,
/// undo(branch)]` reverted to the pick would have handed the branch back and
/// written a `Redo` line nobody asked for (m5/07 review, C2). The entry it
/// reversed is not in force, so it is skipped too, and every remaining
/// compare-and-swap still sees the value the step before it restored.
///
/// The second rule is [`undoable`]: a fetch or a push has no local inverse, and a pull that
/// moved nothing has nothing to put back. A push inside the range therefore
/// leaves the remote where it is — ADR-0037 §3's three-choice box is the
/// app's to raise before calling this, never something to infer from a plan
/// that is silent about it.
///
/// # Errors
/// [`NotInvertible::NothingToUndo`] when `target` is not in the journal or
/// nothing in the range is in force; otherwise the first entry's own reason
/// — a `Pending` entry in the range stops the whole revert rather than
/// leaving a half-finished operation in the middle of it.
pub fn plan_revert_to(
    entries: &[JournalEntry],
    target: &EntryId,
) -> Result<Vec<InversePlan>, NotInvertible> {
    let at = entries
        .iter()
        .position(|entry| entry.id == *target)
        .ok_or(NotInvertible::NothingToUndo)?;
    let flags = in_force_all(entries);
    let mut plans = Vec::new();
    for index in (at..entries.len()).rev() {
        let entry = &entries[index];
        if !flags[index] || !is_operation(entry) || !undoable(entry) {
            continue;
        }
        plans.push(inverse(entry, root_operation(entries, entry))?);
    }
    if plans.is_empty() {
        return Err(NotInvertible::NothingToUndo);
    }
    Ok(plans)
}

/// The repository as it is right now, read fresh by the executor for the
/// pre-undo comparison (§6.7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoStateNow {
    /// `HEAD` now.
    pub head: HeadRef,
    /// The refs the entry recorded, read again; a missing name is absent.
    pub refs: BTreeMap<String, Option<Oid>>,
    /// The index now, merged or unmerged. Never absent and never an error:
    /// an unmerged index is a state, and comparing it against a recorded one
    /// is what tells an undone conflict apart from a drifted one.
    pub index: IndexState,
    /// The working-tree tree now (§6.3's hashing).
    pub worktree_tree: Oid,
    /// The merge git has in progress now, `None` when there is none
    /// (m6-conflicts/20).
    pub merge: Option<MergeState>,
}

/// Which way git's merge state moved since an entry recorded it
/// ([`Drift::merge`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeDrift {
    /// No merge was recorded; one is in progress now.
    Begun,
    /// A merge was recorded; none is in progress now.
    Ended,
    /// A merge was recorded and one is in progress, but not that one: its
    /// commit, message or mode differ.
    Changed,
}

impl MergeDrift {
    /// How `recorded` became `now`, or `None` when they are equal.
    fn between(recorded: Option<&MergeState>, now: Option<&MergeState>) -> Option<Self> {
        match (recorded, now) {
            (None, None) => None,
            (None, Some(_)) => Some(MergeDrift::Begun),
            (Some(_), None) => Some(MergeDrift::Ended),
            (Some(recorded), Some(now)) => (recorded != now).then_some(MergeDrift::Changed),
        }
    }
}

/// What changed since the entry's after-state was recorded: the three
/// comparisons of §6.7, refs never alone — and what could not be compared
/// because the entry never recorded it.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Drift {
    /// Refs whose value now differs from `refs_after`: name, recorded, now.
    pub refs: Vec<(String, Option<Oid>, Option<Oid>)>,
    /// `HEAD` moved.
    pub head: bool,
    /// The index differs from what the entry recorded: what it recorded, and
    /// what it is now. A merged index that is unmerged now is drift like any
    /// other — and says so in different words.
    pub index: Option<(IndexState, IndexState)>,
    /// The working-tree tree differs.
    pub worktree: bool,
    /// git's merge state differs from the entry's `merge_after` — say a new
    /// `git merge` was begun in the shell after `c` finished the last one
    /// (m6-conflicts/20). Which way it went; the states themselves are the
    /// entry's and the repository's.
    pub merge: Option<MergeDrift>,
    /// What the inverse restores whole because its after-state was never
    /// recorded (`PathScope::Everything`): `"HEAD"`, `"the index"`, `"the
    /// working tree"`. Unknown is not clean: the user is told the whole
    /// thing is about to be overwritten, and asked (§6.7).
    pub unknown: Vec<&'static str>,
    /// What running the inverse anyway would replace (m6-conflicts/77).
    /// [`drift`] leaves it empty — it compares tree ids and names no path —
    /// and the executor fills it, with a read of its own, only when the
    /// drift is not clean. Not part of [`Drift::is_clean`]: it says what the
    /// drift costs, not whether there is any.
    pub replaced: Replaced,
}

/// What running an inverse anyway replaces that the journal never recorded
/// (m6-conflicts/77): the user's own edits since the operation, on paths the
/// inverse writes. The undo's own backup (ADR-0009) keeps every one of them,
/// and its redo puts them back.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Replaced {
    /// Paths the inverse writes whose content now differs both from the
    /// entry's recorded after-state (the user changed them since) and from
    /// the before-state the inverse writes (so the write loses something);
    /// in the working tree or the index. Sorted, unique.
    pub paths: Vec<PathBuf>,
    /// No after-state was recorded for the working tree
    /// ([`PathScope::Everything`]; `Drift::unknown` names it): the inverse
    /// restores every file, so no narrower list is known.
    pub whole_tree: bool,
    /// The same for the index ([`IndexPaths::Everything`]): every index
    /// entry is restored. Apart from `whole_tree` because an index-only
    /// restore writes no file, and must not be worded as if it did
    /// (m6-conflicts/80, review m8).
    pub whole_index: bool,
}

impl Replaced {
    /// Nothing the user made is replaced.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty() && !self.whole_tree && !self.whole_index
    }

    /// The union with `other`: what the time machine's steps replace
    /// between them.
    pub fn extend(&mut self, other: Replaced) {
        self.whole_tree |= other.whole_tree;
        self.whole_index |= other.whole_index;
        self.paths.extend(other.paths);
        self.paths.sort();
        self.paths.dedup();
    }
}

impl Drift {
    /// Nothing moved since the entry, and every restore is scoped to what
    /// the operation changed.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.refs.is_empty()
            && !self.head
            && self.index.is_none()
            && !self.worktree
            && self.merge.is_none()
            && self.unknown.is_empty()
    }

    /// `refs/heads/main moved; the index changed; the working tree changed`,
    /// or `the working tree's state after the operation was never recorded;
    /// undo restores all of it`.
    #[must_use]
    pub fn explain(&self) -> String {
        let mut parts: Vec<String> = self
            .refs
            .iter()
            .map(|(name, _, _)| format!("{name} moved"))
            .collect();
        if self.head {
            parts.push("HEAD moved".to_owned());
        }
        if let Some((recorded, now)) = self.index {
            parts.push(
                match (recorded.is_unmerged(), now.is_unmerged()) {
                    (false, true) => "the index is unmerged now",
                    (true, false) => "the index is merged now",
                    _ => "the index changed",
                }
                .to_owned(),
            );
        }
        if self.worktree {
            parts.push("the working tree changed".to_owned());
        }
        if let Some(merge) = self.merge {
            parts.push(
                match merge {
                    MergeDrift::Begun => "a merge is in progress now",
                    MergeDrift::Ended => "the merge is no longer in progress",
                    MergeDrift::Changed => "the merge in progress changed",
                }
                .to_owned(),
            );
        }
        for what in &self.unknown {
            parts.push(format!(
                "{what}'s state after the operation was never recorded; undo restores all of it"
            ));
        }
        if parts.is_empty() {
            "nothing changed since".to_owned()
        } else {
            parts.join("; ")
        }
    }
}

/// Compares the entry's recorded after-state with `now`. A field the entry
/// never recorded is reported as `unknown` rather than compared: the inverse
/// restores it whole (`PathScope::Everything`), so the rail must speak
/// loudest exactly there.
#[must_use]
pub fn drift(entry: &JournalEntry, now: &RepoStateNow) -> Drift {
    let refs = entry
        .refs_after
        .iter()
        .filter_map(|(name, recorded)| {
            let current = now.refs.get(name).copied().flatten();
            (current != *recorded).then(|| (name.clone(), *recorded, current))
        })
        .collect();
    let mut unknown = Vec::new();
    if entry.head_after.is_none() {
        unknown.push("HEAD");
    }
    if entry.touches.index && entry.index_after.is_none() {
        unknown.push("the index");
    }
    if entry.touches.worktree && entry.worktree_after.is_none() {
        unknown.push("the working tree");
    }
    Drift {
        refs,
        head: entry
            .head_after
            .as_ref()
            .is_some_and(|head| *head != now.head),
        index: entry
            .index_after
            .filter(|recorded| *recorded != now.index)
            .map(|recorded| (recorded, now.index)),
        worktree: entry
            .worktree_after
            .is_some_and(|tree| tree != now.worktree_tree),
        merge: MergeDrift::between(entry.merge_after.as_ref(), now.merge.as_ref()),
        unknown,
        replaced: Replaced::default(),
    }
}

#[cfg(test)]
mod tests;
