//! The cases the promise actually rests on: parity with the commands Graft
//! logs, the stash reflog, a compare-and-swap that refuses, an operation that
//! died before its after-state could be read, an unborn `HEAD`, and a linked
//! worktree.

// The scaffolding below (`parity` and friends) is not a `#[test]` function,
// so `clippy.toml`'s "expect is fine in tests" does not reach it. It is fine
// here for the same reason: a scaffold that cannot set the repository up has
// to stop the test loudly, and has no caller to hand an error to.
#![allow(clippy::expect_used, reason = "a failing test should panic")]

use std::path::PathBuf;
use std::process::{Command, Stdio};

use graft_core::journal::{EntryStatus, HeadRef, OperationKind};
use graft_core::oid::Oid;
use graft_ops::apply::{ApplyError, StoppedAt, apply};
use graft_ops::facts;
use graft_ops::operation::{
    CommitParams, FactsScope, Operation, PatchSide, StashParams, StashScope, Target,
};
use graft_ops::undo::{self, NotInvertible, PathScope, Step, UndoError};

use crate::support::{
    NOW, Never, StopWhenGone, TempRepo, context, expire_and_prune, holds, journal_for, round_trip,
    run, run_undo, run_with_stop, stash_list, state_of, try_run,
};
use graft_core::on_disk::CURRENT_DIR;

fn paths(of: &[&str]) -> Target {
    Target::Paths(of.iter().map(PathBuf::from).collect())
}

fn commit(message: &str) -> Operation {
    Operation::Commit(CommitParams {
        message: message.to_owned(),
        sign_off: false,
        no_verify: false,
        allow_empty: false,
        include: graft_ops::operation::Include::Staged,
    })
}

// ── parity with the commands Graft logs ─────────────────────────────────

/// Apply `op` on `a`, and run the *logged command lines* through a shell on
/// `b`. The two repositories have to end up saying the same thing.
///
/// This is the Command Log's promise under test (PROMPT principle 5): the
/// lines Graft shows are the lines it ran, runnable by hand.
fn parity(build: fn(&TempRepo), make: fn(&TempRepo) -> Operation) {
    let a = TempRepo::init();
    let b = TempRepo::init();
    build(&a);
    build(&b);
    // Built from `a`, because a stash operation names the commit it found
    // there. The *command lines* name `stash@{n}`, which is what `b` replays.
    let op = &make(&a);

    let handle = a.open();
    let never = Never;
    let facts = facts::read_facts(
        &handle,
        crate::support::git_version(),
        &never,
        &FactsScope::default(),
    )
    .expect("the facts read");
    let plan = op.plan(&facts);
    run(&a, op, NOW);

    for invocation in &plan {
        let mut shell = Command::new("sh");
        shell
            .arg("-c")
            .arg(invocation.command_line())
            .current_dir(b.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if invocation.stdin.is_some() {
            shell.stdin(Stdio::piped());
        }
        let mut child = shell.spawn().expect("the shell runs");
        if let Some(bytes) = &invocation.stdin {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .expect("stdin is piped")
                .write_all(bytes)
                .expect("stdin is written");
        }
        let status = child.wait().expect("the shell finishes");
        assert!(status.success(), "`{}` failed", invocation.command_line());
    }

    // Refs are compared by the *tree* they reach, not by object id: two
    // repositories that did the same thing a second apart hold commits with
    // different ids and identical content, and it is the content that has to
    // match.
    for what in [
        vec!["for-each-ref", "--format=%(refname) %(tree)"],
        vec!["ls-files", "-s"],
        vec!["status", "--porcelain=v2", "--untracked-files=all"],
    ] {
        let from_graft = a
            .git(&what)
            .lines()
            .filter(|line| !line.contains(&format!("refs/{CURRENT_DIR}/")))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            from_graft,
            b.git(&what),
            "`git {}` differs after {}",
            what.join(" "),
            op.describe()
        );
    }
}

fn one_commit_and_an_edit(repo: &TempRepo) {
    repo.write("a.rs", b"fn a() {}\n");
    repo.write("b.rs", b"fn b() {}\n");
    repo.commit_all("one");
    repo.write("a.rs", b"fn a() { changed }\n");
    repo.write("new.txt", b"untracked\n");
}

#[test]
fn stage_matches_the_command_it_logs() {
    parity(one_commit_and_an_edit, |_repo| {
        Operation::Stage(paths(&["a.rs"]))
    });
}

#[test]
fn discard_matches_the_command_it_logs() {
    parity(one_commit_and_an_edit, |_repo| {
        Operation::Discard(paths(&["a.rs"]))
    });
}

