//! An **octopus** merge begun in the shell (m6-conflicts/38):
//! `git merge --no-commit --no-ff a b` leaves a `MERGE_HEAD` of two lines.
//! Every journaled operation reads git's merge state as it begins, so
//! reading it must never refuse an unrelated one — and what it records must
//! be the two lines exactly, so `u` puts them back as they were.
//!
//! Which operations *begin* during a merge begun in the shell: not the
//! forward ones `check` admits (stage, unstage, fetch) — with no entry of
//! Graft's behind the merge they fold as steps inside it and write no line
//! (`apply::fold::folded_step`) — and not `c`/`a`, which refuse an octopus
//! merge by name before they begin. What begins is a restore
//! (`restore::execute`), journaling an entry of its own. So the unrelated
//! operation here is the restore that takes back an entry made before the
//! merge, run at the recording layer.
//!
//! Since m6-conflicts/57 the undo **executor** refuses that same `u`
//! (`NotInvertible::OperationLive`: the entry is older than the merge), so
//! the key never reaches the restore; these tests drive the restore
//! directly, which is where `begin` reads git's merge state.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::{Path, PathBuf};

use graft_core::journal::{JournalEntry, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_git::state::InProgress;
use graft_ops::apply::{ApplyError, Resume};
use graft_ops::operation::{BranchCreateParams, MergeEnd, Operation, Refusal};
use graft_ops::restore;
use graft_ops::undo::{self, InversePlan, NotInvertible, Step, UndoError};

use crate::conflict_fixtures::run_resume;
use crate::support::{NOW, Never, TempRepo, context, journal_for, run_undo, try_run};

/// `main` with `base.txt`, and a `topic` branch created **through Graft**
/// (the entry `u` will take back); `a` adds `a.txt`, `b` adds `b.txt`;
/// back on `main`, `git merge --no-commit --no-ff a b` in the shell — an
/// octopus merge in progress, nothing conflicted.
fn octopus_in_progress() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("base");
    let at = Oid::from_hex(&repo.git(&["rev-parse", "HEAD"])).expect("an object id");
    let create = Operation::BranchCreate(BranchCreateParams {
        name: "topic".to_owned(),
        at,
        checkout: false,
        on_edits: graft_ops::operation::OnEdits::Refuse,
    });
    try_run(&repo, &create, NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    for branch in ["a", "b"] {
        repo.git(&["switch", "-q", "-c", branch, "main"]);
        repo.write(&format!("{branch}.txt"), format!("{branch}\n").as_bytes());
        repo.commit_all(branch);
    }
    repo.git(&["switch", "-q", "main"]);
    repo.git(&["merge", "--no-commit", "--no-ff", "a", "b"]);
    let head = merge_files(&repo)[0].clone().expect("MERGE_HEAD");
    assert_eq!(
        String::from_utf8_lossy(&head).lines().count(),
        2,
        "MERGE_HEAD names two commits: {}",
        String::from_utf8_lossy(&head)
    );
    repo
}

fn git_path(repo: &TempRepo, name: &str) -> PathBuf {
    let printed = repo.git(&["rev-parse", "--git-path", name]);
    let path = Path::new(&printed);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo.path().join(path)
    }
}

/// `MERGE_HEAD`, `MERGE_MSG`, `MERGE_MODE`, byte for byte (`None` = absent).
fn merge_files(repo: &TempRepo) -> Vec<Option<Vec<u8>>> {
    ["MERGE_HEAD", "MERGE_MSG", "MERGE_MODE"]
        .into_iter()
        .map(|name| match std::fs::read(git_path(repo, name)) {
            Ok(bytes) => Some(bytes),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
            Err(why) => panic!("{name}: {why}"),
        })
        .collect()
}

/// The restore that takes back `topic`'s creation, run straight through
/// `restore::execute` — `begin` reads the octopus merge and records it.
/// The executor's `u` is refused first, and must be the gate's refusal,
/// never a failure to read the merge.
fn undo_restored(repo: &TempRepo) {
    for force in [false, true] {
        let refused = run_undo(repo, NOW + 1, force).expect_err("older than the merge");
        assert!(
            matches!(
                refused,
                UndoError::Nothing(NotInvertible::OperationLive {
                    kind: InProgress::Merge
                })
            ),
            "{}",
            refused.explain(&crate::support::Letters)
        );
    }
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let plan = undo::plan_undo(journal.entries()).expect("topic's creation is undoable");
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    restore::execute(&mut ctx, &plan)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
}

fn last_entry(repo: &TempRepo) -> JournalEntry {
    journal_for(&repo.open())
        .entries()
        .last()
        .cloned()
        .expect("an entry")
}

