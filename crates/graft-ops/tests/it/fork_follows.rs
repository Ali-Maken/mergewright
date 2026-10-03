//! A branch forked off a rewritten commit follows when its replay is clean
//! (ADR-0059), against real repositories. Truth is read with git
//! (`rev-parse`, `diff`, `for-each-ref`), never with Graft's own reads; the
//! preview's fates are checked only where the ticket names one.
//!
//! Whether Graft's todo does what a hand-run `git rebase -i` does is
//! `rewrite_oracle::a_fork_follows_in_the_same_rebase`'s.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use graft_core::journal::EntryStatus;
use graft_core::oid::Oid;
use graft_git::version::GitVersion;
use graft_ops::apply::{Applied, ApplyError, Resume, recover_interrupted, resume};
use graft_ops::facts::read_facts;
use graft_ops::operation::{FactsScope, Operation, Refusal};
use graft_ops::preview::{ForkFate, PreviewPlan, WhyStays, plan_as_run};
use graft_ops::rewrite::{RewriteError, RewriteParams, RewriteRequest, TodoAction, plan, progress};

use crate::support::{
    NOW, Never, TempRepo, context, git_version, journal_for, run, run_undo, state_of, try_run,
};

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a commit id")
}

/// One commit changing its own file, so no two commits conflict.
fn commit(repo: &TempRepo, name: &str) {
    repo.write(&format!("{name}.txt"), format!("{name}\n").as_bytes());
    repo.commit_all(name);
}

fn head(repo: &TempRepo) -> String {
    repo.try_git(&["symbolic-ref", "-q", "HEAD"])
        .unwrap_or_else(|_| "detached".to_owned())
}

fn heads(repo: &TempRepo) -> String {
    repo.git(&[
        "for-each-ref",
        "--format=%(refname) %(objectname)",
        "refs/heads",
    ])
}

/// `git diff <rev>~1 <rev>`: a commit's own change.
fn own_diff(repo: &TempRepo, rev: &str) -> String {
    repo.git(&["diff", &format!("{rev}~1"), rev])
}

/// `main` m1; `feature-a` c1 c2 c3 off it (`HEAD`), `shared.txt` made in
/// c1; `feature-x` x1 off c2, its own file.
fn forked() -> TempRepo {
    let repo = TempRepo::init();
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    repo.write("shared.txt", b"base\n");
    commit(&repo, "c1");
    commit(&repo, "c2");
    commit(&repo, "c3");
    repo.git(&["switch", "-q", "-c", "feature-x", "feature-a~1"]);
    commit(&repo, "x1");
    repo.git(&["switch", "-q", "feature-a"]);
    repo
}

fn planned(repo: &TempRepo, request: &RewriteRequest) -> Result<RewriteParams, RewriteError> {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    plan(&handle, &git_version(), request, &facts, None, &Never)
}

/// What `y` runs: the preview's `plan_as_run`, which fixes the forks' fates.
fn as_run_with(
    repo: &TempRepo,
    params: RewriteParams,
    git: &GitVersion,
) -> (Operation, PreviewPlan) {
    let handle = repo.open();
    let op = Operation::Rewrite(params);
    let facts = read_facts(&handle, *git, &Never, &op.facts_scope()).expect("the facts read");
    plan_as_run(&handle, git, op, &facts, &Never).unwrap_or_else(|why| panic!("it plans: {why:?}"))
}

fn as_run(repo: &TempRepo, params: RewriteParams) -> (Operation, PreviewPlan) {
    as_run_with(repo, params, &git_version())
}

/// `a` of what is staged into `rev`, as `y` runs it.
fn amend_into(repo: &TempRepo, rev: &str) -> (Operation, PreviewPlan) {
    let params = planned(
        repo,
        &RewriteRequest::AmendInto {
            target: id(repo, rev),
        },
    )
    .unwrap_or_else(|why| panic!("it plans: {why:?}"));
    as_run(repo, params)
}

