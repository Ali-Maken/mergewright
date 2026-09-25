//! A stash put back that clashed (ADR-0045 §1–2).
//!
//! git keeps no state for a stash that clashed: no `MERGE_HEAD`, nothing to
//! `--continue`, only unmerged paths (research `commit-stash.md` A1.3). So
//! Graft records every put-back it starts on the journal line, and
//! [`live_clash`] reads "is the conflict in the tree now one of ours, and
//! which stash is it" back out of the journal with one pure rule.

use std::path::PathBuf;

use graft_core::journal::{EntryId, IndexState, JournalEntry, OperationKind, PutBackWhy};
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

/// A put-back clash the journal names, before the index now is compared:
/// [`clash_candidate`]'s answer. The app reads the index only while one
/// exists (ADR-0045 §2, amended 2026-09-25).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClashCandidate {
    /// The clash, if the index now is `expect`.
    pub clash: StashClash,
    /// The index the newest in-force entry that recorded one left: the
    /// candidate's own `index_after`, or that of a step Graft took inside
    /// the clash since. Always unmerged.
    pub expect: IndexState,
}

/// ADR-0045 §2's walk over the journal, without the index now.
///
/// `entries` is the journal in order; `stashes` the list now as
/// `(commit, index)`, newest first.
///
/// - Nothing unmerged, or git has an operation in progress: none.
/// - Walk the in-force entries newest first. Skip one whose `index_before`
///   is unmerged or never recorded (a step taken inside the clash, or an
///   entry that never touched the index), and an `Undo`/`Redo` whose
///   `index_after` is unmerged (walking back into the clash).
/// - The first entry left is the candidate only if its `index_before` is
///   merged, its `index_after` unmerged and it carries a `put_back`, and that
///   stash is still in the list, found by commit id. Anything else: none —
///   among them a pop made in the shell, which has no record.
/// - `expect` is the `index_after` of the newest in-force entry that
///   recorded an index at all. Not recorded, or merged: none, since the tie
///   to the index now cannot hold.
#[must_use]
pub fn clash_candidate(
    entries: &[JournalEntry],
    in_progress: Option<InProgress>,
    any_unmerged: bool,
    stashes: &[(Oid, usize)],
) -> Option<ClashCandidate> {
    if in_progress.is_some() || !any_unmerged {
        return None;
    }
    let flags = in_force_all(entries);
    let newest_first = || {
        entries
            .iter()
            .zip(flags.iter())
            .rev()
            .filter(|(_, in_force)| **in_force)
            .map(|(entry, _)| entry)
    };
    let first = newest_first().find(|entry| !taken_inside_a_clash(entry))?;
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
    let expect = newest_first()
        .find(|entry| entry.index_before.is_some() || entry.index_after.is_some())?
        .index_after
        .filter(IndexState::is_unmerged)?;
    Some(ClashCandidate {
        clash: StashClash {
            entry: first.id.clone(),
            stash: put_back.stash,
            index: Some(index),
            why: put_back.why,
        },
        expect,
    })
}

