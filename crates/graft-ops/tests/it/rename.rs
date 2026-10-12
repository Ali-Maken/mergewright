//! The one-time move to the new on-disk names (m8-polish-and-ship/26 Part
//! B): refs, every worktree's folder, the journal read through it, and the
//! lock that keeps a second window out. Judged by reading the repository
//! back with `git` and the filesystem, never by the return value alone.
//!
//! The tests that need git to die or lie mid-move run a child of this test
//! binary behind a `git` shim (`shim.rs`), as `set_aside_kill` does: no
//! fail-point lives in production code.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use graft_core::journal::{JournalEntry, OperationKind};
use graft_core::oid::Oid;
use graft_core::on_disk::{Generation, MIGRATED_MARKER};
use graft_git::commit_graph::{self, Offer};
use graft_git::file_lock::FileLock;
use graft_ops::operation::{BranchCreateParams, OnEdits, Operation, Target};
use graft_ops::rename::{LOCK_FILE, MigrateError, SettleOutcome, Settled, settle_names};

use crate::support::{NOW, TempRepo, journal_for, run, run_undo, state_of};
use graft_core::on_disk::CURRENT_DIR;

mod generations;
mod more;
mod review;
mod shim;

/// Every ref under `prefix`, name to value.
fn refs_under(repo: &TempRepo, prefix: &str) -> BTreeMap<String, String> {
    repo.git(&["for-each-ref", "--format=%(refname) %(objectname)", prefix])
        .lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(name, oid)| (name.to_owned(), oid.to_owned()))
        .collect()
}

/// `refs_under(old)` spelled as the new names would hold it.
fn as_new(old: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    old.iter()
        .map(|(name, oid)| {
            (
                name.replacen("refs/graft/", &format!("refs/{CURRENT_DIR}/"), 1),
                oid.clone(),
            )
        })
        .collect()
}

fn settle(repo: &TempRepo) -> Settled {
    settle_names(&repo.open())
}

fn stage_edit(repo: &TempRepo, text: &str, at: i64) {
    repo.write("a.rs", text.as_bytes());
    run(
        repo,
        &Operation::Stage(Target::Paths(vec![PathBuf::from("a.rs")])),
        at,
    );
}

/// A repository a build before the rename used: no marker, `backups`
/// stages (each with a backup under `refs/graft/backup/main/`) after
/// `branches` branch creates (none), all journaled under `.git/graft/`.
fn old_repo(branches: usize, backups: usize) -> TempRepo {
    let repo = TempRepo::init_on(Generation::Graft);
    repo.write("a.rs", b"one\n");
    repo.commit_all("one");
    let head = Oid::from_hex(&repo.git(&["rev-parse", "HEAD"])).expect("a commit id");
    for n in 0..branches {
        let op = Operation::BranchCreate(BranchCreateParams {
            name: format!("b{n}"),
            at: head,
            checkout: false,
            on_edits: OnEdits::Refuse,
        });
        run(&repo, &op, NOW + i64::try_from(n).expect("small"));
    }
    for n in 0..backups {
        stage_edit(
            &repo,
            &format!("edit {n}\n"),
            NOW + 100 + i64::try_from(n).expect("small"),
        );
    }
    assert_eq!(refs_under(&repo, "refs/graft/").len(), backups);
    assert!(repo.exists(".git/graft/undo-journal.jsonl"));
    repo
}

fn marker(repo: &TempRepo) -> Option<String> {
    std::fs::read_to_string(
        repo.path()
            .join(format!(".git/{CURRENT_DIR}"))
            .join(MIGRATED_MARKER),
    )
    .ok()
}

/// Opened again, the journal holds the 20 entries and the move, the
/// newest's backup read under the new prefix.
fn the_journal_reads_the_move(repo: &TempRepo, files: usize, newest: &JournalEntry) {
    let handle = repo.open();
    assert_eq!(handle.on_disk_names(), Generation::CURRENT);
    let entries = journal_for(&handle).entries().to_vec();
    assert_eq!(entries.len(), 21, "the 20 and the move");
    let moved = entries.last().expect("the move");
    assert_eq!(moved.kind, OperationKind::Migrate);
    assert_eq!(
        moved.description,
        graft_ops::rename::description(Generation::CURRENT, 3, files)
    );
    assert_eq!(
        entries[19].backup_ref,
        newest.backup_ref.as_ref().map(|name| name.replacen(
            "refs/graft/",
            &format!("refs/{CURRENT_DIR}/"),
            1
        )),
        "read under the new prefix"
    );
}

