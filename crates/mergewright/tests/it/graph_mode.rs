//! Every branch (ease-pass 06h) through the real app: `wallet_release`
//! opened with the graph mode `shown`, as `[graph] merged_branches =
//! "shown"` starts it — nothing folds, every merged branch on its own lane.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use mergewright::config::MergedBranches;
use mergewright::history::View;

use crate::app_harness::App;
use crate::fold_by_default::{drawn, wallet_release, window};

/// `wallet_release` opened at 120×36 in Every branch, every count landed.
#[test]
fn every_branch_first_paint_draws_every_merged_commit_and_says_so() {
    let scratch = wallet_release();
    let (app, mut state) = App::open(scratch.path());
    state.graph_mode = MergedBranches::Shown;
    let state = app.resize(&state, 120, 36);
    let state = app.pump(state, "the first rows", |state| {
        window(state).is_some_and(|window| !window.rows.is_empty())
    });
    let state = crate::captures::at_rest(&app, state);

    let history = state.history.as_ref().expect("a walk");
    assert_eq!(history.merged_branches(), MergedBranches::Shown);
    assert_eq!(history.view, View::default(), "the unfolded layout");
    every_merged_commit_held_unfolded(&state);

    let frame = drawn(&state);
    assert!(
        frame[0].trim_end().ends_with("every branch shown"),
        "{}",
        frame[0]
    );
    assert!(!frame[0].contains("merged branch"), "{}", frame[0]);
    let tail = |line: &String| line.contains(" commits from ") || line.contains(" commit from ");
    assert!(
        !frame.iter().any(tail),
        "no fold tail:\n{}",
        frame.join("\n")
    );
    crate::captures::check_shot(
        "ease-pass/main-merged-shown-120.txt",
        &format!(
            "ease-pass 06h, 120x36, `wallet_release` in Every branch (`[graph] merged_branches = \
             \"shown\"`), cursor on the first merge (`HEAD`).\n\
             What right looks like: every commit of PR 16021 and `sara/fix-toc` drawn on its \
             own lane joining its merge; no `▸` tail on any row; the top line ends `every \
             branch shown` and says nothing of merged branches folded. Colour unverified \
             (rule 5).\n\n{}\n",
            frame.join("\n")
        ),
    );
    drop(state);
    app.close();
}

/// The first window holds every merged branch's commit, and no row folds.
fn every_merged_commit_held_unfolded(state: &mergewright::update::AppState) {
    let first = window(state).expect("a window");
    let subjects: Vec<String> = first
        .rows
        .iter()
        .map(|row| {
            String::from_utf8_lossy(&row.commit.message)
                .trim()
                .to_owned()
        })
        .collect();
    for merged in [
        "totals: read the ledger",
        "totals: round once",
        "totals: drop the cache",
        "totals: test the sum",
        "toc: fix the anchors",
    ] {
        assert!(
            subjects.iter().any(|s| s == merged),
            "{merged}: {subjects:?}"
        );
    }
    assert!(
        first.fold.iter().all(|fact| fact.folds.is_none()),
        "nothing folds"
    );
}
