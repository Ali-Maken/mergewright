//! Cancelling an undo: `Esc` between steps puts back what already ran.
//!
//! No sleeps and no hooks: the stops here are a **counting** one —
//! `should_stop` true from its n-th poll — for sweeping every place a cancel
//! can land, and a **state-keyed** one for aiming at a single step boundary.
//! Counting alone cannot aim: the runner polls the same stop while it waits
//! on a child, so the n-th poll is a different place on a different machine
//! (the reasoning `support::StopWhenGone` already sets out). Keying the stop
//! to the plan's own effect — "once the ref step has landed" — means the same
//! place every time.
//!
//! What every test asserts is the invariant, read back out of the repository
//! with `git` and not from a returned value: **a cancelled undo either
//! finished or changed nothing.** Whether the cancel landed between two steps
//! or inside one does not matter, and deliberately so: both have to hold.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use graft_core::journal::EntryStatus;
use graft_core::oid::Oid;
use graft_git::run::Stop;
use graft_ops::apply::Applied;
use graft_ops::operation::{Operation, ResetMode, ResetParams, ResetTarget, Tip};
use graft_ops::restore::{self, RestoreError};
use graft_ops::undo::{self, InversePlan, Step, UndoError};

use crate::support::{NOW, Never, RepoState, TempRepo, context, journal_for, run, state_of};

// ── the repository every test here cancels an undo in ───────────────────

/// Three commits, then a `reset --hard` back over the last two.
///
/// Undoing that reset is a three-step plan — the branch ref, the index, the
/// working tree — which is the smallest plan that can be cut short in every
/// interesting place. Three commits rather than two so the middle one is a
/// commit id that is neither where the ref step puts `main` nor where the
/// restore found it: `rollback_refused_by_a_moved_ref_is_reported_not_hidden`
/// needs somewhere genuinely else to move the ref to.
fn reset_repo() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.write("a.txt", b"first a\n");
    repo.commit_all("two");
    repo.write("b.txt", b"first b\n");
    repo.commit_all("three");
    repo
}

fn head_tip(repo: &TempRepo) -> Tip {
    Tip {
        name: repo.git(&["symbolic-ref", "HEAD"]),
        oid: oid(repo, "HEAD"),
    }
}

fn oid(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("git prints an object id")
}

/// Run the reset and return the repository as it stands afterwards: the
/// state a cancelled undo has to leave untouched.
fn reset_and_state(repo: &TempRepo) -> RepoState {
    let to = oid(repo, "HEAD~2");
    let op = Operation::Reset(ResetParams {
        target: ResetTarget::Head {
            branch: head_tip(repo),
            mode: ResetMode::Hard,
        },
        to,
    });
    run(repo, &op, NOW);
    state_of(repo)
}

// ── stops ───────────────────────────────────────────────────────────────

/// A `Stop` that counts polls and never fires.
struct CountPolls {
    seen: AtomicUsize,
}

impl CountPolls {
    fn new() -> CountPolls {
        CountPolls {
            seen: AtomicUsize::new(0),
        }
    }

    fn count(&self) -> usize {
        self.seen.load(Ordering::Relaxed)
    }
}

impl Stop for CountPolls {
    fn should_stop(&self) -> bool {
        self.seen.fetch_add(1, Ordering::Relaxed);
        false
    }
}

/// A `Stop` that is true from its n-th poll onwards.
struct StopAtPoll {
    at: usize,
    seen: AtomicUsize,
}

impl StopAtPoll {
    fn new(at: usize) -> StopAtPoll {
        StopAtPoll {
            at,
            seen: AtomicUsize::new(0),
        }
    }
}

impl Stop for StopAtPoll {
    fn should_stop(&self) -> bool {
        self.seen.fetch_add(1, Ordering::Relaxed) + 1 >= self.at
    }
}

/// What makes a [`StopOnce`] fire.
enum Trigger {
    /// A ref stands where the plan's ref step puts it: that step has landed.
    RefIs { name: String, at: String },
    /// A path the plan's working-tree step writes is on disk: every step
    /// before that one has landed.
    PathExists(PathBuf),
}

