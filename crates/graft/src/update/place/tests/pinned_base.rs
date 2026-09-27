//! 20's pinned-base and shallow rulings, and the trail's bounds.

use super::*;

/// `pane_at(row)` as a compare: Mo's commit pinned as the base, the merge
/// the head.
fn compare_at(row: usize) -> (AppState, CompareBase) {
    let mut state = pane_at(row);
    let (head, _) = pane_commit(&state);
    let base = CompareBase { oid: mo(), row: 5 };
    state.compare_base = Some(base);
    state.detail.spec = Some(DetailSpec::Compare { base: mo(), head });
    (state, base)
}

/// `B` on a compare at line 22, answered with `blame`.
fn blamed_with_a_base(blame: Blame) -> (AppState, CompareBase) {
    let (state, base) = compare_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, _) = blame_job(&commands);
    let read = answer(&opened, job, Ok(BlameAnswer { blame, line: None })).0;
    (read, base)
}

const SET_ASIDE: &str = "compare base b2b2b2b set aside · it comes back when you leave blame";

#[test]
fn b_sets_a_pinned_base_aside() {
    let (state, base) = blamed_with_a_base(a_blame(false, false));
    assert_eq!(state.compare_base, None, "nothing pinned while blame lives");
    assert_eq!(
        state.trail.origin.as_ref().and_then(|origin| origin.base),
        Some(base)
    );
}

#[test]
fn enter_with_a_compare_base_lands_on_the_commits_own_changes() {
    let (state, _) = blamed_with_a_base(a_blame(false, false));
    let (landed, _) = press(&state, KeyCode::Enter);
    let landing = landed.line_landing.as_ref().expect("a landing");
    assert_eq!(landing.spec, DetailSpec::Commit(lina()), "not a compare");
    assert_eq!(landing.at.map(|at| at.line), Some(22));
    assert_eq!(landed.compare_base, None, "no pin");
    assert_eq!(landed.status, Status::Waiting(SET_ASIDE.to_owned()));
}

#[test]
fn esc_after_enter_with_a_base_goes_back_to_blame() {
    let (state, base) = blamed_with_a_base(a_blame(false, false));
    let (mut landed, _) = press(&state, KeyCode::Enter);
    landed.detail.focus = DetailFocus::Diff;
    landed.detail.shown = true;
    let (back, _) = press(&landed, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "one Esc: blame is back");
    assert_eq!(cursor_line(&back), 22);
    assert_eq!(back.compare_base, None, "the base is still set aside");
    assert_eq!(
        back.trail.origin.as_ref().and_then(|origin| origin.base),
        Some(base)
    );
}

#[test]
fn esc_from_blame_restores_the_compare_base() {
    let (before, _) = compare_at(22);
    let (state, base) = blamed_with_a_base(a_blame(false, false));
    let (back, _) = press(&state, KeyCode::Escape);
    assert!(!back.trail.is_live());
    assert_eq!(back.compare_base, Some(base), "pinned again");
    assert_eq!(back.detail.spec, before.detail.spec, "the same compare");
    assert_eq!(back.detail.selected_entry(), before.detail.selected_entry());
    assert_eq!(back.detail.body_cursor, before.detail.body_cursor);
    assert_eq!(back.detail.focus, before.detail.focus);
    // m7-14: re-pinning restores, it does not press `x` — the header is the
    // same compare's words, not the base's `x clears it`.
    assert_eq!(
        crate::view::detail_view(&back).header.subject,
        crate::view::detail_view(&before).header.subject,
        "the same compare header"
    );

    // Via an uncommitted row: `⏎` went to the changes pane, and leaving
    // still puts the base back and lands on the compare.
    let (mut state, base) = blamed_with_a_base(a_blame(false, true));
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
    if let Some(blame) = state.trail.top_mut() {
        blame.cursor = 2;
    }
    let (mut landed, _) = press(&state, KeyCode::Enter);
    assert_eq!(
        landed.line_landing.as_ref().map(|landing| landing.spec),
        Some(DetailSpec::WorkingTree)
    );
    landed.detail.focus = DetailFocus::Diff;
    landed.detail.shown = true;
    let (blame, _) = press(&landed, KeyCode::Escape);
    let (left, _) = press(&blame, KeyCode::Escape);
    assert!(!left.trail.is_live());
    assert_eq!(left.compare_base, Some(base), "the base kept");
    let going_to = left
        .line_landing
        .as_ref()
        .map(|landing| landing.spec)
        .or(left.detail.spec);
    assert_eq!(going_to, before.detail.spec, "back on the compare");
}

#[test]
fn a_write_with_a_base_aside_pins_it_again() {
    let (mut state, base) = blamed_with_a_base(a_blame(false, false));
    let mut commands = vec![Command::Undo(JobId::new(9), false)];
    after_commands(&mut state, &mut commands);
    assert!(!state.trail.is_live());
    assert_eq!(state.compare_base, Some(base));
}

