//! m7-power-tools/22b: graft-fixtures' "lost work" shape, with Graft's own
//! operations at its two Graft points, read back as the journal's list.
//!
//! The fixture builds the git side (graft-fixtures depends on no Graft
//! crate); this supplies the `●` rows — a fetch, and a discard undone over
//! his edit — and asserts every row the list draws, as journal-screen and
//! the acceptance walk will see it.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::{Path, PathBuf};

use graft_core::journal::{JournalEntry, OperationKind};
use graft_core::oid::Oid;
use graft_git::history::read_commit;
use graft_git::repo::RepoHandle;
use graft_ops::apply::apply;
use graft_ops::journal_list::{
    JournalList, LOST_CAP, ListRow, LostEdit, LostScan, RECORDS_CAP, build, find_lost, lost_edits,
    read_outside,
};
use graft_ops::operation::{FetchParams, Operation, Target};
use graft_ops::undo;

use graft_fixtures::lost_work::{self, Graft, LostWork};

use crate::journal_list_oracle::{branch_exists, lines};
use crate::support::{Never, context, journal_for};

/// Graft's own operations, run the way the app runs them.
struct GraftOps;

fn apply_in(work: &Path, op: &Operation, at_ms: i64) -> Result<(), String> {
    let repo = RepoHandle::open(work).map_err(|why| why.to_string())?;
    let mut journal = journal_for(&repo);
    let never = Never;
    let mut ctx = context(&repo, &mut journal, at_ms, &never);
    apply(&mut ctx, op)
        .map(|_| ())
        .map_err(|why| why.explain(&crate::support::Letters))
}

impl Graft for GraftOps {
    fn fetch(&mut self, work: &Path, at_ms: i64) -> Result<(), String> {
        let op = Operation::Fetch(FetchParams {
            remote: Some("origin".to_owned()),
            prune: false,
        });
        apply_in(work, &op, at_ms)
    }

    fn discard(&mut self, work: &Path, path: &str, at_ms: i64) -> Result<(), String> {
        let op = Operation::Discard(Target::Paths(vec![PathBuf::from(path)]));
        apply_in(work, &op, at_ms)
    }

    fn undo_forced(&mut self, work: &Path, at_ms: i64) -> Result<(), String> {
        let repo = RepoHandle::open(work).map_err(|why| why.to_string())?;
        let mut journal = journal_for(&repo);
        let never = Never;
        let mut ctx = context(&repo, &mut journal, at_ms, &never);
        undo::undo(&mut ctx, true)
            .map(|_| ())
            .map_err(|why| why.explain(&crate::support::Letters))
    }
}

fn git(work: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(work)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_owned()
}

/// The three stages and the lost edits, as the app runs them.
fn the_list(work: &Path) -> (JournalList, Vec<JournalEntry>, Vec<LostEdit>) {
    let handle = RepoHandle::open(work).expect("the fixture opens");
    let entries = journal_for(&handle).entries().to_vec();
    let outside = read_outside(&handle, RECORDS_CAP, &Never).expect("the reflogs read");
    let found =
        find_lost(&handle, &entries, &outside, LOST_CAP, &Never).expect("the lost scan runs");
    let edits = lost_edits(&handle, &entries, &Never).expect("the lost edits read");
    let subject_of = |tip: Oid| {
        let commit = read_commit(&handle, tip).expect("a lost tip reads");
        Some(String::from_utf8_lossy(commit.subject()).into_owned())
    };
    let exists = |name: &str| branch_exists(&handle, name);
    let list = build(
        &entries,
        &outside,
        Some(&found),
        &subject_of,
        &exists,
        &edits,
        LostScan::Done,
    );
    (list, entries, edits)
}

fn oid(hex: &str) -> Oid {
    Oid::from_hex(hex).expect("a full id")
}

#[test]
fn the_lost_work_fixture_draws_every_row_in_place() {
    let fixture = LostWork::build(&mut GraftOps).expect("the fixture builds");
    let work = fixture.work();

    let (list, entries, _edits) = the_list(work);

    let kinds: Vec<OperationKind> = entries.iter().map(|entry| entry.kind).collect();
    assert_eq!(
        kinds,
        [
            OperationKind::Discard,
            OperationKind::Undo,
            OperationKind::Fetch
        ],
        "the three Graft steps, oldest first"
    );
    assert_eq!(
        lines(&list, &entries),
        [
            "● Fetch",
            "$ reset main 1 commit back",
            "✗ Add rate limiter · main · under 1",
            "$ switch from spike to main",
            "✗ Try redis cache · spike gone · under 3",
            "$ committed \"Try redis cache\" on spike",
            "$ switch from main to spike",
            "● Undo",
            "✗ edit under 7",
            "● Discard",
            "$ committed \"Add rate limiter\" on main",
            "$ committed \"Add metrics endpoint\" on main",
            "$ committed \"Add request logging\" on main",
            // The first commit reads as any commit (`the_first_commit_reads_committed`).
            "$ committed \"Start the api\" on main",
        ]
    );
}

#[test]
fn the_lost_rows_are_the_fixtures_commits_and_his_edit() {
    let fixture = LostWork::build(&mut GraftOps).expect("the fixture builds");
    let work = fixture.work();
    let commits = fixture.commits();

    let (list, _entries, edits) = the_list(work);

    let tips: Vec<Oid> = list
        .rows
        .iter()
        .filter_map(|row| match row {
            ListRow::Lost { commit, .. } => Some(commit.run.tip),
            _ => None,
        })
        .collect();
    assert_eq!(tips, [oid(&commits.d), oid(&commits.e)]);

    // The stash holds S; S is not lost.
    assert_eq!(git(work, &["rev-parse", "refs/stash"]), commits.s);
    assert!(!tips.contains(&oid(&commits.s)));

    // His edit lives only in the undo's backup: the file on disk is the one
    // the undo put back.
    assert_eq!(edits.len(), 1, "{edits:#?}");
    assert_eq!(edits[0].paths, [PathBuf::from(lost_work::EDITED)]);
    let kept = git(
        work,
        &[
            "cat-file",
            "blob",
            &format!("{}:{}", edits[0].backup_tree, lost_work::EDITED),
        ],
    );
    assert_eq!(format!("{kept}\n").as_bytes(), lost_work::LOST_EDIT);
    let on_disk = std::fs::read(work.join(lost_work::EDITED)).expect("src/a.rs reads");
    assert_eq!(on_disk, lost_work::DISCARDED_EDIT);

    // The fetch brought origin/main two commits past main.
    assert_eq!(git(work, &["rev-parse", "origin/main"]), commits.fetched);
    assert_eq!(
        git(work, &["rev-list", "--count", "main..origin/main"]),
        "2"
    );
}

/// Every row's clock is the fixture's 10:12–10:44, run to run: the shell's
/// records and Graft's entries alike.
#[test]
fn every_row_is_timed_between_10_12_and_10_44() {
    let fixture = LostWork::build(&mut GraftOps).expect("the fixture builds");
    let (list, entries, _edits) = the_list(fixture.work());
    let window = lost_work::START..=lost_work::END;

    for row in &list.rows {
        let at = match row {
            ListRow::Graft(id) => {
                let entry = entries
                    .iter()
                    .find(|entry| entry.id == *id)
                    .expect("its entry");
                entry.at_unix_ms / 1000
            }
            ListRow::Outside(step) => step.record.at,
            ListRow::Lost { .. } | ListRow::LostEdit { .. } => continue,
        };
        assert!(window.contains(&at), "{row:?} at {at}");
    }
}