/// A `Stop` that fires once the plan itself has done something, and latches
/// there.
///
/// Counting polls is the wrong clock for aiming at one step boundary: the
/// runner polls the same stop while it waits on a child, so the n-th poll is
/// a different place on a different machine, and `begin`'s own snapshot polls
/// several times before the first step even starts. Keying the stop to the
/// plan's own effect makes "stop once the ref step has run" mean exactly
/// that (the reasoning `support::StopWhenGone` already sets out).
///
/// With `then_move_to`, the poll that fires also moves a ref somewhere else:
/// the repository changing under the roll-back's feet, with no sleep and no
/// hook, because the write happens inside the very `should_stop` call that
/// cuts the plan short.
struct StopOnce {
    repo: PathBuf,
    trigger: Trigger,
    then_move_to: Option<(String, String)>,
    fired: AtomicUsize,
}

impl StopOnce {
    fn new(repo: &Path, trigger: Trigger) -> StopOnce {
        StopOnce {
            repo: repo.to_path_buf(),
            trigger,
            then_move_to: None,
            fired: AtomicUsize::new(0),
        }
    }

    fn then_move(mut self, name: &str, to: &str) -> StopOnce {
        self.then_move_to = Some((name.to_owned(), to.to_owned()));
        self
    }

    fn fired(&self) -> bool {
        self.fired.load(Ordering::Relaxed) > 0
    }

    fn git(&self, args: &[&str]) -> Option<String> {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.repo)
            .args(args)
            .output()
            .expect("git runs");
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim_end().to_owned())
    }

    fn triggered(&self) -> bool {
        match &self.trigger {
            Trigger::RefIs { name, at } => {
                self.git(&["rev-parse", "--verify", "--quiet", name])
                    .as_deref()
                    == Some(at.as_str())
            }
            Trigger::PathExists(path) => self.repo.join(path).exists(),
        }
    }
}

impl Stop for StopOnce {
    fn should_stop(&self) -> bool {
        // Latched: once it has fired, the repository may well have been
        // changed back again by this very stop, and a cancel does not un-ask
        // itself.
        if self.fired() {
            return true;
        }
        if !self.triggered() {
            return false;
        }
        self.fired.fetch_add(1, Ordering::Relaxed);
        if let Some((name, to)) = &self.then_move_to {
            self.git(&["update-ref", name, to])
                .expect("the test's own update-ref succeeds");
        }
        true
    }
}

// ── running the undo plan under a stop ──────────────────────────────────

/// The plan `u` would run, and `restore::execute` under a stop of the
/// caller's choosing.
///
/// This goes straight at `execute` rather than through `undo::undo` because
/// `undo` reads the whole repository first (§6.7's rails), and those reads
/// poll the same stop: counting polls would then mostly count the rail, not
/// the plan.
fn execute_under(repo: &TempRepo, at: i64, stop: &dyn Stop) -> Result<Applied, RestoreError> {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let plan = undo::plan_undo(journal.entries()).expect("the reset is undoable");
    let mut ctx = context(&handle, &mut journal, at, stop);
    restore::execute(&mut ctx, &plan)
}

/// Where the plan's first step puts `refs/heads/main`.
fn first_ref_step_target(repo: &TempRepo) -> String {
    let plan = undo_plan(repo);
    match plan.steps.first() {
        Some(Step::UpdateRef {
            name, to: Some(to), ..
        }) if name == "refs/heads/main" => to.to_string(),
        other => panic!("the plan's first step is not the branch ref: {other:?}"),
    }
}

fn undo_plan(repo: &TempRepo) -> InversePlan {
    let handle = repo.open();
    let journal = journal_for(&handle);
    undo::plan_undo(journal.entries()).expect("the reset is undoable")
}

/// Every journal entry, newest last.
fn entries(repo: &TempRepo) -> Vec<graft_core::journal::JournalEntry> {
    let handle = repo.open();
    journal_for(&handle).entries().to_vec()
}

/// Is there a stale lock anywhere under `.git`?
fn locks(repo: &TempRepo) -> Vec<String> {
    let mut found = Vec::new();
    let mut stack = vec![repo.path().join(".git")];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|end| end == "lock") {
                found.push(path.to_string_lossy().into_owned());
            }
        }
    }
    found
}

