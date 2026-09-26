//! The blame place and its trail (m7-power-tools/20), through `update` as
//! the event loop delivers keys and job answers.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::blame::{Blame, BlameCommit, BlameLine, IgnoreRevs, LineMark, Rev};
use graft_git::diff::Hunk;
use graft_ui::detail::DetailFocus;

use super::*;
use crate::update::events::{AppEvent, Command, Product};
use crate::update::tests::{commit_pane, key};
use crate::update::update;

/// Lina's commit: lines 1–30 but 5, which Mo wrote.
fn lina() -> Oid {
    Oid::from_bytes(&[0xa1; Oid::SHA1_BYTES]).expect("an oid")
}

fn mo() -> Oid {
    Oid::from_bytes(&[0xb2; Oid::SHA1_BYTES]).expect("an oid")
}

fn lina_parent() -> Oid {
    Oid::from_bytes(&[0xc3; Oid::SHA1_BYTES]).expect("an oid")
}

fn commit(id: Oid, author: &str, previous: Option<Oid>, shallow: bool) -> BlameCommit {
    BlameCommit {
        id,
        author: author.to_owned(),
        author_time: 0,
        summary: format!("{author}'s change"),
        path: "a.rs".into(),
        previous: previous.map(|parent| (parent, PathBuf::from("a.rs"))),
        boundary: previous.is_none(),
        shallow,
    }
}

/// `a.rs`, 30 lines: line 5 Mo's (a root), the rest Lina's (with a parent);
/// lines 22 and 27 say `retry`; with `uncommitted`, line 3 is not
/// committed yet.
fn a_blame(ignore_file: bool, uncommitted: bool) -> Blame {
    let lines = (1..=30)
        .map(|line: u32| BlameLine {
            commit: match line {
                3 if uncommitted => None,
                5 => Some(mo()),
                _ => Some(lina()),
            },
            orig_line: line,
            final_line: line,
            text: if line == 22 || line == 27 {
                format!("retry {line}").into_bytes()
            } else {
                format!("line {line}").into_bytes()
            },
            mark: LineMark::Plain,
        })
        .collect();
    let commits = BTreeMap::from([
        (lina(), commit(lina(), "Lina", Some(lina_parent()), false)),
        (mo(), commit(mo(), "Mo", None, false)),
    ]);
    Blame {
        path: "a.rs".into(),
        rev: Rev::Commit(lina()),
        lines,
        commits,
        ignore_revs: ignore_file.then(|| IgnoreRevs {
            file: ".git-blame-ignore-revs".into(),
            revisions: 2,
            applied: true,
        }),
    }
}

/// A place reading `a.rs` as of Lina's commit, ready, cursor on line 22.
fn ready_place(blame: Blame) -> BlameState {
    let rows: Arc<[graft_ui::blame::BlameRowView]> = rows::blame_rows(&blame).into();
    BlameState {
        path: "a.rs".into(),
        rev: Rev::Commit(lina()),
        title: graft_ui::blame::BlameRev::AsOf(short(lina())),
        options: graft_git::blame::BlameOptions::default(),
        body: BlameLoad::Ready {
            blame: Arc::new(blame),
            rows,
        },
        cursor: 21,
        top: 0,
        find: None,
        find_prompt: None,
        heatmap: false,
        last_click: None,
    }
}

/// A trail of one ready place over `state`, origin where `state` stands.
fn trail_over(state: &mut AppState, place: BlameState, shown: bool) {
    state.trail = Trail {
        places: vec![Place::Blame(Box::new(place))],
        shown,
        origin: Some(Origin {
            spec: state.detail.spec.unwrap_or(DetailSpec::WorkingTree),
            path: "a.rs".into(),
            at: None,
            focus: state.detail.focus,
            base: None,
            spot: Spot::of(state),
        }),
    };
}

/// `update::tests`' sweep: a place on screen with the find and the ignore
/// file as asked.
pub(crate) fn place_on_screen(mut state: AppState, finding: bool, ignore_file: bool) -> AppState {
    let mut place = ready_place(a_blame(ignore_file, false));
    if finding {
        place.find = Some("retry".to_owned());
    }
    trail_over(&mut state, place, true);
    state
}

/// `update::tests`' sweep: a trail waiting under `state`.
pub(crate) fn hide_a_trail(state: &mut AppState) {
    trail_over(state, ready_place(a_blame(false, false)), false);
}

/// `view::frame::tests`' overlay sweep: a place standing over the graph.
pub(crate) fn open_a_place(state: &mut AppState) {
    trail_over(state, ready_place(a_blame(false, false)), true);
}

