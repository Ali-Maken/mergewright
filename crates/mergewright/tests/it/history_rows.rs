//! Rows and commits under the fold (ADR-0065 D3), through the real app: a
//! repository with one merged branch of [`K`] commits, its branch deleted.
//! Folded, the branch's commits have no row, so rows + hidden commits is
//! every commit the walk reached; with every branch shown nothing hides.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use mergewright::config::MergedBranches;
use mergewright::update::AppState;

use crate::app_harness::App;

/// Commits on the merged branch.
const K: usize = 3;

/// `main`: a root, then a merge of a [`K`]-commit branch that is deleted.
fn one_merged_branch() -> tempfile::TempDir {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let repo = scratch.path();
    mergewright_fixtures::git::init_repo(repo).expect("git init");
    let mut at: i64 = 1_700_000_000;
    let mut step = |args: &[&str]| {
        at += 60;
        mergewright_fixtures::git::run_at(repo, args, &format!("@{at} +0000"))
            .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    };
    step(&["commit", "--quiet", "--allow-empty", "-m", "root"]);
    step(&["switch", "--quiet", "-c", "feature"]);
    for n in 1..=K {
        step(&[
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            &format!("feature {n}"),
        ]);
    }
    step(&["switch", "--quiet", "-"]);
    step(&[
        "merge",
        "--quiet",
        "--no-ff",
        "-m",
        "Merged PR 1",
        "feature",
    ]);
    step(&["branch", "--quiet", "-D", "feature"]);
    scratch
}

fn all_commits(repo: &Path) -> usize {
    let out =
        mergewright_fixtures::git::run(repo, &["rev-list", "--count", "--all"]).expect("rev-list");
    String::from_utf8(out)
        .expect("utf-8")
        .trim()
        .parse()
        .expect("a number")
}

/// The finished walk's `(laid_out, walked, hidden)` in `mode`.
fn finished(repo: &Path, mode: MergedBranches) -> (usize, usize, usize) {
    let (app, mut state) = App::open(repo);
    state.graph_mode = mode;
    let state = app.resize(&state, 120, 36);
    let state = app.pump(state, "the walk laid out", |state: &AppState| {
        crate::fold_by_default::window(state).is_some_and(|window| {
            !window.rows.is_empty() && window.done && window.end() == window.laid_out
        })
    });
    let window = crate::fold_by_default::window(&state).expect("a window");
    let counts = (window.laid_out, window.walked, window.hidden);
    drop(state);
    app.close();
    counts
}

#[test]
fn folded_rows_and_hidden_commits_add_up_to_every_commit() {
    let scratch = one_merged_branch();
    let every = all_commits(scratch.path());
    assert_eq!(every, 1 + K + 1, "the fixture: root, the branch, the merge");

    let (laid_out, walked, hidden) = finished(scratch.path(), MergedBranches::Folded);
    assert_eq!(hidden, K, "the branch's commits sit behind the merge");
    assert_eq!(walked, every, "the walk reached every commit");
    assert_eq!(laid_out + hidden, walked, "rows + hidden == walked");

    let (laid_out, walked, hidden) = finished(scratch.path(), MergedBranches::Shown);
    assert_eq!(hidden, 0);
    assert_eq!(laid_out, walked);
    assert_eq!(walked, every);
}
