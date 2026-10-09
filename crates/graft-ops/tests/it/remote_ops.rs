//! Fetch, pull and push through `apply`, against real remotes: a `file://`
//! bare (`LocalRemote`) everywhere, smart HTTP (`HttpRemote`) where auth
//! matters. Every claim about git is read back from the repositories
//! (`rev-parse`, `for-each-ref`, `git config`, file bytes), never taken from
//! a return value alone (the M3 group-03 standard).
//!
//! `apply`'s git keeps the user's own config (only `LocalRemote`'s own git
//! is hermetic), so the HTTP tests blank `credential.helper` in the work
//! clone: a keychain helper must neither answer for the test nor store its
//! password.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use graft_core::journal::{EntryStatus, JournalEntry, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_fixtures::Shape;
use graft_fixtures::remote::{HttpRemote, LocalRemote};
use graft_git::remote::{RemoteFailure, upstream_of};
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;
use graft_git::run::askpass::{AskPass, Secret};
use graft_ops::apply::{Applied, ApplyError, Refresh, RemoteHooks, apply};
use graft_ops::journal::{next_redo, next_undo};
use graft_ops::operation::{
    CommitParams, FetchParams, Lease, Operation, PullParams, PushParams, Refusal, Target,
};
use graft_ops::undo;

use crate::support::{NOW, Never, context, journal_for};

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

/// `refs/heads/<default>`.
fn branch(remote: &LocalRemote) -> String {
    format!("refs/heads/{}", remote.default_branch())
}

/// `refs/remotes/origin/<default>`.
fn tracking(remote: &LocalRemote) -> String {
    format!("refs/remotes/origin/{}", remote.default_branch())
}

/// Apply one operation in `dir` at clock `at`, headless.
fn run_in(dir: &Path, op: &Operation, at: i64) -> Result<Applied, ApplyError> {
    let repo = RepoHandle::open(dir).expect("the repository opens");
    let mut journal = journal_for(&repo);
    let never = Never;
    let mut ctx = context(&repo, &mut journal, at, &never);
    apply(&mut ctx, op)
}

/// Apply with hooks and a stop of the caller's choosing.
fn run_hooked(
    dir: &Path,
    op: &Operation,
    at: i64,
    hooks: RemoteHooks,
    stop: &dyn Stop,
) -> Result<Applied, ApplyError> {
    let repo = RepoHandle::open(dir).expect("the repository opens");
    let mut journal = journal_for(&repo);
    let mut ctx = context(&repo, &mut journal, at, stop);
    ctx.remote = Some(hooks);
    apply(&mut ctx, op)
}

