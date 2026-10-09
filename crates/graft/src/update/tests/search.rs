//! `update` tests: search. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

fn hit(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

pub(super) fn found(id: JobId, hits: &[u8]) -> AppEvent {
    AppEvent::Progress(
        id,
        Progress::Search(hits.iter().map(|byte| hit(*byte)).collect()),
    )
}

pub(super) fn typed(state: &AppState, text: &str) -> (AppState, Vec<Command>) {
    let mut state = state.clone();
    let mut commands = Vec::new();
    for code in std::iter::once(KeyCode::Char('/'))
        .chain(text.chars().map(KeyCode::Char))
        .chain(std::iter::once(KeyCode::Enter))
    {
        let (next, more) = update(&state, key(code));
        state = next;
        commands.extend(more);
    }
    (state, commands)
}

/// A walk `walk` with rows 0..30 on hand, commit `i + 1` on row `i`, of 500
/// laid out; ten graph rows.
pub(super) fn on_hand(walk: JobId) -> AppState {
    let (state, _) = update(&awaiting(walk), sized(80, 12));
    let AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(mut window))) =
        rows_for(walk, 2, 0..30, 500)
    else {
        unreachable!("`rows_for` answers with a window");
    };
    window.rows = (0..30u8)
        .map(|at| {
            let mut row = window.rows[0].clone();
            row.commit.id = hit(at + 1);
            row
        })
        .collect();
    let (state, _) = update(
        &state,
        AppEvent::Progress(walk, Progress::History(HistoryUpdate::Window(window))),
    );
    state
}

fn search_of(state: &AppState) -> &Search {
    state.search.as_ref().expect("a search")
}

#[test]
fn slash_opens_a_prompt_that_takes_every_letter_and_enter_searches() {
    let state = on_hand(JobId::new(4));
    let (state, _) = update(&state, key(KeyCode::Char('/')));
    let (state, _) = update(&state, key(KeyCode::Char('q')));
    assert_eq!(state.quit, None, "q is text while the prompt is open");
    let (state, _) = update(&state, key(KeyCode::Char('x')));
    let (state, _) = update(&state, key(KeyCode::Backspace));
    assert_eq!(state.search_prompt.as_deref(), Some("q"));

    let (state, commands) = update(&state, key(KeyCode::Enter));
    assert_eq!(state.search_prompt, None);
    assert_eq!(
        commands,
        vec![Command::Search(Query::parse("q").expect("q"))]
    );
    assert_eq!(search_of(&state).state, SearchState::Running { hits: 0 });
}

#[test]
fn an_empty_query_or_esc_closes_the_prompt_without_searching() {
    let state = on_hand(JobId::new(4));
    let (closed, commands) = typed(&state, "  ");
    assert_eq!(
        (closed.search_prompt.clone(), closed.search.clone()),
        (None, None)
    );
    assert!(commands.is_empty());
    let (open, _) = update(&state, key(KeyCode::Char('/')));
    let (closed, commands) = update(&open, key(KeyCode::Escape));
    assert_eq!((closed.search_prompt, commands), (None, vec![]));
}

/// Hits count as partial while the job runs, the first is jumped to at
/// once, and the count settles when the job finishes.
#[test]
fn the_count_is_partial_while_running_and_settles_when_done() {
    let walk = JobId::new(4);
    let (state, _) = typed(&on_hand(walk), "fix");
    let job = JobId::new(9);
    let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
    let (state, _) = update(&state, found(job, &[3]));
    assert_eq!(search_of(&state).state.describe(), "1 match so far");
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(history.cursor, 2, "the first hit is selected at once");
    assert!(search_of(&state).line(&state.keymap).contains("(1/1)"));

    let (state, _) = update(&state, found(job, &[12, 20]));
    assert_eq!(search_of(&state).state.describe(), "3 matches so far");
    let (state, _) = update(&state, core(Event::JobFinished(job)));
    assert_eq!(search_of(&state).state, SearchState::Done { hits: 3 });
    assert_eq!(search_of(&state).state.describe(), "3 matches");
}

#[test]
fn n_and_capital_n_walk_the_hits_and_wrap() {
    let walk = JobId::new(4);
    let (state, _) = typed(&on_hand(walk), "fix");
    let job = JobId::new(9);
    let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
    let (state, _) = update(&state, found(job, &[3, 12, 20]));
    let cursor = |state: &AppState| state.history.as_ref().map(|h| h.cursor);
    assert_eq!(cursor(&state), Some(2));
    let (state, _) = update(&state, key(KeyCode::Char('n')));
    assert_eq!(cursor(&state), Some(11));
    let (state, _) = update(&state, key(KeyCode::Char('n')));
    let (state, _) = update(&state, key(KeyCode::Char('n')));
    assert_eq!(cursor(&state), Some(2), "n wraps past the last hit");
    let (state, _) = update(&state, key(KeyCode::Char('N')));
    assert_eq!(cursor(&state), Some(19), "N wraps before the first");
}

