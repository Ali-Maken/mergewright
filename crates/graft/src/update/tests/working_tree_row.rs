//! `update` tests: working tree row. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// [`merge_on_screen`] plus a dirty tree and `HEAD` at row 0 — the
/// uncommitted-changes row is shown, cursor at the top, nothing selected on
/// it yet.
pub(super) fn dirty_on_screen(id: JobId) -> AppState {
    let (mut state, _) = merge_on_screen(id, 0);
    let head_id = state
        .history
        .as_ref()
        .and_then(History::selected)
        .map(|row| row.commit.id)
        .expect("row 0 has arrived");
    state.working_tree = Some(std::sync::Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![
            graft_git::status::StatusEntry {
                path: "staged.txt".into(),
                original_path: None,
                staged: Some(graft_git::status::Change::Modified),
                unstaged: None,
                untracked: false,
                unmerged: false,
                unmerged_how: None,
                submodule: None,
            },
            graft_git::status::StatusEntry {
                path: "unstaged.txt".into(),
                original_path: None,
                staged: None,
                unstaged: Some(graft_git::status::Change::Modified),
                untracked: false,
                unmerged: false,
                unmerged_how: None,
                submodule: None,
            },
            graft_git::status::StatusEntry {
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
    }));
    let badges = crate::refs::badges(
        &[],
        &graft_git::repo::HeadState::Detached(head_id),
        &std::collections::HashMap::new(),
        &[],
        |_, _| unreachable!("no branches on this fixture to count"),
    )
    .expect("no refs to fail reading");
    state.refs = Some(Arc::new(badges));
    state
}

#[test]
fn k_from_the_top_row_selects_the_working_tree_row_and_asks_history_for_nothing() {
    let state = dirty_on_screen(JobId::new(40));
    let (next, commands) = update(&state, key(KeyCode::Char('k')));
    assert!(next.on_working_tree);
    assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(0));
    assert!(commands.is_empty(), "{commands:?}");
}

/// m3-working-tree-undo-and-stash/04: the row is pinned, so `k` still
/// reaches it when `HEAD` is nowhere near the top. Before this it vanished
/// the moment the user scrolled — the one row they can always act on, gone
/// because they scrolled (`KNOWN_GAPS.md`, m2/06b).
#[test]
fn k_reaches_the_working_tree_row_even_with_head_scrolled_off() {
    let mut state = dirty_on_screen(JobId::new(40));
    // `HEAD` on a commit that is not the topmost visible row.
    let elsewhere = graft_core::oid::Oid::from_bytes(&[0x5a; graft_core::oid::Oid::SHA1_BYTES])
        .expect("20 bytes");
    let badges = crate::refs::badges(
        &[],
        &graft_git::repo::HeadState::Detached(elsewhere),
        &std::collections::HashMap::new(),
        &[],
        |_, _| unreachable!("no branches on this fixture to count"),
    )
    .expect("no refs to fail reading");
    state.refs = Some(Arc::new(badges));
    assert!(
        state.uncommitted_row_shown(),
        "the row draws: the tree is dirty"
    );
    assert!(
        !state.uncommitted_row_attached(),
        "but it is not on HEAD's lane"
    );

    let (next, _) = update(&state, key(KeyCode::Char('k')));
    assert!(next.on_working_tree);
}

#[test]
fn enter_on_the_working_tree_row_opens_the_pane_on_it() {
    let mut state = dirty_on_screen(JobId::new(41));
    state.on_working_tree = true;
    let (opened, commands) = update(&state, key(KeyCode::Enter));
    assert!(opened.detail.shown);
    assert_eq!(opened.detail.focus, DetailFocus::Files);
    let lists = list_commands(&commands);
    assert_eq!(lists.len(), 1, "{commands:?}");
    assert_eq!(lists[0].1, graft_git::diff::DiffSpec::WorkingTreeVsHead);
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, Command::ContainedIn(..))),
        "the working tree is not a commit to ask branch containment about: {commands:?}"
    );
}

#[test]
fn j_off_the_row_with_the_pane_shown_follows_to_heads_row() {
    let mut state = dirty_on_screen(JobId::new(42));
    state.on_working_tree = true;
    let (mut opened, _) = update(&state, key(KeyCode::Enter));
    opened.detail.focus = DetailFocus::Graph;
    let (moved, commands) = update(&opened, key(KeyCode::Char('j')));
    assert!(!moved.on_working_tree);
    let lists = list_commands(&commands);
    assert_eq!(lists.len(), 1, "{commands:?}");
    assert_eq!(
        lists[0].1,
        graft_git::diff::DiffSpec::Commit {
            commit: state
                .history
                .as_ref()
                .and_then(History::selected)
                .map(|row| row.commit.id)
                .expect("row 0"),
            against: None
        }
    );
}

#[test]
fn a_clean_tree_while_shown_clears_the_flag_and_follows_off_it() {
    let mut state = dirty_on_screen(JobId::new(43));
    state.on_working_tree = true;
    let (opened, _) = update(&state, key(KeyCode::Enter));
    let (clean, commands) = update(
        &opened,
        AppEvent::Produced(
            JobId::new(1),
            Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
                rewrite: None,
                status: std::sync::Arc::new(graft_git::status::WorkingTreeStatus::default()),
                in_progress: None,
                mergetool: None,
            }))),
        ),
    );
    assert!(!clean.on_working_tree);
    let lists = list_commands(&commands);
    assert_eq!(lists.len(), 1, "{commands:?}");
    assert!(matches!(
        lists[0].1,
        graft_git::diff::DiffSpec::Commit { .. }
    ));
}

#[test]
fn y_on_the_working_tree_row_is_a_no_op() {
    let mut state = dirty_on_screen(JobId::new(44));
    state.on_working_tree = true;
    let (next, commands) = update(&state, key(KeyCode::Char('y')));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next.status, state.status);
}

#[test]
fn a_click_on_the_first_body_row_selects_the_working_tree_row() {
    let mut state = dirty_on_screen(JobId::new(45));
    state.size = Size {
        columns: 80,
        rows: 5,
    };
    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, click);
    assert!(next.on_working_tree);
    assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(0));
}

#[test]
fn a_click_one_row_lower_selects_the_top_history_row_and_clears_the_flag() {
    let mut state = dirty_on_screen(JobId::new(46));
    state.on_working_tree = true;
    state.size = Size {
        columns: 80,
        rows: 5,
    };
    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS + 1,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, click);
    assert!(!next.on_working_tree);
    assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(0));
}