// ── the tests ───────────────────────────────────────────────────────────

#[test]
fn the_plan_this_file_cancels_is_three_steps() {
    // The other tests are only as sharp as the plan they cut: if the inverse
    // of a `reset --hard` ever stops touching all three of the ref, the index
    // and the working tree, they would still pass while proving much less.
    let repo = reset_repo();
    reset_and_state(&repo);
    let plan = undo_plan(&repo);
    assert_eq!(plan.steps.len(), 3, "steps: {:?}", plan.steps);
    assert!(plan.touches.index, "the plan touches the index");
    assert!(plan.touches.worktree, "the plan touches the working tree");
    assert_eq!(
        plan.touches.refs,
        vec!["refs/heads/main".to_owned()],
        "the plan touches the branch"
    );
}

#[test]
fn cancel_between_steps_rolls_back_to_pre_undo_state() {
    let repo = reset_repo();
    let before = reset_and_state(&repo);
    let next_undo_before = undo_plan(&repo).reverses;

    // Cut the plan the moment its first step — the branch ref — has landed,
    // so the index and working-tree steps are still ahead of it.
    let undone = first_ref_step_target(&repo);
    let stop = StopOnce::new(
        repo.path(),
        Trigger::RefIs {
            name: "refs/heads/main".to_owned(),
            at: undone,
        },
    );
    let why = execute_under(&repo, NOW + 1, &stop).expect_err("the undo is cancelled");
    assert!(stop.fired(), "the stop fired on the ref step's own effect");

    match &why {
        RestoreError::Cancelled { rolled_back } => assert!(
            *rolled_back >= 1,
            "at least the ref step was put back, not a cancel before anything ran"
        ),
        other => panic!("expected a clean cancel, got: {}", other.explain()),
    }
    assert_eq!(
        state_of(&repo),
        before,
        "refs, the index and the working-tree bytes are back where the undo found them"
    );
    assert_eq!(
        locks(&repo),
        Vec::<String>::new(),
        "no stale lock under .git"
    );

    // The cancelled undo is journaled, `Aborted("cancelled")`, and its
    // after-state equals its before-state — so it is not in force, shadows
    // nothing, and `u` still points at the reset.
    let entries = entries(&repo);
    let last = entries.last().expect("the cancelled undo is journaled");
    assert_eq!(
        last.status,
        EntryStatus::Aborted {
            reason: "cancelled".to_owned()
        }
    );
    assert_eq!(last.refs_after, last.refs_before, "the refs came back");
    assert_eq!(last.index_after, last.index_before, "the index came back");
    assert_eq!(
        last.worktree_after, last.worktree_before,
        "the working tree came back"
    );
    assert_eq!(
        undo_plan(&repo).reverses,
        next_undo_before,
        "u still points at the reset"
    );
}

#[test]
fn cancel_at_every_step_boundary_is_clean() {
    // Every poll the plan makes, not only the ones between steps: the runner
    // polls the same stop while it waits on a child, so a cancel can also
    // land inside a step, and the invariant has to hold there too.
    let counted = {
        let repo = reset_repo();
        reset_and_state(&repo);
        let counter = CountPolls::new();
        execute_under(&repo, NOW + 1, &counter).expect("an undo that is never stopped succeeds");
        counter.count()
    };
    assert!(counted >= 4, "the plan polls the stop more than once");

    // The invariant, at every one of them: the undo either finished or left
    // nothing behind. A run can finish even with the stop armed — the runner
    // polls while it waits on a child, so a fast git makes fewer polls than
    // the counting run did and the stop is simply never reached — and that is
    // a pass, not a miss: it is the "or finished" half.
    let mut rolled_something_back = 0;
    let done = state_after_undo();
    for at in 1..=counted {
        let repo = reset_repo();
        let before = reset_and_state(&repo);
        let stop = StopAtPoll::new(at);
        match execute_under(&repo, NOW + 1, &stop) {
            Ok(_applied) => assert_eq!(
                state_of(&repo).files,
                done.files,
                "the undo at poll {at} finished, so the files are the undone ones"
            ),
            Err(why) => {
                assert!(
                    !matches!(why, RestoreError::CancelledPartway { .. }),
                    "nothing moved the repository, so the roll-back had no reason to refuse \
                     (poll {at}): {}",
                    why.explain()
                );
                if matches!(&why, RestoreError::Cancelled { rolled_back } if *rolled_back > 0) {
                    rolled_something_back += 1;
                }
                assert_eq!(
                    state_of(&repo),
                    before,
                    "cancelling at poll {at} left the repository somewhere else"
                );
            }
        }
        assert_eq!(
            locks(&repo),
            Vec::<String>::new(),
            "cancelling at poll {at} left a lock under .git"
        );
    }
    // Without this the sweep could pass on cancels that never reached a
    // single step, proving the roll-back runs nowhere.
    assert!(
        rolled_something_back >= 2,
        "the sweep has to cut the plan after a step that ran, not only before the first"
    );
}

