//! The journal file and the backup refs take the repository's on-disk names
//! in force (ADR-0057, Amendment §4; m8-polish-and-ship/26 Part A). With no
//! marker every write lands where every earlier build put it; with the
//! marker, under the new names, and `u` still puts the write back.
//!
//! Judged by reading the repository back with `git` and the filesystem.

use std::path::PathBuf;

use graft_core::on_disk::{Generation, MIGRATED_MARKER};
use graft_ops::journal::journal_path;
use graft_ops::operation::{Operation, Target};

use crate::support::{NOW, TempRepo, run, run_undo};

fn stage_a(repo: &TempRepo) -> graft_core::journal::JournalEntry {
    run(
        repo,
        &Operation::Stage(Target::Paths(vec![PathBuf::from("a.rs")])),
        NOW,
    )
}

fn edited(repo: TempRepo) -> TempRepo {
    repo.write("a.rs", b"fn a() {}\n");
    repo.commit_all("one");
    repo.write("a.rs", b"fn a() { changed }\n");
    repo
}

#[test]
fn no_marker_writes_the_old_names() {
    let repo = edited(TempRepo::init_on(Generation::Graft));
    let entry = stage_a(&repo);

    let backup = entry.backup_ref.expect("a stage takes a backup");
    assert!(backup.starts_with("refs/graft/backup/main/"), "{backup}");
    assert_eq!(
        repo.git(&["rev-parse", "--verify", "-q", &backup]).len(),
        40
    );
    let journal = journal_path(&repo.open()).expect("resolves");
    assert!(
        journal.ends_with(".git/graft/undo-journal.jsonl"),
        "{}",
        journal.display()
    );
    assert!(journal.is_file());
}

#[test]
fn the_marker_writes_the_new_names_and_u_still_restores() {
    let repo = edited(TempRepo::init_on(Generation::Graft));
    let marker_dir = repo.path().join(".git").join(Generation::CURRENT.dir());
    std::fs::create_dir_all(&marker_dir).expect("makes the dir");
    std::fs::write(marker_dir.join(MIGRATED_MARKER), "clean\n").expect("writes");
    let index_before = repo.git(&["ls-files", "-s"]);

    let entry = stage_a(&repo);

    let backup = entry.backup_ref.clone().expect("a stage takes a backup");
    assert!(
        backup.starts_with("refs/tributary/backup/main/"),
        "{backup}"
    );
    assert_eq!(
        repo.git(&["rev-parse", "--verify", "-q", &backup]).len(),
        40
    );
    assert_eq!(
        repo.git(&["for-each-ref", "refs/graft/"]),
        "",
        "nothing under the old prefix"
    );
    let journal = journal_path(&repo.open()).expect("resolves");
    assert!(
        journal.ends_with(".git/tributary/undo-journal.jsonl"),
        "{}",
        journal.display()
    );
    assert!(journal.is_file());
    assert!(!repo.path().join(".git/graft").exists(), "no old dir made");

    run_undo(&repo, NOW + 1, false).expect("u runs");
    assert_eq!(repo.git(&["ls-files", "-s"]), index_before);
}
