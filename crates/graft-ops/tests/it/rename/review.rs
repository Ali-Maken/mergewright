//! Review m8-26's fixes (ticket 26r Part A): a failed move retries from
//! the old names, and what an older build writes under the old names is
//! carried or kept aside, never deleted unread.

use std::collections::BTreeMap;
use std::path::Path;

use graft_core::journal::OperationKind;
use graft_core::on_disk::Generation;
use graft_ops::rename::{KEPT_ASIDE_DIR, MigrateError, SettleOutcome, settle_names};

use super::{as_new, marker, old_repo, refs_under, settle, shim, stage_edit};
use crate::support::{NOW, TempRepo};
use graft_core::on_disk::CURRENT_DIR;

/// `.git/tributary/older-version/<n>/<path>`.
fn kept(n: usize, path: &str) -> String {
    format!(
        ".git/{}/{KEPT_ASIDE_DIR}/{n}/{path}",
        Generation::CURRENT.dir()
    )
}

fn add_worktree(repo: &TempRepo, name: &str) {
    repo.git(&["worktree", "add", "-q", "--detach", name, "HEAD"]);
    repo.harden(Path::new(name));
}

/// A second commit, for a value no ref holds yet.
fn another_commit(repo: &TempRepo) -> String {
    repo.git(&["commit", "-q", "--allow-empty", "-m", "another"]);
    repo.git(&["rev-parse", "HEAD"])
}

fn fsck_is_clean(repo: &TempRepo) {
    let fsck = repo
        .try_git(&["fsck", "--no-progress", "--strict"])
        .unwrap_or_else(|why| panic!("fsck fails: {why}"));
    for word in ["missing", "broken", "error"] {
        assert!(!fsck.contains(word), "{fsck}");
    }
}

/// Migrated, with the marker `clean`: a repository on the new names.
fn moved_repo() -> TempRepo {
    let repo = old_repo(0, 1);
    assert!(matches!(
        settle(&repo).outcome,
        SettleOutcome::Migrated {
            kept_aside: None,
            ..
        }
    ));
    assert_eq!(marker(&repo).as_deref(), Some("clean\n"));
    repo
}

#[test]
fn a_failed_copy_retries_cleanly() {
    let repo = old_repo(0, 1);
    repo.write(
        ".git/graft/rewrite/plan-id/plan.json",
        b"{\"paused\":true}\n",
    );
    add_worktree(&repo, "linked");
    // The copy walks the common dir's folder first, then the worktree's,
    // where a symlink stops it: part of the move is copied, then it fails.
    let linked = repo.path().join(".git/worktrees/linked/graft");
    std::fs::create_dir_all(&linked).expect("the folder is made");
    std::os::unix::fs::symlink("elsewhere", linked.join("link")).expect("the link is made");

    let failed = settle(&repo);
    assert!(
        matches!(&failed.outcome, SettleOutcome::KeptOld(MigrateError::Files { path, .. }) if path.ends_with("link")),
        "{:?}",
        failed.outcome
    );
    assert!(
        repo.exists(&format!(".git/{CURRENT_DIR}/rewrite/plan-id/plan.json"))
            && repo.exists(&format!(".git/{CURRENT_DIR}/undo-journal.jsonl")),
        "copied part-way"
    );
    drop(failed);

    // The run goes on under the old names: the journal grows by one, and
    // the paused rewrite finishes (its plan goes).
    std::fs::remove_file(linked.join("link")).expect("the link goes");
    let before = repo.read(".git/graft/undo-journal.jsonl");
    stage_edit(&repo, "after the failure\n", NOW + 500);
    let grown = repo.read(".git/graft/undo-journal.jsonl");
    let parse = |bytes: &[u8]| graft_ops::journal::parse(bytes).expect("parses").entries;
    assert_eq!(parse(&grown).len(), parse(&before).len() + 1);
    std::fs::remove_dir_all(repo.path().join(".git/graft/rewrite")).expect("the plan goes");
    let old = refs_under(&repo, "refs/graft/");

    let settled = settle(&repo);

    assert!(
        matches!(
            settled.outcome,
            SettleOutcome::Migrated {
                refs: 2,
                kept_aside: None,
                not_journaled: None,
                ..
            }
        ),
        "{:?}",
        settled.outcome
    );
    // The new journal is the grown old one, then the move's own entry.
    let new = repo.read(&format!(".git/{CURRENT_DIR}/undo-journal.jsonl"));
    assert_eq!(&new[..grown.len()], &grown[..]);
    let tail = parse(&new[grown.len()..]);
    assert_eq!(tail.len(), 1);
    assert_eq!(tail[0].kind, OperationKind::Migrate);
    assert!(
        !repo.exists(&format!(".git/{CURRENT_DIR}/rewrite/plan-id")),
        "no stale plan"
    );
    assert_eq!(
        refs_under(&repo, &format!("refs/{CURRENT_DIR}/")),
        as_new(&old)
    );
    assert!(!repo.exists(".git/graft") && !repo.exists(".git/worktrees/linked/graft"));
    fsck_is_clean(&repo);
}

