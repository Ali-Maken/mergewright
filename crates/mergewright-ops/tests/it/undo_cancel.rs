//! Cancelling an undo: `Esc` between steps puts back what already ran.
//!
//! No sleeps, and one hook: the stops here are a **counting** one —
//! `should_stop` true from its n-th poll — for sweeping the places a cancel
//! can land (see `SWEEP_STRIDE`: a sample in the fast tier, all of them in the
//! heavy one), and a **state-keyed** one for aiming at a single step boundary.
//! Counting alone cannot aim: the runner polls the same stop while it waits
//! on a child, so the n-th poll is a different place on a different machine
//! (the reasoning `support::StopWhenGone` already sets out). Keying the stop
//! to the plan's own effect — "once the ref step has landed" — means the same
//! place every time.
//!
//! A stop the runner polls after git has exited is not a stop: the step ran
//! to the end and is reported done (m7-power-tools/27b). So "once the ref
//! step has landed" cuts the plan at the *next* boundary, and the one test
//! that needs a step genuinely killed inside it holds git in its
//! `reference-transaction` hook until the runner kills it
//! ([`hold_main_update_until_killed`]) — the same device `set_aside.rs` uses.
//!
//! What every test asserts is the invariant, read back out of the repository
//! with `git` and not from a returned value: **a cancelled undo either
//! finished or changed nothing.** Whether the cancel landed between two steps
//! or inside one does not matter, and deliberately so: both have to hold.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use mergewright_core::journal::EntryStatus;
use mergewright_core::oid::Oid;
use mergewright_git::run::Stop;
use mergewright_ops::apply::Applied;
use mergewright_ops::facts;
use mergewright_ops::journal::{next_redo, next_undo};
use mergewright_ops::operation::{Operation, ResetMode, ResetParams, ResetTarget, Tip};
use mergewright_ops::restore::{self, RestoreError};
use mergewright_ops::undo::{self, InversePlan, Step, UndoError};

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
    /// before that one has landed. Also a marker a test's hook writes.
    PathExists(PathBuf),
    /// A path the plan's index step stages is in the index: that step has
    /// landed, and the working-tree step has not started.
    InIndex(PathBuf),
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
            Trigger::InIndex(path) => {
                let path = path.to_str().expect("utf-8");
                self.git(&["ls-files", "--", path])
                    .is_some_and(|listed| !listed.is_empty())
            }
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

/// Hold the next transaction that updates `refs/heads/main` in its
/// `reference-transaction` hook at `state` — `prepared`: the ref locked, not
/// yet written; `committed`: written — until the runner's stop kills git
/// (its parent pid gone). The
/// hook first writes a marker under `.git`, returned here relative to the
/// repository root, for a [`Trigger::PathExists`] stop to key on. So the stop
/// always lands while git is still running, never racing it to the finish.
/// Bounded at ten seconds, so a stop that never fires fails the test instead
/// of hanging it; removes itself when it fires.
fn hold_main_update_until_killed(repo: &TempRepo, state: &str) -> PathBuf {
    let marker = PathBuf::from(".git/held-in-transaction");
    let hooks = repo.path().join(".git/test-hooks");
    std::fs::create_dir_all(&hooks).expect("the hooks dir is made");
    let hook = hooks.join("reference-transaction");
    let script = format!(
        "#!/bin/sh\n\
         [ \"$1\" = {state} ] || exit 0\n\
         case \"$(cat)\" in *\" refs/heads/main\"*) ;; *) exit 0 ;; esac\n\
         rm -f \"$0\"\n\
         : > '{marker}'\n\
         p=$PPID; n=0\n\
         while kill -0 \"$p\" 2>/dev/null && [ \"$n\" -lt 1000 ]; do sleep 0.01; n=$((n + 1)); done\n",
        marker = repo.path().join(&marker).display(),
    );
    std::fs::write(&hook, script).expect("the hook is written");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
        .expect("the hook is executable");
    repo.git(&["config", "core.hooksPath", hooks.to_str().expect("utf-8")]);
    marker
}

