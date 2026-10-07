//! 20's Navigation box: `^`, `⏎`, `Esc` along the trail, `is_write`.

use super::*;

#[test]
fn caret_opens_the_parent_and_esc_comes_back_to_the_same_line() {
    let state = blamed_at(22);
    let line = cursor_line(&state);
    let (parent, commands) = press(&state, KeyCode::Char('^'));
    let (_, request) = blame_job(&commands);
    assert_eq!(request.rev, Rev::Commit(lina_parent()));
    assert_eq!(parent.trail.places.len(), 2);

    let (back, _) = press(&parent, KeyCode::Escape);
    assert_eq!(back.trail.places.len(), 1);
    assert_eq!(cursor_line(&back), line);
}

#[test]
fn caret_lands_where_the_line_came_from() {
    let state = blamed_at(22);
    let (parent, commands) = press(&state, KeyCode::Char('^'));
    let (job, request) = blame_job(&commands);
    assert!(request.came_from.is_some(), "asks where the line came from");
    let (read, _) = answer(
        &parent,
        job,
        Ok(BlameAnswer {
            blame: a_blame(false, false),
            line: Some(9),
        }),
    );
    assert_eq!(cursor_line(&read), 9);
}

fn hunk(old_start: u32, old_lines: u32, new_start: u32, new_lines: u32) -> Hunk {
    Hunk {
        old_start,
        old_lines,
        new_start,
        new_lines,
        heading: Vec::new(),
        lines: Vec::new(),
    }
}

#[test]
fn came_from_a_line_mid_hunk_is_the_hunks_first_old_line() {
    assert_eq!(came_from(&[hunk(10, 2, 10, 5)], 12), 10);
}

#[test]
fn came_from_a_pure_add_at_line_1_is_line_1() {
    assert_eq!(came_from(&[hunk(0, 0, 1, 3)], 2), 1);
}

#[test]
fn came_from_a_line_after_a_hunk_shifts_by_it() {
    assert_eq!(came_from(&[hunk(10, 2, 10, 5)], 20), 17);
}

/// git writes a deletion of old 5–6 as `@@ -5,2 +4,0 @@`: a zero-count
/// `new_start` is the line *before* it (review m7-20e-20g Minor 2).
#[test]
fn came_from_the_line_before_a_pure_deletion_is_unshifted() {
    assert_eq!(came_from(&[hunk(5, 2, 4, 0)], 4), 4);
    assert_eq!(came_from(&[hunk(5, 2, 4, 0)], 5), 7);
}

#[test]
fn caret_at_the_root_says_nothing_before_it() {
    let mut state = blamed_at(22);
    if let Some(blame) = state.trail.top_mut() {
        blame.cursor = 4; // line 5, Mo's root commit
    }
    let (next, commands) = press(&state, KeyCode::Char('^'));
    assert_eq!(next.trail.places.len(), 1);
    assert!(!commands.iter().any(|c| matches!(c, Command::ReadBlame(..))));
    assert!(
        matches!(&next.status, Status::Sticky { reason, .. } if reason.ends_with("added this file · nothing before it")),
        "{:?}",
        next.status
    );
}

#[test]
fn caret_on_an_uncommitted_line_says_so() {
    let mut state = pane_at(22);
    state.detail.body_cursor = 22;
    place_with(&mut state, a_blame(false, true), 2);
    let (next, _) = press(&state, KeyCode::Char('^'));
    assert!(
        matches!(&next.status, Status::Sticky { reason, .. } if reason == "line 3 isn't committed yet · nothing before it"),
        "{:?}",
        next.status
    );
}

