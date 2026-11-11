//! The force-push probe on a remote-tracking ref whose reflog holds only its
//! creation entry (a branch pushed once), and on one with no reflog at all
//! (pre-publish-cleanup/13, review M2).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_ops::mistake_radar::probes::{ForcePushVerdict, probe_force_push};

use crate::support::{Never, TempRepo};

fn repo_with_remote_ref(log: bool) -> (TempRepo, Oid, RefName) {
    let repo = TempRepo::init();
    std::fs::write(repo.path().join("a"), "a").expect("write");
    repo.commit_all("one");
    repo.git(&["config", "core.logAllRefUpdates", "false"]);
    let tip = repo.git(&["rev-parse", "HEAD"]);
    let tip = tip.trim();
    let mut args = vec!["update-ref"];
    if log {
        args.push("--create-reflog");
    }
    args.extend(["refs/remotes/origin/feature", tip]);
    repo.git(&args);
    let oid = Oid::from_hex(tip).expect("a full id");
    let name = RefName::new("refs/remotes/origin/feature").expect("a ref name");
    (repo, oid, name)
}

#[test]
fn a_branch_pushed_once_with_a_creation_only_reflog_is_clean() {
    let (repo, tip, name) = repo_with_remote_ref(true);
    let verdict = probe_force_push(&repo.open(), "feature", &name, tip, &Never);
    assert_eq!(verdict, ForcePushVerdict::Clean);
}

#[test]
fn a_remote_ref_with_no_reflog_is_still_not_checkable() {
    let (repo, tip, name) = repo_with_remote_ref(false);
    let verdict = probe_force_push(&repo.open(), "feature", &name, tip, &Never);
    assert_eq!(
        verdict,
        ForcePushVerdict::NotCheckable("no remote-tracking reflog".into())
    );
}
