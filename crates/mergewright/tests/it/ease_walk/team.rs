//! G12 on 01r's team topologies (ticket 14 Scope, "G12 on `team`"), text
//! only, and the two captures from them.
//!
//! Every branch shown: by the 2026-10-18 fold rule (a) `sara/fix-toc` is
//! merged into `bob/x`'s sync merge and folds under it at the default, and
//! an opened row takes the merge row's branch — so no row of hers would be
//! a row the cursor rests on as hers. `[graph] merged_branches = "shown"`
//! draws her own rows; the pane's answers are the same either way (they are
//! read from git, not from the fold).

use std::path::Path;

use mergewright::config::MergedBranches;
use mergewright::update::AppState;

use super::fixture::{self, team};
use super::screen::{ROWS, cell, cursor_on, drawn, node_column, pane, shown};
use crate::app_harness::App;

/// The team repository opened with every branch shown, at 120 × [`ROWS`],
/// pumped to rest.
pub fn open_every_branch(repo: &Path) -> (App, AppState) {
    let (app, mut state) = App::open(repo);
    state.graph_mode = MergedBranches::Shown;
    let state = app.resize(&state, 120, ROWS);
    let state = app.pump(state, "the first rows and the refs", |state| {
        state.refs.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    });
    let state = crate::captures::at_rest(&app, state);
    (app, state)
}

/// The pane's `built on` route, one stop a line, as drawn.
fn route(lines: &[String]) -> Vec<String> {
    let at = lines
        .iter()
        .position(|line| line == "built on")
        .unwrap_or_else(|| panic!("no built on: {lines:#?}"));
    lines[at + 1..]
        .iter()
        .take_while(|line| !line.is_empty())
        .cloned()
        .collect()
}

fn on_sara(moved: bool) -> (tempfile::TempDir, App, AppState) {
    let scratch = team(moved);
    let (app, state) = open_every_branch(scratch.path());
    let state = cursor_on(&app, state, fixture::TOC_TWO);
    (scratch, app, state)
}

fn shot(name: &str, right: &str, frame: &[String]) {
    crate::captures::check_shot(
        &format!("ease-pass/{name}"),
        &format!(
            "ease-pass 14, 120x{ROWS}, 01r's team repository, every branch shown, cursor on \
             `toc: number the headings` (sara/fix-toc).\nWhat right looks like: {right} Colour \
             unverified (rule 5).\n\n{}\n",
            frame.join("\n")
        ),
    );
}

/// Sibling never built on: wallet-release moved after `ali/x` and sara were
/// cut from it, so by Q15 (a) her route is `main ─◆ develop ─◆
/// sara/fix-toc`, naming neither `ali/x` nor `wallet-release`; `ali/x`'s
/// lane on her row reads `┊`.
#[test]
fn g12_sibling_never_built_on() {
    let (_scratch, app, state) = on_sara(true);
    let frame = drawn(&state);
    let lines = pane(&frame, 120);
    assert_eq!(
        route(&lines),
        ["main", "─◆ develop", "─◆ sara/fix-toc"],
        "{lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("ali/x")),
        "{lines:#?}"
    );
    let ali = node_column(&frame, fixture::ALI);
    assert_eq!(
        cell(&frame, fixture::TOC_TWO, ali),
        '┊',
        "{}",
        shown(&frame)
    );
    shot(
        "main-pane-sibling-120.txt",
        "the pane on sara reads `sara/fix-toc`, then four lines marked `!` `·` `✓` `·`, then \
         `built on` `main` `─◆ develop` `─◆ sara/fix-toc` — never `ali/x`, never \
         `wallet-release` (it moved on: Q15 a); the merged line reads `not merged`, not \
         `merged into bob/x`; `ali/x`'s lane on sara's row is `┊`.",
        &frame,
    );
    app.close();
}

/// A teammate's sync is not a merge: bob cut from sara's older commit,
/// committed, then merged hers in; her merged line still reads `not
/// merged`, never `merged into bob/x`.
#[test]
fn g12_teammate_sync_is_not_a_merge() {
    let (_scratch, app, state) = on_sara(true);
    let lines = pane(&drawn(&state), 120);
    assert!(
        lines.iter().any(|line| line == "· not merged"),
        "{lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("merged into")),
        "{lines:#?}"
    );
    app.close();
}

/// Dependent, before the move: wallet-release has nothing sara lacks, so
/// her route names it: `main ─◆ develop ─◆ wallet-release ─◆ sara/fix-toc`.
#[test]
fn g12_dependent_before_the_move() {
    let (_scratch, app, state) = on_sara(false);
    let frame = drawn(&state);
    let lines = pane(&frame, 120);
    assert_eq!(
        route(&lines),
        ["main", "─◆ develop", "─◆ wallet-release", "─◆ sara/fix-toc"],
        "{lines:#?}"
    );
    shot(
        "main-pane-dependent-120.txt",
        "taken before wallet-release's extra commit: the pane's `built on` reads `main` `─◆ \
         develop` `─◆ wallet-release` `─◆ sara/fix-toc` (all of wallet-release sits under \
         hers); the merged line reads `not merged`.",
        &frame,
    );
    app.close();
}

/// G12's `(you're here)`: on `HEAD`'s own row the heading carries it.
#[test]
fn g12_heading_says_youre_here_on_head() {
    let scratch = team(true);
    let (app, state) = open_every_branch(scratch.path());
    let state = cursor_on(&app, state, fixture::MORE_RELEASE);
    let lines = pane(&drawn(&state), 120);
    assert_eq!(lines[0], "wallet-release  (you're here)", "{lines:#?}");
    app.close();
}
