//! The move from whatever generation is in force straight to `CURRENT`
//! (ADR-0057, Amendment 2026-10-11 §3–§7; ticket 48 Part B): a repository
//! still on Graft names, one on Tributary names (`clean` or `switched`), a
//! failed earlier move's scratch, every generation's lock, and what older
//! builds write after the move. Truth comes from git (`for-each-ref`,
//! `fsck`), the filesystem and the journal reader, never from the outcome
//! alone. Every name is built from the `Generation` table.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_core::journal::{JournalEntry, OperationKind};
use graft_core::on_disk::Generation;
use graft_git::file_lock::FileLock;
use graft_ops::operation::{Operation, Target};
use graft_ops::rename::{
    KEPT_ASIDE_DIR, LOCK_FILE, MigrateError, SettleOutcome, description, settle_names,
};

use super::{old_repo, shim, stage_edit};
use crate::support::{NOW, journal_for, run, state_of};

mod setup;

use setup::{
    JOURNAL, as_tributary, folders_named, fsck_is_clean, in_dir, loose_commit, marker_of,
    migrate_rows, moved, no_earlier_names_left, refs_of, undo_puts_back,
};

/// Seventeen branch creates and three stages under Graft's names (the
/// 26 test's 20 entries, 3 backups): no marker, no `tributary` anywhere.
#[test]
fn graft_repo_moves_straight_to_mergewright() {
    let repo = old_repo(17, 2);
    repo.write("a.rs", b"edit 2\n");
    let state = state_of(&repo);
    run(
        &repo,
        &Operation::Stage(Target::Paths(vec![PathBuf::from("a.rs")])),
        NOW + 200,
    );
    let old = refs_of(&repo, Generation::Graft);
    let before = journal_for(&repo.open()).entries().to_vec();
    assert_eq!((before.len(), old.len()), (20, 3));
    let newest = before.last().expect("an entry").clone();

    let settled = settle_names(&repo.open());

    let SettleOutcome::Migrated {
        refs: 3,
        files,
        not_journaled: None,
        kept_aside: None,
    } = settled.outcome
    else {
        panic!("migrated: {:?}", settled.outcome);
    };
    assert_eq!(settled.names, Generation::CURRENT);
    assert_eq!(
        refs_of(&repo, Generation::CURRENT),
        moved(&old, Generation::CURRENT)
    );
    no_earlier_names_left(&repo);
    assert_eq!(
        marker_of(&repo, Generation::CURRENT).as_deref(),
        Some("clean\n")
    );

    let entries = journal_for(&repo.open()).entries().to_vec();
    assert_eq!(entries.len(), 21, "the 20 and the move");
    let migrate: Vec<&JournalEntry> = entries
        .iter()
        .filter(|entry| entry.kind == OperationKind::Migrate)
        .collect();
    assert_eq!(migrate.len(), 1);
    assert_eq!(
        migrate[0].description,
        description(Generation::CURRENT, 3, files)
    );
    assert_eq!(
        entries[19].backup_ref,
        newest.backup_ref.as_ref().map(|name| name.replacen(
            Generation::Graft.ref_prefix(),
            Generation::CURRENT.ref_prefix(),
            1
        )),
    );

    undo_puts_back(&repo, &newest, &state);
}

