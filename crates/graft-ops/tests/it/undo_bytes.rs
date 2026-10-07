//! m7-power-tools/30q: `u` gives back his bytes, not git's re-encoding of
//! them (ADR-0052).
//!
//! Each test runs an ordinary operation over an uncommitted file git would
//! convert, then `u`, through the same `apply`/`undo` path the app uses, and
//! reads the file back off disk byte for byte, never from a returned value.
//!
//! The first two are R0's evidence (30q-a): red on 8c3f4b1b, where `u` wrote
//! the file back with CRLF endings. The snapshot keeps the bytes (30q-a) and
//! `RestoreWorktree` writes them (30q-b).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use graft_core::journal::{EntryStatus, HeadRef, PutBackWhy};
use graft_core::oid::Oid;
use graft_git::run::Stop;
use graft_ops::apply::{AfterState, ApplyError};
use graft_ops::operation::{
    CheckoutParams, CheckoutTarget, OnEdits, Operation, RecoverParams, RecoverTarget, StashParams,
    StashScope, Target,
};
use graft_ops::restore::{self, RestoreError};
use graft_ops::undo::{self, Step, UndoError};

use crate::set_aside::{set_aside_of, stash_commits};
use crate::support::{NOW, TempRepo, context, journal_for, run, run_redo, run_undo, try_run};

/// What `HEAD` holds at `src/a.rs`.
const COMMITTED: &[u8] = b"pub fn a() -> u32 {\n    1\n}\n";
/// His uncommitted edit (`W`): LF endings only.
const WORKING: &[u8] = b"pub fn a() -> u32 {\n    4\n}\n";

/// `core.autocrlf=true`, a committed `src/a.rs`, then `W` written over it
/// with LF endings only.
fn lf_edit_under_autocrlf() -> TempRepo {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("src/a.rs", COMMITTED);
    repo.commit_all("first");
    repo.write("src/a.rs", WORKING);
    repo
}

/// A0 (30q R0): a plain discard, then `u`.
#[test]
fn u_after_a_discard_brings_back_an_lf_file_under_autocrlf() {
    let repo = lf_edit_under_autocrlf();
    run(
        &repo,
        &Operation::Discard(Target::Paths(vec![PathBuf::from("src/a.rs")])),
        NOW,
    );
    assert_ne!(repo.read("src/a.rs"), WORKING, "the discard wrote over W");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.read("src/a.rs"), WORKING, "W is back, byte for byte");
}

/// A0 (30q R0): a stash push, then `u`.
#[test]
fn u_after_a_stash_brings_back_an_lf_file_under_autocrlf() {
    let repo = lf_edit_under_autocrlf();
    run(
        &repo,
        &Operation::StashPush(StashParams {
            message: Some("wip".to_owned()),
            scope: StashScope::All {
                include_untracked: false,
            },
        }),
        NOW,
    );
    assert_ne!(repo.read("src/a.rs"), WORKING, "the stash took W away");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.read("src/a.rs"), WORKING, "W is back, byte for byte");
}

