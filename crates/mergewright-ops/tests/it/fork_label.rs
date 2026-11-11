//! A rewrite stopped inside a fork's section round-trips through the label
//! it wrote into git's todo: reopening reads the fork back from disk, and
//! continuing finishes as the same rewrite run without a stop does.
//!
//! The assertions never spell the label, so this test is unchanged by a
//! rename of it.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use mergewright_core::journal::EntryStatus;
use mergewright_ops::apply::{ApplyError, Resume, recover_interrupted, resume};
use mergewright_ops::rewrite::progress;

use crate::fork_follows::{as_run, forked, id, reword_c1};
use crate::support::{NOW, Never, TempRepo, context, journal_for, run, try_run};

/// Put a `break` after the first pick that follows a `label` line in the
/// todo git is running, and continue up to it in the shell.
fn break_after_a_label(repo: &TempRepo) {
    let todo_path = repo.path().join(".git/rebase-merge/git-rebase-todo");
    let todo = std::fs::read_to_string(&todo_path).expect("the todo reads");
    let mut out = String::new();
    let mut labelled = false;
    let mut broke = false;
    for line in todo.lines() {
        out.push_str(line);
        out.push('\n');
        if line.starts_with("label ") {
            labelled = true;
        } else if labelled && !broke && line.starts_with("pick ") {
            out.push_str("break\n");
            broke = true;
        }
    }
    assert!(broke, "the todo has a labelled section: {todo}");
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

/// Each branch's commits from `main`: tree and message, oldest first.
fn shape(repo: &TempRepo) -> Vec<String> {
    ["feature-a", "feature-x"]
        .iter()
        .map(|branch| {
            repo.git(&[
                "log",
                "--reverse",
                "--format=%T %s",
                &format!("main..{branch}"),
            ])
        })
        .collect()
}

#[test]
fn rewrite_fork_label_round_trips() {
    let straight = forked();
    let (op, _) = as_run(&straight, reword_c1(&straight, &[]));
    run(&straight, &op, NOW);

    let repo = forked();
    let (op, _) = as_run(&repo, reword_c1(&repo, &["c2"]));
    match try_run(&repo, &op, NOW) {
        Err(ApplyError::Conflicted { entry, .. }) => {
            assert_eq!(entry.status, EntryStatus::Pending);
        }
        Err(other) => panic!(
            "expected a stop: {}",
            other.explain(&crate::support::Letters)
        ),
        Ok(_) => panic!("expected a stop, it finished"),
    }
    break_after_a_label(&repo);

    // Reopening: progress read from disk names the fork by its label.
    let at = progress(&repo.open(), &Never)
        .expect("progress reads")
        .expect("a rebase is under way");
    let fork = at.fork.as_ref().expect("inside a fork's section");
    assert_eq!(fork.name, "feature-x");
    assert_eq!(fork.at, id(&repo, "feature-x~1"), "c2, old");

    // The relaunch: a fresh journal and context, then continue to the end.
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    let recovered = recover_interrupted(&mut ctx)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(recovered.live.len(), 1, "the stopped rewrite is live");
    let applied = resume(&mut ctx, Resume::Continue { solved: Vec::new() })
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(applied.entry.status, EntryStatus::Committed);

    assert_eq!(
        shape(&repo),
        shape(&straight),
        "the same result as a run without a stop"
    );
    assert_eq!(
        id(&repo, "feature-x~1"),
        id(&repo, "feature-a~1"),
        "the fork followed"
    );
}