fn undo_in(dir: &Path, at: i64) -> Applied {
    let repo = RepoHandle::open(dir).expect("the repository opens");
    let mut journal = journal_for(&repo);
    let never = Never;
    let mut ctx = context(&repo, &mut journal, at, &never);
    undo::undo(&mut ctx, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
}

fn redo_in(dir: &Path, at: i64) -> Applied {
    let repo = RepoHandle::open(dir).expect("the repository opens");
    let mut journal = journal_for(&repo);
    let never = Never;
    let mut ctx = context(&repo, &mut journal, at, &never);
    undo::redo(&mut ctx, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
}

fn entries(dir: &Path) -> Vec<JournalEntry> {
    let repo = RepoHandle::open(dir).expect("the repository opens");
    journal_for(&repo).entries().to_vec()
}

fn fetch_all() -> Operation {
    Operation::Fetch(FetchParams {
        remote: None,
        prune: true,
    })
}

/// A pull of the head branch against its upstream as configured now.
fn pull(remote: &LocalRemote) -> Operation {
    let repo = RepoHandle::open(remote.work()).expect("the work clone opens");
    let upstream = upstream_of(&repo, &branch(remote))
        .expect("the upstream reads")
        .expect("the work clone tracks origin");
    Operation::Pull(PullParams {
        branch: branch(remote),
        upstream,
    })
}

fn push(remote: &LocalRemote) -> PushParams {
    PushParams {
        branch: branch(remote),
        remote: "origin".to_owned(),
        remote_branch: branch(remote),
        set_upstream: false,
        lease: None,
        source_commit: None,
    }
}

fn aborted_reason(entry: &JournalEntry) -> &str {
    match &entry.status {
        EntryStatus::Aborted { reason } => reason,
        other => panic!("expected Aborted, got {other:?}"),
    }
}

/// A ref's value, or `None` when it is absent.
fn ref_value(remote: &LocalRemote, dir: &Path, name: &str) -> Option<String> {
    let listed = git(
        remote,
        dir,
        &["for-each-ref", "--format=%(objectname)", name],
    );
    (!listed.is_empty()).then_some(listed)
}

#[test]
fn fetch_moves_tracking_ref_and_journals() {
    let remote = local();
    let tip = remote.commit_on_remote("from a teammate").unwrap();
    assert_ne!(
        ref_value(&remote, remote.work(), &tracking(&remote)),
        Some(tip.clone())
    );

    let applied = run_in(remote.work(), &fetch_all(), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(
        ref_value(&remote, remote.work(), &tracking(&remote)),
        Some(tip)
    );
    let entry = &applied.entry;
    assert_eq!(entry.kind, OperationKind::Fetch);
    assert_eq!(entry.status, EntryStatus::Committed);
    assert!(entry.refs_before.is_empty() && entry.refs_after.is_empty());
    assert_eq!(entry.commands, ["git fetch --progress --prune --all"]);
    assert_eq!(entry.description, "fetch all remotes");
    assert_eq!(applied.refresh, Refresh::Everything);
    // Read back from the file, not the returned value.
    assert_eq!(entries(remote.work()).last(), Some(entry));
}

#[test]
fn fetch_prunes() {
    let remote = local();
    let tip = git(&remote, remote.bare(), &["rev-parse", "HEAD"]);
    git(&remote, remote.bare(), &["branch", "gone", &tip]);
    git(&remote, remote.work(), &["fetch", "-q", "origin"]);
    assert!(ref_value(&remote, remote.work(), "refs/remotes/origin/gone").is_some());
    git(&remote, remote.bare(), &["branch", "-D", "gone"]);

    run_in(remote.work(), &fetch_all(), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(
        ref_value(&remote, remote.work(), "refs/remotes/origin/gone"),
        None
    );
    assert!(ref_value(&remote, remote.work(), &tracking(&remote)).is_some());
}

#[test]
fn fetch_allowed_in_bare() {
    let remote = local();
    let mirror = remote.bare().parent().unwrap().join("mirror.git");
    let bare = remote.bare().to_str().unwrap().to_owned();
    git(
        &remote,
        remote.bare().parent().unwrap(),
        &["clone", "-q", "--bare", &bare, "mirror.git"],
    );
    let tip = remote.commit_on_remote("after the mirror").unwrap();

    let op = Operation::Fetch(FetchParams {
        remote: Some("origin".to_owned()),
        prune: false,
    });
    let applied = run_in(&mirror, &op, NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(applied.entry.status, EntryStatus::Committed);
    // A bare clone has no fetch refspec: what the fetch got is FETCH_HEAD.
    let fetched = git(&remote, &mirror, &["rev-parse", "FETCH_HEAD"]);
    assert_eq!(fetched, tip);
}

#[test]
fn pull_fast_forwards_then_undo_keeps_tracking_ref() {
    let remote = local();
    let before = git(&remote, remote.work(), &["rev-parse", "HEAD"]);
    let tip = remote.commit_on_remote("from a teammate").unwrap();
    git(&remote, remote.work(), &["fetch", "-q", "origin"]);
    let added = git(
        &remote,
        remote.work(),
        &["diff-tree", "--no-commit-id", "--name-only", "-r", &tip],
    );
    let file = remote.work().join(&added);
    assert!(!file.exists());

    let applied = run_in(remote.work(), &pull(&remote), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(
        git(&remote, remote.work(), &["rev-parse", &branch(&remote)]),
        tip
    );
    let expected = git(
        &remote,
        remote.work(),
        &["cat-file", "blob", &format!("{tip}:{added}")],
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap().trim_end(), expected);
    let entry = &applied.entry;
    assert_eq!(entry.kind, OperationKind::Pull);
    assert_eq!(
        entry.touches,
        Touches {
            refs: vec![branch(&remote)],
            index: true,
            worktree: true
        }
    );
    assert!(entry.backup_ref.is_some());
    assert_eq!(applied.refresh, Refresh::Everything);

    let undone = undo_in(remote.work(), NOW + 10);
    assert_eq!(undone.entry.kind, OperationKind::Undo);
    assert_eq!(
        git(&remote, remote.work(), &["rev-parse", &branch(&remote)]),
        before
    );
    assert!(!file.exists(), "the pulled file is gone again");
    assert_eq!(
        ref_value(&remote, remote.work(), &tracking(&remote)),
        Some(tip.clone())
    );

    redo_in(remote.work(), NOW + 20);
    assert_eq!(
        git(&remote, remote.work(), &["rev-parse", &branch(&remote)]),
        tip
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap().trim_end(), expected);
}

#[test]
fn pull_diverged_is_refused_after_fetching() {
    let remote = local();
    let theirs = remote.commit_on_remote("theirs").unwrap();
    let ours = remote.commit_locally("ours").unwrap();

    let Err(ApplyError::Remote { entry, failure, .. }) = run_in(remote.work(), &pull(&remote), NOW)
    else {
        panic!("a diverged pull fails as a remote failure");
    };

    assert_eq!(failure, RemoteFailure::NotFastForward);
    assert_eq!(
        aborted_reason(&entry),
        "diverged: not possible to fast-forward"
    );
    assert_eq!(
        git(&remote, remote.work(), &["rev-parse", &branch(&remote)]),
        ours
    );
    assert_eq!(
        ref_value(&remote, remote.work(), &tracking(&remote)),
        Some(theirs)
    );
}

/// Was `pull_dirty_file_refused_bytes_unchanged`: before m7-power-tools/03b
/// git refused this pull over his edit. Now the edit is set aside, the pull
/// fast-forwards, and his edit — on the line the pull changed — clashes on
/// the way back: a clash, his bytes kept in the set-aside stash, which the
/// resolver starts from. The full set-aside behaviour is `pull_set_aside`.
#[test]
fn pull_dirty_file_sets_it_aside_bytes_kept() {
    let remote = local();
    // The work clone commits a change to a tracked file, pushes it, and steps
    // back: origin is one commit ahead, and that commit changes `path`.
    let path = git(&remote, remote.work(), &["ls-files"])
        .lines()
        .next()
        .unwrap()
        .to_owned();
    let file = remote.work().join(&path);
    std::fs::write(&file, b"changed upstream\n").unwrap();
    git(
        &remote,
        remote.work(),
        &["commit", "-q", "-am", "change upstream"],
    );
    git(&remote, remote.work(), &["push", "-q", "origin", "HEAD"]);
    let upstream = git(&remote, remote.work(), &["rev-parse", "HEAD"]);
    git(&remote, remote.work(), &["reset", "-q", "--hard", "HEAD~1"]);
    std::fs::write(&file, b"my uncommitted edit\n").unwrap();

    let Err(ApplyError::Clashed { entry, clash, .. }) = run_in(remote.work(), &pull(&remote), NOW)
    else {
        panic!("a pull over an edit to the line it changes clashes on the put-back");
    };

    assert_eq!(clash.paths, vec![PathBuf::from(&path)]);
    assert_eq!(entry.status, EntryStatus::Committed);
    assert_eq!(
        git(&remote, remote.work(), &["rev-parse", "HEAD"]),
        upstream
    );
    assert_eq!(
        git(
            &remote,
            remote.work(),
            &["show", &format!("{}:{path}", clash.stash)]
        ),
        "my uncommitted edit",
        "his bytes kept in the set-aside stash"
    );
}

#[test]
fn pull_refusals() {
    let remote = local();
    let op = pull(&remote);
    let journaled = entries(remote.work()).len();

    git(&remote, remote.work(), &["branch", "--unset-upstream"]);
    match run_in(remote.work(), &op, NOW) {
        Err(ApplyError::Refused(Refusal::NoUpstream { branch })) => {
            assert_eq!(branch, remote.default_branch());
        }
        other => panic!("expected NoUpstream, got {other:?}"),
    }

    git(&remote, remote.work(), &["checkout", "-q", "--detach"]);
    assert!(matches!(
        run_in(remote.work(), &op, NOW + 10),
        Err(ApplyError::Refused(Refusal::DetachedHead))
    ));
    assert_eq!(
        entries(remote.work()).len(),
        journaled,
        "a refusal journals nothing"
    );
}

/// Stage and commit one new file through `apply`, so the journal holds the
/// commit entry. The work clone's own identity: `apply`'s git reads config.
fn commit_through_apply(remote: &LocalRemote, name: &str, at: i64) -> JournalEntry {
    git(
        remote,
        remote.work(),
        &["config", "user.name", "Graft Test"],
    );
    git(
        remote,
        remote.work(),
        &["config", "user.email", "test@graft.invalid"],
    );
    std::fs::write(remote.work().join(name), format!("{name}\n")).unwrap();
    run_in(
        remote.work(),
        &Operation::Stage(Target::Paths(vec![PathBuf::from(name)])),
        at,
    )
    .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    let commit = Operation::Commit(CommitParams {
        message: format!("add {name}"),
        sign_off: false,
        no_verify: true,
        allow_empty: false,
    });
    run_in(remote.work(), &commit, at + 1)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
        .entry
}

#[test]
fn push_then_undo_touches_only_local() {
    let remote = local();
    let old = git(&remote, remote.work(), &["rev-parse", "HEAD"]);
    let commit = commit_through_apply(&remote, "local.txt", NOW);
    let new = git(&remote, remote.work(), &["rev-parse", "HEAD"]);

    let applied = run_in(remote.work(), &Operation::Push(push(&remote)), NOW + 10)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(
        git(&remote, remote.bare(), &["rev-parse", &branch(&remote)]),
        new
    );
    let entry = &applied.entry;
    assert_eq!(entry.kind, OperationKind::Push);
    assert_eq!(
        entry.refs_before.get(&tracking(&remote)),
        Some(&Some(oid(&old)))
    );
    assert_eq!(
        entry.refs_after.get(&tracking(&remote)),
        Some(&Some(oid(&new)))
    );
    let journal = entries(remote.work());
    assert_eq!(
        next_undo(&journal).map(|e| &e.id),
        Some(&commit.id),
        "u skips the push"
    );

    undo_in(remote.work(), NOW + 20);
    assert_eq!(
        git(&remote, remote.work(), &["rev-parse", &branch(&remote)]),
        old
    );
    assert_eq!(
        git(&remote, remote.bare(), &["rev-parse", &branch(&remote)]),
        new
    );
    assert_eq!(
        ref_value(&remote, remote.work(), &tracking(&remote)),
        Some(new)
    );
}

#[test]
fn push_sets_upstream() {
    let remote = local();
    git(&remote, remote.work(), &["checkout", "-q", "-b", "topic"]);
    remote.commit_locally("on topic").unwrap();
    let op = Operation::Push(PushParams {
        branch: "refs/heads/topic".to_owned(),
        remote: "origin".to_owned(),
        remote_branch: "refs/heads/topic".to_owned(),
        set_upstream: true,
        lease: None,
        source_commit: None,
    });
    assert!(op.describe().ends_with("(set upstream)"));

    run_in(remote.work(), &op, NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(
        git(&remote, remote.work(), &["config", "branch.topic.remote"]),
        "origin"
    );
    assert_eq!(
        git(&remote, remote.work(), &["config", "branch.topic.merge"]),
        "refs/heads/topic"
    );
    assert_eq!(
        git(&remote, remote.bare(), &["rev-parse", "refs/heads/topic"]),
        git(&remote, remote.work(), &["rev-parse", "HEAD"])
    );
}

#[test]
fn push_non_fast_forward_classified() {
    let remote = local();
    let theirs = remote.commit_on_remote("theirs").unwrap();
    remote.commit_locally("ours").unwrap();

    let Err(ApplyError::Remote { entry, failure, .. }) =
        run_in(remote.work(), &Operation::Push(push(&remote)), NOW)
    else {
        panic!("a rejected push fails as a remote failure");
    };

    assert_eq!(failure, RemoteFailure::NonFastForward);
    assert!(matches!(entry.status, EntryStatus::Aborted { .. }));
    assert_eq!(
        git(&remote, remote.bare(), &["rev-parse", &branch(&remote)]),
        theirs
    );
}

/// Commit locally, push and fetch, so the tracking ref is at the pushed tip;
/// then amend. Returns the pushed tip (the lease's expected value).
fn pushed_then_amended(remote: &LocalRemote) -> String {
    remote.commit_locally("first draft").unwrap();
    git(remote, remote.work(), &["push", "-q", "origin", "HEAD"]);
    git(remote, remote.work(), &["fetch", "-q", "origin"]);
    let pushed = git(remote, remote.work(), &["rev-parse", &tracking(remote)]);
    git(
        remote,
        remote.work(),
        &["commit", "-q", "--amend", "-m", "second draft"],
    );
    pushed
}

#[test]
fn lease_push_succeeds_after_amend() {
    let remote = local();
    let pushed = pushed_then_amended(&remote);
    let amended = git(&remote, remote.work(), &["rev-parse", "HEAD"]);
    let op = Operation::Push(PushParams {
        lease: Some(Lease {
            expected: oid(&pushed),
        }),
        ..push(&remote)
    });

    run_in(remote.work(), &op, NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(
        git(&remote, remote.bare(), &["rev-parse", &branch(&remote)]),
        amended
    );
}

#[test]
fn lease_push_refused_when_stale() {
    let remote = local();
    let pushed = pushed_then_amended(&remote);
    // Someone else pushes after our last fetch.
    let theirs = remote.commit_on_remote("theirs").unwrap();
    let op = Operation::Push(PushParams {
        lease: Some(Lease {
            expected: oid(&pushed),
        }),
        ..push(&remote)
    });

    let Err(ApplyError::Remote { failure, entry, .. }) = run_in(remote.work(), &op, NOW) else {
        panic!("a stale lease fails as a remote failure");
    };

    assert_eq!(failure, RemoteFailure::StaleInfo);
    assert!(matches!(entry.status, EntryStatus::Aborted { .. }));
    assert_eq!(
        git(&remote, remote.bare(), &["rev-parse", &branch(&remote)]),
        theirs
    );
}

/// The case git's own lease cannot catch: their commit was **fetched** after
/// the box read `expected`, so the tracking ref moved but nobody looked.
/// `check` refuses before git runs; nothing is journaled and the remote keeps
/// their commit (m4/04 review, M1).
#[test]
fn lease_push_refused_before_git_when_fetched_since() {
    let remote = local();
    let pushed = pushed_then_amended(&remote);
    let theirs = remote.commit_on_remote("theirs").unwrap();
    git(&remote, remote.work(), &["fetch", "-q", "origin"]);
    assert_eq!(
        ref_value(&remote, remote.work(), &tracking(&remote)),
        Some(theirs.clone()),
        "the tracking ref is at their commit"
    );
    let op = Operation::Push(PushParams {
        lease: Some(Lease {
            expected: oid(&pushed),
        }),
        ..push(&remote)
    });

    let result = run_in(remote.work(), &op, NOW);

    let Err(ApplyError::Refused(Refusal::LeaseStale { remote_side })) = result else {
        panic!("a lease behind the tracking ref is refused: {result:?}");
    };
    assert_eq!(remote_side, format!("origin/{}", remote.default_branch()));
    assert!(
        entries(remote.work()).is_empty(),
        "a refusal journals nothing"
    );
    assert_eq!(
        git(&remote, remote.bare(), &["rev-parse", &branch(&remote)]),
        theirs,
        "the remote is unchanged"
    );
}

/// A pull that finds nothing to do is journaled (its fetch ran) but is
/// transparent to `u`, like a fetch: `next_undo` stays on the commit, and
/// `u` undoes the commit (m4/04 review, M3).
#[test]
fn an_up_to_date_pull_is_transparent_to_undo() {
    let remote = local();
    let commit = commit_through_apply(&remote, "local.txt", NOW);
    let before = git(&remote, remote.work(), &["rev-parse", "HEAD"]);

    let pulled = run_in(remote.work(), &pull(&remote), NOW + 10)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(pulled.entry.kind, OperationKind::Pull);
    assert_eq!(pulled.entry.status, EntryStatus::Committed);
    let journal = entries(remote.work());
    assert_eq!(journal.last().map(|e| &e.id), Some(&pulled.entry.id));
    assert_eq!(next_undo(&journal).map(|e| &e.id), Some(&commit.id));

    let undone = undo_in(remote.work(), NOW + 20);
    assert_eq!(undone.entry.reverses.as_ref(), Some(&commit.id));
    assert_ne!(
        git(&remote, remote.work(), &["rev-parse", "HEAD"]),
        before,
        "one u undid the commit"
    );
}

/// A remote section Graft cannot read (a non-UTF-8 URL) blocks only the
/// operations that read remotes: a commit still goes through, a fetch is
/// refused with the reason (m4/04 review, M2).
#[test]
fn a_non_utf8_remote_url_does_not_block_a_commit() {
    let remote = local();
    let config = remote.work().join(".git/config");
    let mut bytes = std::fs::read(&config).unwrap();
    bytes.extend_from_slice(b"[remote \"odd\"]\n\turl = /srv/\xff\xfe.git\n");
    std::fs::write(&config, bytes).unwrap();

    let commit = commit_through_apply(&remote, "local.txt", NOW);
    assert_eq!(commit.kind, OperationKind::Commit);
    assert_eq!(commit.status, EntryStatus::Committed);

    let fetched = run_in(remote.work(), &fetch_all(), NOW + 10);
    assert!(
        matches!(fetched, Err(ApplyError::Facts(_))),
        "a remote operation still reports the unreadable config: {fetched:?}"
    );
}

#[test]
fn fetch_does_not_empty_redo() {
    let remote = local();
    commit_through_apply(&remote, "local.txt", NOW);
    let undone = undo_in(remote.work(), NOW + 10);
    remote.commit_on_remote("theirs").unwrap();
    run_in(remote.work(), &fetch_all(), NOW + 20)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    let journal = entries(remote.work());
    assert_eq!(journal.last().map(|e| e.kind), Some(OperationKind::Fetch));
    assert_eq!(next_redo(&journal).map(|e| &e.id), Some(&undone.entry.id));
}

/// Answers every prompt with one password, counting the asks.
struct Answers {
    password: &'static str,
    asked: AtomicUsize,
}

impl AskPass for Answers {
    fn ask(&self, _prompt: &str) -> Option<Secret> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        Some(Secret::new(self.password.as_bytes().to_vec()))
    }
}

/// The test stand-in for Graft's askpass mode (`src/bin/askpass_test_helper.rs`).
fn helper() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_askpass_test_helper"))
}

const USER: &str = "tester";
const PASSWORD: &str = "correct-horse-battery-staple";

/// Point `origin` at `url` and turn the user's credential helpers off for
/// this clone, so only the test's own answers count.
fn over_http(remote: &LocalRemote, url: &str) {
    git(remote, remote.work(), &["remote", "set-url", "origin", url]);
    git(remote, remote.work(), &["config", "credential.helper", ""]);
}

#[test]
fn http_wrong_password_is_auth_failed() {
    let remote = local();
    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).unwrap();
    let wrong = http
        .url()
        .replacen(&format!("{USER}@"), &format!("{USER}:wrong@"), 1);
    assert_ne!(wrong, http.url(), "the URL carries the user");
    over_http(&remote, &wrong);

    let op = Operation::Fetch(FetchParams {
        remote: Some("origin".to_owned()),
        prune: false,
    });
    let Err(ApplyError::Remote { failure, entry, .. }) = run_in(remote.work(), &op, NOW) else {
        panic!("a wrong password fails as a remote failure");
    };

    assert!(
        matches!(failure, RemoteFailure::AuthFailed { .. }),
        "{failure:?}"
    );
    assert!(aborted_reason(&entry).starts_with("authentication failed"));
    assert!(
        http.requests().1 >= 1,
        "the server rejected at least one request"
    );
}

#[test]
fn http_askpass_answers_and_progress_flows() {
    let remote = local();
    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).unwrap();
    over_http(&remote, &http.url());
    let tip = remote.commit_locally("over http").unwrap();
    let answers = Arc::new(Answers {
        password: PASSWORD,
        asked: AtomicUsize::new(0),
    });
    let progressed = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&progressed);
    let hooks = RemoteHooks {
        progress: Box::new(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        }),
        askpass: answers.clone(),
        helper: helper(),
    };

    run_hooked(
        remote.work(),
        &Operation::Push(push(&remote)),
        NOW,
        hooks,
        &Never,
    )
    .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(
        git(&remote, remote.bare(), &["rev-parse", &branch(&remote)]),
        tip
    );
    assert!(
        answers.asked.load(Ordering::SeqCst) >= 1,
        "git asked through the helper"
    );
    assert!(
        progressed.load(Ordering::SeqCst) >= 1,
        "progress reached the hook"
    );
    let repo = RepoHandle::open(remote.work()).unwrap();
    let journal_file = graft_ops::journal::journal_path(&repo).unwrap();
    let bytes = std::fs::read(journal_file).unwrap();
    assert!(
        !bytes
            .windows(PASSWORD.len())
            .any(|w| w == PASSWORD.as_bytes()),
        "the password is not in the journal"
    );
}

/// Stops once the progress hook has fired.
struct StopAfterProgress(Arc<AtomicBool>);

impl Stop for StopAfterProgress {
    fn should_stop(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A commit on the remote big enough that fetching it takes a while: 32 MiB
/// of incompressible bytes, written with plumbing straight into the bare.
fn big_commit_on_remote(remote: &LocalRemote) -> String {
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut bytes = Vec::with_capacity(32 << 20);
    while bytes.len() < 32 << 20 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        bytes.extend_from_slice(&state.to_le_bytes());
    }
    let blob_file = remote.bare().parent().unwrap().join("big.bin");
    std::fs::write(&blob_file, &bytes).unwrap();
    let bare = remote.bare();
    let blob = git(
        remote,
        bare,
        &["hash-object", "-w", blob_file.to_str().unwrap()],
    );
    let tree_line = format!("100644 blob {blob}\tbig.bin");
    let tree_file = remote.bare().parent().unwrap().join("tree.txt");
    std::fs::write(&tree_file, format!("{tree_line}\n")).unwrap();
    // `mktree` reads stdin; the fixture runner has none, so go through a
    // shell redirect inside git's own alias mechanism instead.
    let tree = git(
        remote,
        bare,
        &[
            "-c",
            &format!("alias.mk=!git mktree < '{}'", tree_file.display()),
            "mk",
        ],
    );
    let parent = git(remote, bare, &["rev-parse", &branch(remote)]);
    let commit = git(
        remote,
        bare,
        &["commit-tree", &tree, "-p", &parent, "-m", "big"],
    );
    git(remote, bare, &["update-ref", &branch(remote), &commit]);
    commit
}

#[test]
fn cancel_mid_fetch_is_clean() {
    let remote = local();
    let tracked_before = ref_value(&remote, remote.work(), &tracking(&remote));
    big_commit_on_remote(&remote);
    let flipped = Arc::new(AtomicBool::new(false));
    let flip = Arc::clone(&flipped);
    let hooks = RemoteHooks {
        progress: Box::new(move |_| flip.store(true, Ordering::SeqCst)),
        askpass: Arc::new(Answers {
            password: "unused",
            asked: AtomicUsize::new(0),
        }),
        helper: helper(),
    };
    let stop = StopAfterProgress(flipped);

    let result = run_hooked(remote.work(), &fetch_all(), NOW, hooks, &stop);

    let Err(ApplyError::Remote { failure, entry, .. }) = result else {
        panic!("the fetch was cancelled: {result:?}");
    };
    assert_eq!(failure, RemoteFailure::Cancelled);
    assert_eq!(aborted_reason(&entry), "cancelled");
    assert!(
        ApplyError::Remote {
            entry: entry.clone(),
            failure,
            stderr_tail: vec![],
            names: Box::new(fetch_all().remote_names().expect("a fetch has names")),
            moves_unread: None,
        }
        .explain(&crate::support::Letters)
        .ends_with("fetch all remotes cancelled")
    );
    assert_eq!(
        ref_value(&remote, remote.work(), &tracking(&remote)),
        tracked_before
    );
    let locks = git(
        &remote,
        remote.work(),
        &["-c", "core.quotepath=off", "rev-parse", "--git-dir"],
    );
    let git_dir = remote.work().join(locks);
    assert_eq!(
        find_locks(&git_dir),
        Vec::<PathBuf>::new(),
        "no lock left behind"
    );
}

fn find_locks(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(find_locks(&path));
        } else if path.extension().is_some_and(|ext| ext == "lock") {
            found.push(path);
        }
    }
    found
}

#[test]
fn a_local_operation_with_hooks_runs_as_before() {
    // Hooks whose askpass would panic and whose helper does not exist: a
    // local operation must never reach either.
    struct Panics;
    impl AskPass for Panics {
        fn ask(&self, _prompt: &str) -> Option<Secret> {
            panic!("a local operation asked for a credential");
        }
    }
    let remote = local();
    git(
        &remote,
        remote.work(),
        &["config", "user.name", "Graft Test"],
    );
    git(
        &remote,
        remote.work(),
        &["config", "user.email", "test@graft.invalid"],
    );
    std::fs::write(remote.work().join("a.txt"), b"a\n").unwrap();
    git(&remote, remote.work(), &["add", "a.txt"]);
    let hooks = RemoteHooks {
        progress: Box::new(|_| panic!("a local operation reported progress")),
        askpass: Arc::new(Panics),
        helper: PathBuf::from("/nonexistent/graft"),
    };
    let commit = Operation::Commit(CommitParams {
        message: "a".to_owned(),
        sign_off: false,
        no_verify: true,
        allow_empty: false,
    });

    let applied = run_hooked(remote.work(), &commit, NOW, hooks, &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(applied.entry.commands, ["git commit -F - --no-verify"]);
    assert_eq!(
        git(&remote, remote.work(), &["log", "-1", "--format=%s"]),
        "a"
    );
}