#[test]
fn enter_lands_on_the_commit_with_the_diff_at_this_line() {
    let state = blamed_at(22);
    let (landed, _) = press(&state, KeyCode::Enter);
    assert!(landed.trail.hidden(), "the trail waits under the graph");
    let landing = landed.line_landing.as_ref().expect("a landing");
    assert_eq!(landing.spec, DetailSpec::Commit(lina()));
    assert_eq!(landing.at.map(|at| at.line), Some(22));
    assert_eq!(landing.path, PathBuf::from("a.rs"));

    // Renamed since: the commit's own path, and the line in its version.
    let mut renamed = a_blame(false, false);
    if let Some(commit) = renamed.commits.get_mut(&lina()) {
        commit.path = "old.rs".into();
    }
    if let Some(line) = renamed.lines.get_mut(21) {
        line.orig_line = 19;
    }
    let mut state = pane_at(22);
    place_with(&mut state, renamed, 21);
    let (landed, _) = press(&state, KeyCode::Enter);
    let landing = landed.line_landing.as_ref().expect("a landing");
    assert_eq!(landing.spec, DetailSpec::Commit(lina()));
    assert_eq!(
        landing.path,
        PathBuf::from("old.rs"),
        "the commit's own path"
    );
    assert_eq!(landing.at.map(|at| at.line), Some(19));
}

#[test]
fn enter_on_an_uncommitted_line_lands_in_the_changes_pane() {
    let mut state = pane_at(22);
    state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![graft_git::status::StatusEntry {
            path: "a.rs".into(),
            original_path: None,
            staged: None,
            unstaged: Some(graft_git::status::Change::Modified),
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }],
    }));
    place_with(&mut state, a_blame(false, true), 2);
    let (landed, _) = press(&state, KeyCode::Enter);
    let landing = landed.line_landing.as_ref().expect("a landing");
    assert_eq!(landing.spec, DetailSpec::WorkingTree);
}

#[test]
fn esc_in_the_diff_after_enter_returns_to_blame_same_line() {
    let state = blamed_at(22);
    let line = cursor_line(&state);
    let (mut landed, _) = press(&state, KeyCode::Enter);
    landed.detail.focus = DetailFocus::Diff;
    landed.detail.shown = true;
    let (back, _) = press(&landed, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "blame is back");
    assert_eq!(cursor_line(&back), line);
}

#[test]
fn esc_from_blame_returns_where_b_was_pressed() {
    let before = pane_at(22);
    let state = blamed_at(22);
    let (back, _) = press(&state, KeyCode::Escape);
    assert!(!back.trail.is_live());
    assert_eq!(back.detail.focus, before.detail.focus);
    assert_eq!(back.detail.body_cursor, before.detail.body_cursor);
    assert_eq!(
        back.history.as_ref().map(|history| history.cursor),
        before.history.as_ref().map(|history| history.cursor)
    );

    // After `⏎`, `Esc Esc`: with its landing still seeking (Lina's commit is
    // off the rows on hand), the landing is dropped and nothing moved.
    let (mut landed, _) = press(&state, KeyCode::Enter);
    landed.detail.focus = DetailFocus::Diff;
    let left = esc_esc(&landed);
    assert_eq!(left.line_landing, None, "the ⏎ landing must not land later");
    assert_eq!(left.detail.spec, before.detail.spec);
    assert_eq!(left.detail.focus, before.detail.focus);
    assert_eq!(left.detail.body_cursor, before.detail.body_cursor);
    assert_graph_cursor(&left, &before);

    // With the graph moved (the landing got there), leaving lands back.
    if let Some(history) = landed.history.as_mut() {
        history.cursor += 1;
    }
    let left = esc_esc(&landed);
    assert_graph_cursor(&left, &before);
    assert_landed_back(&left, &before);
}