/// Discard `path`, then `u`: what is on disk after is what was before.
fn discard_then_u(repo: &TempRepo, path: &str) {
    let before = repo.read(path);
    run(
        repo,
        &Operation::Discard(Target::Paths(vec![PathBuf::from(path)])),
        NOW,
    );
    assert_ne!(repo.read(path), before, "the discard wrote over it");

    run_undo(repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(
        repo.read(path),
        before,
        "{path} is back, byte for byte, not {:?}",
        String::from_utf8_lossy(&repo.read(path))
    );
}

/// `run.sh`, `0o755`, LF, changed, under `core.autocrlf=true`: the bytes and
/// the exec bit both come back (R4: the mode is the `raw/` entry's).
#[test]
fn u_brings_back_an_lf_executable_under_autocrlf() {
    use std::os::unix::fs::PermissionsExt;
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    let exec = |bytes: &[u8]| {
        repo.write("run.sh", bytes);
        std::fs::set_permissions(
            repo.path().join("run.sh"),
            std::fs::Permissions::from_mode(0o755),
        )
        .expect("the exec bit is set");
    };
    exec(b"#!/bin/sh\necho 1\n");
    repo.commit_all("first");
    exec(b"#!/bin/sh\necho 4\n");

    discard_then_u(&repo, "run.sh");
    let mode = std::fs::metadata(repo.path().join("run.sh"))
        .expect("the file is there")
        .permissions()
        .mode();
    assert_eq!(mode & 0o111, 0o111, "the exec bit is set: {mode:o}");
}

/// A one-way clean filter (nbstripout's shape: clean strips, smudge is
/// `cat`): `u` gives back the line the clean filter strips (R1).
#[test]
fn u_brings_back_what_a_one_way_clean_filter_strips() {
    let repo = TempRepo::init();
    repo.write(".git/info/attributes", b"nb.txt filter=strip\n");
    repo.git(&["config", "filter.strip.clean", "sed '/^#out/d'"]);
    repo.git(&["config", "filter.strip.smudge", "cat"]);
    repo.write("nb.txt", b"cell 1\n");
    repo.commit_all("first");
    repo.write("nb.txt", b"cell 2\n#out 42\n");

    discard_then_u(&repo, "nb.txt");
    assert!(
        repo.read("nb.txt").ends_with(b"#out 42\n"),
        "the stripped line is back"
    );
}

/// `core.autocrlf=input`: git's checkout writes LF, so a CRLF edit is kept.
#[test]
fn u_brings_back_a_crlf_file_under_autocrlf_input() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "input"]);
    repo.write("src/a.rs", COMMITTED);
    repo.commit_all("first");
    repo.write("src/a.rs", b"pub fn a() -> u32 {\r\n    4\r\n}\r\n");

    discard_then_u(&repo, "src/a.rs");
}

/// One CRLF line and one LF line under `core.autocrlf=true`: git's checkout
/// would write every line CRLF.
#[test]
fn u_brings_back_mixed_endings_under_autocrlf() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("src/a.rs", COMMITTED);
    repo.commit_all("first");
    repo.write("src/a.rs", b"pub fn a() -> u32 {\r\n    4\n}\n");

    discard_then_u(&repo, "src/a.rs");
}

/// An untracked LF file under `core.autocrlf=true`: the discard removes it
/// (`clean`), and `u` re-creates it through the untracked arm (a temporary
/// index and `checkout-index`), then writes his bytes.
#[test]
fn u_brings_back_an_lf_untracked_file_under_autocrlf() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("src/a.rs", COMMITTED);
    repo.commit_all("first");
    repo.write("src/new.rs", WORKING);

    run(
        &repo,
        &Operation::Discard(Target::Paths(vec![PathBuf::from("src/new.rs")])),
        NOW,
    );
    assert!(!repo.exists("src/new.rs"), "the discard removed it");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.read("src/new.rs"), WORKING, "W is back, byte for byte");
    assert_eq!(
        repo.git(&["ls-files", "--", "src/new.rs"]),
        "",
        "still untracked"
    );
}

