//! Parity fence (pre-publish-cleanup/11b3, ADR-0057 Amendment §7): a journal
//! entry naming its backup under a prefix this build doesn't read
//! (`refs/elsewhere/`) undoes exactly as it did before the ref mapping and
//! the old-tag readers went. Observed values, not desired ones.
//!
//! Open question: undo of a pruned entry leaves a pending line and a backup
//! ref before failing; question filed (gap pp/11b3-undo-pruned-discard), not
//! fixed here.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::{Path, PathBuf};

use mergewright_core::journal::JournalEntry;
use mergewright_core::on_disk::REF_PREFIX;
use mergewright_ops::journal::{Journal, journal_path};
use mergewright_ops::operation::{Operation, Target};
use mergewright_ops::undo::UndoError;

use crate::support::{Letters, NOW, TempRepo, run, run_undo};

/// A repo with `a.rs` changed, `op` run on it, its backup ref deleted and
/// the journal rewritten to name it under `refs/elsewhere/`.
fn unmapped(op: &Operation, prune: bool) -> (TempRepo, JournalEntry, PathBuf) {
    let repo = TempRepo::init();
    repo.write("a.rs", b"fn a() {}\n");
    repo.commit_all("one");
    repo.write("a.rs", b"fn a() { changed }\n");
    let entry = run(&repo, op, NOW);
    let backup = entry.backup_ref.clone().expect("the op takes a backup");
    repo.git(&["update-ref", "-d", &backup]);
    if prune {
        repo.git(&["reflog", "expire", "--expire=now", "--all"]);
        repo.git(&["gc", "-q", "--prune=now"]);
    }
    let elsewhere = backup.replacen(REF_PREFIX, "refs/elsewhere/", 1);
    assert_ne!(elsewhere, backup);
    let path = journal_path(&repo.open()).expect("resolves");
    let text = std::fs::read_to_string(&path).expect("reads");
    std::fs::write(&path, text.replace(&backup, &elsewhere)).expect("writes");
    let journal = Journal::open(&path).expect("the journal loads");
    let listed = journal
        .entries()
        .iter()
        .find(|e| e.id == entry.id)
        .expect("the entry lists");
    assert_eq!(listed.backup_ref.as_deref(), Some(elsewhere.as_str()));
    (repo, entry, path)
}

fn own_refs(repo: &TempRepo) -> usize {
    repo.git(&["for-each-ref", "--format=%(refname)", REF_PREFIX])
        .lines()
        .count()
}

fn lines(path: &Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(path)
        .expect("reads")
        .lines()
        .map(|l| serde_json::from_str(l).expect("a json line"))
        .collect()
}

#[test]
fn ref_gone_entry_undoes_as_today() {
    let op = Operation::Stage(Target::Paths(vec![PathBuf::from("a.rs")]));
    let (repo, _, _) = unmapped(&op, false);

    run_undo(&repo, NOW + 1, false).expect("u undoes from the recorded oids");

    assert_eq!(repo.git(&["ls-files", "-s"]), {
        // The index as the commit left it: `a.rs` unstaged again.
        repo.git(&[
            "ls-tree",
            "-r",
            "--format=%(objectmode) %(objectname) 0\t%(path)",
            "HEAD",
        ])
    });
    assert_eq!(
        std::fs::read(repo.path().join("a.rs")).expect("reads"),
        b"fn a() { changed }\n"
    );
}

#[test]
fn pruned_entry_fails_as_today() {
    let op = Operation::Discard(Target::Paths(vec![PathBuf::from("a.rs")]));
    let (repo, _, path) = unmapped(&op, true);
    let lines_before = lines(&path).len();
    let refs_before = own_refs(&repo);

    let why = run_undo(&repo, NOW + 1, false).expect_err("u fails");

    assert!(matches!(why, UndoError::Restore(_)), "{why:?}");
    let words = why.explain(&Letters);
    assert!(words.contains("fatal: bad object"), "{words}");
    assert!(words.contains("rolled back part-way"), "{words}");
    assert!(words.contains("U restores the backup"), "{words}");

    let after = lines(&path);
    let appended = &after[lines_before..];
    // A pending undo line, then its resolution: aborted, same id.
    assert_eq!(appended.len(), 2, "{appended:?}");
    assert_eq!(appended[0]["kind"], "undo");
    assert_eq!(appended[0]["status"], "pending");
    assert_eq!(appended[1]["id"], appended[0]["id"]);
    assert!(appended[1]["status"]["aborted"].is_object(), "{appended:?}");
    assert_eq!(own_refs(&repo), refs_before + 1, "one new backup ref");
}
