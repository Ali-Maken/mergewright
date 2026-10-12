//! The windows after the switch (review m8-48 Major 1–2): a
//! stray pass cut short, an older build moving alongside, a `switched`
//! clean-up from each source. Each builds its state on disk, as an older build
//! or a crash would leave it, and reads the truth back from git and the
//! filesystem.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use graft_core::on_disk::{Generation, MIGRATED_MARKER};
use graft_git::file_lock::FileLock;
use graft_ops::rename::{KEPT_ASIDE_DIR, LOCK_FILE, SettleOutcome, settle_names};

use super::super::old_repo;
use super::super::shim::{After, settle_in_child};
use super::setup::{
    JOURNAL, as_tributary, copy_dir, fsck_is_clean, in_dir, loose_commit, marker_of, moved,
    no_earlier_names_left, refs_of,
};
use crate::support::TempRepo;

/// Where the older build of [`tributary_build_moves`] says what it did.
const TRIED: &str = ".git/tributary-build";

/// Every ref of the repository, name to value.
fn all_refs(repo: &TempRepo) -> BTreeMap<String, String> {
    super::super::refs_under(repo, "refs/")
}

/// A mode on a folder, as a permission a test sets and takes back.
fn mode(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
}

/// The move from Tributary names, cut short in its stray pass: an empty
/// folder under `refs/graft/` that cannot be removed fails the pass after
/// Tributary's refs and files are settled. A Tributary build still open on
/// its names (it holds no lock) then writes a backup and a journal line.
/// The next open must carry or keep both, never remove them unread.
#[test]
fn a_stray_pass_cut_short_keeps_what_tributary_wrote_next() {
    let tributary = Generation::Tributary;
    let repo = old_repo(0, 2);
    as_tributary(&repo, "clean\n", false);
    let held = refs_of(&repo, tributary);
    let stuck = repo.path().join(".git/refs/graft/sub");
    std::fs::create_dir_all(stuck.join("inner")).expect("the folders are made");
    mode(&stuck, 0o500);

    let first = settle_names(&repo.open());
    mode(&stuck, 0o700);

    assert!(
        matches!(first.outcome, SettleOutcome::LeftOld(_)),
        "the pass was cut short: {:?}",
        first.outcome
    );
    assert!(
        marker_of(&repo, Generation::CURRENT).is_some_and(|line| line.starts_with("switched")),
        "{:?}",
        marker_of(&repo, Generation::CURRENT)
    );
    // The Tributary build writes on.
    let theirs = loose_commit(&repo);
    let backup = format!("{}backup/main/theirs", tributary.ref_prefix());
    repo.git(&["update-ref", &backup, &theirs]);
    let mut journal = repo.read(&in_dir(Generation::CURRENT, JOURNAL));
    journal.extend(b"{\"a line\":\"tributary wrote\"}\n");
    repo.write(&in_dir(tributary, JOURNAL), &journal);

    let second = settle_names(&repo.open());

    let kept = repo
        .path()
        .join(in_dir(Generation::CURRENT, KEPT_ASIDE_DIR))
        .join("1");
    assert!(
        matches!(
            &second.outcome,
            SettleOutcome::CleanedUp { refs: 1, files: 1, kept_aside: Some(at) } if *at == kept
        ),
        "{:?}",
        second.outcome
    );
    let mut expected = moved(&held, Generation::CURRENT);
    expected.insert(
        format!("{}backup/main/theirs", Generation::CURRENT.ref_prefix()),
        theirs,
    );
    assert_eq!(
        refs_of(&repo, Generation::CURRENT),
        expected,
        "the backup is carried"
    );
    assert_eq!(
        std::fs::read(kept.join(tributary.dir()).join(JOURNAL)).expect("the journal is kept"),
        journal,
        "the journal line is kept"
    );
    assert_eq!(
        marker_of(&repo, Generation::CURRENT).as_deref(),
        Some("clean\n")
    );
    no_earlier_names_left(&repo);
    fsck_is_clean(&repo);
}