/// Redo restores the `Undo` entry's own before, raw bytes included:
/// 1. a discard;
/// 2. his LF edit over the path it wrote (drift);
/// 3. a forced `u`, which replaces the edit;
/// 4. redo. After redo the path is his LF edit.
#[test]
fn redo_after_a_forced_undo_brings_back_an_lf_drift_under_autocrlf() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("src/a.rs", COMMITTED);
    repo.commit_all("first");
    let first_edit: &[u8] = b"pub fn a() -> u32 {\r\n    2\r\n}\r\n";
    repo.write("src/a.rs", first_edit);

    run(
        &repo,
        &Operation::Discard(Target::Paths(vec![PathBuf::from("src/a.rs")])),
        NOW,
    );
    repo.write("src/a.rs", WORKING);

    run_undo(&repo, NOW + 1, true)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.read("src/a.rs"), first_edit, "the forced u replaced W");

    run_redo(&repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("redo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.read("src/a.rs"), WORKING, "W is back, byte for byte");
}

/// R4: the index is not touched, and his bytes clean to the blob it holds.
/// `W` staged, then a stash push takes both; after `u` git sees nothing
/// unstaged at the path, so the raw write left no phantom edit behind.
#[test]
fn u_of_a_staged_lf_file_leaves_nothing_unstaged() {
    let repo = lf_edit_under_autocrlf();
    repo.git(&["add", "--", "src/a.rs"]);
    let index = repo.git(&["ls-files", "-s"]);
    run(
        &repo,
        &Operation::StashPush(StashParams {
            message: Some("wip".to_owned()),
            scope: StashScope::All {
                include_untracked: false,
            },
        }),
        NOW,
    );

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.read("src/a.rs"), WORKING, "W is back, byte for byte");
    assert_eq!(
        repo.git(&["ls-files", "-s"]),
        index,
        "the index is as staged"
    );
    assert_eq!(
        repo.git(&["diff", "--name-only"]),
        "",
        "nothing unstaged: clean(W) is the staged blob"
    );
}

/// 30q-a's new failure mode, pinned: a smudge filter that is `required` and
/// fails (git-lfs's shape, offline) on a changed file fails the
/// before-snapshot's `raw_record`, so an operation over **another** path is
/// refused before it writes anything, and his edit there stays.
#[test]
fn a_failing_required_smudge_filter_refuses_the_operation_before_it_writes() {
    let repo = TempRepo::init();
    repo.write(".git/info/attributes", b"nb.txt filter=broken\n");
    repo.git(&["config", "filter.broken.clean", "cat"]);
    repo.git(&["config", "filter.broken.smudge", "false"]);
    repo.git(&["config", "filter.broken.required", "true"]);
    repo.write("nb.txt", b"cell 1\n");
    repo.write("src/a.rs", COMMITTED);
    repo.commit_all("first");
    repo.write("nb.txt", b"cell 2\n");
    repo.write("src/a.rs", WORKING);

    let refused = try_run(
        &repo,
        &Operation::Discard(Target::Paths(vec![PathBuf::from("src/a.rs")])),
        NOW,
    );
    assert!(
        refused.is_err(),
        "the discard ran past a snapshot that could not be taken"
    );
    assert_eq!(repo.read("src/a.rs"), WORKING, "his edit is untouched");
    assert_eq!(repo.read("nb.txt"), b"cell 2\n", "and so is nb.txt");
    assert!(
        journal_for(&repo.open()).entries().is_empty(),
        "nothing journaled: the operation never began"
    );
}

/// m8-polish-and-ship/21 item 3: a file whose name is 250 bytes (`NAME_MAX` is
/// 255). The raw write's old temporary, `<name>.graft-raw-<pid>`, passed the
/// limit and made `u` fail every time; now `u` brings his LF bytes back.
#[test]
fn u_brings_back_a_file_with_a_250_byte_name() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    let path = format!("src/{}.rs", "n".repeat(247));
    assert_eq!(path.len() - "src/".len(), 250);
    repo.write(&path, COMMITTED);
    repo.commit_all("first");
    repo.write(&path, WORKING);

    discard_then_u(&repo, &path);
}

/// m8-polish-and-ship/21 item 2: a kept set whose paths add up to over
/// 200 000 bytes (1 000 files, 209-byte paths, each an LF edit under
/// `core.autocrlf=true`) snapshots and restores: a stash push takes every
/// edit, `u` brings each back byte for byte.
#[test]
fn u_brings_back_a_kept_set_of_over_200k_path_bytes() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    let dir = format!("{}/{}", "d".repeat(100), "e".repeat(100));
    let paths: Vec<String> = (0..1000).map(|n| format!("{dir}/{n:04}.rs")).collect();
    let bytes: usize = paths.iter().map(String::len).sum();
    assert!(bytes > 200_000, "{bytes} bytes of paths");
    for path in &paths {
        repo.write(path, COMMITTED);
    }
    repo.commit_all("first");
    for path in &paths {
        repo.write(path, WORKING);
    }

    run(
        &repo,
        &Operation::StashPush(StashParams {
            message: Some("wip".to_owned()),
            scope: StashScope::All {
                include_untracked: false,
            },
        }),
        NOW,
    );
    assert_ne!(repo.read(&paths[0]), WORKING, "the stash took the edits");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    for path in &paths {
        assert_eq!(repo.read(path), WORKING, "{path} is back, byte for byte");
    }
}