#[test]
fn old_repo_moves_and_undo_still_restores() {
    // Seventeen branch creates and two stages, then a third stage of an
    // edit already on disk: `state` is what `u` must land on.
    let repo = old_repo(17, 2);
    repo.write("a.rs", b"edit 2\n");
    let state = state_of(&repo);
    run(
        &repo,
        &Operation::Stage(Target::Paths(vec![PathBuf::from("a.rs")])),
        NOW + 200,
    );
    let old = refs_under(&repo, "refs/graft/");
    let journal_before = journal_for(&repo.open()).entries().to_vec();
    assert_eq!((journal_before.len(), old.len()), (20, 3));
    let newest = journal_before.last().expect("an entry").clone();

    let settled = settle(&repo);

    let SettleOutcome::Migrated {
        refs,
        files,
        not_journaled,
        kept_aside: None,
    } = settled.outcome
    else {
        panic!("migrated: {:?}", settled.outcome);
    };
    assert_eq!((refs, not_journaled), (3, None));
    assert!(files >= 1, "the journal at least: {files}");
    assert_eq!(settled.names, Generation::CURRENT);
    assert!(settled.hold.is_none());
    assert_eq!(refs_under(&repo, "refs/graft/"), BTreeMap::new());
    assert_eq!(
        refs_under(&repo, &format!("refs/{CURRENT_DIR}/")),
        as_new(&old)
    );
    assert!(!repo.exists(".git/graft"), "the old folder is gone");
    assert_eq!(marker(&repo).as_deref(), Some("clean\n"));
    the_journal_reads_the_move(&repo, files, &newest);

    // `u` steps past the move and undoes the newest stage, byte-exactly.
    let undone = run_undo(&repo, NOW + 1_000, false)
        .unwrap_or_else(|why| panic!("u runs: {why:?}"))
        .entry;
    assert_eq!(undone.reverses.as_ref(), Some(&newest.id));
    assert_eq!(state_of(&repo), state);
}

#[test]
fn read_back_mismatch_keeps_old() {
    let repo = old_repo(0, 2);
    let old = refs_under(&repo, "refs/graft/");
    let journal = repo.read(".git/graft/undo-journal.jsonl");
    let wrong_ref = as_new(&old).into_keys().next().expect("a backup");
    let wrong_oid = repo.git(&["rev-parse", "HEAD"]);

    let outcome = shim::settle_in_child(&repo, &shim::After::Move(&wrong_ref, &wrong_oid), 1);

    assert!(outcome[0].starts_with("KeptOld(ReadBack"), "{outcome:?}");
    assert_eq!(refs_under(&repo, "refs/graft/"), old, "nothing old deleted");
    assert_eq!(repo.read(".git/graft/undo-journal.jsonl"), journal);
    assert_eq!(marker(&repo), None, "no switch");
    assert_eq!(repo.open().on_disk_names(), Generation::Graft);

    // The relaunch moves it: the ref the failed attempt left at another
    // value is the move's own scratch, overwritten.
    let settled = settle(&repo);
    assert!(
        matches!(settled.outcome, SettleOutcome::Migrated { refs: 2, .. }),
        "{:?}",
        settled.outcome
    );
    assert_eq!(
        refs_under(&repo, &format!("refs/{CURRENT_DIR}/")),
        as_new(&old)
    );
    assert_eq!(refs_under(&repo, "refs/graft/"), BTreeMap::new());
}

#[test]
fn killed_after_create_completes_next_open() {
    let repo = old_repo(0, 3);
    let old = refs_under(&repo, "refs/graft/");

    let outcome = shim::settle_in_child(&repo, &shim::After::Kill, 1);
    assert!(outcome.is_empty(), "the child died mid-move: {outcome:?}");
    assert_eq!(
        refs_under(&repo, &format!("refs/{CURRENT_DIR}/")),
        as_new(&old),
        "created"
    );
    assert_eq!(refs_under(&repo, "refs/graft/"), old, "and nothing deleted");
    assert_eq!(marker(&repo), None);

    let settled = settle(&repo);
    assert!(
        matches!(settled.outcome, SettleOutcome::Migrated { refs: 3, .. }),
        "{:?}",
        settled.outcome
    );
    assert_eq!(
        refs_under(&repo, &format!("refs/{CURRENT_DIR}/")),
        as_new(&old),
        "none lost or doubled"
    );
    assert_eq!(refs_under(&repo, "refs/graft/"), BTreeMap::new());
    assert_eq!(marker(&repo).as_deref(), Some("clean\n"));
}

