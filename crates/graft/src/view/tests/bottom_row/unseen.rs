//! No failure goes unseen under a box (m6-conflicts/78, Decided 1): a
//! message that arrives while a box covers the bottom row waits for the box
//! to close; a failure is shown when it closes, even if the box's own act
//! started something new. Nothing is acknowledged while the row cannot show
//! it.

use graft_core::job::JobId;

use super::*;
use crate::update::{Command, Product};

const FETCH_FAILED: &str = "fetch failed: could not resolve host";

/// The graph, no pane, `f` launched for real: the fetch is the running
/// write. Returns its id.
fn fetching(columns: u16, rows: u16) -> (AppState, JobId) {
    let mut state = changes_pane_state(columns, rows);
    state.detail.shown = false;
    state.on_working_tree = false;
    let (state, commands) =
        crate::update::update(&state, AppEvent::Core(Event::Key(key(KeyCode::Char('f')))));
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::Operate(id, _) => Some(*id),
            _ => None,
        })
        .expect("the fetch launched");
    (state, id)
}

/// The fetch's own answer: it failed, as git said.
fn fetch_fails(state: &AppState, id: JobId) -> AppState {
    crate::update::update(
        state,
        AppEvent::Produced(
            id,
            Product::Operated(Box::new(crate::jobs::operate::Operated::Failed {
                error: FETCH_FAILED.to_owned(),
                refresh: None,
                remote: None,
                refused: None,
                edits: None,
            })),
        ),
    )
    .0
}

fn type_in(state: &AppState, text: &str) -> AppState {
    text.chars().fold(state.clone(), |state, character| {
        press(&state, KeyCode::Char(character))
    })
}

/// The failing test first: `f`, `b`, the fetch fails under the prompt. A
/// later answer replaces the status under the box; `esc` closes the prompt
/// and the row reads the fetch failure; `esc` again, the later line.
///
/// No second job can really answer while the fetch runs — one write at a
/// time (`claim_write`) — so the later `Done` is written onto the status
/// directly: it stands for any answer that replaces a failure under a box.
#[test]
fn a_failure_under_a_box_is_shown_when_it_closes() {
    for (columns, rows) in SIZES {
        let (state, fetch) = fetching(columns, rows);
        let boxed = press(&state, KeyCode::Char('b'));
        assert!(boxed.name_prompt.is_some(), "b opened the prompt");
        let mut failed = fetch_fails(&boxed, fetch);
        assert!(failed.name_prompt.is_some(), "the prompt is still up");
        assert_eq!(
            bottom_row(&failed, columns, rows),
            box_keys(&failed, columns),
            "{columns}x{rows}: the prompt owns the row"
        );
        failed.status = Status::done("fetched");

        let closed = press(&failed, KeyCode::Escape);
        assert!(closed.name_prompt.is_none(), "esc closed the prompt");
        let row = bottom_row(&closed, columns, rows);
        assert!(row.contains(FETCH_FAILED), "{columns}x{rows}: {row}");
        assert!(row.contains("esc dismiss"), "{columns}x{rows}: {row}");

        let dismissed = press(&closed, KeyCode::Escape);
        assert_eq!(dismissed.unseen, None);
        let row = bottom_row(&dismissed, columns, rows);
        assert!(row.contains("fetched ✓"), "{columns}x{rows}: {row}");
    }
}

/// The review's steps: `f`, `b`, the fetch fails, `topic ⏎`. The prompt's
/// act writes the create's `Busy` over the failure; the row draws the
/// failure with `esc dismiss`. `Esc` only dismisses it — the create is not
/// cancelled — and the row is the create's line.
#[test]
fn a_boxs_act_does_not_hide_a_failure() {
    for (columns, rows) in SIZES {
        let (state, fetch) = fetching(columns, rows);
        let boxed = press(&state, KeyCode::Char('b'));
        let failed = fetch_fails(&boxed, fetch);
        let typed = type_in(&failed, "topic");
        let (created, commands) =
            crate::update::update(&typed, AppEvent::Core(Event::Key(key(KeyCode::Enter))));
        assert!(created.name_prompt.is_none(), "the prompt closed");
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::Operate(..))),
            "the create launched: {commands:?}"
        );
        assert!(
            matches!(&created.status, Status::Busy(words) if words.contains("topic")),
            "{:?}",
            created.status
        );
        let row = bottom_row(&created, columns, rows);
        assert!(row.contains(FETCH_FAILED), "{columns}x{rows}: {row}");
        assert!(row.contains("esc dismiss"), "{columns}x{rows}: {row}");

        let (after, commands) =
            crate::update::update(&created, AppEvent::Core(Event::Key(key(KeyCode::Escape))));
        assert_eq!(after.operating, created.operating, "the create runs on");
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::CancelJob(..))),
            "not cancelled: {commands:?}"
        );
        assert_eq!(after.status, created.status);
        let row = bottom_row(&after, columns, rows);
        assert!(row.contains("topic"), "{columns}x{rows}: {row}");
        assert!(!row.contains(FETCH_FAILED), "{columns}x{rows}: {row}");
    }
}

/// A `Done` that arrives under a box is not acknowledged by keys the row
/// never showed it under; `esc` draws it; the next key acknowledges it. The
/// name prompt (its keys never reach `acknowledge`) and the lease box (its
/// keys go through `key_event`, which acknowledged before 78).
#[test]
fn a_done_under_a_box_is_not_acknowledged_unseen() {
    for (columns, rows) in SIZES {
        let (state, _) = fetching(columns, rows);
        let prompt = press(&state, KeyCode::Char('b'));
        let lease = press(&rejected_push(columns, rows), KeyCode::Char('P'));
        for (modal, mut boxed, keys) in
            vec![("name prompt", prompt, "abc"), ("lease box", lease, "xyz")].into_boxed_slice()
        {
            assert!(boxed.modal_owns_bottom_row(), "{modal}: open");
            // The fetch's own `Done` comes through `operated` with a
            // refresh; what matters here is the line under the box, so it
            // is written.
            boxed.status = Status::done("fetched");
            let typed = type_in(&boxed, keys);
            assert!(typed.modal_owns_bottom_row(), "{modal}: still open");
            assert_eq!(
                typed.status,
                Status::done("fetched"),
                "{modal} {columns}x{rows}: three keys typed"
            );
            let closed = press(&typed, KeyCode::Escape);
            assert!(!closed.modal_owns_bottom_row(), "{modal}: esc closed it");
            let row = bottom_row(&closed, columns, rows);
            assert!(row.contains("fetched ✓"), "{modal} {columns}x{rows}: {row}");
            let next = press(&closed, KeyCode::Char('j'));
            assert_eq!(next.status, Status::Ready, "{modal} {columns}x{rows}");
        }
    }
}

/// A failure that stood **before** the box opened is not held: the lease
/// box's act replaces it, as 69 Decided 2 has it.
#[test]
fn a_failure_that_stood_before_the_box_is_not_held() {
    for (columns, rows) in SIZES {
        let boxed = press(&rejected_push(columns, rows), KeyCode::Char('P'));
        assert!(boxed.confirm.is_some(), "P opened the lease box");
        let pushed = press(&boxed, KeyCode::Right);
        let pushed = press(&pushed, KeyCode::Enter);
        assert!(pushed.confirm.is_none(), "the box acted");
        assert_eq!(pushed.unseen, None, "{columns}x{rows}");
    }
}