/// A raw write that cannot land (here: a file already at every temporary
/// name it may use, none of which is ever written over) fails `u` with the
/// path named in its own words, and the roll-back puts back what `u` found.
#[test]
fn a_raw_write_that_cannot_land_fails_u_and_names_the_path() {
    let repo = lf_edit_under_autocrlf();
    let in_the_way = repo.block_raw_writes_in("src");
    run(
        &repo,
        &Operation::Discard(Target::Paths(vec![PathBuf::from("src/a.rs")])),
        NOW,
    );
    let discarded = repo.read("src/a.rs");

    let Err(UndoError::Restore(why)) = run_undo(&repo, NOW + 1, false) else {
        panic!("u wrote over a file at its temporary's name");
    };
    assert!(
        matches!(&why, RestoreError::WriteRaw { path, .. } if path == &PathBuf::from("src/a.rs")),
        "{why:?}"
    );
    assert!(
        why.explain(&crate::support::Letters)
            .starts_with("could not write src/a.rs back as it was: "),
        "{}",
        why.explain(&crate::support::Letters)
    );
    assert_eq!(
        repo.read("src/a.rs"),
        discarded,
        "rolled back to what u found"
    );
    for path in &in_the_way {
        assert_eq!(repo.read(path), b"", "{path}, in the way, is untouched");
    }
}

/// A stop that fires, and stays fired, once `path` no longer holds `from`:
/// keyed to the plan's own effect (the working-tree step has written it),
/// never to a poll count (`undo_cancel.rs` sets out why).
struct StopOnceChanged {
    path: PathBuf,
    from: &'static [u8],
    fired: AtomicBool,
}

impl Stop for StopOnceChanged {
    fn should_stop(&self) -> bool {
        if !self.fired.load(Ordering::Relaxed)
            && std::fs::read(&self.path).map_or(true, |now| now != self.from)
        {
            self.fired.store(true, Ordering::Relaxed);
        }
        self.fired.load(Ordering::Relaxed)
    }
}

/// R3, the roll-back writes the restore's own `raw/`: a stash push, then his
/// LF edit over the path it wrote. The undo (straight at `restore::execute`,
/// so no drift rail) restores the path, and is cancelled before its last
/// step, the `StashDrop`. The roll-back puts back what the **undo** found,
/// his LF edit, byte for byte.
#[test]
fn a_cancelled_undo_rolls_back_to_his_lf_bytes_under_autocrlf() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("src/a.rs", COMMITTED);
    repo.commit_all("first");
    repo.write("src/a.rs", b"pub fn a() -> u32 {\r\n    2\r\n}\r\n");
    run(
        &repo,
        &Operation::StashPush(StashParams {
            message: Some("wip".to_owned()),
            scope: StashScope::All {
                include_untracked: false,
            },
        }),
        NOW,
    );
    repo.write("src/a.rs", WORKING);

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let plan = undo::plan_undo(journal.entries()).expect("the push is undoable");
    let worktree_step = plan
        .steps
        .iter()
        .position(|step| matches!(step, Step::RestoreWorktree { .. }))
        .expect("the plan restores the working tree");
    assert!(
        worktree_step + 1 < plan.steps.len(),
        "a step follows it, so a cancel can land after it: {:?}",
        plan.steps
    );
    let stop = StopOnceChanged {
        path: repo.path().join("src/a.rs"),
        from: WORKING,
        fired: AtomicBool::new(false),
    };
    let outcome = {
        let mut ctx = context(&handle, &mut journal, NOW + 1, &stop);
        restore::execute(&mut ctx, &plan)
    };

    match outcome {
        Err(RestoreError::Cancelled { rolled_back }) => {
            assert!(rolled_back >= 1, "the working-tree step was rolled back");
        }
        other => panic!("expected a clean cancel: {other:?}"),
    }
    assert_eq!(
        repo.read("src/a.rs"),
        WORKING,
        "the roll-back gave back his bytes, not {:?}",
        String::from_utf8_lossy(&repo.read("src/a.rs"))
    );
}

