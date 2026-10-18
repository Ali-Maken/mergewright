//! A repository a Tributary build moved off the first names and left there
//! (marker `clean`): S2's starting point, written the way mergewright-ops'
//! `rename/generations/setup.rs` writes it.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};

use mergewright_core::journal::{EntryId, EntryStatus, JournalEntry, OperationKind, Touches};
use mergewright_core::on_disk::{Generation, MIGRATED_MARKER};
use mergewright_ops::journal::Journal;

use super::older::{JOURNAL, at, in_dir, moved, open, refs_of};

/// What a Tributary build left after moving a Mergewright repository and before
/// this build: Mergewright's refs and folder copied to Tributary's names, its
/// `rename to Tributary` entry, its lock file and its marker `clean`.
pub fn as_tributary(repo: &Path) {
    let (graft, tributary) = (Generation::Graft, Generation::Tributary);
    let old = refs_of(repo, graft);
    let mut lines = String::new();
    for (name, oid) in moved(&old, tributary) {
        lines.push_str(&format!("create {name} {oid}\n"));
    }
    for (name, oid) in &old {
        lines.push_str(&format!("delete {name} {oid}\n"));
    }
    let mut child = Command::new("git")
        .args(["update-ref", "--stdin"])
        .current_dir(repo)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap_or_else(|why| panic!("update-ref: {why}"));
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().unwrap_or_else(|| panic!("stdin"));
        stdin
            .write_all(lines.as_bytes())
            .unwrap_or_else(|why| panic!("{why}"));
    }
    let status = child.wait().unwrap_or_else(|why| panic!("{why}"));
    assert!(status.success(), "update-ref");
    let (from, to) = (
        repo.join(".git").join(graft.dir()),
        repo.join(".git").join(tributary.dir()),
    );
    copy_dir(&from, &to);
    std::fs::remove_dir_all(&from).unwrap_or_else(|why| panic!("{why}"));
    append_migrate(repo, tributary, old.len());
    std::fs::write(
        in_dir(repo, tributary, mergewright_ops::rename::LOCK_FILE),
        b"",
    )
    .unwrap_or_else(|why| panic!("{why}"));
    std::fs::write(in_dir(repo, tributary, MIGRATED_MARKER), b"clean\n")
        .unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(open(repo).on_disk_names(), tributary);
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap_or_else(|why| panic!("{why}"));
    for entry in std::fs::read_dir(from).unwrap_or_else(|why| panic!("{why}")) {
        let path = entry.unwrap_or_else(|why| panic!("{why}")).path();
        let target = to.join(path.file_name().unwrap_or_else(|| panic!("a name")));
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap_or_else(|why| panic!("{why}"));
        }
    }
}

/// The `Migrate` entry a build of `to` appends when it moves `refs` refs
/// and the journal file, both lines, as `rename/record.rs` writes them.
fn append_migrate(repo: &Path, to: Generation, refs: usize) {
    let path = in_dir(repo, to, JOURNAL);
    let last = Journal::open_as(&path, to)
        .unwrap_or_else(|why| panic!("{why:?}"))
        .entries()
        .last()
        .cloned()
        .unwrap_or_else(|| panic!("an entry"));
    let head = last.head_after.clone().unwrap_or(last.head_before);
    let when = at(20);
    let mut entry = JournalEntry {
        id: EntryId::new(when, [9; 10]),
        at_unix_ms: when,
        kind: OperationKind::Migrate,
        description: mergewright_ops::rename::description(to, refs, 1),
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
    let mut lines =
        mergewright_ops::journal::render_pending(&entry).unwrap_or_else(|why| panic!("{why:?}"));
    entry.status = EntryStatus::Committed;
    lines.extend(
        mergewright_ops::journal::render_outcome(&entry).unwrap_or_else(|why| panic!("{why:?}")),
    );
    let mut bytes = std::fs::read(&path).unwrap_or_else(|why| panic!("{why}"));
    bytes.extend(lines);
    std::fs::write(&path, bytes).unwrap_or_else(|why| panic!("{why}"));
}
