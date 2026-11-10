//! pre-publish-cleanup/04, Part A: the branch pane's focused branch, row by
//! row, through the real update path on the main screen's stacked repository
//! (`feature/receipts` built on `wallet-release`, `sara/fix-toc` merged in
//! and folded under the merge until `space` opens it).
//!
//! The expected table is written by hand from how the fixture builds each
//! commit and ADR-0064's rule ("a lane takes a reached tip's name from that
//! row down; straight on"), never read from the code under test.

use mergewright::update::AppState;
use mergewright::update::branch_pane::focused;
use mergewright_core::event::KeyCode;

use super::{NAMED_MERGE, main_screen_repo, opened};
use crate::captures::at_rest;

/// Commit subject → the branch the pane must describe on that row.
const EXPECTED: [(&str, &str); 6] = [
    // HEAD: feature/receipts' own commit, its tip.
    ("Draw the receipts list", "refs/heads/feature/receipts"),
    // wallet-release's tip: the lane takes its name from here down.
    (NAMED_MERGE, "refs/heads/wallet-release"),
    // Below wallet-release's tip on its lane, straight on.
    ("Start the wallet release", "refs/heads/wallet-release"),
    ("Try a read cache", "refs/heads/spike/cache"),
    ("Pin the CI image", "refs/heads/ops/ci"),
    ("Add the readme", "refs/heads/main"),
];

fn cursor_subject(state: &AppState) -> String {
    let history = state.history.as_ref().expect("history");
    let window = history.window.as_ref().expect("a window");
    let row = &window.rows[history.cursor - window.first];
    String::from_utf8_lossy(&row.commit.message)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// Measurement only (`scripts/pp-04-pane-focus.sh`): when the script's
/// `git` shim logs spawns to `MWR_SPAWN_LOG`, a marker line between focus
/// changes splits the log per focus, and each change's settle time goes to
/// `MWR_FOCUS_TIMES`. Unset in the fast tier: nothing is written.
fn mark(step: usize, elapsed: std::time::Duration) {
    use std::io::Write;
    for (var, line) in [
        ("MWR_SPAWN_LOG", format!("# focus {step}")),
        (
            "MWR_FOCUS_TIMES",
            format!("{:.1}", elapsed.as_secs_f64() * 1000.0),
        ),
    ] {
        if let Some(path) = std::env::var_os(var) {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .expect("the measuring script's log opens");
            writeln!(file, "{line}").expect("the measuring script's log takes a line");
        }
    }
}

/// `j` from the top row to the bottom; after each settle the pane's focused
/// branch is the one the hand-written table names for the cursor's commit.
#[test]
fn the_pane_focuses_the_hand_expected_branch_on_every_stacked_row() {
    let scratch = main_screen_repo();
    let (app, mut state) = opened(scratch.path());
    mark(0, std::time::Duration::ZERO);
    while state.history.as_ref().is_some_and(|h| h.cursor > 0) {
        state = at_rest(&app, app.key(&state, KeyCode::Char('k')));
    }
    let mut seen = Vec::new();
    let mut step = 0;
    loop {
        let subject = cursor_subject(&state);
        let wanted = EXPECTED
            .iter()
            .find(|(row, _)| *row == subject)
            .unwrap_or_else(|| panic!("row {subject:?} is not in the table"))
            .1;
        let got = focused(&state).map(|it| it.branch.name);
        assert_eq!(
            got.as_deref(),
            Some(wanted),
            "row {step} {subject:?}: pane focuses {got:?}, table says {wanted}"
        );
        seen.push(subject);
        let before = state.history.as_ref().map(|h| h.cursor);
        let started = std::time::Instant::now();
        let next = at_rest(&app, app.key(&state, KeyCode::Char('j')));
        if next.history.as_ref().map(|h| h.cursor) == before {
            break;
        }
        step += 1;
        mark(step, started.elapsed());
        state = next;
    }
    // Every laid-out row was walked: the fold may hide a merged side
    // branch's rows (here `toc: fix the anchors`), and those have no cursor.
    let window = state
        .history
        .as_ref()
        .and_then(|h| h.window.as_ref())
        .expect("a window");
    assert_eq!(
        seen.len(),
        window.rows.len(),
        "every drawn row walked: {seen:?}"
    );
    assert!(
        seen.len() >= 6,
        "the stacked chain and its neighbours: {seen:?}"
    );

    drop(state);
    app.close();
}

/// The folded side branch, opened: the row inside the merge keeps the
/// merge's branch in focus (G4).
#[test]
fn inside_the_opened_merge_the_pane_keeps_the_merges_branch() {
    let scratch = main_screen_repo();
    let (app, mut state) = opened(scratch.path());
    // The folded side branch: `space` on the merge opens it, `j` reaches
    // sara/fix-toc's own commit. A row inside an opened merge focuses the
    // branch the merge was made on (G4, `view/entries/roles.rs` `focus`), so
    // the pane keeps describing wallet-release there, not sara/fix-toc.
    for _ in 0..super::ROWS {
        if cursor_subject(&state) == NAMED_MERGE {
            break;
        }
        state = at_rest(&app, app.key(&state, KeyCode::Char('j')));
    }
    assert_eq!(
        cursor_subject(&state),
        NAMED_MERGE,
        "the merge row is reachable"
    );
    state = at_rest(&app, app.key(&state, KeyCode::Char(' ')));
    state = at_rest(&app, app.key(&state, KeyCode::Char('j')));
    assert_eq!(
        cursor_subject(&state),
        "toc: fix the anchors",
        "the fold opened under the merge"
    );
    let got = focused(&state).map(|it| it.branch.name);
    assert_eq!(
        got.as_deref(),
        Some("refs/heads/wallet-release"),
        "inside the opened merge (G4)"
    );
    drop(state);
    app.close();
}
