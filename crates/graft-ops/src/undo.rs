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
//! A plan is data; `ops-execution` runs the steps. Nothing here spawns `git`.

mod execute;

use std::collections::BTreeMap;

use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches};
use graft_core::oid::Oid;

use crate::journal::{next_redo, next_undo, root_operation};

pub use execute::{UndoError, redo, undo};

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
    /// Read `paths` of `tree` into the index.
    RestoreIndex {
        /// The recorded index tree.
        tree: Oid,
        /// Which paths.
        paths: PathScope,
    },
    /// Write `paths` of `tree` into the working tree: a path in the tree is
    /// written, a path only in the scope's other side is removed.
    RestoreWorktree {
        /// The recorded working-tree tree (tracked content and untracked
        /// files, §6.3).
        tree: Oid,
        /// Which paths.
        paths: PathScope,
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
    /// In order: refs, `HEAD`, index, working tree, then the stash reflog.
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
    /// A kind `u` never reverses: `Fetch` and `Push` (`journal::undoable_kind`).
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
    // remote (M4 design note, hard choice 1).
    if !crate::journal::undoable_kind(entry.kind) {
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
    steps.extend(stash_step(root, forward)?);
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
    if entry.head_after.as_ref() != Some(&entry.head_before) {
        steps.push(Step::SetHead(entry.head_before.clone()));
    }
    if entry.touches.index {
        let tree = entry.index_before.ok_or(NotInvertible::NoBackup)?;
        steps.push(Step::RestoreIndex {
            tree,
            paths: scope(entry.index_after, tree),
        });
    }
    if entry.touches.worktree {
        let tree = entry.worktree_before.ok_or(NotInvertible::NoBackup)?;
        steps.push(Step::RestoreWorktree {
            tree,
            paths: scope(entry.worktree_after, tree),
        });
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
/// its own effect again when redoing. A push that pushed nothing (`git stash
/// push` with nothing to save exits 0, so the entry is `Committed` with
/// `refs/stash` unmoved) has no reflog step either way: the commit at
/// `refs/stash` is the user's own earlier stash, and dropping it would
/// destroy something the operation never touched.
fn stash_step(root: &JournalEntry, forward: bool) -> Result<Option<Step>, NotInvertible> {
    let pinned = || {
        root.pins
            .first()
            .copied()
            .ok_or(NotInvertible::Incomplete { field: "pins" })
    };
    Ok(match (root.kind, forward) {
        (OperationKind::StashPush, false) => pushed(root)?.map(|commit| Step::StashDrop { commit }),
        (OperationKind::StashPush, true) => pushed(root)?.map(|commit| Step::StashStore { commit }),
        (OperationKind::StashPop | OperationKind::StashDrop, false) => {
            Some(Step::StashStore { commit: pinned()? })
        }
        (OperationKind::StashPop | OperationKind::StashDrop, true) => {
            Some(Step::StashDrop { commit: pinned()? })
        }
        (OperationKind::Undo | OperationKind::Redo, _) => {
            return Err(NotInvertible::Incomplete {
                field: "reverses (root is not an operation)",
            });
        }
        _ => None,
    })
}

/// The stash commit a push created: `refs/stash` after, when it differs from
/// `refs/stash` before. Both sides must be recorded — an unknown before-value
/// cannot tell a new stash from the user's old one.
fn pushed(root: &JournalEntry) -> Result<Option<Oid>, NotInvertible> {
    let before = root
        .refs_before
        .get(STASH_REF)
        .copied()
        .ok_or(NotInvertible::Incomplete {
            field: "refs_before[refs/stash]",
        })?;
    let after = root
        .refs_after
        .get(STASH_REF)
        .copied()
        .ok_or(NotInvertible::Incomplete {
            field: "refs_after[refs/stash]",
        })?;
    Ok(after.filter(|_| after != before))
}

fn touches_of(steps: &[Step]) -> Touches {
    let mut touches = Touches::default();
    for step in steps {
        match step {
            Step::UpdateRef { name, .. } => touches.refs.push(name.clone()),
            Step::SetHead(_) => touches.refs.push("HEAD".to_owned()),
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

/// The repository as it is right now, read fresh by the executor for the
/// pre-undo comparison (§6.7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoStateNow {
    /// `HEAD` now.
    pub head: HeadRef,
    /// The refs the entry recorded, read again; a missing name is absent.
    pub refs: BTreeMap<String, Option<Oid>>,
    /// The index tree now — `None` when the index is unmerged, which has no
    /// tree. That is not "the same as whatever was recorded": an entry that
    /// recorded a tree has drifted, and one that recorded none is still
    /// unknown.
    pub index_tree: Option<Oid>,
    /// The working-tree tree now (§6.3's hashing).
    pub worktree_tree: Oid,
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
    /// The index tree differs.
    pub index: bool,
    /// The working-tree tree differs.
    pub worktree: bool,
    /// What the inverse restores whole because its after-state was never
    /// recorded (`PathScope::Everything`): `"HEAD"`, `"the index"`, `"the
    /// working tree"`. Unknown is not clean: the user is told the whole
    /// thing is about to be overwritten, and asked (§6.7).
    pub unknown: Vec<&'static str>,
}

impl Drift {
    /// Nothing moved since the entry, and every restore is scoped to what
    /// the operation changed.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.refs.is_empty()
            && !self.head
            && !self.index
            && !self.worktree
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
        if self.index {
            parts.push("the index changed".to_owned());
        }
        if self.worktree {
            parts.push("the working tree changed".to_owned());
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
            .is_some_and(|tree| Some(tree) != now.index_tree),
        worktree: entry
            .worktree_after
            .is_some_and(|tree| tree != now.worktree_tree),
        unknown,
    }
}

#[cfg(test)]
mod tests;
