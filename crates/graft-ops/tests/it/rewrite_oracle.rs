//! The rewrite oracle (ADR-0047): each row runs Graft's plan and apply on a
//! repository, and a hand-written `git rebase -i` todo on a `cp -R` copy of
//! it — what he would have typed in the shell for the same decisions. The
//! two must agree on every branch's commits (tree and message, oldest
//! first), `HEAD`'s symbolic ref, and `git status --porcelain=v2`.
//!
//! Commit ids are not compared: the two routes commit at different moments,
//! so the committer dates differ. A row where they disagree is a failure.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::process::Command;

use graft_core::oid::Oid;
use graft_ops::apply::{ApplyError, Resume, resume};
use graft_ops::facts::read_facts;
use graft_ops::operation::{FactsScope, Operation};
use graft_ops::rewrite::{RewriteParams, RewriteRequest, TodoAction, plan};

use crate::support::{NOW, Never, TempRepo, context, git_version, journal_for, try_run};

// ── The two routes ───────────────────────────────────────────────────────

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a commit id")
}

/// One commit changing its own file, so no two commits conflict.
fn commit(repo: &TempRepo, name: &str) {
    repo.write(&format!("{name}.txt"), format!("{name}\n").as_bytes());
    repo.commit_all(name);
}

/// `main` with `m1` (and a tracked `notes.txt`); `feature` with `f1`..`f4`
/// on it, `HEAD` on `feature`.
fn four() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("notes.txt", b"notes\n");
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    for name in ["f1", "f2", "f3", "f4"] {
        commit(&repo, name);
    }
    repo
}

/// `feature-a` (`a1`, `a2`) off `main`, `feature-b` (`b1`) on it, and
/// `feature-c` (`c1`) on that when `three`. `HEAD` on `feature-a`.
fn line(three: bool) -> TempRepo {
    let repo = TempRepo::init();
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    commit(&repo, "a1");
    commit(&repo, "a2");
    repo.git(&["switch", "-q", "-c", "feature-b"]);
    commit(&repo, "b1");
    if three {
        repo.git(&["switch", "-q", "-c", "feature-c"]);
        commit(&repo, "c1");
    }
    repo.git(&["switch", "-q", "feature-a"]);
    repo
}

/// Graft's plan for `request`, as the app would ask for it.
fn planned(repo: &TempRepo, request: &RewriteRequest) -> RewriteParams {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    plan(&handle, &git_version(), request, &facts, None, &Never)
        .unwrap_or_else(|why| panic!("it plans: {why:?}"))
}

/// `R` on `from`.
fn open(repo: &TempRepo, from: &str) -> RewriteParams {
    planned(
        repo,
        &RewriteRequest::Open {
            from: id(repo, from),
        },
    )
}

/// The row whose commit has this subject.
fn row(params: &RewriteParams, subject: &str) -> usize {
    params
        .todo
        .iter()
        .position(|line| line.subject == subject)
        .unwrap_or_else(|| panic!("no row {subject} in {:?}", params.todo))
}

fn set(params: &mut RewriteParams, subject: &str, action: TodoAction) {
    let at = row(params, subject);
    params.todo[at].action = action;
}

/// Graft's route to the end: apply, and — when it stops on an `edit` —
/// `between` at the stop, then resume, until the entry closes.
fn graft(repo: &TempRepo, params: RewriteParams, between: &dyn Fn(&TempRepo)) {
    let mut result = try_run(repo, &Operation::Rewrite(params), NOW);
    let mut at = NOW;
    loop {
        match result {
            Ok(_) => return,
            Err(ApplyError::Conflicted { paths, .. }) if paths.is_empty() => {
                between(repo);
                at += 1;
                let handle = repo.open();
                let mut journal = journal_for(&handle);
                let never = Never;
                let mut ctx = context(&handle, &mut journal, at, &never);
                result = resume(&mut ctx, Resume::Continue);
            }
            Err(why) => panic!("Graft's route failed: {}", why.explain()),
        }
    }
}