#[test]
fn discarding_an_untracked_file_matches_the_command_it_logs() {
    parity(one_commit_and_an_edit, |_repo| {
        Operation::Discard(paths(&["new.txt"]))
    });
}

#[test]
fn unstage_matches_the_command_it_logs() {
    parity(
        |repo| {
            one_commit_and_an_edit(repo);
            repo.git(&["add", "a.rs"]);
        },
        |_repo| Operation::Unstage(paths(&["a.rs"])),
    );
}

#[test]
fn commit_matches_the_command_it_logs() {
    parity(
        |repo| {
            one_commit_and_an_edit(repo);
            repo.git(&["add", "a.rs"]);
        },
        |_repo| commit("a change\n"),
    );
}

#[test]
fn stash_push_matches_the_command_it_logs() {
    parity(one_commit_and_an_edit, |_repo| {
        Operation::StashPush(StashParams {
            message: Some("wip".to_owned()),
            scope: StashScope::All {
                include_untracked: false,
            },
        })
    });
}

// The four kinds the A/B harness did not cover (m3/03 review, M2). Each
// builds its operation from the repository it was just handed, because a
// stash operation names the commit it found there; the command lines the
// other repository replays name `stash@{n}`, which is the same in both.

#[test]
fn amend_matches_the_command_it_logs() {
    parity(
        |repo| {
            one_commit_and_an_edit(repo);
            repo.git(&["add", "a.rs"]);
        },
        |_repo| {
            Operation::Amend(CommitParams {
                message: "one, amended\n".to_owned(),
                sign_off: false,
                no_verify: false,
                allow_empty: false,
                include: graft_ops::operation::Include::Staged,
            })
        },
    );
}

#[test]
fn stash_apply_matches_the_command_it_logs() {
    parity(one_stash, |repo| Operation::StashApply {
        index: 0,
        commit: stash_at(repo, 0),
    });
}

#[test]
fn stash_pop_matches_the_command_it_logs() {
    parity(one_stash, |repo| Operation::StashPop {
        index: 0,
        commit: stash_at(repo, 0),
    });
}

#[test]
fn stash_drop_matches_the_command_it_logs() {
    parity(one_stash, |repo| Operation::StashDrop {
        index: 0,
        commit: stash_at(repo, 0),
    });
}

/// One commit and one stash, with nothing left in the working tree: the
/// setup `apply` and `pop` need, since `check` refuses either over a path
/// that has a change of its own.
fn one_stash(repo: &TempRepo) {
    repo.write("a.txt", b"a\n");
    repo.write("b.txt", b"b\n");
    repo.commit_all("one");
    repo.write("a.txt", b"a edited\n");
    repo.git(&["stash", "push", "-q", "-m", "first"]);
}

/// The commit at `stash@{n}`.
fn stash_at(repo: &TempRepo, index: usize) -> Oid {
    let id = repo.git(&["rev-parse", &format!("stash@{{{index}}}")]);
    Oid::from_hex(&id).expect("a stash id")
}

// ── the stash reflog facts the inverse rests on ─────────────────────────
//
// These were verified by hand in a throwaway repository and lived nowhere
// (group 02 review, Major 5). They are the assumptions `Step::StashStore` and
// `Step::StashDrop` are built on, so git changing one has to break a test.

fn with_two_stashes() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.write("b.txt", b"b\n");
    repo.commit_all("one");
    repo.write("a.txt", b"a edited\n");
    repo.git(&["stash", "push", "-q", "-m", "first"]);
    repo.write("b.txt", b"b edited\n");
    repo.git(&["stash", "push", "-q", "-m", "second"]);
    repo
}

#[test]
fn a_stash_commits_subject_is_its_reflog_message() {
    let repo = with_two_stashes();
    let subject = repo.git(&["log", "-1", "--format=%s", "stash@{0}"]);
    let reflog = repo.git(&["reflog", "show", "--format=%gs", "refs/stash"]);
    assert_eq!(
        subject,
        reflog.lines().next().expect("the reflog has a line"),
        "`stash store -m <subject>` can put the entry back unchanged only \
         while these are the same string"
    );
}

#[test]
fn storing_a_commit_the_reflog_already_holds_below_the_top_duplicates_it() {
    let repo = with_two_stashes();
    let second = repo.git(&["rev-parse", "stash@{1}"]);
    let subject = repo.git(&["log", "-1", "--format=%s", "stash@{1}"]);
    let before = stash_list(&repo).lines().count();
    repo.git(&["stash", "store", "-m", &subject, &second]);
    assert_eq!(
        stash_list(&repo).lines().count(),
        before + 1,
        "git's `stash store` is not idempotent: the executor has to scan the \
         reflog for the commit before storing it"
    );
}