/// The working tree as it stands once the undo has run to the end.
fn state_after_undo() -> RepoState {
    let repo = reset_repo();
    let done = state_of(&repo);
    reset_and_state(&repo);
    done
}

#[test]
fn rollback_refused_by_a_moved_ref_is_reported_not_hidden() {
    let repo = reset_repo();
    // The middle commit, read before the reset buries it: somewhere the ref
    // step does not put `main` and the restore did not find it. Moving it to
    // either of those would be no move at all, and the roll-back would be
    // right to succeed.
    let elsewhere = repo.git(&["rev-parse", "HEAD~1"]);
    let found_at = repo.git(&["rev-parse", "HEAD~2"]);
    reset_and_state(&repo);
    let undone = first_ref_step_target(&repo);
    assert_ne!(elsewhere, undone, "the test moves main somewhere new");
    assert_ne!(elsewhere, found_at, "the test moves main somewhere new");

    // The stop fires once the plan's *last* step has started writing files,
    // so the ref step is long finished and certainly in the roll-back's list,
    // and moves `main` away in the same breath: the roll-back's own
    // compare-and-swap then meets a ref something else has moved.
    let stop = StopOnce::new(repo.path(), Trigger::PathExists(PathBuf::from("a.txt")))
        .then_move("refs/heads/main", &elsewhere);
    let why = execute_under(&repo, NOW + 1, &stop).expect_err("the undo is cancelled");
    assert!(stop.fired(), "the test's own write ran");
    assert_eq!(
        repo.git(&["rev-parse", "refs/heads/main"]),
        elsewhere,
        "main is where the test moved it, not where the undo left it"
    );

    let RestoreError::CancelledPartway { applied, rollback } = &why else {
        panic!("expected a part-way cancel, got: {}", why.explain());
    };
    assert!(
        matches!(rollback.as_ref(), RestoreError::RefMoved { name } if name == "refs/heads/main"),
        "the roll-back names the ref it refused over: {rollback}"
    );
    assert_eq!(
        applied,
        &vec!["refs/heads/main".to_owned()],
        "only the ref is still in force — the index and working-tree steps \
         were reversed before the roll-back reached it"
    );
    assert!(
        why.explain().contains("u shows the backup"),
        "the user is pointed at the backup: {}",
        why.explain()
    );
    // And it reversed what it could: the files the working-tree step had
    // written are gone again.
    assert!(
        !repo.exists("b.txt"),
        "the working-tree step was reversed even though the ref step could not be"
    );

    // The outcome line says so too, naming the ref, and the entry keeps its
    // backup ref so `u` on it can still get everything back.
    let entries = entries(&repo);
    let last = entries.last().expect("the cancelled undo is journaled");
    let EntryStatus::Aborted { reason } = &last.status else {
        panic!("expected an aborted entry, got {:?}", last.status);
    };
    assert!(
        reason.starts_with("cancelled — rollback failed:") && reason.contains("refs/heads/main"),
        "the outcome reason names the ref: {reason}"
    );
    assert!(last.backup_ref.is_some(), "the backup ref is recorded");
    assert!(
        repo.try_git(&["rev-parse", "--verify", last.backup_ref.as_ref().unwrap()])
            .is_ok(),
        "the backup ref still exists"
    );
}

