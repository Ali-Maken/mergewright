//! Pure tests for the snapshot scope and the backup ref's name. Everything
//! that spawns `git` is in `tests/it/`.

use super::*;
use graft_core::on_disk::OnDiskNames;

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
        backup_ref_name(OnDiskNames::Old, "main", &id),
        format!("refs/graft/backup/main/{id}")
    );
    assert_eq!(
        backup_ref_name(OnDiskNames::Old, "feature", &id),
        format!("refs/graft/backup/feature/{id}")
    );
}

#[test]
fn old_link_trailer_reads() {
    // Backup commits are immutable: a chain written before the rename keeps
    // its `Graft-Link:` lines forever, so both spellings read; only the new
    // one is written (ADR-0057, Amendment, Consequences).
    let commit = Oid::from_hex("0123456789abcdef0123456789abcdef01234567").expect("an oid");
    for (message, kind) in [
        ("graft x\n\nGraft-Link: resume", LinkKind::Resume),
        ("graft x\n\nGraft-Link: step\n", LinkKind::Step),
        ("x\n\nTributary-Link: resume", LinkKind::Resume),
        ("x\n\nTributary-Link: step", LinkKind::Step),
        ("x step within: merge feature", LinkKind::Step),
    ] {
        assert_eq!(
            chain::link_of(commit, message).expect("reads"),
            kind,
            "{message:?}"
        );
    }
    assert!(matches!(
        chain::link_of(commit, "x\n\nTributary-Link: sideways"),
        Err(SnapshotError::UnknownLink { .. })
    ));
    assert_eq!(LinkKind::Step.trailer(), "Tributary-Link: step");
    assert_eq!(LinkKind::Resume.trailer(), "Tributary-Link: resume");
}

#[test]
fn the_backup_ref_takes_the_prefix_in_force() {
    let id = EntryId::new(1_700_000_000_000, [7; 10]);
    assert_eq!(
        backup_ref_name(OnDiskNames::New, "main", &id),
        format!("refs/tributary/backup/main/{id}")
    );
}