/// `R` from `feature-a`'s first commit, `c1` reworded, `edit` on the rows
/// whose subject is in `edits`.
fn reword_c1(repo: &TempRepo, edits: &[&str]) -> RewriteParams {
    let first = repo.git(&["rev-list", "--reverse", "main..feature-a"]);
    let first = first.lines().next().expect("feature-a has commits");
    let mut params = planned(
        repo,
        &RewriteRequest::Open {
            from: Oid::from_hex(first).expect("an id"),
        },
    )
    .unwrap_or_else(|why| panic!("it plans: {why:?}"));
    let row = params
        .todo
        .iter()
        .position(|line| line.subject == "c1")
        .expect("c1 is a row");
    params.todo[row].action = TodoAction::Reword;
    params.messages.insert(row, "c1, reworded\n".to_owned());
    for line in &mut params.todo {
        if edits.contains(&line.subject.as_str()) {
            line.action = TodoAction::Edit;
        }
    }
    params
}

/// Every fork the preview names, short name → fate.
fn fates(drawn: &PreviewPlan) -> Vec<(String, ForkFate)> {
    drawn
        .forks
        .iter()
        .map(|fork| {
            let name = String::from_utf8_lossy(fork.branch.as_bytes());
            (
                name.trim_start_matches("refs/heads/").to_owned(),
                fork.fate.clone(),
            )
        })
        .collect()
}

fn fate_of(drawn: &PreviewPlan, branch: &str) -> ForkFate {
    fates(drawn)
        .into_iter()
        .find(|(name, _)| name == branch)
        .map_or_else(
            || panic!("{branch} is not a named fork: {:?}", fates(drawn)),
            |(_, fate)| fate,
        )
}

fn stays(fate: &ForkFate) -> &WhyStays {
    match fate {
        ForkFate::Stays(why) => why,
        ForkFate::Follows { .. } => panic!("it follows"),
    }
}

/// Apply `op`, which must stop: the entry it left.
fn stopped(repo: &TempRepo, op: &Operation) {
    match try_run(repo, op, NOW) {
        Err(ApplyError::Conflicted { entry, .. }) => {
            assert_eq!(entry.status, EntryStatus::Pending);
        }
        Err(other) => panic!("expected a stop, got {}", other.explain()),
        Ok(applied) => panic!("expected a stop, it finished: {:?}", applied.entry.status),
    }
}

/// A resume with a fresh context: the same as after a relaunch.
fn resume_with(repo: &TempRepo, how: Resume) -> Result<Applied, ApplyError> {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    resume(&mut ctx, how)
}

// ── Acceptance 1, 2: a fork follows when clean, stays when not ──────────

#[test]
fn a_clean_fork_follows_an_amend_into() {
    let repo = forked();
    let x1 = own_diff(&repo, "feature-x");
    repo.write("c1.txt", b"c1, amended\n");
    repo.git(&["add", "c1.txt"]);

    let (op, drawn) = amend_into(&repo, "feature-a~2");
    assert!(
        matches!(fate_of(&drawn, "feature-x"), ForkFate::Follows { .. }),
        "{:?}",
        fates(&drawn)
    );
    run(&repo, &op, NOW);

    assert_eq!(
        id(&repo, "feature-x~1"),
        id(&repo, "feature-a~1"),
        "feature-x left the new c2"
    );
    assert_eq!(own_diff(&repo, "feature-x"), x1, "x1's change is the same");
    assert_eq!(repo.git(&["log", "-1", "--format=%s", "feature-x"]), "x1");
    assert_eq!(repo.git(&["show", "feature-x:c1.txt"]), "c1, amended");
    assert_eq!(head(&repo), "refs/heads/feature-a", "HEAD came home");
}

