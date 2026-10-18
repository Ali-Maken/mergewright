//! `update` tests: the vertical carve, read back from the drawn frame
//! (m8-polish-and-ship/40).
//!
//! `geometry::rows_above_graph` says where the first graph row is; the
//! tour card and the still-to-apply block are two of its terms. This sweeps
//! them against what the frame actually draws, so the click row and the
//! drawn row cannot drift whichever side the arithmetic lives on.

use mergewright_core::journal::EntryId;
use mergewright_git::state::{InProgress, InProgressDetail};
use mergewright_ops::rewrite::{RewriteProgress, TodoAction, TodoLine};

use super::*;
use crate::update::banner::RewriteStop;
use crate::update::{LiveOrigin, Stop};
use crate::view::tests::{draw, history_of, oid, refs_with_head};

/// Commits in the walk: more than the tallest case shows, so every case
/// fills its body.
const COMMITS: u8 = 60;

/// A rebase Mergewright began, stopped at an `edit` with `pending` picks still to
/// apply, on a `columns`×`rows` screen; the tour's card on or off. The
/// cursor rests on row 3, so a click on row 0 is a move it can be seen to
/// make.
fn stopped_with(columns: u16, rows: u16, pending: u8, tour: bool) -> AppState {
    let stop = RewriteStop {
        progress: RewriteProgress {
            step: (2, 5),
            current: Some(TodoLine {
                action: TodoAction::Edit,
                commit: oid(COMMITS),
                subject: "wip".to_owned(),
            }),
            pending: (1..=pending)
                .map(|n| TodoLine {
                    action: TodoAction::Pick,
                    commit: oid(100 + n),
                    subject: format!("p{n}"),
                })
                .collect(),
            branch: None,
            begun_in_mergewright: false,
            followers: Vec::new(),
            fork: None,
        },
        edit: None,
    };
    AppState {
        history: Some(history_of(COMMITS, 3)),
        refs: Some(Arc::new(refs_with_head(Some(oid(COMMITS))))),
        in_progress: Some(InProgressDetail {
            kind: InProgress::Rebase,
            step: Some((2, 5)),
            head_name: None,
            onto: None,
            theirs: Ok(None),
        }),
        live_entries: vec![EntryId::new(1, [0; 10])],
        stopped: Some(Stop {
            origin: LiveOrigin::ThisRun,
            again: false,
            kind: InProgress::Rebase,
            paths_at_stop: 0,
            rewrite: Some(Ok(stop)),
        }),
        tour: tour.then(Box::default),
        size: Size { columns, rows },
        ..AppState::default()
    }
}

/// The screen row the newest commit is drawn on: `HEAD`'s node, beside its
/// subject. At 40 columns the subject is cut to `Comm…` (labels take up to
/// half the message column, ease-pass G7), so the node finds the row.
fn first_commit_row(drawn: &str) -> Option<u16> {
    drawn
        .lines()
        .position(|line| line.contains("Comm") && line.contains('\u{25c6}'))
        .and_then(|row| u16::try_from(row).ok())
}

/// **The pending block's geometry, and the tour card's, against the
/// frame.** Every size the sweep names, a still-to-apply block of 0, 2 and
/// 5 picks (none, a four-row block, a seven-row one), the card on and off:
/// the newest commit is drawn on row `rows_above_graph`, and a click on
/// that row selects that commit.
///
/// **Mutation:** add one to `banner::pending_rows_taken` (or to
/// `tour::card_rows`) and the drawn row and the click row part at every
/// case where the block (or the card) is drawn.
#[test]
fn a_click_on_the_first_graph_row_lands_on_the_commit_drawn_there() {
    let mut cases = 0;
    let mut with_block = 0;
    let mut with_card = 0;
    for columns in [40, 80, 120] {
        for rows in [12, 24, 48] {
            for pending in [0, 2, 5] {
                for tour in [false, true] {
                    let state = stopped_with(columns, rows, pending, tour);
                    let case = format!("{columns}x{rows}, {pending} pending, tour {tour}");
                    let drawn = draw(&state, columns, rows);
                    let above = geometry::rows_above_graph(&state);
                    assert_eq!(
                        first_commit_row(&drawn),
                        Some(above),
                        "{case}: the first graph row is drawn at rows_above_graph\n{drawn}"
                    );
                    if crate::update::banner::pending_rows_taken(&state) > 0 {
                        with_block += 1;
                    }
                    if crate::update::tour::card_rows(&state) > 0 {
                        with_card += 1;
                    }

                    let click = core(Event::Mouse(MouseInput {
                        kind: MouseKind::Down(MouseButton::Left),
                        column: 10,
                        row: above,
                        modifiers: Modifiers::NONE,
                    }));
                    let (next, _) = update(&state, click);
                    assert_eq!(
                        next.history
                            .as_ref()
                            .and_then(History::selected)
                            .map(|row| row.commit.id),
                        Some(oid(COMMITS)),
                        "{case}: the click selects the commit drawn on that row"
                    );
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 3 * 3 * 3 * 2);
    // The sweep is only a test of the block and the card if both are drawn
    // somewhere in it.
    assert!(with_block > 0, "no case drew the still-to-apply block");
    assert!(with_card > 0, "no case drew the tour card");
}