fn undo_plan(repo: &TempRepo) -> InversePlan {
    let handle = repo.open();
    let journal = journal_for(&handle);
    undo::plan_undo(journal.entries()).expect("the reset is undoable")
}

/// Every journal entry, newest last.
fn entries(repo: &TempRepo) -> Vec<mergewright_core::journal::JournalEntry> {
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
        other => panic!(
            "expected a clean cancel, got: {}",
            other.explain(&crate::support::Letters)
        ),
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

/// How many of the plan's poll points the sweep below cuts at: every one
/// under `--features heavy-tests`, every third in the fast tier.
///
/// The cost is one whole `TempRepo` — `git init`, three commits, a journaled
/// reset — per cut, about 0.8 s, and the plan makes around thirty polls. At
/// full depth that is a 16–24 s test, which the fast tier cannot carry
/// (PROMPT 7.1); the heavy tier exists for exactly this. What the fast tier
/// keeps is the invariant at ten points spread across all three steps, still
/// including cuts inside a running step, with `rolled_something_back` below
/// proving the sweep reaches past the first step. What it loses is
/// exhaustiveness: one particular poll where the invariant broke could now be
/// missed until the heavy tier runs in the milestone review.
const SWEEP_STRIDE: usize = if cfg!(feature = "heavy-tests") { 1 } else { 3 };

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
    for at in (1..=counted).step_by(SWEEP_STRIDE) {
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
                    why.explain(&crate::support::Letters)
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

/// One part-way cancel, set up the same way for both tests below.
///
/// The stop fires once the index step has landed, so the ref step and the
/// index step are both in the roll-back's list and the working-tree step
/// never starts, and moves `main` away in the same breath: the roll-back's
/// own compare-and-swap then meets a ref something else has moved.
///
/// Until 27b this keyed on the working-tree step writing `a.txt`, and relied
/// on the runner calling that step `Stopped` once git had written it and
/// exited. That step is the plan's last: a stop seen after it finished is a
/// finished undo (`Applied`), not a part-way one — so the part-way cancel
/// has to land at a boundary with a step still to come.
struct PartWay {
    repo: TempRepo,
    /// Where the undo found the repository — what a recovery has to reach.
    before: RepoState,
    /// Where the test moved `main`.
    elsewhere: String,
    why: RestoreError,
}

fn cancel_part_way() -> PartWay {
    let repo = reset_repo();
    // The middle commit, read before the reset buries it: somewhere the ref
    // step does not put `main` and the restore did not find it. Moving it to
    // either of those would be no move at all, and the roll-back would be
    // right to succeed.
    let elsewhere = repo.git(&["rev-parse", "HEAD~1"]);
    let found_at = repo.git(&["rev-parse", "HEAD~2"]);
    let before = reset_and_state(&repo);
    let undone = first_ref_step_target(&repo);
    assert_ne!(elsewhere, undone, "the test moves main somewhere new");
    assert_ne!(elsewhere, found_at, "the test moves main somewhere new");

    let stop = StopOnce::new(repo.path(), Trigger::InIndex(PathBuf::from("a.txt")))
        .then_move("refs/heads/main", &elsewhere);
    let why = execute_under(&repo, NOW + 1, &stop).expect_err("the undo is cancelled");
    assert!(stop.fired(), "the test's own write ran");
    assert_eq!(
        repo.git(&["rev-parse", "refs/heads/main"]),
        elsewhere,
        "main is where the test moved it, not where the undo left it"
    );
    PartWay {
        repo,
        before,
        elsewhere,
        why,
    }
}

#[test]
fn rollback_refused_by_a_moved_ref_is_reported_not_hidden() {
    let PartWay { repo, why, .. } = cancel_part_way();

    let RestoreError::CancelledPartway { applied, rollback } = &why else {
        panic!(
            "expected a part-way cancel, got: {}",
            why.explain(&crate::support::Letters)
        );
    };
    assert!(
        matches!(rollback.as_ref(), RestoreError::RefMoved { name } if name == "refs/heads/main"),
        "the roll-back names the ref it refused over: {rollback}"
    );
    assert_eq!(
        applied,
        &vec!["refs/heads/main".to_owned()],
        "only the ref is still in force — the index step was reversed before \
         the roll-back reached it"
    );
    // And it reversed what it could: the paths the index step had staged are
    // gone from the index again, and the working-tree step never ran.
    assert_eq!(
        repo.git(&["ls-files", "--", "a.txt", "b.txt"]),
        "",
        "the index step was reversed even though the ref step could not be"
    );
    assert!(
        !repo.exists("a.txt") && !repo.exists("b.txt"),
        "the working-tree step never started"
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

/// The other half of the same requirement, and the promise behind it: after a
/// part-way cancel the user gets everything back, and the key the message
/// names is the key that does it.
#[test]
fn after_a_part_way_cancel_the_named_key_restores_everything() {
    let PartWay {
        repo,
        before,
        elsewhere,
        why,
    } = cancel_part_way();
    let entries = entries(&repo);
    let last = entries.last().expect("the cancelled undo is journaled");

    // §6.7's rail, asked of the half-cancelled entry.
    //
    // The ticket asked for "drift after reports it"; the repository says
    // otherwise, and the repository is right. `finish` reads the after-state
    // *fresh*, after the cancel, so the entry records where things actually
    // ended up — main at `elsewhere` included. Drift compares that record
    // with now, and nothing has moved since. The rail has nothing to warn
    // about because the entry is not lying, which is the property that
    // matters: a warning here would be noise, and, better, it means `U` needs
    // no `force` to get the user's work back (below).
    let drifted = {
        let handle = repo.open();
        let mut journal = journal_for(&handle);
        let never = Never;
        let ctx = context(&handle, &mut journal, NOW + 2, &never);
        let names: Vec<String> = last.refs_after.keys().cloned().collect();
        let now = facts::read_state_now(ctx.repo, &ctx.scratch, &names, &never)
            .expect("the repository reads");
        undo::drift(last, &now)
    };
    assert!(
        drifted.is_clean(),
        "the entry recorded the half-way state it really left, so nothing has drifted from it: {}",
        drifted.explain()
    );
    assert_eq!(
        last.refs_after.get("refs/heads/main").copied().flatten(),
        Some(Oid::from_hex(&elsewhere).expect("git prints an object id")),
        "and what it recorded is the ref as the test left it, not as the undo meant to leave it"
    );

    // Recovery is `U`, not `u`: the new entry is an in-force *Undo* entry,
    // and `next_undo` steps over those. `u` would reach an older operation
    // or nothing at all — which is why the part-way message names `U`.
    assert_eq!(
        next_redo(&entries).map(|entry| entry.id.clone()),
        Some(last.id.clone()),
        "U reaches the cancelled undo"
    );
    assert_ne!(
        next_undo(&entries).map(|entry| entry.id.clone()),
        Some(last.id.clone()),
        "u never reaches it"
    );
    assert!(
        why.explain(&crate::support::Letters)
            .contains("U restores the backup"),
        "the user is pointed at the key that works: {}",
        why.explain(&crate::support::Letters)
    );

    // And following that instruction gets everything back — with `force`
    // false, so this is the one keystroke the message promises and not a
    // keystroke plus a drift prompt.
    {
        let handle = repo.open();
        let mut journal = journal_for(&handle);
        let never = Never;
        let mut ctx = context(&handle, &mut journal, NOW + 2, &never);
        undo::redo(&mut ctx, false).expect("U puts the backup back");
    }
    assert_eq!(
        state_of(&repo),
        before,
        "the repository is back where the cancelled undo found it — nothing was lost"
    );
}

#[test]
fn a_killed_step_whose_ref_is_back_where_it_started_is_not_reversed() {
    // The one test that observes the *read* rather than the compare-and-swap.
    //
    // The ref step's `update-ref` is held inside its transaction — `main`
    // locked, not yet written — until the stop kills it, so the step comes
    // back `Stopped` and whether it landed is not knowable from the error.
    // The repository says, truthfully, "that step did not land": there is
    // nothing to reverse, nothing is written, and the user is told the undo
    // was cancelled and nothing changed.
    //
    // A guess ("a killed step is an applied step") gets this wrong in a way
    // the compare-and-swap cannot hide: it would swap `main` against the value
    // the step would have written, git would refuse, and the user would be
    // told a ref moved when nothing had. So `settled` returning `Applied`
    // unconditionally fails this test — which is what makes it evidence for
    // "read the repository, never guess" and not merely for the
    // compare-and-swap.
    //
    // Until 27b the stop keyed on `main` having been written, and the step
    // still came back `Stopped` because the runner answered a stop polled
    // after git's exit with `Stopped`. It no longer does — that step
    // finished — so the kill has to land while git is still running.
    let repo = reset_repo();
    let before = reset_and_state(&repo);
    // Where the restore finds `main`, i.e. the entry's own `refs_before`.
    let found_at = repo.git(&["rev-parse", "refs/heads/main"]);
    let undone = first_ref_step_target(&repo);
    assert_ne!(found_at, undone, "the ref step has somewhere to move main");

    let held = hold_main_update_until_killed(&repo, "prepared");
    let stop = StopOnce::new(repo.path(), Trigger::PathExists(held));
    let why = execute_under(&repo, NOW + 1, &stop).expect_err("the undo is cancelled");
    assert!(stop.fired(), "the stop fired inside the ref step");

    match &why {
        RestoreError::Cancelled { rolled_back } => assert_eq!(
            *rolled_back, 0,
            "the repository says the killed step did not land, so there is nothing to put back"
        ),
        other => panic!(
            "expected a clean cancel with nothing to reverse, got: {}",
            other.explain(&crate::support::Letters)
        ),
    }
    assert_eq!(
        repo.git(&["rev-parse", "refs/heads/main"]),
        found_at,
        "the roll-back wrote nothing: main is exactly where it was read to be"
    );
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
}

#[test]
fn a_ref_moved_under_a_killed_step_is_reported_not_overwritten() {
    // The other half of the moved-ref story: the stop fires the moment the
    // ref step's own `update-ref` has written, while git is held inside that
    // step, so the runner SIGTERMs git and the step comes back `Stopped`
    // (until 27b the "inside" was the runner calling a stop polled after
    // git's exit `Stopped`; it now reports that step done). Whether it
    // landed is then not knowable from the error — and the ref has been moved
    // away as well, so reading it says neither "landed" nor "did not". That
    // is reported, never overwritten.
    //
    // This one is about what the roll-back *does not write*. It cannot tell a
    // read from a compare-and-swap — both refuse here — which is why the test
    // above exists.
    let repo = reset_repo();
    let elsewhere = repo.git(&["rev-parse", "HEAD~1"]);
    reset_and_state(&repo);
    let undone = first_ref_step_target(&repo);
    assert_ne!(elsewhere, undone, "the test moves main somewhere new");

    let held = hold_main_update_until_killed(&repo, "committed");
    let stop = StopOnce::new(repo.path(), Trigger::PathExists(held))
        .then_move("refs/heads/main", &elsewhere);
    let why = execute_under(&repo, NOW + 1, &stop).expect_err("the undo is cancelled");
    assert!(stop.fired(), "the test's own write ran");

    let RestoreError::CancelledPartway { applied, rollback } = &why else {
        panic!(
            "expected a part-way cancel, got: {}",
            why.explain(&crate::support::Letters)
        );
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
    // on its own.
    //
    // The stop is keyed to the plan's own effect, not to a poll count. A
    // count calibrated on a throwaway repository is a clock that runs at the
    // machine's speed — the runner polls once per pipe event and once per
    // millisecond of `try_wait` — so "the n-th poll" is a different place
    // under load than it was when the count was taken, and this test used to
    // fail roughly one full-suite run in four because of it. "Once the branch
    // ref stands where the undo puts it" is the same place every time, and it
    // is past §6.7's rails by construction: the rails only read.
    let repo = reset_repo();
    let before = reset_and_state(&repo);
    let next_undo_before = undo_plan(&repo).reverses;
    let undone = first_ref_step_target(&repo);

    let stop = StopOnce::new(
        repo.path(),
        Trigger::RefIs {
            name: "refs/heads/main".to_owned(),
            at: undone,
        },
    );
    let why = {
        let handle = repo.open();
        let mut journal = journal_for(&handle);
        let mut ctx = context(&handle, &mut journal, NOW + 1, &stop);
        undo::undo(&mut ctx, false).expect_err("the undo is cancelled")
    };
    assert!(stop.fired(), "the stop fired on the ref step's own effect");

    // A stop that lands in the rails, before `begin`, would prove nothing
    // about the roll-back, so it is not accepted here: the cancel has to be
    // one that had already applied a step and put it back.
    match &why {
        UndoError::Restore(RestoreError::Cancelled { rolled_back }) => assert!(
            *rolled_back >= 1,
            "at least the ref step was put back, not a cancel before anything ran"
        ),
        other => panic!(
            "a cancel with a roll-back, not: {}",
            other.explain(&crate::support::Letters)
        ),
    }
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

/// `KNOWN_GAPS` :495, the planner's ruling (m7/01b): a stop leaves a
/// restore, and an undo, as `Cancelled` or `CancelledPartway` — never any
/// other shape — at **every** poll, the ones before the first step included.
/// A stop before any step is `Cancelled { rolled_back: 0 }` and changes
/// nothing. Stride 1, so heavy tier (PROMPT.md 7.1: ~110 s);
/// [`every_stop_is_one_shape_sampled`] is the fast tier's cut of it.
#[cfg(feature = "heavy-tests")]
#[test]
fn every_stop_is_one_shape() {
    for through_undo in [false, true] {
        stop_shape_sweep(through_undo, |counted| (1..=counted).collect());
    }
}

/// The fast tier's [`every_stop_is_one_shape`]: the first poll (before
/// anything, on both paths), the last, and one in the middle of the steps.
#[test]
fn every_stop_is_one_shape_sampled() {
    stop_shape_sweep(false, |counted| vec![1, counted / 2, counted]);
    stop_shape_sweep(true, |counted| vec![1, counted]);
}

/// Stop the undo at each poll `pick` names out of the ones an unstopped run
/// makes, and check the shape each stop leaves as.
fn stop_shape_sweep(through_undo: bool, pick: impl Fn(usize) -> Vec<usize>) {
    let counted = {
        let repo = reset_repo();
        reset_and_state(&repo);
        let counter = CountPolls::new();
        stopped_at(&repo, &counter, through_undo).expect("an unstopped undo succeeds");
        counter.count()
    };
    let mut before_any_step = 0;
    for at in pick(counted) {
        let repo = reset_repo();
        let before = reset_and_state(&repo);
        let stop = StopAtPoll::new(at);
        // Finished before the stop fired: the "or finished" half.
        let Err(why) = stopped_at(&repo, &stop, through_undo) else {
            continue;
        };
        let rolled_back = match &why {
            RestoreError::Cancelled { rolled_back } => *rolled_back,
            RestoreError::CancelledPartway { .. } => continue,
            other => {
                panic!("poll {at} (through undo: {through_undo}) left as another shape: {other:?}")
            }
        };
        if rolled_back == 0 {
            before_any_step += 1;
            assert_eq!(state_of(&repo), before, "poll {at} changed the repository");
        }
    }
    assert!(
        before_any_step > 0,
        "the sweep reached a stop before any step (through undo: {through_undo})"
    );
}

/// The undo of `reset_and_state`, under `stop`: straight at `execute`, or
/// through `undo::undo` (its rails poll the same stop).
fn stopped_at(
    repo: &TempRepo,
    stop: &dyn Stop,
    through_undo: bool,
) -> Result<Applied, RestoreError> {
    if !through_undo {
        return execute_under(repo, NOW + 1, stop);
    }
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let mut ctx = context(&handle, &mut journal, NOW + 1, stop);
    undo::undo(&mut ctx, false).map_err(|why| match why {
        UndoError::Restore(why) => why,
        other => panic!("a stopped undo left as {other:?}"),
    })
}
