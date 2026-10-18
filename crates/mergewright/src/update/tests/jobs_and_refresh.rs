//! `update` tests: jobs and refresh. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

#[test]
fn a_failed_job_reaches_the_status_line() {
    let failure = JobFailure::Failed("not a git repository".to_owned());

    let (next, commands) = update(
        &AppState::default(),
        core(Event::JobFailed(JobId::new(9), failure)),
    );

    assert_eq!(
        next.status,
        Status::Failed("job 9 failed: not a git repository".to_owned())
    );
    assert!(commands.is_empty());
}

#[test]
fn a_panicking_job_says_so_and_names_its_report() {
    let failure = JobFailure::Panicked {
        message: "lane 3 vanished".to_owned(),
        report: Some("/state/graft/crash-1.log".to_owned()),
    };

    let (next, _) = update(
        &AppState::default(),
        core(Event::JobFailed(JobId::new(2), failure)),
    );

    assert_eq!(
        next.status,
        Status::Failed(
            "job 2 failed: panicked: lane 3 vanished — report: /state/graft/crash-1.log".to_owned()
        )
    );
}

#[test]
fn a_job_that_worked_clears_a_stale_failure() {
    let failed = AppState {
        status: Status::Failed("job 1 failed: not a git repository".to_owned()),
        ..AppState::default()
    };

    let (next, _) = update(&failed, core(Event::JobFinished(JobId::new(2))));

    assert_eq!(next.status, Status::Ready);
}

#[test]
fn a_cancelled_job_leaves_the_status_line_alone() {
    // Cancellation is something Mergewright asked for. Reporting it would put an
    // error in front of the user when nothing is wrong (CLAUDE.md).
    let failed = AppState {
        status: Status::Failed("job 1 failed: not a git repository".to_owned()),
        ..AppState::default()
    };

    let (next, _) = update(&failed, core(Event::JobCancelled(JobId::new(1))));

    assert_eq!(next, failed);
}

#[test]
fn control_r_asks_for_the_repository_to_be_re_read() {
    let state = AppState {
        repo_path: PathBuf::from("/code/api"),
        ..AppState::default()
    };

    let (next, commands) = update(
        &state,
        core(Event::Key(KeyPress::control(KeyCode::Char('r')))),
    );

    assert_eq!(
        commands,
        vec![
            // The journal is re-read too: another Mergewright on the same worktree
            // can have appended to it (m3-working-tree-undo-and-stash/06).
            Command::ReadJournal(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 1)),
            Command::OpenRepository,
            // The walk runs under an id `update` minted here, so a second
            // `Ctrl+R` before its `Started` lands can still name it
            // (m5-history-and-ghost-preview/05).
            Command::ReadHistory(
                JobId::new(crate::update::detail::LOCAL_JOB_BIT | 2),
                Order::Topological,
            ),
            Command::ReadRefs(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 3)),
            // The remote facts, beside the refs (m4-remotes/05).
            Command::ReadRemotes(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 4)),
            Command::ReadWorkingTree(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 5)),
        ]
    );
    // Asking for work is all it does — apart from remembering which job the
    // journal read is, so a stale answer can be dropped by id. Everything
    // else is the jobs' to change, once they come back.
    let expected = AppState {
        journal: crate::update::journal::JournalState {
            entries: crate::update::detail::Loading::Pending(JobId::new(
                crate::update::detail::LOCAL_JOB_BIT | 1,
            )),
            reading: Some(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 1)),
            ..state.journal.clone()
        },
        remotes: crate::update::detail::Loading::Pending(JobId::new(
            crate::update::detail::LOCAL_JOB_BIT | 4,
        )),
        remote: crate::update::remote::RemoteState {
            reading: Some(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 4)),
            ..crate::update::remote::RemoteState::default()
        },
        // The walk is named at once, not when its `Started` event lands.
        awaiting: Some(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 2)),
        // So is the refs read: `P` waits on it after a rejection
        // (m6-conflicts/49).
        refs_read: RefsRead::Pending(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 3)),
        // The working-tree read is named too: the status waits on it.
        status_read: Some(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 5)),
        // The opening is asked beside the walk: the walk's first request
        // waits for HEAD (ease-pass 06 D2).
        facts_pending: true,
        local_jobs: next.local_jobs,
        ..state.clone()
    };
    assert_eq!(next, expected);
}

/// `Ctrl+R`, and the job id the walk it asked for will run under.
fn refresh(state: &AppState) -> (AppState, Vec<Command>, JobId) {
    let (next, commands) = update(
        state,
        core(Event::Key(KeyPress::control(KeyCode::Char('r')))),
    );
    let walk = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadHistory(id, _) => Some(*id),
            _ => None,
        })
        .expect("a refresh asks for a history walk");
    (next, commands, walk)
}

/// The `Ctrl+R` race (ADR-0037 §6, m5-history-and-ghost-preview/05).
///
/// Two refreshes with nothing fed back in between — no `Started`, no window,
/// nothing. The second one still has to cancel the walk the first one asked
/// for, which it can only do if that walk was named when it was asked for.
/// Left uncancelled, the walk parks on a worker for the rest of the run.
///
/// Deterministic by construction: this is `update` alone, so the ordering
/// under test is the one written here, not one a scheduler happened to pick.
#[test]
fn a_refresh_before_the_previous_walk_started_still_cancels_it() {
    let state = AppState {
        repo_path: PathBuf::from("/code/api"),
        ..AppState::default()
    };

    let (after_first, _, first) = refresh(&state);
    let (after_second, second_commands, second) = refresh(&after_first);

    assert_eq!(
        after_first.awaiting,
        Some(first),
        "the walk is named the moment it is asked for, not when it reports"
    );
    assert_ne!(first, second, "each refresh asks for its own walk");
    assert!(
        second_commands.contains(&Command::CancelJob(first)),
        "the second refresh must cancel the first refresh's walk: {second_commands:?}"
    );
    assert_eq!(after_second.awaiting, Some(second));
}

/// A walk that has been superseded still announces itself — the announcement
/// was already in flight when it was cancelled. Taking it would blank the
/// rows the live walk has already put on screen.
#[test]
fn stale_started_is_dropped() {
    let state = AppState {
        repo_path: PathBuf::from("/code/api"),
        ..AppState::default()
    };
    let (after_first, _, first) = refresh(&state);
    let (mut after_second, _, second) = refresh(&after_first);
    // The live walk has answered: the screen holds rows.
    let mut live = History::started();
    live.rows = 40;
    live.phase = Phase::Complete;
    after_second.history = Some(live.clone());

    let (next, commands) = update(&after_second, AppEvent::Started(first, Task::ReadHistory));

    assert_eq!(
        next.history,
        Some(live),
        "a superseded walk's `Started` must leave the rows on screen alone"
    );
    assert_eq!(next.awaiting, Some(second), "the live walk keeps the name");
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ShowRows(id, _) if *id == first)),
        "no rows are asked of a walk that was cancelled: {commands:?}"
    );
}

#[test]
fn a_bare_r_asks_for_nothing() {
    // `r` is rebase on the history screen (DESIGN.md §2.3), not refresh.
    let (_, commands) = update(&AppState::default(), key(KeyCode::Char('r')));

    assert!(commands.is_empty());
}