/// The older build of [`an_older_build_moving_alongside_is_shut_out`]: a
/// Tributary build on a repository it sees on Graft names. It takes its own
/// lock alone, as its code does, and if it gets it, moves: its marker, a
/// backup under its prefix, a journal. It writes what it did to [`TRIED`].
#[test]
#[ignore = "run by the git shim, in a child of a child of this test binary"]
fn tributary_build_moves() {
    let tributary = Generation::Tributary;
    let repo = PathBuf::from(std::env::var_os("RENAME_CHILD_REPO").expect("the repository"));
    let dir = repo.join(".git").join(tributary.dir());
    std::fs::create_dir_all(&dir).expect("its folder is made");
    let tried = match FileLock::try_exclusive(&dir.join(LOCK_FILE)).expect("flock answers") {
        None => "refused".to_owned(),
        Some(_lock) => {
            let git = |args: &[&str]| {
                let output = std::process::Command::new("git")
                    .args(args)
                    .current_dir(&repo)
                    .output()
                    .expect("git runs");
                assert!(output.status.success(), "{args:?}");
                String::from_utf8(output.stdout)
                    .expect("utf-8")
                    .trim()
                    .to_owned()
            };
            let theirs = git(&["commit-tree", "HEAD^{tree}", "-p", "HEAD", "-m", "theirs"]);
            let backup = format!("{}backup/main/theirs", tributary.ref_prefix());
            git(&["update-ref", &backup, &theirs]);
            std::fs::write(dir.join(JOURNAL), "{\"a line\":\"tributary wrote\"}\n")
                .expect("its journal");
            std::fs::write(dir.join(MIGRATED_MARKER), "clean\n").expect("its marker");
            format!("moved {theirs}")
        }
    };
    std::fs::write(repo.join(TRIED), tried).expect("says what it did");
}

/// A Graft repository no Tributary build ever opened, so there is no
/// Tributary lock yet. A Tributary build launched while this move copies
/// must be shut out, or what it writes must survive the move.
#[test]
fn an_older_build_moving_alongside_is_shut_out() {
    let repo = old_repo(0, 1);
    assert!(!repo.exists(&in_dir(Generation::Tributary, LOCK_FILE)));
    let helper = std::env::current_exe().expect("the test binary");
    let script = format!(
        "'{}' --exact rename::generations::windows::tributary_build_moves --ignored \
         --test-threads=1 >/dev/null 2>&1 || true",
        helper.display()
    );

    let outcomes = settle_in_child(
        &repo,
        &After::Run {
            on: "update-ref --stdin",
            script: &script,
        },
        1,
    );

    assert!(outcomes[0].starts_with("Migrated"), "{outcomes:?}");
    let tried = String::from_utf8(repo.read(TRIED)).expect("utf-8");
    let survives = |oid: &str| all_refs(&repo).values().any(|value| value == oid);
    match tried.strip_prefix("moved ") {
        None => assert_eq!(tried, "refused"),
        Some(theirs) => assert!(
            survives(theirs),
            "the older build moved, and its backup was removed unread"
        ),
    }
    no_earlier_names_left(&repo);
    fsck_is_clean(&repo);
}

/// Major 2 (a): Mergewright `switched` after a Tributary source, its clean
/// not yet written: the Tributary refs and folder, copies already made,
/// go through the stray pass and are deleted as equal.
#[test]
fn switched_from_tributary_cleans_up() {
    let tributary = Generation::Tributary;
    let repo = old_repo(0, 2);
    as_tributary(&repo, "clean\n", false);
    let held = refs_of(&repo, tributary);
    copy_as_mergewright(&repo, tributary, &held);
    repo.write(
        &in_dir(Generation::CURRENT, MIGRATED_MARKER),
        b"switched tributary\n",
    );

    let settled = settle_names(&repo.open());

    assert_eq!(
        settled.outcome,
        SettleOutcome::CleanedUp {
            refs: 2,
            files: 1,
            kept_aside: None
        }
    );
    assert_eq!(
        refs_of(&repo, Generation::CURRENT),
        moved(&held, Generation::CURRENT)
    );
    no_earlier_names_left(&repo);
    assert_eq!(
        marker_of(&repo, Generation::CURRENT).as_deref(),
        Some("clean\n")
    );
    fsck_is_clean(&repo);
}

