//! The mouse in the picker (ticket 10, Decided: click selects,
//! double-click switches). Its own file so `picker/tests.rs` stays under
//! its size row.

use super::*;

pub(super) fn click(state: &AppState, column: u16, row: u16) -> (AppState, Vec<Command>) {
    use mergewright_core::event::{MouseButton, MouseInput, MouseKind};
    update(
        state,
        AppEvent::Core(Event::Mouse(MouseInput {
            kind: MouseKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: Modifiers::NONE,
        })),
    )
}

/// The screen row the drawn frame shows `name` on, and a column inside the
/// box — read off the real frame, so the click and the drawing are checked
/// against each other, not against a number this test assumed.
pub(super) fn drawn_at(state: &AppState, name: &str) -> (u16, u16) {
    let frame = crate::view::tests::draw(state, state.size.columns, state.size.rows);
    // Inside the box: the graph's branch column names branches too
    // (ease-pass G5).
    let row = frame
        .lines()
        .enumerate()
        .skip_while(|(_, line)| !line.contains(['\u{256d}', '\u{250c}']))
        .find_map(|(at, line)| line.contains(&format!(" {name} ")).then_some(at))
        .unwrap_or_else(|| panic!("`{name}` is drawn in the box\n{frame}"));
    (
        state.size.columns / 2,
        u16::try_from(row).expect("a screen row"),
    )
}

pub(super) fn at(state: &AppState, seconds: i64) -> AppState {
    let mut state = state.clone();
    state.harness.clock = crate::session::Clock::Fixed(seconds);
    state
}

#[test]
fn a_click_on_a_row_selects_it_and_writes_nothing() {
    let state = at(&opened(&fixture()), 1_000);
    let (column, row) = drawn_at(&state, "catalog");
    let (state, commands) = click(&state, column, row);
    let picked = picker(&state).selected_row().expect("a row");
    assert_eq!(picked.name.as_bytes(), b"refs/heads/catalog");
    assert!(operated(&commands).is_none(), "one click writes nothing");
    assert!(state.picker.is_some(), "the picker stays up");
}

#[test]
fn a_double_click_on_a_row_switches_to_it() {
    let state = at(&opened(&fixture()), 1_000);
    let (column, row) = drawn_at(&state, "login-page");
    let (state, _) = click(&state, column, row);
    let (state, commands) = click(&state, column, row);
    assert_eq!(
        checkout_to(operated(&commands).expect("launched at once")),
        &CheckoutTarget::Branch("refs/heads/login-page".to_owned())
    );
    assert!(state.picker.is_none(), "the picker closed, as on ⏎");
}

#[test]
fn two_slow_clicks_are_two_selections() {
    let state = at(&opened(&fixture()), 1_000);
    let (column, row) = drawn_at(&state, "login-page");
    let (state, _) = click(&state, column, row);
    let (state, commands) = click(&at(&state, 1_001), column, row);
    assert!(
        operated(&commands).is_none(),
        "a second apart is not a double"
    );
    assert!(state.picker.is_some());
}

#[test]
fn a_click_outside_the_box_reaches_nothing_behind_it() {
    let state = at(&opened(&fixture()), 1_000);
    let cursor = state.history.as_ref().expect("a walk").cursor;
    // The first graph row, above the box.
    let (state, commands) = click(&state, 10, 1);
    assert_eq!(state.history.as_ref().expect("a walk").cursor, cursor);
    assert!(state.picker.is_some(), "the picker is modal");
    assert!(operated(&commands).is_none());
}
