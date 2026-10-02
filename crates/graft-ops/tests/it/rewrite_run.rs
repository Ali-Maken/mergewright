//! The rewrite engine's run half (ADR-0047), against real repositories: every
//! branch of the line recorded and moved, `u` putting all of them back, an
//! abort and a relaunch coming home to his branch, and the saved plan gone
//! once the entry closes.
//!
//! Whether a rewrite does what a hand-run `git rebase -i` does is
//! `rewrite_oracle`'s; here it is what Graft records around it.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use graft_core::journal::{EntryStatus, JournalEntry, OperationKind};
use graft_core::oid::Oid;
use graft_git::state::InProgress;
use graft_ops::apply::{Applied, ApplyError, Resume, recover_interrupted, resume};
use graft_ops::facts::read_facts;
use graft_ops::operation::{FactsScope, Operation, RebaseParams, Refusal, Source, Tip};
use graft_ops::preview::plan_as_run;
use graft_ops::rewrite::{RewriteRequest, TodoAction, plan};

use crate::support::{
    NOW, Never, TempRepo, context, git_version, journal_for, run, run_undo, state_of, try_run,
};

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a commit id")
}

fn tip(repo: &TempRepo, branch: &str) -> Tip {
    Tip {
        name: format!("refs/heads/{branch}"),
        oid: id(repo, branch),
    }
}

/// One commit changing its own file, so no two commits conflict.
fn commit(repo: &TempRepo, name: &str) {
    repo.write(&format!("{name}.txt"), format!("{name}\n").as_bytes());
    repo.commit_all(name);
}

/// A line of three: `feature-a` (two commits) off `main`'s first commit,
/// `feature-b` one on it, `feature-c` one on that; `main` then moves on.
/// `HEAD` on `feature-a`.
fn line() -> TempRepo {
    let repo = TempRepo::init();
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    commit(&repo, "a1");
    commit(&repo, "a2");
    repo.git(&["switch", "-q", "-c", "feature-b"]);
    commit(&repo, "b1");
    repo.git(&["switch", "-q", "-c", "feature-c"]);
    commit(&repo, "c1");
    repo.git(&["switch", "-q", "main"]);
    commit(&repo, "m2");
    repo.git(&["switch", "-q", "feature-a"]);
    repo
}

const LINE: [&str; 3] = [
    "refs/heads/feature-a",
    "refs/heads/feature-b",
    "refs/heads/feature-c",
];

/// `r` of `feature-a` onto `main`, as a key builds it: his branch alone,
/// until its preview completes it ([`r_onto_main`]).
fn rebase_onto_main(repo: &TempRepo) -> Operation {
    Operation::Rebase(RebaseParams::new(
        tip(repo, "feature-a"),
        Source::Branch("refs/heads/main".to_owned()),
        id(repo, "main"),
    ))
}

/// The whole line of `feature-a` replayed onto `main`: `R`'s plan (which
/// carries the line) with its base moved to `main`'s tip. `r`'s own line
/// runs are proven with [`r_onto_main`].
fn line_onto_main(repo: &TempRepo) -> Operation {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    let mut params = plan(
        &handle,
        &git_version(),
        &RewriteRequest::Open {
            from: id(repo, "feature-a~1"),
        },
        &facts,
        None,
        &Never,
    )
    .unwrap_or_else(|why| panic!("it plans: {why:?}"));
    let names: Vec<&str> = params
        .line
        .branches
        .iter()
        .map(|tip| tip.name.as_str())
        .collect();
    assert_eq!(names, LINE, "R's plan carries the line");
    params.onto = id(repo, "main");
    Operation::Rewrite(params)
}

fn head(repo: &TempRepo) -> String {
    repo.try_git(&["symbolic-ref", "-q", "HEAD"])
        .unwrap_or_else(|_| "detached".to_owned())
}

fn parent(repo: &TempRepo, rev: &str) -> Oid {
    id(repo, &format!("{rev}^"))
}

// ── `r`: the line its preview drew (05b) ────────────────────────────────

/// `op` as its preview completed it (`plan_as_run`): what `y` runs.
fn completed(repo: &TempRepo, op: Operation) -> Operation {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("the facts read");
    plan_as_run(&handle, &git_version(), op, &facts, &Never)
        .unwrap_or_else(|why| panic!("it plans: {why:?}"))
        .0
}

