//! How many hits are in folded merges (ease-pass 06c, 2026-10-18): the
//! search asks the walk beside the rows when its hits finish, after each
//! reveal, and once more when the walk completes; the line says so while
//! any are.

use std::sync::Arc;

use graft_core::event::{Event, KeyCode};
use graft_core::job::JobId;
use graft_core::oid::Oid;

use crate::history::{Found, HistoryUpdate};
use crate::search::{Query, Search, SearchState};
use crate::update::events::{AppEvent, Command, Progress};
use crate::update::state::AppState;
use crate::update::tests::reveal::{answer, asked_folded, find_for, folded_hits, oid_of, on};
use crate::update::tests::{awaiting, core, key};
use crate::update::update;

const WALK: JobId = JobId::new(4242);
const JOB: JobId = JobId::new(77);

/// A fresh walk with search job [`JOB`] running, its hits `hits` in walk
/// order, the first already gone to.
fn running(hits: &[u8]) -> AppState {
    let mut state = awaiting(WALK);
    let query = Query::parse("login").expect("a non-empty query");
    let mut search = Search::new(query, 0, (0, 0));
    let ids: Vec<Oid> = hits.iter().copied().map(oid_of).collect();
    search.batches = vec![Arc::from(ids)];
    search.job = Some(JOB);
    search.state = SearchState::Running { hits: hits.len() };
    search.current = Some(0);
    state.search = Some(search);
    state
}

fn line(state: &AppState) -> String {
    let search = state.search.as_ref().expect("the search");
    search.line(&state.keymap)
}

/// The hits finish: one row request carries every hit, asked; its answer
/// is the line's. An answer to any other ask is not this search's.
#[test]
fn when_the_hits_finish_the_walk_is_asked_and_its_count_is_said() {
    let hits = [0x30, 0x11, 0x12, 0x05];
    let (state, commands) = update(&running(&hits), core(Event::JobFinished(JOB)));

    let ask = asked_folded(&commands).expect("a row request carries the ask");
    let every: Vec<Oid> = hits.iter().copied().map(oid_of).collect();
    assert_eq!(&*ask.ids, every.as_slice(), "every hit, walk order");
    assert_eq!(
        line(&state),
        "/login  4 matches (1/4)  n/N next/prev  esc close"
    );

    let (state, _) = update(&state, folded_hits(WALK, ask.search + 1, 1, 0));
    assert!(
        !line(&state).contains("folded"),
        "not its ask: {}",
        line(&state)
    );

    let (state, _) = update(&state, folded_hits(WALK, ask.search, 3, 0));
    assert_eq!(
        line(&state),
        "/login  4 matches \u{b7} 3 in folded merges (1/4)  n/N next/prev  esc close"
    );
}

/// `n` to a hit the walk answers folded: the reveal's own find carries a
/// new ask, so the count follows the merge it opened.
fn revealed_and_counted(
    state: &AppState,
    (hit, merge): (u8, u8),
    (shown, opened): (&[u8], &[u8]),
    folded: usize,
) -> AppState {
    let (state, commands) = update(state, key(KeyCode::Char('n')));
    let find = find_for(&commands, oid_of(hit));
    let behind = Some(Found::Folded {
        behind: oid_of(merge),
    });
    let (state, commands) = update(&state, answer(WALK, find, shown, behind));
    let again = find_for(&commands, oid_of(hit));
    let ask = asked_folded(&commands).expect("the reveal's find carries an ask");
    assert_eq!(
        state.search.as_ref().and_then(|search| search.folded_ask),
        Some(ask.search),
        "the search's newest ask"
    );
    let (state, _) = update(&state, answer(WALK, again, opened, Some(Found::Row)));
    let (state, _) = update(&state, folded_hits(WALK, ask.search, folded, 0));
    landed_on(&state, hit);
    state
}

/// Neither `(not in this view)` nor a seek still out.
fn landed_on(state: &AppState, hit: u8) {
    let search = state.search.as_ref().expect("the search");
    assert!(!search.missed, "not in this view at {hit:#x}");
    assert_eq!(search.seeking, None, "the seek is over");
    assert_eq!(on(state), Some(oid_of(hit)), "the cursor on {hit:#x}");
}