fn press(state: &AppState, code: KeyCode) -> (AppState, Vec<Command>) {
    update(state, key(code))
}

fn press_all(state: &AppState, codes: &[KeyCode]) -> AppState {
    codes
        .iter()
        .fold(state.clone(), |state, &code| press(&state, code).0)
}

/// The blame job `commands` asks for.
fn blame_job(commands: &[Command]) -> (JobId, BlameRequest) {
    commands
        .iter()
        .find_map(|command| match command {
            Command::ReadBlame(job, request) => Some((*job, (**request).clone())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected a blame job, got {commands:?}"))
}

fn answer(
    state: &AppState,
    job: JobId,
    answer: Result<BlameAnswer, BlameFailure>,
) -> (AppState, Vec<Command>) {
    update(
        state,
        AppEvent::Produced(job, Product::Blame(answer.map(Arc::new))),
    )
}

/// The pane on the merge's diff, the cursor on body row `row`.
fn pane_at(row: usize) -> AppState {
    let mut state = commit_pane(JobId::new(1), false);
    on_a_file(&mut state);
    state.detail.body_cursor = row;
    state.size = crate::update::Size {
        columns: 120,
        rows: 40,
    };
    state
}

/// The files cursor off `commit_pane`'s `src/` row onto `a.rs`, whose diff
/// is the one loaded: `B` has a file to blame.
pub(crate) fn on_a_file(state: &mut AppState) {
    if let Loading::Ready(files) = &mut state.detail.files {
        let mut view = (**files).clone();
        view.cursor = 1;
        *files = Arc::new(view);
    }
}

/// `B` pressed on the pane at `row`, and the job's answer delivered.
fn blamed_at(row: usize) -> AppState {
    let state = pane_at(row);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, _) = blame_job(&commands);
    answer(
        &opened,
        job,
        Ok(BlameAnswer {
            blame: a_blame(false, false),
            line: None,
        }),
    )
    .0
}

fn cursor_line(state: &AppState) -> u32 {
    let blame = state.trail.on_screen().expect("a place on screen");
    let BlameLoad::Ready { blame: lines, .. } = &blame.body else {
        panic!("not ready: {:?}", blame.body);
    };
    lines.lines[blame.cursor].final_line
}

// ---- Open ----

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

/// The commit `pane_at`'s pane is on, and its first parent.
fn pane_commit(state: &AppState) -> (Oid, Oid) {
    let row = state
        .history
        .as_ref()
        .and_then(|history| history.selected())
        .expect("a selected row");
    (row.commit.id, row.commit.parents[0])
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
            submodule: false,
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

#[test]
fn b_while_the_commit_graph_offer_is_up_goes_to_the_offer() {
    let mut state = pane_at(22);
    state.commit_graph = crate::update::CommitGraphOffer::Offered;
    state.system_git = Some(crate::update::GitCheck::Ran(
        graft_git::version::SystemGit::Supported(graft_git::version::GitVersion::MINIMUM),
    ));
    let (next, commands) = press(&state, KeyCode::Char('B'));
    assert!(!next.trail.is_live(), "no place opened");
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::WriteCommitGraph)),
        "{commands:?}"
    );
    assert!(!commands.iter().any(|c| matches!(c, Command::ReadBlame(..))));
}

// ---- Routing ----

#[test]
fn slash_in_blame_finds_in_the_file_not_the_graph() {
    let state = blamed_at(22);
    let typed = press_all(
        &state,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('r'),
            KeyCode::Char('e'),
            KeyCode::Enter,
        ],
    );
    assert_eq!(
        typed.trail.on_screen().and_then(|blame| blame.find.clone()),
        Some("re".to_owned())
    );
    assert!(typed.search.is_none(), "the graph search never opened");
}

#[test]
fn graph_verbs_are_inert_in_blame() {
    let state = blamed_at(22);
    for verb in ['m', 'r', 'a', 'c', 'o', 'u'] {
        let (next, commands) = press(&state, KeyCode::Char(verb));
        assert_eq!(next.trail, state.trail, "`{verb}` moved the place");
        assert!(
            next.preview.is_none() && next.commit_editor.is_none(),
            "`{verb}` reached the graph"
        );
        assert!(
            !commands.iter().any(is_write),
            "`{verb}` launched a write: {commands:?}"
        );
    }
}

#[test]
fn l_and_w_in_blame_never_reach_the_diff_pane() {
    let state = blamed_at(22);
    for code in [KeyCode::Char('l'), KeyCode::Char('w')] {
        let (next, _) = press(&state, code);
        assert_eq!(next.detail, state.detail, "{code:?} reached the pane");
    }
}

