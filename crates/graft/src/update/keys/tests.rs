//! `Esc` and the history walk (m6-conflicts/85 A): `Esc` with nothing to
//! back out of stops the walk only while it is reading, and a stopped walk's
//! motions stay on the rows it left on hand.

use graft_core::event::KeyCode;
use graft_core::job::JobId;
use graft_ui::graph_screen::WalkView;

use crate::history::{HistoryUpdate, Phase, Window};
use crate::update::tests::{awaiting, key, progress, rows_for, sized};
use crate::update::{AppEvent, AppState, Command, Progress, update};
use crate::view::graph_view;

/// `rows_for`'s window, marked as the walk's last.
fn last_window(id: JobId, request: u64, rows: usize) -> AppEvent {
    let AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(window))) =
        rows_for(id, request, 0..rows, rows)
    else {
        unreachable!("`rows_for` answers with a window");
    };
    AppEvent::Progress(
        id,
        Progress::History(HistoryUpdate::Window(Window {
            done: true,
            ..window
        })),
    )
}

fn walk(state: &AppState) -> WalkView {
    graph_view(state, 0).walk
}

/// An 80×12 screen whose walk has answered its first screen (30 rows on
/// hand of 500 laid out) and rests: nothing asked.
fn paused(id: JobId) -> AppState {
    let (state, _) = update(&awaiting(id), sized(80, 12));
    let (state, _) = update(&state, rows_for(id, 2, 0..30, 500));
    state
}

/// [`paused`], then `PgDn` until the screen asks the walk for rows it has
/// not sent: the walk is reading.
fn reading(id: JobId) -> AppState {
    let mut state = paused(id);
    for _ in 0..10 {
        let (next, commands) = update(&state, key(KeyCode::PageDown));
        state = next;
        if commands
            .iter()
            .any(|command| matches!(command, Command::ShowRows(..)))
        {
            return state;
        }
    }
    panic!("paging down never asked for rows");
}

/// A 6-commit history, every row laid out and on screen: the walk stays
/// `awaiting` (it serves rows on request) but nothing runs. `Esc` used to
/// cancel it and draw `stopped at 6 rows` (review m6-usability-2 A).
#[test]
fn esc_on_a_loaded_history_does_nothing() {
    let id = JobId::new(4);
    let (settled, _) = update(&awaiting(id), last_window(id, 1, 6));
    assert_eq!(progress(&settled), (Phase::Complete, 6));
    assert_eq!(walk(&settled), WalkView::Complete { rows: 6 });

    let (next, commands) = update(&settled, key(KeyCode::Escape));

    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next.awaiting, Some(id), "the walk is left serving rows");
    assert_eq!(walk(&next), WalkView::Complete { rows: 6 });
    let screen = crate::view::tests::draw(&next, 80, 12);
    assert!(screen.contains("6 commits"), "{screen}");
    assert!(!screen.contains("stopped"), "{screen}");
}

/// The other resting state: not every row laid out, nothing asked. Also
/// nothing running, so also nothing to cancel.
#[test]
fn esc_on_a_paused_walk_does_nothing() {
    let id = JobId::new(4);
    let state = paused(id);
    assert_eq!(walk(&state), WalkView::Paused { laid_out: 500 });

    let (next, commands) = update(&state, key(KeyCode::Escape));

    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next, state);
}

/// Rows asked and not yet laid out: the walk is working, the header says
/// so, and `Esc` stops it — `stopped at <N> rows · ⌃R reloads`.
#[test]
fn esc_while_reading_cancels_the_walk() {
    let id = JobId::new(4);
    // The first screen asked for, nothing answered: 40 rows laid out so far.
    let (opening, _) = update(
        &awaiting(id),
        AppEvent::Progress(
            id,
            Progress::History(HistoryUpdate::Progress {
                request: 1,
                laid_out: 40,
            }),
        ),
    );
    // And a walk that rested, then was asked for more by `PgDn`.
    for state in [opening, reading(id)] {
        assert!(
            matches!(walk(&state), WalkView::Reading { .. }),
            "{:?}",
            walk(&state)
        );
        let laid_out = progress(&state).1;

        let (next, commands) = update(&state, key(KeyCode::Escape));

        assert_eq!(commands, vec![Command::CancelJob(id)]);
        assert_eq!(next.awaiting, None);
        assert_eq!(walk(&next), WalkView::Cancelled { laid_out });
        let screen = crate::view::tests::draw(&next, 120, 12);
        assert!(
            screen.contains(&format!(
                "stopped at {laid_out} rows \u{b7} \u{2303}R reloads"
            )),
            "{screen}"
        );
    }
}