#[test]
fn a_conflicting_fork_stays_and_the_line_moves() {
    let repo = forked();
    repo.git(&["switch", "-q", "feature-x"]);
    repo.write("shared.txt", b"x\n");
    repo.commit_all("x2");
    repo.git(&["switch", "-q", "feature-a"]);
    let x2 = id(&repo, "feature-x");
    let fork_was = id(&repo, "feature-x");
    let line_was = id(&repo, "feature-a");
    repo.write("shared.txt", b"amended\n");
    repo.git(&["add", "shared.txt"]);

    let (op, drawn) = amend_into(&repo, "feature-a~2");
    match stays(&fate_of(&drawn, "feature-x")) {
        WhyStays::WouldConflict { commit, paths } => {
            assert_eq!(*commit, x2, "its own commit that edits the line");
            assert_eq!(paths, &[PathBuf::from("shared.txt")]);
        }
        other => panic!("expected WouldConflict, got {other:?}"),
    }
    run(&repo, &op, NOW);

    assert_eq!(id(&repo, "feature-x"), fork_was, "feature-x stayed");
    assert_ne!(id(&repo, "feature-a"), line_was, "the line moved");
    assert_eq!(repo.git(&["show", "feature-a:shared.txt"]), "amended");
}

// ── Acceptance 3: forks with a dependent ────────────────────────────────

/// `forked()` with `feature-b` b1 b2 b3 built on `feature-a`'s tip.
fn with_dependent(repo: &TempRepo) {
    repo.git(&["switch", "-q", "-c", "feature-b", "feature-a"]);
    commit(repo, "b1");
    commit(repo, "b2");
    commit(repo, "b3");
    repo.git(&["switch", "-q", "feature-a"]);
}

/// (a) A branch started from a dependent's middle commit is a second
/// dependent (ADR-0059 §1, §5): `RewordHead` is refused before anything
/// runs, both named — so no preview, and no `Fork`, is ever made.
#[test]
fn a_branch_off_a_dependents_middle_is_refused_as_a_second_dependent() {
    let repo = TempRepo::init();
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    commit(&repo, "c1");
    commit(&repo, "c2");
    commit(&repo, "c3");
    with_dependent(&repo);
    repo.git(&["switch", "-q", "-c", "feature-x", "feature-b~1"]);
    commit(&repo, "x1");
    repo.git(&["switch", "-q", "feature-a"]);
    let before = heads(&repo);

    let refused = planned(
        &repo,
        &RewriteRequest::RewordHead {
            message: "c3, reworded\n".to_owned(),
        },
    );
    match refused {
        Err(RewriteError::Refused(refusal @ Refusal::ForkedDependents { .. })) => {
            let words = refusal.explain();
            assert!(words.contains("feature-b"), "{words}");
            assert!(words.contains("feature-x"), "{words}");
            assert!(words.contains("are both built on yours"), "{words}");
        }
        other => panic!("expected ForkedDependents, got {other:?}"),
    }
    assert_eq!(heads(&repo), before, "no ref moved");
}

/// (b) A dependent and a fork off his own middle commit: both follow.
#[test]
fn a_dependent_and_a_fork_off_his_middle_both_follow() {
    let repo = forked();
    with_dependent(&repo);
    let b_diffs: Vec<String> = ["feature-b~2", "feature-b~1", "feature-b"]
        .iter()
        .map(|rev| own_diff(&repo, rev))
        .collect();
    let x1 = own_diff(&repo, "feature-x");

    let (op, drawn) = as_run(&repo, reword_c1(&repo, &[]));
    assert!(
        matches!(fate_of(&drawn, "feature-x"), ForkFate::Follows { .. }),
        "{:?}",
        fates(&drawn)
    );
    run(&repo, &op, NOW);

    assert_eq!(id(&repo, "feature-x~1"), id(&repo, "feature-a~1"));
    assert_eq!(own_diff(&repo, "feature-x"), x1);
    repo.git(&["merge-base", "--is-ancestor", "feature-a", "feature-b"]);
    assert_eq!(id(&repo, "feature-b~3"), id(&repo, "feature-a"));
    let after: Vec<String> = ["feature-b~2", "feature-b~1", "feature-b"]
        .iter()
        .map(|rev| own_diff(&repo, rev))
        .collect();
    assert_eq!(after, b_diffs, "feature-b's own changes are the same");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s", "feature-a~2"]),
        "c1, reworded"
    );
    assert_eq!(head(&repo), "refs/heads/feature-a", "HEAD came home");
}