/// `r` of `feature-a` onto `main`, as its preview completed it.
fn r_onto_main(repo: &TempRepo) -> Operation {
    completed(repo, rebase_onto_main(repo))
}

#[test]
fn r_moves_every_branch_of_its_line() {
    let repo = line();
    let was: Vec<Oid> = LINE.iter().map(|name| id(&repo, name)).collect();

    let entry = run(&repo, &r_onto_main(&repo), NOW);

    for (name, was) in LINE.iter().zip(&was) {
        assert_ne!(id(&repo, name), *was, "{name} moved");
    }
    assert_eq!(parent(&repo, "feature-a~1"), id(&repo, "main"));
    assert_eq!(parent(&repo, "feature-b"), id(&repo, "feature-a"));
    assert_eq!(parent(&repo, "feature-c"), id(&repo, "feature-b"));
    assert_eq!(head(&repo), "refs/heads/feature-a", "HEAD came home");
    assert_eq!(entry.kind, OperationKind::Rebase);
    assert_eq!(entry.status, EntryStatus::Committed);
}

#[test]
fn r_records_and_undoes_every_branch_of_its_line() {
    let repo = line();
    let before = state_of(&repo);
    let was: Vec<Oid> = LINE.iter().map(|name| id(&repo, name)).collect();

    let entry = run(&repo, &r_onto_main(&repo), NOW);
    for (name, was) in LINE.iter().zip(&was) {
        assert_eq!(
            entry.refs_before.get(*name),
            Some(&Some(*was)),
            "{name} recorded before: {:?}",
            entry.refs_before
        );
        assert_eq!(
            entry.refs_after.get(*name),
            Some(&Some(id(&repo, name))),
            "{name} recorded after"
        );
    }

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{why:?}"));
    assert_eq!(state_of(&repo), before, "u puts each branch back");
}

#[test]
fn an_r_line_that_stops_continues_home() {
    let repo = conflicted_line();
    let was: Vec<Oid> = LINE.iter().map(|name| id(&repo, name)).collect();
    let (entry, paths) = stopped(&repo, &r_onto_main(&repo));
    assert_eq!(paths, [PathBuf::from("shared.txt")]);
    assert_eq!(entry.status, EntryStatus::Pending);
    assert_eq!(head(&repo), "detached", "stopped inside the line");
    resolve(&repo);

    let applied =
        resume_with(&repo, Resume::Continue).unwrap_or_else(|why| panic!("{}", why.explain()));

    assert_eq!(applied.entry.status, EntryStatus::Committed);
    for (name, was) in LINE.iter().zip(&was) {
        assert_ne!(id(&repo, name), *was, "{name} moved");
    }
    assert_eq!(parent(&repo, "feature-b"), id(&repo, "feature-a"));
    assert_eq!(parent(&repo, "feature-c"), id(&repo, "feature-b"));
    assert_eq!(head(&repo), "refs/heads/feature-a", "HEAD came home");
    assert_eq!(plans_left(&repo), Vec::<PathBuf>::new());
}

#[test]
fn a_branch_built_on_his_after_the_preview_is_refused_moved() {
    let repo = line();
    let op = r_onto_main(&repo);
    // After the preview: feature-d built on feature-c.
    repo.git(&["switch", "-q", "-c", "feature-d", "feature-c"]);
    commit(&repo, "d1");
    repo.git(&["switch", "-q", "feature-a"]);
    let before = state_of(&repo);

    match try_run(&repo, &op, NOW) {
        Err(ApplyError::Refused(Refusal::Moved { name })) => {
            assert_eq!(name, "refs/heads/feature-d");
        }
        Err(other) => panic!("expected Moved, got {}", other.explain()),
        Ok(applied) => panic!("it ran: {:?}", applied.entry.status),
    }
    assert_eq!(state_of(&repo), before, "no ref moved");
}

/// `feature-a`'s three commits squash-merged into `main` as one commit,
/// then `main` moves on; `feature-b` built on it.
fn squash_merged() -> TempRepo {
    let repo = TempRepo::init();
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    commit(&repo, "a1");
    commit(&repo, "a2");
    commit(&repo, "a3");
    repo.git(&["switch", "-q", "-c", "feature-b"]);
    commit(&repo, "b1");
    repo.git(&["switch", "-q", "main"]);
    commit(&repo, "m2");
    repo.git(&["merge", "-q", "--squash", "feature-a"]);
    repo.git(&["commit", "-q", "-m", "Add OAuth (#42)"]);
    commit(&repo, "m3");
    repo.git(&["switch", "-q", "feature-a"]);
    repo
}