#[test]
fn dropping_the_last_stash_removes_the_ref_entirely() {
    let repo = with_two_stashes();
    repo.git(&["stash", "drop", "-q", "stash@{0}"]);
    repo.git(&["stash", "drop", "-q", "stash@{0}"]);
    assert!(
        repo.try_git(&["rev-parse", "--verify", "refs/stash"])
            .is_err(),
        "with no entries left there is no refs/stash to read a reflog from"
    );
}

#[test]
fn a_failed_pop_keeps_its_entry() {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.write("a.txt", b"stashed\n");
    repo.git(&["stash", "push", "-q", "-m", "conflicting"]);
    repo.write("a.txt", b"in the way\n");

    let popped = repo.try_git(&["stash", "pop", "--index"]);
    assert!(popped.is_err(), "the pop should have refused");
    assert_eq!(
        stash_list(&repo).lines().count(),
        1,
        "a failed pop leaves the entry, which is why undo of an aborted pop \
         must not store it a second time"
    );
}

// ── stash inverses ──────────────────────────────────────────────────────

#[test]
fn undoing_a_push_drops_only_the_stash_it_made() {
    let repo = with_two_stashes();
    repo.write("a.txt", b"a edited again\n");
    let before = stash_list(&repo);

    run(
        &repo,
        &Operation::StashPush(StashParams {
            message: Some("third".to_owned()),
            scope: StashScope::All {
                include_untracked: true,
            },
        }),
        NOW,
    );
    assert_eq!(stash_list(&repo).lines().count(), 3);

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(
        stash_list(&repo),
        before,
        "the user's own stashes are intact"
    );
    assert_eq!(
        repo.read("a.txt"),
        b"a edited again\n",
        "the work came back"
    );
}

#[test]
fn undoing_a_pop_puts_the_entry_back_and_the_tree_with_it() {
    let repo = with_two_stashes();
    let top = repo.git(&["rev-parse", "stash@{0}"]);
    let before = state_of(&repo);

    let entry = run(
        &repo,
        &Operation::StashPop {
            index: 0,
            commit: graft_core::oid::Oid::from_hex(&top).expect("a stash id"),
        },
        NOW,
    );
    assert_eq!(entry.status, EntryStatus::Committed);
    assert_eq!(stash_list(&repo).lines().count(), 1);

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(
        repo.git(&["rev-parse", "stash@{0}"]),
        top,
        "the popped commit is stash@{{0}} again"
    );
    assert_eq!(state_of(&repo), before, "and the tree is back");
}

#[test]
fn a_dropped_stash_survives_gc_and_comes_back() {
    let repo = with_two_stashes();
    let top = repo.git(&["rev-parse", "stash@{0}"]);

    run(
        &repo,
        &Operation::StashDrop {
            index: 0,
            commit: graft_core::oid::Oid::from_hex(&top).expect("a stash id"),
        },
        NOW,
    );
    assert_eq!(
        stash_list(&repo).lines().count(),
        1,
        "it is gone from the list"
    );

    // Everything that made it reachable is gone but the backup ref.
    repo.git(&[
        "reflog",
        "expire",
        "--expire=now",
        "--expire-unreachable=now",
        "--all",
    ]);
    repo.git(&["gc", "--prune=now", "-q"]);
    assert!(
        repo.try_git(&["cat-file", "-e", &top]).is_ok(),
        "the pin kept the stash commit alive through gc"
    );

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(
        repo.git(&["rev-parse", "stash@{0}"]),
        top,
        "and undo put it back"
    );
}

// ── the compare-and-swap rail ───────────────────────────────────────────

#[test]
fn a_branch_moved_behind_graft_stops_the_undo_and_then_refuses_it() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"one\n");
    repo.commit_all("one");
    repo.write("a.rs", b"two\n");
    repo.git(&["add", "a.rs"]);
    run(&repo, &commit("two\n"), NOW);

    // Someone runs git themselves.
    repo.write("a.rs", b"three\n");
    repo.git(&["add", "a.rs"]);
    repo.git(&["commit", "-q", "-m", "three"]);
    let tip = repo.git(&["rev-parse", "HEAD"]);

    match run_undo(&repo, NOW + 1, false) {
        Err(UndoError::Drift { drift: moved, .. }) => {
            assert!(
                moved
                    .refs
                    .iter()
                    .any(|(name, _, _)| name == "refs/heads/main"),
                "the rail names the ref: {moved:?}"
            );
        }
        other => panic!("the rail should have fired: {other:?}"),
    }

    match run_undo(&repo, NOW + 2, true) {
        Err(UndoError::Restore(graft_ops::restore::RestoreError::RefMoved { name })) => {
            assert_eq!(name, "refs/heads/main");
        }
        other => panic!("the compare-and-swap should have refused: {other:?}"),
    }
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        tip,
        "and nothing moved after it refused"
    );
    assert_eq!(repo.read("a.rs"), b"three\n", "nor did the working tree");
}