#[test]
fn a_ref_moved_under_a_killed_step_is_reported_not_guessed() {
    // The other half of the moved-ref story: the stop fires the moment the
    // ref step's own `update-ref` has written, which is inside that step, so
    // the runner SIGTERMs git and the step comes back `Stopped`. Whether it
    // landed is then not knowable from the error — and the ref has been moved
    // away as well, so reading it says neither "landed" nor "did not". That
    // is reported, never guessed at with a write.
    let repo = reset_repo();
    let elsewhere = repo.git(&["rev-parse", "HEAD~1"]);
    reset_and_state(&repo);
    let undone = first_ref_step_target(&repo);
    assert_ne!(elsewhere, undone, "the test moves main somewhere new");

    let stop = StopOnce::new(
        repo.path(),
        Trigger::RefIs {
            name: "refs/heads/main".to_owned(),
            at: undone,
        },
    )
    .then_move("refs/heads/main", &elsewhere);
    let why = execute_under(&repo, NOW + 1, &stop).expect_err("the undo is cancelled");
    assert!(stop.fired(), "the test's own write ran");

    let RestoreError::CancelledPartway { applied, rollback } = &why else {
        panic!("expected a part-way cancel, got: {}", why.explain());
    };
    assert!(
        matches!(rollback.as_ref(), RestoreError::RefMoved { name } if name == "refs/heads/main"),
        "the ref it could not place is named: {rollback}"
    );
    assert!(
        applied.contains(&"refs/heads/main".to_owned()),
        "the ref nobody can account for is named: {applied:?}"
    );
    // Nothing was written on a guess: `main` is exactly where the test put
    // it, not forced to either of the two values the restore knew about.
    assert_eq!(
        repo.git(&["rev-parse", "refs/heads/main"]),
        elsewhere,
        "the roll-back did not overwrite a ref it could not account for"
    );
}

#[test]
fn a_stop_that_never_fires_changes_nothing() {
    // The existing behaviour, restated where the cancel path can break it: an
    // undo under a stop that never says yes is the same undo as before.
    let repo = reset_repo();
    let before = state_of(&repo);
    reset_and_state(&repo);
    let never = Never;
    execute_under(&repo, NOW + 1, &never).expect("the undo runs");
    assert_eq!(state_of(&repo), before, "the undo put the reset back");
}

#[test]
fn a_cancelled_undo_through_u_leaves_u_where_it_was() {
    // The whole path, rails included: `undo::undo`, not `restore::execute`
    // on its own. The poll count is learnt from a throwaway repository of the
    // same shape — the two hold different commit ids, so only the count
    // crosses over, never a state.
    let counted = {
        let repo = reset_repo();
        reset_and_state(&repo);
        let counter = CountPolls::new();
        let handle = repo.open();
        let mut journal = journal_for(&handle);
        let mut ctx = context(&handle, &mut journal, NOW + 1, &counter);
        undo::undo(&mut ctx, false).expect("an undo that is never stopped succeeds");
        counter.count()
    };
    assert!(counted > 2, "the rails and the plan both poll the stop");

    let repo = reset_repo();
    let before = reset_and_state(&repo);
    let next_undo_before = undo_plan(&repo).reverses;

    // Near the end, so the rails are long past and the plan is under way.
    let stop = StopAtPoll::new(counted - 1);
    let why = {
        let handle = repo.open();
        let mut journal = journal_for(&handle);
        let mut ctx = context(&handle, &mut journal, NOW + 1, &stop);
        undo::undo(&mut ctx, false).expect_err("the undo is cancelled")
    };

    assert!(
        matches!(
            &why,
            UndoError::Restore(RestoreError::Cancelled { .. } | RestoreError::Stopped)
                | UndoError::Facts(_)
        ),
        "a cancel, not a half-done undo: {}",
        why.explain()
    );
    assert_eq!(
        state_of(&repo),
        before,
        "the repository is where u found it"
    );
    assert_eq!(
        undo_plan(&repo).reverses,
        next_undo_before,
        "u still points at the reset"
    );
}
