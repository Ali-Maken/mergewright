//! `q` asks first while a write runs; git stopped mid-operation quits with no
//! box (m8-polish-and-ship/13 ruling 4; 13b). Driven through `update`, so every
//! handler between the key and `graft_key` is in the path.

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_ops::operation::{FetchParams, Operation};

use crate::update::events::{AppEvent, Command};
use crate::update::state::{AppState, Quit};

pub(crate) fn press(state: &AppState, key: KeyPress) -> (AppState, Vec<Command>) {
    crate::update::update(state, AppEvent::Core(Event::Key(key)))
}

pub(crate) fn plain(character: char) -> KeyPress {
    KeyPress::plain(KeyCode::Char(character))
}

/// A fetch launched and still running.
fn fetching() -> AppState {
    let mut state = AppState::default();
    let mut commands = Vec::new();
    let mut next = 40;
    crate::update::operate::launch(
        &mut state,
        &mut commands,
        Operation::Fetch(FetchParams {
            remote: Some("origin".to_owned()),
            prune: false,
        }),
        &mut || {
            next += 1;
            JobId::new(next)
        },
    )
    .expect("nothing else runs");
    state
}

fn box_line(state: &AppState) -> String {
    let open = state.confirm.as_ref().expect("a box is up");
    open.view
        .message
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn q_with_nothing_running_quits() {
    let (next, commands) = press(&AppState::default(), plain('q'));
    assert_eq!(next.quit, Some(Quit::Requested));
    assert!(next.confirm.is_none());
    assert!(commands.is_empty());
}

#[test]
fn q_during_a_fetch_opens_the_box() {
    let state = fetching();
    let (next, commands) = press(&state, plain('q'));
    assert_eq!(next.quit, None, "not yet: the box asks");
    assert!(commands.is_empty(), "nothing stopped yet: {commands:?}");
    assert_eq!(
        box_line(&next),
        "fetch origin is still running \u{b7} quit stops it"
    );
    let open = next.confirm.as_ref().expect("the box");
    let labels: Vec<&str> = open
        .view
        .choices
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(labels, ["Cancel", "Quit"]);
}

/// `[ Quit ]` (here by `→ ⏎`) cancels the job, then quits: the loop
/// carries out the commands before it reads `quit` (`event_loop::running`).
#[test]
fn quit_from_the_box_cancels_the_fetch_and_quits() {
    let state = fetching();
    let id = state.operating.expect("running");
    let (state, _) = press(&state, plain('q'));
    let (state, _) = press(&state, KeyPress::plain(KeyCode::Right));
    let (next, commands) = press(&state, KeyPress::plain(KeyCode::Enter));
    assert_eq!(next.quit, Some(Quit::Requested));
    assert_eq!(commands, [Command::CancelJob(id)]);
}

/// `y` takes `[ Quit ]` from wherever the cursor is (ADR-0044 §3): it
/// cancels the fetch and quits, and the bar offers `y quit`.
#[test]
fn y_quits_from_the_box_and_cancels_the_fetch() {
    let state = fetching();
    let id = state.operating.expect("running");
    let (state, _) = press(&state, plain('q'));
    let open = state.confirm.as_ref().expect("the box");
    assert_eq!(
        open.yes(),
        Some(graft_ui::graph_screen::ConfirmYes::Quit),
        "the bar draws `y quit`"
    );
    let (next, commands) = press(&state, plain('y'));
    assert_eq!(next.quit, Some(Quit::Requested));
    assert_eq!(commands, [Command::CancelJob(id)]);
}

#[test]
fn cancel_or_esc_keeps_the_app_and_the_fetch() {
    let state = fetching();
    let (boxed, _) = press(&state, plain('q'));
    for key in [KeyCode::Escape, KeyCode::Enter] {
        let (next, commands) = press(&boxed, KeyPress::plain(key));
        assert_eq!(next.quit, None, "{key:?}");
        assert!(next.confirm.is_none(), "{key:?} closed the box");
        assert!(commands.is_empty(), "{key:?}: {commands:?}");
        assert_eq!(next.operating, state.operating, "the fetch runs on");
    }
}

/// The fetch finished while the box was up: `[ Quit ]` just quits.
#[test]
fn quit_after_the_write_ended_cancels_nothing() {
    let state = fetching();
    let (mut state, _) = press(&state, plain('q'));
    state.operating = None;
    let (state, _) = press(&state, KeyPress::plain(KeyCode::Right));
    let (next, commands) = press(&state, KeyPress::plain(KeyCode::Enter));
    assert_eq!(next.quit, Some(Quit::Requested));
    assert!(commands.is_empty(), "{commands:?}");
}

/// `Ctrl+C` never asks (keymap §2.1).
#[test]
fn ctrl_c_during_a_fetch_quits_without_a_box() {
    let (next, _) = press(&fetching(), KeyPress::control(KeyCode::Char('c')));
    assert_eq!(next.quit, Some(Quit::Requested));
    assert!(next.confirm.is_none());
}

/// Git stopped, nothing running: `q` quits with no box (module doc).
#[test]
fn q_with_a_stopped_rebase_quits_without_a_box() {
    // The user's ruling (13b): the stop stays on disk and the next launch
    // offers continue or abort, so there is nothing to ask. The relaunch is
    // `it::conflict_interim::q_at_a_stop_quits_and_the_relaunch_offers_continue`.
    let state = AppState {
        in_progress: Some(graft_git::state::InProgressDetail {
            kind: graft_git::state::InProgress::Rebase,
            step: None,
            head_name: None,
            onto: None,
            theirs: Ok(None),
        }),
        ..AppState::default()
    };
    let (next, commands) = press(&state, plain('q'));
    assert_eq!(next.quit, Some(Quit::Requested));
    assert!(next.confirm.is_none(), "no box");
    assert!(commands.is_empty(), "nothing to stop: {commands:?}");
}

// ── review m8-13 Major 1: `q` from the focused banner ──
//
// A stop, a relaunch into one and an undo back into one all hand the
// keyboard to the banner. `q` pressed there must reach the same quit path
// as the graph's; it used to fall into the banner's silent catch-all. The
// undo way lives with its fixtures, in `undo`'s tests.

fn merge_detail() -> graft_git::state::InProgressDetail {
    graft_git::state::InProgressDetail {
        kind: graft_git::state::InProgress::Merge,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    }
}

/// The working-tree read `id` lands: a merge live, `paths` unmerged.
fn merge_read(id: JobId, paths: &[&str]) -> AppEvent {
    use graft_git::status::{StatusEntry, WorkingTreeStatus};
    let entries = paths
        .iter()
        .map(|path| StatusEntry {
            path: (*path).into(),
            original_path: None,
            staged: None,
            unstaged: None,
            untracked: false,
            unmerged: true,
            unmerged_how: None,
            submodule: None,
        })
        .collect();
    AppEvent::Produced(
        id,
        crate::update::Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
            rewrite: None,
            status: std::sync::Arc::new(WorkingTreeStatus { entries }),
            in_progress: Some(merge_detail()),
            mergetool: None,
        }))),
    )
}

