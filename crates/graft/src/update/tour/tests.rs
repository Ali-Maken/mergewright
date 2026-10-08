//! The tour's state over scripted events: when the ticks are asked for,
//! what an answer does, and how `Esc` leaves only once nothing else takes it.
//! The practice repository itself is `graft_ops::tour`'s; the walk through
//! it is `tests/it/tour.rs`.

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::HeadState;

use super::*;
use crate::update::events::{AppEvent, Product};
use crate::update::update;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn name(full: &str) -> RefName {
    RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
}

fn key(code: KeyCode) -> KeyPress {
    KeyPress {
        code,
        modifiers: Modifiers::NONE,
    }
}

fn press(state: &AppState, pressed: KeyPress) -> AppState {
    update(state, AppEvent::Core(Event::Key(pressed))).0
}

/// Three commits, `main` checked out, the cursor on row 0, in the tour with
/// its arrival read already answered.
fn in_the_tour() -> AppState {
    let refs = crate::refs::badges(
        &[(name("refs/heads/main"), oid(2))],
        &HeadState::Branch(name("refs/heads/main")),
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("no ahead/behind walk can fail here");
    AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        refs: Some(std::sync::Arc::new(refs)),
        size: crate::update::Size {
            columns: 100,
            rows: 30,
        },
        tour: Some(Box::default()),
        ..AppState::default()
    }
}

fn asked(commands: &[Command]) -> Vec<JobId> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::TourProgress(id) => Some(*id),
            _ => None,
        })
        .collect()
}

#[test]
fn the_ticks_are_asked_once_at_arrival_and_not_again_while_one_is_out() {
    let mut state = in_the_tour();
    state.tour = Some(Box::new(TourState::arrived()));
    let (next, commands) = update(&state, AppEvent::Core(Event::FocusGained));
    let ids = asked(&commands);
    assert_eq!(ids.len(), 1, "one read at arrival: {commands:?}");
    let tour = next.tour.as_ref().expect("still in the tour");
    assert_eq!(tour.asked, Some(ids[0]));
    assert!(!tour.wanted);

    let (_, commands) = update(&next, AppEvent::Core(Event::FocusGained));
    assert!(asked(&commands).is_empty(), "never per event: {commands:?}");
}

#[test]
fn a_write_that_ends_asks_for_the_ticks_again() {
    let mut before = in_the_tour();
    before.operating = Some(JobId::new(40));
    let mut next = before.clone();
    next.operating = None;
    let mut commands = Vec::new();
    request(&before, &mut next, &mut commands);
    assert_eq!(asked(&commands).len(), 1, "{commands:?}");

    // Nothing ended: nothing asked.
    let quiet = in_the_tour();
    let mut again = quiet.clone();
    let mut commands = Vec::new();
    request(&quiet, &mut again, &mut commands);
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn outside_the_tour_nothing_is_ever_asked() {
    let mut before = in_the_tour();
    before.tour = None;
    before.operating = Some(JobId::new(40));
    let mut next = before.clone();
    next.operating = None;
    let mut commands = Vec::new();
    request(&before, &mut next, &mut commands);
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn an_answer_sets_the_ticks_and_a_stale_one_is_dropped() {
    let mut state = in_the_tour();
    let id = JobId::new(50);
    state.tour.as_mut().unwrap().asked = Some(id);
    let mut done = [false; 8];
    done[0] = true;
    let (late, _) = update(
        &state,
        AppEvent::Produced(JobId::new(49), Product::TourProgress(Ok(done))),
    );
    assert_eq!(late.tour.as_ref().unwrap().done, [false; 8]);

    let (next, _) = update(
        &state,
        AppEvent::Produced(id, Product::TourProgress(Ok(done))),
    );
    let tour = next.tour.as_ref().unwrap();
    assert_eq!(tour.done, done);
    assert_eq!(tour.asked, None);
    assert_eq!(tour.current(), Some(1));
    assert_eq!(next.quit, None, "one of eight is not the end");
}

#[test]
fn all_eight_ticked_leaves_the_tour_finished() {
    let mut state = in_the_tour();
    let id = JobId::new(50);
    state.tour.as_mut().unwrap().asked = Some(id);
    let (next, _) = update(
        &state,
        AppEvent::Produced(id, Product::TourProgress(Ok([true; 8]))),
    );
    assert_eq!(next.quit, Some(Quit::LeaveTour { finished: true }));
}

#[test]
fn a_failed_read_is_said_and_the_ticks_kept() {
    let mut state = in_the_tour();
    let id = JobId::new(50);
    let tour = state.tour.as_mut().unwrap();
    tour.asked = Some(id);
    tour.done[0] = true;
    let (next, _) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::TourProgress(Err("the journal is unreadable".to_owned())),
        ),
    );
    assert_eq!(
        next.status,
        Status::Failed("tour: the journal is unreadable".to_owned())
    );
    assert!(next.tour.as_ref().unwrap().done[0]);
    assert_eq!(next.tour.as_ref().unwrap().asked, None);
}

#[test]
fn esc_on_the_bare_graph_leaves_the_tour() {
    let next = press(&in_the_tour(), key(KeyCode::Escape));
    assert_eq!(next.quit, Some(Quit::LeaveTour { finished: false }));
}

