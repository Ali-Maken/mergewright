//! An edit git can only see by its content: a **racily clean** entry
//! (m8-polish-and-ship/30).
//!
//! git trusts an index entry whose stat matches the file — unless the entry
//! is as new as the index file itself, when the stat proves nothing and git
//! reads the content. Every snapshot and forecast reads a *copy* of the
//! index; a copy dated "now" made every entry look older than its index, so
//! git trusted a stat that an edit in the same clock tick had left matching,
//! and the edit was missed. On Linux, whose file times tick coarsely, the
//! put-back forecast missed one 8 times in 200 runs
//! (`put_back_forecast::a_rebase_with_drops_measures_the_put_back_from_where_it_lands`).
//!
//! Here the race is built, not waited for: the entry, the file and the index
//! all carry one instant, and `core.trustctime` is off so the ctime the edit
//! moved does not give it away.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use graft_core::journal::HeadRef;
use graft_ops::facts::read_facts;
use graft_ops::operation::{CheckoutParams, CheckoutTarget, OnEdits, Operation};
use graft_ops::preview::{self, ConflictForecast};
use graft_ops::snapshot;

use crate::support::{Never, TempRepo, git_version};

/// 2001-09-09: any fixed instant in the past.
fn instant() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000)
}

fn date(repo: &TempRepo, relative: &str, when: SystemTime) {
    std::fs::File::options()
        .write(true)
        .open(repo.path().join(relative))
        .expect("the file opens")
        .set_modified(when)
        .expect("the time is set");
}

/// `relative` holds `old` in the index and `new` on disk — the same length —
/// with the entry, the file and the index all dated [`instant`]. git itself
/// sees the edit (asserted): only a reader that keeps the index's date can.
fn racily_clean(repo: &TempRepo, relative: &str, old: &[u8], new: &[u8]) {
    assert_eq!(old.len(), new.len(), "a size change would give it away");
    repo.git(&["config", "core.trustctime", "false"]);
    repo.write(relative, old);
    date(repo, relative, instant());
    // Re-added at the old date, so the entry records `instant` as its mtime.
    repo.git(&["add", relative]);
    repo.write(relative, new);
    date(repo, relative, instant());
    date(repo, ".git/index", instant());
    assert_eq!(
        repo.git(&["--no-optional-locks", "status", "--porcelain"]),
        format!(" M {relative}"),
        "git sees the edit in the real index"
    );
}

/// The undo's before-snapshot (`take`) builds its working-tree side this
/// way: a missed edit is an edit `u` cannot bring back.
#[test]
fn the_worktree_snapshot_keeps_a_racily_clean_edit() {
    let repo = TempRepo::init();
    repo.write("x.txt", b"feature\n");
    repo.commit_all("one");
    racily_clean(&repo, "x.txt", b"feature\n", b"my edit\n");

    let handle = repo.open();
    let scratch = repo.path().join(".git/graft");
    std::fs::create_dir_all(&scratch).expect("the scratch is made");
    let tree = snapshot::worktree_tree(&handle, &scratch, &Never).expect("the worktree tree");

    assert_eq!(
        repo.git(&["cat-file", "-p", &format!("{tree}:x.txt")]),
        "my edit"
    );
}

/// The flaky test's own path: `git stash create` on a copy of the index,
/// for the put-back forecast of a switch whose edit clashes.
#[test]
fn the_put_back_forecast_sees_a_racily_clean_edit() {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("base.txt", b"feature's line\n");
    repo.commit_all("feature changes base");
    repo.git(&["switch", "-q", "main"]);
    racily_clean(&repo, "base.txt", b"base\n", b"mine\n");

    let handle = repo.open();
    let op = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch("refs/heads/feature".to_owned()),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        on_edits: OnEdits::SetAside,
    });
    let facts = read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("facts");
    let plan = preview::plan(&handle, &git_version(), &op, &facts, &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(
        plan.put_back,
        Some(ConflictForecast::Likely {
            paths: vec![PathBuf::from("base.txt")]
        })
    );
}