/// After a cancel nobody answers an ask, so the motions stay on the rows on
/// hand: `G`, `j` at the end and `PgDn` land on the last of them, and the
/// graph draws rows. It drew a blank pane: the cursor went to row 499 of a
/// window holding 0–29 (review m6-usability-2 A, third bullet).
#[test]
fn g_after_a_cancelled_walk_lands_on_the_last_laid_out_row() {
    let id = JobId::new(4);
    let (stopped, commands) = update(&reading(id), key(KeyCode::Escape));
    assert_eq!(commands, vec![Command::CancelJob(id)]);

    let (bottom, commands) = update(&stopped, key(KeyCode::Char('G')));

    assert!(
        commands.is_empty(),
        "a dead walk is asked nothing: {commands:?}"
    );
    let history = bottom.history.as_ref().expect("a walk");
    assert_eq!(history.cursor, 29, "the last row on hand");
    assert!(
        history.selected().is_some(),
        "the cursor is on a row the screen holds"
    );
    let view = graph_view(&bottom, 0);
    assert!(!view.rows.is_empty(), "the graph pane draws rows");
    for motion in [KeyCode::Char('j'), KeyCode::PageDown] {
        let (moved, commands) = update(&bottom, key(motion));
        assert!(commands.is_empty(), "{commands:?}");
        assert_eq!(moved.history.as_ref().map(|h| h.cursor), Some(29));
    }
}

// ------------------------------ m6-conflicts/85 m2: `S` and `J` from the pane

/// The changes pane focused, one file listed: where a pull's refusal names
/// `S` (review m6-usability-2 m2).
fn pane_focused() -> AppState {
    use graft_ui::detail::{DetailFocus, WorkingTreeSection};
    let mut state = AppState {
        detail: crate::update::detail::tests::pane_on(
            &[(WorkingTreeSection::Unstaged, &["a.rs"][..])],
            0,
            0,
        ),
        ..AppState::default()
    };
    state.detail.focus = DetailFocus::Files;
    state
}

/// `S` from the changes pane opens the stash list — it did nothing there —
/// and the keyboard goes with it: the next `j` is the list's, not the pane's.
#[test]
fn s_opens_the_stash_list_from_the_changes_pane() {
    use graft_ui::detail::DetailFocus;
    let (open, commands) = update(&pane_focused(), key(KeyCode::Char('S')));
    assert!(open.stash.shown, "the list is open");
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadStashes(_))),
        "{commands:?}"
    );
    assert_eq!(open.detail.focus, DetailFocus::Graph);
    let (after, _) = update(&open, key(KeyCode::Char('j')));
    assert!(after.stash.shown, "`j` does not close it");
    assert_eq!(after.detail, open.detail, "`j` is not the pane's");
}

/// `J` likewise opens the journal from the changes pane.
#[test]
fn j_opens_the_journal_from_the_changes_pane() {
    use graft_ui::detail::DetailFocus;
    let (open, commands) = update(&pane_focused(), key(KeyCode::Char('J')));
    assert!(open.journal.shown, "the journal is open");
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadJournal(_))),
        "{commands:?}"
    );
    assert_eq!(open.detail.focus, DetailFocus::Graph);
    let (after, _) = update(&open, key(KeyCode::Char('j')));
    assert!(after.journal.shown, "`j` does not close it");
    assert_eq!(after.detail, open.detail, "`j` is not the pane's");
}
