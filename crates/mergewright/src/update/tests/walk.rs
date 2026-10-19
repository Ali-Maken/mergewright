//! `update` tests: walk. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// What a walk says first: the order it is in.
pub(super) fn started(requested: Order, order: Order) -> AppEvent {
    AppEvent::Progress(
        JobId::new(0),
        Progress::History(HistoryUpdate::Started(Box::new(WalkFacts {
            requested,
            order,
            default_branch: None,
            integration: Vec::new().into(),
        }))),
    )
}

/// The same event, from job `id`.
pub(super) fn from(id: JobId, event: AppEvent) -> AppEvent {
    match event {
        AppEvent::Progress(_, progress) => AppEvent::Progress(id, progress),
        other => other,
    }
}

#[test]
fn rows_land_as_they_arrive_and_the_walk_completes() {
    let id = JobId::new(4);
    let state = awaiting(id);

    let (state, _) = update(&state, rows(id, 500));
    assert_eq!(progress(&state), (Phase::Streaming, 500));
    let (state, _) = update(&state, rows(id, 512));
    assert_eq!(progress(&state), (Phase::Streaming, 512));

    let (state, commands) = update(&state, core(Event::JobFinished(id)));
    assert_eq!(progress(&state), (Phase::Complete, 512));
    assert_eq!(state.awaiting, None);
    assert!(commands.is_empty());
}

/// Moving past the rows held asks the walk for the rows there, and
/// only then: moving within them asks for nothing.
#[test]
fn moving_past_the_window_asks_for_the_rows_there() {
    let id = JobId::new(4);
    let (state, _) = update(&awaiting(id), sized(80, 12));
    // Nine graph rows (ten lines, the cursor's second line takes one,
    // ease-pass/07b): the resize asked for them, and they arrive.
    let (state, _) = update(&state, rows_for(id, 2, 0..30, 500));

    let (state, commands) = update(&state, key(KeyCode::Char('j')));
    assert!(commands.is_empty(), "row 1 is held: {commands:?}");
    assert_eq!(state.history.as_ref().map(|h| h.cursor), Some(1));

    // Page 1 (9..18) is drawn from the rows held, but the second page
    // after it is not held: ask now, three screens either side, so
    // neither waits.
    let (_, commands) = update(&state, key(KeyCode::PageDown));
    assert_eq!(
        commands,
        vec![Command::ShowRows(id, request(3, Wanted::Rows(0..45)))]
    );
}

/// `G` on a walk not laid out to its end asks for the last rows and leaves
/// the viewport where it is; `Esc` then withdraws the jump — not the walk.
#[test]
fn esc_withdraws_a_jump_before_it_cancels_the_walk() {
    let id = JobId::new(4);
    let (state, _) = update(&awaiting(id), sized(80, 12));
    let (state, _) = update(&state, rows_for(id, 2, 0..30, 500));

    let (jumping, commands) = update(&state, key(KeyCode::Char('G')));
    assert_eq!(
        commands,
        vec![Command::ShowRows(id, request(3, Wanted::Last(18)))]
    );
    let history = jumping.history.as_ref().expect("a walk");
    assert_eq!((history.top, history.cursor, history.jump), (0, 0, Some(3)));

    let (withdrawn, commands) = update(&jumping, key(KeyCode::Escape));
    assert_eq!(
        commands,
        vec![Command::ShowRows(id, request(4, Wanted::Rows(0..36)))]
    );
    assert_eq!(withdrawn.awaiting, Some(id), "the walk goes on");
    assert_eq!(withdrawn.history.as_ref().and_then(|h| h.jump), None);

    let (after, _) = update(&withdrawn, rows_for(id, 3, 480..500, 500));
    assert_eq!(after, withdrawn, "the jump's answer does not land");
}

/// Job `id` reporting `laid_out` rows laid out for the first request,
/// before its window: the walk is still reading.
fn laid_out_so_far(id: JobId, laid_out: usize) -> AppEvent {
    AppEvent::Progress(
        id,
        Progress::History(HistoryUpdate::Progress {
            request: 1,
            laid_out,
        }),
    )
}

/// Still reading: the first screen asked for and not yet sent, 500 rows
/// laid out so far. A walk resting between asks is not cancelled
/// (m6-conflicts/85, `keys::tests::esc_on_a_paused_walk_does_nothing`).
#[test]
fn esc_cancels_the_walk_the_screen_is_waiting_for() {
    let id = JobId::new(4);
    let (state, _) = update(&awaiting(id), laid_out_so_far(id, 500));

    let (next, commands) = update(&state, key(KeyCode::Escape));

    assert_eq!(commands, vec![Command::CancelJob(id)]);
    assert_eq!(
        next.awaiting, None,
        "nothing is waited on once `Esc` has been pressed"
    );
    assert_eq!(progress(&next), (Phase::Cancelled, 500));
}

#[test]
fn esc_with_no_walk_running_does_nothing() {
    let before = AppState::default();

    let (next, commands) = update(&before, key(KeyCode::Escape));

    assert_eq!(next, before);
    assert!(commands.is_empty());
}

/// The race `Esc` has to survive: the walk lays out another chunk between
/// the key press and the cancel flag being read, and sends it anyway. It
/// must not appear, and neither may the walk's ending undo the cancel.
#[test]
fn no_chunk_lands_after_esc() {
    let id = JobId::new(4);
    let (state, _) = update(&awaiting(id), laid_out_so_far(id, 500));
    let (cancelled, _) = update(&state, key(KeyCode::Escape));

    let (next, commands) = update(&cancelled, rows(id, 500));
    assert_eq!(progress(&next), (Phase::Cancelled, 500));
    assert!(commands.is_empty());

    for ending in [Event::JobFinished(id), Event::JobCancelled(id)] {
        let (after, _) = update(&next, core(ending));
        assert_eq!(progress(&after), (Phase::Cancelled, 500));
    }
}

/// Two walks in flight: the older one's rows are about a question nobody
/// is asking any more.
#[test]
fn a_superseded_walk_loses_to_the_newer_one() {
    let old = JobId::new(1);
    let new = JobId::new(2);
    let (state, _) = update(&awaiting(old), rows(old, 3));
    let (state, _) = walk_started(&state, new);
    assert_eq!(progress(&state), (Phase::Streaming, 0));

    let (state, _) = update(&state, rows(old, 500));
    assert_eq!(
        progress(&state),
        (Phase::Streaming, 0),
        "stale rows dropped"
    );

    let (state, _) = update(&state, rows(new, 2));
    assert_eq!(progress(&state), (Phase::Streaming, 2));
}

/// ADR-0008 in the state: a repository with no commit-graph file answers
/// in commit-date order, and the screen is told the order is not final.
#[test]
fn a_walk_that_fell_back_to_date_order_says_it_is_provisional() {
    let id = JobId::new(7);

    let (next, _) = update(
        &awaiting(id),
        from(id, started(Order::Topological, Order::CommitDate)),
    );
    let walk = next.history.and_then(|history| history.walk);
    assert!(walk.expect("the walk said what it is").is_provisional());

    let (next, _) = update(
        &awaiting(id),
        from(id, started(Order::Topological, Order::Topological)),
    );
    let walk = next.history.and_then(|history| history.walk);
    assert!(!walk.expect("the walk said what it is").is_provisional());
}