/// Major 2 (b): Mergewright `switched` after a Graft source, with a failed
/// earlier move's Tributary scratch (no Tributary marker): the scratch is
/// removed unread in the clean-up, the Graft copies compared.
#[test]
fn switched_from_graft_removes_scratch() {
    let (graft, tributary) = (Generation::Graft, Generation::Tributary);
    let repo = old_repo(0, 1);
    let held = refs_of(&repo, graft);
    let tip = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["update-ref", &format!("{}x", tributary.ref_prefix()), &tip]);
    repo.write(&in_dir(tributary, JOURNAL), b"{\"scratch\":1}\n");
    copy_as_mergewright(&repo, graft, &held);
    repo.write(
        &in_dir(Generation::CURRENT, MIGRATED_MARKER),
        b"switched graft\n",
    );

    let settled = settle_names(&repo.open());

    assert_eq!(
        settled.outcome,
        SettleOutcome::CleanedUp {
            refs: 1,
            files: 1,
            kept_aside: None
        }
    );
    assert_eq!(
        refs_of(&repo, Generation::CURRENT),
        moved(&held, Generation::CURRENT)
    );
    no_earlier_names_left(&repo);
    assert!(!repo.exists(&in_dir(Generation::CURRENT, KEPT_ASIDE_DIR)));
    fsck_is_clean(&repo);
}

/// Major 1, the re-check: the same `switched graft` state, but a Tributary
/// build has moved since (its marker is there). Its names are no longer
/// scratch: its backup is carried, its journal kept.
#[test]
fn a_newer_generation_with_a_marker_is_never_scratch() {
    let (graft, tributary) = (Generation::Graft, Generation::Tributary);
    let repo = old_repo(0, 1);
    let held = refs_of(&repo, graft);
    let theirs = loose_commit(&repo);
    repo.git(&[
        "update-ref",
        &format!("{}backup/main/theirs", tributary.ref_prefix()),
        &theirs,
    ]);
    repo.write(
        &in_dir(tributary, JOURNAL),
        b"{\"a line\":\"tributary wrote\"}\n",
    );
    repo.write(&in_dir(tributary, MIGRATED_MARKER), b"clean\n");
    copy_as_mergewright(&repo, graft, &held);
    repo.write(
        &in_dir(Generation::CURRENT, MIGRATED_MARKER),
        b"switched graft\n",
    );

    let settled = settle_names(&repo.open());

    assert!(
        matches!(
            settled.outcome,
            SettleOutcome::CleanedUp {
                refs: 2,
                kept_aside: Some(_),
                ..
            }
        ),
        "{:?}",
        settled.outcome
    );
    let mut expected = moved(&held, Generation::CURRENT);
    expected.insert(
        format!("{}backup/main/theirs", Generation::CURRENT.ref_prefix()),
        theirs,
    );
    assert_eq!(refs_of(&repo, Generation::CURRENT), expected);
    let kept = in_dir(
        Generation::CURRENT,
        &format!("{KEPT_ASIDE_DIR}/1/{}/{JOURNAL}", tributary.dir()),
    );
    assert_eq!(repo.read(&kept), b"{\"a line\":\"tributary wrote\"}\n");
    no_earlier_names_left(&repo);
    fsck_is_clean(&repo);
}

/// What a move from `from` copies before its switch: every ref under the
/// new prefix, and `from`'s folder, less its lock and marker.
fn copy_as_mergewright(repo: &TempRepo, from: Generation, held: &BTreeMap<String, String>) {
    for (name, oid) in moved(held, Generation::CURRENT) {
        repo.git(&["update-ref", &name, &oid]);
    }
    let git = repo.path().join(".git");
    let to = git.join(Generation::CURRENT.dir());
    copy_dir(&git.join(from.dir()), &to);
    for machinery in [LOCK_FILE, MIGRATED_MARKER] {
        match std::fs::remove_file(to.join(machinery)) {
            Ok(()) => {}
            // Graft's folder has neither.
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            Err(why) => panic!("{machinery}: {why}"),
        }
    }
}
