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

/// B1 `:1103-1104`'s two wordings of the ignore line, at the list's 2.
const SKIPPING: &str = "skipping 2 commits listed in .git-blame-ignore-revs · i shows them";
const SHOWING: &str = "showing every commit · i skips the 2 in .git-blame-ignore-revs";

/// The job's answer with the ignore list `applied` or not.
fn ignore_answer(applied: bool) -> BlameAnswer {
    let mut blame = a_blame(true, false);
    if let Some(found) = blame.ignore_revs.as_mut() {
        found.applied = applied;
    }
    BlameAnswer { blame, line: None }
}

/// The frame at 120×40, as text.
fn screen(state: &AppState) -> String {
    crate::view::tests::draw(state, 120, 40)
}

#[test]
fn i_toggles_ignore_revs_and_the_line_says_which() {
    let state = place_on_screen(pane_at(22), false, true);
    assert!(screen(&state).contains(SKIPPING), "{}", screen(&state));
    let (next, commands) = press(&state, KeyCode::Char('i'));
    let (job, request) = blame_job(&commands);
    assert!(!request.options.ignore_revs);
    assert!(!next.trail.top().expect("the place").options.ignore_revs);
    let (off, _) = answer(&next, job, Ok(ignore_answer(false)));
    assert!(screen(&off).contains(SHOWING), "{}", screen(&off));

    let (again, commands) = press(&off, KeyCode::Char('i'));
    let (job, request) = blame_job(&commands);
    assert!(request.options.ignore_revs);
    let (on, _) = answer(&again, job, Ok(ignore_answer(true)));
    assert!(screen(&on).contains(SKIPPING), "{}", screen(&on));
}

/// Review Minor 7: the line and `i` do not vanish while git re-reads — the
/// list did not change, so its count is the last answer's.
#[test]
fn the_ignore_line_and_i_stay_through_a_re_read() {
    let state = place_on_screen(pane_at(22), false, true);
    let (reading, commands) = press(&state, KeyCode::Char('i'));
    let (job, _) = blame_job(&commands);
    assert!(
        screen(&reading).contains(SHOWING),
        "worded for the option just chosen:\n{}",
        screen(&reading)
    );
    assert!(
        super::hints::bar(&reading).contains(&("i", "ignore list")),
        "{:?}",
        super::hints::bar(&reading)
    );

    // `i` during the read asks again, as `w` does.
    let (again, commands) = press(&reading, KeyCode::Char('i'));
    assert!(commands.contains(&Command::CancelJob(job)), "{commands:?}");
    let (_, request) = blame_job(&commands);
    assert!(request.options.ignore_revs, "back on");
    assert!(screen(&again).contains(SKIPPING), "{}", screen(&again));
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

/// The answer the job gives `request`: `line` only when it was asked where
/// the line came from (`jobs::detail::read_blame`).
fn job_answer(request: &BlameRequest, line: u32) -> BlameAnswer {
    BlameAnswer {
        blame: a_blame(false, false),
        line: request.came_from.as_ref().map(|_| line),
    }
}

#[test]
fn caret_then_w_before_the_answer_keeps_the_came_from_line() {
    let state = blamed_at(22);
    let (reading, _) = press(&state, KeyCode::Char('^'));
    let (again, commands) = press(&reading, KeyCode::Char('w'));
    let (job, request) = blame_job(&commands);
    assert!(
        request.came_from.is_some(),
        "`w` asks where it came from too"
    );
    let (read, _) = answer(&again, job, Ok(job_answer(&request, 9)));
    assert_eq!(cursor_line(&read), 9, "the came-from line, not line 1");
}

#[test]
fn a_came_from_answer_without_a_line_says_so() {
    let state = blamed_at(22);
    let (reading, commands) = press(&state, KeyCode::Char('^'));
    let (job, _) = blame_job(&commands);
    let (read, _) = answer(
        &reading,
        job,
        Ok(BlameAnswer {
            blame: a_blame(false, false),
            line: None,
        }),
    );
    let top = read.trail.on_screen().expect("the older place");
    assert_eq!(
        top.body,
        BlameLoad::Failed(format!(
            "can't tell where line 22 came from in {} · esc back",
            short(lina())
        ))
    );
    let (back, _) = press(&read, KeyCode::Escape);
    assert_eq!(back.trail.places.len(), 1, "back to the newer blame");
    assert_eq!(cursor_line(&back), 22, "on the same line");
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

/// git's words are the outside world's: a bidi override or an escape must
/// not reach the terminal (review Minor 5; as `view/finish.rs:32`).
#[test]
fn blame_failure_words_are_sanitised() {
    let raw = "fatal: \u{202E}evil\x1b[31m red";
    let mut state = blamed_at(22);
    if let Some(blame) = state.trail.top_mut() {
        blame.body = BlameLoad::Failed(raw.to_owned());
    }
    let blame = state.trail.on_screen().expect("the place");
    let shown = blame.view("a.rs", 0, |view| match view.body {
        mergewright_ui::blame::BlameBody::Failed(words) => words.to_owned(),
        _ => panic!("not failed"),
    });
    assert_eq!(
        shown,
        mergewright_term::sanitize::sanitize_line(raw).as_str(),
        "the words as sanitize_line makes them"
    );
    assert!(
        !shown.contains('\u{202E}') && !shown.contains('\x1b'),
        "{shown:?}"
    );
}

// ---- is_write ----

#[test]
fn a_blame_read_is_not_a_write() {
    let (_, commands) = press(&pane_at(22), KeyCode::Char('B'));
    assert!(!commands.iter().any(is_write), "{commands:?}");
}
