//! m8-polish-and-ship/58: a journal read that runs while an operation is
//! still under way must not close that operation's entry as interrupted.
//!
//! The app reads the journal on `J` in a second worker, and recovery
//! (`recover_interrupted`) closes every `Pending` entry git reports no live
//! operation for. Mid-operation — git done, the outcome line not yet
//! written — that entry is not a dead run's: its writer is alive and about
//! to append `committed`. Two outcomes for one id, and every later read of
//! the journal refuses the whole file.
//!
//! Deterministic, no sleeps: the "second worker" runs from inside the
//! operation's own `Stop` poll, at the first poll after git has moved the
//! branch, which is exactly the window the reporter hit.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::sync::Mutex;

use graft_core::journal::EntryStatus;
use graft_core::oid::Oid;
use graft_git::run::Stop;
use graft_ops::apply::{Recovered, recover_interrupted};
use graft_ops::operation::{Operation, RebaseParams, Source, Tip};

use crate::support::{NOW, Never, TempRepo, context, journal_for, run_with_stop};

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a full id")
}

/// `feature` one commit off `base`, `main` one commit on; `feature` checked
/// out, so the rebase has one commit to replay.
fn diverged() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"base\n");
    repo.commit_all("base");
    repo.git(&["checkout", "-q", "-b", "feature"]);
    repo.write("b.txt", b"feature\n");
    repo.commit_all("feature work");
    repo.git(&["checkout", "-q", "main"]);
    repo.write("c.txt", b"main\n");
    repo.commit_all("main work");
    repo.git(&["checkout", "-q", "feature"]);
    repo
}

/// The `J` read, run once from inside the operation's stop poll: the first
/// poll that finds the entry `Pending`, git with nothing in progress, and
/// `feature` already moved off its old tip.
struct ReaderMidway<'a> {
    repo: &'a TempRepo,
    old_tip: Oid,
    /// What the read's recovery did, once it has run.
    ran: Mutex<Option<Recovered>>,
}

impl Stop for ReaderMidway<'_> {
    fn should_stop(&self) -> bool {
        let mut ran = self.ran.lock().unwrap();
        if ran.is_some() {
            return false;
        }
        let handle = self.repo.open();
        let moved = id(self.repo, "refs/heads/feature") != self.old_tip;
        let git_done = graft_git::state::in_progress(&handle)
            .expect("git's state reads")
            .is_none();
        let mut journal = journal_for(&handle);
        if !moved || !git_done || journal.unresolved().is_empty() {
            return false;
        }
        let never = Never;
        let mut ctx = context(&handle, &mut journal, NOW + 1_000, &never);
        *ran = Some(recover_interrupted(&mut ctx).expect("the read's recovery runs"));
        false
    }
}

#[test]
fn a_journal_read_mid_rebase_leaves_the_running_entry_one_outcome() {
    let repo = diverged();
    let old_tip = id(&repo, "feature");
    let op = Operation::Rebase(RebaseParams::new(
        Tip {
            name: "refs/heads/feature".to_owned(),
            oid: old_tip,
        },
        Source::Branch("main".to_owned()),
        id(&repo, "main"),
    ));
    let reader = ReaderMidway {
        repo: &repo,
        old_tip,
        ran: Mutex::new(None),
    };

    let applied = run_with_stop(&repo, &op, NOW, &reader);

    let recovered = reader.ran.lock().unwrap().take();
    let recovered = recovered.expect("the read ran inside the window: git done, outcome unwritten");
    applied.expect("the rebase lands");
    let handle = repo.open();
    let journal = graft_ops::journal::Journal::open_as(
        &graft_ops::journal::journal_path(&handle).expect("the journal path resolves"),
        handle.on_disk_names(),
    )
    .expect("the journal still reads after a read raced the rebase");
    let statuses: Vec<&EntryStatus> = journal.entries().iter().map(|e| &e.status).collect();
    assert_eq!(
        statuses,
        [&EntryStatus::Committed],
        "one entry, one outcome"
    );
    assert!(
        recovered.closed.is_empty(),
        "the read closed an entry whose writer is still running: {:?}",
        recovered.closed
    );
}
