//! `a` on a merge **begun in the shell** (m6-conflicts/57): no entry of
//! Graft's behind it, so `resume(Abort)` aborts it as an operation of its
//! own — `Operation::AbortMerge`, `git merge --abort` with a full backup —
//! and `u` puts the merge back as it stood: the stages, the hand edits,
//! git's merge files.
//!
//! Every merge here is started with plain `git`, never through Graft, and
//! every assertion is read back out of git or the disk.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::{Path, PathBuf};

use graft_core::journal::{EntryStatus, JournalEntry, OperationKind};
use graft_core::oid::Oid;
use graft_ops::apply::Resume;

use crate::conflict_fixtures::run_resume;
use crate::conflict_ops::journal_ids;
use crate::support::{NOW, TempRepo, journal_for, run_undo};

/// A hand edit to `notes.txt`, made over git's markers before the abort.
const HAND_EDIT: &[u8] = b"one\nmy own two\nthree\nfour\nfive\nsix\nmy own seven\neight\n";

/// `notes.txt` at the fork: two lines the sides both rewrite.
const NOTES_BASE: &[u8] = b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\n";

/// The walk's `two_files`, built here: `gone.txt` changed on `main` and
/// deleted on `feature` (deleted by them); `notes.txt` rewritten on both
/// sides (both modified). `main` checked out, nothing merged yet.
pub fn two_files() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("gone.txt", b"gone\n");
    repo.write("notes.txt", NOTES_BASE);
    repo.commit_all("base");
    repo.git(&["branch", "feature"]);
    repo.write("gone.txt", b"gone, changed on main\n");
    repo.write(
        "notes.txt",
        b"one\ntwo from main\nthree\nfour\nfive\nsix\nseven from main\neight\n",
    );
    repo.commit_all("main's side");
    repo.git(&["switch", "-q", "feature"]);
    repo.git(&["rm", "-q", "--", "gone.txt"]);
    repo.write(
        "notes.txt",
        b"one\nTWO from feature\nthree\nfour\nfive\nsix\nSEVEN from feature\neight\n",
    );
    repo.commit_all("feature's side");
    repo.git(&["switch", "-q", "main"]);
    repo
}

/// `git merge feature` in the shell, stopped on `notes.txt` and `gone.txt`.
/// Graft journaled nothing.
pub fn shell_merge_stopped() -> TempRepo {
    let repo = two_files();
    stop_the_merge(&repo);
    repo
}

pub fn stop_the_merge(repo: &TempRepo) {
    assert!(
        repo.try_git(&["merge", "--no-edit", "feature"]).is_err(),
        "the merge is meant to stop"
    );
    assert_eq!(
        repo.git(&["diff", "--name-only", "--diff-filter=U"]),
        "gone.txt\nnotes.txt",
        "stopped on both files"
    );
    assert!(journal_ids(repo).is_empty(), "Graft journaled nothing");
}

/// Where git keeps one of its files: `rev-parse --git-path`.
pub fn git_path(repo: &TempRepo, name: &str) -> PathBuf {
    let printed = repo.git(&["rev-parse", "--git-path", name]);
    let path = Path::new(&printed);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo.path().join(path)
    }
}

/// A file's bytes, or `None` when it is absent.
fn maybe_read(path: &Path) -> Option<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
        Err(why) => panic!("{}: {why}", path.display()),
    }
}

/// The three merge files Graft models (`MergeState`), each at its
/// `--git-path`, byte for byte.
pub fn merge_files(repo: &TempRepo) -> Vec<(&'static str, Option<Vec<u8>>)> {
    ["MERGE_HEAD", "MERGE_MSG", "MERGE_MODE"]
        .into_iter()
        .map(|name| (name, maybe_read(&git_path(repo, name))))
        .collect()
}

pub fn oid(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("git prints an object id")
}

pub fn merging(repo: &TempRepo) -> bool {
    repo.try_git(&["rev-parse", "-q", "--verify", "MERGE_HEAD"])
        .is_ok()
}

pub fn entries(repo: &TempRepo) -> Vec<JournalEntry> {
    journal_for(&repo.open()).entries().to_vec()
}

pub fn abort(repo: &TempRepo, at: i64) -> JournalEntry {
    run_resume(repo, Resume::Abort, at)
        .unwrap_or_else(|why| panic!("abort: {}", why.explain(&crate::support::Letters)))
        .entry
}

/// What the abort's own entry says: its kind and words, the merge before
/// and none after, a full backup, and `git merge --abort` in the log.
fn an_abort_entry(entry: &JournalEntry) {
    assert_eq!(entry.kind, OperationKind::AbortMerge);
    assert_eq!(entry.status, EntryStatus::Committed);
    assert_eq!(entry.description, "abort merge of feature");
    assert!(entry.merge_before.is_some(), "the merge read before");
    assert_eq!(entry.merge_after, None, "and gone after");
    assert!(entry.backup_ref.is_some(), "a full backup of its own");
    assert!(
        entry
            .commands
            .iter()
            .any(|line| line.ends_with("merge --abort")),
        "the Command Log: {:?}",
        entry.commands
    );
}