#[test]
fn a_drop_not_among_the_rows_is_refused_moved() {
    let repo = squash_merged();
    let mut op = r_onto_main(&repo);
    let Operation::Rebase(params) = &mut op else {
        panic!("a rebase");
    };
    assert_eq!(params.drops.len(), 3, "the preview filled the drops");
    params.drops.push(id(&repo, "main"));
    let before = state_of(&repo);

    match try_run(&repo, &op, NOW) {
        Err(ApplyError::Refused(Refusal::Moved { name })) => {
            assert_eq!(name, "refs/heads/feature-a");
        }
        Err(other) => panic!("expected Moved, got {}", other.explain()),
        Ok(applied) => panic!("it ran: {:?}", applied.entry.status),
    }
    assert_eq!(state_of(&repo), before, "no ref moved");
}

#[test]
fn r_drops_the_squash_merged_commits() {
    // feature-b's commit and main's last both change `shared.txt`, so git
    // stops on b1 with the rows it ran in `rebase-merge/done`.
    let repo = squash_merged();
    repo.git(&["switch", "-q", "feature-b"]);
    repo.write("shared.txt", b"b\n");
    repo.commit_all("b2");
    repo.git(&["switch", "-q", "main"]);
    repo.write("shared.txt", b"m\n");
    repo.commit_all("m4");
    repo.git(&["switch", "-q", "feature-a"]);
    let dropped: Vec<Oid> = ["feature-a~2", "feature-a~1", "feature-a"]
        .iter()
        .map(|rev| id(&repo, rev))
        .collect();

    stopped(&repo, &r_onto_main(&repo));
    let done = std::fs::read_to_string(repo.path().join(".git/rebase-merge/done"))
        .expect("git's done file reads");
    let ran: Vec<(&str, &str)> = done
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            Some((words.next()?, words.next()?))
        })
        .collect();
    for commit in &dropped {
        let hex = commit.to_string();
        assert!(
            ran.iter()
                .any(|(word, id)| *word == "drop" && hex.starts_with(id)),
            "{hex} dropped: {done}"
        );
    }

    repo.write("shared.txt", b"m\nb\n");
    repo.git(&["add", "shared.txt"]);
    let applied =
        resume_with(&repo, Resume::Continue).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert_eq!(
        id(&repo, "feature-a"),
        id(&repo, "main"),
        "all of it in main"
    );
    assert_eq!(
        repo.git(&["log", "--format=%s", "main..feature-b"]),
        "b2\nb1",
        "the rest land on it"
    );
    assert_eq!(head(&repo), "refs/heads/feature-a");
}

// ── Record and undo ─────────────────────────────────────────────────────

#[test]
fn a_three_branch_rewrite_records_every_ref() {
    let repo = line();
    let before: Vec<Oid> = LINE.iter().map(|name| id(&repo, name)).collect();

    let entry = run(&repo, &line_onto_main(&repo), NOW);

    for (name, was) in LINE.iter().zip(&before) {
        assert_eq!(
            entry.refs_before.get(*name),
            Some(&Some(*was)),
            "{name} is recorded before: {:?}",
            entry.refs_before
        );
        let now = id(&repo, name);
        assert_ne!(now, *was, "{name} moved");
        assert_eq!(
            entry.refs_after.get(*name),
            Some(&Some(now)),
            "{name} is recorded after"
        );
    }
    // Each branch is built on the one below, rewritten, and the bottom on
    // `main`.
    assert_eq!(parent(&repo, "feature-a~1"), id(&repo, "main"));
    assert_eq!(parent(&repo, "feature-b"), id(&repo, "feature-a"));
    assert_eq!(parent(&repo, "feature-c"), id(&repo, "feature-b"));
    assert_eq!(head(&repo), "refs/heads/feature-a", "HEAD came home");
    assert_eq!(entry.kind, OperationKind::Rewrite);
    assert_eq!(entry.status, EntryStatus::Committed);
}

#[test]
fn undo_puts_every_branch_in_the_line_back() {
    let repo = line();
    let before = state_of(&repo);

    run(&repo, &line_onto_main(&repo), NOW);
    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{why:?}"));

    assert_eq!(state_of(&repo), before);
}

