//! The search's folded count asked again on every view change (ease-pass
//! 06h, gap `ease/06c-suffix-stale`). Split from `graph_mode`.

use super::graph_mode::{WALK, oid, one_request, refs_with_stash_at};
use super::*;
use crate::config::MergedBranches;
use crate::history::FoldedAsk;
use crate::update::keys::view_change::switch_graph_mode;
use crate::update::tests::reveal::{asked_folded, folded_hits};

/// A search finished with four hits, its first count answered 3.
fn searched(mut state: AppState) -> (AppState, Vec<Oid>) {
    let query = crate::search::Query::parse("login").expect("a query");
    let mut search = Search::new(query, 0, (0, 0));
    let hits: Vec<Oid> = [1, 4, 5, 6].map(oid).to_vec();
    search.batches = vec![Arc::from(hits.clone())];
    search.state = SearchState::Done { hits: 4 };
    search.current = Some(0);
    state.search = Some(search);
    (state, hits)
}

fn suffix(state: &AppState) -> String {
    let search = state.search.as_ref().expect("the search");
    let line = search.line(&state.keymap);
    line.split(" (")
        .next()
        .expect("a line")
        .trim_start_matches("/login  ")
        .to_owned()
}

/// One view change on `state`: exactly one row request, carrying a new ask
/// over every hit; the walk's count `folded` for it is the line's.
fn changed_and_counted(
    state: &AppState,
    hits: &[Oid],
    change: &dyn Fn(&AppState) -> (AppState, Vec<Command>),
    folded: usize,
) -> (AppState, u64) {
    let before = state.search.as_ref().and_then(|search| search.folded_ask);
    let (state, commands) = change(state);
    let request = one_request(&commands);
    let ask: FoldedAsk = request.folded.clone().expect("the ask rides on it");
    assert_eq!(&*ask.ids, hits);
    assert!(Some(ask.search) > before, "a new ask");
    let (state, _) = update(&state, folded_hits(WALK, ask.search, folded, 0));
    (state, ask.search)
}

/// Gap `ease/06c-suffix-stale`: every view change asks the count again on
/// its own request, and the line follows the new view's answer.
#[test]
fn every_view_change_asks_the_folded_count_again() {
    let (state, _) = merge_on_screen(WALK, 1);
    let (state, hits) = searched(state);
    let (state, _) = update(&state, folded_hits(WALK, 0, 3, 0));
    let press = |code| move |state: &AppState| update(state, key(code));
    let switch = |to| {
        move |state: &AppState| {
            let mut next = state.clone();
            let commands = switch_graph_mode(&mut next, to);
            (next, commands)
        }
    };

    let (state, opened) = changed_and_counted(&state, &hits, &press(KeyCode::Char(' ')), 1);
    assert_eq!(suffix(&state), "4 matches \u{b7} 1 in folded merges");
    let (state, _) = changed_and_counted(&state, &hits, &press(KeyCode::Char(' ')), 3);
    assert_eq!(suffix(&state), "4 matches \u{b7} 3 in folded merges");
    let (state, _) = update(&state, folded_hits(WALK, opened, 1, 0));
    assert_eq!(
        suffix(&state),
        "4 matches \u{b7} 3 in folded merges",
        "a superseded ask's answer is dropped"
    );
    let (state, _) = changed_and_counted(&state, &hits, &press(KeyCode::Char('1')), 0);
    assert_eq!(suffix(&state), "4 matches");
    let (state, _) = changed_and_counted(&state, &hits, &press(KeyCode::Char('1')), 3);
    assert_eq!(suffix(&state), "4 matches \u{b7} 3 in folded merges");
    let (state, _) = changed_and_counted(&state, &hits, &switch(MergedBranches::Shown), 0);
    assert_eq!(suffix(&state), "4 matches");
    let (state, _) = changed_and_counted(&state, &hits, &switch(MergedBranches::Folded), 3);
    assert_eq!(suffix(&state), "4 matches \u{b7} 3 in folded merges");
}

/// No search, or one still running: a view change carries no ask.
#[test]
fn a_view_change_with_no_finished_search_asks_nothing() {
    let (state, _) = merge_on_screen(WALK, 1);
    let (_, commands) = update(&state, key(KeyCode::Char(' ')));
    assert_eq!(asked_folded(&commands), None);

    let (mut running, _) = searched(state);
    if let Some(search) = running.search.as_mut() {
        search.state = SearchState::Running { hits: 4 };
    }
    let (_, commands) = update(&running, key(KeyCode::Char(' ')));
    assert_eq!(asked_folded(&commands), None);
}

/// The refs read's view change (a stash entry learnt) carries the ask too;
/// with no search it carries none.
#[test]
fn a_refs_read_view_change_asks_the_folded_count_again() {
    let refs = || AppEvent::Produced(JobId::new(3), Product::Refs(Ok(refs_with_stash_at(oid(9)))));
    let (state, _) = merge_on_screen(WALK, 1);
    let (_, commands) = update(&state, refs());
    assert_eq!(one_request(&commands).folded, None);

    let (state, hits) = searched(state);
    let (_, commands) = update(&state, refs());
    let ask = one_request(&commands).folded.expect("the ask rides on it");
    assert_eq!(&*ask.ids, hits.as_slice());
}
