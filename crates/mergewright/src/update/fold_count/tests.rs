//! A folding merge's count (ease-pass 06 box 6; m8-polish-and-ship/17b
//! decision 1): asked at rest for the folding merges on screen, once per
//! merge, kept for the session, a failure said once.

use mergewright_core::event::{Event, KeyCode, KeyPress};
use mergewright_core::oid::Oid;

use super::FoldCount;
use crate::history::{HistoryUpdate, RowFold, Window};
use crate::update::events::{Command, RestRead};
use crate::update::state::{AppState, Status};
use crate::update::tests::{core, merge_on_screen, sized};
use crate::update::{AppEvent, JobId, Product, update};

fn key(code: KeyCode) -> AppEvent {
    core(Event::Key(KeyPress::plain(code)))
}

/// The merges the `CountFolded` commands in `commands` ask for, in order.
fn counts_asked(commands: &[Command]) -> Vec<Oid> {
    commands
        .iter()
        .flat_map(|command| match command {
            Command::AtRest(_, RestRead::CountFolded { merges }) => merges.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// How many `CountFolded` commands `commands` holds.
fn count_commands(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|c| matches!(c, Command::AtRest(_, RestRead::CountFolded { .. })))
        .count()
}

/// The screen of `uncounted` with `extra` more folding merges below the
/// first, none counted. Returns the state and every merge, top row first.
fn merges_on_screen(extra: u8) -> (AppState, Vec<Oid>) {
    let (mut state, first) = uncounted();
    let history = state.history.as_mut().expect("a walk");
    let mut rows = vec![
        history.row_at(0).expect("row 0").clone(),
        history.row_at(1).expect("the merge").clone(),
    ];
    let mut merges = vec![first];
    for n in 0..extra {
        let mut row = rows[1].clone();
        let merge = Oid::from_bytes(&[7 + n; Oid::SHA1_BYTES]).expect("an oid");
        row.commit.id = merge;
        rows.push(row);
        merges.push(merge);
    }
    let folds = RowFold {
        folds: Some(true),
        inside: None,
    };
    let fold: Vec<RowFold> = std::iter::once(RowFold::default())
        .chain(std::iter::repeat_n(folds, rows.len() - 1))
        .collect();
    history.apply(
        HistoryUpdate::Window(Window {
            request: history.asked,
            first: 0,
            laid_out: rows.len(),
            walked: rows.len(),
            hidden: 0,
            rows: rows.into(),
            done: true,
            promoted: false,
            fold: fold.into(),
            sought: None,
        }),
        10,
    );
    (state, merges)
}

fn batch_answer(counts: Vec<(Oid, Result<usize, String>)>) -> AppEvent {
    AppEvent::Produced(JobId::new(900), Product::FoldedCounted { counts })
}

/// `merge_on_screen` with its merge's count never asked, on a screen tall
/// enough to draw both rows.
fn uncounted() -> (AppState, Oid) {
    let (mut state, merge) = merge_on_screen(JobId::new(5), 0);
    state.fold_counts.clear();
    (state, merge)
}

/// An event at rest: the terminal resized to the size it already was.
fn rest(state: &AppState) -> (AppState, Vec<Command>) {
    update(state, sized(120, 30))
}

/// The merge's count asked at rest.
fn asked() -> (AppState, Oid) {
    let (state, merge) = uncounted();
    let (state, commands) = rest(&state);
    assert_eq!(counts_asked(&commands), [merge]);
    (state, merge)
}

fn answered(state: &AppState, merge: Oid, count: Result<usize, String>) -> AppState {
    update(state, batch_answer(vec![(merge, count)])).0
}

#[test]
fn a_folding_merge_on_screen_has_its_count_asked_at_rest_once() {
    let (state, merge) = asked();
    assert_eq!(state.fold_counts.get(&merge), Some(&FoldCount::Counting));
    let (_, commands) = rest(&state);
    assert_eq!(counts_asked(&commands), [] as [Oid; 0], "asked once");
}

/// ADR-0065 D2: every uncounted folding merge on screen, one read, top first.
#[test]
fn one_rest_asks_every_uncounted_folding_merge_on_screen_in_one_read() {
    let (state, merges) = merges_on_screen(2);
    assert_eq!(merges.len(), 3);
    let (state, commands) = rest(&state);
    assert_eq!(count_commands(&commands), 1, "{commands:?}");
    assert_eq!(counts_asked(&commands), merges, "top row first");
    for merge in &merges {
        assert_eq!(state.fold_counts.get(merge), Some(&FoldCount::Counting));
    }
}

/// ADR-0065 D1: nothing is asked by another read's answer.
#[test]
fn a_count_answer_asks_no_count() {
    let (state, merges) = merges_on_screen(2);
    let (state, _) = rest(&state);
    let answer = merges.iter().map(|&m| (m, Ok(2))).collect();
    let (state, commands) = update(&state, batch_answer(answer));
    assert_eq!(count_commands(&commands), 0, "{commands:?}");
    for merge in &merges {
        assert_eq!(state.fold_counts.get(merge), Some(&FoldCount::Counted(2)));
    }
}

#[test]
fn a_merge_being_counted_is_not_asked_again() {
    let (mut state, merges) = merges_on_screen(1);
    // The second merge was counted already; only the first is asked.
    state.fold_counts.insert(merges[1], FoldCount::Counted(4));
    let (state, commands) = rest(&state);
    assert_eq!(counts_asked(&commands), [merges[0]]);
    // The first is now `Counting`: a second rest asks nothing.
    let (_, again) = rest(&state);
    assert_eq!(count_commands(&again), 0, "{again:?}");
}

#[test]
fn one_failed_count_marks_only_that_merge() {
    let (state, merges) = merges_on_screen(2);
    let (state, _) = rest(&state);
    let (state, _) = update(
        &state,
        batch_answer(vec![
            (merges[0], Ok(3)),
            (merges[1], Err("fatal: bad object".to_owned())),
            (merges[2], Ok(5)),
        ]),
    );
    assert_eq!(
        state.fold_counts.get(&merges[0]),
        Some(&FoldCount::Counted(3))
    );
    assert_eq!(state.fold_counts.get(&merges[1]), Some(&FoldCount::Failed));
    assert_eq!(
        state.fold_counts.get(&merges[2]),
        Some(&FoldCount::Counted(5))
    );
    let sha7 = &merges[1].to_string()[..7];
    assert_eq!(
        state.status,
        Status::Failed(format!(
            "couldn't count {sha7}'s commits: fatal: bad object"
        ))
    );
}

#[test]
fn a_burst_asks_no_count() {
    let (mut state, merges) = merges_on_screen(2);
    state.defer_follow = true;
    let (state, commands) = rest(&state);
    assert_eq!(count_commands(&commands), 0, "{commands:?}");
    assert!(merges.iter().all(|m| !state.fold_counts.contains_key(m)));
}

#[test]
fn a_counted_fold_is_not_recounted_on_refresh_or_open_and_close() {
    let (state, merge) = asked();
    let state = answered(&state, merge, Ok(3));
    assert_eq!(state.fold_counts.get(&merge), Some(&FoldCount::Counted(3)));

    let (state, commands) = update(
        &state,
        core(Event::Key(KeyPress::control(KeyCode::Char('r')))),
    );
    // The refresh ran (its walk was asked), so the missing count is its
    // choice, not a key that did nothing (21b-4 item 7).
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadHistory(..))),
        "Ctrl+R re-reads history: {commands:?}"
    );
    assert_eq!(counts_asked(&commands), [] as [Oid; 0], "{commands:?}");
    assert_eq!(state.fold_counts.get(&merge), Some(&FoldCount::Counted(3)));
}