// ── the set-aside put-back (m7-power-tools/30s) ─────────────────────────
//
// `stash apply` writes his content back in git's encoding; the put-back
// then writes his own bytes over each path whose content came back as it
// was (`restore::raw::same_content_paths`).

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("an object id")
}

/// `path` on disk is `bytes`, byte for byte; shown as text when not.
fn assert_bytes(repo: &TempRepo, path: &str, bytes: &[u8], when: &str) {
    let on_disk = repo.read(path);
    assert!(
        on_disk == bytes,
        "{when}: {path} is {:?}, not {:?}",
        String::from_utf8_lossy(&on_disk),
        String::from_utf8_lossy(bytes)
    );
}

/// The shell's `reset --hard HEAD~1`, then `edit` over the tree: `J ⏎ y`'s
/// start (26's `promise/recover.rs`). The `MoveRef` that puts `main` back
/// on the tip the reset left, over his edit, and that tip.
fn shell_reset_then(repo: &TempRepo, edit: impl FnOnce(&TempRepo)) -> (Operation, Oid) {
    let lost = id(repo, "HEAD");
    repo.git(&["reset", "-q", "--hard", "HEAD~1"]);
    let reset_to = id(repo, "HEAD");
    edit(repo);
    let op = Operation::Recover(RecoverParams {
        target: RecoverTarget::MoveRef {
            name: "main".to_owned(),
            to: lost,
            expected: reset_to,
        },
        because: "the shell's reset".to_owned(),
    });
    (op, lost)
}

/// `core.autocrlf=true`; `main` at `first` (`src/a.rs`) then `second`
/// (`b.txt`, so `src/a.rs` is the same on both); the shell's reset; `W` over
/// `src/a.rs`, LF.
fn reset_off_second_with_an_lf_edit() -> (TempRepo, Operation, Oid) {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("src/a.rs", COMMITTED);
    repo.commit_all("first");
    repo.write("b.txt", b"b\n");
    repo.commit_all("second");
    let (op, second) = shell_reset_then(&repo, |repo| repo.write("src/a.rs", WORKING));
    (repo, op, second)
}

/// The scenario's `J ⏎ y`, then `u`: `W`, byte for byte (26's F-recover).
#[test]
fn u_after_a_set_aside_put_back_brings_back_an_lf_file_under_autocrlf() {
    let (repo, op, second) = reset_off_second_with_an_lf_edit();
    run(&repo, &op, NOW);
    assert_eq!(id(&repo, "refs/heads/main"), second, "main is on second");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_ne!(id(&repo, "refs/heads/main"), second, "main is off second");
    assert_bytes(&repo, "src/a.rs", WORKING, "after u");
}

/// The same `J ⏎ y`, forward: his edit is put back as it was — LF, still
/// unstaged, the set-aside stash dropped.
#[test]
fn a_set_aside_put_back_keeps_an_lf_file_under_autocrlf() {
    let (repo, op, second) = reset_off_second_with_an_lf_edit();
    run(&repo, &op, NOW);
    assert_eq!(id(&repo, "refs/heads/main"), second, "main is on second");
    assert_bytes(&repo, "src/a.rs", WORKING, "after the put-back");
    assert_eq!(
        repo.git(&["status", "--porcelain"]),
        " M src/a.rs",
        "unstaged (a blank index column), as it was"
    );
    assert_eq!(stash_commits(&repo), Vec::<Oid>::new(), "S is dropped");
}