// ── the abort, and its undo ─────────────────────────────────────────────

/// The M3 promise across `a`: aborting a shell merge is one journaled
/// `AbortMerge` entry, and `u` puts back the merge files byte for byte,
/// both paths' stages, and the hand edit — with `HEAD` never moving.
#[test]
fn abort_of_a_shell_merge_is_journaled_and_undoable() {
    let repo = shell_merge_stopped();
    repo.write("notes.txt", HAND_EDIT);
    let head = oid(&repo, "HEAD");
    let files = merge_files(&repo);
    assert!(
        files.iter().all(|(_, bytes)| bytes.is_some()),
        "the fixture has all three merge files: {files:?}"
    );
    let stages = repo.git(&["ls-files", "-u"]);
    assert_eq!(
        stages.lines().count(),
        5,
        "notes.txt three stages, gone.txt base and ours: {stages}"
    );

    let entry = abort(&repo, NOW);
    assert!(!merging(&repo), "MERGE_HEAD is gone");
    assert_eq!(repo.git(&["ls-files", "-u"]), "", "nothing unmerged");
    assert_eq!(journal_ids(&repo), vec![entry.id.to_string()], "one entry");
    an_abort_entry(&entry);
    assert_eq!(oid(&repo, "HEAD"), head, "the abort leaves HEAD");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(oid(&repo, "MERGE_HEAD"), oid(&repo, "feature"));
    assert_eq!(merge_files(&repo), files, "the merge files, byte for byte");
    assert_eq!(
        repo.git(&["ls-files", "-u"]),
        stages,
        "both paths' stages, the same oids and modes"
    );
    assert_eq!(
        repo.read("notes.txt"),
        HAND_EDIT,
        "the hand edit, byte for byte"
    );
    assert_eq!(oid(&repo, "HEAD"), head, "HEAD before = after the undo");
    assert_eq!(
        repo.git(&["symbolic-ref", "HEAD"]),
        "refs/heads/main",
        "still on the branch"
    );
    assert_eq!(entries(&repo).len(), 2, "the undo is an entry of its own");
}

/// A path he had already resolved and staged before the abort comes back
/// resolved: its stage-0 entry sits in the same index as the other path's
/// stages, and the file holds his resolution.
#[test]
fn undo_of_an_abort_keeps_a_file_already_resolved() {
    let repo = shell_merge_stopped();
    repo.write("notes.txt", HAND_EDIT);
    repo.git(&["add", "--", "notes.txt"]);
    let index = repo.git(&["ls-files", "--stage"]);
    assert_eq!(
        repo.git(&["diff", "--name-only", "--diff-filter=U"]),
        "gone.txt",
        "notes.txt resolved, gone.txt still unmerged"
    );
    let resolved = repo.git(&["ls-files", "--stage", "--", "notes.txt"]);
    assert!(resolved.contains(" 0\tnotes.txt"), "stage 0: {resolved}");

    abort(&repo, NOW);
    assert!(!merging(&repo));

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert!(merging(&repo), "the merge is back");
    assert_eq!(
        repo.git(&["ls-files", "--stage"]),
        index,
        "the whole index: notes.txt at stage 0, gone.txt's stages"
    );
    assert_eq!(
        repo.git(&["diff", "--name-only", "--diff-filter=U"]),
        "gone.txt",
        "only gone.txt unmerged again"
    );
    assert_eq!(repo.read("notes.txt"), HAND_EDIT, "his resolution on disk");
}

/// The abort's entry survives the journal file: written as
/// `"kind":"abort_merge"`, parsed back equal to what `apply` returned, and
/// rendered back byte-equal to the file.
#[test]
fn journal_round_trips_an_abort_merge_entry() {
    let repo = shell_merge_stopped();
    let entry = abort(&repo, NOW);

    let path = graft_ops::journal::journal_path(&repo.open()).expect("the journal path");
    let written = std::fs::read(&path).expect("the journal reads");
    assert!(
        String::from_utf8_lossy(&written).contains("\"kind\":\"abort_merge\""),
        "{}",
        String::from_utf8_lossy(&written)
    );
    let parsed = graft_ops::journal::parse(&written).expect("the journal parses");
    assert!(parsed.torn_tail.is_none());
    assert_eq!(parsed.entries, vec![entry.clone()], "read back equal");

    let mut rendered = graft_ops::journal::render_pending(&entry).expect("the pending line");
    rendered.extend(graft_ops::journal::render_outcome(&entry).expect("the outcome line"));
    assert_eq!(
        String::from_utf8_lossy(&rendered),
        String::from_utf8_lossy(&written),
        "rendered back as it was written"
    );
}