/// A Tributary build moved the Graft repository (`rename to Tributary`)
/// and ran two stages under its names; this build moves it on, once more.
#[test]
fn tributary_repo_moves_and_both_rename_rows_read() {
    let tributary = Generation::Tributary;
    let repo = old_repo(17, 3);
    as_tributary(&repo, "clean\n", false);
    stage_edit(&repo, "under tributary\n", NOW + 300);
    repo.write("a.rs", b"under tributary, again\n");
    let state = state_of(&repo);
    run(
        &repo,
        &Operation::Stage(Target::Paths(vec![PathBuf::from("a.rs")])),
        NOW + 301,
    );
    let old = refs_of(&repo, tributary);
    assert_eq!(old.len(), 5, "3 moved from Graft, 2 made under Tributary");
    let before = journal_for(&repo.open()).entries().to_vec();
    assert_eq!(before.len(), 23);
    let newest = before.last().expect("an entry").clone();
    assert!(
        newest
            .backup_ref
            .as_deref()
            .is_some_and(|name| name.starts_with(tributary.ref_prefix())),
        "{newest:?}"
    );

    let settled = settle_names(&repo.open());

    assert!(
        matches!(
            settled.outcome,
            SettleOutcome::Migrated {
                refs: 5,
                files: 1,
                not_journaled: None,
                kept_aside: None,
            }
        ),
        "the five refs and the journal: {:?}",
        settled.outcome
    );
    assert_eq!(
        refs_of(&repo, Generation::CURRENT),
        moved(&old, Generation::CURRENT)
    );
    no_earlier_names_left(&repo);

    let entries = journal_for(&repo.open()).entries().to_vec();
    assert_eq!(entries.len(), 24, "the 20, two moves and two stages");
    // The J rows' words, pinned: each names the generation it moved to.
    assert_eq!(
        migrate_rows(&entries),
        [
            (20, "rename to Tributary (3 refs, 1 file)".to_owned()),
            (23, "rename to Mergewright (5 refs, 1 file)".to_owned()),
        ]
    );
    // Every backup, whichever generation wrote it, reads under the new names.
    let prefix = Generation::CURRENT.ref_prefix();
    let backups = entries.iter().filter_map(|entry| entry.backup_ref.as_ref());
    assert!(
        backups.clone().all(|name| name.starts_with(prefix)),
        "{:?}",
        backups.collect::<Vec<_>>()
    );

    undo_puts_back(&repo, &newest, &state);
}

/// A Tributary move killed after its switch: the Graft journal (a prefix of
/// Tributary's) and refs still there, one of them moved by a Graft build
/// since.
#[test]
fn switched_tributary_with_graft_leftovers() {
    let (graft, tributary) = (Generation::Graft, Generation::Tributary);
    let repo = old_repo(0, 2);
    as_tributary(&repo, "switched\n", true);
    let leftovers = refs_of(&repo, graft);
    let mut names = leftovers.keys();
    let same = names.next().expect("one").clone();
    let differs = names.next().expect("two").clone();
    let theirs = loose_commit(&repo);
    repo.git(&["update-ref", &differs, &theirs]);
    assert!(repo.exists(&in_dir(graft, JOURNAL)));
    let held = refs_of(&repo, tributary);

    let settled = settle_names(&repo.open());

    let kept_prefix = format!("{}{KEPT_ASIDE_DIR}/1/", Generation::CURRENT.ref_prefix());
    assert!(
        matches!(
            &settled.outcome,
            SettleOutcome::Migrated { refs: 2, not_journaled: None, kept_aside: Some(at), .. }
                if at == Path::new(&kept_prefix)
        ),
        "{:?}",
        settled.outcome
    );
    let kept = |name: &str| {
        let rest = name.strip_prefix(graft.ref_prefix()).expect("a Graft ref");
        format!("{kept_prefix}{}/{rest}", graft.dir())
    };
    let mut expected = moved(&held, Generation::CURRENT);
    expected.insert(kept(&differs), theirs);
    let now = refs_of(&repo, Generation::CURRENT);
    assert_eq!(now, expected);
    assert!(
        !now.contains_key(&kept(&same)),
        "the same value is deleted, not kept"
    );
    no_earlier_names_left(&repo);
    assert!(
        !repo.exists(&in_dir(Generation::CURRENT, KEPT_ASIDE_DIR)),
        "the prefix journal was deleted, not kept"
    );
    fsck_is_clean(&repo);
}