/// `q` on the focused banner, a merge stopped and nothing running: it
/// quits, as from the graph, with no box (the user's ruling on 13b).
fn q_quits_from_the_banner(state: &AppState) {
    assert!(state.banner_focus, "the banner has the keyboard");
    assert!(
        crate::update::banner::banner_view(state).is_some(),
        "the banner is drawn"
    );
    let (next, commands) = press(state, plain('q'));
    assert_eq!(next.quit, Some(Quit::Requested), "`q` quits");
    assert!(next.confirm.is_none(), "no box");
    assert!(commands.is_empty(), "{commands:?}");
}

/// A Graft with its history up and nothing else read.
fn launched() -> AppState {
    AppState {
        history: Some(crate::view::tests::history_of(5, 0)),
        ..AppState::default()
    }
}

/// Way 1: a merge stops with nothing left unmerged, and the re-read it
/// asked for lands. The stop gave the banner the keyboard.
#[test]
fn q_on_the_banner_after_a_conflict_stop_quits_with_no_box() {
    let id = JobId::new(7);
    let state = AppState {
        operating: Some(id),
        ..launched()
    };
    let (state, commands) = crate::update::update(
        &state,
        AppEvent::Produced(
            id,
            crate::update::Product::Operated(Box::new(
                crate::jobs::operate::Operated::Conflicted {
                    rewrite: None,
                    description: "merge feature into main".to_owned(),
                    kind: graft_git::state::InProgress::Merge,
                    paths: Vec::new(),
                    stalled: None,
                    entry: graft_core::journal::EntryId::new(42, [4; 10]),
                },
            )),
        ),
    );
    let read = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadWorkingTree(id) => Some(*id),
            _ => None,
        })
        .expect("the stop re-reads the working tree");
    let (state, _) = crate::update::update(&state, merge_read(read, &[]));
    q_quits_from_the_banner(&state);
}