/// What he would type: `git rebase -i <onto>` with `todo` as the todo, and
/// `message` answering every message git asks for (`GIT_EDITOR=true` when
/// none). `between` runs at each `edit` stop, then `git rebase --continue`.
fn by_hand(
    repo: &TempRepo,
    config: &[&str],
    onto: &str,
    todo: &str,
    message: Option<&str>,
    between: &dyn Fn(&TempRepo),
) {
    let todo_file = repo.path().join(".git/hand-todo");
    std::fs::write(&todo_file, todo).expect("the todo writes");
    let editor = match message {
        Some(message) => {
            let file = repo.path().join(".git/hand-message");
            std::fs::write(&file, message).expect("the message writes");
            format!("cp '{}'", file.display())
        }
        None => "true".to_owned(),
    };
    let run = |args: &[&str]| {
        let mut git = Command::new("git");
        git.arg("-C").arg(repo.path());
        for pair in config {
            git.args(["-c", pair]);
        }
        let output = git
            .args(args)
            .env(
                "GIT_SEQUENCE_EDITOR",
                format!("cp '{}'", todo_file.display()),
            )
            .env("GIT_EDITOR", &editor)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?} by hand: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    run(&["rebase", "-i", onto]);
    while repo.exists(".git/rebase-merge") {
        between(repo);
        run(&["rebase", "--continue"]);
    }
}

fn nothing(_: &TempRepo) {}

/// Everything the two routes must agree on.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    /// Per branch: each commit's tree and message, oldest first.
    branches: Vec<(String, String)>,
    head: String,
    status: String,
}

fn outcome(repo: &TempRepo) -> Outcome {
    let names = repo.git(&["for-each-ref", "--format=%(refname)", "refs/heads"]);
    Outcome {
        branches: names
            .lines()
            .map(|name| {
                let log = repo.git(&["log", "--reverse", "--format=%T %B%x1e", name]);
                (name.to_owned(), log)
            })
            .collect(),
        head: repo
            .try_git(&["symbolic-ref", "-q", "HEAD"])
            .unwrap_or_else(|_| "detached".to_owned()),
        status: repo.git(&["status", "--porcelain=v2"]),
    }
}

/// Both routes agree, and the rewrite changed something (a row that moved
/// nothing would agree trivially).
fn agree(graft_side: &TempRepo, hand_side: &TempRepo, before: &Outcome) {
    let graft = outcome(graft_side);
    assert_eq!(
        graft,
        outcome(hand_side),
        "Graft's route against the hand's"
    );
    assert_ne!(&graft, before, "the rewrite changed nothing");
}

/// A commit's full id, as a todo line names it.
fn full(repo: &TempRepo, rev: &str) -> String {
    repo.git(&["rev-parse", rev])
}

// ── The rows ─────────────────────────────────────────────────────────────

#[test]
fn squash_three() {
    let repo = four();
    let hand = repo.copy();
    let before = outcome(&repo);
    let message = "f1, f2 and f3 together\n\nOne change.";
    let mut params = open(&repo, "feature~3");
    set(&mut params, "f2", TodoAction::Squash);
    set(&mut params, "f3", TodoAction::Squash);
    params
        .messages
        .insert(row(&params, "f1"), message.to_owned());

    graft(&repo, params, &nothing);
    let todo = format!(
        "pick {}\nsquash {}\nsquash {}\npick {}\n",
        full(&hand, "feature~3"),
        full(&hand, "feature~2"),
        full(&hand, "feature~1"),
        full(&hand, "feature"),
    );
    by_hand(&hand, &[], "main", &todo, Some(message), &nothing);

    agree(&repo, &hand, &before);
}