#[test]
fn a_stale_new_ref_before_the_switch_is_overwritten() {
    let repo = old_repo(0, 2);
    let other = another_commit(&repo);
    let old = refs_under(&repo, "refs/graft/");
    let first_new = as_new(&old).into_keys().next().expect("a backup");
    // An earlier, failed attempt's scratch: one at another value, one with
    // no old ref at all.
    repo.git(&["update-ref", &first_new, &other]);
    repo.git(&["update-ref", &format!("refs/{CURRENT_DIR}/stale"), &other]);

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
fn an_old_file_written_after_its_copy_is_kept_aside() {
    let repo = old_repo(0, 1);
    let copy = repo.read(".git/graft/undo-journal.jsonl");
    let journal = repo.path().join(".git/graft/undo-journal.jsonl");
    // An older build appends at the stray pass's first spawn: after the
    // copy, the switch and the `Migrate` entry, before step 8.
    let script = format!("printf '{{\"late\":1}}\\n' >> '{}'", journal.display());

    let outcomes = shim::settle_in_child(
        &repo,
        &shim::After::Run {
            on: &format!("refs/graft/ refs/{CURRENT_DIR}/"),
            script: &script,
        },
        1,
    );

    assert!(
        outcomes[0].starts_with("Migrated") && outcomes[0].contains("kept_aside: Some("),
        "{outcomes:?}"
    );
    let mut appended = copy.clone();
    appended.extend_from_slice(b"{\"late\":1}\n");
    assert_eq!(repo.read(&kept(1, "graft/undo-journal.jsonl")), appended);
    let new = repo.read(&format!(".git/{CURRENT_DIR}/undo-journal.jsonl"));
    assert_eq!(&new[..copy.len()], &copy[..], "the new journal is the copy");
    assert!(!repo.exists(".git/graft"), "the old tree is gone");
    assert_eq!(marker(&repo).as_deref(), Some("clean\n"));
}

#[test]
fn a_write_after_the_rename_is_found_next_open() {
    let repo = moved_repo();
    let head = repo.git(&["rev-parse", "HEAD"]);
    repo.write(&format!(".git/{CURRENT_DIR}/layout"), b"new\n");
    repo.write(".git/graft/layout", b"old\n");
    repo.git(&["update-ref", "refs/graft/x", &head]);
    let layout = repo.path().join(".git/graft/layout");
    // The older build writes its layout again at the pass's ref
    // transaction, after every old file was renamed into the pass's
    // folder (and the emptied old folder removed: it makes it again).
    let script = format!(
        "mkdir -p '{}' && printf 'again\\n' > '{}'",
        layout.parent().expect("a folder").display(),
        layout.display()
    );

    let outcomes = shim::settle_in_child(
        &repo,
        &shim::After::Run {
            on: "update-ref --stdin",
            script: &script,
        },
        1,
    );
    assert!(outcomes[0].starts_with("OlderWrote"), "{outcomes:?}");
    assert_eq!(repo.read(&kept(1, "graft/layout")), b"old\n");
    assert_eq!(repo.read(".git/graft/layout"), b"again\n", "written again");

    let settled = settle(&repo);

    assert!(
        matches!(&settled.outcome, SettleOutcome::OlderWrote { refs: 0, files: 1, kept_aside: Some(at) } if at.ends_with("older-version/2")),
        "{:?}",
        settled.outcome
    );
    assert_eq!(repo.read(&kept(2, "graft/layout")), b"again\n");
    assert!(!repo.exists(".git/graft"));
}

#[test]
fn an_older_build_writing_after_clean_is_carried_or_kept() {
    let repo = old_repo(0, 1);
    add_worktree(&repo, "linked");
    assert!(matches!(
        settle(&repo).outcome,
        SettleOutcome::Migrated { .. }
    ));
    let head = repo.git(&["rev-parse", "HEAD"]);
    // An older build, still open, writes under the old names: a layout
    // that differs from the new one, a backup ref, and a linked worktree's
    // first journal.
    repo.write(&format!(".git/{CURRENT_DIR}/layout"), b"new\n");
    repo.write(".git/graft/layout", b"older\n");
    repo.git(&["update-ref", "refs/graft/x", &head]);
    repo.write(
        ".git/worktrees/linked/graft/undo-journal.jsonl",
        b"{\"older\":1}\n",
    );
    let new_before = refs_under(&repo, &format!("refs/{CURRENT_DIR}/"));

    let (outcomes, spawned_second) = shim::settle_twice_counting(&repo);

    assert!(
        outcomes[0].starts_with("OlderWrote { refs: 1,"),
        "{outcomes:?}"
    );
    assert_eq!(outcomes[1], "Done");
    assert_eq!(
        spawned_second,
        Vec::<String>::new(),
        "the second open spawned git"
    );
    let mut carried = new_before;
    carried.insert(format!("refs/{CURRENT_DIR}/x"), head);
    assert_eq!(refs_under(&repo, &format!("refs/{CURRENT_DIR}/")), carried);
    assert_eq!(refs_under(&repo, "refs/graft/"), BTreeMap::new());
    assert_eq!(repo.read(&kept(1, "graft/layout")), b"older\n");
    assert_eq!(repo.read(&format!(".git/{CURRENT_DIR}/layout")), b"new\n");
    assert_eq!(
        repo.read(&format!(
            ".git/worktrees/linked/{CURRENT_DIR}/undo-journal.jsonl"
        )),
        b"{\"older\":1}\n"
    );
    assert!(!repo.exists(".git/graft") && !repo.exists(".git/worktrees/linked/graft"));
    assert_eq!(marker(&repo).as_deref(), Some("clean\n"));
}

#[test]
fn an_empty_old_refs_dir_spawns_nothing() {
    let repo = moved_repo();
    std::fs::create_dir_all(repo.path().join(".git/refs/graft/backup/main"))
        .expect("the folders are made");

    let (outcomes, spawned) = shim::settle_once_counting(&repo);

    assert_eq!(outcomes, ["Done"]);
    assert_eq!(spawned, Vec::<String>::new());
}

#[test]
fn a_differing_stray_ref_is_kept_aside() {
    let repo = moved_repo();
    let theirs = repo.git(&["rev-parse", "HEAD"]);
    let ours = another_commit(&repo);
    repo.git(&["update-ref", &format!("refs/{CURRENT_DIR}/x"), &ours]);
    repo.git(&["update-ref", "refs/graft/x", &theirs]);

    let settled = settle_names(&repo.open());

    assert!(
        matches!(&settled.outcome, SettleOutcome::OlderWrote { refs: 1, files: 0, kept_aside: Some(at) } if at == Path::new(&format!("refs/{CURRENT_DIR}/older-version/1/"))),
        "{:?}",
        settled.outcome
    );
    let refs = refs_under(&repo, &format!("refs/{CURRENT_DIR}/"));
    assert_eq!(
        refs.get(&format!("refs/{CURRENT_DIR}/x")),
        Some(&ours),
        "ours untouched"
    );
    assert_eq!(
        refs.get(&format!("refs/{CURRENT_DIR}/older-version/1/graft/x")),
        Some(&theirs),
        "theirs kept, not deleted"
    );
    assert_eq!(refs_under(&repo, "refs/graft/"), BTreeMap::new());
    assert_eq!(repo.open().on_disk_names(), Generation::CURRENT);
}

#[test]
fn a_copied_journal_naming_a_missing_backup_is_refused() {
    let repo = old_repo(0, 1);
    let backup = refs_under(&repo, "refs/graft/")
        .into_keys()
        .next()
        .expect("a backup");
    repo.git(&["update-ref", "-d", &backup]);

    let settled = settle(&repo);

    assert!(
        matches!(&settled.outcome, SettleOutcome::KeptOld(MigrateError::ReadBack { what }) if what.contains(&format!("refs/{CURRENT_DIR}/backup/"))),
        "{:?}",
        settled.outcome
    );
    assert_eq!(marker(&repo), None, "no switch");
    assert!(
        repo.exists(".git/graft/undo-journal.jsonl"),
        "nothing old deleted"
    );
}