/// A hit not in the rows on hand is a find request to the walk; the
/// answer lands the cursor on it.
#[test]
fn a_far_hit_asks_the_walk_to_lay_out_to_it() {
    let walk = JobId::new(4);
    let (state, _) = typed(&on_hand(walk), "fix");
    let job = JobId::new(9);
    let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
    let (state, commands) = update(&state, found(job, &[200]));
    let [Command::ShowRows(to, asked)] = commands.as_slice() else {
        panic!("one request: {commands:?}");
    };
    assert_eq!(*to, walk);
    assert!(matches!(asked.wanted, Wanted::Find { commit, .. } if commit == hit(200)));
    assert_eq!(search_of(&state).seeking, Some(hit(200)));
}

/// Results from a superseded search job are dropped: its start is
/// cancelled, and its hits never count.
#[test]
fn a_superseded_search_jobs_hits_are_discarded() {
    let walk = JobId::new(4);
    let (state, _) = typed(&on_hand(walk), "fix");
    // Typed again before the first search's job said it started.
    let (state, _) = typed(&state, "bug");
    let (old, new) = (JobId::new(9), JobId::new(10));
    let (state, commands) = update(&state, AppEvent::Started(old, Task::Search));
    assert_eq!(commands, vec![Command::CancelJob(old)]);
    let (state, _) = update(&state, AppEvent::Started(new, Task::Search));
    let (state, _) = update(&state, found(old, &[3]));
    assert_eq!(
        search_of(&state).hits(),
        0,
        "the old job's hits are dropped"
    );
    let (state, _) = update(&state, core(Event::JobFinished(old)));
    assert_eq!(search_of(&state).state, SearchState::Running { hits: 0 });
    assert_eq!(search_of(&state).job, Some(new));
    assert_eq!(search_of(&state).query.text(), "bug");
}

#[test]
fn esc_cancels_a_running_search_and_restores_the_selection() {
    let walk = JobId::new(4);
    let (state, _) = typed(&on_hand(walk), "fix");
    let job = JobId::new(9);
    let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
    let (state, _) = update(&state, found(job, &[6]));
    assert_eq!(state.history.as_ref().map(|h| h.cursor), Some(5));

    let (state, commands) = update(&state, key(KeyCode::Escape));
    assert!(commands.contains(&Command::CancelJob(job)), "{commands:?}");
    assert_eq!(
        state.history.as_ref().map(|h| (h.cursor, h.top)),
        Some((0, 0))
    );
    assert_eq!(search_of(&state).state, SearchState::Cancelled { hits: 1 });
    assert_eq!(state.awaiting, Some(walk), "the walk goes on");

    let (state, _) = update(&state, core(Event::JobCancelled(job)));
    assert_eq!(search_of(&state).state, SearchState::Cancelled { hits: 1 });
    let (state, _) = update(&state, key(KeyCode::Escape));
    assert_eq!(state.search, None, "a second Esc closes the line");
}

/// No matches, cancelled and failed are three states that read
/// differently, and a failure stays on the search's line.
#[test]
fn no_matches_cancelled_and_failed_are_distinct() {
    let walk = JobId::new(4);
    let (state, _) = typed(&on_hand(walk), "fix");
    let job = JobId::new(9);
    let (state, _) = update(&state, AppEvent::Started(job, Task::Search));

    let (none, _) = update(&state, core(Event::JobFinished(job)));
    let (cancelled, _) = update(&state, core(Event::JobCancelled(job)));
    let failure = JobFailure::Failed("walk broke".to_owned());
    let (failed, _) = update(&state, core(Event::JobFailed(job, failure)));

    // References, not the states: three `AppState`s outgrow clippy's
    // stack-array limit (m6-conflicts/82 added `clash_index`).
    let lines = [&none, &cancelled, &failed].map(|state| search_of(state).state.describe());
    assert_eq!(lines[0], "no matches");
    assert_eq!(lines[1], "search cancelled, 0 matches found");
    assert_eq!(lines[2], "search failed: walk broke");
}

#[test]
fn a_handed_back_key_ends_the_run_naming_the_key() {
    let state = AppState {
        harness: Harness {
            hand_back: vec![KeyCode::Char(']')],
            clock: crate::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
            ..Harness::default()
        },
        ..AppState::default()
    };
    let (next, commands) = update(&state, key(KeyCode::Char(']')));
    assert_eq!(next.quit, Some(Quit::HandedBack(KeyCode::Char(']'))));
    assert!(commands.is_empty());
    assert!(next.should_quit());

    let (next, _) = update(&state, key(KeyCode::Char('j')));
    assert_eq!(next.quit, None, "other keys are the graph's");
}

#[test]
fn an_open_search_prompt_keeps_a_handed_back_key_as_text() {
    let state = AppState {
        harness: Harness {
            hand_back: vec![KeyCode::Char(']')],
            clock: crate::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
            ..Harness::default()
        },
        search_prompt: Some(String::from("fox")),
        ..AppState::default()
    };
    let (next, _) = update(&state, key(KeyCode::Char(']')));
    assert_eq!(next.quit, None);
    assert_eq!(next.search_prompt.as_deref(), Some("fox]"));
}
