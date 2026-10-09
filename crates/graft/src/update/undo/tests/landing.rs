//! `update::undo` tests: the landing after an undo that puts an operation back. Moved out of the inline module (m8/21b-5,
//! move-only).

use super::*;

const FINISH_UNDONE: &str = "undo: finish merge of feature";

/// `u` on a finish, answered `Done` with nothing live yet: the read the
/// answer asked for, and the state waiting on it. A working tree has
/// been read before, as it has in any running app: the first read is
/// the launch's landing (`relaunch`), not the undo's.
pub(super) fn finish_undone(state: AppState) -> (AppState, JobId) {
    let mut state = AppState {
        operating: Some(JobId::new(2)),
        working_tree: Some(Arc::new(graft_git::status::WorkingTreeStatus {
            entries: Vec::new(),
        })),
        ..state
    };
    let mut next = {
        let mut raw = 100;
        move || {
            raw += 1;
            JobId::new(raw)
        }
    };
    let commands = undone(
        &mut state,
        JobId::new(2),
        Undone::Done(applied(FINISH_UNDONE, Refresh::WorkingTree)),
        &mut next,
    );
    let read = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadWorkingTree(id) => Some(*id),
            _ => None,
        })
        .expect("the answer asks for the working tree");
    (state, read)
}

/// Read `id` lands, reporting `kind` live (or nothing).
pub(super) fn read_lands(
    state: &AppState,
    id: JobId,
    kind: Option<graft_git::state::InProgress>,
) -> AppState {
    let read = crate::working_tree::WorkingTree {
        rewrite: None,
        status: Arc::new(graft_git::status::WorkingTreeStatus {
            entries: Vec::new(),
        }),
        in_progress: kind.map(|kind| graft_git::state::InProgressDetail {
            kind,
            step: None,
            head_name: None,
            onto: None,
            theirs: Ok(None),
        }),
        mergetool: None,
    };
    crate::update::update(
        state,
        crate::update::AppEvent::Produced(
            id,
            crate::update::Product::WorkingTree(Ok(Box::new(read))),
        ),
    )
    .0
}

pub(super) fn done_line_of(status: &Status) -> String {
    match status {
        Status::Done { what, also, keys } => {
            crate::view::status::done_line(what, also.as_deref(), keys.as_deref())
        }
        other => panic!("not a done line: {other:?}"),
    }
}

#[test]
fn an_undo_that_puts_a_merge_back_lands_on_the_banner() {
    let (state, read) = finish_undone(AppState::default());
    assert!(state.in_progress.is_none(), "nothing live when it answers");
    let state = read_lands(&state, read, Some(graft_git::state::InProgress::Merge));
    assert!(state.banner_focus, "the banner has the keyboard");
    assert!(!state.undo_landing_owed, "the landing is spent");
    assert_eq!(
        state.status,
        Status::done_with_keys(FINISH_UNDONE, "c finishes the merge")
    );
    assert_eq!(
        done_line_of(&state.status),
        "undo: finish merge of feature ✓ · c finishes the merge"
    );
}

/// Review m8-13 Major 1, the undo way: the banner took the keyboard
/// after an undo put a merge back, and `q` there reaches the quit path
/// rather than the banner's silent catch-all. Git stopped and nothing
/// running, it quits with no box (the user's ruling on 13b). The stop
/// and relaunch ways are in `quit`'s tests.
#[test]
fn q_on_the_banner_after_an_undo_puts_a_merge_back_quits_with_no_box() {
    use graft_core::event::{Event, KeyCode, KeyPress};
    let (state, read) = finish_undone(AppState::default());
    let state = read_lands(&state, read, Some(graft_git::state::InProgress::Merge));
    assert!(state.banner_focus, "the banner has the keyboard");
    let (next, commands) = crate::update::update(
        &state,
        crate::update::AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char('q')))),
    );
    assert_eq!(next.quit, Some(crate::update::Quit::Requested), "quits");
    assert!(next.confirm.is_none(), "no box");
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn an_undo_with_nothing_put_back_leaves_focus_alone() {
    let (state, read) = finish_undone(AppState::default());
    assert!(state.undo_landing_owed, "owed while the read is out");
    let focus = (state.graph_has_focus(), state.banner_focus);
    let state = read_lands(&state, read, None);
    assert_eq!((state.graph_has_focus(), state.banner_focus), focus);
    assert_eq!(state.status, Status::done(FINISH_UNDONE));
    assert!(!state.undo_landing_owed, "spent all the same");
}

#[test]
fn a_stale_read_does_not_spend_the_landing() {
    // A read asked before the undo's own refresh: older, so it read the
    // repository before the undo wrote.
    let older = JobId::new(50);
    let (state, read) = finish_undone(AppState {
        status_read: Some(older),
        ..AppState::default()
    });
    assert_ne!(read, older);
    let state = read_lands(&state, older, None);
    assert!(!state.banner_focus, "no focus on a stale read");
    assert!(state.undo_landing_owed, "the stale read did not spend it");
    let state = read_lands(&state, read, Some(graft_git::state::InProgress::Merge));
    assert!(state.banner_focus, "the undo's own read lands it");
    assert!(!state.undo_landing_owed);
}

/// The Map, the journal browser and the stash list replace the graph
/// screen and its banner, and `u` works under the journal browser, where
/// the key that ran the undo was pressed; any of the three can also be
/// opened while the undo runs.
#[test]
fn under_a_modal_the_landing_is_dropped() {
    let (mut state, read) = finish_undone(AppState::default());
    state.journal.shown = true;
    assert_eq!(
        state.graph_overlay(),
        Some(crate::update::GraphOverlay::Journal)
    );
    let state = read_lands(&state, read, Some(graft_git::state::InProgress::Merge));
    assert!(!state.banner_focus, "no focus under the overlay");
    assert_eq!(state.status, Status::done(FINISH_UNDONE), "no keys added");
    assert!(!state.undo_landing_owed, "dropped, not kept for later");
}

#[test]
fn a_rebase_put_back_says_continues() {
    let (state, read) = finish_undone(AppState::default());
    let state = read_lands(&state, read, Some(graft_git::state::InProgress::Rebase));
    assert!(state.banner_focus);
    let Status::Done { keys, .. } = &state.status else {
        panic!("{:?}", state.status);
    };
    assert_eq!(keys.as_deref(), Some("c continues the rebase"));
    let Status::Done {
        keys: every_file_done_keys,
        ..
    } = crate::update::resolver::every_file_done(&state)
    else {
        panic!("every_file_done is a done line");
    };
    assert_eq!(
        *keys, every_file_done_keys,
        "the same words as every file done"
    );
}