// ── Reword HEAD with branches built on it (m8/14 item 1) ─────────────────

/// `reword_head` on `feature-a` with `feature-b` and `feature-c` built on
/// it, as its preview would run it.
fn reword_a2(repo: &TempRepo, message: &str) -> Operation {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    let params = plan(
        &handle,
        &git_version(),
        &RewriteRequest::RewordHead {
            message: message.to_owned(),
        },
        &facts,
        None,
        &Never,
    )
    .unwrap_or_else(|why| panic!("it plans: {why:?}"));
    Operation::Rewrite(params)
}

#[test]
fn reword_head_plans_one_reword_row_with_the_line() {
    let repo = line();
    let Operation::Rewrite(params) = reword_a2(&repo, "a2, reworded\n") else {
        unreachable!("reword_a2 builds a rewrite")
    };
    assert_eq!(params.todo.len(), 1, "{:?}", params.todo);
    assert_eq!(params.todo[0].action, TodoAction::Reword);
    assert_eq!(params.todo[0].commit, id(&repo, "feature-a"));
    assert_eq!(params.onto, id(&repo, "feature-a~1"));
    assert_eq!(
        params.messages.get(&0).map(String::as_str),
        Some("a2, reworded\n")
    );
    let names: Vec<&str> = params
        .line
        .branches
        .iter()
        .map(|tip| tip.name.as_str())
        .collect();
    assert_eq!(names, LINE);
    assert!(params.made.is_empty());
}

/// J19 `reword-head`, the operation half: `HEAD`'s message changes, the
/// branches built on it follow, and `u` puts all of them back. Truth read
/// with git.
#[test]
fn reword_head_rewords_moves_the_line_and_undoes_as_one() {
    let repo = line();
    let before = state_of(&repo);
    let a1 = id(&repo, "feature-a~1");
    let tree = repo.git(&["rev-parse", "feature-a^{tree}"]);

    let entry = run(
        &repo,
        &reword_a2(&repo, "a2, reworded\n\nwith a body\n"),
        NOW,
    );

    assert_eq!(
        repo.git(&["log", "-1", "--format=%B", "feature-a"])
            .trim_end(),
        "a2, reworded\n\nwith a body"
    );
    assert_eq!(parent(&repo, "feature-a"), a1, "only HEAD is rewritten");
    assert_eq!(
        repo.git(&["rev-parse", "feature-a^{tree}"]),
        tree,
        "its content stays"
    );
    assert_eq!(parent(&repo, "feature-b"), id(&repo, "feature-a"));
    assert_eq!(parent(&repo, "feature-c"), id(&repo, "feature-b"));
    assert_eq!(head(&repo), "refs/heads/feature-a", "HEAD came home");
    assert_eq!(entry.kind, OperationKind::Rewrite);
    assert_eq!(entry.status, EntryStatus::Committed);

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{why:?}"));
    assert_eq!(state_of(&repo), before);
}

/// Nothing built on `HEAD`: not a rewrite — the plain amend rewords it.
#[test]
fn reword_head_with_nothing_built_on_it_is_a_plain_amend() {
    let repo = line();
    repo.git(&["switch", "-q", "feature-c"]);
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    let refused = plan(
        &handle,
        &git_version(),
        &RewriteRequest::RewordHead {
            message: "c1!\n".to_owned(),
        },
        &facts,
        None,
        &Never,
    );
    assert!(
        matches!(
            refused,
            Err(graft_ops::rewrite::RewriteError::Refused(
                Refusal::PlainAmend
            ))
        ),
        "{refused:?}"
    );
}

// ── Stopped, resumed, relaunched ────────────────────────────────────────

/// `line()`, but `feature-b`'s commit and `main`'s second one both change
/// `shared.txt`: `r` of `feature-a` onto `main` stops on `b1`, with `HEAD`
/// detached and git's `head-name` the top of the line, `feature-c`.
fn conflicted_line() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("shared.txt", b"base\n");
    repo.commit_all("m1");
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    commit(&repo, "a1");
    commit(&repo, "a2");
    repo.git(&["switch", "-q", "-c", "feature-b"]);
    repo.write("shared.txt", b"b\n");
    repo.commit_all("b1");
    repo.git(&["switch", "-q", "-c", "feature-c"]);
    commit(&repo, "c1");
    repo.git(&["switch", "-q", "main"]);
    repo.write("shared.txt", b"m\n");
    repo.commit_all("m2");
    repo.git(&["switch", "-q", "feature-a"]);
    repo
}

