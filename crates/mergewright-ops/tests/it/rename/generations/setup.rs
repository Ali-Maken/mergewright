//! Setup and checks shared by the generation tests: refs and folders read
//! per generation, a Tributary-era repository built by hand, and the
//! journal lines an earlier build wrote.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mergewright_core::journal::{EntryId, EntryStatus, JournalEntry, OperationKind, Touches};
use mergewright_core::on_disk::{Generation, MIGRATED_MARKER};
use mergewright_ops::journal::Journal;
use mergewright_ops::rename::{KEPT_ASIDE_DIR, LOCK_FILE, description};

use super::super::refs_under;
use crate::support::{NOW, RepoState, TempRepo, run_undo, state_of};

pub(super) const JOURNAL: &str = "undo-journal.jsonl";

/// `refs_under(repo, from)` as `to`'s prefix spells it.
pub(super) fn moved(old: &BTreeMap<String, String>, to: Generation) -> BTreeMap<String, String> {
    old.iter()
        .map(|(name, oid)| {
            let generation = Generation::of_ref(name).expect("one of ours");
            (
                name.replacen(generation.ref_prefix(), to.ref_prefix(), 1),
                oid.clone(),
            )
        })
        .collect()
}

pub(super) fn refs_of(repo: &TempRepo, generation: Generation) -> BTreeMap<String, String> {
    refs_under(repo, generation.ref_prefix())
}

/// `.git/<generation's dir>/<path>`.
pub(super) fn in_dir(generation: Generation, path: &str) -> String {
    format!(".git/{}/{path}", generation.dir())
}

pub(super) fn marker_of(repo: &TempRepo, generation: Generation) -> Option<String> {
    std::fs::read_to_string(repo.path().join(in_dir(generation, MIGRATED_MARKER))).ok()
}

pub(super) fn fsck_is_clean(repo: &TempRepo) {
    let fsck = repo
        .try_git(&["fsck", "--no-progress", "--strict"])
        .unwrap_or_else(|why| panic!("fsck fails: {why}"));
    for word in ["missing", "broken", "error"] {
        assert!(!fsck.contains(word), "{fsck}");
    }
}

/// Every folder named `name` anywhere under `.git`.
pub(super) fn folders_named(repo: &TempRepo, name: &str) -> Vec<PathBuf> {
    fn walk(dir: &Path, name: &str, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("reads") {
            let path = entry.expect("an entry").path();
            // The kept-aside folders hold earlier generations' names on
            // purpose (`older-version/<n>/graft/...`): they are kept, not
            // left over.
            if path.is_dir() && path.file_name().is_none_or(|file| file != KEPT_ASIDE_DIR) {
                if path.file_name().is_some_and(|file| file == name) {
                    found.push(path.clone());
                }
                walk(&path, name, found);
            }
        }
    }
    let mut found = Vec::new();
    walk(&repo.path().join(".git"), name, &mut found);
    found
}

/// No ref under an earlier generation's prefix, and no folder of its name
/// outside the kept-aside ones.
pub(super) fn no_earlier_names_left(repo: &TempRepo) {
    for older in Generation::CURRENT.older() {
        assert_eq!(refs_of(repo, older), BTreeMap::new(), "{older:?}");
        assert_eq!(folders_named(repo, older.dir()), Vec::<PathBuf>::new());
    }
}

/// `u` after the move reverses `newest`, the last entry before it, and
/// puts the repository back byte-exactly to `state`; `fsck` clean.
pub(super) fn undo_puts_back(repo: &TempRepo, newest: &JournalEntry, state: &RepoState) {
    let undone = run_undo(repo, NOW + 1_000, false)
        .unwrap_or_else(|why| panic!("u runs: {why:?}"))
        .entry;
    assert_eq!(undone.reverses.as_ref(), Some(&newest.id));
    assert_eq!(&state_of(repo), state, "byte-exactly");
    fsck_is_clean(repo);
}

