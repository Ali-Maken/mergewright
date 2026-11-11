//! A fetch records the refs it moved, outside the undo record (ADR-0062,
//! m8-polish-and-ship/21c).
//!
//! `git fetch --all` updates one remote's tracking refs and then fails on
//! another: the entry is `Aborted`, and before 21c it said nothing moved —
//! J marked it `nothing written` while `origin/main` stood at the
//! teammate's tip. Every claim here is read back from the repository
//! (`for-each-ref`) and from the journal file, never from a return value
//! alone.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mergewright_core::journal::{EntryStatus, JournalEntry, OperationKind, RefMove};
use mergewright_core::oid::Oid;
use mergewright_fixtures::Shape;
use mergewright_fixtures::remote::LocalRemote;
use mergewright_git::remote::upstream_of;
use mergewright_git::repo::RepoHandle;
use mergewright_ops::apply::{Applied, ApplyError, apply};
use mergewright_ops::journal::{in_force, next_undo, wrote_anything};
use mergewright_ops::operation::{CommitParams, FetchParams, Operation, PullParams, Target};
use mergewright_ops::undo::{Step, plan_revert_to, plan_undo};

use crate::support::{NOW, Never, context, journal_for};
use mergewright_core::on_disk::DIR;

/// A remote that is configured but has nothing behind it: git's fetch of it
/// fails with `does not appear to be a git repository`.
const GONE: &str = "file:///nonexistent/mwr-21c.git";

fn local() -> LocalRemote {
    LocalRemote::new(Shape::Linear).expect("the Linear shape clones into a local remote")
}

fn git(remote: &LocalRemote, dir: &Path, args: &[&str]) -> String {
    remote
        .git(dir, args)
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"))
}

fn oid(hex: &str) -> Oid {
    Oid::from_hex(hex).expect("a 40-hex id")
}

/// `refs/remotes/origin/<default>`.
fn tracking(remote: &LocalRemote) -> String {
    format!("refs/remotes/origin/{}", remote.default_branch())
}

/// A ref's value read from the repository, `None` when it is absent.
fn ref_value(remote: &LocalRemote, name: &str) -> Option<Oid> {
    let listed = git(
        remote,
        remote.work(),
        &["for-each-ref", "--format=%(objectname)", name],
    );
    (!listed.is_empty()).then(|| oid(&listed))
}

/// Apply one operation in the work clone at clock `at`, headless.
fn run_in(remote: &LocalRemote, op: &Operation, at: i64) -> Result<Applied, ApplyError> {
    let repo = RepoHandle::open(remote.work()).expect("the repository opens");
    let mut journal = journal_for(&repo);
    let never = Never;
    let mut ctx = context(&repo, &mut journal, at, &never);
    apply(&mut ctx, op)
}

/// The journal as the file holds it.
fn entries(remote: &LocalRemote) -> Vec<JournalEntry> {
    let repo = RepoHandle::open(remote.work()).expect("the repository opens");
    journal_for(&repo).entries().to_vec()
}

fn fetch_all(prune: bool) -> Operation {
    Operation::Fetch(FetchParams {
        remote: None,
        prune,
    })
}

/// The probe of 21b-3 NOTES item 7: `origin` brings the teammate's commit,
/// `gone` fails, git exits non-zero. The entry is `Aborted`, records the one
/// ref that moved, and is still not in force — `u` has nothing to take back.
#[test]
fn a_fetch_that_moved_refs_before_one_remote_failed_records_them() {
    let remote = local();
    let tracking = tracking(&remote);
    let old = ref_value(&remote, &tracking).expect("the clone has its tracking ref");
    let tip = oid(&remote.commit_on_remote("from a teammate").unwrap());
    git(&remote, remote.work(), &["remote", "add", "gone", GONE]);

    let Err(error) = run_in(&remote, &fetch_all(false), NOW) else {
        panic!("a fetch with one remote gone fails");
    };
    let ApplyError::Remote { entry, .. } = &error else {
        panic!("a fetch with one remote gone fails as a remote failure: {error}");
    };
    let status = error.explain(&crate::support::Letters);
    assert!(
        status.ends_with(" \u{b7} origin/main moved before it stopped"),
        "the status line names what moved: {status}"
    );

    assert_eq!(
        ref_value(&remote, &tracking),
        Some(tip),
        "origin/main moved"
    );
    assert_eq!(entry.kind, OperationKind::Fetch);
    assert!(
        matches!(entry.status, EntryStatus::Aborted { .. }),
        "{:?}",
        entry.status
    );
    let expected = BTreeMap::from([(
        tracking.clone(),
        RefMove {
            before: Some(old),
            after: Some(tip),
        },
    )]);
    assert_eq!(entry.fetched_refs, Some(expected));
    // Read back from the file, not the returned value.
    let journaled = entries(&remote);
    assert_eq!(journaled.last(), Some(&**entry));
    assert!(wrote_anything(entry), "a ref moved: something was written");
    assert!(
        !in_force(&journaled, &entry.id),
        "`u` has nothing to take back: tracking refs are outside the undo record"
    );
}