#[test]
fn esc_in_blame_never_cancels_a_running_write() {
    let mut state = blamed_at(22);
    state.status = Status::Busy("merging".to_owned());
    let (next, commands) = press(&state, KeyCode::Escape);
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::CancelJob(_))),
        "a ready place has no job to stop, and the write is not the place's: {commands:?}"
    );
    assert_eq!(next.status, state.status, "the write is still running");
    assert!(!next.trail.is_live(), "Esc went back along the trail");
}

// ---- Navigation ----

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

/// A place over `state` reading `blame`, the cursor on row `cursor`.
fn place_with(state: &mut AppState, blame: Blame, cursor: usize) {
    let mut place = ready_place(blame);
    place.cursor = cursor;
    trail_over(state, place, true);
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
            submodule: false,
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
    // before the search's), the trail waiting under the graph. The pane is
    // closed: with it open, the pane's own `Esc` (close it) comes first.
    let mut landed = pane_at(22);
    hide_a_trail(&mut landed);
    landed.detail.focus = DetailFocus::Graph;
    landed.detail.shown = false;
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
    let launches = [
        Command::Operate(
            job,
            Box::new(graft_ops::operation::Operation::Stage(
                graft_ops::operation::Target::Paths(vec!["a.rs".into()]),
            )),
        ),
        Command::Resume(job, graft_ops::apply::Resume::Continue),
        Command::RevertTo(
            job,
            graft_core::journal::EntryId::new(1_000, [0; 10]),
            false,
        ),
        Command::Undo(job, false),
        Command::Redo(job, false),
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

// ---- A pinned compare base (20's pinned-base ruling) ----

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
            submodule: false,
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

// ---- Keys ----

#[test]
fn find_and_n_move_between_matches() {
    let mut state = blamed_at(22);
    if let Some(blame) = state.trail.top_mut() {
        blame.cursor = 0;
    }
    let found = press_all(
        &state,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('r'),
            KeyCode::Char('e'),
            KeyCode::Char('t'),
            KeyCode::Enter,
        ],
    );
    assert_eq!(cursor_line(&found), 22);
    let next = press(&found, KeyCode::Char('n')).0;
    assert_eq!(cursor_line(&next), 27);
    let wrapped = press(&next, KeyCode::Char('n')).0;
    assert_eq!(cursor_line(&wrapped), 22);
    let back = press(&wrapped, KeyCode::Char('N')).0;
    assert_eq!(cursor_line(&back), 27);
}

#[test]
fn n_without_a_find_does_nothing() {
    let state = blamed_at(22);
    assert_eq!(
        press(&state, KeyCode::Char('n')),
        press(&state, KeyCode::Function(12))
    );
}

#[test]
fn esc_clears_the_find_before_popping() {
    let state = place_on_screen(pane_at(22), true, false);
    let (cleared, _) = press(&state, KeyCode::Escape);
    assert!(cleared.trail.is_live(), "still in blame");
    assert_eq!(
        cleared
            .trail
            .on_screen()
            .and_then(|blame| blame.find.clone()),
        None
    );
}

#[test]
fn i_toggles_ignore_revs_and_the_line_says_which() {
    let state = place_on_screen(pane_at(22), false, true);
    let (next, commands) = press(&state, KeyCode::Char('i'));
    let (_, request) = blame_job(&commands);
    assert!(!request.options.ignore_revs);
    assert!(!next.trail.top().expect("the place").options.ignore_revs);
}

#[test]
fn i_is_unbound_without_the_file() {
    let state = place_on_screen(pane_at(22), false, false);
    // Measured against a key nothing binds: `update` does ambient work on
    // every event (the evil-merge probe).
    assert_eq!(
        press(&state, KeyCode::Char('i')),
        press(&state, KeyCode::Function(12))
    );
}

#[test]
fn w_toggles_whitespace() {
    let state = blamed_at(22);
    let (next, commands) = press(&state, KeyCode::Char('w'));
    let (_, request) = blame_job(&commands);
    assert!(!request.options.ignore_whitespace);
    assert!(matches!(
        next.trail.top().expect("the place").body,
        BlameLoad::Reading {
            want: Want::Line(22),
            ..
        }
    ));
}

// ---- States ----

#[test]
fn blame_reading_title_then_lines() {
    let state = pane_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    assert!(matches!(
        opened.trail.top().expect("the place").body,
        BlameLoad::Reading { .. }
    ));
    let (job, _) = blame_job(&commands);
    let (read, _) = answer(
        &opened,
        job,
        Ok(BlameAnswer {
            blame: a_blame(false, false),
            line: None,
        }),
    );
    assert_eq!(read.trail.top().expect("the place").row_count(), 30);
}

#[test]
fn esc_while_reading_cancels_the_job() {
    let state = pane_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, _) = blame_job(&commands);
    let (back, commands) = press(&opened, KeyCode::Escape);
    assert!(!back.trail.is_live());
    assert!(commands.contains(&Command::CancelJob(job)), "{commands:?}");
}

