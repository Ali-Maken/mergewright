//! A journal entry written before the rename names a backup under a prefix
//! the app no longer reads (pre-publish-cleanup/11b3, ADR-0057 Amendment §7).
//! Journal lines are read as written: the entry still lists, and `u` on it
//! refuses with words and writes nothing once the backup's objects are
//! gone (a `git gc` after the move). While they remain, `u` still restores
//! from the oids the entry records; that is not this test's subject.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::PathBuf;

use mergewright_core::on_disk::REF_PREFIX;
use mergewright_ops::journal::{Journal, journal_path};
use mergewright_ops::operation::{Operation, Target};

use crate::support::{Letters, NOW, TempRepo, run, run_undo, state_of};

#[test]
fn journal_reads_unmapped_ref_entry() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"fn a() {}\n");
    repo.commit_all("one");
    repo.write("a.rs", b"fn a() { changed }\n");
    let entry = run(
        &repo,
        &Operation::Discard(Target::Paths(vec![PathBuf::from("a.rs")])),
        NOW,
    );
    let backup = entry.backup_ref.expect("a discard takes a backup");
    // The backup the entry names is gone, as the move left it, and the
    // entry spells it under a prefix that is not this build's.
    // Its objects are pruned too, as a later `git gc` leaves them: nothing
    // else holds the discarded bytes.
    repo.git(&["update-ref", "-d", &backup]);
    repo.git(&["reflog", "expire", "--expire=now", "--all"]);
    repo.git(&["gc", "-q", "--prune=now"]);
    let elsewhere = backup.replacen(REF_PREFIX, "refs/elsewhere/", 1);
    assert_ne!(elsewhere, backup);
    let path = journal_path(&repo.open()).expect("resolves");
    let text = std::fs::read_to_string(&path).expect("reads");
    assert!(text.contains(&backup));
    std::fs::write(&path, text.replace(&backup, &elsewhere)).expect("writes");
    let journal_before = std::fs::read(&path).expect("reads");
    let state_before = state_of(&repo);
    let refs_before = repo.git(&["for-each-ref", "--format=%(refname) %(objectname)"]);

    let journal = Journal::open(&path).expect("the journal loads");
    let listed = journal
        .entries()
        .iter()
        .find(|e| e.id == entry.id)
        .expect("the entry lists");
    assert_eq!(listed.backup_ref.as_deref(), Some(elsewhere.as_str()));

    let why = run_undo(&repo, NOW + 1, false).expect_err("u refuses");
    let words = why.explain(&Letters);
    println!("refusal: {words}");
    assert!(!words.is_empty());

    assert_eq!(std::fs::read(&path).expect("reads"), journal_before);
    assert_eq!(state_of(&repo), state_before);
    assert_eq!(
        repo.git(&["for-each-ref", "--format=%(refname) %(objectname)"]),
        refs_before
    );
}
