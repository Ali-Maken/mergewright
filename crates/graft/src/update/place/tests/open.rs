//! 20's Open box: `B` from each place it answers.

use super::*;

#[test]
fn b_on_a_diff_line_opens_blame_at_that_line() {
    let state = pane_at(22);
    let at = diff_cursor(&state)
        .and_then(|cursor| cursor.at)
        .expect("a line");

    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, request) = blame_job(&commands);
    assert_eq!(request.path, PathBuf::from("a.rs"));
    assert!(matches!(request.rev, Rev::Commit(_)), "{request:?}");
    assert!(opened.trail.on_screen().is_some(), "the place is on screen");

    let (read, _) = answer(
        &opened,
        job,
        Ok(BlameAnswer {
            blame: a_blame(false, false),
            line: None,
        }),
    );
    assert_eq!(cursor_line(&read), at.line);
}

#[test]
fn b_with_the_graph_focused_is_unchanged() {
    let mut state = pane_at(22);
    state.detail.focus = DetailFocus::Graph;

    let (next, commands) = press(&state, KeyCode::Char('B'));
    assert!(!next.trail.is_live());
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ReadBlame(..))),
        "{commands:?}"
    );
}

#[test]
fn a_job_side_refusal_pops_and_says_so() {
    let state = pane_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, _) = blame_job(&commands);

    let (refused, _) = answer(&opened, job, Err(BlameFailure::Binary));
    assert!(!refused.trail.is_live(), "the place is gone");
    assert!(
        matches!(&refused.status, Status::Sticky { reason, .. } if reason.contains("is binary · no blame")),
        "{:?}",
        refused.status
    );
}

fn diff_line(
    kind: graft_ui::detail::LineKindView,
    old_no: Option<u32>,
    new_no: Option<u32>,
) -> graft_ui::detail::LineView {
    graft_ui::detail::LineView {
        kind,
        old_no,
        new_no,
        text: graft_term::sanitize::sanitize_line("x"),
        words: Vec::new(),
        syntax: Vec::new(),
        pair: None,
    }
}

/// Two hunks. Body rows: `0` heading · `1` ctx 1/1 · `2` − old 2 ·
/// `3` + new 2 · `4` + new 3 · `5` ctx 3/4 · `6` heading · `7` ctx 10/11 ·
/// `8` − old 11 · `9` ctx 12/12. The second hunk only removes.
fn two_hunks() -> graft_ui::detail::DiffView {
    use graft_ui::detail::HunkView;
    use graft_ui::detail::LineKindView::{Added, Context, HunkHeading, Removed};
    let heading = |text: &str| graft_ui::detail::LineView {
        text: graft_term::sanitize::sanitize_line(text),
        ..diff_line(HunkHeading, None, None)
    };
    graft_ui::detail::DiffView {
        hunks: vec![
            HunkView {
                heading: graft_term::sanitize::sanitize_line("@@ -1,3 +1,4 @@"),
                lines: vec![
                    heading("@@ -1,3 +1,4 @@"),
                    diff_line(Context, Some(1), Some(1)),
                    diff_line(Removed, Some(2), None),
                    diff_line(Added, None, Some(2)),
                    diff_line(Added, None, Some(3)),
                    diff_line(Context, Some(3), Some(4)),
                ],
            },
            HunkView {
                heading: graft_term::sanitize::sanitize_line("@@ -10,3 +11,2 @@"),
                lines: vec![
                    heading("@@ -10,3 +11,2 @@"),
                    diff_line(Context, Some(10), Some(11)),
                    diff_line(Removed, Some(11), None),
                    diff_line(Context, Some(12), Some(12)),
                ],
            },
        ],
        ..crate::update::detail::tests::empty_diff_view()
    }
}

/// `pane_at(row)` with [`two_hunks`] as `a.rs`'s diff.
fn two_hunks_at(row: usize) -> AppState {
    let mut state = pane_at(row);
    state.detail.diff = Loading::Ready(Arc::new(two_hunks()));
    state
}