/// A failed earlier move's scratch: Tributary names in a repository still
/// on Graft, with no Tributary marker. Removed after the switch, never
/// compared, never kept.
#[test]
fn marker_less_newer_generation_is_scratch() {
    let tributary = Generation::Tributary;
    let repo = old_repo(0, 1);
    let old = refs_of(&repo, Generation::Graft);
    let head = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["update-ref", &format!("{}x", tributary.ref_prefix()), &head]);
    repo.write(&in_dir(tributary, LOCK_FILE), b"");
    repo.write(&in_dir(tributary, JOURNAL), b"{\"scratch\":1}\n");
    assert_eq!(repo.open().on_disk_names(), Generation::Graft);

    let settled = settle_names(&repo.open());

    assert!(
        matches!(
            settled.outcome,
            SettleOutcome::Migrated {
                refs: 1,
                not_journaled: None,
                kept_aside: None,
                ..
            }
        ),
        "{:?}",
        settled.outcome
    );
    assert_eq!(refs_of(&repo, tributary), BTreeMap::new());
    assert_eq!(folders_named(&repo, tributary.dir()), Vec::<PathBuf>::new());
    assert_eq!(
        refs_of(&repo, Generation::CURRENT),
        moved(&old, Generation::CURRENT)
    );
    assert!(!repo.exists(&in_dir(Generation::CURRENT, KEPT_ASIDE_DIR)));
    fsck_is_clean(&repo);
}

/// Any generation's lock held shared (a build of it still open on the
/// names in force) keeps the move from starting.
#[test]
fn any_generations_lock_blocks_the_move() {
    assert_eq!(
        MigrateError::Busy.to_string(),
        "another Mergewright window is open on this repository"
    );
    for held in [Generation::Tributary, Generation::CURRENT] {
        let repo = old_repo(0, 1);
        let old = refs_of(&repo, Generation::Graft);
        let path = repo.path().join(in_dir(held, LOCK_FILE));
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("the folder is made");
        let other = FileLock::shared(&path).expect("the lock is shared");

        let blocked = settle_names(&repo.open());
        assert_eq!(
            blocked.outcome,
            SettleOutcome::KeptOld(MigrateError::Busy),
            "{held:?}"
        );
        assert_eq!(blocked.names, Generation::Graft);
        assert!(blocked.hold.is_some(), "this run holds a lock for its life");
        assert_eq!(refs_of(&repo, Generation::Graft), old);
        assert_eq!(marker_of(&repo, Generation::CURRENT), None);

        drop(other);
        drop(blocked);
        let settled = settle_names(&repo.open());
        assert!(
            matches!(
                settled.outcome,
                SettleOutcome::Migrated {
                    refs: 1,
                    kept_aside: None,
                    ..
                }
            ),
            "{held:?}: {:?}",
            settled.outcome
        );
        assert_eq!(
            refs_of(&repo, Generation::CURRENT),
            moved(&old, Generation::CURRENT)
        );
        fsck_is_clean(&repo);
    }
}

/// After `clean`, a Tributary build writes its folder and a Graft build a
/// ref: one stray pass takes both, and the next open spawns nothing.
#[test]
fn older_generations_writing_after_clean() {
    let repo = old_repo(0, 1);
    assert!(matches!(
        settle_names(&repo.open()).outcome,
        SettleOutcome::Migrated { .. }
    ));
    let head = repo.git(&["rev-parse", "HEAD"]);
    repo.write(&in_dir(Generation::Tributary, "layout"), b"tributary\n");
    let graft_y = format!("{}y", Generation::Graft.ref_prefix());
    repo.git(&["update-ref", &graft_y, &head]);
    let mut expected = refs_of(&repo, Generation::CURRENT);
    expected.insert(format!("{}y", Generation::CURRENT.ref_prefix()), head);

    let (outcomes, spawned_second) = shim::settle_twice_counting(&repo);

    assert_eq!(
        outcomes,
        ["OlderWrote { refs: 1, files: 1, kept_aside: None }", "Done"]
    );
    assert_eq!(
        spawned_second,
        Vec::<String>::new(),
        "the second open spawned git"
    );
    assert_eq!(refs_of(&repo, Generation::CURRENT), expected);
    assert_eq!(
        repo.read(&in_dir(Generation::CURRENT, "layout")),
        b"tributary\n"
    );
    no_earlier_names_left(&repo);
    fsck_is_clean(&repo);
}