/// Apply `op`, which must stop: the entry it left `Pending`, and the paths.
fn stopped(repo: &TempRepo, op: &Operation) -> (JournalEntry, Vec<PathBuf>) {
    match try_run(repo, op, NOW) {
        Err(ApplyError::Conflicted { entry, paths, .. }) => (*entry, paths),
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

/// The saved plans still on disk.
fn plans_left(repo: &TempRepo) -> Vec<PathBuf> {
    match std::fs::read_dir(repo.path().join(".git/graft/rewrite")) {
        Ok(entries) => entries
            .map(|entry| entry.expect("it reads").path())
            .collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => panic!("the plan root reads: {error}"),
    }
}

/// `b1` against `m2`: both sides kept.
fn resolve(repo: &TempRepo) {
    repo.write("shared.txt", b"m\nb\n");
    repo.git(&["add", "shared.txt"]);
}

fn in_progress(repo: &TempRepo) -> Option<InProgress> {
    graft_git::state::in_progress(&repo.open()).expect("git's state reads")
}

#[test]
fn abort_returns_head_to_his_branch_and_every_ref() {
    let repo = conflicted_line();
    let before = state_of(&repo);

    let (entry, paths) = stopped(&repo, &line_onto_main(&repo));
    assert_eq!(paths, [PathBuf::from("shared.txt")]);
    assert_eq!(entry.status, EntryStatus::Pending);
    assert_eq!(plans_left(&repo).len(), 1, "the plan waits for the resume");

    let applied =
        resume_with(&repo, Resume::Abort).unwrap_or_else(|why| panic!("{}", why.explain()));

    assert_eq!(state_of(&repo), before, "every branch and HEAD put back");
    assert_eq!(
        applied.entry.status,
        EntryStatus::Aborted {
            reason: "aborted".to_owned()
        }
    );
    assert_eq!(plans_left(&repo), Vec::<PathBuf>::new());
}

#[test]
fn a_stopped_line_continues_home_with_every_branch_moved() {
    let repo = conflicted_line();
    let was: Vec<Oid> = LINE.iter().map(|name| id(&repo, name)).collect();
    stopped(&repo, &line_onto_main(&repo));
    resolve(&repo);

    let applied =
        resume_with(&repo, Resume::Continue).unwrap_or_else(|why| panic!("{}", why.explain()));

    assert_eq!(applied.entry.status, EntryStatus::Committed);
    for (name, was) in LINE.iter().zip(&was) {
        assert_ne!(id(&repo, name), *was, "{name} moved");
    }
    assert_eq!(parent(&repo, "feature-a~1"), id(&repo, "main"));
    assert_eq!(parent(&repo, "feature-b"), id(&repo, "feature-a"));
    assert_eq!(parent(&repo, "feature-c"), id(&repo, "feature-b"));
    assert_eq!(head(&repo), "refs/heads/feature-a", "HEAD came home");
    assert_eq!(repo.git(&["log", "-1", "--format=%B", "feature-b"]), "b1");
    assert_eq!(plans_left(&repo), Vec::<PathBuf>::new());
}

#[test]
fn kill_mid_rewrite_then_relaunch_continues_from_the_saved_plan() {
    let repo = conflicted_line();
    let (entry, _) = stopped(&repo, &line_onto_main(&repo));

    // The relaunch: a fresh journal and context, nothing kept in memory.
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    let recovered = recover_interrupted(&mut ctx).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(
        recovered.live,
        std::slice::from_ref(&entry.id),
        "the stopped line is still live"
    );
    assert!(recovered.closed.is_empty());
    assert_eq!(in_progress(&repo), Some(InProgress::Rebase));
    resolve(&repo);

    let applied =
        resume(&mut ctx, Resume::Continue).unwrap_or_else(|why| panic!("{}", why.explain()));

    assert_eq!(applied.entry.id, entry.id);
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert_eq!(head(&repo), "refs/heads/feature-a", "HEAD came home");
    assert_eq!(parent(&repo, "feature-c"), id(&repo, "feature-b"));
    assert_eq!(plans_left(&repo), Vec::<PathBuf>::new());
}

/// `R` on `a1` in `line()`, planned as the app would, with `a1` set to
/// `edit`.
fn edit_a1(repo: &TempRepo) -> Operation {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    let mut params = plan(
        &handle,
        &git_version(),
        &RewriteRequest::Open {
            from: id(repo, "feature-a~1"),
        },
        &facts,
        None,
        &Never,
    )
    .unwrap_or_else(|why| panic!("it plans: {why:?}"));
    assert_eq!(params.todo[0].subject, "a1");
    params.todo[0].action = TodoAction::Edit;
    Operation::Rewrite(params)
}

#[test]
fn an_edit_row_stops_and_the_resume_takes_the_line_home() {
    let repo = line();
    let (entry, paths) = stopped(&repo, &edit_a1(&repo));
    assert!(paths.is_empty(), "an edit stop lists nothing: {paths:?}");
    assert_eq!(entry.status, EntryStatus::Pending);
    assert_eq!(in_progress(&repo), Some(InProgress::Rebase));

    // Amended at the stop, as he would in the shell or the app.
    repo.write("a1.txt", b"a1 amended\n");
    repo.git(&["commit", "-q", "--amend", "--no-edit", "-a"]);
    let applied =
        resume_with(&repo, Resume::Continue).unwrap_or_else(|why| panic!("{}", why.explain()));

    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert_eq!(head(&repo), "refs/heads/feature-a", "HEAD came home");
    assert_eq!(
        repo.git(&["show", "feature-c:a1.txt"]),
        "a1 amended",
        "the line is built on the amended commit"
    );
    assert_eq!(parent(&repo, "feature-b"), id(&repo, "feature-a"));
    assert_eq!(parent(&repo, "feature-c"), id(&repo, "feature-b"));
    assert_eq!(plans_left(&repo), Vec::<PathBuf>::new());
}

#[test]
fn a_relaunch_that_closes_a_rewrite_removes_its_plan() {
    let repo = conflicted_line();
    let (entry, _) = stopped(&repo, &line_onto_main(&repo));
    // Called off in the shell while Graft was down: nothing is live.
    repo.git(&["rebase", "--abort"]);

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    let recovered = recover_interrupted(&mut ctx).unwrap_or_else(|why| panic!("{}", why.explain()));

    assert_eq!(recovered.closed, [entry.id]);
    assert!(recovered.plans_left.is_empty());
    assert_eq!(plans_left(&repo), Vec::<PathBuf>::new());
}

#[test]
fn the_plan_dir_is_removed_when_the_entry_closes() {
    // Done.
    let repo = line();
    run(&repo, &r_onto_main(&repo), NOW);
    assert_eq!(plans_left(&repo), Vec::<PathBuf>::new(), "done");

    // Failed: a pre-rebase hook refuses before git moves anything.
    let repo = line();
    let hooks = repo.path().join(".git/test-hooks");
    std::fs::create_dir_all(&hooks).expect("the hook dir");
    let hook = hooks.join("pre-rebase");
    std::fs::write(&hook, b"#!/bin/sh\nexit 1\n").expect("the hook writes");
    make_executable(&hook);
    repo.git(&[
        "config",
        "core.hooksPath",
        hooks.to_str().expect("a utf-8 path"),
    ]);
    let before = state_of(&repo);
    let failed = try_run(&repo, &r_onto_main(&repo), NOW);
    assert!(
        matches!(failed, Err(ApplyError::Git { .. })),
        "the hook refused: {:?}",
        failed.map(|applied| applied.entry.status)
    );
    assert_eq!(state_of(&repo), before);
    assert_eq!(plans_left(&repo), Vec::<PathBuf>::new(), "failed");
    // Aborted: `abort_returns_head_to_his_branch_and_every_ref`.
}

fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
}

#[test]
fn a_resume_never_turns_his_update_refs_config_on() {
    // `rebase.updateRefs=true` in his config, and a branch of his inside the
    // range that is not in the line: the run and the resume both leave it
    // where it was — it is not recorded, so it must not move (ADR-0047 §2).
    let repo = conflicted_line();
    repo.git(&["config", "rebase.updateRefs", "true"]);
    repo.git(&["branch", "lower", "feature-a~1"]);
    let lower = id(&repo, "lower");
    stopped(&repo, &line_onto_main(&repo));
    resolve(&repo);

    resume_with(&repo, Resume::Continue).unwrap_or_else(|why| panic!("{}", why.explain()));

    assert_eq!(id(&repo, "lower"), lower, "lower stays");
    assert_eq!(head(&repo), "refs/heads/feature-a");
}
