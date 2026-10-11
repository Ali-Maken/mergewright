//! Whether a lost run is really lost, or only left behind by a step `u`
//! can still take back (usability Q1, m8/46).
//!
//! A rebase leaves the branch's old tip held by nothing — but while that
//! rebase is in force and undoable, `u` puts the branch back on it. Calling
//! that commit "lost" tells him real damage happened when none did; calling
//! a shell reset's commit anything but lost would tell him the opposite.

use graft_core::journal::{JournalEntry, OperationKind};
use graft_core::oid::Oid;

use crate::journal::{is_operation, root_operation, undoable};

/// The entry a run under `entries[index]` is judged by, and whether it is in
/// force: the entry itself, or for a `Redo`, the operation it put back. `u`
/// steps over the redo and undoes that operation, which puts the old tip
/// back (review m8 g1, minor 6). Both must be in force.
pub(super) fn judged_by<'a>(
    entries: &'a [JournalEntry],
    in_force: &[bool],
    index: usize,
) -> (&'a JournalEntry, bool) {
    let entry = &entries[index];
    let root = (entry.kind == OperationKind::Redo)
        .then(|| root_operation(entries, entry))
        .flatten()
        .and_then(|root| entries.iter().position(|each| each.id == root.id));
    match root {
        Some(at) => (&entries[at], in_force[index] && in_force[at]),
        // Not a redo, or a redo whose operation is gone: the entry itself
        // (a dangling redo stays lost, `fate_of` refusing a non-operation).
        None => (entry, in_force[index]),
    }
}

/// What became of a lost run's tip, by the event it sits under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fate {
    /// Nothing holds it and nothing will bring it back by itself.
    Lost,
    /// An in-force, undoable step moved a ref off it; `u` puts it back.
    Replaced,
    /// An in-force, undoable step deleted the ref that held it; `u` puts
    /// it back.
    Deleted,
}

/// What became of `tip`. Pure.
///
/// `event` is the `●` entry the run sits under and whether that entry is in
/// force; `None` for a `$` event, whose runs are always [`Fate::Lost`]. The
/// entry must be in force and [`undoable`], and one of its before-refs (a
/// branch or `HEAD`, the refs a run is placed by) must have pointed at `tip`:
/// [`Fate::Replaced`] when its after-state has that ref on another commit,
/// [`Fate::Deleted`] when it has the ref recorded as absent. A ref missing
/// from the after-state was never recorded — unknown, not deleted — and
/// leaves the run [`Fate::Lost`]. `u` steps over an `Undo` or `Redo` entry,
/// so commits an undo dropped stay lost; a redo's run is judged by the
/// operation it put back ([`judged_by`]).
#[must_use]
pub fn fate_of(event: Option<(&JournalEntry, bool)>, tip: Oid) -> Fate {
    let Some((entry, in_force)) = event else {
        return Fate::Lost;
    };
    // `undoable` alone passes an `Undo` or `Redo` entry; `u` steps over those
    // (`is_operation`), so what they dropped is redo's business, not `u`'s.
    if !in_force || !is_operation(entry) || !undoable(entry) {
        return Fate::Lost;
    }
    let mut fate = Fate::Lost;
    for (name, before) in super::branch_or_head(&entry.refs_before) {
        if before != tip {
            continue;
        }
        match entry.refs_after.get(name) {
            // A move off the tip is the stronger word: the ref still exists.
            Some(Some(after)) if *after != tip => return Fate::Replaced,
            Some(None) => fate = Fate::Deleted,
            // Unmoved, or never recorded: says nothing.
            Some(Some(_)) | None => {}
        }
    }
    fate
}
