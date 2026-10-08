//! Rule 3 on the resume and restore paths (m7-power-tools/01a): an error is
//! reported as what it is, a stop is honoured where it lands, and a restore
//! that fails part-way does not leave half of itself behind.
//!
//! Every test reads the repository back with `git` rather than trusting a
//! returned value, as the rest of this binary does.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::atomic::{AtomicUsize, Ordering};

use graft_core::oid::Oid;
use graft_git::run::Stop;
use graft_ops::apply::{ApplyError, Resume, resume};
use graft_ops::conflict::{ConflictSetError, operation_conflicts};
use graft_ops::facts::{FactsError, read_facts};
use graft_ops::operation::{FactsScope, Operation, ResetMode, ResetParams, ResetTarget, Tip};
use graft_ops::restore::{self, RestoreError};
use graft_ops::undo::{self, InversePlan, Step};

use crate::conflict_fixtures::two_file_conflict;
use crate::support::{NOW, Never, TempRepo, context, git_version, journal_for, run};

// ── helpers ─────────────────────────────────────────────────────────────

fn oid(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("git prints an object id")
}

/// A `Stop` that is true from its n-th poll onwards.
struct StopAtPoll {
    at: usize,
    seen: AtomicUsize,
}

impl Stop for StopAtPoll {
    fn should_stop(&self) -> bool {
        self.seen.fetch_add(1, Ordering::Relaxed) + 1 >= self.at
    }
}

/// Three commits, then a `reset --hard` of `main` back over the last two:
/// the entry whose undo plan these tests bend.
fn after_a_reset() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.write("a.txt", b"first a\n");
    repo.commit_all("two");
    repo.write("b.txt", b"first b\n");
    repo.commit_all("three");
    let op = Operation::Reset(ResetParams {
        target: ResetTarget::Head {
            branch: Tip {
                name: repo.git(&["symbolic-ref", "HEAD"]),
                oid: oid(&repo, "HEAD"),
            },
            mode: ResetMode::Hard,
        },
        to: oid(&repo, "HEAD~2"),
    });
    run(&repo, &op, NOW);
    repo
}

/// The reset's undo plan, its steps replaced by `steps`.
fn bent_plan(repo: &TempRepo, steps: Vec<Step>) -> InversePlan {
    let handle = repo.open();
    let journal = journal_for(&handle);
    let mut plan = undo::plan_undo(journal.entries()).expect("the reset is undoable");
    plan.steps = steps;
    plan
}

fn execute(repo: &TempRepo, plan: &InversePlan) -> Result<(), RestoreError> {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    restore::execute(&mut ctx, plan).map(|_| ())
}

fn entry_count(repo: &TempRepo) -> usize {
    let handle = repo.open();
    journal_for(&handle).entries().len()
}

// ── :640 live_entry honours the stop ────────────────────────────────────

/// A merge begun in the shell, resumed with `Skip`: no entry of Graft's
/// matches it, so without a stop the answer is `NotInProgress` and nothing
/// polls again. `resume` polls once, `live_entry` once on entry; the third
/// poll is the one `live_entry` makes while it reads git's state (the
/// incoming commit's `rev-list`, and the check after it). A stop raised
/// there is `Stopped`, not an answer computed after the caller gave up.
#[test]
fn live_entry_stops_when_asked() {
    let repo = two_file_conflict();
    assert!(
        repo.try_git(&["merge", "feature"]).is_err(),
        "the merge stops"
    );
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let stop = StopAtPoll {
        at: 3,
        seen: AtomicUsize::new(0),
    };
    let mut ctx = context(&handle, &mut journal, NOW, &stop);
    match resume(&mut ctx, Resume::Skip) {
        Err(ApplyError::Stopped) => {}
        Err(other) => panic!(
            "expected Stopped, got {}",
            other.explain(&crate::support::Letters)
        ),
        Ok(_) => panic!("expected Stopped, the skip ran"),
    }
}

// ── :613 a facts failure is a facts failure ─────────────────────────────