#[test]
fn fixup() {
    let repo = four();
    let hand = repo.copy();
    let before = outcome(&repo);
    let mut params = open(&repo, "feature~3");
    set(&mut params, "f3", TodoAction::Fixup);

    graft(&repo, params, &nothing);
    let todo = format!(
        "pick {}\npick {}\nfixup {}\npick {}\n",
        full(&hand, "feature~3"),
        full(&hand, "feature~2"),
        full(&hand, "feature~1"),
        full(&hand, "feature"),
    );
    by_hand(&hand, &[], "main", &todo, None, &nothing);

    agree(&repo, &hand, &before);
}

#[test]
fn reword_keeps_a_hash_line() {
    // `#123 fixes login`: kept as written on both routes. By hand that takes
    // `commit.cleanup=whitespace`, which is what Graft's route sets.
    let repo = four();
    let hand = repo.copy();
    let before = outcome(&repo);
    let message = "#123 fixes login\n\nThe token was checked after expiry.";
    let mut params = open(&repo, "feature~3");
    set(&mut params, "f2", TodoAction::Reword);
    params
        .messages
        .insert(row(&params, "f2"), message.to_owned());

    graft(&repo, params, &nothing);
    let todo = format!(
        "pick {}\nreword {}\npick {}\npick {}\n",
        full(&hand, "feature~3"),
        full(&hand, "feature~2"),
        full(&hand, "feature~1"),
        full(&hand, "feature"),
    );
    by_hand(
        &hand,
        &["commit.cleanup=whitespace"],
        "main",
        &todo,
        Some(message),
        &nothing,
    );

    agree(&repo, &hand, &before);
    assert!(
        repo.git(&["log", "-1", "--format=%B", "feature~2"])
            .starts_with("#123 fixes login")
    );
}

#[test]
fn reorder() {
    let repo = four();
    let hand = repo.copy();
    let before = outcome(&repo);
    let mut params = open(&repo, "feature~3");
    params.todo.swap(0, 1);

    graft(&repo, params, &nothing);
    let todo = format!(
        "pick {}\npick {}\npick {}\npick {}\n",
        full(&hand, "feature~2"),
        full(&hand, "feature~3"),
        full(&hand, "feature~1"),
        full(&hand, "feature"),
    );
    by_hand(&hand, &[], "main", &todo, None, &nothing);

    agree(&repo, &hand, &before);
}

#[test]
fn drop() {
    let repo = four();
    let hand = repo.copy();
    let before = outcome(&repo);
    let mut params = open(&repo, "feature~3");
    set(&mut params, "f3", TodoAction::Drop);

    graft(&repo, params, &nothing);
    let todo = format!(
        "pick {}\npick {}\ndrop {}\npick {}\n",
        full(&hand, "feature~3"),
        full(&hand, "feature~2"),
        full(&hand, "feature~1"),
        full(&hand, "feature"),
    );
    by_hand(&hand, &[], "main", &todo, None, &nothing);

    agree(&repo, &hand, &before);
}

/// At an `edit` stop on `f2`: change its file and amend.
fn amend_f2(repo: &TempRepo) {
    repo.write("f2.txt", b"f2, amended at the stop\n");
    repo.git(&["commit", "-q", "--amend", "--no-edit", "-a"]);
}

#[test]
fn edit_then_continue() {
    let repo = four();
    let hand = repo.copy();
    let before = outcome(&repo);
    let mut params = open(&repo, "feature~3");
    set(&mut params, "f2", TodoAction::Edit);

    graft(&repo, params, &amend_f2);
    let todo = format!(
        "pick {}\nedit {}\npick {}\npick {}\n",
        full(&hand, "feature~3"),
        full(&hand, "feature~2"),
        full(&hand, "feature~1"),
        full(&hand, "feature"),
    );
    by_hand(&hand, &[], "main", &todo, None, &amend_f2);

    agree(&repo, &hand, &before);
}

