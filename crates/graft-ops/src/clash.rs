//! A stash put back that clashed (ADR-0045 §1–2).
//!
//! git keeps no state for a stash that clashed: no `MERGE_HEAD`, nothing to
//! `--continue`, only unmerged paths (research `commit-stash.md` A1.3). So
//! Graft records every put-back it starts on the journal line, and
//! [`live_clash`] reads "is the conflict in the tree now one of ours, and
//! which stash is it" back out of the journal with one pure rule.

use std::path::PathBuf;

use graft_core::journal::{EntryId, JournalEntry, OperationKind, PutBackWhy};
use graft_core::oid::Oid;
use graft_git::state::InProgress;

use crate::journal::in_force_all;

/// What a put-back that clashed left: `apply`'s answer in
/// [`crate::apply::ApplyError::Clashed`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clash {
    /// The stash put back — still in the list: git keeps it on a clash.
    pub stash: Oid,
    /// Pop, apply or set-aside.
    pub why: PutBackWhy,
    /// The unmerged paths the status lists after it.
    pub paths: Vec<PathBuf>,
}

/// A clash that is live now, as [`live_clash`] finds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StashClash {
    /// The entry that put the stash back.
    pub entry: EntryId,
    /// The stash's commit.
    pub stash: Oid,
    /// Its position in the list now, looked up by `stash`. Always `Some`
    /// from [`live_clash`]: a stash no longer listed is no clash (ADR-0045
    /// §2, third bullet).
    pub index: Option<usize>,
    /// Pop, apply or set-aside: whether finishing drops the stash.
    pub why: PutBackWhy,
}

/// Is the conflict in the tree now a stash Graft put back? ADR-0045 §2.
///
/// `entries` is the journal in order; `stashes` the list now as
/// `(commit, index)`, newest first.
///
/// - Nothing unmerged, or git has an operation in progress: no clash.
/// - Walk the in-force entries newest first. Skip one whose `index_before`
///   is unmerged or never recorded (a step taken inside the clash, or an
///   entry that never touched the index), and an `Undo`/`Redo` whose
///   `index_after` is unmerged (walking back into the clash).
/// - The first entry left is the clash only if its `index_before` is merged,
///   its `index_after` unmerged and it carries a `put_back`, and that stash
///   is still in the list, found by commit id. Anything else: no clash —
///   among them a pop made in the shell, which has no record.
#[must_use]
pub fn live_clash(
    entries: &[JournalEntry],
    in_progress: Option<InProgress>,
    any_unmerged: bool,
    stashes: &[(Oid, usize)],
) -> Option<StashClash> {
    if in_progress.is_some() || !any_unmerged {
        return None;
    }
    let flags = in_force_all(entries);
    let first = entries
        .iter()
        .zip(flags)
        .rev()
        .filter(|(_, in_force)| *in_force)
        .map(|(entry, _)| entry)
        .find(|entry| !taken_inside_a_clash(entry))?;
    let merged_before = first.index_before.is_some_and(|index| !index.is_unmerged());
    let unmerged_after = first.index_after.is_some_and(|index| index.is_unmerged());
    if !(merged_before && unmerged_after) {
        return None;
    }
    let put_back = first.put_back.as_ref()?;
    let index = stashes
        .iter()
        .find(|(commit, _)| *commit == put_back.stash)
        .map(|(_, index)| *index)?;
    Some(StashClash {
        entry: first.id.clone(),
        stash: put_back.stash,
        index: Some(index),
        why: put_back.why,
    })
}

/// An entry the walk passes over: its index before was unmerged or never
/// recorded, or it is an undo or redo that landed back in a clash.
fn taken_inside_a_clash(entry: &JournalEntry) -> bool {
    let before_unmerged_or_unknown = entry.index_before.is_none_or(|index| index.is_unmerged());
    let walked_back_in = matches!(entry.kind, OperationKind::Undo | OperationKind::Redo)
        && entry.index_after.is_some_and(|index| index.is_unmerged());
    before_unmerged_or_unknown || walked_back_in
}

#[cfg(test)]
mod tests {
    use graft_core::journal::{IndexState, PutBack};

    use super::*;
    use crate::journal::tests::{committed, oid, reversal};