/// The live entry cannot be settled because `main` does not read: that is
/// the repository's failure, and it must reach the caller as one — never as
/// "stopped", which the app treats as the user's own cancel.
#[test]
fn a_facts_failure_is_not_reported_as_stopped() {
    let repo = two_file_conflict();
    assert!(
        repo.try_git(&["merge", "feature"]).is_err(),
        "the merge stops"
    );
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    // `main` as a loose ref holding no object id: `live_entry` reads the
    // branch tip of a merge, and that read fails.
    repo.git(&["pack-refs", "--all", "--prune"]);
    repo.write(".git/refs/heads/main", b"not an object id\n");
    match operation_conflicts(&handle, &facts) {
        Err(ConflictSetError::Facts(FactsError::Stopped)) => {
            panic!("a failed read reported as a stop")
        }
        Err(ConflictSetError::Facts(_)) => {}
        Err(other) => panic!("expected a facts failure, got {other}"),
        Ok(set) => panic!("expected a facts failure, got {set:?}"),
    }
}

// ── :496 a restore that fails part-way rolls back ───────────────────────

/// The first write lands (`main` moves to the reset's pre-image), the second
/// is a compare-and-swap that refuses. What the first wrote is put back, so
/// the repository stands where the restore found it and the entry is not in
/// force.
#[test]
fn a_restore_that_fails_mid_way_rolls_back() {
    let repo = after_a_reset();
    let found = oid(&repo, "refs/heads/main");
    let three = oid(&repo, "HEAD@{1}");
    let two = oid(&repo, "HEAD@{1}~1");
    assert_ne!(found, three);
    let steps = vec![
        Step::UpdateRef {
            name: "refs/heads/main".to_owned(),
            expect: Some(found),
            to: Some(three),
        },
        // The fault: expects `main` somewhere it is not.
        Step::UpdateRef {
            name: "refs/heads/main".to_owned(),
            expect: Some(two),
            to: Some(found),
        },
    ];
    let plan = bent_plan(&repo, steps);
    let why = execute(&repo, &plan).expect_err("the second step refuses");
    assert_eq!(
        oid(&repo, "refs/heads/main"),
        found,
        "main is back where the restore found it ({why})"
    );
    assert!(
        matches!(why, RestoreError::RefMoved { ref name } if name == "refs/heads/main"),
        "the step's own failure is the headline: {why}"
    );
}

/// When the roll-back itself cannot finish, the words name what is left.
#[test]
fn a_failed_roll_back_names_what_is_left() {
    let repo = after_a_reset();
    let found = oid(&repo, "refs/heads/main");
    let three = oid(&repo, "HEAD@{1}");
    repo.git(&["branch", "side", "HEAD"]);
    let side = oid(&repo, "refs/heads/side");
    let steps = vec![
        // Lands, and cannot be put back: `NoBefore`, since the plan's
        // touches do not name `side`, so the entry recorded nothing for it.
        Step::UpdateRef {
            name: "refs/heads/side".to_owned(),
            expect: Some(side),
            to: Some(three),
        },
        Step::UpdateRef {
            name: "refs/heads/main".to_owned(),
            expect: Some(three),
            to: Some(found),
        },
    ];
    let plan = bent_plan(&repo, steps);
    let why = execute(&repo, &plan).expect_err("the second step refuses");
    let RestoreError::FailedPartway { applied, .. } = &why else {
        panic!("expected FailedPartway, got {why}");
    };
    assert_eq!(applied, &vec!["refs/heads/side".to_owned()]);
    assert!(
        why.explain(&crate::support::Letters)
            .contains("refs/heads/side"),
        "{}",
        why.explain(&crate::support::Letters)
    );
}

// ── :497 a bad ref name is refused before any write ─────────────────────

#[test]
fn a_bad_ref_name_is_refused_before_any_write() {
    let repo = after_a_reset();
    let found = oid(&repo, "refs/heads/main");
    let three = oid(&repo, "HEAD@{1}");
    let entries = entry_count(&repo);
    for bad in ["refs/heads/bad..name", "-d", "refs/heads/a b"] {
        let steps = vec![
            Step::UpdateRef {
                name: "refs/heads/main".to_owned(),
                expect: Some(found),
                to: Some(three),
            },
            Step::UpdateRef {
                name: bad.to_owned(),
                expect: None,
                to: Some(three),
            },
        ];
        let plan = bent_plan(&repo, steps);
        let why = execute(&repo, &plan).expect_err("the name is refused");
        assert!(
            matches!(why, RestoreError::BadRefName { .. }),
            "{bad}: expected BadRefName, got {why}"
        );
        assert_eq!(
            oid(&repo, "refs/heads/main"),
            found,
            "{bad}: main untouched"
        );
        assert_eq!(entry_count(&repo), entries, "{bad}: no entry written");
    }
}