#[test]
fn blame_failed_shows_gits_words() {
    let state = pane_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, _) = blame_job(&commands);
    let (failed, _) = answer(
        &opened,
        job,
        Err(BlameFailure::Failed("fatal: no such path".to_owned())),
    );
    assert_eq!(
        failed.trail.top().expect("the place").body,
        BlameLoad::Failed("fatal: no such path".to_owned())
    );
}

// ---- is_write ----

#[test]
fn a_blame_read_is_not_a_write() {
    let (_, commands) = press(&pane_at(22), KeyCode::Char('B'));
    assert!(!commands.iter().any(is_write), "{commands:?}");
}

fn click(row: u16) -> AppEvent {
    use graft_core::event::{Event, Modifiers, MouseButton, MouseInput, MouseKind};
    AppEvent::Core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 60,
        row,
        modifiers: Modifiers::NONE,
    }))
}

#[test]
fn mouse_double_click_is_enter() {
    let mut state = blamed_at(22);
    state.harness.clock = crate::session::Clock::Fixed(1_000);
    // Row 0 is the title; screen row 5 is the fifth line (top 0).
    let (once, _) = update(&state, click(5));
    assert_eq!(cursor_line(&once), 5);
    assert!(once.trail.on_screen().is_some(), "one click only selects");
    let (twice, _) = update(&once, click(5));
    assert!(twice.trail.hidden(), "a double-click is `⏎`");
    assert_eq!(
        twice.line_landing.as_ref().map(|landing| landing.spec),
        Some(DetailSpec::Commit(mo()))
    );
}

#[test]
fn a_click_in_blame_never_reaches_the_graph() {
    let state = blamed_at(22);
    let cursor = |state: &AppState| state.history.as_ref().map(|history| history.cursor);
    for row in [0, 1, 5, 20, 39] {
        let (next, _) = update(&state, click(row));
        assert_eq!(cursor(&next), cursor(&state), "row {row} moved the graph");
        assert_eq!(next.detail, state.detail, "row {row} reached the pane");
    }
}

// ---- Hints ----

/// The keys the footer draws for `state`, at 200 columns.
fn bar(state: &AppState) -> Vec<(&'static str, &'static str)> {
    use graft_ui::graph_screen::{Divider, hints_for};
    let banner = crate::update::banner::banner_view(state);
    let context = crate::view::rows::footer_context(state, &[], banner.as_ref());
    hints_for(&context, Divider::Hidden, 200)
        .into_iter()
        .map(|hint| (hint.key, hint.verb))
        .collect()
}

#[test]
fn pane_bar_offers_b_blame() {
    assert!(
        bar(&pane_at(22)).contains(&("B", "blame")),
        "{:?}",
        bar(&pane_at(22))
    );
}

#[test]
fn pane_bar_hides_b_while_the_offer_is_up() {
    let mut state = pane_at(22);
    state.commit_graph = crate::update::CommitGraphOffer::Offered;
    state.system_git = Some(crate::update::GitCheck::Ran(
        graft_git::version::SystemGit::Supported(graft_git::version::GitVersion::MINIMUM),
    ));
    assert!(state.commit_graph_actionable(), "the offer is up");
    assert!(!bar(&state).contains(&("B", "blame")), "{:?}", bar(&state));
}

#[test]
fn graph_bar_leads_esc_back_to_blame_while_hidden() {
    let state = blamed_at(22);
    let (mut landed, _) = press(&state, KeyCode::Enter);
    landed.detail.focus = DetailFocus::Graph;
    assert!(landed.trail.hidden());
    assert_eq!(
        bar(&landed).first(),
        Some(&("esc", "back to blame")),
        "{:?}",
        bar(&landed)
    );
}

#[test]
fn the_place_bar_offers_n_only_while_finding() {
    let finding = place_on_screen(pane_at(22), true, false);
    let not = place_on_screen(pane_at(22), false, false);
    assert!(bar(&finding).contains(&("n N", "match")));
    assert!(!bar(&not).contains(&("n N", "match")));
    assert!(!bar(&not).contains(&("i", "ignore list")));
}

#[test]
fn a_live_global_passes_through_the_place() {
    assert!(live_global(KeyPress::plain(KeyCode::Char('q'))));
    assert!(!live_global(KeyPress::plain(KeyCode::Char('m'))));
}
