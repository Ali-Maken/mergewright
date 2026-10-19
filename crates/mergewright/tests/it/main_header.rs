//! The top line's lane words through the real app (ease-pass 06b, boxes 9
//! and 10): `lanes drawn: N of M · K merged branches folded`, counted over
//! the rows in view, on a repository shaped like his work repo with one
//! branch still open whose lane passes through the window.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use mergewright::session::Clock;
use mergewright::update::AppState;
use mergewright_core::event::KeyCode;

use crate::app_harness::App;
use crate::captures::{at_rest, check_shot, frame_at};
use crate::fold_by_default::wallet_release;

const ROWS: u16 = 36;
/// After every commit of [`wallet_release_wide`], so ages read the same.
const NOW: i64 = 1_700_100_000;
/// `wallet-release`'s commits after the named merge and before the ledger's
/// tip: more than the window holds, so the ledger's tip can sit above it.
const EARLY: u32 = 40;
/// The open branch's one commit, and the first row the window must not hold.
const LEDGER_TIP: &str = "ledger: sketch the export";
/// The ledger's fork point, below the window.
const FORK: &str = "Add the readme";
/// The lower of the two folded merges.
const PR_MERGE: &str = "Merged PR 16021: Fix the wallet totals";

/// [`wallet_release`], unchanged, then: [`EARLY`] commits on
/// `wallet-release`, `feature/ledger` forked from `main` (the readme) with
/// one commit, and two more on `wallet-release` (`HEAD`). The ledger is
/// never merged: its lane runs from its tip near the top down to the readme
/// at the bottom, past every row between. Each step a minute after the
/// last, through the fixtures' hermetic git.
fn wallet_release_wide() -> tempfile::TempDir {
    let scratch = wallet_release();
    let repo = scratch.path();
    let mut at: i64 = 1_700_001_000;
    let mut step = |args: &[&str]| {
        at += 60;
        mergewright_fixtures::git::run_at(repo, args, &format!("@{at} +0000"))
            .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    };
    for note in 1..=EARLY {
        let subject = format!("release: note {note:02}");
        step(&["commit", "--quiet", "--allow-empty", "-m", &subject]);
    }
    step(&["switch", "--quiet", "-c", "feature/ledger", "main"]);
    step(&["commit", "--quiet", "--allow-empty", "-m", LEDGER_TIP]);
    step(&["switch", "--quiet", "wallet-release"]);
    for subject in ["release: tag the candidate", "release: bump the version"] {
        step(&["commit", "--quiet", "--allow-empty", "-m", subject]);
    }
    scratch
}

fn drawn(state: &AppState, columns: u16) -> Vec<String> {
    frame_at(state, columns, ROWS, Some(Clock::Fixed(NOW)))
}

/// Opened at 120×36, walked, then `j` down until the window holds both
/// folded merges and neither the ledger's tip (above) nor its fork point
/// (below): the ledger's lane passes through with no commit in view.
fn scrolled_between(repo: &Path) -> (App, AppState) {
    let (app, state) = App::open(repo);
    let state = app.resize(&state, 120, ROWS);
    let mut state = at_rest(&app, state);
    let between = |frame: &[String]| {
        let holds = |subject: &str| frame.iter().any(|line| line.contains(subject));
        holds(PR_MERGE) && !holds(LEDGER_TIP) && !holds(FORK)
    };
    // Bounded, so a fixture that never reaches the window fails here.
    for _ in 0..2 * EARLY {
        if between(&drawn(&state, 120)) {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    let frame = drawn(&state, 120);
    assert!(between(&frame), "{}", frame.join("\n"));
    // The merges' counts are read once they come into view. Their tails
    // give way to the subjects beside the branch pane (ease-pass 09 ruling
    // W), so the wait reads the state, not the frame.
    let state = app.pump(
        state,
        "both counts",
        mergewright::update::fold_count::settled,
    );
    (app, state)
}

/// The header row: the first line the screen draws.
fn top_line(frame: &[String]) -> &str {
    frame
        .iter()
        .find(|line| !line.trim().is_empty())
        .expect("a header")
}

/// Boxes 9 and 10: N < M (the ledger's lane is open and holds no commit
/// in view) and K = 2 (both merges folded, both in view), at 120 and 90.
#[test]
fn capture_main_header_at_120_and_90() {
    let scratch = wallet_release_wide();
    let (app, state) = scrolled_between(scratch.path());

    let wide = drawn(&state, 120);
    let top = top_line(&wide);
    assert!(
        top.trim_end()
            .ends_with("47 commits  lanes drawn: 1 of 2 \u{b7} 2 merged branches folded"),
        "{top:?}"
    );
    assert_eq!(top.matches("wallet-release").count(), 1, "{top:?}");
    assert!(!top.contains(" on "), "{top:?}");
    assert!(
        !top.contains('+') && !top.contains("merges folded"),
        "{top:?}"
    );
    check_shot(
        "ease-pass/main-header-120.txt",
        &format!(
            "ease-pass 06b box 9, 120x36, the main screen scrolled so `feature/ledger`'s lane \
             passes through the window with its commit above it and its fork point below; \
             `wallet_release_wide` (crates/mergewright/tests/it/main_header.rs) drawn by \
             `view::view` from a real AppState. Not a pty capture: a character grid, colour \
             unverified (CLAUDE.md rule 5); the look is yours to judge (rule 6).\n\
             What right looks like: the top line ends `lanes drawn: 1 of 2 \u{b7} 2 merged \
             branches folded` — two columns open in view, only `wallet-release`'s holds a \
             commit, so the ledger's lane is not drawn; both merges in view are folded. \
             `wallet-release` appears once on the top line, no `on`, no `+N lanes`.\n\n{}\n",
            wide.join("\n")
        ),
    );

    let narrow = drawn(&state, 90);
    let top = top_line(&narrow);
    assert_eq!(
        top.trim_end(),
        " \u{25c6} api  \u{2026}api \u{b7} wallet-release  47 commits  lanes drawn: 1 of 2 \
         \u{b7} 2 merged branches folded"
    );
    check_shot(
        "ease-pass/main-header-90.txt",
        &format!(
            "ease-pass 06b box 10, 90x36, the same repository and scroll as \
             main-header-120.txt. Character grid, colour unverified (rule 5); yours to judge \
             (rule 6).\n\
             What right looks like: both lane phrases still fit; the path gives way first \
             (cut to `\u{2026}api`), the branch name stays whole. Tail room at 90 is 81 \
             cells, 64 once ` \u{b7} wallet-release` (17) is kept; the tail is 58.\n\n{}\n",
            narrow.join("\n")
        ),
    );
    drop(state);
    app.close();
}
