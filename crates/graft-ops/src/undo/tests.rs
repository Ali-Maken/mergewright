//! The exact step vectors per kind, the planners over a journal, and drift.

use std::collections::BTreeMap;

use graft_core::journal::{EntryStatus, HeadRef, OperationKind, Touches};

use crate::journal::tests::{MAIN, committed, oid, reversal};

use super::{
    Drift, InversePlan, NotInvertible, PathScope, RepoStateNow, Step, drift, inverse, plan_redo,
    plan_undo,
};

fn changed(from: u8, to: u8) -> PathScope {
    PathScope::ChangedBetween {
        from: oid(from),
        to: oid(to),
    }
}

fn restores(before: u8, after: u8, index: bool, worktree: bool) -> Vec<Step> {
    let mut steps = Vec::new();
    if index {
        steps.push(Step::RestoreIndex {
            tree: oid(0xa0 + before),
            paths: changed(0xa0 + after, 0xa0 + before),
        });
    }
    if worktree {
        steps.push(Step::RestoreWorktree {
            tree: oid(0xb0 + before),
            paths: changed(0xb0 + after, 0xb0 + before),
        });
    }
    steps
}

/// `committed` with the touches an operation of that kind really has.
fn op(at: i64, kind: OperationKind, before: u8, after: u8) -> graft_core::journal::JournalEntry {
    let (refs, index, worktree) = match kind {
        OperationKind::Stage | OperationKind::Unstage => (vec![], true, false),
        OperationKind::Discard => (vec![], true, true),
        OperationKind::Commit | OperationKind::Amend => (vec![MAIN.to_owned()], true, false),
        OperationKind::StashDrop => (vec!["refs/stash".to_owned()], false, false),
        _ => (vec!["refs/stash".to_owned()], true, true),
    };
    let mut entry = committed(at, kind, before, after);
    entry.touches = Touches {
        refs,
        index,
        worktree,
    };
    if !entry.touches.refs.contains(&MAIN.to_owned()) {
        entry.refs_before.clear();
        entry.refs_after.clear();
    }
    if entry.touches.refs.contains(&"refs/stash".to_owned()) {
        entry
            .refs_before
            .insert("refs/stash".to_owned(), Some(oid(0x70)));
        entry
            .refs_after
            .insert("refs/stash".to_owned(), Some(oid(0x71)));
    }
    if !index {
        entry.index_before = None;
        entry.index_after = None;
    }
    if !worktree {
        entry.worktree_before = None;
        entry.worktree_after = None;
    }
    entry
}

#[test]
fn commit_undo_is_a_ref_cas_and_an_index_restore() {
    let entry = op(10, OperationKind::Commit, 1, 2);
    let plan = inverse(&entry, None).unwrap();
    assert_eq!(
        plan,
        InversePlan {
            reverses: entry.id.clone(),
            kind: OperationKind::Undo,
            description: "undo: Commit at 10".to_owned(),
            touches: Touches {
                refs: vec![MAIN.to_owned()],
                index: true,
                worktree: false
            },
            steps: [
                vec![Step::UpdateRef {
                    name: MAIN.to_owned(),
                    expect: Some(oid(2)),
                    to: Some(oid(1)),
                }],
                restores(1, 2, true, false),
            ]
            .concat(),
            pins: vec![],
        }
    );
}

#[test]
fn a_first_commit_deletes_the_branch_and_puts_head_back_to_unborn() {
    let mut entry = op(10, OperationKind::Commit, 1, 2);
    entry.head_before = HeadRef::Unborn(MAIN.to_owned());
    entry.refs_before.insert(MAIN.to_owned(), None);
    let steps = inverse(&entry, None).unwrap().steps;
    assert_eq!(
        steps[..2],
        [
            Step::UpdateRef {
                name: MAIN.to_owned(),
                expect: Some(oid(2)),
                to: None,
            },
            Step::SetHead(HeadRef::Unborn(MAIN.to_owned())),
        ]
    );
}