#[test]
fn a_squash_group_after_an_edit() {
    // The squash's message is asked for after the resume: the saved plan's
    // helper answers it, not `GIT_EDITOR=true`.
    let repo = four();
    let hand = repo.copy();
    let before = outcome(&repo);
    let message = "f2 and f3, after the edit";
    let mut params = open(&repo, "feature~3");
    set(&mut params, "f2", TodoAction::Edit);
    set(&mut params, "f3", TodoAction::Squash);
    params
        .messages
        .insert(row(&params, "f2"), message.to_owned());

    graft(&repo, params, &amend_f2);
    let todo = format!(
        "pick {}\nedit {}\nsquash {}\npick {}\n",
        full(&hand, "feature~3"),
        full(&hand, "feature~2"),
        full(&hand, "feature~1"),
        full(&hand, "feature"),
    );
    by_hand(&hand, &[], "main", &todo, Some(message), &amend_f2);

    agree(&repo, &hand, &before);
}

#[test]
fn a_three_branch_line() {
    // Every branch moved, and `HEAD` back on the bottom branch.
    let repo = line(true);
    let hand = repo.copy();
    let before = outcome(&repo);
    let message = "a1, reworded";
    let mut params = open(&repo, "feature-a~1");
    assert_eq!(params.line.branches.len(), 3);
    set(&mut params, "a1", TodoAction::Reword);
    params
        .messages
        .insert(row(&params, "a1"), message.to_owned());

    graft(&repo, params, &nothing);
    let todo = format!(
        "reword {}\npick {}\nupdate-ref refs/heads/feature-a\npick {}\n\
         update-ref refs/heads/feature-b\npick {}\n",
        full(&hand, "feature-a~1"),
        full(&hand, "feature-a"),
        full(&hand, "feature-b"),
        full(&hand, "feature-c"),
    );
    hand.git(&["switch", "-q", "feature-c"]);
    by_hand(&hand, &[], "main", &todo, Some(message), &nothing);
    hand.git(&["switch", "-q", "feature-a"]);

    agree(&repo, &hand, &before);
    assert_eq!(outcome(&repo).head, "refs/heads/feature-a");
}

#[test]
fn amend_into_three_back() {
    let repo = four();
    repo.write("f1.txt", b"f1, with the staged change\n");
    repo.git(&["add", "f1.txt"]);
    let hand = repo.copy();
    let before = outcome(&repo);
    let params = planned(
        &repo,
        &RewriteRequest::AmendInto {
            target: id(&repo, "feature~3"),
        },
    );

    graft(&repo, params, &nothing);
    hand.git(&["commit", "-q", "-m", "fixup! f1"]);
    let todo = format!(
        "pick {}\nfixup {}\npick {}\npick {}\npick {}\n",
        full(&hand, "feature~4"),
        full(&hand, "feature"),
        full(&hand, "feature~3"),
        full(&hand, "feature~2"),
        full(&hand, "feature~1"),
    );
    by_hand(&hand, &[], "main", &todo, None, &nothing);

    agree(&repo, &hand, &before);
}

#[test]
fn amend_into_head_with_a_dependent() {
    let repo = line(false);
    repo.write("a2.txt", b"a2, with the staged change\n");
    repo.git(&["add", "a2.txt"]);
    let hand = repo.copy();
    let before = outcome(&repo);
    let params = planned(
        &repo,
        &RewriteRequest::AmendInto {
            target: id(&repo, "feature-a"),
        },
    );

    graft(&repo, params, &nothing);
    // By hand: the staged change rides to the top of the line, is committed
    // there, and the todo moves it down under `a2`.
    hand.git(&["switch", "-q", "feature-b"]);
    hand.git(&["commit", "-q", "-m", "fixup! a2"]);
    let todo = format!(
        "pick {}\npick {}\nfixup {}\nupdate-ref refs/heads/feature-a\npick {}\n",
        full(&hand, "feature-a~1"),
        full(&hand, "feature-a"),
        full(&hand, "feature-b"),
        full(&hand, "feature-b~1"),
    );
    by_hand(&hand, &[], "main", &todo, None, &nothing);
    hand.git(&["switch", "-q", "feature-a"]);

    agree(&repo, &hand, &before);
}