#[test]
fn an_edit_after_a_commit_stops_the_undo_and_forcing_it_keeps_a_backup() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"one\n");
    repo.commit_all("one");
    repo.write("a.rs", b"two\n");
    repo.git(&["add", "a.rs"]);
    run(&repo, &commit("two\n"), NOW);

    repo.write("a.rs", b"an edit nobody journaled\n");
    match run_undo(&repo, NOW + 1, false) {
        Err(UndoError::Drift { drift: moved, .. }) => assert!(moved.worktree, "{moved:?}"),
        other => panic!("the rail should have fired: {other:?}"),
    }

    let forced = run_undo(&repo, NOW + 2, true).expect("forcing it goes ahead");
    let backup = forced
        .entry
        .backup_ref
        .as_deref()
        .expect("the undo took its own backup first");
    let tree = repo.git(&["rev-parse", &format!("{backup}^{{tree}}:worktree")]);
    assert_eq!(
        repo.git(&["cat-file", "-p", &format!("{tree}:a.rs")]),
        "an edit nobody journaled",
        "the undo's own backup holds the edit it overwrote"
    );
}

// ── failure part-way through ────────────────────────────────────────────

#[test]
fn a_pop_that_conflicts_leaves_a_committed_clash_that_undo_restores() {
    let repo = TempRepo::init();
    repo.write("a.txt", b"one\ntwo\nthree\n");
    repo.commit_all("one");

    // Stash a change to the first line.
    repo.write("a.txt", b"stashed\ntwo\nthree\n");
    repo.git(&["stash", "push", "-q", "-m", "will conflict"]);
    let stashed = repo.git(&["rev-parse", "stash@{0}"]);

    // Commit a different change to the same line. The working tree is clean
    // now, so nothing `check` can see says no — but the merge git does
    // inside `stash pop` cannot succeed.
    repo.write("a.txt", b"committed\ntwo\nthree\n");
    repo.git(&["add", "a.txt"]);
    repo.git(&["commit", "-q", "-m", "two"]);
    let before = state_of(&repo);

    let failed = try_run(
        &repo,
        &Operation::StashPop {
            index: 0,
            commit: graft_core::oid::Oid::from_hex(&stashed).expect("a stash id"),
        },
        NOW,
    );

    // Before m6-conflicts/72 this was `Git` + `Aborted`; a clash is now an
    // outcome of its own, `Committed` (ADR-0045 §1).
    let entry = match failed {
        Err(ApplyError::Clashed { entry, .. }) => *entry,
        other => panic!("the pop should have clashed: {other:?}"),
    };
    assert_eq!(entry.status, EntryStatus::Committed);
    assert!(entry.worktree_before.is_some(), "the backup is recorded");
    assert!(
        entry.head_after.is_some(),
        "the after-state is recorded on abort as well as on success"
    );
    assert!(
        entry.refs_after.contains_key("refs/stash"),
        "every key of refs_before is recorded after: {:?}",
        entry.refs_after
    );
    assert_ne!(
        state_of(&repo),
        before,
        "the half-applied merge really did change the tree"
    );

    run_undo(&repo, NOW + 1, true)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(
        state_of(&repo),
        before,
        "undo restored the tree as it stood before the pop"
    );
}

#[test]
fn an_operation_that_changed_nothing_never_shadows_the_one_before_it() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"one\n");
    repo.commit_all("one");
    repo.write("a.rs", b"two\n");
    run(&repo, &Operation::Stage(paths(&["a.rs"])), NOW);

    // A commit git refuses: nothing moves, so `u` must still aim at the stage.
    let refused = try_run(&repo, &commit(""), NOW + 1);
    assert!(refused.is_err());

    run_undo(&repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(
        repo.git(&["diff", "--cached", "--name-only"]),
        "",
        "the undo took back the stage, not the refusal"
    );
}

// ── an operation that died before its after-state was recorded ──────────

