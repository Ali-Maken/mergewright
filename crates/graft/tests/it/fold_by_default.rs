//! Fold by default through the real app (ease-pass 06, boxes 5, 7 and the
//! first-paint half of 8; built by 06e): a repository shaped like his work
//! repo — `main ← wallet-release ← sara/fix-toc`, a PR merged with its
//! branch deleted and one merge that names its branch — opened over the
//! real job pool and `update`, as the binary opens it.
//!
//! The words and forms of a tail (pending, failed, ASCII) are
//! `view::rows::fold::tests`' and graft-ui's `graph_screen::tests::fold_tail`;
//! here the walked repository's own facts produce them.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::session::Clock;
use graft::update::AppState;
use graft_core::event::{KeyCode, MouseButton};
use graft_core::oid::Oid;

use crate::app_harness::App;
use crate::captures::frame_at;

const COLUMNS: u16 = 120;
const ROWS: u16 = 36;
/// After every commit of [`wallet_release`], so ages read the same.
const NOW: i64 = 1_700_100_000;

/// The PR's merge, its branch deleted: Azure DevOps' subject.
const PR_MERGE: &str = "Merged PR 16021: Fix the wallet totals";
/// The merge that names its branch.
const NAMED_MERGE: &str = "Merge branch 'sara/fix-toc' into wallet-release";
/// The PR branch's four commits, oldest first.
const PR_COMMITS: [&str; 4] = [
    "totals: read the ledger",
    "totals: round once",
    "totals: drop the cache",
    "totals: test the sum",
];
/// `sara/fix-toc`'s one commit.
const TOC_COMMIT: &str = "toc: fix the anchors";

/// `main ← wallet-release ← sara/fix-toc`: `main` has the readme;
/// `wallet-release` (`HEAD`) starts from it, takes PR 16021's four commits
/// through a merge whose branch was then deleted, and `sara/fix-toc`'s one
/// commit through a merge that names it. `sara/fix-toc` itself is kept: a
/// live ref that a shown merge brought in folds as well. Every commit a
/// minute after the last, through the fixtures' hermetic git.
pub fn wallet_release() -> tempfile::TempDir {
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
    step(&["switch", "--quiet", "-c", "wallet-release"]);
    step(&[
        "commit",
        "--quiet",
        "--allow-empty",
        "-m",
        "Start the wallet release",
    ]);
    step(&["switch", "--quiet", "-c", "feature/totals"]);
    for subject in PR_COMMITS {
        step(&["commit", "--quiet", "--allow-empty", "-m", subject]);
    }
    step(&["switch", "--quiet", "wallet-release"]);
    step(&[
        "merge",
        "--quiet",
        "--no-ff",
        "-m",
        PR_MERGE,
        "feature/totals",
    ]);
    step(&["branch", "--quiet", "-D", "feature/totals"]);
    step(&["switch", "--quiet", "-c", "sara/fix-toc"]);
    step(&["commit", "--quiet", "--allow-empty", "-m", TOC_COMMIT]);
    step(&["switch", "--quiet", "wallet-release"]);
    step(&[
        "merge",
        "--quiet",
        "--no-ff",
        "-m",
        NAMED_MERGE,
        "sara/fix-toc",
    ]);
    scratch
}

pub(crate) fn drawn(state: &AppState) -> Vec<String> {
    frame_at(state, COLUMNS, ROWS, Some(Clock::Fixed(NOW)))
}

pub(crate) fn window(state: &AppState) -> Option<&graft::history::Window> {
    state.history.as_ref().and_then(|h| h.window.as_ref())
}

/// The repository opened at 120×36, up to its first window with rows: the
/// first paint.
pub(crate) fn first_paint(repo: &Path) -> (App, AppState) {
    let (app, state) = App::open(repo);
    let state = app.resize(&state, COLUMNS, ROWS);
    let state = app.pump(state, "the first rows", |state| {
        window(state).is_some_and(|window| !window.rows.is_empty())
    });
    (app, state)
}

/// The window's row whose subject is `subject`, with its index.
pub(crate) fn row_of<'w>(
    window: &'w graft::history::Window,
    subject: &str,
) -> (usize, &'w graft_graph::row::Row) {
    window
        .rows
        .iter()
        .enumerate()
        .find(|(_, row)| row.commit.message.starts_with(subject.as_bytes()))
        .unwrap_or_else(|| panic!("no row {subject:?} in the window"))
}