#[test]
fn stage_discard_and_stash_kinds_have_their_own_step_vectors() {
    let stage = op(10, OperationKind::Stage, 1, 2);
    assert_eq!(
        inverse(&stage, None).unwrap().steps,
        restores(1, 2, true, false)
    );

    let discard = op(10, OperationKind::Discard, 1, 2);
    assert_eq!(
        inverse(&discard, None).unwrap().steps,
        restores(1, 2, true, true)
    );

    let push = op(10, OperationKind::StashPush, 1, 2);
    let plan = inverse(&push, None).unwrap();
    assert_eq!(
        plan.steps,
        [
            restores(1, 2, true, true),
            vec![Step::StashDrop { commit: oid(0x71) }]
        ]
        .concat()
    );
    assert_eq!(
        plan.touches,
        Touches {
            refs: vec!["refs/stash".to_owned()],
            index: true,
            worktree: true
        }
    );

    let mut drop = op(10, OperationKind::StashDrop, 1, 2);
    drop.pins = vec![oid(0x70)];
    assert_eq!(
        inverse(&drop, None).unwrap().steps,
        [Step::StashStore { commit: oid(0x70) }]
    );

    let mut pop = op(10, OperationKind::StashPop, 1, 2);
    pop.pins = vec![oid(0x70)];
    assert_eq!(
        inverse(&pop, None).unwrap().steps,
        [
            restores(1, 2, true, true),
            vec![Step::StashStore { commit: oid(0x70) }]
        ]
        .concat()
    );

    let apply = op(10, OperationKind::StashApply, 1, 2);
    assert_eq!(
        inverse(&apply, None).unwrap().steps,
        restores(1, 2, true, true)
    );
}

#[test]
fn the_stash_ref_is_never_moved_by_update_ref() {
    let push = op(10, OperationKind::StashPush, 1, 2);
    let steps = inverse(&push, None).unwrap().steps;
    assert!(
        !steps
            .iter()
            .any(|step| matches!(step, Step::UpdateRef { name, .. } if name == "refs/stash"))
    );
}

#[test]
fn redo_of_an_undone_stash_pop_drops_the_stash_after_the_generic_steps() {
    let mut pop = op(10, OperationKind::StashPop, 1, 2);
    pop.pins = vec![oid(0x70)];
    let undo = reversal(20, OperationKind::Undo, &pop);
    let entries = vec![pop.clone(), undo.clone()];
    let plan = plan_redo(&entries).unwrap();
    assert_eq!(plan.reverses, undo.id);
    assert_eq!(plan.kind, OperationKind::Redo);
    assert_eq!(plan.description, "redo: StashPop at 10");
    // The undo's own before/after are the pop's after/before, so its generic
    // inverse restores the popped state: trees 2 from trees 1.
    assert_eq!(
        plan.steps,
        [
            restores(2, 1, true, true),
            vec![Step::StashDrop { commit: oid(0x70) }]
        ]
        .concat()
    );
    let push = op(10, OperationKind::StashPush, 1, 2);
    let undo_push = reversal(20, OperationKind::Undo, &push);
    let redo = inverse(&undo_push, Some(&push)).unwrap();
    assert_eq!(
        redo.steps.last(),
        Some(&Step::StashStore { commit: oid(0x71) })
    );
}

#[test]
fn a_plan_pins_every_stash_commit_it_drops() {
    // Undo of a push drops the pushed stash: the Undo entry must pin it, or
    // the redo's `StashStore` depends on gc timing.
    let push = op(10, OperationKind::StashPush, 1, 2);
    assert_eq!(inverse(&push, None).unwrap().pins, [oid(0x71)]);
    // Undo of a pop stores the pinned stash back: nothing becomes unreachable.
    let mut pop = op(10, OperationKind::StashPop, 1, 2);
    pop.pins = vec![oid(0x70)];
    assert_eq!(inverse(&pop, None).unwrap().pins, []);
    // Redo of that undo drops it again, so the Redo entry pins it too.
    let undo = reversal(20, OperationKind::Undo, &pop);
    assert_eq!(plan_redo(&[pop, undo]).unwrap().pins, [oid(0x70)]);
    let commit = op(10, OperationKind::Commit, 1, 2);
    assert_eq!(inverse(&commit, None).unwrap().pins, []);
}