/// Way 2: Graft launched into a merge git already stopped (`relaunch`).
#[test]
fn q_on_the_banner_after_a_relaunch_quits_with_no_box() {
    let (state, _) = crate::update::update(&launched(), merge_read(JobId::new(90), &["gone.txt"]));
    q_quits_from_the_banner(&state);
}

/// A write running while the banner has the keyboard (a `c` launched from
/// it): the running box, since the write is what quitting cuts off.
#[test]
fn q_on_the_banner_while_a_write_runs_names_the_write() {
    let (state, _) = crate::update::update(&launched(), merge_read(JobId::new(90), &[]));
    let state = AppState {
        operating: Some(JobId::new(3)),
        ..state
    };
    assert!(state.banner_focus, "the banner has the keyboard");
    let (next, _) = press(&state, plain('q'));
    assert_eq!(next.quit, None);
    assert_eq!(
        box_line(&next),
        "a write is still running \u{b7} quit stops it"
    );
}

/// 13b: the banner's own `c`, a `--continue` (`Stoppable::No`), running
/// while the banner keeps the keyboard: `q` there opens the waiting box,
/// not the stopping one.
#[test]
fn q_on_the_banner_while_its_continue_runs_waits_for_it() {
    let (mut state, _) = crate::update::update(&launched(), merge_read(JobId::new(90), &[]));
    assert!(state.banner_focus, "the banner has the keyboard");
    let mut commands = Vec::new();
    let mut next = 60;
    assert!(crate::update::banner::resume(
        &mut state,
        &mut commands,
        graft_ops::apply::Resume::Continue,
        &mut || {
            next += 1;
            JobId::new(next)
        },
    ));
    let id = state.operating.expect("the continue runs");
    assert_eq!(
        commands.first(),
        Some(&Command::Unstoppable(id, description(&state))),
        "marked before it is queued: {commands:?}"
    );
    let (next, commands) = press(&state, plain('q'));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next.quit, None);
    assert_eq!(
        box_line(&next),
        "continue merge is still running \u{b7} quit waits for it to finish \
         if it stops at a conflict or fails, you stay to see it"
    );
}

// ── 13b: `q` while a write that cannot stop runs ──

use crate::jobs::operate::Operated;
use crate::update::state::Status;

/// A merge launched (`Stoppable::No`, as `m`'s preview launches it) and
/// still running, over a Graft with its history up.
fn merging() -> (AppState, JobId) {
    let mut state = launched();
    let mut commands = Vec::new();
    let mut next = 40;
    let oid = |byte| graft_core::oid::Oid::from_bytes(&[byte; 20]).expect("20 bytes");
    crate::update::operate::launch_unstoppable(
        &mut state,
        &mut commands,
        Operation::Merge(graft_ops::operation::MergeParams {
            source: graft_ops::operation::Source::Branch("feature".to_owned()),
            into: graft_ops::operation::Tip {
                name: "refs/heads/main".to_owned(),
                oid: oid(2),
            },
            source_tip: oid(3),
            ff: graft_ops::operation::FastForward::Default,
        }),
        crate::update::preview::HISTORY_CANNOT_STOP,
        Some("m"),
        &mut || {
            next += 1;
            JobId::new(next)
        },
    )
    .expect("nothing else runs");
    let id = state.operating.expect("the merge runs");
    assert_eq!(
        commands.first(),
        Some(&Command::Unstoppable(id, description(&state))),
        "the pool is told before the job is queued: {commands:?}"
    );
    (state, id)
}