/// Neither the frame nor the window holds a commit of a merged branch, and
/// the window carries a fold fact per row.
fn no_merged_commit_drawn_or_held(state: &AppState) {
    let frame = drawn(state);
    for hidden in PR_COMMITS.iter().chain([&TOC_COMMIT]) {
        assert!(
            frame.iter().all(|line| !line.contains(hidden)),
            "first paint draws {hidden:?}\n{}",
            frame.join("\n")
        );
    }
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
    assert!(
        PR_COMMITS
            .iter()
            .chain([&TOC_COMMIT])
            .all(|hidden| !subjects.iter().any(|subject| subject == hidden)),
        "the window holds a merged-branch commit: {subjects:?}"
    );
    // Not vacuous: the window carries the fold.
    assert_eq!(first.fold.len(), first.rows.len(), "one fold fact per row");
}

/// The opened PR's four rows: one lane, not the merge's, below the merge;
/// the merge's second-parent edge leaves on that lane to the side's tip;
/// each row inside the opened merge.
fn the_side_joins_its_merge(window: &graft::history::Window) {
    let (merge_at, merge) = row_of(window, PR_MERGE);
    let side: Vec<(usize, &graft_graph::row::Row)> = PR_COMMITS
        .iter()
        .map(|subject| row_of(window, subject))
        .collect();
    let lanes: Vec<usize> = side.iter().map(|(_, row)| row.lane).collect();
    let lane = lanes[0];
    assert!(
        lanes.iter().all(|each| *each == lane),
        "one lane for the side: {lanes:?}"
    );
    assert_ne!(lane, merge.lane, "the side is not on the merge's column");
    let tip: Oid = row_of(window, PR_COMMITS[3]).1.commit.id;
    assert_eq!(
        merge.drawn_parents.get(1),
        Some(&tip),
        "the merge draws its edge to the side's tip"
    );
    assert!(
        merge.outgoing.contains(&(lane, tip)),
        "the merge's second-parent edge leaves on the side's lane: {:?}",
        merge.outgoing
    );
    assert!(
        side.iter().all(|(at, _)| *at > merge_at),
        "the side is drawn below its merge"
    );
    for (at, _) in &side {
        assert_eq!(
            window.fold[*at].inside.as_ref().map(|inside| inside.merge),
            Some(merge.commit.id),
            "row {at} is inside the opened merge"
        );
    }
}

/// Box 5: the first paint draws none of the merged branches' commits, the
/// window carries the fold's facts for both merges, and once the counts
/// land the tails read `▸ 4 commits from PR 16021` and
/// `▸ 1 commit from sara/fix-toc`.
#[test]
fn first_paint_folds_merged_branches_and_the_tails_name_them() {
    let scratch = wallet_release();
    let (app, state) = first_paint(scratch.path());
    no_merged_commit_drawn_or_held(&state);
    let merges = [PR_MERGE, NAMED_MERGE];
    let state = app.pump(state, "the fold facts of both merges", |state| {
        window(state).is_some_and(|window| {
            merges.iter().all(|merge| {
                let (at, _) = row_of(window, merge);
                window.fold[at].folds == Some(true)
            })
        })
    });

    // The named merge's subject is cut with `…` beside `HEAD`'s badge; the
    // tail never is.
    let counted = app.pump(state, "both counts", |state| {
        let frame = drawn(state);
        [
            ("Merged PR 16021", "\u{25b8} 4 commits from PR 16021"),
            (
                "Merge branch 'sara/fix-toc'",
                "\u{25b8} 1 commit from sara/fix-toc",
            ),
        ]
        .iter()
        .all(|(merge, tail)| {
            frame
                .iter()
                .any(|line| line.contains(merge) && line.contains(tail))
        })
    });
    let frame = drawn(&counted);
    assert_eq!(
        frame
            .iter()
            .filter(
                |line| line.contains("Start the wallet release") || line.contains("Add the readme")
            )
            .count(),
        2,
        "the kept rows are drawn\n{}",
        frame.join("\n")
    );
    drop(counted);
    app.close();
}