#[test]
fn switched_marker_cleans_leftovers() {
    let repo = old_repo(0, 1);
    let old = refs_under(&repo, "refs/graft/");
    let settled = settle(&repo);
    assert!(matches!(settled.outcome, SettleOutcome::Migrated { .. }));
    // A move killed after the switch: the marker says so, the old ref and
    // a file of the old folder are still there.
    for (name, oid) in &old {
        repo.git(&["update-ref", name, oid]);
    }
    repo.write(".git/graft/layout", b"left\n");
    std::fs::write(
        repo.path()
            .join(format!(".git/{CURRENT_DIR}"))
            .join(MIGRATED_MARKER),
        "switched\n",
    )
    .expect("the marker writes");
    let new_before = refs_under(&repo, &format!("refs/{CURRENT_DIR}/"));

    let settled = settle(&repo);

    assert_eq!(
        settled.outcome,
        SettleOutcome::CleanedUp {
            refs: 1,
            files: 1,
            kept_aside: None
        }
    );
    assert_eq!(settled.names, Generation::CURRENT);
    assert_eq!(refs_under(&repo, "refs/graft/"), BTreeMap::new());
    assert_eq!(
        refs_under(&repo, &format!("refs/{CURRENT_DIR}/")),
        new_before,
        "creates none"
    );
    assert!(!repo.exists(".git/graft"));
    assert_eq!(marker(&repo).as_deref(), Some("clean\n"));
}

#[test]
fn clean_marker_spawns_nothing() {
    let repo = old_repo(0, 1);
    let (outcomes, spawned_second) = shim::settle_twice_counting(&repo);
    assert!(outcomes[0].starts_with("Migrated"), "{outcomes:?}");
    assert_eq!(outcomes[1], "Done");
    assert_eq!(
        spawned_second,
        Vec::<String>::new(),
        "the second open spawned git"
    );

    // Ticket 48 Part B: empty folders under every earlier generation's names
    // (what git leaves of a deleted ref, what an older build made and left)
    // hold nothing to settle; `clean` still opens with no spawn.
    for older in graft_core::on_disk::Generation::CURRENT.older() {
        let dir = older.dir();
        for empty in [format!(".git/{dir}"), format!(".git/refs/{dir}/a")] {
            std::fs::create_dir_all(repo.path().join(empty)).expect("the folder is made");
        }
    }
    let (outcomes, spawned) = shim::settle_once_counting(&repo);
    assert_eq!(outcomes, ["Done"]);
    assert_eq!(spawned, Vec::<String>::new(), "an empty folder spawned git");
}

#[test]
fn old_names_hold_blocks_migration() {
    let repo = old_repo(0, 1);
    let old = refs_under(&repo, "refs/graft/");
    let dir = repo.path().join(format!(".git/{CURRENT_DIR}"));
    std::fs::create_dir_all(&dir).expect("the dir is made");
    // Another window, still on the old names, holds the lock shared.
    let other = FileLock::shared(&dir.join(LOCK_FILE)).expect("the lock is shared");

    let blocked = settle(&repo);
    assert_eq!(blocked.outcome, SettleOutcome::KeptOld(MigrateError::Busy));
    assert_eq!(blocked.names, Generation::Graft);
    assert!(
        blocked.hold.is_some(),
        "this run holds it too, for its life"
    );
    assert_eq!(refs_under(&repo, "refs/graft/"), old);
    assert_eq!(
        MigrateError::Busy.to_string(),
        "another Mergewright window is open on this repository"
    );

    drop(other);
    drop(blocked);
    let settled = settle(&repo);
    assert!(
        matches!(settled.outcome, SettleOutcome::Migrated { refs: 1, .. }),
        "{:?}",
        settled.outcome
    );
}

#[test]
fn decline_read_from_new_dir() {
    let repo = old_repo(0, 1);
    commit_graph::remember_declined(&repo.open()).expect("the decline writes");
    assert!(repo.exists(".git/graft/commit-graph-offer"));

    assert!(matches!(
        settle(&repo).outcome,
        SettleOutcome::Migrated { .. }
    ));

    let handle = repo.open();
    assert_eq!(handle.on_disk_names(), Generation::CURRENT);
    assert_eq!(
        commit_graph::offer_state(&handle).expect("reads"),
        Offer::Declined
    );
}

#[test]
fn fsck_clean_after() {
    let repo = old_repo(2, 2);
    assert!(matches!(
        settle(&repo).outcome,
        SettleOutcome::Migrated { .. }
    ));
    let fsck = repo
        .try_git(&["fsck", "--no-progress", "--strict"])
        .unwrap_or_else(|why| panic!("fsck fails: {why}"));
    for word in ["missing", "broken", "error"] {
        assert!(!fsck.contains(word), "{fsck}");
    }
}

/// A migrated journal still refuses nothing it did before: an operation
/// after the move journals under the new names.
#[test]
fn an_operation_after_the_move_writes_the_new_names() {
    let repo = old_repo(0, 1);
    assert!(matches!(
        settle(&repo).outcome,
        SettleOutcome::Migrated { .. }
    ));
    stage_edit(&repo, "after\n", NOW + 500);
    let entries = journal_for(&repo.open()).entries().to_vec();
    let last = entries.last().expect("the stage");
    assert!(
        last.backup_ref
            .as_deref()
            .is_some_and(|name| name.starts_with(&format!("refs/{CURRENT_DIR}/backup/main/"))),
        "{last:?}"
    );
    assert!(!repo.exists(".git/graft"));
}