/// Is the conflict in the tree now a stash Graft put back? ADR-0045 §2
/// whole, pure: [`clash_candidate`]'s walk, with "anything unmerged" read
/// off `index_now`, then **tied to the index now** — the candidate holds
/// only when `index_now` is exactly its `expect`. Anything that changed the
/// index without a journal line since (a shell `add`, a shell pop, a reset)
/// breaks the tie: no clash, so nothing is named and nothing dropped.
#[must_use]
pub fn live_clash(
    entries: &[JournalEntry],
    in_progress: Option<InProgress>,
    index_now: &IndexState,
    stashes: &[(Oid, usize)],
) -> Option<StashClash> {
    clash_candidate(entries, in_progress, index_now.is_unmerged(), stashes)
        .filter(|candidate| candidate.expect == *index_now)
        .map(|candidate| candidate.clash)
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
    use graft_core::journal::PutBack;

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

    /// The stage tree the pop left.
    fn popped() -> IndexState {
        IndexState::Unmerged {
            unmerged: oid(0xc1),
        }
    }

    /// The stage tree after one file of a two-file clash is marked done.
    fn one_done() -> IndexState {
        IndexState::Unmerged {
            unmerged: oid(0xc2),
        }
    }

    /// A stage tree Graft never recorded: what a shell `add` or a shell pop
    /// leaves.
    fn made_in_the_shell() -> IndexState {
        IndexState::Unmerged {
            unmerged: oid(0xc9),
        }
    }

    /// A step taken inside a two-file clash: one file marked done, the other
    /// still unmerged.
    fn take(at: i64) -> JournalEntry {
        JournalEntry {
            index_before: Some(popped()),
            index_after: Some(one_done()),
            ..committed(at, OperationKind::ResolveConflict, 1, 1)
        }
    }

    /// The last file of the clash marked done: the index is merged again.
    fn last_take(at: i64) -> JournalEntry {
        JournalEntry {
            index_before: Some(popped()),
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

    /// Nothing in progress, the stash listed at 1, the index now `index_now`.
    fn live(entries: &[JournalEntry], index_now: &IndexState) -> Option<StashClash> {
        live_clash(entries, None, index_now, &listed())
    }

    /// ADR-0045 §2 (amended 2026-09-25), one assertion per row of its rule;
    /// every row names the index now.
    #[test]
    fn live_clash_table() {
        let pop = put_back(1, PutBackWhy::Pop);
        let apply = put_back(1, PutBackWhy::Apply);
        let taken = take(2);
        let undo_of_take = reversal(3, OperationKind::Undo, &taken);
        let pop_clash = Some(clash_of(&pop, PutBackWhy::Pop));
        let merged = IndexState::Merged(oid(0xa2));

        let one = std::slice::from_ref(&pop);
        // §2 bullet 1: nothing unmerged.
        assert_eq!(live(one, &merged), None, "nothing unmerged");
        // §2 bullet 1: git has an operation in progress.
        assert_eq!(
            live_clash(one, Some(InProgress::Merge), &popped(), &listed()),
            None,
            "in progress"
        );
        // §2 bullets 2–3: the pop left exactly the index now.
        assert_eq!(live(one, &popped()), pop_clash, "pop clash");
        assert_eq!(
            live(std::slice::from_ref(&apply), &popped()),
            Some(clash_of(&apply, PutBackWhy::Apply)),
            "apply clash"
        );
        // §2 bullet 2: an undo walks back into the clash; its `index_after`
        // is the pop's stage tree, which is the index now.
        assert_eq!(
            live(&[pop.clone(), taken, undo_of_take], &popped()),
            pop_clash,
            "an undo back into the clash"
        );
        // §2 bullet 5: a pop made in the shell has no record.
        assert_eq!(
            live(&[committed(1, OperationKind::Commit, 1, 2)], &popped()),
            None,
            "shell pop, no record"
        );
        // §2 bullet 4: the stash is found by commit, and it is gone.
        assert_eq!(
            live_clash(one, None, &popped(), &[(oid(0x11), 0)]),
            None,
            "stash gone"
        );
    }

    /// ADR-0045 §7 (amended 2026-09-25): the five rows that tie a clash to
    /// the index now. Split from `live_clash_table` for length only.
    #[test]
    fn live_clash_table_tie_rows() {
        let pop = put_back(1, PutBackWhy::Pop);
        let apply = put_back(1, PutBackWhy::Apply);
        let pop_clash = Some(clash_of(&pop, PutBackWhy::Pop));
        let one = std::slice::from_ref(&pop);
        // §2 bullet 3 (tie), §7 row 3: a Graft step inside the clash, the
        // index now equal to that step's `index_after`.
        assert_eq!(
            live(&[pop.clone(), take(2)], &one_done()),
            pop_clash,
            "after a take, tied to the take's index"
        );
        // §2 bullet 3 (tie), §7 row 1 — review 72+72b M2: Graft popped A and
        // it clashed; a shell `add` and a shell pop of B changed the index
        // with no journal line. Not A's conflict: nothing named, nothing
        // dropped.
        assert_eq!(
            live(one, &made_in_the_shell()),
            None,
            "shell pop after an unfinished Graft pop"
        );
        // §2 bullet 3 (tie), §7 row 2: the shell pop clashes on the same path
        // with other blobs — another stage tree, so another id.
        let same_path_other_blobs = IndexState::Unmerged {
            unmerged: oid(0xc3),
        };
        assert_eq!(
            live(one, &same_path_other_blobs),
            None,
            "same path, other blobs"
        );
        // §2 bullet 3 (tie), §7 row 4: the newest index-touching entry did
        // not record its `index_after` — the tie cannot hold.
        let unrecorded = JournalEntry {
            index_after: None,
            ..take(2)
        };
        assert_eq!(
            live(&[pop.clone(), unrecorded], &one_done()),
            None,
            "newest index_after unrecorded"
        );
        // §2 bullet 6 (the replaced residual), §7 row 5: an apply clash
        // finished in Graft, then a shell pop clashes. The finish left a
        // merged index, so no candidate can be tied.
        assert_eq!(
            live(&[apply, last_take(2)], &made_in_the_shell()),
            None,
            "Graft apply clash finished, then a shell pop"
        );
    }

    /// The candidate carries the index the tie needs: the newest recorded
    /// `index_after`, not the pop's own once a step followed.
    #[test]
    fn the_candidate_expects_the_newest_recorded_index() {
        let pop = put_back(1, PutBackWhy::Pop);
        let found =
            clash_candidate(&[pop.clone(), take(2)], None, true, &listed()).expect("a candidate");
        assert_eq!(found.clash, clash_of(&pop, PutBackWhy::Pop));
        assert_eq!(found.expect, one_done());
    }

    /// The pop itself undone: the pop is reversed, so not in force, and its
    /// `Undo` started from the clash — nothing is left to name, even while
    /// some other conflict is in the tree.
    #[test]
    fn an_undone_pop_is_no_clash() {
        let pop = put_back(1, PutBackWhy::Pop);
        let undo = reversal(2, OperationKind::Undo, &pop);
        assert_eq!(live(&[pop, undo], &popped()), None);
    }
}