/// Box 7 (Q10 a): `space` on the PR's folded merge opens it — `▾` — and its
/// four commits sit on one lane that is not the merge's own column, the
/// lane the merge's second-parent edge leaves on: they join the merge row.
#[test]
fn space_opens_the_merged_side_on_its_own_lane_joining_the_merge() {
    let scratch = wallet_release();
    let (app, state) = first_paint(scratch.path());
    let frame = drawn(&state);
    let y = frame
        .iter()
        .position(|line| line.contains(PR_MERGE))
        .unwrap_or_else(|| panic!("the PR merge is drawn\n{}", frame.join("\n")));
    let state = app.mouse_down(
        &state,
        MouseButton::Left,
        30,
        u16::try_from(y).expect("short"),
    );
    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.pump(state, "the PR's commits drawn", |state| {
        let frame = drawn(state);
        PR_COMMITS
            .iter()
            .all(|subject| frame.iter().any(|line| line.contains(subject)))
            && frame
                .iter()
                .any(|line| line.contains(&format!("{PR_MERGE}  \u{25be}")))
    });
    the_side_joins_its_merge(window(&state).expect("a window"));
    let state = app.pump(state, "the opened merge's count", |state| {
        drawn(state)
            .iter()
            .any(|line| line.contains(&format!("{PR_MERGE}  \u{25be} 4 commits from PR 16021")))
    });
    drop(state);
    app.close();
}

/// The caption every box 9 capture starts with: what it is and what it
/// cannot show.
const BOX_9_HEAD: &str = "drawn by `view::view` from a real AppState over the job pool \
     (crates/graft/tests/it/fold_by_default.rs, `wallet_release`: `main <- wallet-release <- \
     sara/fix-toc`, PR 16021 merged with its branch deleted). Judge beside \
     prototypes/ease-mocks/mergewright-main-screen.html. Not a pty capture: a character grid, \
     colour unverified (CLAUDE.md rule 5); the look is yours to judge (rule 6).";

/// Box 9: the main screen folded at first paint, and with `space` on the
/// PR 16021 merge, at 120 columns, every count landed.
#[test]
fn capture_main_folded_and_opened_at_120() {
    let scratch = wallet_release();
    let (app, state) = first_paint(scratch.path());
    let tails = |frame: &[String]| {
        [
            "\u{25b8} 4 commits from PR 16021",
            "1 commit from sara/fix-toc",
        ]
        .iter()
        .all(|tail| frame.iter().any(|line| line.contains(tail)))
    };
    let state = app.pump(state, "both counts", |state| tails(&drawn(state)));
    let state = crate::captures::at_rest(&app, state);
    let folded = drawn(&state);
    assert!(tails(&folded), "{}", folded.join("\n"));
    crate::captures::check_shot(
        "ease-pass/main-folded-120.txt",
        &format!(
            "ease-pass 06 box 9, 120x36, main screen at first paint: {BOX_9_HEAD}\n\
             What right looks like: no row of a merged branch; the PR's merge ends \
             `▸ 4 commits from PR 16021`, the named merge `▸ 1 commit from sara/fix-toc`; \
             the PR's merge draws `◉` (the named merge is `HEAD`, `◆`); the header counts \
             the shown commits.\n\n{}\n",
            folded.join("\n")
        ),
    );

    let y = folded
        .iter()
        .position(|line| line.contains(PR_MERGE))
        .expect("the PR merge is drawn");
    let state = app.mouse_down(
        &state,
        MouseButton::Left,
        30,
        u16::try_from(y).expect("short"),
    );
    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.pump(state, "the opened merge, counted", |state| {
        let frame = drawn(state);
        PR_COMMITS
            .iter()
            .all(|subject| frame.iter().any(|line| line.contains(subject)))
            && frame
                .iter()
                .any(|line| line.contains("\u{25be} 4 commits from PR 16021"))
    });
    let state = crate::captures::at_rest(&app, state);
    let opened = drawn(&state);
    crate::captures::check_shot(
        "ease-pass/main-opened-120.txt",
        &format!(
            "ease-pass 06 box 9, 120x36, `space` on the PR 16021 merge: {BOX_9_HEAD}\n\
             What right looks like: the merge's tail turns `▾ 4 commits from PR 16021`; its \
             four commits appear under it on one lane of their own that joins the merge row; \
             the cursor stays on the merge; `sara/fix-toc`'s merge stays folded.\n\n{}\n",
            opened.join("\n")
        ),
    );
    drop(state);
    app.close();
}
