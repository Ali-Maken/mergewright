//! The bar per row kind through the real app (ease-pass/07b, mock G13):
//! `view::rows::context::cursor_row_context` picks each from the row's
//! fold and its live branch, over `fold_by_default`'s `wallet_release`
//! (`main ← wallet-release ← sara/fix-toc`, PR 16021 merged with its
//! branch deleted). Words are 03b/03r's.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use mergewright::update::AppState;
use mergewright_core::event::{KeyCode, MouseButton};

use crate::app_harness::App;
use crate::fold_by_default::{drawn, first_paint, row_of, wallet_release, window};

const PR_MERGE: &str = "Merged PR 16021: Fix the wallet totals";
const PR_TIP: &str = "totals: test the sum";
const TOC_COMMIT: &str = "toc: fix the anchors";

/// The first paint, with the refs read: a row's bar names its branch.
fn opened(repo: &std::path::Path) -> (App, AppState) {
    let (app, state) = first_paint(repo);
    let state = app.pump(state, "the refs", |state| state.refs.is_some());
    (app, state)
}

/// The bar: the frame's last line.
fn bar(state: &AppState) -> String {
    drawn(state).last().cloned().unwrap_or_default()
}

/// Clicks the drawn row whose line holds `subject`.
fn click(app: &App, state: &AppState, subject: &str) -> AppState {
    let frame = drawn(state);
    let y = crate::row_reads::row_reading(&frame, subject).map_or_else(
        || panic!("{subject:?} is drawn\n{}", frame.join("\n")),
        |(y, _)| y,
    );
    app.mouse_down(
        state,
        MouseButton::Left,
        30,
        u16::try_from(y).expect("short"),
    )
}

/// The subject of the row the cursor is on.
fn selected(state: &AppState) -> String {
    let row = state
        .history
        .as_ref()
        .and_then(mergewright::history::History::selected)
        .expect("a selected row");
    String::from_utf8_lossy(&row.commit.message)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// `space` on the cursor's merge, pumped until `subject` is drawn.
fn open(app: &App, state: &AppState, subject: &str) -> AppState {
    let state = app.key(state, KeyCode::Char(' '));
    app.pump(state, "the merged side drawn", |state| {
        drawn(state).iter().any(|line| line.contains(subject))
    })
}

#[test]
fn his_own_row_and_the_trunks_row_take_their_branchs_bar() {
    let scratch = wallet_release();
    let (app, state) = opened(scratch.path());
    let own = click(&app, &state, "Start the wallet release");
    assert!(
        // An ancestor of `HEAD`: `o` would refuse it, so no `o take`
        // (pre-publish-cleanup/13, review M1).
        bar(&own).starts_with(" ⏎ open · P push · c commit · "),
        "{:?}",
        bar(&own)
    );
    // `main`'s row, made on `main`: the trunk's bar.
    let trunk = click(&app, &own, "Add the readme");
    assert!(
        bar(&trunk)
            .starts_with(" m merge main into yours · ⏎ open · C switch to main · x compare · "),
        "{:?}",
        bar(&trunk)
    );
    app.close();
}

#[test]
fn a_folded_merge_with_its_count_says_what_space_shows() {
    let scratch = wallet_release();
    let (app, state) = opened(scratch.path());
    let state = click(&app, &state, PR_MERGE);
    let state = app.pump(state, "the fold's count", |state| {
        bar(state).starts_with(" space show the 4 commits inside · ")
    });
    assert!(
        bar(&state).starts_with(" space show the 4 commits inside · ⏎ open · x compare · "),
        "{:?}",
        bar(&state)
    );
    app.close();
}

#[test]
fn the_opened_merge_and_its_rows_from_a_deleted_branch() {
    let scratch = wallet_release();
    let (app, state) = opened(scratch.path());
    let state = click(&app, &state, PR_MERGE);
    let state = open(&app, &state, PR_TIP);
    assert!(
        bar(&state).starts_with(" space hide the commits inside · ⏎ open · x compare · "),
        "{:?}",
        bar(&state)
    );
    // An inner row whose merged branch was deleted: no live branch, so no
    // `C switch to`, and no `space` hint (the mock draws none).
    let inner = click(&app, &state, PR_TIP);
    assert_eq!(selected(&inner), PR_TIP);
    let words = bar(&inner);
    assert!(!words.contains("C switch"), "{words:?}");
    assert!(!words.contains("space"), "{words:?}");
    // `space` there still closes the merge, the cursor on its row.
    let closed = app.key(&inner, KeyCode::Char(' '));
    let closed = app.pump(closed, "the side folded again", |state| {
        drawn(state).iter().all(|line| !line.contains(PR_TIP))
    });
    assert_eq!(selected(&closed), PR_MERGE);
    app.close();
}

#[test]
fn an_inner_row_of_a_live_branchs_merge_offers_its_switch() {
    let scratch = wallet_release();
    let (app, state) = opened(scratch.path());
    let state = click(&app, &state, "Merge branch 'sara/fix-toc'");
    let state = open(&app, &state, TOC_COMMIT);
    let inner = click(&app, &state, TOC_COMMIT);
    let at = row_of(window(&inner).expect("a window"), TOC_COMMIT).0;
    assert!(
        window(&inner).expect("a window").fold[at].inside.is_some(),
        "the toc commit is inside the opened merge"
    );
    let words = bar(&inner);
    assert!(
        words.starts_with(
            " m merge fix-toc into yours · ⏎ open · C switch to fix-toc · x compare · "
        ),
        "{words:?}"
    );
    assert!(!words.contains("space"), "{words:?}");
    app.close();
}