// ── Acceptance 4: one `u` puts every ref back ───────────────────────────

#[test]
fn undo_after_a_follow_puts_every_ref_back() {
    let repo = forked();
    let before = state_of(&repo);
    let refs = heads(&repo);

    let (op, _) = as_run(&repo, reword_c1(&repo, &[]));
    run(&repo, &op, NOW);
    assert_ne!(heads(&repo), refs, "the line and the fork moved");
    assert_eq!(
        journal_for(&repo.open()).entries().len(),
        1,
        "the follow is one journal entry"
    );

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{why:?}"));
    assert_eq!(heads(&repo), refs, "every branch, the fork included");
    assert_eq!(state_of(&repo), before);
}

// ── Acceptance 6: an abort after a section leaves the fork ──────────────

#[test]
fn an_abort_at_a_stop_after_a_section_leaves_the_fork() {
    let repo = forked();
    let before = state_of(&repo);
    let fork_was = id(&repo, "feature-x");

    let (op, drawn) = as_run(&repo, reword_c1(&repo, &["c3"]));
    assert!(matches!(
        fate_of(&drawn, "feature-x"),
        ForkFate::Follows { .. }
    ));
    stopped(&repo, &op);
    let done = std::fs::read_to_string(repo.path().join(".git/rebase-merge/done"))
        .expect("git's done file reads");
    assert!(
        done.contains("reset graft-fork-1"),
        "the stop is past the section: {done}"
    );
    assert_eq!(
        id(&repo, "feature-x"),
        fork_was,
        "update-ref waits for the end"
    );

    resume_with(&repo, Resume::Abort).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(id(&repo, "feature-x"), fork_was);
    assert_eq!(state_of(&repo), before);
}

// ── Acceptance 7: progress inside a section ─────────────────────────────

