//! Landing on an operation git is part-way through at launch
//! (m6-conflicts/12, scope item 4), driven through `update` the way the job
//! pool answers: the working-tree read and the journal read, in either
//! order.

use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;
use graft_git::state::{InProgress, InProgressDetail};
use graft_git::status::{StatusEntry, WorkingTreeStatus};
use graft_ui::detail::WorkingTreeSection;

use super::LiveOrigin;
use crate::update::detail::DetailSpec;
use crate::update::{AppEvent, AppState, Product, update};

fn merge() -> InProgressDetail {
    InProgressDetail {
        kind: InProgress::Merge,
        step: None,
        head_name: None,
        onto: None,
        theirs: None,
    }
}

fn unmerged(path: &str) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged: true,
        unmerged_how: None,
        submodule: false,
    }
}

/// A freshly launched Graft: the history is up, neither launch read is in.
fn launched() -> AppState {
    AppState {
        history: Some(crate::view::tests::history_of(5, 0)),
        ..AppState::default()
    }
}

fn working_tree_read(paths: &[&str]) -> AppEvent {
    AppEvent::Produced(
        JobId::new(90),
        Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
            status: Arc::new(WorkingTreeStatus {
                entries: paths.iter().map(|path| unmerged(path)).collect(),
            }),
            in_progress: Some(merge()),
            mergetool: None,
        }))),
    )
}

/// The journal read, naming one live entry — or none, for a journal that
/// has never heard of the operation.
fn journal_read(state: &mut AppState, live: bool) -> AppEvent {
    let id = JobId::new(91);
    state.journal.reading = Some(id);
    AppEvent::Produced(
        id,
        Product::Journal(Ok(Arc::new(crate::jobs::undo::JournalRead {
            entries: Vec::new(),
            unreadable: Vec::new(),
            live: if live {
                vec![graft_core::journal::EntryId::new(1, [0; 10])]
            } else {
                Vec::new()
            },
        }))),
    )
}

fn press(state: &AppState, code: KeyCode) -> AppState {
    update(
        state,
        AppEvent::Core(Event::Key(KeyPress {
            code,
            modifiers: Modifiers::NONE,
        })),
    )
    .0
}

/// The landing, as the user sees it: banner focused, the changes pane
/// open on the working tree with its cursor owed to `path` under
/// CONFLICTS.
fn landed_on(state: &AppState, path: &str) {
    assert!(state.banner_focus, "the banner has the keyboard");
    assert!(state.detail.shown, "the changes pane is open");
    assert_eq!(state.detail.spec, Some(DetailSpec::WorkingTree));
    let kept = state
        .detail
        .kept_cursor
        .as_ref()
        .expect("the cursor is owed a file");
    assert_eq!(kept.section, Some(WorkingTreeSection::Unmerged));
    assert_eq!(kept.path, path.as_bytes());
}

#[test]
fn a_merge_begun_in_the_shell_lands_on_the_banner() {
    let (state, _) = update(&launched(), working_tree_read(&["gone.txt"]));
    landed_on(&state, "gone.txt");
    assert_eq!(
        state.stopped.as_ref().map(|stop| stop.origin),
        Some(LiveOrigin::OutsideGraft)
    );
    let reason = crate::view::rows::stopped_line(&state).expect("a stop is standing");
    assert!(
        reason.as_str().contains("begun outside Graft"),
        "{reason:?}"
    );
    // A journal that never heard of it changes nothing.
    let mut state = state;
    let event = journal_read(&mut state, false);
    let (state, _) = update(&state, event);
    assert_eq!(
        state.stopped.as_ref().map(|stop| stop.origin),
        Some(LiveOrigin::OutsideGraft)
    );
    // One `⏎` opens the resolver.
    let state = press(&state, KeyCode::Enter);
    assert_eq!(
        state
            .resolver
            .as_ref()
            .map(|resolver| resolver.path.clone()),
        Some("gone.txt".into())
    );
}

#[test]
fn a_previous_runs_merge_still_lands_the_same_way() {
    // The journal first (scenario 26's order), then git's word.
    let mut state = launched();
    let event = journal_read(&mut state, true);
    let (state, _) = update(&state, event);
    assert!(
        !state.banner_focus,
        "the control: the journal alone lands nothing"
    );
    let (state, _) = update(&state, working_tree_read(&["a.rs", "b.rs"]));
    landed_on(&state, "a.rs");
    assert_eq!(
        crate::view::rows::stopped_line(&state).map(|line| line.as_str().to_owned()),
        Some("merge still in progress from a previous run · 2 files conflicted".to_owned())
    );

    // Git's word first: it lands as begun outside Graft, and the journal's
    // live entry, arriving second, says whose it really is.
    let (state, _) = update(&launched(), working_tree_read(&["a.rs"]));
    landed_on(&state, "a.rs");
    let mut state = state;
    let event = journal_read(&mut state, true);
    let (state, _) = update(&state, event);
    assert_eq!(
        crate::view::rows::stopped_line(&state).map(|line| line.as_str().to_owned()),
        Some("merge still in progress from a previous run · 1 file conflicted".to_owned())
    );
}

#[test]
fn a_modal_defers_the_landing() {
    let state = AppState {
        search_prompt: Some(String::new()),
        ..launched()
    };
    assert!(state.modal_owns_keys(), "the control: a modal is up");
    let (state, _) = update(&state, working_tree_read(&["gone.txt"]));
    assert!(state.stopped.is_some(), "the stop is recorded");
    assert!(!state.banner_focus, "the keyboard stays with the modal");
    assert!(state.live_landing_owed);

    let state = press(&state, KeyCode::Escape);
    assert!(!state.modal_owns_keys(), "the control: the modal closed");
    let (state, _) = update(&state, working_tree_read(&["gone.txt"]));
    landed_on(&state, "gone.txt");
    assert!(!state.live_landing_owed, "the debt is paid");
}