fn description(state: &AppState) -> String {
    state
        .remote
        .running
        .as_ref()
        .expect("the write is recorded")
        .description
        .clone()
}

fn waiting_line(state: &AppState) -> Status {
    Status::Waiting(format!(
        "{} \u{b7} quitting when it finishes \u{b7} esc stays",
        description(state)
    ))
}

/// The merge running, `q`, then `y`: Graft waiting to quit.
pub(crate) fn waiting() -> (AppState, JobId) {
    let (state, id) = merging();
    let (state, _) = press(&state, plain('q'));
    let (state, commands) = press(&state, plain('y'));
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.quit_when_done.is_some(), "the wait is set");
    (state, id)
}

fn not_cancelled(commands: &[Command]) {
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::CancelJob(_))),
        "nothing asks the write to stop: {commands:?}"
    );
}

fn answer(state: &AppState, id: JobId, outcome: Operated) -> (AppState, Vec<Command>) {
    crate::update::update(
        state,
        AppEvent::Produced(id, crate::update::Product::Operated(Box::new(outcome))),
    )
}

fn applied() -> graft_ops::apply::Applied {
    use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind};
    graft_ops::apply::Applied {
        came_back: Vec::new(),
        stopped_at: None,
        entry: JournalEntry {
            id: EntryId::new(1, [0; 10]),
            at_unix_ms: 1,
            kind: OperationKind::Merge,
            description: "merge feature into main".to_owned(),
            status: EntryStatus::Committed,
            touches: graft_core::journal::Touches::default(),
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            head_after: None,
            refs_before: std::collections::BTreeMap::new(),
            refs_after: std::collections::BTreeMap::new(),
            index_before: None,
            index_after: None,
            worktree_before: None,
            worktree_after: None,
            worktree_raw_before: None,
            backup_ref: None,
            pins: Vec::new(),
            commands: Vec::new(),
            reverses: None,
            merge_before: None,
            merge_after: None,
            config_before: std::collections::BTreeMap::new(),
            config_after: std::collections::BTreeMap::new(),
            put_back: None,
        },
        refresh: graft_ops::apply::Refresh::Everything,
        step_backup: None,
        replaced: graft_ops::undo::Replaced::default(),
    }
}

/// Acceptance 1: the box, its two lines exactly, Cancel first, `y quit`.
#[test]
fn q_during_a_merge_says_quit_waits_for_it() {
    let (state, _) = merging();
    let (next, commands) = press(&state, plain('q'));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next.quit, None);
    let open = next.confirm.as_ref().expect("the box");
    assert_eq!(open.view.title.as_str(), "Quit?");
    let lines: Vec<&str> = open
        .view
        .message
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(
        lines,
        [
            format!(
                "{} is still running \u{b7} quit waits for it to finish",
                description(&state)
            )
            .as_str(),
            "if it stops at a conflict or fails, you stay to see it",
        ]
    );
    let labels: Vec<&str> = open
        .view
        .choices
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(labels, ["Cancel", "Quit"]);
    assert_eq!(open.view.cursor, 0, "the cursor rests on Cancel");
    assert_eq!(
        open.yes(),
        Some(graft_ui::graph_screen::ConfirmYes::Quit),
        "the bar leads `y quit`"
    );
}

/// Acceptance 2: `y` closes the box, keeps the app and the merge, and the
/// row says what happens next.
#[test]
fn y_over_a_merge_waits_and_cancels_nothing() {
    let (state, id) = merging();
    let (state, _) = press(&state, plain('q'));
    let (next, commands) = press(&state, plain('y'));
    not_cancelled(&commands);
    assert!(next.confirm.is_none(), "the box closed");
    assert_eq!(next.quit, None, "Graft runs on");
    assert_eq!(next.operating, Some(id), "the merge runs on");
    assert_eq!(next.shown_status(), &waiting_line(&next));
}