/// A name git quotes (a space, a non-ASCII byte, and a leading `"`, which
/// `--stdin-paths` would otherwise read as the start of a quoted name) gets
/// his bytes back all the same: `hash-object` reads it C-quoted.
#[test]
fn a_set_aside_put_back_keeps_an_lf_file_with_a_quoted_name() {
    let name = "\"é q\".rs";
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write(name, COMMITTED);
    repo.commit_all("first");
    repo.write("b.txt", b"b\n");
    repo.commit_all("second");
    let (op, second) = shell_reset_then(&repo, |repo| repo.write(name, WORKING));
    run(&repo, &op, NOW);
    assert_eq!(id(&repo, "refs/heads/main"), second, "main is on second");
    assert_bytes(&repo, name, WORKING, "after the put-back");
}

/// The put-back's raw step cannot land (a file already at every temporary
/// name, as `a_raw_write_that_cannot_land_fails_u_and_names_the_path` sets
/// it up): the operation stands, `Committed`; `S` is dropped; his edit is on
/// disk in git's encoding; and the answer says so first, naming the path,
/// with no `u` clause (Ruling 2026-10-01 §1).
#[test]
fn a_put_back_whose_raw_step_cannot_land_says_so_and_drops_s() {
    let (repo, op, second) = reset_off_second_with_an_lf_edit();
    let in_the_way = repo.block_raw_writes_in("src");

    let Err(ApplyError::AfterState { applied, unread }) = try_run(&repo, &op, NOW) else {
        panic!("the raw step's failure was not reported");
    };
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert!(
        matches!(
            unread.as_slice(),
            [AfterState::NotAsTheyWere {
                error: RestoreError::WriteRaw { path, .. },
                files: 1,
            }] if path == &PathBuf::from("src/a.rs")
        ),
        "{unread:?}"
    );
    let words = unread[0].to_string();
    assert!(
        words.starts_with("your edits are back · src/a.rs not written back as it was: ")
            && words.ends_with(" · its exact bytes are in the backup"),
        "{words}"
    );
    assert_eq!(id(&repo, "refs/heads/main"), second, "main is on second");
    assert_eq!(stash_commits(&repo), Vec::<Oid>::new(), "S is dropped");
    assert_ne!(repo.read("src/a.rs"), WORKING, "git's encoding, not his");
    assert_eq!(content(&repo.read("src/a.rs")), WORKING, "his edit is back");
    for path in &in_the_way {
        assert_eq!(repo.read(path), b"", "{path}, in the way, is untouched");
    }
}

/// A second kind that sets aside: a checkout of `other`, which does not
/// touch `src/a.rs`, over `W`. After it `W`; after `u` `W`.
#[test]
fn a_switch_with_edits_keeps_an_lf_file_under_autocrlf() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("src/a.rs", COMMITTED);
    repo.commit_all("first");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.write("o.txt", b"o\n");
    repo.commit_all("other");
    repo.git(&["switch", "-q", "main"]);
    repo.write("src/a.rs", WORKING);
    let op = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch("refs/heads/other".to_owned()),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        on_edits: OnEdits::SetAside,
    });

    let entry = run(&repo, &op, NOW);
    set_aside_of(&entry);
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/other");
    assert_bytes(&repo, "src/a.rs", WORKING, "after the checkout");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/main");
    assert_bytes(&repo, "src/a.rs", WORKING, "after u");
}

/// Seven lines, far enough apart for git to merge a change at each end.
const SEVEN: &[u8] = b"one\ntwo\nthree\nfour\nfive\nsix\nseven\n";
/// `second`'s change, at the top.
const SEVEN_TOP: &[u8] = b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\n";
/// His edit, at the bottom.
const SEVEN_HIS: &[u8] = b"one\ntwo\nthree\nfour\nfive\nsix\nhis seven\n";
/// Both.
const SEVEN_BOTH: &[u8] = b"ONE\ntwo\nthree\nfour\nfive\nsix\nhis seven\n";

/// `bytes` with every CRLF read as LF: content, not endings.
fn content(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while let Some((&byte, tail)) = rest.split_first() {
        if byte == b'\r' && tail.first() == Some(&b'\n') {
            rest = tail;
            continue;
        }
        out.push(byte);
        rest = tail;
    }
    out
}