#[test]
fn an_entry_with_no_after_state_restores_the_whole_tree_after_asking() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"one\n");
    repo.write("b.rs", b"b\n");
    repo.commit_all("one");
    repo.write("a.rs", b"an hour of work\n");
    repo.write("untracked.txt", b"and this\n");
    let before = state_of(&repo);

    // The operation runs and the process dies before the outcome line: only
    // the pending line is on disk.
    run(&repo, &Operation::Discard(paths(&["a.rs"])), NOW);
    let path = repo
        .path()
        .join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl"));
    let text = std::fs::read_to_string(&path).expect("the journal reads");
    let pending = text.lines().next().expect("a pending line");
    std::fs::write(&path, format!("{pending}\n")).expect("the journal is truncated");

    // Recovery cannot read the repository either, so the entry is closed
    // with nothing known about what it left behind.
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let id = journal.entries()[0].id.clone();
    journal
        .mark_interrupted(&id, None)
        .expect("the entry is closed");
    let entry = &journal.entries()[0];
    assert!(entry.worktree_after.is_none(), "nothing is known after it");

    // The inverse widens to the whole tree, and says so.
    let plan = undo::plan_undo(journal.entries()).expect("it is still invertible");
    assert!(
        plan.steps.iter().any(|step| matches!(
            step,
            Step::RestoreWorktree {
                paths: PathScope::Everything,
                ..
            }
        )),
        "{:?}",
        plan.steps
    );

    let stopped = run_undo(&repo, NOW + 1, false);
    match stopped {
        Err(UndoError::Drift { drift: moved, .. }) => {
            assert!(
                moved.unknown.contains(&"the working tree"),
                "the user is told first: {moved:?}"
            );
            assert!(
                moved.explain().contains("never recorded"),
                "{}",
                moved.explain()
            );
        }
        other => panic!("the rail should have fired: {other:?}"),
    }

    run_undo(&repo, NOW + 2, true)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(
        state_of(&repo),
        before,
        "and the whole tree came back from the backup"
    );
}

#[test]
fn recovery_that_can_read_the_repository_keeps_the_restore_narrow() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"one\n");
    repo.commit_all("one");
    repo.write("a.rs", b"work\n");

    run(&repo, &Operation::Discard(paths(&["a.rs"])), NOW);
    let path = repo
        .path()
        .join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl"));
    let text = std::fs::read_to_string(&path).expect("the journal reads");
    let pending = text.lines().next().expect("a pending line");
    std::fs::write(&path, format!("{pending}\n")).expect("the journal is truncated");

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    {
        let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
        graft_ops::apply::recover_interrupted(&mut ctx).expect("recovery runs");
    }
    let plan = undo::plan_undo(journal.entries()).expect("it is invertible");
    assert!(
        plan.steps.iter().any(|step| matches!(
            step,
            Step::RestoreWorktree {
                paths: PathScope::ChangedBetween { .. },
                ..
            }
        )),
        "a readable repository keeps the scope to what changed: {:?}",
        plan.steps
    );
}

#[test]
fn an_index_only_operation_with_no_backup_cannot_be_undone_blind() {
    // `NoBackup` is the refusal that keeps a restore from guessing; assert it
    // exists on an entry with no backup rather than inventing one.
    let repo = TempRepo::init();
    repo.write("a.rs", b"one\n");
    repo.commit_all("one");
    repo.write("a.rs", b"two\n");
    run(&repo, &Operation::Stage(paths(&["a.rs"])), NOW);

    let handle = repo.open();
    let journal = journal_for(&handle);
    let mut entry = journal.entries()[0].clone();
    entry.worktree_before = None;
    entry.index_before = None;
    assert!(matches!(
        undo::inverse(&entry, None),
        Err(NotInvertible::NoBackup)
    ));
    assert!(journal.unresolved().is_empty());
}

// ── an unborn HEAD ──────────────────────────────────────────────────────

#[test]
fn undoing_the_first_commit_leaves_head_unborn_again() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"the first file\n");
    repo.write("untracked.txt", b"never staged\n");
    run(&repo, &Operation::Stage(paths(&["a.rs"])), NOW);
    let staged = repo.git(&["ls-files", "-s"]);

    let entry = run(&repo, &commit("the first commit\n"), NOW + 1);
    assert!(matches!(entry.head_before, HeadRef::Unborn(_)));
    assert!(repo.try_git(&["rev-parse", "HEAD"]).is_ok());

    run_undo(&repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert!(
        repo.try_git(&["rev-parse", "--verify", "HEAD"]).is_err(),
        "HEAD is unborn again"
    );
    assert_eq!(
        repo.git(&["symbolic-ref", "HEAD"]),
        "refs/heads/main",
        "still pointing at the branch it would create"
    );
    assert!(
        repo.try_git(&["rev-parse", "--verify", "refs/heads/main"])
            .is_err(),
        "the branch the commit created is gone"
    );
    assert_eq!(repo.git(&["ls-files", "-s"]), staged, "the index is back");
    assert_eq!(repo.read("untracked.txt"), b"never staged\n");
}