/// `U` after `u` of an abort: the entry `U` reverses is the undo, recorded
/// inside the merge it put back — the merge standing now — so the gate
/// lets it through, and the redo aborts the merge again, as its own entry.
#[test]
fn redo_of_an_undone_abort_is_allowed_while_the_merge_is_back() {
    let repo = shell_merge_stopped();
    repo.write("notes.txt", HAND_EDIT);
    abort(&repo, NOW);
    let clean = repo.git(&["status", "--porcelain=v2"]);
    let head = oid(&repo, "HEAD");
    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert!(merging(&repo), "the merge is back");

    let redone = crate::support::run_redo(&repo, NOW + 2, false).unwrap_or_else(|why| {
        panic!(
            "redo while the merge is back: {}",
            why.explain(&crate::support::Letters)
        )
    });
    assert_eq!(redone.entry.kind, OperationKind::Redo);
    assert!(!merging(&repo), "aborted again");
    assert_eq!(repo.git(&["ls-files", "-u"]), "", "nothing unmerged");
    assert_eq!(
        repo.git(&["status", "--porcelain=v2"]),
        clean,
        "as the abort left it"
    );
    assert_eq!(oid(&repo, "HEAD"), head, "HEAD never moves");
    assert_eq!(entries(&repo).len(), 3, "abort, undo, redo");
}

/// Every git file under the repository's git dir that is not a directory
/// Graft keeps (`graft/`, `objects/`, `refs/`, `logs/`, `hooks/`, `info/`)
/// nor the index, with its bytes — plus `rr-cache/`, walked. What a merge
/// and its abort leave or take away, and what `u` does or does not put back.
fn git_state_files(repo: &TempRepo) -> std::collections::BTreeMap<String, Vec<u8>> {
    const SKIP: [&str; 9] = [
        "graft",
        "objects",
        "refs",
        "logs",
        "hooks",
        "info",
        "index",
        "config",
        "description",
    ];
    let git_dir = git_path(repo, ".");
    let mut found = std::collections::BTreeMap::new();
    let mut pending = vec![git_dir.clone()];
    while let Some(dir) = pending.pop() {
        for item in std::fs::read_dir(&dir).expect("the git dir reads") {
            let item = item.expect("a directory entry");
            let path = item.path();
            let name = path
                .strip_prefix(&git_dir)
                .expect("under the git dir")
                .to_string_lossy()
                .into_owned();
            if dir == git_dir && SKIP.contains(&name.as_str()) {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else {
                found.insert(name, std::fs::read(&path).expect("the file reads"));
            }
        }
    }
    found
}

/// `rerere.enabled=true`: git's merge writes files `MergeState` does not
/// model. After the abort and its undo, exactly these are missing — the
/// known limit the ticket's table names — and nothing else differs:
///
/// - `MERGE_RR` — rerere's map from each conflicted path to its
///   `rr-cache/<id>/`. Without it, a resolution he records after the undo
///   is not remembered by rerere (the preimage itself stays in `rr-cache/`).
/// - `AUTO_MERGE` — the tree git's ort merge wrote with its conflict
///   markers (`git diff AUTO_MERGE`), when this git writes one.
///
/// Observed on git 2.54: `ORIG_HEAD`, `COMMIT_EDITMSG` and the rr-cache
/// preimage stay byte-equal throughout.
#[test]
fn undo_of_an_abort_restores_exactly_the_merge_files_graft_knows() {
    let repo = two_files();
    repo.git(&["config", "rerere.enabled", "true"]);
    stop_the_merge(&repo);
    let before = git_state_files(&repo);
    assert!(before.contains_key("MERGE_RR"), "rerere is on: {before:?}");
    assert!(
        before.keys().any(|name| name.starts_with("rr-cache/")),
        "rerere kept a preimage"
    );

    abort(&repo, NOW);
    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    let after = git_state_files(&repo);

    let missing: Vec<&str> = before
        .keys()
        .filter(|name| !after.contains_key(*name))
        .map(String::as_str)
        .collect();
    let changed: Vec<&str> = before
        .iter()
        .filter(|(name, bytes)| after.get(*name).is_some_and(|now| now != *bytes))
        .map(|(name, _)| name.as_str())
        .collect();
    let added: Vec<&str> = after
        .keys()
        .filter(|name| !before.contains_key(*name))
        .map(String::as_str)
        .collect();
    // AUTO_MERGE depends on the git on PATH; when git wrote it, it does not
    // come back either.
    let mut expected = Vec::new();
    if before.contains_key("AUTO_MERGE") {
        expected.push("AUTO_MERGE");
    }
    expected.push("MERGE_RR");
    assert_eq!(missing, expected, "what u does not put back");
    assert!(changed.is_empty(), "changed under the git dir: {changed:?}");
    assert!(added.is_empty(), "added under the git dir: {added:?}");
}
