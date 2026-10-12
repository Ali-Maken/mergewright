//! m7-power-tools/30o-b R3: one undo outcome. `u`/`U` in J and `⏎` on the
//! `← u`/`← U` row read the same answer: launched → J closes; asked (the
//! undo-after-push box) → J stays under the box; refused → J stays, the
//! words on its status row. Keys go through the whole router
//! (`update::update`), and what he sees is read from the frame.

use graft_core::event::{Event, KeyCode, KeyPress};

use super::*;

/// One key through the whole router, as the loop sends it.
fn key(state: &AppState, code: KeyCode) -> (AppState, Vec<Command>) {
    crate::update::update(
        state,
        crate::update::AppEvent::Core(Event::Key(KeyPress::plain(code))),
    )
}

fn frame(state: &AppState) -> Vec<String> {
    crate::view::tests::draw(state, 120, 30)
        .lines()
        .map(|line| line.trim_matches('"').trim_end().to_owned())
        .collect()
}

/// J's status row: the line above the legend.
fn status_row(frame: &[String]) -> &str {
    let legend = frame
        .iter()
        .rposition(|line| line.starts_with("● done in Mergewright"))
        .expect("J's legend is drawn");
    &frame[legend - 1]
}

fn in_progress(kind: graft_git::state::InProgress) -> graft_git::state::InProgressDetail {
    graft_git::state::InProgressDetail {
        kind,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    }
}

/// `main` committed, then pushed: `u` would take back work the remote has,
/// so it asks first. J open, newest first: `[push, commit ← u]`.
fn j_after_a_push() -> AppState {
    let mut commit = entry(1, OperationKind::Commit, "commit \"to push\"");
    commit
        .refs_before
        .insert("refs/heads/main".to_owned(), Some(oid(1)));
    commit
        .refs_after
        .insert("refs/heads/main".to_owned(), Some(oid(2)));
    let mut push = entry(2, OperationKind::Push, "push main → origin/main");
    push.touches.refs = vec!["refs/remotes/origin/main".to_owned()];
    let mut state = AppState::default();
    state.journal.entries = Loading::Ready(Arc::new(JournalRead {
        entries: vec![commit, push],
        unreadable: Vec::new(),
        live: Vec::new(),
        interrupted_edits: Vec::new(),
    }));
    state.journal.shown = true;
    put_cursor(&mut state, 1);
    state
}

/// `⏎` on the `← u` row while a rebase is stopped (a merge begun in the
/// shell lets `u` through, keymap §2.6a): refused, and the refusal
/// is on J's status row with J still up — not on a graph he can't see.
#[test]
fn enter_on_the_undo_row_during_an_operation_keeps_j() {
    let mut state = state_with_two();
    state.journal.shown = true;
    state.in_progress = Some(in_progress(graft_git::state::InProgress::Rebase));
    let (state, commands) = key(&state, KeyCode::Enter);
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Undo(..) | Command::Redo(..))),
        "{commands:?}"
    );
    let frame = frame(&state);
    let body = frame.join("\n");
    assert!(state.journal.shown, "J still up:\n{body}");
    assert!(frame[0].starts_with("JOURNAL"), "J on screen:\n{body}");
    assert!(
        status_row(&frame).contains("finish or abort the rebase first · ⇥ banner"),
        "the refusal on J's status row:\n{body}"
    );
}

/// `⏎` on the `← u` row after a push: the box, over J.
#[test]
fn enter_on_the_undo_row_after_a_push_opens_the_box_over_j() {
    let (state, commands) = key(&j_after_a_push(), KeyCode::Enter);
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Undo(..))),
        "nothing runs yet: {commands:?}"
    );
    assert!(state.confirm.is_some(), "the undo-after-push box");
    let frame = frame(&state);
    let body = frame.join("\n");
    assert!(state.journal.shown, "J stays under the box:\n{body}");
    assert!(frame[0].starts_with("JOURNAL"), "J's title on top:\n{body}");
    assert!(
        body.contains("Undo after a push"),
        "the box drawn over J:\n{body}"
    );
}

/// `u` in J after a push, then `[ Undo locally ]`: the undo launches, and
/// J closes as it does when `u` launches at once.
#[test]
fn u_in_j_after_a_push_then_undo_locally_closes_j() {
    let (state, _) = key(&j_after_a_push(), KeyCode::Char('u'));
    assert!(state.confirm.is_some(), "the box");
    assert!(state.journal.shown, "J under the box");
    let (state, _) = key(&state, KeyCode::Right);
    let (state, commands) = key(&state, KeyCode::Enter);
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::Undo(_, false, ..))),
        "the undo launched: {commands:?}"
    );
    assert!(!state.journal.shown, "J closed: the result is on the graph");
}

/// `Esc` on the box brings J back as it was, on the same row.
#[test]
fn esc_on_the_push_box_from_j_returns_to_j_on_the_same_row() {
    let before = j_after_a_push();
    let row = before.journal.selected();
    let (state, _) = key(&before, KeyCode::Char('u'));
    assert!(state.confirm.is_some(), "the box");
    let (state, commands) = key(&state, KeyCode::Escape);
    assert!(state.confirm.is_none(), "the box went");
    assert!(state.journal.shown, "J is back");
    assert_eq!(state.journal.selected(), row, "on the same row");
    assert!(commands.is_empty(), "nothing ran: {commands:?}");
}

/// V1: `u` while a write runs used to say nothing (`undo::launch` returned
/// no command and left the status alone). It says to wait, on the graph and
/// on J's status row — J staying up.
#[test]
fn u_while_a_write_runs_says_wait() {
    let mut graph = state_with_two();
    graph.operating = Some(JobId::new(77));
    let (after, commands) = key(&graph, KeyCode::Char('u'));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(
        after.operating,
        Some(JobId::new(77)),
        "the write keeps its slot"
    );
    assert_eq!(
        after.status,
        Status::Failed("wait for the write to finish".to_owned()),
        "said on the graph"
    );

    let mut j = graph;
    j.journal.shown = true;
    let (after, _) = key(&j, KeyCode::Char('u'));
    let frame = frame(&after);
    let body = frame.join("\n");
    assert!(after.journal.shown, "J still up:\n{body}");
    assert!(
        status_row(&frame).contains("wait for the write to finish"),
        "said on J's status row:\n{body}"
    );
}
