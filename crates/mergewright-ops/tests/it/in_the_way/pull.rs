//! Pull: the tracking tip as last fetched, and what the fetch brings.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::Path;

use mergewright_core::journal::EntryStatus;
use mergewright_fixtures::Shape;
use mergewright_fixtures::remote::LocalRemote;
use mergewright_git::remote::{RemoteFailure, upstream_of};
use mergewright_git::repo::RepoHandle;
use mergewright_ops::apply::{Applied, ApplyError, apply};
use mergewright_ops::operation::{Operation, PullParams, Refusal};
use mergewright_ops::preview::PreviewError;

use super::{MINE, applied_words, preview_in, preview_words};
use crate::support::{NOW, Never, context, journal_for};

// ── pull ────────────────────────────────────────────────────────────────

fn remote_git(remote: &LocalRemote, args: &[&str]) -> String {
    remote
        .git(remote.work(), args)
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"))
}

fn pull(remote: &LocalRemote) -> Operation {
    let branch = format!("refs/heads/{}", remote.default_branch());
    let repo = RepoHandle::open(remote.work()).expect("the work clone opens");
    let upstream = upstream_of(&repo, &branch)
        .expect("the upstream reads")
        .expect("the work clone tracks origin");
    Operation::Pull(PullParams { branch, upstream })
}

fn run_in(dir: &Path, op: &Operation) -> Result<Applied, ApplyError> {
    let repo = RepoHandle::open(dir).expect("the repository opens");
    let mut journal = journal_for(&repo);
    let mut ctx = context(&repo, &mut journal, NOW, &Never);
    apply(&mut ctx, op)
}

/// The file the remote's new commit adds, ignored in the work clone and
/// holding his bytes.
fn ignore_what_it_adds(remote: &LocalRemote, tip: &str) -> String {
    // Read from the bare remote: the work clone may not have fetched it.
    let added = remote
        .git(
            remote.bare(),
            &["diff-tree", "--no-commit-id", "--name-only", "-r", tip],
        )
        .unwrap_or_else(|why| panic!("diff-tree: {why}"));
    std::fs::write(
        remote.work().join(".git/info/exclude"),
        format!("{added}\n"),
    )
    .unwrap();
    std::fs::write(remote.work().join(&added), MINE).unwrap();
    assert_eq!(remote_git(remote, &["status", "--porcelain"]), "");
    added
}

/// The tracking tip already tracks the file: refused before the fetch.
#[test]
fn pull_over_an_ignored_file_is_refused() {
    let remote = LocalRemote::new(Shape::Linear).expect("a local remote");
    let tip = remote.commit_on_remote("adds a file").unwrap();
    remote_git(&remote, &["fetch", "-q", "origin"]);
    let added = ignore_what_it_adds(&remote, &tip);
    // A second commit the fetch would bring: the tracking ref still at
    // `tip` afterwards proves the fetch never ran.
    remote.commit_on_remote("after").unwrap();
    let tracking = format!("refs/remotes/origin/{}", remote.default_branch());
    let op = pull(&remote);
    let words = format!(
        "{added} (ignored) is in the way of pulling origin/{} · move it aside first",
        remote.default_branch()
    );
    let handle = RepoHandle::open(remote.work()).unwrap();
    let before = journal_for(&handle).entries().len();

    let preview = preview_in(&handle, &op);
    let applied = run_in(remote.work(), &op);

    assert_eq!(
        (
            preview_words(&preview),
            applied_words(&applied),
            std::fs::read(remote.work().join(&added)).unwrap(),
            journal_for(&handle).entries().len() - before,
            remote_git(&remote, &["rev-parse", &tracking]),
        ),
        (words.clone(), words, MINE.to_vec(), 0, tip),
        "(preview, apply, his bytes, new entries, the tracking ref)"
    );
}

/// The tracking tip lacks the file and the remote has it: Mergewright's read
/// can't see it, the fetch brings it, and git's `--no-overwrite-ignore`
/// refuses the merge.
#[test]
fn pull_whose_fetch_brings_an_ignored_path_is_refused_by_git() {
    let remote = LocalRemote::new(Shape::Linear).expect("a local remote");
    let before_head = remote_git(&remote, &["rev-parse", "HEAD"]);
    let tip = remote.commit_on_remote("adds a file").unwrap();
    let added = ignore_what_it_adds(&remote, &tip);

    let applied = run_in(remote.work(), &pull(&remote));

    assert_eq!(
        std::fs::read(remote.work().join(&added)).unwrap(),
        MINE,
        "his ignored file was overwritten"
    );
    assert_eq!(remote_git(&remote, &["rev-parse", "HEAD"]), before_head);
    match applied {
        Err(ApplyError::Remote { failure, entry, .. }) => {
            assert_eq!(
                failure,
                RemoteFailure::WouldOverwrite { paths: vec![added] }
            );
            assert!(matches!(entry.status, EntryStatus::Aborted { .. }));
        }
        other => panic!("not git's refusal: {}", applied_words(&other)),
    }
}

/// Review 28 M1: he is ahead by one commit that stops tracking a file the
/// tracking tip still tracks, now ignored and holding his bytes. A pull
/// writes only the incoming side, which is empty here: plain git says
/// `Already up to date.`, and so must Mergewright.
#[test]
fn pull_while_ahead_by_an_untracking_commit_is_not_refused() {
    let remote = LocalRemote::new(Shape::Linear).expect("a local remote");
    let tip = remote.commit_on_remote("adds a file").unwrap();
    remote_git(&remote, &["fetch", "-q", "origin"]);
    remote_git(&remote, &["merge", "-q", "--ff-only", &tip]);
    let added = remote
        .git(
            remote.bare(),
            &["diff-tree", "--no-commit-id", "--name-only", "-r", &tip],
        )
        .unwrap();
    remote_git(&remote, &["rm", "-q", "--cached", &added]);
    remote_git(&remote, &["commit", "-q", "-m", "stop tracking it"]);
    let ahead = remote_git(&remote, &["rev-parse", "HEAD"]);
    ignore_what_it_adds(&remote, &tip);
    let op = pull(&remote);
    let handle = RepoHandle::open(remote.work()).unwrap();

    let preview = preview_in(&handle, &op);
    let applied = run_in(remote.work(), &op);

    assert!(
        !matches!(
            preview,
            Err(PreviewError::Refused(Refusal::InTheWay { .. }))
        ),
        "the preview refused: {}",
        preview_words(&preview)
    );
    assert!(applied.is_ok(), "{}", applied_words(&applied));
    assert_eq!(
        (
            std::fs::read(remote.work().join(&added)).unwrap(),
            remote_git(&remote, &["rev-parse", "HEAD"]),
        ),
        (MINE.to_vec(), ahead),
        "(his bytes, HEAD)"
    );
    // Plain git, same repository, same moment: nothing to do.
    let tracking = format!("origin/{}", remote.default_branch());
    let plain = remote_git(
        &remote,
        &["merge", "--ff-only", "--no-overwrite-ignore", &tracking],
    );
    assert!(plain.contains("Already up to date"), "git said: {plain}");
}
