//! A find the walk answers folded, as the screen takes it (ease-pass 06c):
//! the merge opens — once per round, so a commit behind two merges takes
//! two — and nothing else changes. A view he chose is left as it is, and a
//! merge already open is a walk that answered for another view: said, not
//! retried.

use mergewright_core::job::JobId;
use mergewright_core::oid::Oid;
use mergewright_graph::Follow;

use crate::history::Found;
use crate::update::events::Command;
use crate::update::keys::view_change::open_merge;
use crate::update::state::{AppState, Status};
use crate::update::tests::awaiting;
use crate::update::tests::reveal::{answer, folded_answer, oid_of, on, revealed};
use crate::update::update;

const WALK: JobId = JobId::new(4242);

/// A fresh walk, asked to find `target`: the state and the find's id.
fn finding(target: Oid, setup: impl FnOnce(&mut AppState)) -> (AppState, u64) {
    let mut state = awaiting(WALK);
    setup(&mut state);
    let history = state.history.as_mut().expect("a walk");
    let find = history.seek(target, 1).expect("not on hand: a find").id;
    (state, find)
}

fn opened(state: &AppState) -> Vec<Oid> {
    let history = state.history.as_ref().expect("a walk");
    let opened = history.view.inclusion.opened().expect("a folding view");
    [0x20, 0x30, 0x31]
        .map(oid_of)
        .into_iter()
        .filter(|merge| opened.contains(*merge))
        .collect()
}

fn row_requests(commands: &[Command]) -> usize {
    let asks = commands
        .iter()
        .filter(|c| matches!(c, Command::ShowRows(..)));
    asks.count()
}

/// 0x11 is on a branch merged at 0x31, itself on a branch merged at 0x20.
/// The first answer names the shown merge, the second the one it opened.
#[test]
fn a_commit_behind_two_merges_opens_both_in_two_rounds_and_lands() {
    let (outer, inner, target) = (oid_of(0x20), oid_of(0x31), oid_of(0x11));
    let (state, find) = finding(target, |_| {});

    let (state, again) = folded_answer(&state, WALK, find, (outer, target), &[0x30, 0x20, 0x10]);
    assert_eq!(opened(&state), [outer], "round one opens the outer merge");
    let middle: &[u8] = &[0x30, 0x20, 0x31, 0x10];
    let opened_rows: &[u8] = &[0x30, 0x20, 0x31, 0x11, 0x10];
    let (state, _) = revealed(&state, WALK, again, (inner, target), (middle, opened_rows));

    assert_eq!(opened(&state), [outer, inner], "both open");
    assert_eq!(on(&state), Some(target), "the cursor on it");
}

/// The walk names a merge the view has open already: the find answered a
/// view other than the one asked. Nothing is re-asked; the cursor stays on
/// the merge and the failure is said.
#[test]
fn folded_behind_a_merge_already_open_ends_the_seek_and_says_so() {
    let (merge, target) = (oid_of(0x20), oid_of(0x11));
    let (state, find) = finding(target, |state| {
        let history = state.history.as_mut().expect("a walk");
        history.view = open_merge(&history.view, merge).expect("folded until now");
    });
    let folded = Some(Found::Folded { behind: merge });

    let (state, commands) = update(&state, answer(WALK, find, &[0x30, 0x20, 0x10], folded));

    assert_eq!(row_requests(&commands), 0, "nothing re-asked: {commands:?}");
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(history.seeking, None, "the seek is over");
    assert_eq!(on(&state), Some(merge), "the cursor on the merge, not it");
    let Status::Failed(words) = &state.status else {
        panic!("the failure is said: {:?}", state.status);
    };
    assert!(words.contains("open already"), "{words}");
}

/// First-parent `1` is a view he chose: off its line is not in this view,
/// and nothing is opened to show it.
#[test]
fn off_the_first_parent_line_opens_nothing() {
    let target = oid_of(0x11);
    let (state, find) = finding(target, |state| {
        state.history.as_mut().expect("a walk").view.follow = Follow::FirstParent;
    });
    let before = state.history.as_ref().map(|history| history.view.clone());

    let not_in_view = Some(Found::NotInView);
    let (state, commands) = update(&state, answer(WALK, find, &[0x30, 0x20], not_in_view));

    assert_eq!(row_requests(&commands), 0, "nothing re-asked: {commands:?}");
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(
        Some(history.view.clone()),
        before,
        "the view as he chose it"
    );
    assert_ne!(on(&state), Some(target));
    assert_eq!(state.status, Status::Ready, "nothing failed");
}

/// A reveal is a view change, not an operation: it launches no write, so
/// nothing reaches the journal, and `u` launches exactly what it would
/// have before the reveal.
#[test]
fn a_reveal_is_not_journaled_and_u_is_unchanged() {
    let (merge, target) = (oid_of(0x20), oid_of(0x11));
    let (before, find) = finding(target, |_| {});
    let rows: (&[u8], &[u8]) = (&[0x30, 0x20, 0x10], &[0x30, 0x20, 0x11, 0x10]);
    let (after, commands) = revealed(&before, WALK, find, (merge, target), rows);
    assert_eq!(on(&after), Some(target), "revealed");
    assert!(after.operating.is_none(), "no write launched");
    let wrote = commands.iter().any(|c| matches!(c, Command::Operate(..)));
    assert!(!wrote, "{commands:?}");

    let u = crate::update::tests::key(mergewright_core::event::KeyCode::Char('u'));
    let undo = |state: &AppState| {
        let (state, commands) = update(state, u.clone());
        let undos: Vec<Command> = commands
            .into_iter()
            .filter(|c| matches!(c, Command::Undo(..) | Command::Redo(..)))
            .collect();
        (undos, state.status, state.confirm.is_some())
    };
    assert_eq!(undo(&after), undo(&before));
}