// ── ADR-0010: one journal per worktree ──────────────────────────────────

#[test]
fn a_linked_worktree_keeps_its_own_journal_and_namespaced_backup_refs() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"one\n");
    repo.commit_all("one");
    repo.git(&["branch", "side"]);
    repo.git(&["worktree", "add", "-q", "linked", "side"]);
    repo.harden(std::path::Path::new("linked"));

    let linked = repo.path().join("linked");
    std::fs::write(linked.join("a.rs"), b"changed in the linked worktree\n")
        .expect("the file is written");

    let handle = repo.open_at("linked");
    assert_eq!(handle.worktree_name(), "linked");
    let mut journal = journal_for(&handle);
    let never = Never;
    let entry = {
        let mut ctx = context(&handle, &mut journal, NOW, &never);
        apply(&mut ctx, &Operation::Stage(paths(&["a.rs"])))
            .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
            .entry
    };

    assert!(
        repo.path()
            .join(format!(
                ".git/worktrees/linked/{CURRENT_DIR}/undo-journal.jsonl"
            ))
            .exists(),
        "the journal is the linked worktree's own"
    );
    assert!(
        !repo
            .path()
            .join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl"))
            .exists(),
        "the main worktree's journal never saw it"
    );
    assert_eq!(
        entry.backup_ref.as_deref(),
        Some(
            format!(
                "{}backup/linked/{}",
                graft_core::on_disk::Generation::CURRENT.ref_prefix(),
                entry.id
            )
            .as_str()
        ),
        "the backup ref carries the worktree's name"
    );
    assert_eq!(
        repo.git(&[
            "for-each-ref",
            "--format=%(refname)",
            &format!("refs/{CURRENT_DIR}")
        ])
        .lines()
        .count(),
        1
    );

    // And the main worktree's journal is empty, so `u` there has nothing.
    let main = repo.open();
    let main_journal = journal_for(&main);
    assert!(main_journal.entries().is_empty());
}

// ── the undo entry's own shape ──────────────────────────────────────────

#[test]
fn an_undo_is_journaled_as_an_undo_with_its_own_commands() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"one\n");
    repo.commit_all("one");
    repo.write("a.rs", b"two\n");
    let staged = run(&repo, &Operation::Stage(paths(&["a.rs"])), NOW);
    let undone = run_undo(&repo, NOW + 1, false).expect("undo runs");

    assert_eq!(undone.entry.kind, OperationKind::Undo);
    assert_eq!(undone.entry.reverses.as_ref(), Some(&staged.id));
    assert!(undone.entry.description.starts_with("undo: "));
    assert!(
        undone
            .entry
            .commands
            .iter()
            .any(|line| line.contains("update-index")),
        "{:?}",
        undone.entry.commands
    );
}

// ── everything the entry names outlives git's own windows ───────────────
//
// `gc.pruneExpire` is two weeks and `gc.reflogExpireUnreachable` is thirty
// days, and `gc --auto` runs behind ordinary commits. An id a journal entry
// records has to be reachable from that entry's backup ref, or `u` works for
// a fortnight and then stops — with the entry still on screen promising it
// can be undone (m3/03 review, C1). Each of these prunes between the
// operation and the undo.