#[test]
fn an_unsaved_edit_is_set_aside_and_put_back() {
    // An unsaved edit to a file the rewrite does not touch: Graft sets it
    // aside and puts it back (ADR-0045); by hand, `--autostash`.
    let repo = four();
    repo.write("notes.txt", b"notes, not saved\n");
    let hand = repo.copy();
    let before = outcome(&repo);
    let mut params = open(&repo, "feature~3");
    set(&mut params, "f3", TodoAction::Drop);

    graft(&repo, params, &nothing);
    let todo = format!(
        "pick {}\npick {}\ndrop {}\npick {}\n",
        full(&hand, "feature~3"),
        full(&hand, "feature~2"),
        full(&hand, "feature~1"),
        full(&hand, "feature"),
    );
    by_hand(
        &hand,
        &["rebase.autoStash=true"],
        "main",
        &todo,
        None,
        &nothing,
    );

    agree(&repo, &hand, &before);
    assert_eq!(repo.read("notes.txt"), b"notes, not saved\n");
}

// ── Forks that follow (ADR-0059) ─────────────────────────────────────────

/// The params as `y` runs them: the preview's `plan_as_run`, which fixes
/// the forks' fates and writes the followers' sections in.
fn as_run(repo: &TempRepo, params: RewriteParams) -> RewriteParams {
    let handle = repo.open();
    let op = Operation::Rewrite(params);
    let facts =
        read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("the facts read");
    match graft_ops::preview::plan_as_run(&handle, &git_version(), op, &facts, &Never) {
        Ok((Operation::Rewrite(params), _)) => params,
        Ok((other, _)) => panic!("a rewrite came back as {other:?}"),
        Err(why) => panic!("it previews: {why:?}"),
    }
}

/// `feature` (`f1`..`f4`) with `feature-x` forked off `f2` (`x1`, `x2`) and
/// `feature-x-wip` at `x1`: reword `f1`, and the fork follows in the same
/// rebase — its own commits replayed onto the new `f2`, the branch on it
/// moved too. By hand: plain `rebase -i` (no `--rebase-merges`) on the
/// same todo, `label`/`reset` and all (ADR-0059 §3's "verify against git").
#[test]
fn a_fork_follows_in_the_same_rebase() {
    let repo = four();
    repo.git(&["switch", "-q", "-c", "feature-x", "feature~2"]);
    commit(&repo, "x1");
    repo.git(&["branch", "feature-x-wip"]);
    commit(&repo, "x2");
    repo.git(&["switch", "-q", "feature"]);
    let hand = repo.copy();
    let before = outcome(&repo);
    let message = "f1, reworded\n";
    let mut params = open(&repo, "feature~3");
    set(&mut params, "f1", TodoAction::Reword);
    params
        .messages
        .insert(row(&params, "f1"), message.to_owned());
    let params = as_run(&repo, params);
    assert_eq!(params.forks.len(), 1, "{:?}", params.forks);

    graft(&repo, params, &nothing);
    let todo = format!(
        "reword {}\npick {}\nlabel graft-fork-1\npick {}\nupdate-ref refs/heads/feature-x-wip\n\
         pick {}\nupdate-ref refs/heads/feature-x\nreset graft-fork-1\npick {}\npick {}\n",
        full(&hand, "feature~3"),
        full(&hand, "feature~2"),
        full(&hand, "feature-x~1"),
        full(&hand, "feature-x"),
        full(&hand, "feature~1"),
        full(&hand, "feature"),
    );
    by_hand(
        &hand,
        &["rebase.updateRefs=false", "commit.cleanup=whitespace"],
        "main",
        &todo,
        Some(message),
        &nothing,
    );

    agree(&repo, &hand, &before);
    for side in [&repo, &hand] {
        assert_eq!(
            side.git(&["rev-parse", "feature-x~2"]),
            side.git(&["rev-parse", "feature~2"]),
            "feature-x left the new f2"
        );
        assert_eq!(
            side.git(&["rev-parse", "feature-x-wip"]),
            side.git(&["rev-parse", "feature-x~1"])
        );
    }
}