#[test]
fn undo_of_a_push_that_pushed_nothing_never_drops_the_users_own_stash() {
    // `git stash push` with nothing to save exits 0: `Committed`, and
    // `refs/stash` is still the stash the user already had.
    let mut push = op(10, OperationKind::StashPush, 1, 2);
    push.refs_after
        .insert("refs/stash".to_owned(), Some(oid(0x70)));
    let undo = inverse(&push, None).unwrap();
    assert!(
        !undo
            .steps
            .iter()
            .any(|step| matches!(step, Step::StashDrop { .. })),
        "{:?}",
        undo.steps
    );
    assert_eq!(undo.pins, []);
    // Nor does redoing that undo store anything.
    let undone = reversal(20, OperationKind::Undo, &push);
    let redo = inverse(&undone, Some(&push)).unwrap();
    assert!(
        !redo
            .steps
            .iter()
            .any(|step| matches!(step, Step::StashStore { .. }))
    );
    // A push with no stash before is the normal case: the new one is dropped.
    let mut first = op(10, OperationKind::StashPush, 1, 2);
    first.refs_before.insert("refs/stash".to_owned(), None);
    assert_eq!(
        inverse(&first, None).unwrap().steps.last(),
        Some(&Step::StashDrop { commit: oid(0x71) })
    );
    // An unrecorded before-value cannot tell a new stash from an old one.
    let mut unknown = op(10, OperationKind::StashPush, 1, 2);
    unknown.refs_before.remove("refs/stash");
    assert_eq!(
        inverse(&unknown, None),
        Err(NotInvertible::Incomplete {
            field: "refs_before[refs/stash]"
        })
    );
}

#[test]
fn redo_of_an_undo_that_failed_half_way_says_so() {
    let discard = op(10, OperationKind::Discard, 1, 2);
    let mut undo = reversal(20, OperationKind::Undo, &discard);
    undo.status = EntryStatus::Aborted {
        reason: "restore: I/O error".to_owned(),
    };
    undo.worktree_after = None;
    let redo = inverse(&undo, Some(&discard)).unwrap();
    assert_eq!(redo.kind, OperationKind::Redo);
    assert_eq!(
        redo.description,
        "redo: Discard at 10 (after a failed undo)"
    );
}

#[test]
fn undoing_a_redo_is_undoing_the_original() {
    let mut pop = op(10, OperationKind::StashPop, 1, 2);
    pop.pins = vec![oid(0x70)];
    let undo = reversal(20, OperationKind::Undo, &pop);
    let redo = reversal(30, OperationKind::Redo, &undo);
    let plan = inverse(&redo, Some(&pop)).unwrap();
    assert_eq!(plan.kind, OperationKind::Undo);
    assert_eq!(plan.description, "undo: StashPop at 10");
    assert_eq!(
        plan.steps.last(),
        Some(&Step::StashStore { commit: oid(0x70) })
    );
    assert_eq!(
        inverse(&undo, None),
        Err(NotInvertible::Incomplete {
            field: "reverses (no root operation)"
        })
    );
}

#[test]
fn plan_undo_walks_the_journal() {
    let a = op(10, OperationKind::Commit, 1, 2);
    let b = op(20, OperationKind::Stage, 2, 3);
    let entries = vec![a.clone(), b.clone()];
    assert_eq!(plan_undo(&entries).unwrap().reverses, b.id);
    let undone = vec![a.clone(), b.clone(), reversal(30, OperationKind::Undo, &b)];
    assert_eq!(plan_undo(&undone).unwrap().reverses, a.id);
    assert_eq!(plan_undo(&[]), Err(NotInvertible::NothingToUndo));
    assert_eq!(plan_redo(&entries), Err(NotInvertible::NothingToRedo));
}

#[test]
fn pending_no_backup_and_incomplete_entries_are_not_invertible() {
    let mut pending = op(10, OperationKind::Stage, 1, 2);
    pending.status = EntryStatus::Pending;
    assert_eq!(inverse(&pending, None), Err(NotInvertible::Pending));

    let mut aborted = op(10, OperationKind::Discard, 1, 2);
    aborted.status = EntryStatus::Aborted {
        reason: "x".to_owned(),
    };
    aborted.worktree_before = None;
    assert_eq!(inverse(&aborted, None), Err(NotInvertible::NoBackup));

    // Aborted with a backup but no after-trees: restore everything.
    let mut half = op(10, OperationKind::StashPop, 1, 2);
    half.pins = vec![oid(0x70)];
    half.status = EntryStatus::Aborted {
        reason: "conflict".to_owned(),
    };
    half.index_after = None;
    half.worktree_after = None;
    let steps = inverse(&half, None).unwrap().steps;
    assert_eq!(
        steps,
        [
            Step::RestoreIndex {
                tree: oid(0xa1),
                paths: PathScope::Everything
            },
            Step::RestoreWorktree {
                tree: oid(0xb1),
                paths: PathScope::Everything
            },
            Step::StashStore { commit: oid(0x70) },
        ]
    );

    let mut no_after_ref = op(10, OperationKind::Commit, 1, 2);
    no_after_ref.refs_after.clear();
    assert_eq!(
        inverse(&no_after_ref, None),
        Err(NotInvertible::Incomplete {
            field: "refs_after"
        })
    );
    let no_pin = op(10, OperationKind::StashDrop, 1, 2);
    assert_eq!(
        inverse(&no_pin, None),
        Err(NotInvertible::Incomplete { field: "pins" })
    );
}

