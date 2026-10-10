//! The pure half of the rename: how a journal written under the old names
//! reads under the new ones. The move itself is judged against real
//! repositories in `tests/it/rename.rs`.

use graft_core::journal::OperationKind;
use graft_core::on_disk::OnDiskNames;

use super::{description, map_entries, map_ref};
use crate::journal::parse;
use crate::journal::tests::committed;

#[test]
fn an_old_prefix_reads_under_the_new_names_only() {
    let old = "refs/graft/backup/main/01HF7YAT000000000000000000";
    assert_eq!(
        map_ref(old, OnDiskNames::New),
        "refs/tributary/backup/main/01HF7YAT000000000000000000"
    );
    assert_eq!(
        map_ref(old, OnDiskNames::Old),
        old,
        "old names read as written"
    );
    for untouched in ["refs/heads/graft/x", "refs/heads/main", "refs/graftish/a"] {
        assert_eq!(map_ref(untouched, OnDiskNames::New), untouched);
    }
}

/// The M5 golden, kept byte-identical (ticket 26): written under the old
/// names, it reads under the new ones with every backup ref moved, and
/// renders back exactly as it was written — the mapping is a read, never a
/// rewrite.
#[test]
fn the_m5_golden_reads_its_backups_under_the_new_prefix() {
    let golden = include_bytes!("../../tests/it/goldens/m5-journal.jsonl");
    let mut entries = parse(golden).unwrap().entries;
    let as_written = entries.clone();
    map_entries(&mut entries, OnDiskNames::New);

    assert_eq!(entries.len(), 3);
    for (mapped, written) in entries.iter().zip(&as_written) {
        let backup = mapped.backup_ref.as_deref().unwrap();
        assert!(
            backup.starts_with("refs/tributary/backup/main/"),
            "{backup}"
        );
        assert_eq!(
            Some(backup.replacen("refs/tributary/", "refs/graft/", 1)),
            written.backup_ref
        );
        assert_eq!(
            mapped.refs_before, written.refs_before,
            "branches untouched"
        );
    }
    let mut rendered = Vec::new();
    for entry in &as_written {
        rendered.extend(crate::journal::render_pending(entry).unwrap());
        rendered.extend(crate::journal::render_outcome(entry).unwrap());
    }
    assert_eq!(rendered, golden, "the bytes on disk never change");
}

#[test]
fn every_ref_field_of_an_entry_is_mapped() {
    let old_backup = "refs/graft/backup/main/x".to_owned();
    let mut entry = committed(10, OperationKind::Commit, 1, 2);
    entry.backup_ref = Some(old_backup.clone());
    entry.touches.refs.push(old_backup.clone());
    entry.refs_before.insert(old_backup.clone(), None);
    entry.refs_after.insert(old_backup.clone(), None);
    let mut entries = vec![entry];
    map_entries(&mut entries, OnDiskNames::New);
    let [entry] = entries.as_slice() else {
        panic!()
    };
    let new = "refs/tributary/backup/main/x".to_owned();
    assert_eq!(entry.backup_ref.as_ref(), Some(&new));
    assert!(entry.touches.refs.contains(&new));
    assert!(entry.refs_before.contains_key(&new) && entry.refs_after.contains_key(&new));
    assert!(!entry.refs_before.contains_key(&old_backup));
}

#[test]
fn the_entry_words_count_what_moved() {
    assert_eq!(description(3, 12), "rename to Tributary (3 refs, 12 files)");
    assert_eq!(description(1, 1), "rename to Tributary (1 ref, 1 file)");
}
