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
         # settled — `… (1/N)  n/N next/prev  esc close`; no match — `no matches  esc close`.\n\
         # Since m8/15b the line is the header's tail and the bar is the row's keys, led by\n\
         # `n next · N prev` while the search has a hit.\n",
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

/// The capture's header for [`after_enter_the_bar_is_the_rows_keys_and_the_search_is_in_the_header`].
const AFTER_SEARCH_NOTE: &str = "# m8-bar-after-search-120x36: drawn by `view::view` from a real AppState\n\
    # (crates/graft/tests/it/search_hint.rs). Not a pty capture: a character grid,\n\
    # no colour (CLAUDE.md rule 5 — colour unverified; a failed search's warning\n\
    # style in the header is unverified too).\n\
    #\n\
    # The corpus octopus, shipped keymap, `/topic ⏎` (m8/15b, ticket 15 A2). Right:\n\
    # after Enter — the header's tail leads with the search's line\n\
    # (`/topic  … (1/N)  n/N next/prev  esc close`), the bottom row is the row's\n\
    # keys led by `n next · N prev`; after Esc — the search is gone from both.\n";

/// m8-polish-and-ship/15b (ticket 15 A2): after `/topic ⏎` the search leaves
/// the footer. The bar is the row's own keys led by `n next · N prev`; the
/// search's line, worded as the footer drew it, is the header's tail. `Esc`
/// ends the search and both go. Asserts and writes
/// `docs/shots/m8-bar-after-search-120x36.txt`.
#[test]
fn after_enter_the_bar_is_the_rows_keys_and_the_search_is_in_the_header() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::OctopusMerge);
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let state = app.resize(&state, 120, 36);

    let settled = searched(&app, &state, "topic");
    let search = settled.search.as_ref().expect("the search stands");
    assert!(search.hits() > 0, "the corpus has `topic` commits");
    let line = search.line(graft_ui::keymap::Keymap::shipped());
    let after = crate::captures::frame_at(&settled, 120, 36, Some(CLOCK));
    let header = &after[0];
    let footer = after.last().expect("a footer row");
    assert!(
        header.contains(&line),
        "the header holds the search: {header:?}"
    );
    assert!(
        footer.starts_with(" n next \u{b7} N prev \u{b7} "),
        "the bar leads with n N: {footer:?}"
    );
    assert!(
        footer.contains("\u{23ce} open"),
        "and the row's own keys follow: {footer:?}"
    );
    assert!(!footer.contains("/topic"), "{footer:?}");

    let closed = app.key(&settled, KeyCode::Escape);
    assert!(closed.search.is_none(), "Esc ends the search");
    let cleared = crate::captures::frame_at(&closed, 120, 36, Some(CLOCK));
    assert!(!cleared[0].contains("/topic"), "{:?}", cleared[0]);
    let bar = cleared.last().expect("a footer row");
    assert!(
        !bar.contains("n next") && !bar.contains("N prev"),
        "{bar:?}"
    );

    // No match: `n`/`N` step nowhere, so the bar does not offer them; the
    // search still stands in the header.
    let none = searched(&app, &closed, "zzqx");
    let missed = crate::captures::frame_at(&none, 120, 36, Some(CLOCK));
    assert!(missed[0].contains("/zzqx  no matches"), "{:?}", missed[0]);
    let bar = missed.last().expect("a footer row");
    assert!(
        !bar.contains("n next") && !bar.contains("N prev"),
        "{bar:?}"
    );

    let mut out = String::from(AFTER_SEARCH_NOTE);
    for (name, drawn) in [("after Enter", &after), ("after Esc", &cleared)] {
        out.push_str("\n## ");
        out.push_str(name);
        out.push_str("\n\n");
        for row in drawn {
            out.push_str(row);
            out.push('\n');
        }
    }
    app.close();
    crate::captures::check_shot("m8-bar-after-search-120x36.txt", &out);
}