/// Through `update`, not `after_commands` (review m7-20e-20g Major 2): the
/// hook `update` calls after every event is what ends the trail.
#[test]
fn u_on_the_graph_ends_a_hidden_trail_and_pins_the_base_again() {
    let (state, base) = blamed_with_a_base(a_blame(false, false));
    let (mut hidden, _) = press(&state, KeyCode::Enter);
    hidden.detail.focus = DetailFocus::Graph;
    assert!(hidden.trail.hidden(), "the trail waits under the graph");
    assert_eq!(hidden.compare_base, None, "the base is set aside");

    let (after, commands) = press(&hidden, KeyCode::Char('u'));
    assert!(
        commands.iter().any(is_write),
        "`u` asked for a write: {commands:?}"
    );
    assert!(!after.trail.is_live(), "the write ended the trail");
    assert_eq!(after.compare_base, Some(base), "the base is pinned again");
}

#[test]
fn x_while_blame_is_hidden_ends_the_trail_and_says_so() {
    let (state, base) = blamed_with_a_base(a_blame(false, false));
    let (mut landed, _) = press(&state, KeyCode::Enter);
    landed.detail.focus = DetailFocus::Graph;
    assert!(landed.trail.hidden());
    let (next, _) = press(&landed, KeyCode::Char('x'));
    assert!(!next.trail.is_live(), "a new compare ends the trail");
    let pinned = next.compare_base.expect("x pinned a base");
    assert_ne!(pinned, base, "the set-aside base is dropped");
    assert_eq!(
        next.status,
        Status::Waiting(format!("left blame for compare base {}", short(pinned.oid)))
    );
}

#[test]
fn a_job_side_refusal_restores_the_base() {
    let (state, base) = compare_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    assert_eq!(opened.compare_base, None, "set aside while it reads");
    let (job, _) = blame_job(&commands);
    let (refused, _) = answer(&opened, job, Err(BlameFailure::Binary));
    assert!(!refused.trail.is_live());
    assert_eq!(refused.compare_base, Some(base));
}

// ---- A shallow clone (20's shallow ruling, Part C) ----

/// [`a_blame`] with Lina's commit on the shallow boundary.
fn shallow_blame() -> Blame {
    let mut blame = a_blame(false, false);
    if let Some(commit) = blame.commits.get_mut(&lina()) {
        commit.shallow = true;
    }
    blame
}

const SHALLOW_22: &str =
    "line 22 is from a1a1a1a or older · this clone is shallow; nothing older was fetched";

#[test]
fn a_shallow_boundary_line_is_marked_shallow() {
    use graft_ui::blame::RowMark;
    let mut blame = shallow_blame();
    if let Some(line) = blame.lines.get_mut(9) {
        line.mark = LineMark::Skipped; // shallow outranks skipped
    }
    if let Some(line) = blame.lines.get_mut(10) {
        line.mark = LineMark::Unblamable; // unblamable outranks shallow
    }
    let rows = rows::blame_rows(&blame);
    assert_eq!(rows[21].mark, RowMark::Shallow);
    assert_eq!(rows[9].mark, RowMark::Shallow);
    assert_eq!(rows[10].mark, RowMark::Unblamable);
    assert_eq!(rows[4].mark, RowMark::Plain, "Mo's line is not shallow");
    let uncommitted = rows::blame_rows(&{
        let mut blame = shallow_blame();
        blame.lines[2].commit = None;
        blame
    });
    assert_eq!(uncommitted[2].mark, RowMark::NotCommitted);
}

#[test]
fn enter_on_a_shallow_line_says_the_clone_is_shallow_and_opens_nothing() {
    let mut state = pane_at(22);
    place_with(&mut state, shallow_blame(), 21);
    let (next, commands) = press(&state, KeyCode::Enter);
    assert!(
        !commands.iter().any(|c| matches!(
            c,
            Command::ReadBlame(..)
                | Command::ListFiles(..)
                | Command::ReadFileDiff(..)
                | Command::ShowRows(..)
        )),
        "no job: {commands:?}"
    );
    assert!(next.line_landing.is_none(), "opens nothing");
    assert!(next.trail.on_screen().is_some(), "blame stays");
    assert_eq!(next.status, Status::sticky(SHALLOW_22));
}

#[test]
fn caret_on_a_shallow_line_says_the_clone_is_shallow_not_nothing_before_it() {
    let mut blame = shallow_blame();
    // A boundary commit is parentless to git: the root check alone would
    // say "added this file".
    if let Some(commit) = blame.commits.get_mut(&lina()) {
        commit.previous = None;
    }
    let mut state = pane_at(22);
    place_with(&mut state, blame, 21);
    let (next, commands) = press(&state, KeyCode::Char('^'));
    assert!(!commands.iter().any(|c| matches!(c, Command::ReadBlame(..))));
    assert_eq!(next.trail.places.len(), 1);
    assert_eq!(next.status, Status::sticky(SHALLOW_22));
}

#[test]
fn a_write_clears_the_trail() {
    let mut state = commit_pane(JobId::new(1), false);
    hide_a_trail(&mut state);
    let mut commands = vec![Command::Undo(JobId::new(9), false)];
    after_commands(&mut state, &mut commands);
    assert!(!state.trail.is_live());
}

#[test]
fn the_trail_holds_16() {
    let mut state = blamed_at(22);
    for _ in 0..TRAIL_MAX + 3 {
        let _commands = open_place(
            &mut state,
            Rev::Commit(lina()),
            "a.rs".into(),
            graft_ui::blame::BlameRev::Now,
            Want::Line(1),
            None,
        );
    }
    assert_eq!(state.trail.places.len(), TRAIL_MAX);
}