/// Put a `break` after the first pick of the fork section in the todo git
/// is running, and `git rebase --continue` in the shell up to it.
fn break_in_the_section(repo: &TempRepo) {
    let todo_path = repo.path().join(".git/rebase-merge/git-rebase-todo");
    let todo = std::fs::read_to_string(&todo_path).expect("the todo reads");
    let mut out = String::new();
    let mut in_section = false;
    let mut broke = false;
    for line in todo.lines() {
        out.push_str(line);
        out.push('\n');
        if line.starts_with("label graft-fork-") {
            in_section = true;
        } else if in_section && !broke && line.starts_with("pick ") {
            out.push_str("break\n");
            broke = true;
        }
    }
    assert!(broke, "Graft's todo has a section: {todo}");
    std::fs::write(&todo_path, out).expect("the todo writes");
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(["rebase", "--continue"])
        .env("GIT_EDITOR", "true")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Graft's own todo, stopped at an `edit` on c1 before the section, then a
/// `break` put after the section's first pick and run on by hand.
#[test]
fn progress_inside_a_section_names_the_fork() {
    let repo = forked();
    repo.git(&["switch", "-q", "feature-x"]);
    commit(&repo, "x2");
    repo.git(&["switch", "-q", "feature-a"]);
    let (op, _) = as_run(&repo, reword_c1(&repo, &["c1"]));
    stopped(&repo, &op);

    break_in_the_section(&repo);

    let at = progress(&repo.open(), &Never)
        .expect("progress reads")
        .expect("a rebase is under way");
    // Named, and where it leaves the line: c2, the old commit under x1
    // (feature-x has not moved: git writes it when the rebase ends).
    let fork = at.fork.as_ref().expect("inside feature-x's section");
    assert_eq!(fork.name, "feature-x");
    assert_eq!(fork.at, id(&repo, "feature-x~2"), "c2, old");
    let done = std::fs::read_to_string(repo.path().join(".git/rebase-merge/done"))
        .expect("git's done file reads");
    let pick_like = done
        .lines()
        .filter(|line| {
            let word = line.split_whitespace().next().unwrap_or("");
            matches!(
                word,
                "pick"
                    | "p"
                    | "reword"
                    | "r"
                    | "edit"
                    | "e"
                    | "squash"
                    | "s"
                    | "fixup"
                    | "f"
                    | "drop"
                    | "d"
            )
        })
        .count();
    assert_eq!(at.step.0, pick_like, "done: {done}");
}

// ── Acceptance 8: each reason a fork stays, never `Follows` ─────────────

fn the_fate(repo: &TempRepo, branch: &str) -> ForkFate {
    let (_, drawn) = as_run(repo, reword_c1(repo, &[]));
    fate_of(&drawn, branch)
}

#[test]
fn a_pushed_fork_stays() {
    let repo = forked();
    for (key, value) in [
        ("remote.origin.url", "."),
        ("remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"),
        ("branch.feature-x.remote", "origin"),
        ("branch.feature-x.merge", "refs/heads/feature-x"),
    ] {
        repo.git(&["config", key, value]);
    }
    let x1 = id(&repo, "feature-x").to_string();
    repo.git(&["update-ref", "refs/remotes/origin/feature-x", &x1]);
    match stays(&the_fate(&repo, "feature-x")) {
        WhyStays::Pushed { upstream } => {
            assert_eq!(upstream.as_bytes(), b"refs/remotes/origin/feature-x");
        }
        other => panic!("expected Pushed, got {other:?}"),
    }
}

#[test]
fn a_fork_checked_out_elsewhere_stays() {
    let repo = forked();
    repo.git(&["worktree", "add", "-q", "linked", "feature-x"]);
    assert!(
        matches!(
            stays(&the_fate(&repo, "feature-x")),
            WhyStays::Elsewhere { .. }
        ),
        "checked out in linked"
    );
}

#[test]
fn a_fork_with_a_merge_stays() {
    let repo = forked();
    repo.git(&["switch", "-q", "-c", "side", "feature-a~1"]);
    commit(&repo, "s1");
    repo.git(&["switch", "-q", "feature-x"]);
    repo.git(&["merge", "-q", "--no-ff", "-m", "merge side", "side"]);
    repo.git(&["branch", "-q", "-D", "side"]);
    repo.git(&["switch", "-q", "feature-a"]);
    let merge = id(&repo, "feature-x");
    match stays(&the_fate(&repo, "feature-x")) {
        WhyStays::HasMerge { merge: found } => assert_eq!(*found, merge),
        other => panic!("expected HasMerge, got {other:?}"),
    }
}

#[test]
fn a_fork_of_a_fork_stays_nested() {
    let repo = forked();
    repo.git(&["switch", "-q", "feature-x"]);
    commit(&repo, "x2");
    repo.git(&["switch", "-q", "-c", "feature-y", "feature-x~1"]);
    commit(&repo, "y1");
    repo.git(&["switch", "-q", "feature-a"]);
    let (_, drawn) = as_run(&repo, reword_c1(&repo, &[]));
    assert!(matches!(
        fate_of(&drawn, "feature-x"),
        ForkFate::Follows { .. }
    ));
    match stays(&fate_of(&drawn, "feature-y")) {
        WhyStays::Nested { on } => assert_eq!(on.as_bytes(), b"refs/heads/feature-x"),
        other => panic!("expected Nested, got {other:?}"),
    }
}

#[test]
fn the_default_branch_stays() {
    let repo = forked();
    repo.git(&["switch", "-q", "-c", "release", "feature-a~1"]);
    commit(&repo, "r1");
    repo.git(&["switch", "-q", "feature-a"]);
    repo.git(&["config", "init.defaultBranch", "release"]);
    assert_eq!(stays(&the_fate(&repo, "release")), &WhyStays::DefaultBranch);
}

/// Nine forks: eight named, the ninth only counted — and it stays (git).
#[test]
fn the_ninth_fork_is_past_the_cap_and_stays() {
    let repo = forked();
    repo.git(&["branch", "-q", "-D", "feature-x"]);
    for k in 0..9 {
        repo.git(&["switch", "-q", "-c", &format!("fork-{k}"), "feature-a~1"]);
        commit(&repo, &format!("k{k}"));
    }
    repo.git(&["switch", "-q", "feature-a"]);
    let (op, drawn) = as_run(&repo, reword_c1(&repo, &[]));
    assert_eq!(drawn.forks.len(), 8, "{:?}", fates(&drawn));
    assert_eq!(drawn.forks_more, 1);
    let named: Vec<String> = fates(&drawn).into_iter().map(|(name, _)| name).collect();
    let ninth: Vec<String> = (0..9)
        .map(|k| format!("fork-{k}"))
        .filter(|name| !named.contains(name))
        .collect();
    assert_eq!(ninth.len(), 1, "{named:?}");
    let was = id(&repo, &ninth[0]);

    run(&repo, &op, NOW);
    assert_eq!(id(&repo, &ninth[0]), was, "{} stayed", ninth[0]);
    for name in &named {
        assert_eq!(
            id(&repo, &format!("{name}~1")),
            id(&repo, "feature-a~1"),
            "{name} followed"
        );
    }
}

#[test]
fn an_unchecked_forecast_never_follows() {
    let repo = forked();
    let old = GitVersion {
        major: 2,
        minor: 39,
        patch: 0,
    };
    let (_, drawn) = as_run_with(&repo, reword_c1(&repo, &[]), &old);
    assert!(
        matches!(
            stays(&fate_of(&drawn, "feature-x")),
            WhyStays::NotChecked(_)
        ),
        "{:?}",
        fates(&drawn)
    );
}

// ── Acceptance 9: a fork moved after the preview ────────────────────────

#[test]
fn a_fork_moved_after_the_preview_is_refused_moved() {
    let repo = forked();
    let (op, _) = as_run(&repo, reword_c1(&repo, &[]));
    repo.git(&["switch", "-q", "feature-x"]);
    commit(&repo, "x2");
    repo.git(&["switch", "-q", "feature-a"]);
    let before = state_of(&repo);

    match try_run(&repo, &op, NOW) {
        Err(ApplyError::Refused(Refusal::Moved { name })) => {
            assert_eq!(name, "refs/heads/feature-x");
        }
        Err(other) => panic!("expected Moved, got {}", other.explain()),
        Ok(applied) => panic!("it ran: {:?}", applied.entry.status),
    }
    assert_eq!(state_of(&repo), before, "nothing written");
}

// ── Acceptance 10: a relaunch finishes the section ──────────────────────

#[test]
fn a_stop_resumed_after_relaunch_finishes_the_section() {
    let repo = forked();
    let x1 = own_diff(&repo, "feature-x");
    let (op, _) = as_run(&repo, reword_c1(&repo, &["c1"]));
    stopped(&repo, &op);

    // The relaunch: a fresh journal and context, nothing kept in memory.
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    let recovered = recover_interrupted(&mut ctx).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(recovered.live.len(), 1, "the stopped rewrite is live");
    let applied =
        resume(&mut ctx, Resume::Continue).unwrap_or_else(|why| panic!("{}", why.explain()));

    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert_eq!(id(&repo, "feature-x~1"), id(&repo, "feature-a~1"));
    assert_eq!(own_diff(&repo, "feature-x"), x1);
    assert_eq!(head(&repo), "refs/heads/feature-a", "HEAD came home");
}