#[test]
fn an_index_only_operations_after_tree_survives_an_aggressive_prune() {
    // Two files edited and only one staged, so the tree staging leaves is a
    // tree nothing else in the repository holds: `a.rs` new, `b.rs` old. The
    // backup's own worktree tree has both new, and `HEAD`'s has both old.
    let repo = TempRepo::init();
    repo.write("a.rs", b"one\n");
    repo.write("b.rs", b"one\n");
    repo.commit_all("one");
    repo.write("a.rs", b"two\n");
    repo.write("b.rs", b"two\n");
    let before = state_of(&repo);

    let entry = run(&repo, &Operation::Stage(paths(&["a.rs"])), NOW);
    let after_tree = entry
        .index_after
        .unwrap_or_else(|| panic!("staging records the index it left"));
    let after = state_of(&repo);

    expire_and_prune(&repo);
    assert!(
        holds(&repo, &after_tree.tree().to_string()),
        "the tree the entry names must outlive gc; without it `u` cannot \
         even work out which paths to put back"
    );

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(state_of(&repo), before, "and the undo still puts it back");

    crate::support::run_redo(&repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(state_of(&repo), after, "and the redo still puts it forward");
}

#[test]
fn an_amends_pre_amend_commit_survives_an_aggressive_prune() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"one\n");
    repo.commit_all("one");
    repo.write("a.rs", b"two\n");
    repo.git(&["add", "a.rs"]);
    let replaced = repo.git(&["rev-parse", "HEAD"]);
    let before = state_of(&repo);

    run(
        &repo,
        &Operation::Amend(CommitParams {
            message: "one, amended\n".to_owned(),
            sign_off: false,
            no_verify: false,
            allow_empty: false,
            include: graft_ops::operation::Include::Staged,
        }),
        NOW,
    );
    assert_ne!(repo.git(&["rev-parse", "HEAD"]), replaced);

    expire_and_prune(&repo);
    assert!(
        holds(&repo, &replaced),
        "the commit the amend replaced must outlive gc: its reflog entry is \
         gone after thirty days, and with it the message and the tree"
    );

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        replaced,
        "undo put the pre-amend commit back"
    );
    assert_eq!(state_of(&repo), before);
}

#[test]
fn a_commits_own_commit_survives_an_aggressive_prune_across_undo_and_redo() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"one\n");
    repo.commit_all("one");
    repo.write("a.rs", b"two\n");
    repo.git(&["add", "a.rs"]);
    let before = state_of(&repo);

    run(&repo, &commit("two\n"), NOW);
    let made = repo.git(&["rev-parse", "HEAD"]);
    let after = state_of(&repo);

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(state_of(&repo), before);

    // The branch has moved off the commit, so only a backup ref is holding
    // it — which is exactly what redo needs a fortnight later.
    expire_and_prune(&repo);
    assert!(holds(&repo, &made), "the undone commit is still there");

    crate::support::run_redo(&repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(state_of(&repo), after, "redo put the commit back");
}

#[test]
fn a_conflicted_pops_after_tree_survives_an_aggressive_prune() {
    let repo = TempRepo::init();
    repo.write("a.txt", b"one\ntwo\nthree\n");
    repo.commit_all("one");
    repo.write("a.txt", b"stashed\ntwo\nthree\n");
    repo.git(&["stash", "push", "-q", "-m", "will conflict"]);
    let stashed = stash_at(&repo, 0);
    repo.write("a.txt", b"committed\ntwo\nthree\n");
    repo.git(&["add", "a.txt"]);
    repo.git(&["commit", "-q", "-m", "two"]);
    let before = state_of(&repo);

    let failed = try_run(
        &repo,
        &Operation::StashPop {
            index: 0,
            commit: stashed,
        },
        NOW,
    );
    // A clash since m6-conflicts/72 (ADR-0045 §1), journaled `Committed`.
    let entry = match failed {
        Err(ApplyError::Clashed { entry, .. }) => *entry,
        other => panic!("the pop should have clashed: {other:?}"),
    };
    // The index is unmerged, and since m6-conflicts/08 that is a recorded
    // state of its own — the stage tree — not an absence. Both it and the
    // conflicted working tree have to outlive `gc`.
    let index_after = entry
        .index_after
        .unwrap_or_else(|| panic!("the unmerged index is recorded"));
    assert!(
        index_after.is_unmerged(),
        "a conflicted pop leaves an unmerged index: {index_after:?}"
    );
    let after_tree = entry
        .worktree_after
        .unwrap_or_else(|| panic!("the conflicted working tree is recorded"));

    expire_and_prune(&repo);
    assert!(
        holds(&repo, &after_tree.to_string()),
        "the conflicted tree the entry names must outlive gc"
    );
    assert!(
        holds(&repo, &index_after.tree().to_string()),
        "the stage tree the entry names must outlive gc too"
    );

    // No `force`: the recorded after-state and the repository now are the
    // same unmerged index, so there is no drift to force past
    // (`conflicted_pop_undo_is_clean_without_force` proves the drift read
    // itself; this proves `u` goes through).
    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(state_of(&repo), before);
}

// ── stash inverses, both legs ───────────────────────────────────────────

#[test]
fn stash_apply_round_trips() {
    let repo = one_stash_repo();
    round_trip(
        &repo,
        &Operation::StashApply {
            index: 0,
            commit: stash_at(&repo, 0),
        },
    );
}

