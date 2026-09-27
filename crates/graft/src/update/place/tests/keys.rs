//! 20's Keys and States boxes: routing, find, `w`/`i`, reading, the mouse.

use super::*;

#[test]
fn slash_in_blame_finds_in_the_file_not_the_graph() {
    let state = blamed_at(22);
    let typed = press_all(
        &state,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('r'),
            KeyCode::Char('e'),
            KeyCode::Enter,
        ],
    );
    assert_eq!(
        typed.trail.on_screen().and_then(|blame| blame.find.clone()),
        Some("re".to_owned())
    );
    assert!(typed.search.is_none(), "the graph search never opened");
}

#[test]
fn graph_verbs_are_inert_in_blame() {
    let state = blamed_at(22);
    for verb in ['m', 'r', 'a', 'c', 'o', 'u'] {
        let (next, commands) = press(&state, KeyCode::Char(verb));
        assert_eq!(next.trail, state.trail, "`{verb}` moved the place");
        assert!(
            next.preview.is_none() && next.commit_editor.is_none(),
            "`{verb}` reached the graph"
        );
        assert!(
            !commands.iter().any(is_write),
            "`{verb}` launched a write: {commands:?}"
        );
    }
}

#[test]
fn l_and_w_in_blame_never_reach_the_diff_pane() {
    let state = blamed_at(22);
    for code in [KeyCode::Char('l'), KeyCode::Char('w')] {
        let (next, _) = press(&state, code);
        assert_eq!(next.detail, state.detail, "{code:?} reached the pane");
    }
}

#[test]
fn esc_in_blame_never_cancels_a_running_write() {
    let mut state = blamed_at(22);
    state.status = Status::Busy("merging".to_owned());
    let (next, commands) = press(&state, KeyCode::Escape);
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::CancelJob(_))),
        "a ready place has no job to stop, and the write is not the place's: {commands:?}"
    );
    assert_eq!(next.status, state.status, "the write is still running");
    assert!(!next.trail.is_live(), "Esc went back along the trail");
}

#[test]
fn find_and_n_move_between_matches() {
    let mut state = blamed_at(22);
    if let Some(blame) = state.trail.top_mut() {
        blame.cursor = 0;
    }
    let found = press_all(
        &state,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('r'),
            KeyCode::Char('e'),
            KeyCode::Char('t'),
            KeyCode::Enter,
        ],
    );
    assert_eq!(cursor_line(&found), 22);
    let next = press(&found, KeyCode::Char('n')).0;
    assert_eq!(cursor_line(&next), 27);
    let wrapped = press(&next, KeyCode::Char('n')).0;
    assert_eq!(cursor_line(&wrapped), 22);
    let back = press(&wrapped, KeyCode::Char('N')).0;
    assert_eq!(cursor_line(&back), 27);
}

#[test]
fn n_without_a_find_does_nothing() {
    let state = blamed_at(22);
    assert_eq!(
        press(&state, KeyCode::Char('n')),
        press(&state, KeyCode::Function(12))
    );
}

#[test]
fn esc_clears_the_find_before_popping() {
    let state = place_on_screen(pane_at(22), true, false);
    let (cleared, _) = press(&state, KeyCode::Escape);
    assert!(cleared.trail.is_live(), "still in blame");
    assert_eq!(
        cleared
            .trail
            .on_screen()
            .and_then(|blame| blame.find.clone()),
        None
    );
}

#[test]
fn i_toggles_ignore_revs_and_the_line_says_which() {
    let state = place_on_screen(pane_at(22), false, true);
    let (next, commands) = press(&state, KeyCode::Char('i'));
    let (_, request) = blame_job(&commands);
    assert!(!request.options.ignore_revs);
    assert!(!next.trail.top().expect("the place").options.ignore_revs);
}

#[test]
fn i_is_unbound_without_the_file() {
    let state = place_on_screen(pane_at(22), false, false);
    // Measured against a key nothing binds: `update` does ambient work on
    // every event (the evil-merge probe).
    assert_eq!(
        press(&state, KeyCode::Char('i')),
        press(&state, KeyCode::Function(12))
    );
}

#[test]
fn w_toggles_whitespace() {
    let state = blamed_at(22);
    let (next, commands) = press(&state, KeyCode::Char('w'));
    let (_, request) = blame_job(&commands);
    assert!(!request.options.ignore_whitespace);
    assert!(matches!(
        next.trail.top().expect("the place").body,
        BlameLoad::Reading {
            want: Want::Line(22),
            ..
        }
    ));
}

// ---- States ----

#[test]
fn blame_reading_title_then_lines() {
    let state = pane_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    assert!(matches!(
        opened.trail.top().expect("the place").body,
        BlameLoad::Reading { .. }
    ));
    let (job, _) = blame_job(&commands);
    let (read, _) = answer(
        &opened,
        job,
        Ok(BlameAnswer {
            blame: a_blame(false, false),
            line: None,
        }),
    );
    assert_eq!(read.trail.top().expect("the place").row_count(), 30);
}

#[test]
fn esc_while_reading_cancels_the_job() {
    let state = pane_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, _) = blame_job(&commands);
    let (back, commands) = press(&opened, KeyCode::Escape);
    assert!(!back.trail.is_live());
    assert!(commands.contains(&Command::CancelJob(job)), "{commands:?}");
}

#[test]
fn blame_failed_shows_gits_words() {
    let state = pane_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, _) = blame_job(&commands);
    let (failed, _) = answer(
        &opened,
        job,
        Err(BlameFailure::Failed("fatal: no such path".to_owned())),
    );
    assert_eq!(
        failed.trail.top().expect("the place").body,
        BlameLoad::Failed("fatal: no such path".to_owned())
    );
}

// ---- is_write ----

#[test]
fn a_blame_read_is_not_a_write() {
    let (_, commands) = press(&pane_at(22), KeyCode::Char('B'));
    assert!(!commands.iter().any(is_write), "{commands:?}");
}

fn click(row: u16) -> AppEvent {
    use graft_core::event::{Event, Modifiers, MouseButton, MouseInput, MouseKind};
    AppEvent::Core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 60,
        row,
        modifiers: Modifiers::NONE,
    }))
}

#[test]
fn mouse_double_click_is_enter() {
    let mut state = blamed_at(22);
    state.harness.clock = crate::session::Clock::Fixed(1_000);
    // Row 0 is the title; screen row 5 is the fifth line (top 0).
    let (once, _) = update(&state, click(5));
    assert_eq!(cursor_line(&once), 5);
    assert!(once.trail.on_screen().is_some(), "one click only selects");
    let (twice, _) = update(&once, click(5));
    assert!(twice.trail.hidden(), "a double-click is `⏎`");
    assert_eq!(
        twice.line_landing.as_ref().map(|landing| landing.spec),
        Some(DetailSpec::Commit(mo()))
    );
}

#[test]
fn a_click_in_blame_never_reaches_the_graph() {
    let state = blamed_at(22);
    let cursor = |state: &AppState| state.history.as_ref().map(|history| history.cursor);
    for row in [0, 1, 5, 20, 39] {
        let (next, _) = update(&state, click(row));
        assert_eq!(cursor(&next), cursor(&state), "row {row} moved the graph");
        assert_eq!(next.detail, state.detail, "row {row} reached the pane");
    }
}
