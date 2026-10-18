use mergewright_core::job::JobId;
use std::sync::Arc;

use mergewright_git::status::{Change, StatusEntry, WorkingTreeStatus};

use super::*;
use crate::history::{History, Motion};
use crate::refs::RefBadges;
use crate::update::detail::DetailSpec;
use crate::update::state::{AppState, Harness, Size};

fn counts() -> Arc<WorkingTreeStatus> {
    Arc::new(WorkingTreeStatus {
        entries: vec![
            StatusEntry {
                path: "staged.txt".into(),
                original_path: None,
                staged: Some(Change::Modified),
                unstaged: None,
                untracked: false,
                unmerged: false,
                unmerged_how: None,
                submodule: None,
            },
            StatusEntry {
                path: "unstaged.txt".into(),
                original_path: None,
                staged: None,
                unstaged: Some(Change::Modified),
                untracked: false,
                unmerged: false,
                unmerged_how: None,
                submodule: None,
            },
            StatusEntry {
                path: "untracked.txt".into(),
                original_path: None,
                staged: None,
                unstaged: None,
                untracked: true,
                unmerged: false,
                unmerged_how: None,
                submodule: None,
            },
        ],
    })
}

fn clean() -> Arc<WorkingTreeStatus> {
    Arc::new(WorkingTreeStatus::default())
}

/// A state with refs answered, `HEAD` at row 0, and a dirty tree — the
/// row is shown, cursor at the top.
fn shown_state() -> AppState {
    let mut history = History::started(crate::config::MergedBranches::Folded);
    history.rows = 3;
    history.top = 0;
    history.cursor = 0;
    let mut state = AppState {
        size: Size {
            columns: 80,
            rows: 30,
        },
        harness: Harness::default(),
        history: Some(history),
        working_tree: Some(counts()),
        ..Default::default()
    };
    // `HEAD` unborn reads as topmost with no refs to compare against, so
    // an empty `RefBadges` (no head, no branches) is enough to make the
    // row attach without a real repository.
    state.refs = Some(std::sync::Arc::new(RefBadges::default()));
    state
}

#[test]
fn up_one_from_the_top_row_selects_the_virtual_row_and_asks_history_for_nothing() {
    let mut state = shown_state();
    let motion = move_from_graph(&mut state, Motion::Up(1));
    assert!(state.on_working_tree);
    assert_eq!(motion, None);
}

#[test]
fn up_one_from_the_top_row_scrolls_as_in_m1_when_the_row_is_hidden() {
    let mut state = shown_state();
    state.working_tree = Some(clean());
    let motion = move_from_graph(&mut state, Motion::Up(1));
    assert!(!state.on_working_tree);
    assert_eq!(motion, Some(Motion::Up(1)));
}

#[test]
fn down_one_from_the_row_selects_heads_row_and_asks_history_for_nothing() {
    let mut state = shown_state();
    state.on_working_tree = true;
    let motion = move_from_graph(&mut state, Motion::Down(1));
    assert!(!state.on_working_tree);
    assert_eq!(motion, None);
}

#[test]
fn down_three_from_the_row_leaves_down_two_for_history() {
    let mut state = shown_state();
    state.on_working_tree = true;
    let motion = move_from_graph(&mut state, Motion::Down(3));
    assert!(!state.on_working_tree);
    assert_eq!(motion, Some(Motion::Down(2)));
}

#[test]
fn page_down_from_the_row_clears_the_flag_and_applies() {
    let mut state = shown_state();
    state.on_working_tree = true;
    let motion = move_from_graph(&mut state, Motion::PageDown);
    assert!(!state.on_working_tree);
    assert_eq!(motion, Some(Motion::PageDown));
}

#[test]
fn bottom_from_the_row_clears_the_flag_and_applies() {
    let mut state = shown_state();
    state.on_working_tree = true;
    let motion = move_from_graph(&mut state, Motion::Bottom);
    assert!(!state.on_working_tree);
    assert_eq!(motion, Some(Motion::Bottom));
}

