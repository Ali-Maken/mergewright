//! ease-pass 06c (search folded count): `/totals` on `wallet_release`
//! through the real app. The PR's merge is a hit and is shown; its four
//! commits are hits the default fold hides. The search counts all five and
//! says, before any `n`, that four are in folded merges.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use mergewright::search::SearchState;
use mergewright::session::Clock;
use mergewright::update::AppState;
use mergewright_core::event::KeyCode;

use crate::app_harness::App;
use crate::captures::frame_at;
use crate::fold_by_default::wallet_release;

/// After every commit of [`wallet_release`], so ages read the same.
const NOW: i64 = 1_700_100_000;
/// The PR's merge, its branch deleted: the first hit, and shown.
const PR_MERGE: &str = "Merged PR 16021: Fix the wallet totals";

/// Types `/query⏎` and pumps until the search has finished and the walk has
/// answered how many of its hits are in folded merges.
fn searched_and_counted(app: &App, state: &AppState, query: &str) -> AppState {
    let mut state = app.key(state, KeyCode::Char('/'));
    for c in query.chars() {
        state = app.key(&state, KeyCode::Char(c));
    }
    let state = app.key(&state, KeyCode::Enter);
    app.pump(
        state,
        "the search finished, its fold count answered",
        |state| {
            let done = state
                .search
                .as_ref()
                .is_some_and(|search| matches!(search.state, SearchState::Done { .. }));
            let answered = state
                .history
                .as_ref()
                .is_some_and(|history| !history.folded_ask_out() && history.seeking.is_none());
            done && answered
        },
    )
}

#[test]
fn capture_search_folded_count_at_120() {
    let scratch = wallet_release();
    let (app, state) = App::open(scratch.path());
    let state = app.resize(&state, 120, 36);
    let state = crate::captures::at_rest(&app, state);

    let state = searched_and_counted(&app, &state, "totals");

    let search = state.search.as_ref().expect("the search stands");
    assert_eq!((search.hits(), search.folded), (5, 4), "every hit counted");
    let history = state.history.as_ref().expect("a walk");
    let selected = history.selected().expect("a row under the cursor");
    assert!(
        selected.commit.message.starts_with(PR_MERGE.as_bytes()),
        "the cursor on the first hit, the PR's merge"
    );
    let drawn = frame_at(&state, 120, 36, Some(Clock::Fixed(NOW)));
    assert!(
        drawn[0].contains("/totals  5 matches \u{b7} 4 in folded merges (1/5)"),
        "{:?}",
        drawn[0]
    );
    // The closed tail read at his full-screen 187: at 120 beside the branch
    // pane it gives way to the subject (ease-pass 09 ruling W).
    let wide = frame_at(&state, 187, 36, Some(Clock::Fixed(NOW)));
    assert!(
        wide.iter()
            .any(|line| crate::row_reads::reads_subject(line, PR_MERGE)
                && line.contains("\u{25b8} 4 commits from PR 16021")),
        "nothing opened before `n`:\n{}",
        wide.join("\n")
    );
    app.close();
    crate::captures::check_shot(
        "ease-pass/main-search-folded-120.txt",
        &format!(
            "ease-pass 06c, 120x36, `/totals` then Enter on the main screen, before any `n`: drawn \
             by `view::view` from a real AppState over the job pool \
             (crates/mergewright/tests/it/search_folded.rs, `wallet_release`: PR 16021's four \
             `totals: …` commits folded behind its merge). Not a pty capture: a character grid, \
             colour unverified (CLAUDE.md rule 5); the look is yours to judge (rule 6).\n\
             What right looks like: the header's search words read `/totals  5 matches · 4 in \
             folded merges (1/5)` — every hit counted, the four behind the folded merge said \
             once; the cursor on the first hit, the PR's merge, still folded (`▸ 4 commits from \
             PR 16021`). At this width the header's `2 merged branches folded` gives way to the \
             search (the header drops its lane words first).\n\n{}\n",
            drawn.join("\n")
        ),
    );
}