/// Each `Migrate` entry's place in the journal and its stored words.
pub(super) fn migrate_rows(entries: &[JournalEntry]) -> Vec<(usize, String)> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.kind == OperationKind::Migrate)
        .map(|(at, entry)| (at, entry.description.clone()))
        .collect()
}

pub(super) fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the folder is made");
    for entry in std::fs::read_dir(from).expect("reads") {
        let path = entry.expect("an entry").path();
        let target = to.join(path.file_name().expect("a name"));
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            std::fs::copy(&path, &target).expect("copies");
        }
    }
}

/// The `Migrate` entry a build of `to` appended when it moved `refs` refs
/// and `files` files, both lines, as `rename/record.rs` writes it.
pub(super) fn append_migrate(repo: &TempRepo, to: Generation, refs: usize, files: usize, at: i64) {
    let path = repo.path().join(in_dir(to, JOURNAL));
    let journal = Journal::open(&path).expect("the journal opens");
    let last = journal.entries().last().expect("an entry").clone();
    let head = last.head_after.clone().unwrap_or(last.head_before);
    let mut entry = JournalEntry {
        id: EntryId::new(at, [9; 10]),
        at_unix_ms: at,
        kind: OperationKind::Migrate,
        description: description(to, refs, files),
        status: EntryStatus::Pending,
        touches: Touches::default(),
        head_after: Some(head.clone()),
        head_before: head,
        refs_before: BTreeMap::new(),
        refs_after: BTreeMap::new(),
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        worktree_raw_before: None,
        backup_ref: None,
        pins: Vec::new(),
        commands: Vec::new(),
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: BTreeMap::new(),
        config_after: BTreeMap::new(),
        put_back: None,
        fetched_refs: None,
    };
    let mut lines = mergewright_ops::journal::render_pending(&entry).expect("renders");
    entry.status = EntryStatus::Committed;
    lines.extend(mergewright_ops::journal::render_outcome(&entry).expect("renders"));
    let mut bytes = std::fs::read(&path).expect("the journal reads");
    bytes.extend(lines);
    std::fs::write(&path, bytes).expect("the journal writes");
}

/// What a Tributary build left after moving a Mergewright repository: the refs
/// and the folder copied to Tributary's names, its `Migrate` entry, its
/// lock, and its marker reading `marker`. With `keep_graft` the Mergewright
/// copies stay (a `switched` move killed before its clean-up).
pub(super) fn as_tributary(repo: &TempRepo, marker: &str, keep_graft: bool) {
    let (graft, tributary) = (Generation::Graft, Generation::Tributary);
    let old = refs_of(repo, graft);
    let mut lines = String::new();
    for (name, oid) in moved(&old, tributary) {
        lines.push_str(&format!("create {name} {oid}\n"));
    }
    if !keep_graft {
        for (name, oid) in &old {
            lines.push_str(&format!("delete {name} {oid}\n"));
        }
    }
    let stdin = std::process::Command::new("git")
        .args(["update-ref", "--stdin"])
        .current_dir(repo.path())
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .expect("stdin")
                .write_all(lines.as_bytes())?;
            child.wait()
        })
        .expect("update-ref runs");
    assert!(stdin.success());
    let (from, to) = (
        repo.path().join(".git").join(graft.dir()),
        repo.path().join(".git").join(tributary.dir()),
    );
    copy_dir(&from, &to);
    if !keep_graft {
        std::fs::remove_dir_all(&from).expect("the Mergewright folder goes");
    }
    append_migrate(repo, tributary, old.len(), 1, NOW + 150);
    repo.write(&in_dir(tributary, LOCK_FILE), b"");
    repo.write(&in_dir(tributary, MIGRATED_MARKER), marker.as_bytes());
    assert_eq!(repo.open().on_disk_names(), tributary);
}

/// A commit no ref holds, HEAD unmoved.
pub(super) fn loose_commit(repo: &TempRepo) -> String {
    repo.git(&["commit-tree", "HEAD^{tree}", "-p", "HEAD", "-m", "theirs"])
}
