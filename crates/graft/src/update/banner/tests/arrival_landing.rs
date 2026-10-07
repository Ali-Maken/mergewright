//! A launch's or an open's landing on `HEAD` gives way to a key that moves
//! the graph cursor first (m8-polish-and-ship/12d Decision 3): at launch he
//! types at once, and the landing must not yank a cursor he already moved.
//! A `--continue`'s landing follows a write he asked for and does not give
//! way.

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};

use super::{continued_rebase, oid, refs_at};
use crate::update::{AppEvent, AppState, Product, Size, Task};
use graft_core::job::JobId;

fn j() -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress {
        code: KeyCode::Char('j'),
        modifiers: Modifiers::NONE,
    }))
}

fn selected(state: &AppState) -> Option<graft_core::oid::Oid> {
    state
        .history
        .as_ref()
        .and_then(|history| history.selected())
        .map(|row| row.commit.id)
}

/// A launch with the landing owed: the walk started and its rows in, the
/// refs read still out; that read's id.
fn launched() -> (AppState, JobId) {
    let (mut state, _) = crate::update::opening(
        std::path::PathBuf::from("/nowhere"),
        Size {
            columns: 120,
            rows: 36,
        },
    );
    crate::update::lands_on_head_at_open(&mut state);
    let walk = state.awaiting.expect("the opening walk");
    let refs = state.head_landing.refs.expect("the opening refs read");
    let (mut state, _) = crate::update::update(&state, AppEvent::Started(walk, Task::ReadHistory));
    assert!(state.land_on_head, "owed until the refs answer");
    // The walk's rows: oid(5) on top, oid(1) at the bottom.
    state.history = Some(crate::view::tests::history_of(5, 0));
    (state, refs)
}

#[test]
fn a_key_before_the_launch_landing_keeps_his_cursor() {
    let (state, refs) = launched();
    let (state, _) = crate::update::update(&state, j());
    assert_eq!(selected(&state), Some(oid(4)), "j moved the cursor");
    // HEAD is further down; the refs answer after his key.
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Produced(refs, Product::Refs(Ok(refs_at(oid(2))))),
    );
    assert!(!state.land_on_head, "no landing left owed");
    assert_eq!(
        selected(&state),
        Some(oid(4)),
        "the cursor stays where j put it"
    );
}

#[test]
fn with_no_key_the_launch_lands_on_head() {
    let (state, refs) = launched();
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Produced(refs, Product::Refs(Ok(refs_at(oid(2))))),
    );
    assert_eq!(selected(&state), Some(oid(2)), "on HEAD");
}

#[test]
fn a_key_does_not_drop_a_continue_landing() {
    let (state, _, _) = continued_rebase();
    assert!(state.land_on_head);
    let (state, _) = crate::update::update(&state, j());
    assert!(state.land_on_head, "a --continue still lands on HEAD");
}
