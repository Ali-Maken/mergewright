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

/// The `CountFolded` commands in `commands`, by merge.
fn counts_asked(commands: &[Command]) -> Vec<Oid> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::AtRest(_, RestRead::CountFolded { merge }) => Some(*merge),
            _ => None,
        })
        .collect()
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
    update(
        state,
        AppEvent::Produced(JobId::new(900), Product::FoldedCounted { merge, count }),
    )
    .0
}

#[test]
fn a_folding_merge_on_screen_has_its_count_asked_at_rest_once() {
    let (state, merge) = asked();
    assert_eq!(state.fold_counts.get(&merge), Some(&FoldCount::Counting));
    let (_, commands) = rest(&state);
    assert_eq!(counts_asked(&commands), [] as [Oid; 0], "asked once");
}

#[test]
fn no_count_is_asked_mid_burst() {
    let (mut state, merge) = uncounted();
    state.defer_follow = true;
    let (state, commands) = rest(&state);
    assert_eq!(counts_asked(&commands), [] as [Oid; 0], "{commands:?}");
    assert_eq!(state.fold_counts.get(&merge), None);
}

/// Every folding merge on screen is counted — one job each, one per rest,
/// top row first, each answer's rest asking for the next — and never a
/// second job for any of them.
#[test]
fn every_folding_merge_on_screen_is_counted_one_job_each() {
    let (mut state, first) = uncounted();
    let history = state.history.as_mut().expect("a walk");
    let mut second_row = history.row_at(1).expect("the merge").clone();
    let second = Oid::from_bytes(&[7; Oid::SHA1_BYTES]).expect("an oid");
    second_row.commit.id = second;
    let rows = vec![
        history.row_at(0).expect("row 0").clone(),
        history.row_at(1).expect("the merge").clone(),
        second_row,
    ];
    let folds = RowFold {
        folds: Some(true),
        inside: None,
    };
    history.apply(
        HistoryUpdate::Window(Window {
            request: history.asked,
            first: 0,
            rows: rows.into(),
            laid_out: 3,
            done: true,
            promoted: false,
            fold: [RowFold::default(), folds.clone(), folds].into(),
            sought: None,
        }),
        10,
    );

    let mut jobs = Vec::new();
    let (mut state, commands) = rest(&state);
    jobs.extend(counts_asked(&commands));
    for _ in 0..4 {
        let Some(&merge) = jobs.last() else { break };
        let (next, commands) = update(
            &state,
            AppEvent::Produced(
                JobId::new(900),
                Product::FoldedCounted {
                    merge,
                    count: Ok(2),
                },
            ),
        );
        state = next;
        jobs.extend(counts_asked(&commands));
        let (next, commands) = rest(&state);
        state = next;
        jobs.extend(counts_asked(&commands));
    }
    assert_eq!(jobs, [first, second], "one job per merge, top first");
    assert_eq!(state.fold_counts.get(&second), Some(&FoldCount::Counted(2)));
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