    const STASH: u8 = 0x5a;

    /// A `Committed` pop (or apply) from a merged index into a clash.
    fn put_back(at: i64, why: PutBackWhy) -> JournalEntry {
        let kind = match why {
            PutBackWhy::Apply => OperationKind::StashApply,
            PutBackWhy::Pop | PutBackWhy::SetAside => OperationKind::StashPop,
        };
        JournalEntry {
            index_before: Some(IndexState::Merged(oid(0xa1))),
            index_after: Some(IndexState::Unmerged {
                unmerged: oid(0xc1),
            }),
            put_back: Some(PutBack {
                stash: oid(STASH),
                why,
            }),
            ..committed(at, kind, 1, 1)
        }
    }

    /// A step taken inside the clash: a whole-file take, from the stage tree
    /// to a merged index.
    fn take(at: i64) -> JournalEntry {
        JournalEntry {
            index_before: Some(IndexState::Unmerged {
                unmerged: oid(0xc1),
            }),
            index_after: Some(IndexState::Merged(oid(0xa2))),
            ..committed(at, OperationKind::ResolveConflict, 1, 1)
        }
    }

    fn listed() -> Vec<(Oid, usize)> {
        vec![(oid(0x11), 0), (oid(STASH), 1)]
    }

    /// The clash `live_clash` should name for `entry`, at `stash@{1}`.
    fn clash_of(entry: &JournalEntry, why: PutBackWhy) -> StashClash {
        StashClash {
            entry: entry.id.clone(),
            stash: oid(STASH),
            index: Some(1),
            why,
        }
    }

    /// The common case: nothing in progress, something unmerged, the stash
    /// listed at 1.
    fn live(entries: &[JournalEntry]) -> Option<StashClash> {
        live_clash(entries, None, true, &listed())
    }

    /// ADR-0045 §2, one assertion per row of its rule.
    #[test]
    fn live_clash_table() {
        let pop = put_back(1, PutBackWhy::Pop);
        let apply = put_back(1, PutBackWhy::Apply);
        let taken = take(2);
        let undo_of_take = reversal(3, OperationKind::Undo, &taken);
        let pop_clash = Some(clash_of(&pop, PutBackWhy::Pop));

        let one = std::slice::from_ref(&pop);
        assert_eq!(
            live_clash(one, None, false, &listed()),
            None,
            "nothing unmerged"
        );
        assert_eq!(
            live_clash(one, Some(InProgress::Merge), true, &listed()),
            None,
            "in progress"
        );
        assert_eq!(live(one), pop_clash, "pop clash");
        assert_eq!(
            live(std::slice::from_ref(&apply)),
            Some(clash_of(&apply, PutBackWhy::Apply)),
            "apply clash"
        );
        assert_eq!(
            live(&[pop.clone(), taken.clone()]),
            pop_clash,
            "after a take (index_before unmerged)"
        );
        assert_eq!(
            live(&[pop.clone(), taken, undo_of_take]),
            pop_clash,
            "an undo back into the clash"
        );
        // A pop made in the shell: Graft's newest entry is a plain commit.
        assert_eq!(
            live(&[committed(1, OperationKind::Commit, 1, 2)]),
            None,
            "shell pop, no record"
        );
        assert_eq!(
            live_clash(one, None, true, &[(oid(0x11), 0)]),
            None,
            "stash gone"
        );
        // Residual, words only (ADR-0045 §2): an apply-clash resolved, then a
        // shell pop clashes with no Graft write between — named as the
        // apply's stash. The journal cannot tell the two apart; apply never
        // drops, so nothing is lost.
        assert_eq!(
            live(std::slice::from_ref(&apply)),
            Some(clash_of(&apply, PutBackWhy::Apply)),
            "residual shell-after-apply"
        );
    }

    /// The pop itself undone: the pop is reversed, so not in force, and its
    /// `Undo` started from the clash — nothing is left to name, even while
    /// some other conflict is in the tree.
    #[test]
    fn an_undone_pop_is_no_clash() {
        let pop = put_back(1, PutBackWhy::Pop);
        let undo = reversal(2, OperationKind::Undo, &pop);
        assert_eq!(live(&[pop, undo]), None);
    }
}
