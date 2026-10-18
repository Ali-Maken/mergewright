//! `n`/`N` onto hits the default fold hides (ease-pass 06c): each opens
//! its merge and lands; a hit in a merge already opened lands at once; no
//! step reads as not in this view.

use std::sync::Arc;

use graft_core::event::KeyCode;
use graft_core::job::JobId;
use graft_core::oid::Oid;

use crate::history::Found;
use crate::search::{Query, Search, SearchState};
use crate::update::events::Command;
use crate::update::state::AppState;
use crate::update::tests::reveal::{answer, find_for, oid_of, on, revealed};
use crate::update::tests::{awaiting, key};
use crate::update::update;

const WALK: JobId = JobId::new(4242);

/// A fresh walk with a finished search whose hits are `hits`, walk order.
fn searched(hits: &[u8]) -> AppState {
    let mut state = awaiting(WALK);
    let query = Query::parse("login").expect("a non-empty query");
    let mut search = Search::new(query, 0, (0, 0));
    let ids: Vec<Oid> = hits.iter().copied().map(oid_of).collect();
    search.batches = vec![Arc::from(ids)];
    search.state = SearchState::Done { hits: hits.len() };
    state.search = Some(search);
    state
}

fn press(state: &AppState, letter: char) -> (AppState, Vec<Command>) {
    update(state, key(KeyCode::Char(letter)))
}

fn opened(state: &AppState) -> Vec<Oid> {
    let history = state.history.as_ref().expect("a walk");
    let opened = history.view.inclusion.opened().expect("a folding view");
    [0x06, 0x20]
        .map(oid_of)
        .into_iter()
        .filter(|merge| opened.contains(*merge))
        .collect()
}

/// Neither `(not in this view)` nor a seek still out.
fn landed_on(state: &AppState, hit: u8) {
    let search = state.search.as_ref().expect("the search");
    assert!(!search.missed, "not in this view at {hit:#x}");
    assert_eq!(search.seeking, None, "the seek is over");
    assert_eq!(on(state), Some(oid_of(hit)), "the cursor on {hit:#x}");
}

/// Both hits are on a PR merged at 0x20: the first opens the merge, the
/// second is then on hand and opens nothing new.
#[test]
fn n_onto_a_merged_prs_commit_opens_it_and_n_within_the_pr_opens_nothing_new() {
    let (state, commands) = press(&searched(&[0x11, 0x12]), 'n');
    let find = find_for(&commands, oid_of(0x11));
    let rows: (&[u8], &[u8]) = (&[0x30, 0x20, 0x10], &[0x30, 0x20, 0x11, 0x12, 0x10]);
    let (state, _) = revealed(&state, WALK, find, (oid_of(0x20), oid_of(0x11)), rows);
    landed_on(&state, 0x11);
    assert_eq!(opened(&state), [oid_of(0x20)]);

    let (state, _) = press(&state, 'n');

    landed_on(&state, 0x12);
    assert_eq!(opened(&state), [oid_of(0x20)], "nothing new opened");
}

/// Four hits in walk order: 0x30 shown, 0x11 and 0x12 behind 0x20, 0x05
/// behind 0x06. `N` from the first wraps to 0x05, then walks back.
#[test]
fn capital_n_from_the_first_wraps_and_reveals_each_hit_backwards() {
    let shown: &[u8] = &[0x30, 0x20, 0x10, 0x06, 0x01];
    let (state, commands) = press(&searched(&[0x30, 0x11, 0x12, 0x05]), 'n');
    let find = find_for(&commands, oid_of(0x30));
    let (state, _) = update(&state, answer(WALK, find, shown, Some(Found::Row)));
    landed_on(&state, 0x30);

    let (state, commands) = press(&state, 'N');
    let find = find_for(&commands, oid_of(0x05));
    let low: &[u8] = &[0x30, 0x20, 0x10, 0x06, 0x05, 0x01];
    let (state, _) = revealed(
        &state,
        WALK,
        find,
        (oid_of(0x06), oid_of(0x05)),
        (shown, low),
    );
    landed_on(&state, 0x05);

    let (state, commands) = press(&state, 'N');
    let find = find_for(&commands, oid_of(0x12));
    let both: &[u8] = &[0x30, 0x20, 0x11, 0x12, 0x10, 0x06, 0x05, 0x01];
    let (state, _) = revealed(
        &state,
        WALK,
        find,
        (oid_of(0x20), oid_of(0x12)),
        (low, both),
    );
    landed_on(&state, 0x12);

    let (state, _) = press(&state, 'N');
    landed_on(&state, 0x11);
    assert_eq!(opened(&state), [oid_of(0x06), oid_of(0x20)]);
}