#[test]
fn up_or_pageup_or_top_from_the_row_does_nothing_at_the_very_top() {
    let mut state = shown_state();
    state.on_working_tree = true;
    assert_eq!(move_from_graph(&mut state, Motion::Up(1)), None);
    assert!(state.on_working_tree);
    assert_eq!(move_from_graph(&mut state, Motion::PageUp), None);
    assert!(state.on_working_tree);
    assert_eq!(move_from_graph(&mut state, Motion::Top), None);
    assert!(state.on_working_tree);
}

#[test]
fn up_from_the_row_scrolls_up_one_when_the_viewport_is_not_at_the_very_top() {
    let mut state = shown_state();
    state.history.as_mut().unwrap().top = 5;
    state.on_working_tree = true;
    let motion = move_from_graph(&mut state, Motion::Up(1));
    assert!(!state.on_working_tree);
    assert_eq!(motion, Some(Motion::Up(1)));
}

#[test]
fn reconcile_clears_when_the_tree_goes_clean() {
    let mut state = shown_state();
    state.on_working_tree = true;
    state.working_tree = Some(clean());
    assert!(reconcile(&mut state, 0));
    assert!(!state.on_working_tree);
}

#[test]
fn reconcile_clears_when_the_history_cursor_moved() {
    let mut state = shown_state();
    state.on_working_tree = true;
    state.history.as_mut().unwrap().cursor = 1;
    assert!(reconcile(&mut state, 0));
    assert!(!state.on_working_tree);
}

#[test]
fn reconcile_keeps_the_flag_when_the_row_still_shows_and_the_cursor_did_not_move() {
    let mut state = shown_state();
    state.on_working_tree = true;
    assert!(!reconcile(&mut state, 0));
    assert!(state.on_working_tree);
}

#[test]
fn reconcile_is_a_no_op_when_the_flag_is_already_off() {
    let mut state = shown_state();
    assert!(!reconcile(&mut state, 0));
    assert!(!state.on_working_tree);
}

fn detail_shown_state() -> AppState {
    let mut state = shown_state();
    let mut ids = 0u64;
    let mut next_id = || {
        ids += 1;
        JobId::new(super::super::detail::LOCAL_JOB_BIT | ids)
    };
    state
        .detail
        .open(DetailSpec::Commit(head_id()), &mut next_id);
    state
}

fn head_id() -> mergewright_core::oid::Oid {
    mergewright_core::oid::Oid::from_bytes(&[1u8; 20]).unwrap()
}

fn identity() -> mergewright_core::commit::Identity {
    mergewright_core::commit::Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: mergewright_core::commit::Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    }
}

fn commit_row(byte: u8) -> mergewright_graph::Row {
    mergewright_graph::Row {
        drawn_parents: Vec::new(),
        commit: mergewright_core::commit::Commit {
            id: mergewright_core::oid::Oid::from_bytes(&[byte; 20]).unwrap(),
            parents: Vec::new(),
            author: identity(),
            committer: identity(),
            message: b"a commit".to_vec(),
        },
        lane: 0,
        incoming: Vec::new(),
        outgoing: Vec::new(),
        lanes: Vec::new(),
        marker: None,
    }
}

/// `shown_state` with a two-row window on hand (rows 0 and 1, distinct
/// commits), so `selected_spec` names a real commit as the cursor moves
/// — what the compare-follow tests below need
/// (m2-details-and-diffs/11).
fn state_with_rows() -> AppState {
    let mut state = shown_state();
    let history = state.history.as_mut().expect("shown_state sets one");
    history.window = Some(crate::history::Window {
        request: 1,
        first: 0,
        rows: vec![commit_row(1), commit_row(2)].into(),
        laid_out: 2,
        done: false,
        promoted: false,
        fold: [].into(),
        sought: None,
    });
    state
}

