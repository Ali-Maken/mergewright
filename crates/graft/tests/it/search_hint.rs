//! m8-polish-and-ship/22c (B2): the graph search's hint line as the app draws
//! it under the shipped keymap — running, settled with matches, no match.
//! Replaces nothing: the hand-shot `26-search-*` and `m2-18-git-git-search-*`
//! files are records of their tickets (TESTING.md "Captures").

use graft::search::SearchState;
use graft::update::AppState;
use graft_core::event::KeyCode;

use crate::app_harness::App;

const CLOCK: graft::session::Clock = graft::session::Clock::Fixed(graft_fixtures::CORPUS_NOW);

fn opened(app: &App, state: AppState) -> AppState {
    app.pump(state, "the first rows and the refs", |state| {
        state.refs.is_some()
            && state.working_tree.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    })
}

/// Types `/query⏎` and pumps until the search has finished.
fn searched(app: &App, state: &AppState, query: &str) -> AppState {
    let mut state = app.key(state, KeyCode::Char('/'));
    for c in query.chars() {
        state = app.key(&state, KeyCode::Char(c));
    }
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the search to finish", |state| {
        state
            .search
            .as_ref()
            .is_some_and(|search| matches!(search.state, SearchState::Done { .. }))
    });
    crate::captures::settled_frame(app, state)
}

#[test]
fn the_search_hint_draws_his_esc_spelling() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::OctopusMerge);
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let state = app.resize(&state, 120, 36);

    let settled = searched(&app, &state, "topic");
    // The running frame: the same search with its state held at Running and
    // the hits it found, so the frame is still (a fast-tier repo finishes
    // before a frame could catch it). Everything else is the settled state.
    let mut running = settled.clone();
    let search = running.search.as_mut().expect("the settled search is up");
    search.state = SearchState::Running {
        hits: search.hits(),
    };
    let closed = app.key(&settled, KeyCode::Escape);
    let none = searched(&app, &closed, "zzqx");

    let frames = [
        (
            "running",
            crate::captures::frame_at(&running, 120, 36, Some(CLOCK)),
        ),
        (
            "settled with matches",
            crate::captures::frame_at(&settled, 120, 36, Some(CLOCK)),
        ),
        (
            "no match",
            crate::captures::frame_at(&none, 120, 36, Some(CLOCK)),
        ),
    ];
    let mut out = String::from(
        "# m8/search-hint-120x36: drawn by `view::view` from a real AppState\n\
         # (crates/graft/tests/it/search_hint.rs). Not a pty capture: a character grid,\n\
         # no colour (CLAUDE.md rule 5 — colour unverified).\n\
         #\n\
         # The graph search on the corpus octopus, shipped keymap. Three frames. Right:\n\
         # running — `… matches so far …  n/N next/prev  esc cancel` (the running frame\n\
         # is the settled AppState with its search state set back to Running);\n\
         # settled — `… (1/N)  n/N next/prev  esc close`; no match — `no matches  esc close`.\n",
    );
    for (name, drawn) in &frames {
        out.push_str("\n## ");
        out.push_str(name);
        out.push_str("\n\n");
        for line in drawn {
            out.push_str(line);
            out.push('\n');
        }
    }
    app.close();
    crate::captures::check_shot("m8/search-hint-120x36.txt", &out);
}