/// What the top place is waiting for.
fn want(state: &AppState) -> Want {
    match state.trail.top().map(|blame| &blame.body) {
        Some(BlameLoad::Reading { want, .. }) => *want,
        other => panic!("not reading: {other:?}"),
    }
}

/// `B` answered with words and nothing opened.
fn refused_with(state: &AppState, words: &str) {
    let (next, commands) = press(state, KeyCode::Char('B'));
    assert!(!next.trail.is_live(), "no place opened");
    assert!(
        !commands.iter().any(|c| matches!(c, Command::ReadBlame(..))),
        "{commands:?}"
    );
    assert!(
        matches!(&next.status, Status::Sticky { reason, .. } if reason == words),
        "{:?}",
        next.status
    );
}

#[test]
fn b_on_a_removed_line_blames_the_parent_at_the_old_line() {
    let state = two_hunks_at(2);
    let (_, parent) = pane_commit(&state);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (_, request) = blame_job(&commands);
    assert_eq!(request.rev, Rev::Commit(parent));
    assert_eq!(request.path, PathBuf::from("a.rs"));
    assert_eq!(want(&opened), Want::Line(2));
}

#[test]
fn b_on_a_renamed_files_removed_line_uses_the_old_path() {
    let mut state = two_hunks_at(2);
    let entry = state.detail.selected_entry().expect("a file row");
    if let Some(list) = state.detail.list.as_mut() {
        let mut renamed = (**list).clone();
        renamed.files[usize::try_from(entry).expect("small")].old_path = Some(b"old.rs".to_vec());
        *list = Arc::new(renamed);
    }
    let (_, commands) = press(&state, KeyCode::Char('B'));
    let (_, request) = blame_job(&commands);
    assert_eq!(request.path, PathBuf::from("old.rs"), "the parent's path");

    // An added line of the same file is blamed at its own, new path.
    let mut added = state.clone();
    added.detail.body_cursor = 3;
    let (_, commands) = press(&added, KeyCode::Char('B'));
    assert_eq!(blame_job(&commands).1.path, PathBuf::from("a.rs"));
}

#[test]
fn b_on_a_file_row_lands_on_the_first_changed_line() {
    let mut state = two_hunks_at(9);
    state.detail.focus = DetailFocus::Files;
    let (commit, _) = pane_commit(&state);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (_, request) = blame_job(&commands);
    assert_eq!(request.rev, Rev::Commit(commit));
    assert_eq!(want(&opened), Want::Line(2), "the first added line");
}

#[test]
fn b_on_a_heading_lands_on_the_hunks_first_changed_line() {
    // The second hunk's heading: its only change is old line 11, removed.
    let state = two_hunks_at(6);
    let (_, parent) = pane_commit(&state);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (_, request) = blame_job(&commands);
    assert_eq!(request.rev, Rev::Commit(parent));
    assert_eq!(want(&opened), Want::Line(11));
}

#[test]
fn b_on_a_compare_blames_head_or_base() {
    let base = mo();
    let head = lina();
    let mut state = two_hunks_at(3);
    state.detail.spec = Some(DetailSpec::Compare { base, head });

    let (opened, commands) = press(&state, KeyCode::Char('B'));
    assert_eq!(
        blame_job(&commands).1.rev,
        Rev::Commit(head),
        "an added line"
    );
    assert_eq!(want(&opened), Want::Line(2));

    state.detail.body_cursor = 2;
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    assert_eq!(
        blame_job(&commands).1.rev,
        Rev::Commit(base),
        "a removed line"
    );
    assert_eq!(want(&opened), Want::Line(2));
}

/// The changes pane on `a.rs` unstaged, [`two_hunks`] arrived, the diff
/// cursor on body row `row`; `HEAD` is Lina's commit.
fn changes_at(row: usize) -> AppState {
    use graft_ui::detail::WorkingTreeSection;
    let mut detail =
        crate::update::detail::tests::pane_on(&[(WorkingTreeSection::Unstaged, &["a.rs"])], 1, 0);
    detail.focus = DetailFocus::Diff;
    detail.diff = Loading::Ready(Arc::new(two_hunks()));
    detail.body_cursor = row;
    let refs = crate::refs::badges(
        &[],
        &graft_git::repo::HeadState::Detached(lina()),
        &std::collections::HashMap::new(),
        &[],
        |_, _| Err("no upstreams on this fixture".to_owned()),
    )
    .expect("badges");
    AppState {
        size: crate::update::Size {
            columns: 120,
            rows: 40,
        },
        detail,
        refs: Some(Arc::new(refs)),
        ..AppState::default()
    }
}