#[test]
fn esc_outside_the_tour_leaves_nothing() {
    let mut state = in_the_tour();
    state.tour = None;
    let next = press(&state, key(KeyCode::Escape));
    assert_eq!(next.quit, None);
}

/// One open thing per kind: `Esc` closes that, and only the next one
/// leaves the tour.
fn esc_closes_first(opened: &AppState, is_open: impl Fn(&AppState) -> bool, kind: &str) {
    assert!(is_open(opened), "{kind} did not open");
    let closed = press(opened, key(KeyCode::Escape));
    assert!(!is_open(&closed), "Esc did not close the {kind}");
    assert_eq!(closed.quit, None, "Esc over a {kind} left the tour");
    let left = press(&closed, key(KeyCode::Escape));
    assert_eq!(
        left.quit,
        Some(Quit::LeaveTour { finished: false }),
        "the next Esc, with the {kind} shut"
    );
}

#[test]
fn esc_over_a_prompt_closes_the_prompt_not_the_tour() {
    let opened = press(&in_the_tour(), key(KeyCode::Char('b')));
    esc_closes_first(&opened, |s| s.name_prompt.is_some(), "branch prompt");
}

#[test]
fn esc_over_the_menu_closes_the_menu_not_the_tour() {
    let opened = press(&in_the_tour(), key(KeyCode::Char('.')));
    esc_closes_first(&opened, |s| s.context_menu.is_some(), "menu");
}

#[test]
fn esc_over_a_box_closes_the_box_not_the_tour() {
    let opened = press(
        &in_the_tour(),
        KeyPress {
            code: KeyCode::Char('p'),
            modifiers: Modifiers::CONTROL,
        },
    );
    esc_closes_first(&opened, |s| s.palette.is_some(), "palette box");
}

#[test]
fn esc_over_a_search_closes_the_search_not_the_tour() {
    let mut state = in_the_tour();
    let query = crate::search::Query::parse("a").expect("a non-empty query");
    let mut search = crate::search::Search::new(query, 0, (0, 0));
    search.state = crate::search::SearchState::Done { hits: 0 };
    state.search = Some(search);
    esc_closes_first(&state, |s| s.search.is_some(), "search");
}

#[test]
fn a_leave_key_moved_off_esc_leaves_and_esc_then_does_not() {
    let mut state = in_the_tour();
    let loaded = crate::config::tests::loaded_from_a_file("[keys.tour]\nleave = \"Z\"\n");
    let (keymap, problems) = crate::config::keymap_from(&loaded);
    assert!(problems.is_empty(), "{problems:?}");
    state.keymap = std::sync::Arc::new(keymap);
    let esc = press(&state, key(KeyCode::Escape));
    assert_eq!(esc.quit, None, "Esc is no longer the way out");
    // `Z`: no graph row takes it (`L` would not do — the graph borrows
    // `diff.load_all`'s, so it stays the graph's).
    let left = press(&state, key(KeyCode::Char('Z')));
    assert_eq!(left.quit, Some(Quit::LeaveTour { finished: false }));
}

#[test]
fn the_palette_row_starts_the_tour_from_the_cursor_and_inside_says_so() {
    let mut state = in_the_tour();
    state.tour = None;
    let at = state
        .history
        .as_ref()
        .and_then(|h| h.selected())
        .map(|row| row.commit.id);
    start_from_palette(&mut state);
    assert_eq!(state.quit, Some(Quit::Practise { cursor: at }));

    let mut inside = in_the_tour();
    start_from_palette(&mut inside);
    assert_eq!(inside.quit, None);
    assert_eq!(
        inside.status,
        Status::Waiting("tour: you're in it \u{b7} esc leaves".to_owned())
    );
}

#[test]
fn the_card_takes_three_rows_off_the_graph_while_the_tour_is_on() {
    let state = in_the_tour();
    let mut off = state.clone();
    off.tour = None;
    assert_eq!(card_rows(&state), graft_ui::tour_card::TOUR_CARD_ROWS);
    assert_eq!(card_rows(&off), 0);
    assert_eq!(
        off.graph_rows() - state.graph_rows(),
        usize::from(graft_ui::tour_card::TOUR_CARD_ROWS)
    );
}

#[test]
fn a_return_whose_commit_is_gone_lands_on_head_and_says_so() {
    let mut state = in_the_tour();
    state.tour = None;
    // The cursor is on row 0; HEAD (`main`) is oid(2); the commit sought
    // is not in the walk and no find for it is out.
    state.return_to = Some(oid(0xee));
    state.land_on_head = false;
    let mut commands = Vec::new();
    check_return(&mut state, &mut commands);
    assert_eq!(state.return_to, None);
    assert_eq!(
        state.status,
        Status::sticky("the commit you were on is gone \u{b7} on HEAD")
    );
}

#[test]
fn a_return_whose_commit_is_under_the_cursor_says_nothing() {
    let mut state = in_the_tour();
    state.tour = None;
    let here = state
        .history
        .as_ref()
        .and_then(|h| h.selected())
        .map(|row| row.commit.id)
        .expect("a row under the cursor");
    state.return_to = Some(here);
    state.land_on_head = false;
    let before = state.status.clone();
    let mut commands = Vec::new();
    check_return(&mut state, &mut commands);
    assert_eq!(state.return_to, None);
    assert_eq!(state.status, before);
}
