//! The fold rule pinned through the real app (ease-pass 06g): a teammate's
//! branch merged into another teammate's unmerged branch folds.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use crate::fold_by_default::{drawn, first_paint, row_of, window};

/// `omar/fix`'s two commits, oldest first.
const OMAR_COMMITS: [&str; 2] = ["fix: guard the empty report", "fix: test the guard"];
/// Sara's merge of `omar/fix` into her unmerged `sara/report`.
const OMAR_MERGE: &str = "Merge branch 'omar/fix' into sara/report";

/// 06g's first case as a real repository: trunk `main` (the fixtures'
/// default branch; the ticket's `development`), `HEAD` on it; `sara/report`
/// (unmerged) takes `omar/fix` through a merge; `omar/fix`'s ref is kept.
fn omar_into_sara() -> tempfile::TempDir {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let repo = scratch.path();
    graft_fixtures::git::init_repo(repo).expect("git init");
    let mut at: i64 = 1_700_000_000;
    let mut step = |args: &[&str]| {
        at += 60;
        graft_fixtures::git::run_at(repo, args, &format!("@{at} +0000"))
            .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    };
    step(&["commit", "--quiet", "--allow-empty", "-m", "Add the readme"]);
    step(&["switch", "--quiet", "-c", "omar/fix"]);
    for subject in OMAR_COMMITS {
        step(&["commit", "--quiet", "--allow-empty", "-m", subject]);
    }
    step(&["switch", "--quiet", "-c", "sara/report", "main"]);
    step(&[
        "commit",
        "--quiet",
        "--allow-empty",
        "-m",
        "report: first draft",
    ]);
    step(&["merge", "--quiet", "--no-ff", "-m", OMAR_MERGE, "omar/fix"]);
    step(&["switch", "--quiet", "main"]);
    step(&["commit", "--quiet", "--allow-empty", "-m", "Trunk moves on"]);
    scratch
}

/// 06g box 7: a teammate's branch merged into another teammate's unmerged
/// branch folds at first paint; the merge's tail names it.
#[test]
fn a_teammates_merged_branch_folds_and_its_tail_names_it() {
    let scratch = omar_into_sara();
    let (app, state) = first_paint(scratch.path());
    let first = window(&state).expect("a window");
    assert!(
        first.rows.iter().all(|row| OMAR_COMMITS
            .iter()
            .all(|hidden| !row.commit.message.starts_with(hidden.as_bytes()))),
        "the first window holds an omar/fix commit"
    );
    row_of(first, "report: first draft");
    let tail = "\u{25b8} 2 commits from omar/fix";
    let state = app.pump(state, "the merge's tail", |state| {
        drawn(state)
            .iter()
            .any(|line| line.contains(OMAR_MERGE) && line.contains(tail))
    });
    let frame = drawn(&state);
    for hidden in OMAR_COMMITS {
        assert!(
            frame.iter().all(|line| !line.contains(hidden)),
            "{hidden:?} drawn\n{}",
            frame.join("\n")
        );
    }
    drop(state);
    app.close();
}
