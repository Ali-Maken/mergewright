//! Pure tests for the snapshot scope and the backup ref's name. Everything
//! that spawns `git` is in `tests/it/`.

use super::*;

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
        backup_ref_name("main", &id),
        format!("refs/graft/backup/main/{id}")
    );
    assert_eq!(
        backup_ref_name("feature", &id),
        format!("refs/graft/backup/feature/{id}")
    );
}