// ── :601 the errors seal_after and chain_states used to drop ────────────

/// `chain_states` (under `stage_source`, for a path `⏎` already staged in a
/// live merge) passes each kind of `live_entry` failure through as itself:
/// the stop as `Stopped`, a failed read as that read's error.
#[test]
fn chain_states_maps_each_error_kind() {
    use graft_ops::apply::stage_source;

    let repo = two_file_conflict();
    assert!(
        repo.try_git(&["merge", "feature"]).is_err(),
        "the merge stops"
    );
    repo.write("a.txt", b"1\nboth\n3\n");
    repo.git(&["add", "a.txt"]);
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    let path = std::path::Path::new("a.txt");

    // The stop: `live_entry`'s first poll.
    {
        let mut journal = journal_for(&handle);
        let stop = StopAtPoll {
            at: 1,
            seen: AtomicUsize::new(0),
        };
        let ctx = context(&handle, &mut journal, NOW, &stop);
        match stage_source(&ctx, &facts, path) {
            Err(FactsError::Stopped) => {}
            other => panic!("expected Stopped, got {other:?}"),
        }
    }

    // A failed read: `main` no longer resolves.
    repo.git(&["pack-refs", "--all", "--prune"]);
    repo.write(".git/refs/heads/main", b"not an object id\n");
    let mut journal = journal_for(&handle);
    let never = Never;
    let ctx = context(&handle, &mut journal, NOW, &never);
    match stage_source(&ctx, &facts, path) {
        Err(FactsError::Stopped) => panic!("a failed read reported as a stop"),
        Err(_) => {}
        Ok(found) => panic!("expected a facts failure, got {found:?}"),
    }
}

/// Breaks every loose ref under `refs/graft/` the moment the commit lands:
/// after `begin` made the backup ref, before `finish` seals onto it.
const BREAK_BACKUP_REFS: &str = "#!/bin/sh
for f in $(find \"$(git rev-parse --git-dir)/refs/graft\" -type f); do
  echo 'not an object id' > \"$f\"
done
";

/// A `post-commit` hook breaks the commit's backup ref between `begin` and
/// `finish`, so `seal_after` cannot read the tip it chains onto. That read's
/// own failure reaches the caller — not a silent fall-back to the
/// pre-operation commit, sealed on a guessed parent.
#[cfg(unix)]
#[test]
fn a_failed_read_refs_in_seal_after_is_reported() {
    use std::os::unix::fs::PermissionsExt;

    use graft_ops::apply::AfterState;
    use graft_ops::operation::CommitParams;

    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.write("base.txt", b"changed\n");
    repo.git(&["add", "base.txt"]);
    let hooks = repo.path().join("breaking-hooks");
    std::fs::create_dir_all(&hooks).expect("the hooks dir is made");
    let hook = hooks.join("post-commit");
    std::fs::write(&hook, BREAK_BACKUP_REFS).expect("the hook is written");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
        .expect("the hook is executable");
    repo.git(&["config", "core.hooksPath", hooks.to_str().expect("utf-8")]);

    let op = Operation::Commit(CommitParams {
        message: "a change\n".to_owned(),
        sign_off: false,
        no_verify: false,
        allow_empty: false,
    });
    match crate::support::try_run(&repo, &op, NOW) {
        Err(ApplyError::AfterState { unread, .. }) => {
            let words: Vec<String> = unread.iter().map(ToString::to_string).collect();
            assert!(
                unread
                    .iter()
                    .any(|why| matches!(why, AfterState::BackupUnread(_))),
                "the backup ref's read failure is reported: {words:?}"
            );
            // m8-polish-and-ship/20 item 4: said as the backup, not "the refs".
            assert!(
                words
                    .iter()
                    .any(|w| w.starts_with("the backup couldn't be read")),
                "{words:?}"
            );
            assert!(
                !unread
                    .iter()
                    .any(|why| matches!(why, AfterState::Unsealed(_))),
                "nothing was sealed on a guessed parent: {words:?}"
            );
        }
        Err(other) => panic!(
            "expected AfterState, got {}",
            other.explain(&crate::support::Letters)
        ),
        Ok(_) => panic!("expected AfterState: the backup ref was broken"),
    }
}