/// Acceptance 3: the merge ends done → Graft quits, once the answer is in
/// (its words are on `status`, under the quit).
#[test]
fn the_merge_ending_done_quits() {
    let (state, id) = waiting();
    let (next, commands) = answer(&state, id, Operated::Done(applied()));
    not_cancelled(&commands);
    assert_eq!(next.quit, Some(Quit::Requested));
    assert!(next.quit_when_done.is_none());
    assert!(
        matches!(next.status, Status::Done { .. }),
        "the answer was applied first: {:?}",
        next.status
    );
}

/// Acceptance 4: it stops at a conflict → no quit, the resolver on the
/// first conflicted file, the wait gone: a later write ending done does not
/// quit.
#[test]
fn the_merge_stopping_at_a_conflict_stays_in_the_resolver() {
    let (state, id) = waiting();
    let (next, commands) = answer(
        &state,
        id,
        Operated::Conflicted {
            rewrite: None,
            description: description(&state),
            kind: graft_git::state::InProgress::Merge,
            paths: vec!["a.txt".into(), "b.txt".into()],
            stalled: None,
            entry: graft_core::journal::EntryId::new(42, [4; 10]),
        },
    );
    assert_eq!(next.quit, None, "he stays to see the stop");
    assert!(next.quit_when_done.is_none(), "the wait is gone");
    assert!(
        commands.iter().any(|command| matches!(
            command,
            Command::OpenConflict(_, path) if path == std::path::Path::new("a.txt")
        )),
        "the resolver opens on the first conflicted file: {commands:?}"
    );
    // A later write that ends done is not the one waited for.
    let (later, later_id) = merging_over(next);
    let (after, _) = answer(&later, later_id, Operated::Done(applied()));
    assert_eq!(after.quit, None);
}

/// `state` with a second merge launched over it.
fn merging_over(mut state: AppState) -> (AppState, JobId) {
    state.resolver = None;
    state.operating = None;
    let mut commands = Vec::new();
    crate::update::operate::launch_unstoppable(
        &mut state,
        &mut commands,
        Operation::Fetch(FetchParams {
            remote: None,
            prune: false,
        }),
        crate::update::preview::HISTORY_CANNOT_STOP,
        None,
        &mut || JobId::new(77),
    )
    .expect("nothing else runs");
    (state, JobId::new(77))
}

/// Acceptance 5: it fails → no quit, the failure's own words on the row.
#[test]
fn the_merge_failing_stays_with_its_words() {
    let (state, id) = waiting();
    let (next, _) = answer(
        &state,
        id,
        Operated::Failed {
            error: "the merge broke".into(),
            refresh: None,
            remote: None,
            refused: None,
            edits: None,
        },
    );
    assert_eq!(next.quit, None);
    assert!(next.quit_when_done.is_none());
    assert!(
        next.shown_status().is_failure(),
        "{:?}",
        next.shown_status()
    );
    assert!(
        format!("{:?}", next.shown_status()).contains("the merge broke"),
        "{:?}",
        next.shown_status()
    );
}

/// Acceptance 6: it ends incomplete → no quit, its warning kept.
#[test]
fn the_merge_ending_incomplete_stays_with_its_warning() {
    let (state, id) = waiting();
    let (next, _) = answer(
        &state,
        id,
        Operated::Incomplete {
            applied: applied(),
            error: "could not read the refs after".to_owned(),
        },
    );
    assert_eq!(next.quit, None);
    assert!(next.quit_when_done.is_none());
    assert!(
        format!("{:?}", next.shown_status()).contains("could not read the refs after"),
        "{:?}",
        next.shown_status()
    );
}