/// Back on `before`'s spec, file, line and focus: landed already (the pane
/// had them on hand), or on the way.
fn assert_landed_back(state: &AppState, before: &AppState) {
    let at = diff_cursor(before).and_then(|cursor| cursor.at);
    if let Some(landing) = &state.line_landing {
        assert_eq!(Some(landing.spec), before.detail.spec);
        assert_eq!(landing.path, PathBuf::from("a.rs"));
        assert_eq!(landing.at, at);
        assert_eq!(landing.then_focus, Some(before.detail.focus));
    } else {
        assert_eq!(state.detail.spec, before.detail.spec);
        assert_eq!(state.detail.focus, before.detail.focus);
        let cursor = diff_cursor(state).expect("a cursor");
        assert_eq!(cursor.path, PathBuf::from("a.rs"));
        assert_eq!(cursor.at, at);
    }
}

/// `Esc` back to blame from the landed pane, `Esc` again out of the trail.
fn esc_esc(landed: &AppState) -> AppState {
    let (blame, _) = press(landed, KeyCode::Escape);
    assert!(blame.trail.on_screen().is_some(), "blame first");
    let (left, _) = press(&blame, KeyCode::Escape);
    assert!(!left.trail.is_live());
    left
}

fn assert_graph_cursor(state: &AppState, before: &AppState) {
    assert_eq!(
        state.history.as_ref().map(|history| history.cursor),
        before.history.as_ref().map(|history| history.cursor),
        "the graph cursor"
    );
}

#[test]
fn esc_on_the_graph_closes_search_before_returning_to_blame() {
    // Landed and settled: no seek still walking (its `Esc` rung comes
    // before the search's), the trail waiting under the graph, the pane
    // open: it is not closed first (20i).
    let mut landed = pane_at(22);
    hide_a_trail(&mut landed);
    landed.detail.focus = DetailFocus::Graph;
    let query = crate::search::Query::parse("retry").expect("a non-empty query");
    let mut search = crate::search::Search::new(query, 0, (0, 0));
    search.state = crate::search::SearchState::Done { hits: 0 };
    landed.search = Some(search);

    let (closed, _) = press(&landed, KeyCode::Escape);
    assert!(closed.search.is_none(), "the search's line went first");
    assert!(closed.trail.hidden(), "blame still waits");

    let (back, _) = press(&closed, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "then back to blame");
    assert_eq!(cursor_line(&back), 22);
}

#[test]
fn is_write_covers_every_operation_launch() {
    use std::path::Path;
    let job = JobId::new(9);
    let conflict = Arc::new(graft_git::conflict::ConflictFile {
        regions: Vec::new(),
        fingerprint: lina(),
    });
    let keys = || Arc::new(graft_ui::keymap::Keymap::shipped().clone());
    let launches = [
        Command::Operate(
            job,
            Box::new(graft_ops::operation::Operation::Stage(
                graft_ops::operation::Target::Paths(vec!["a.rs".into()]),
            )),
            keys(),
        ),
        Command::Resume(job, graft_ops::apply::Resume::Continue, keys()),
        Command::RevertTo(
            job,
            graft_core::journal::EntryId::new(1_000, [0; 10]),
            false,
            keys(),
        ),
        Command::Undo(job, false, keys()),
        Command::Redo(job, false, keys()),
        Command::TakeSide(
            job,
            crate::update::events::TakeRequest {
                path: "a.rs".into(),
                file: conflict,
                at: crate::update::events::TakeAt::Every,
                take: graft_git::conflict::Take::Ours,
            },
        ),
        Command::OpenExternalEditor {
            path: "a.rs".into(),
            line: Some(3),
        },
        Command::OpenConflictEditor {
            path: "a.rs".into(),
        },
        Command::OpenMergetool {
            path: "a.rs".into(),
        },
    ];
    for launch in launches {
        assert!(is_write(&launch), "{launch:?}");
        let mut state = commit_pane(JobId::new(1), false);
        hide_a_trail(&mut state);
        let mut commands = vec![launch];
        after_commands(&mut state, &mut commands);
        assert!(!state.trail.is_live(), "{:?} ends the trail", commands[0]);
    }
    // A read of the same file is not a write.
    let read = Command::OpenConflict(job, Path::new("a.rs").to_path_buf());
    assert!(!is_write(&read));
}