/// `u` begins and journals its entry while the octopus merge is in
/// progress — red before m6-conflicts/38, when `begin` refused it with
/// `an octopus merge is in progress (2 heads)` — and the entry records both
/// heads, byte-exact.
#[test]
fn an_unrelated_operation_begins_and_records_during_an_octopus_merge() {
    let repo = octopus_in_progress();
    let before = merge_files(&repo);

    undo_restored(&repo);
    assert!(
        repo.try_git(&["rev-parse", "-q", "--verify", "refs/heads/topic"])
            .is_err(),
        "the undo ran: topic is gone"
    );
    let entry = last_entry(&repo);
    assert_eq!(entry.kind, OperationKind::Undo);
    let recorded = entry.merge_before.expect("the octopus merge is recorded");
    assert_eq!(recorded.heads().count(), 2, "{recorded:?}");
    assert_eq!(
        Some(recorded.merge_head_bytes()),
        before[0],
        "MERGE_HEAD recorded byte-exact"
    );
    assert_eq!(entry.merge_after, Some(recorded), "the undo left it alone");
}

/// After `u`, the two-line `MERGE_HEAD` (and the message and mode) are
/// byte-equal to what git wrote.
#[test]
fn undo_during_an_octopus_merge_keeps_merge_head_byte_exact() {
    let repo = octopus_in_progress();
    let before = merge_files(&repo);

    undo_restored(&repo);
    assert_eq!(
        merge_files(&repo),
        before,
        "MERGE_HEAD, MERGE_MSG, MERGE_MODE"
    );
}

/// The write half: the recorded octopus state, put back by
/// `Step::SetMergeState` after the files were removed in the shell
/// (`git merge --quit`), is byte-equal to what git wrote — both lines of
/// `MERGE_HEAD` — and git sees the merge again.
#[test]
fn set_merge_state_writes_an_octopus_merge_head_back_byte_exact() {
    let repo = octopus_in_progress();
    let before = merge_files(&repo);
    undo_restored(&repo);
    let recorded = last_entry(&repo).merge_before;
    assert!(recorded.is_some(), "the octopus merge is recorded");
    repo.git(&["merge", "--quit"]);
    assert!(merge_files(&repo)[0].is_none(), "--quit removed MERGE_HEAD");

    let plan = InversePlan {
        reverses: last_entry(&repo).id,
        kind: OperationKind::Redo,
        description: "put the octopus merge back".to_owned(),
        touches: Touches::default(),
        steps: vec![Step::SetMergeState(recorded)],
        pins: Vec::new(),
    };
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 2, &never);
    restore::execute(&mut ctx, &plan)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(
        merge_files(&repo),
        before,
        "the three merge files, byte-equal"
    );
    assert!(
        repo.try_git(&["rev-parse", "-q", "--verify", "MERGE_HEAD"])
            .is_ok(),
        "git sees the merge"
    );
}

/// `c` on an octopus merge begun in the shell is refused by name — never a
/// `PrepareError`, never "nothing in progress" — and nothing is journaled.
#[test]
fn c_on_an_octopus_merge_names_its_refusal() {
    let repo = octopus_in_progress();
    let before = merge_files(&repo);
    let entries = journal_for(&repo.open()).entries().len();

    let why =
        run_resume(&repo, Resume::Continue { solved: Vec::new() }, NOW + 1).expect_err("refused");
    assert!(
        matches!(
            why,
            ApplyError::Refused(Refusal::OctopusMerge {
                heads: 2,
                end: MergeEnd::Finish
            })
        ),
        "{}",
        why.explain(&crate::support::Letters)
    );
    assert!(
        why.explain(&crate::support::Letters).contains("git commit"),
        "the words say where it is finished: {}",
        why.explain(&crate::support::Letters)
    );
    assert_eq!(
        journal_for(&repo.open()).entries().len(),
        entries,
        "no entry"
    );
    assert_eq!(merge_files(&repo), before, "the merge is untouched");
}

/// `a` on an octopus merge begun in the shell is refused by name too, and
/// the words name `git merge --abort`: `u` of an abort could not be checked
/// to put every head back (m6-conflicts/57). Nothing is journaled, nothing
/// moves.
#[test]
fn abort_of_an_octopus_shell_merge_is_refused() {
    let repo = octopus_in_progress();
    let before = merge_files(&repo);
    let head = repo.git(&["rev-parse", "HEAD"]);
    let entries = journal_for(&repo.open()).entries().len();

    let why = run_resume(&repo, Resume::Abort, NOW + 1).expect_err("refused");
    assert!(
        matches!(
            why,
            ApplyError::Refused(Refusal::OctopusMerge {
                heads: 2,
                end: MergeEnd::Abort
            })
        ),
        "{}",
        why.explain(&crate::support::Letters)
    );
    assert_eq!(
        why.explain(&crate::support::Letters),
        "this merges 2 branches at once (an octopus merge); abort it in the shell: git merge \
         --abort"
    );
    assert_eq!(
        journal_for(&repo.open()).entries().len(),
        entries,
        "no entry"
    );
    assert_eq!(merge_files(&repo), before, "the merge is untouched");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), head, "HEAD unmoved");
}