/// Acceptance 7: `Esc` drops the wait, the merge runs on, and its `Done`
/// then does not quit.
#[test]
fn esc_while_waiting_drops_the_wait() {
    let (state, id) = waiting();
    let (next, commands) = press(&state, KeyPress::plain(KeyCode::Escape));
    not_cancelled(&commands);
    assert!(next.quit_when_done.is_none(), "the wait is dropped");
    assert_eq!(next.operating, Some(id), "the merge runs on");
    assert_ne!(
        next.shown_status(),
        &waiting_line(&state),
        "the line went with it"
    );
    let (after, _) = answer(&next, id, Operated::Done(applied()));
    assert_eq!(after.quit, None, "no quit once the wait is dropped");
}

/// The `Esc` order (§2.2): a failure held while a box was up is dismissed
/// first, and that `Esc` leaves the wait standing.
#[test]
fn esc_dismisses_a_held_failure_before_the_wait() {
    let (mut state, _) = waiting();
    state.unseen = Some(Status::Failed("a read failed".to_owned()));
    let (next, _) = press(&state, KeyPress::plain(KeyCode::Escape));
    assert!(next.unseen.is_none(), "the held failure went first");
    assert!(next.quit_when_done.is_some(), "the wait stands");
    let (next, _) = press(&next, KeyPress::plain(KeyCode::Escape));
    assert!(next.quit_when_done.is_none(), "the next `Esc` drops it");
}

/// `q` and `Ctrl+C` while waiting change nothing: no second box.
#[test]
fn q_while_waiting_changes_nothing() {
    let (state, _) = waiting();
    for key in [plain('q'), KeyPress::control(KeyCode::Char('c'))] {
        let (next, commands) = press(&state, key);
        assert!(commands.is_empty(), "{key:?}: {commands:?}");
        assert!(next.confirm.is_none(), "{key:?}: no second box");
        assert_eq!(next.quit, None, "{key:?}");
        assert_eq!(next.shown_status(), &waiting_line(&state), "{key:?}");
    }
}

/// Acceptance 8: the write's progress lines do not replace the waiting
/// words — the operate job's description and a remote-style progress line
/// both land on `status`, under the wait.
#[test]
fn progress_while_waiting_keeps_the_waiting_words() {
    let (state, id) = waiting();
    let line = waiting_line(&state);
    let (next, _) = crate::update::update(
        &state,
        AppEvent::Progress(
            id,
            crate::update::Progress::Operating {
                description: "merge step 2 of 3".to_owned(),
            },
        ),
    );
    assert_eq!(next.shown_status(), &line);
    let (next, _) = crate::update::update(
        &next,
        AppEvent::Progress(
            id,
            crate::update::Progress::RemoteProgress {
                brief: "receiving objects 45%".to_owned(),
            },
        ),
    );
    assert_eq!(next.shown_status(), &line);
}

/// The conductor's ruling on 13b: `Ctrl+C` over a write that cannot stop is
/// `q`'s box — never a quit that would SIGTERM git half-way through.
#[test]
fn ctrl_c_during_a_merge_boxes_like_q_and_signals_nothing() {
    let (state, _) = merging();
    let (by_c, commands) = press(&state, KeyPress::control(KeyCode::Char('c')));
    not_cancelled(&commands);
    assert_eq!(by_c.quit, None, "no quit while git is half-way");
    let (by_q, _) = press(&state, plain('q'));
    assert_eq!(by_c.confirm, by_q.confirm, "the same box as `q`");
    let (next, commands) = press(&by_c, plain('y'));
    not_cancelled(&commands);
    assert!(next.quit_when_done.is_some(), "the same wait");
}

/// `Ctrl+C` with the waiting box itself up (it passes through the box,
/// `confirm_key`): the box again, still no quit.
#[test]
fn ctrl_c_over_the_waiting_box_does_not_quit() {
    let (state, _) = merging();
    let (boxed, _) = press(&state, plain('q'));
    let (next, commands) = press(&boxed, KeyPress::control(KeyCode::Char('c')));
    not_cancelled(&commands);
    assert_eq!(next.quit, None);
    assert_eq!(next.confirm, boxed.confirm);
}

