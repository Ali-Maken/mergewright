//! A folded merge's `+N` (m8-polish-and-ship/17b decision 1): asked once
//! per merge, kept for the session, a failure said once.

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::oid::Oid;

use super::FoldCount;
use crate::update::events::Command;
use crate::update::state::{AppState, Status};
use crate::update::tests::{core, merge_on_screen};
use crate::update::{AppEvent, JobId, Product, update};

fn key(code: KeyCode) -> AppEvent {
    core(Event::Key(KeyPress::plain(code)))
}

/// The `CountFolded` commands in `commands`, by merge.
fn counts_asked(commands: &[Command]) -> Vec<Oid> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::CountFolded { merge, .. } => Some(*merge),
            _ => None,
        })
        .collect()
}

/// `Space` on the merge row of `merge_on_screen`: folded, and its count asked.
fn folded() -> (AppState, Oid, Vec<Command>) {
    let (state, merge) = merge_on_screen(JobId::new(5), 1);
    let (next, commands) = update(&state, key(KeyCode::Char(' ')));
    (next, merge, commands)
}

fn answered(state: &AppState, merge: Oid, count: Result<usize, String>) -> AppState {
    update(
        state,
        AppEvent::Produced(JobId::new(900), Product::FoldedCounted { merge, count }),
    )
    .0
}

#[test]
fn folding_a_merge_asks_for_its_count() {
    let (next, merge, commands) = folded();
    assert_eq!(counts_asked(&commands), [merge]);
    assert_eq!(next.fold_counts.get(&merge), Some(&FoldCount::Counting));
}

#[test]
fn a_counted_fold_is_not_recounted_on_refresh() {
    let (state, merge, _) = folded();
    let state = answered(&state, merge, Ok(3));
    assert_eq!(state.fold_counts.get(&merge), Some(&FoldCount::Counted(3)));

    let (state, commands) = update(
        &state,
        core(Event::Key(KeyPress::control(KeyCode::Char('r')))),
    );
    assert_eq!(counts_asked(&commands), [] as [Oid; 0], "{commands:?}");
    assert_eq!(state.fold_counts.get(&merge), Some(&FoldCount::Counted(3)));

    // Unfold and fold again: the answer is a fact of the merge, kept.
    let (state, _) = update(&state, key(KeyCode::Char(' ')));
    let (state, commands) = update(&state, key(KeyCode::Char(' ')));
    assert_eq!(counts_asked(&commands), [] as [Oid; 0], "{commands:?}");
    assert_eq!(state.fold_counts.get(&merge), Some(&FoldCount::Counted(3)));
}

#[test]
fn a_failed_fold_count_says_so_once() {
    let (state, merge, _) = folded();
    let state = answered(&state, merge, Err("fatal: bad object".to_owned()));
    let sha7 = &merge.to_string()[..7];
    assert_eq!(state.fold_counts.get(&merge), Some(&FoldCount::Failed));
    assert_eq!(
        state.status,
        Status::Failed(format!(
            "couldn't count {sha7}'s commits: fatal: bad object"
        ))
    );

    // Dismissed, then unfolded and folded again: no second ask, no second line.
    let (state, _) = update(&state, key(KeyCode::Escape));
    assert_eq!(state.status, Status::Ready);
    let (state, _) = update(&state, key(KeyCode::Char(' ')));
    let (state, commands) = update(&state, key(KeyCode::Char(' ')));
    assert_eq!(counts_asked(&commands), [] as [Oid; 0], "{commands:?}");
    assert_eq!(state.status, Status::Ready);
    assert_eq!(state.fold_counts.get(&merge), Some(&FoldCount::Failed));
}