#[test]
fn b_on_the_changes_pane_blames_the_working_tree() {
    let state = changes_at(3);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (_, request) = blame_job(&commands);
    assert_eq!(request.rev, Rev::WorkingTree);
    assert_eq!(request.path, PathBuf::from("a.rs"));
    assert_eq!(want(&opened), Want::Line(2));
}

/// B1 `:1093-1094`: a file row blames the file on disk, cursor on its first
/// changed line — the same rule as a commit's file row (review Minor 4).
#[test]
fn b_on_the_changes_panes_file_row_blames_the_working_tree() {
    let mut state = changes_at(9);
    state.detail.focus = DetailFocus::Files;
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (_, request) = blame_job(&commands);
    assert_eq!(request.rev, Rev::WorkingTree, "the file on disk");
    assert_eq!(request.path, PathBuf::from("a.rs"));
    assert_eq!(want(&opened), Want::Line(2), "the first added line");
}

#[test]
fn b_on_a_changes_pane_removed_line_blames_head() {
    let state = changes_at(2);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    assert_eq!(blame_job(&commands).1.rev, Rev::Commit(lina()), "HEAD");
    assert_eq!(want(&opened), Want::Line(2));
}

#[test]
fn b_with_staged_and_unstaged_edits_says_the_lines_moved() {
    let mut state = changes_at(2);
    state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![graft_git::status::StatusEntry {
            path: "a.rs".into(),
            original_path: None,
            staged: Some(graft_git::status::Change::Modified),
            unstaged: Some(graft_git::status::Change::Modified),
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }],
    }));
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    assert_eq!(blame_job(&commands).1.rev, Rev::WorkingTree);
    assert_eq!(want(&opened), Want::FirstUncommitted);
    assert_eq!(
        opened.status,
        Status::Waiting(
            "staged and unstaged edits both move this file's lines · on its first uncommitted line"
                .to_owned()
        )
    );
}

#[test]
fn b_on_an_untracked_file_says_so_and_opens_nothing() {
    use graft_ui::detail::WorkingTreeSection;
    let mut detail = crate::update::detail::tests::pane_on(
        &[(WorkingTreeSection::Untracked, &["new.rs"])],
        1,
        0,
    );
    detail.focus = DetailFocus::Files;
    let state = AppState {
        detail,
        ..AppState::default()
    };
    refused_with(&state, "new.rs isn't committed yet · no blame");
}

#[test]
fn b_on_a_binary_file_says_so_and_opens_nothing() {
    let mut state = pane_at(0);
    state.detail.diff = Loading::Ready(Arc::new(graft_ui::detail::DiffView {
        binary: Some(graft_term::sanitize::sanitize_line("Binary files differ")),
        ..crate::update::detail::tests::empty_diff_view()
    }));
    refused_with(&state, "a.rs is binary · no blame");
}

/// m7-power-tools/36 (M3): in the pane `B` is blame, offer or not; the
/// offer's `B` answers only with the graph focused.
#[test]
fn b_in_the_pane_blames_while_the_commit_graph_offer_is_up() {
    let mut state = pane_at(22);
    state.commit_graph = crate::update::CommitGraphOffer::Offered;
    state.system_git = Some(crate::update::GitCheck::Ran(
        graft_git::version::SystemGit::Supported(graft_git::version::GitVersion::MINIMUM),
    ));
    let (next, commands) = press(&state, KeyCode::Char('B'));
    assert!(next.trail.is_live(), "a place opened");
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, Command::WriteCommitGraph)),
        "{commands:?}"
    );
    assert_eq!(next.commit_graph, crate::update::CommitGraphOffer::Offered);
}