/// The merge answered while the box was still up, done: `y` just quits.
#[test]
fn y_after_the_merge_ended_done_quits() {
    let (state, id) = merging();
    let (state, _) = press(&state, plain('q'));
    let (state, _) = answer(&state, id, Operated::Done(applied()));
    assert!(state.confirm.is_some(), "the box is still up");
    let (next, _) = press(&state, plain('y'));
    assert_eq!(next.quit, Some(Quit::Requested));
}

/// A wait whose write vanished without an answer (a panicked job) is no
/// wait: not drawn, and a later `q` asks afresh.
#[test]
fn a_wait_outliving_its_write_is_inert() {
    let (mut state, _) = waiting();
    state.operating = None;
    assert_eq!(state.shown_status(), &state.status);
    let (next, _) = press(&state, plain('q'));
    assert_eq!(next.quit, Some(Quit::Requested));
}

// ── 13b: every answer path of a `Stoppable::No` write reaches `write_ended` ──

/// The banner's `c` (a `--continue`) running, `q y` from the banner.
fn waiting_on_a_continue() -> (AppState, JobId) {
    let (mut state, _) = crate::update::update(&launched(), merge_read(JobId::new(90), &[]));
    let mut commands = Vec::new();
    assert!(crate::update::banner::resume(
        &mut state,
        &mut commands,
        graft_ops::apply::Resume::Continue,
        &mut || JobId::new(61),
    ));
    let (state, _) = press(&state, plain('q'));
    let (state, commands) = press(&state, plain('y'));
    not_cancelled(&commands);
    assert!(state.quit_when_done.is_some(), "the wait is set");
    (state, JobId::new(61))
}

/// `banner::resumed`: a `--continue` that ends done quits.
#[test]
fn a_continue_ending_done_quits() {
    let (state, id) = waiting_on_a_continue();
    let (next, _) = crate::update::update(
        &state,
        AppEvent::Produced(
            id,
            crate::update::Product::Resumed(Box::new(Operated::Done(applied()))),
        ),
    );
    assert_eq!(next.quit, Some(Quit::Requested));
}

/// `banner::resumed`: a `--continue` that stops again stays.
#[test]
fn a_continue_stopping_again_stays() {
    let (state, id) = waiting_on_a_continue();
    let (next, _) = crate::update::update(
        &state,
        AppEvent::Produced(
            id,
            crate::update::Product::Resumed(Box::new(Operated::Conflicted {
                rewrite: None,
                description: "merge feature into main".to_owned(),
                kind: graft_git::state::InProgress::Merge,
                paths: vec!["a.txt".into()],
                stalled: None,
                entry: graft_core::journal::EntryId::new(42, [4; 10]),
            })),
        ),
    );
    assert_eq!(next.quit, None);
    assert!(next.quit_when_done.is_none());
}

/// The banner keeps the keyboard during its `c`: its `Esc` drops the wait
/// too, and does only that.
#[test]
fn esc_on_the_banner_drops_the_wait() {
    let (state, id) = waiting_on_a_continue();
    assert!(state.banner_focus, "the banner has the keyboard");
    let (next, commands) = press(&state, KeyPress::plain(KeyCode::Escape));
    not_cancelled(&commands);
    assert!(next.quit_when_done.is_none());
    assert!(next.banner_focus, "that `Esc` only dropped the wait");
    assert_eq!(next.operating, Some(id));
}

/// The journal's time machine (`undo::reverted_to`), launched
/// `Stoppable::No`, waited on.
fn waiting_on_a_revert() -> (AppState, JobId) {
    let id = JobId::new(81);
    let mut state = AppState {
        operating: Some(id),
        ..launched()
    };
    let unstoppable = crate::update::remote::launched(
        &mut state,
        id,
        "revert to before \u{201c}stage a.txt\u{201d}".to_owned(),
        None,
        crate::update::remote::Stoppable::No(crate::update::preview::HISTORY_CANNOT_STOP),
        None,
    );
    assert_eq!(
        unstoppable,
        Some(Command::Unstoppable(
            id,
            "revert to before \u{201c}stage a.txt\u{201d}".to_owned()
        ))
    );
    let (state, _) = press(&state, plain('q'));
    let (state, commands) = press(&state, plain('y'));
    not_cancelled(&commands);
    assert!(state.quit_when_done.is_some(), "the wait is set");
    (state, id)
}