/// Guard: `second` changes `src/a.rs` at the top and his edit is at the
/// bottom, so the put-back merges his edit into new content. His raw bytes
/// are not written over it: the file holds both changes. `u` gives `W`.
#[test]
fn a_put_back_that_merges_into_a_changed_file_writes_no_raw() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("src/a.rs", SEVEN);
    repo.commit_all("first");
    repo.write("src/a.rs", SEVEN_TOP);
    repo.commit_all("second");
    let (op, second) = shell_reset_then(&repo, |repo| repo.write("src/a.rs", SEVEN_HIS));

    run(&repo, &op, NOW);
    assert_eq!(id(&repo, "refs/heads/main"), second, "main is on second");
    assert_eq!(
        content(&repo.read("src/a.rs")),
        SEVEN_BOTH,
        "second's change and his: {:?}",
        String::from_utf8_lossy(&repo.read("src/a.rs"))
    );

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_bytes(&repo, "src/a.rs", SEVEN_HIS, "after u");
}

/// Guard: `second` changes the line `W` changes, so the put-back clashes.
/// As `set_aside::a_clashing_put_back_is_clashed_and_keeps_the_stash`:
/// `Clashed { SetAside }`, the entry `Committed` with an unmerged index
/// after, `S` kept. No raw write: the file holds git's conflict.
#[test]
fn a_clashed_put_back_writes_no_raw() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("src/a.rs", COMMITTED);
    repo.commit_all("first");
    repo.write("src/a.rs", b"pub fn a() -> u32 {\n    9\n}\n");
    repo.commit_all("second");
    let (op, second) = shell_reset_then(&repo, |repo| repo.write("src/a.rs", WORKING));

    let (entry, clash) = match try_run(&repo, &op, NOW) {
        Err(ApplyError::Clashed { entry, clash }) => (*entry, clash),
        other => panic!("the put-back should clash: {other:?}"),
    };
    assert_eq!(clash.why, PutBackWhy::SetAside);
    assert_eq!(clash.paths, vec![PathBuf::from("src/a.rs")]);
    let stash = set_aside_of(&entry);
    assert_eq!(clash.stash, stash);
    assert_eq!(entry.status, EntryStatus::Committed);
    assert!(entry.index_after.is_some_and(|index| index.is_unmerged()));
    assert!(
        !repo.git(&["ls-files", "-u"]).is_empty(),
        "git says unmerged"
    );
    assert_eq!(id(&repo, "refs/heads/main"), second, "main is on second");
    assert_eq!(stash_commits(&repo), vec![stash], "S kept");
    let on_disk = repo.read("src/a.rs");
    assert!(
        on_disk.windows(7).any(|seven| seven == b"<<<<<<<") && on_disk != WORKING,
        "git's conflict, not his bytes: {:?}",
        String::from_utf8_lossy(&on_disk)
    );
}

/// 30q-a's one-way clean filter (nbstripout's shape): `nb.txt` holds a line
/// the clean filter strips. Set aside and put back, the line is still there.
#[test]
fn a_set_aside_under_a_one_way_clean_filter_keeps_the_stripped_lines() {
    let repo = TempRepo::init();
    repo.write(".git/info/attributes", b"nb.txt filter=strip\n");
    repo.git(&["config", "filter.strip.clean", "sed '/^#out/d'"]);
    repo.git(&["config", "filter.strip.smudge", "cat"]);
    repo.write("nb.txt", b"cell 1\n");
    repo.commit_all("first");
    repo.write("b.txt", b"b\n");
    repo.commit_all("second");
    let (op, second) = shell_reset_then(&repo, |repo| repo.write("nb.txt", b"cell 2\n#out 42\n"));

    run(&repo, &op, NOW);
    assert_eq!(id(&repo, "refs/heads/main"), second, "main is on second");
    assert_bytes(&repo, "nb.txt", b"cell 2\n#out 42\n", "after the put-back");
}
