//! Pure tests for the snapshot scope and the backup ref's name. Everything
//! that spawns `git` is in `tests/it/`.

use super::*;
use mergewright_core::on_disk::Generation;

fn touches(refs: &[&str], index: bool, worktree: bool) -> Touches {
    Touches {
        refs: refs.iter().map(|name| (*name).to_owned()).collect(),
        index,
        worktree,
    }
}

#[test]
fn anything_touching_the_index_or_the_working_tree_takes_a_full_snapshot() {
    assert_eq!(scope(&touches(&[], true, false)), SnapshotScope::Full);
    assert_eq!(scope(&touches(&[], false, true)), SnapshotScope::Full);
    assert_eq!(scope(&touches(&[], true, true)), SnapshotScope::Full);
    assert_eq!(
        scope(&touches(&["refs/heads/main"], true, false)),
        SnapshotScope::Full
    );
}

#[test]
fn a_ref_only_operation_takes_no_snapshot() {
    assert_eq!(
        scope(&touches(&["refs/stash"], false, false)),
        SnapshotScope::None
    );
    assert_eq!(scope(&touches(&[], false, false)), SnapshotScope::None);
}

#[test]
fn the_backup_ref_is_namespaced_by_worktree_and_named_by_entry() {
    let id = EntryId::new(1_700_000_000_000, [7; 10]);
    assert_eq!(
        backup_ref_name(Generation::Graft, "main", &id),
        format!("refs/graft/backup/main/{id}")
    );
    assert_eq!(
        backup_ref_name(Generation::Graft, "feature", &id),
        format!("refs/graft/backup/feature/{id}")
    );
}

/// Backup commits are immutable: a chain written under any generation's
/// names keeps its `<Title>-Link:` lines forever, so every generation's key
/// reads; only `CURRENT`'s is written (ADR-0057, Amendment 2026-10-11 §1).
#[test]
fn every_generations_trailer_reads() {
    let commit = Oid::from_hex("0123456789abcdef0123456789abcdef01234567").expect("an oid");
    for generation in Generation::GENERATIONS {
        let key = generation.link_trailer();
        for (message, kind) in [
            (format!("x\n\n{key} resume"), LinkKind::Resume),
            (format!("x\n\n{key} step\n"), LinkKind::Step),
        ] {
            assert_eq!(
                chain::link_of(commit, &message).expect("reads"),
                kind,
                "{message:?}"
            );
        }
        assert!(matches!(
            chain::link_of(commit, &format!("x\n\n{key} sideways")),
            Err(SnapshotError::UnknownLink { .. })
        ));
    }
    assert_eq!(
        chain::link_of(commit, "x step within: merge feature").expect("reads"),
        LinkKind::Step,
        "no trailer at all reads as a step"
    );
    // The writer and the backup identity: the known-good spellings, pinned.
    assert_eq!(LinkKind::Step.trailer(), "Mergewright-Link: step");
    assert_eq!(LinkKind::Resume.trailer(), "Mergewright-Link: resume");
    assert_eq!(
        (BACKUP_AUTHOR_NAME, BACKUP_AUTHOR_EMAIL),
        ("mergewright", "mergewright@backup")
    );
}

#[test]
fn the_backup_ref_takes_the_prefix_in_force() {
    let id = EntryId::new(1_700_000_000_000, [7; 10]);
    assert_eq!(
        backup_ref_name(Generation::CURRENT, "main", &id),
        format!("{}backup/main/{id}", Generation::CURRENT.ref_prefix())
    );
}
