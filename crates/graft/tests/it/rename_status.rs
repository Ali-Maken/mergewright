//! What the one-time move to the new names says on the first frame's
//! status row (ticket 26 Part B): only what it could not finish.

#![allow(clippy::expect_used, reason = "a failing test should panic")]

use std::path::PathBuf;
use std::process::Command;

use graft::rename::words;
use graft::repos::RepoStore;
use graft_core::on_disk::{Generation, MIGRATED_MARKER};
use graft_ops::rename::{LOCK_FILE, MigrateError, SettleOutcome};

fn git(repo: &std::path::Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout)
        .expect("utf-8")
        .trim()
        .to_owned()
}

/// Another run on the old names holds them: this open keeps them too, the
/// old backup still where it was, and the status row says why.
#[test]
fn kept_old_words() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Linear);
    let repo = scratch.path();
    let head = git(repo, &["rev-parse", "HEAD"]);
    git(repo, &["update-ref", "refs/graft/backup/main/1", &head]);
    let common = PathBuf::from(git(repo, &["rev-parse", "--git-common-dir"]));
    let common = if common.is_absolute() {
        common
    } else {
        repo.join(common)
    };
    let new_dir = common.join(Generation::CURRENT.dir());
    // The corpus is built already moved (`graft-fixtures` corpus.rs
    // `names_settled`, which spells the names itself): pinned here.
    let marker = new_dir.join(MIGRATED_MARKER);
    assert_eq!(
        std::fs::read_to_string(&marker).expect("the fixture carries the marker"),
        "clean\n"
    );
    // Taken away: a repository no build of the app has opened.
    std::fs::remove_file(&marker).expect("the marker goes");
    // What a run still on the old names holds for its life.
    let _other_run =
        graft_git::file_lock::FileLock::shared(&new_dir.join(LOCK_FILE)).expect("the shared lock");

    let store = RepoStore::new(repo.to_path_buf(), None);
    store.shared().expect("it opens");

    assert_eq!(
        store.rename_words().as_deref(),
        Some("rename: still using the old names \u{b7} another trib is open on this repository")
    );
    assert_eq!(store.app_dir().expect("opens"), common.join("graft"));
    assert_eq!(
        git(
            repo,
            &[
                "for-each-ref",
                "--format=%(refname)",
                "refs/graft/",
                "refs/tributary/"
            ]
        ),
        "refs/graft/backup/main/1",
        "nothing moved, nothing lost"
    );
}

/// Every outcome's words: the two that could not finish say so, a move
/// that finished says nothing — unless its journal line failed or an older
/// build wrote under the old names, whose kept-aside folder is said
/// relative to the repository.
#[test]
fn every_outcome_has_its_words_or_none() {
    let root = std::path::Path::new("/work/api");
    let said = |outcome: SettleOutcome| words(&outcome, root);
    let read_back = MigrateError::ReadBack {
        what: "refs/tributary/backup/main/1".to_owned(),
    };
    assert_eq!(
        said(SettleOutcome::KeptOld(read_back.clone())).as_deref(),
        Some(
            "rename: still using the old names \u{b7} refs/tributary/backup/main/1 read back different"
        )
    );
    assert_eq!(
        said(SettleOutcome::LeftOld(read_back)).as_deref(),
        Some(
            "rename: old names left behind \u{b7} refs/tributary/backup/main/1 read back different"
        )
    );
    assert_eq!(
        said(SettleOutcome::Migrated {
            refs: 3,
            files: 2,
            not_journaled: Some("disk full".to_owned()),
            kept_aside: None,
        })
        .as_deref(),
        Some("rename: done \u{b7} not in the journal: disk full")
    );
    older_version_words(root);
    for quiet in [
        SettleOutcome::Done,
        SettleOutcome::Migrated {
            refs: 3,
            files: 2,
            not_journaled: None,
            kept_aside: None,
        },
        SettleOutcome::CleanedUp {
            refs: 1,
            files: 1,
            kept_aside: None,
        },
    ] {
        assert_eq!(said(quiet.clone()), None, "{quiet:?}");
    }
}

/// [`every_outcome_has_its_words_or_none`]'s half for an older build's
/// writes, kept aside or not.
fn older_version_words(root: &std::path::Path) {
    let said = |outcome: SettleOutcome| words(&outcome, root);
    let kept = || Some(PathBuf::from("/work/api/.git/tributary/older-version/1"));
    let older = "rename: an older version wrote under the old names \u{b7} close it";
    let older_kept = format!("{older}; kept in .git/tributary/older-version/1");
    for outcome in [
        SettleOutcome::OlderWrote {
            refs: 1,
            files: 1,
            kept_aside: kept(),
        },
        SettleOutcome::Migrated {
            refs: 3,
            files: 2,
            not_journaled: None,
            kept_aside: kept(),
        },
        SettleOutcome::CleanedUp {
            refs: 1,
            files: 1,
            kept_aside: kept(),
        },
    ] {
        assert_eq!(
            said(outcome.clone()).as_deref(),
            Some(older_kept.as_str()),
            "{outcome:?}"
        );
    }
    assert_eq!(
        said(SettleOutcome::OlderWrote {
            refs: 1,
            files: 0,
            kept_aside: None,
        })
        .as_deref(),
        Some(older)
    );
    assert_eq!(
        said(SettleOutcome::Migrated {
            refs: 3,
            files: 2,
            not_journaled: Some("disk full".to_owned()),
            kept_aside: kept(),
        })
        .as_deref(),
        Some(format!("{older_kept} \u{b7} not in the journal: disk full").as_str())
    );
}
