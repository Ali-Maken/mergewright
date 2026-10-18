//! Review m8-48 Minor 2–4: the stray pass meeting two generations' refs
//! under one name, the names a run staying old reports, and a kept-aside
//! journal naming a backup that is gone.

use mergewright_core::on_disk::Generation;
use mergewright_git::file_lock::FileLock;
use mergewright_ops::rename::{
    KEPT_ASIDE_DIR, LOCK_FILE, MigrateError, SettleOutcome, settle_names,
};

use super::super::old_repo;
use super::setup::{
    JOURNAL, as_tributary, fsck_is_clean, in_dir, loose_commit, marker_of, no_earlier_names_left,
    refs_of,
};

/// Minor 2: a `clean` repository meets `refs/tributary/x` and
/// `refs/graft/x` with no `refs/mergewright/x` (a Tributary build died
/// mid-move on Graft leftovers). The newer is carried, the older kept aside
/// when it differs and deleted when it is the same; never stuck.
#[test]
fn two_generations_one_name_carry_the_newer() {
    let (graft, tributary) = (Generation::Graft, Generation::Tributary);
    let repo = old_repo(0, 1);
    assert!(matches!(
        settle_names(&repo.open()).outcome,
        SettleOutcome::Migrated { .. }
    ));
    let before = refs_of(&repo, Generation::CURRENT);
    let head = repo.git(&["rev-parse", "HEAD"]);
    let theirs = loose_commit(&repo);
    for (generation, x, y) in [(tributary, &head, &head), (graft, &theirs, &head)] {
        repo.git(&["update-ref", &format!("{}x", generation.ref_prefix()), x]);
        repo.git(&["update-ref", &format!("{}y", generation.ref_prefix()), y]);
    }

    let settled = settle_names(&repo.open());

    assert!(
        matches!(
            settled.outcome,
            SettleOutcome::OlderWrote {
                refs: 4,
                files: 0,
                kept_aside: Some(_)
            }
        ),
        "{:?}",
        settled.outcome
    );
    let current = Generation::CURRENT.ref_prefix();
    let mut expected = before;
    expected.insert(format!("{current}x"), head.clone());
    expected.insert(format!("{current}y"), head);
    expected.insert(
        format!("{current}{KEPT_ASIDE_DIR}/1/{}/x", graft.dir()),
        theirs,
    );
    assert_eq!(refs_of(&repo, Generation::CURRENT), expected);
    no_earlier_names_left(&repo);
    fsck_is_clean(&repo);
}

/// Minor 4: the handle was opened on Graft names; a Tributary build moved
/// them before this settle, and this one finds a window open. The names it
/// stays on are the ones on disk, Tributary's, not the handle's.
#[test]
fn staying_old_reports_the_names_on_disk() {
    let repo = old_repo(0, 1);
    let handle = repo.open();
    assert_eq!(handle.on_disk_names(), Generation::Graft);
    as_tributary(&repo, "clean\n", false);
    let path = repo.path().join(in_dir(Generation::CURRENT, LOCK_FILE));
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("the folder is made");
    let other = FileLock::shared(&path).expect("the lock is shared");

    let settled = settle_names(&handle);

    assert_eq!(settled.outcome, SettleOutcome::KeptOld(MigrateError::Busy));
    assert_eq!(settled.names, Generation::Tributary);
    drop(other);
}

/// Minor 3, a question answered: a Tributary repository holds a journal
/// its own stray pass kept aside, naming a backup no ref holds (deleted by
/// hand since). The move reads it like any copied journal and refuses
/// before its switch, on every open: nothing is lost, the Tributary names
/// stay in force, and the move never happens while that backup is missing.
#[test]
fn a_kept_aside_journal_naming_a_missing_backup_keeps_old() {
    let tributary = Generation::Tributary;
    let repo = old_repo(0, 1);
    as_tributary(&repo, "clean\n", false);
    let live = String::from_utf8(repo.read(&in_dir(tributary, JOURNAL))).expect("utf-8");
    let backup = refs_of(&repo, tributary)
        .into_keys()
        .next()
        .expect("a backup");
    // The journal spells it as the Graft build wrote it.
    let spelled = backup.replacen(tributary.ref_prefix(), Generation::Graft.ref_prefix(), 1);
    let named = live.replace(&spelled, &format!("{spelled}-deleted"));
    assert_ne!(named, live, "the kept-aside journal names another backup");
    mergewright_ops::journal::parse(named.as_bytes()).expect("the kept-aside journal parses");
    let kept = format!("{KEPT_ASIDE_DIR}/1/{}/{JOURNAL}", Generation::Graft.dir());
    repo.write(&in_dir(tributary, &kept), named.as_bytes());
    let held = refs_of(&repo, tributary);

    for _ in 0..2 {
        let settled = settle_names(&repo.open());
        assert!(
            matches!(
                &settled.outcome,
                SettleOutcome::KeptOld(MigrateError::ReadBack { what }) if what.contains("-deleted")
            ),
            "{:?}",
            settled.outcome
        );
        assert_eq!(settled.names, tributary);
    }
    assert_eq!(refs_of(&repo, tributary), held, "nothing deleted");
    assert_eq!(repo.read(&in_dir(tributary, &kept)), named.as_bytes());
    assert_eq!(marker_of(&repo, Generation::CURRENT), None);
}