/// Nothing new on the remote: the listing was read and nothing differs —
/// `Some({})`, which J reads as `nothing new`, not "unknown".
#[test]
fn a_fetch_that_brought_nothing_records_an_empty_map() {
    let remote = local();
    git(&remote, remote.work(), &["fetch", "-q", "origin"]);

    let applied = run_in(&remote, &fetch_all(false), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    let entry = &applied.entry;
    assert_eq!(entry.status, EntryStatus::Committed);
    assert_eq!(entry.fetched_refs, Some(BTreeMap::new()));
    assert_eq!(entries(&remote).last(), Some(entry));
}

/// One fetch with `--prune`: a tracking ref moved, one new, one pruned, and a
/// tag git followed — each recorded once, with its two values as the
/// repositories hold them. `origin/HEAD` (a symbolic ref) is not a move of
/// its own, and a ref under the app's own `on_disk::REF_PREFIX` is never
/// recorded, even when a refspec makes the fetch write there.
#[test]
fn a_fetch_records_new_moved_pruned_and_tag_refs() {
    let remote = local();
    let main = tracking(&remote);
    let base = oid(&git(&remote, remote.bare(), &["rev-parse", "HEAD"]));
    git(
        &remote,
        remote.bare(),
        &["branch", "old", &base.to_string()],
    );
    git(&remote, remote.work(), &["fetch", "-q", "origin"]);
    git(
        &remote,
        remote.work(),
        &["remote", "set-head", "origin", remote.default_branch()],
    );
    git(
        &remote,
        remote.work(),
        &[
            "config",
            "--add",
            "remote.origin.fetch",
            &format!("+refs/heads/*:refs/{DIR}/fetched/*"),
        ],
    );
    let old_main = ref_value(&remote, &main).expect("the tracking ref");
    git(&remote, remote.bare(), &["branch", "-D", "old"]);
    let tip = oid(&remote.commit_on_remote("from a teammate").unwrap());
    git(
        &remote,
        remote.bare(),
        &["branch", "feat", &tip.to_string()],
    );
    git(&remote, remote.bare(), &["tag", "v1.2", &tip.to_string()]);

    let applied = run_in(&remote, &fetch_all(true), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    // What git did, read back: the four moves happened, the symbolic ref is
    // there, and the refspec wrote under the app's own prefix.
    assert_eq!(ref_value(&remote, &main), Some(tip));
    assert_eq!(ref_value(&remote, "refs/remotes/origin/feat"), Some(tip));
    assert_eq!(ref_value(&remote, "refs/remotes/origin/old"), None);
    assert_eq!(ref_value(&remote, "refs/tags/v1.2"), Some(tip));
    assert!(
        git(
            &remote,
            remote.work(),
            &["symbolic-ref", "refs/remotes/origin/HEAD"]
        )
        .ends_with(remote.default_branch())
    );
    assert_eq!(
        ref_value(&remote, &format!("refs/{DIR}/fetched/feat")),
        Some(tip)
    );

    let change = |before, after| RefMove { before, after };
    let expected = BTreeMap::from([
        (
            "refs/remotes/origin/feat".to_owned(),
            change(None, Some(tip)),
        ),
        (main.clone(), change(Some(old_main), Some(tip))),
        (
            "refs/remotes/origin/old".to_owned(),
            change(Some(base), None),
        ),
        ("refs/tags/v1.2".to_owned(), change(None, Some(tip))),
    ]);
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert_eq!(applied.entry.fetched_refs, Some(expected));
    assert_eq!(entries(&remote).last(), Some(&applied.entry));
}

/// `u` is unchanged by a fetch that moved refs before it failed: it still
/// takes back the local change before it, and its plan touches no tracking
/// ref (ADR-0037 §3, ADR-0062 decision 2).
#[test]
fn u_after_a_partial_fetch_takes_back_the_change_before_it() {
    let remote = local();
    let commit = commit_through_apply(&remote, "local.txt", NOW);
    remote.commit_on_remote("from a teammate").unwrap();
    git(&remote, remote.work(), &["remote", "add", "gone", GONE]);

    let Err(ApplyError::Remote { entry: fetch, .. }) = run_in(&remote, &fetch_all(false), NOW + 10)
    else {
        panic!("a fetch with one remote gone fails as a remote failure");
    };
    assert!(
        fetch
            .fetched_refs
            .as_ref()
            .is_some_and(|moved| moved.contains_key(&tracking(&remote))),
        "the fetch moved origin's tracking ref: {:?}",
        fetch.fetched_refs
    );

    let journaled = entries(&remote);
    assert_eq!(
        next_undo(&journaled).map(|entry| &entry.id),
        Some(&commit.id)
    );
    let plan = plan_undo(&journaled).expect("the commit is undoable");
    assert_eq!(plan.reverses, commit.id);
    for step in &plan.steps {
        if let Step::UpdateRef { name, .. } = step {
            assert!(!name.starts_with("refs/remotes/"), "{step:?}");
        }
    }
    assert!(
        plan.touches
            .refs
            .iter()
            .all(|name| !name.starts_with("refs/remotes/")),
        "{:?}",
        plan.touches
    );
}

/// A pull whose fetch moved `origin/<default>` and then failed: git refuses
/// to clobber a tag the work clone already holds under another value
/// (`refs/tags/*:refs/tags/*`, not forced), and exits non-zero after the
/// branch's tracking ref is updated. The entry records that one move.
#[test]
fn a_pull_whose_fetch_failed_records_what_it_moved() {
    let remote = local();
    let main = tracking(&remote);
    let old_main = ref_value(&remote, &main).expect("the tracking ref");
    let base = git(&remote, remote.bare(), &["rev-parse", "HEAD"]);
    git(
        &remote,
        remote.work(),
        &[
            "config",
            "--add",
            "remote.origin.fetch",
            "refs/tags/*:refs/tags/*",
        ],
    );
    git(&remote, remote.work(), &["tag", "v1", &base]);
    let tip = oid(&remote.commit_on_remote("from a teammate").unwrap());
    git(&remote, remote.bare(), &["tag", "v1", &tip.to_string()]);

    let Err(ApplyError::Remote { entry, .. }) = run_in(&remote, &pull(&remote), NOW) else {
        panic!("a pull whose fetch is refused fails as a remote failure");
    };

    assert_eq!(ref_value(&remote, &main), Some(tip), "origin's tip came in");
    assert_eq!(
        ref_value(&remote, "refs/tags/v1"),
        Some(oid(&base)),
        "git kept his tag"
    );
    assert_eq!(entry.kind, OperationKind::Pull);
    assert!(
        matches!(entry.status, EntryStatus::Aborted { .. }),
        "{:?}",
        entry.status
    );
    assert_eq!(
        entry.fetched_refs,
        Some(BTreeMap::from([(
            main,
            RefMove {
                before: Some(old_main),
                after: Some(tip),
            }
        )]))
    );
    assert_eq!(entries(&remote).last(), Some(&*entry));
}

/// The fetch went through, `merge --ff-only` refused: the entry records the
/// tracking ref the fetch moved and not the local branch, and `u` and the
/// time machine plan exactly what they planned before the pull.
#[test]
fn a_diverged_pull_records_its_fetch_but_u_is_unchanged() {
    let remote = local();
    let main = tracking(&remote);
    let old_main = ref_value(&remote, &main).expect("the tracking ref");
    let theirs = oid(&remote.commit_on_remote("theirs").unwrap());
    let commit = commit_through_apply(&remote, "ours.txt", NOW);
    let before_pull = entries(&remote);
    let undo_before = plan_undo(&before_pull).expect("the commit is undoable");
    let revert_before = plan_revert_to(&before_pull, &commit.id).expect("a plan");

    let Err(ApplyError::Remote { entry, .. }) = run_in(&remote, &pull(&remote), NOW + 10) else {
        panic!("a diverged pull fails as a remote failure");
    };

    assert_eq!(ref_value(&remote, &main), Some(theirs));
    assert!(
        matches!(entry.status, EntryStatus::Aborted { .. }),
        "{:?}",
        entry.status
    );
    assert_eq!(
        entry.fetched_refs,
        Some(BTreeMap::from([(
            main,
            RefMove {
                before: Some(old_main),
                after: Some(theirs),
            }
        )]))
    );
    let after_pull = entries(&remote);
    assert_eq!(after_pull.last(), Some(&*entry));
    assert_eq!(
        next_undo(&after_pull).map(|entry| &entry.id),
        Some(&commit.id)
    );
    assert_eq!(plan_undo(&after_pull), Ok(undo_before));
    assert_eq!(plan_revert_to(&after_pull, &commit.id), Ok(revert_before));
}

/// A pull of the head branch against its upstream as configured now.
fn pull(remote: &LocalRemote) -> Operation {
    let repo = RepoHandle::open(remote.work()).expect("the work clone opens");
    let branch = format!("refs/heads/{}", remote.default_branch());
    let upstream = upstream_of(&repo, &branch)
        .expect("the upstream reads")
        .expect("the work clone tracks origin");
    Operation::Pull(PullParams { branch, upstream })
}

/// Stage and commit one new file through `apply`, so the journal holds the
/// commit entry. The work clone's own identity: `apply`'s git reads config.
fn commit_through_apply(remote: &LocalRemote, name: &str, at: i64) -> JournalEntry {
    git(
        remote,
        remote.work(),
        &["config", "user.name", "Maker Test"],
    );
    git(
        remote,
        remote.work(),
        &["config", "user.email", "test@maker.invalid"],
    );
    std::fs::write(remote.work().join(name), format!("{name}\n")).unwrap();
    run_in(
        remote,
        &Operation::Stage(Target::Paths(vec![PathBuf::from(name)])),
        at,
    )
    .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    let commit = Operation::Commit(CommitParams {
        message: format!("add {name}"),
        sign_off: false,
        no_verify: true,
        allow_empty: false,
        include: mergewright_ops::operation::Include::Staged,
    });
    run_in(remote, &commit, at + 1)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
        .entry
}