#[test]
fn follow_selection_follows_onto_the_working_tree_row() {
    let mut state = detail_shown_state();
    state.on_working_tree = true;
    let mut ids = 100u64;
    let commands = follow_selection(&mut state, &mut || {
        ids += 1;
        JobId::new(super::super::detail::LOCAL_JOB_BIT | ids)
    });
    assert_eq!(state.detail.spec, Some(DetailSpec::WorkingTree));
    assert!(!commands.is_empty());
}

#[test]
fn follow_selection_is_a_no_op_while_hidden() {
    let mut state = shown_state();
    state.on_working_tree = true;
    let mut ids = 100u64;
    let commands = follow_selection(&mut state, &mut || {
        ids += 1;
        JobId::new(super::super::detail::LOCAL_JOB_BIT | ids)
    });
    assert!(commands.is_empty());
    assert_eq!(state.detail.spec, None);
}

/// While a base is pinned, moving the cursor opens a fresh `Compare`
/// against the new row each time — never `Commit`
/// (m2-details-and-diffs/11).
#[test]
fn follow_selection_opens_a_fresh_compare_as_the_cursor_moves() {
    let mut state = state_with_rows();
    let mut ids = 0u64;
    let mut next_id = || {
        ids += 1;
        JobId::new(super::super::detail::LOCAL_JOB_BIT | ids)
    };
    let base_oid = commit_row(1).commit.id;
    state
        .detail
        .open(DetailSpec::Commit(base_oid), &mut next_id);
    state.compare_base = Some(super::super::detail::CompareBase {
        oid: base_oid,
        row: 0,
    });
    state.history.as_mut().unwrap().cursor = 1;

    let commands = follow_selection(&mut state, &mut next_id);

    assert!(!commands.is_empty());
    assert_eq!(
        state.detail.spec,
        Some(DetailSpec::Compare {
            base: base_oid,
            head: commit_row(2).commit.id,
        })
    );
    assert_eq!(
        state.compare_base,
        Some(super::super::detail::CompareBase {
            oid: base_oid,
            row: 0
        }),
        "the base stays set as the compare follows"
    );
}

/// Onto the working-tree row, a pinned base is left alone and the pane
/// follows to `WorkingTree` as usual (m2-details-and-diffs/11).
#[test]
fn follow_selection_onto_the_working_tree_row_keeps_the_base() {
    let mut state = state_with_rows();
    let mut ids = 0u64;
    let mut next_id = || {
        ids += 1;
        JobId::new(super::super::detail::LOCAL_JOB_BIT | ids)
    };
    let base_oid = commit_row(1).commit.id;
    state
        .detail
        .open(DetailSpec::Commit(base_oid), &mut next_id);
    state.compare_base = Some(super::super::detail::CompareBase {
        oid: base_oid,
        row: 0,
    });
    state.on_working_tree = true;

    let commands = follow_selection(&mut state, &mut next_id);

    assert!(!commands.is_empty());
    assert_eq!(state.detail.spec, Some(DetailSpec::WorkingTree));
    assert!(state.compare_base.is_some(), "the base stays set");
}

/// Back onto the base's own row the pane opens `Compare { base, head:
/// base }`, whose header says `x` clears it (m7-14; was: no command).
#[test]
fn the_cursor_on_the_base_says_x_clears() {
    let mut state = state_with_rows();
    let mut ids = 0u64;
    let mut next_id = || {
        ids += 1;
        JobId::new(super::super::detail::LOCAL_JOB_BIT | ids)
    };
    let base_oid = commit_row(1).commit.id;
    let head_oid = commit_row(2).commit.id;
    state.detail.open(
        DetailSpec::Compare {
            base: base_oid,
            head: head_oid,
        },
        &mut next_id,
    );
    state.compare_base = Some(super::super::detail::CompareBase {
        oid: base_oid,
        row: 0,
    });
    state.history.as_mut().unwrap().cursor = 0;

    let commands = follow_selection(&mut state, &mut next_id);

    assert!(!commands.is_empty());
    assert_eq!(
        state.detail.spec,
        Some(DetailSpec::Compare {
            base: base_oid,
            head: base_oid,
        })
    );
}