/// Four hits in walk order: 0x30 shown, 0x11 and 0x12 behind 0x20, 0x05
/// behind 0x06. `n` from the first visits each, revealing as it goes; the
/// line counts down the folded ones and, none left, is as it was.
#[test]
fn n_from_the_first_visits_every_hit_and_the_line_counts_the_folded_down() {
    let hits = [0x30, 0x11, 0x12, 0x05];
    let shown: &[u8] = &[0x30, 0x20, 0x10, 0x06, 0x01];
    let (state, commands) = update(&running(&hits), core(Event::JobFinished(JOB)));
    let rows = commands
        .iter()
        .find_map(|command| match command {
            Command::ShowRows(_, request) => Some(request.id),
            _ => None,
        })
        .expect("a row request");
    let ask = asked_folded(&commands).expect("asked");
    let (state, _) = update(&state, answer(WALK, rows, shown, None));
    let (state, _) = update(&state, folded_hits(WALK, ask.search, 3, 0));
    landed_on(&state, 0x30);
    let words = |at: usize, folded: &str| {
        format!("/login  4 matches{folded} ({at}/4)  n/N next/prev  esc close")
    };
    assert_eq!(line(&state), words(1, " \u{b7} 3 in folded merges"));

    let pr: &[u8] = &[0x30, 0x20, 0x11, 0x12, 0x10, 0x06, 0x01];
    let state = revealed_and_counted(&state, (0x11, 0x20), (shown, pr), 1);
    assert_eq!(line(&state), words(2, " \u{b7} 1 in folded merges"));

    let (state, commands) = update(&state, key(KeyCode::Char('n')));
    landed_on(&state, 0x12);
    assert_eq!(
        asked_folded(&commands),
        None,
        "nothing opened, nothing asked"
    );
    assert_eq!(line(&state), words(3, " \u{b7} 1 in folded merges"));

    let all: &[u8] = &[0x30, 0x20, 0x11, 0x12, 0x10, 0x06, 0x05, 0x01];
    let state = revealed_and_counted(&state, (0x05, 0x06), (pr, all), 0);
    assert_eq!(line(&state), words(4, ""));
}

/// The id of the row request in `commands`: the one a finished search's
/// ask rides on.
fn rows_asked(commands: &[Command]) -> u64 {
    commands
        .iter()
        .find_map(|command| match command {
            Command::ShowRows(_, request) => Some(request.id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a row request: {commands:?}"))
}

/// [`answer`] to `request` with history going on past `bytes`.
fn short_of_the_end(request: u64, bytes: &[u8]) -> AppEvent {
    let AppEvent::Progress(walk, Progress::History(HistoryUpdate::Window(mut window))) =
        answer(WALK, request, bytes, None)
    else {
        unreachable!("`answer` sends a window");
    };
    window.done = false;
    window.laid_out = bytes.len() + 10;
    AppEvent::Progress(walk, Progress::History(HistoryUpdate::Window(window)))
}

/// The walk had not reached two hits when it counted. Once it lays out to
/// the end it is asked once more, and that answer is the line's.
#[test]
fn hits_the_walk_had_not_reached_are_counted_once_it_completes() {
    let (state, commands) = update(&running(&[0x30, 0x11, 0x12]), core(Event::JobFinished(JOB)));
    let ask = asked_folded(&commands).expect("asked");
    let (state, _) = update(
        &state,
        short_of_the_end(rows_asked(&commands), &[0x30, 0x20]),
    );
    let (state, commands) = update(&state, folded_hits(WALK, ask.search, 0, 2));
    assert_eq!(
        asked_folded(&commands),
        None,
        "not complete: nothing more to ask"
    );
    assert!(!line(&state).contains("folded"), "{}", line(&state));

    let complete = HistoryUpdate::LaidOut { rows: 14 };
    let (state, commands) = update(
        &state,
        AppEvent::Progress(WALK, Progress::History(complete)),
    );

    let again = asked_folded(&commands).expect("asked once more");
    assert_ne!(again.search, ask.search);
    let (state, _) = update(&state, folded_hits(WALK, again.search, 2, 0));
    assert_eq!(
        line(&state),
        "/login  3 matches \u{b7} 2 in folded merges (1/3)  n/N next/prev  esc close"
    );
}

/// First-parent `1` is a view he chose: a hit off its line is never
/// walked, so the walk counts none folded (`history::idle_tests::
/// folded_tests::hits_off_the_first_parent_line_are_not_counted_folded`).
/// `n` there is not in this view, opens nothing, asks nothing, and the
/// line has no suffix.
#[test]
fn off_the_first_parent_line_the_line_says_not_in_this_view_and_nothing_folded() {
    let mut start = running(&[0x30, 0x11]);
    let history = start.history.as_mut().expect("a walk");
    history.view.follow = graft_graph::Follow::FirstParent;
    let line_rows: &[u8] = &[0x30, 0x20, 0x10];
    let (state, commands) = update(&start, core(Event::JobFinished(JOB)));
    let ask = asked_folded(&commands).expect("asked");
    let (state, _) = update(&state, answer(WALK, rows_asked(&commands), line_rows, None));
    let (state, commands) = update(&state, folded_hits(WALK, ask.search, 0, 1));
    assert_eq!(
        asked_folded(&commands),
        None,
        "the walk is whole: not asked again"
    );
    let view = state.history.as_ref().map(|history| history.view.clone());

    let (state, commands) = update(&state, key(KeyCode::Char('n')));
    let find = find_for(&commands, oid_of(0x11));
    let not_in_view = Some(Found::NotInView);
    let (state, commands) = update(&state, answer(WALK, find, line_rows, not_in_view));

    assert_eq!(
        state.history.as_ref().map(|h| h.view.clone()),
        view,
        "nothing opened"
    );
    assert_eq!(asked_folded(&commands), None, "nothing asked");
    assert_eq!(
        line(&state),
        "/login  2 matches (2/2)  (not in this view)  n/N next/prev  esc close"
    );
}