fn reverted(state: &AppState, id: JobId, outcome: crate::jobs::undo::RevertedTo) -> AppState {
    crate::update::update(
        state,
        AppEvent::Produced(id, crate::update::Product::RevertedTo(Box::new(outcome))),
    )
    .0
}

/// `undo::reverted_to`: every step ran → quit.
#[test]
fn a_revert_ending_done_quits() {
    let (state, id) = waiting_on_a_revert();
    let next = reverted(
        &state,
        id,
        crate::jobs::undo::RevertedTo::Done {
            description: "stage a.txt".to_owned(),
            total: 1,
            steps: vec![applied()],
            replaced: graft_ops::undo::Replaced::default(),
            presses: 0,
        },
    );
    assert_eq!(next.quit, Some(Quit::Requested));
}

/// `undo::reverted_to`: stopped part-way → stays, its words on the row.
#[test]
fn a_revert_failing_part_way_stays() {
    let (state, id) = waiting_on_a_revert();
    let next = reverted(
        &state,
        id,
        crate::jobs::undo::RevertedTo::Failed {
            description: "stage a.txt".to_owned(),
            done: 1,
            total: 2,
            error: "a step broke".to_owned(),
        },
    );
    assert_eq!(next.quit, None);
    assert!(next.quit_when_done.is_none());
    assert!(
        next.shown_status().is_failure(),
        "{:?}",
        next.shown_status()
    );
}

/// 13d's exit lines, exact: the one line a stopped write leaves in the
/// shell says where git was left (paused, landed, stopped before it could
/// pause, or not known). `<n>` is whole seconds; the undo key is the one
/// the keymap spells, here a rebound `z`.
#[test]
fn exit_line_says_where_git_left_it() {
    use core::time::Duration;

    use graft_git::state::InProgress;
    use graft_ops::apply::StoppedAt;

    use crate::update::quit::exit_line;

    let line = |at| {
        exit_line(
            "graft",
            "merge feature into main",
            Duration::from_millis(12_900),
            at,
            "z",
        )
    };
    let stopped = "graft: stopped merge feature into main after 12 s";
    assert_eq!(
        line(Some(StoppedAt::Paused(InProgress::Merge))),
        format!("{stopped} \u{b7} the merge is paused \u{b7} graft here offers continue or abort")
    );
    assert_eq!(
        line(Some(StoppedAt::Paused(InProgress::CherryPick))),
        format!(
            "{stopped} \u{b7} the cherry-pick is paused \u{b7} graft here offers continue or abort"
        )
    );
    assert_eq!(
        line(Some(StoppedAt::Landed)),
        "graft: merge feature into main finished \u{b7} a git hook after it was stopped after 12 s"
    );
    assert_eq!(
        line(Some(StoppedAt::BeforePause { changed: true })),
        format!("{stopped}, before git could pause it \u{b7} z in graft puts your files back")
    );
    assert_eq!(
        line(Some(StoppedAt::Stopped { changed: true })),
        format!("{stopped} \u{b7} z in graft puts your files back")
    );
    for nothing in [
        StoppedAt::BeforePause { changed: false },
        StoppedAt::Stopped { changed: false },
    ] {
        assert_eq!(
            line(Some(nothing)),
            format!("{stopped} \u{b7} nothing changed"),
            "{nothing:?}"
        );
    }
    assert_eq!(
        line(None),
        format!("{stopped} \u{b7} graft here shows where git left it")
    );
    let hostile = exit_line("graft", "merge \u{1b}[2Jx", Duration::ZERO, None, "u");
    assert!(
        !hostile.contains('\u{1b}'),
        "a description from the repository is sanitised: {hostile:?}"
    );
}

mod modals;
mod stuck;