#[test]
fn drift_sets_exactly_the_field_that_moved() {
    // `committed` records every field: a ref, `HEAD` and both trees.
    let entry = committed(10, OperationKind::Discard, 1, 2);
    let now = RepoStateNow {
        head: HeadRef::Branch(MAIN.to_owned()),
        refs: BTreeMap::from([(MAIN.to_owned(), Some(oid(2)))]),
        index_tree: Some(oid(0xa2)),
        worktree_tree: oid(0xb2),
    };
    let clean = drift(&entry, &now);
    assert!(clean.is_clean());
    assert_eq!(clean.explain(), "nothing changed since");

    let moved_ref = RepoStateNow {
        refs: BTreeMap::from([(MAIN.to_owned(), Some(oid(9)))]),
        ..now.clone()
    };
    assert_eq!(
        drift(&entry, &moved_ref),
        Drift {
            refs: vec![(MAIN.to_owned(), Some(oid(2)), Some(oid(9)))],
            ..Drift::default()
        }
    );
    assert_eq!(drift(&entry, &moved_ref).explain(), "refs/heads/main moved");

    let index = RepoStateNow {
        index_tree: Some(oid(0xaf)),
        ..now.clone()
    };
    assert_eq!(
        drift(&entry, &index),
        Drift {
            index: true,
            ..Drift::default()
        }
    );
    assert_eq!(drift(&entry, &index).explain(), "the index changed");

    let worktree = RepoStateNow {
        worktree_tree: oid(0xbf),
        head: HeadRef::Detached(oid(2)),
        ..now.clone()
    };
    let both = drift(&entry, &worktree);
    assert!(both.worktree && both.head && !both.index && both.refs.is_empty());
    assert_eq!(both.explain(), "HEAD moved; the working tree changed");
}

#[test]
fn an_unrecorded_after_state_is_reported_as_unknown_never_as_clean() {
    // The inverse restores such a field whole (`PathScope::Everything`), so
    // the rail must say so even when nothing else moved.
    let entry = committed(10, OperationKind::Discard, 1, 2);
    let now = RepoStateNow {
        head: HeadRef::Branch(MAIN.to_owned()),
        refs: BTreeMap::from([(MAIN.to_owned(), Some(oid(2)))]),
        index_tree: Some(oid(0xa2)),
        worktree_tree: oid(0xb2),
    };
    assert!(drift(&entry, &now).is_clean());

    let mut no_worktree = entry.clone();
    no_worktree.worktree_after = None;
    let unknown = drift(&no_worktree, &now);
    assert!(!unknown.is_clean());
    assert_eq!(
        unknown,
        Drift {
            unknown: vec!["the working tree"],
            ..Drift::default()
        }
    );
    assert_eq!(
        unknown.explain(),
        "the working tree's state after the operation was never recorded; undo restores all of it"
    );

    let mut nothing_after = entry.clone();
    nothing_after.head_after = None;
    nothing_after.index_after = None;
    nothing_after.worktree_after = None;
    assert_eq!(
        drift(&nothing_after, &now).unknown,
        ["HEAD", "the index", "the working tree"]
    );

    // A field the operation never touched was never going to be recorded.
    let mut untouched = entry.clone();
    untouched.touches.index = false;
    untouched.index_after = None;
    assert!(drift(&untouched, &now).is_clean());
}

#[test]
fn remote_entries_are_not_invertible() {
    for kind in [OperationKind::Fetch, OperationKind::Push] {
        let entry = committed(10, kind, 1, 2);
        assert_eq!(
            inverse(&entry, None),
            Err(NotInvertible::NotUndoable(kind)),
            "{kind:?}"
        );
    }
    let pull = committed(10, OperationKind::Pull, 1, 2);
    assert!(inverse(&pull, None).is_ok());
}