#[test]
fn stash_pop_round_trips() {
    let repo = one_stash_repo();
    round_trip(
        &repo,
        &Operation::StashPop {
            index: 0,
            commit: stash_at(&repo, 0),
        },
    );
}

#[test]
fn stash_drop_round_trips() {
    let repo = with_two_stashes();
    round_trip(
        &repo,
        &Operation::StashDrop {
            index: 0,
            commit: stash_at(&repo, 0),
        },
    );
}

fn one_stash_repo() -> TempRepo {
    let repo = TempRepo::init();
    one_stash(&repo);
    repo
}

// ── a plan with more than one invocation ────────────────────────────────

#[test]
fn a_plan_stopped_between_invocations_is_aborted_recoverable_and_narrow() {
    // Three classes, so `plan_discard` is three invocations: the staged new
    // file first, then the unstaged edit, then the untracked file.
    let repo = TempRepo::init();
    repo.write("kept.rs", b"one\n");
    repo.commit_all("one");
    repo.write("staged-new.rs", b"new\n");
    repo.git(&["add", "staged-new.rs"]);
    repo.write("kept.rs", b"edited\n");
    repo.write("untracked.bin", &[0, 255, 7]);
    let before = state_of(&repo);

    // The first invocation removes `staged-new.rs`; from then on the stop
    // answers yes, so the second is the one that gets killed.
    let stop = StopWhenGone::new(repo.path().join("staged-new.rs"));
    let outcome = run_with_stop(
        &repo,
        &Operation::Discard(paths(&["staged-new.rs", "kept.rs", "untracked.bin"])),
        NOW,
        &stop,
    );
    // m8-polish-and-ship/13d Q3: a discard can never be paused, so its stop
    // says only that git was stopped, and that something changed.
    let stopped_at = outcome.as_ref().err().and_then(ApplyError::stopped_at);
    assert_eq!(stopped_at, Some(StoppedAt::Stopped { changed: true }));
    let entry = match outcome {
        Err(ApplyError::Git {
            entry, recoverable, ..
        }) => {
            assert!(recoverable, "a backup was taken before the plan started");
            *entry
        }
        other => panic!("the plan should have been cut short: {other:?}"),
    };
    assert!(
        matches!(&entry.status, EntryStatus::Aborted { reason } if reason == "git was stopped"),
        "{:?}",
        entry.status
    );
    assert_eq!(entry.commands.len(), 3, "every planned line is journaled");
    assert!(!repo.exists("staged-new.rs"), "the first invocation ran");
    assert_eq!(repo.read("kept.rs"), b"edited\n", "the second did not");

    // The after-state was read although the stop was still set — the reads
    // come after the operation, so cancelling one only widens the restore
    // (m3/03 review, M1). A recorded after-state means `u` needs no force.
    assert!(
        entry.worktree_after.is_some(),
        "a cancelled operation still records what it left behind"
    );
    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(state_of(&repo), before, "the half-done discard is undone");
}

#[test]
fn a_two_invocation_patch_discard_that_fails_first_changes_nothing() {
    // `Discard(Patch { side: Index })` plans two: the working tree first,
    // because that is the step that can fail. An unstaged edit over the same
    // lines makes it fail, and the index must be left alone.
    let repo = TempRepo::init();
    repo.write("a.txt", b"one\ntwo\nthree\n");
    repo.commit_all("one");
    repo.write("a.txt", b"one\nSTAGED\nthree\n");
    repo.git(&["add", "a.txt"]);
    repo.write("a.txt", b"one\nIN THE WAY\nthree\n");
    let before = state_of(&repo);

    let patch = repo.git(&["diff", "--cached", "--", "a.txt"]);
    let outcome = try_run(
        &repo,
        &Operation::Discard(Target::Patch {
            path: PathBuf::from("a.txt"),
            patch: format!("{patch}\n").into_bytes(),
            side: PatchSide::Index,
        }),
        NOW,
    );
    let entry = match outcome {
        Err(ApplyError::Git { entry, .. }) => *entry,
        other => panic!("the first invocation should have failed: {other:?}"),
    };
    assert!(matches!(entry.status, EntryStatus::Aborted { .. }));
    assert_eq!(
        entry.commands.len(),
        2,
        "both planned lines are journaled even though one never ran"
    );
    assert_eq!(
        state_of(&repo),
        before,
        "the working tree goes first precisely so a failure leaves nothing \
         changed — the index must not be half-reversed"
    );
}