#[test]
fn opening_and_closing_asks_no_second_count() {
    let (state, merge) = asked();
    let mut state = answered(&state, merge, Ok(3));
    state.history.as_mut().expect("a walk").cursor = 1;
    let (state, open) = update(&state, key(KeyCode::Char(' ')));
    let (state, close) = update(&state, key(KeyCode::Char(' ')));
    assert_eq!(counts_asked(&open), [] as [Oid; 0], "{open:?}");
    assert_eq!(counts_asked(&close), [] as [Oid; 0], "{close:?}");
    assert_eq!(state.fold_counts.get(&merge), Some(&FoldCount::Counted(3)));
}

#[test]
fn a_failed_fold_count_says_so_once() {
    let (state, merge) = asked();
    let state = answered(&state, merge, Err("fatal: bad object".to_owned()));
    let sha7 = &merge.to_string()[..7];
    assert_eq!(state.fold_counts.get(&merge), Some(&FoldCount::Failed));
    assert_eq!(
        state.status,
        Status::Failed(format!(
            "couldn't count {sha7}'s commits: fatal: bad object"
        ))
    );

    // Dismissed, then rested again: no second ask, no second line.
    let (state, _) = update(&state, key(KeyCode::Escape));
    assert_eq!(state.status, Status::Ready);
    let (state, commands) = rest(&state);
    assert_eq!(counts_asked(&commands), [] as [Oid; 0], "{commands:?}");
    assert_eq!(state.status, Status::Ready);
    assert_eq!(state.fold_counts.get(&merge), Some(&FoldCount::Failed));
}
