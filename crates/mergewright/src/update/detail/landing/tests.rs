//! `show_line` and its pending landing (m7-power-tools/20b), driven through
//! `update` as the app drives it: the seek, the file list and the diff each
//! arrive as their job's answer.

use std::path::Path;
use std::sync::Arc;

use mergewright_core::event::KeyCode;
use mergewright_core::job::JobId;
use mergewright_core::oid::Oid;
use mergewright_git::status::{Change, StatusEntry, WorkingTreeStatus};
use mergewright_ui::detail::DetailFocus;

use crate::history::{HistoryUpdate, Wanted};
use crate::update::events::{AppEvent, Command, Product, Progress};
use crate::update::state::{AppState, Status};
use crate::update::tests::{awaiting, key, rows_for, sized};
use crate::update::update;

use super::super::cursor::tests::{files_of, long_diff, raw_diff, sample_diff};
use super::super::state::{DetailSpec, Loading};
use super::super::{LineRef, Side, diff_cursor};
use super::{LandingStage, show_line};

mod disk;
mod reveal;
mod select;

const WALK: JobId = JobId::new(4);

/// Commit `i + 1` on row `i`.
fn commit(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

/// Rows `span` of the walk, answering request `request`, commit `i + 1` on
/// row `i`.
fn window(request: u64, span: std::ops::Range<usize>) -> AppEvent {
    let AppEvent::Progress(id, Progress::History(HistoryUpdate::Window(mut window))) =
        rows_for(WALK, request, span.clone(), 500)
    else {
        unreachable!("`rows_for` answers with a window");
    };
    window.rows = span
        .map(|at| {
            let mut row = window.rows[0].clone();
            row.commit.id = commit(u8::try_from(at + 1).expect("under 256 rows"));
            row
        })
        .collect();
    AppEvent::Progress(id, Progress::History(HistoryUpdate::Window(window)))
}

/// A 120x40 screen, rows 0..30 of 500 on hand, the cursor on row 0.
fn on_graph() -> AppState {
    let (state, _) = update(&awaiting(WALK), sized(120, 40));
    let (state, _) = update(&state, window(2, 0..30));
    state
}

fn step(state: &AppState, event: AppEvent) -> (AppState, Vec<Command>) {
    update(state, event)
}

fn list_job(commands: &[Command]) -> JobId {
    commands
        .iter()
        .find_map(|command| match command {
            Command::ListFiles(id, ..) => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the pane asks for its files: {commands:?}"))
}

/// The file list `paths`, as the job answers it.
fn files(id: JobId, paths: &[&str]) -> AppEvent {
    let (view, list) = files_of(paths, None, false);
    AppEvent::Produced(id, Product::Files(Ok((Arc::new(view), Arc::new(list)))))
}

/// The last diff read in `commands` (any before it were cancelled),
/// answered with `view`.
fn diff_answer(commands: &[Command], view: mergewright_ui::detail::DiffView) -> AppEvent {
    let (id, key) = commands
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::ReadFileDiff(id, key, _) => Some((*id, *key)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the pane asks for a diff: {commands:?}"));
    AppEvent::Produced(id, Product::FileDiff(key, Ok((Arc::new(view), raw_diff()))))
}

fn new_line(line: u32) -> LineRef {
    LineRef {
        side: Side::New,
        line,
    }
}

fn selected(state: &AppState) -> Option<Oid> {
    state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .map(|row| row.commit.id)
}

/// `show_line` on row 200's commit, off the 30 rows on hand: the seek goes
/// to the walk. Returns the state and the walk's find request id.
fn landing_far(path: &str, at: Option<LineRef>) -> (AppState, u64) {
    let mut state = on_graph();
    let commands = show_line(
        &mut state,
        DetailSpec::Commit(commit(200)),
        Path::new(path),
        at,
        None,
    );
    let [Command::ShowRows(to, asked)] = commands.as_slice() else {
        panic!("one find request: {commands:?}");
    };
    assert_eq!(*to, WALK);
    assert!(matches!(asked.wanted, Wanted::Find { commit: c, .. } if c == commit(200)));
    (state, asked.id)
}

#[test]
fn show_line_lands_on_commit_file_and_line() {
    let (state, find) = landing_far("src/y.rs", Some(new_line(3)));
    assert_eq!(
        state.line_landing.as_ref().map(|landing| landing.stage),
        Some(LandingStage::Seeking(commit(200)))
    );
    assert!(
        !state.detail.shown,
        "nothing opens before the graph is there"
    );

    let (state, commands) = step(&state, window(find, 180..210));
    assert_eq!(selected(&state), Some(commit(200)), "the graph is on it");
    assert!(state.detail.shown);
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(commit(200))));

    let (state, commands) = step(
        &state,
        files(list_job(&commands), &["src/x.rs", "src/y.rs"]),
    );
    assert_eq!(
        state.detail.selected_entry(),
        Some(1),
        "src/y.rs is selected"
    );

    let (state, _) = step(&state, diff_answer(&commands, sample_diff()));
    assert_eq!(state.line_landing, None, "landed");
    assert_eq!(state.detail.body_cursor(), Some(4));
    let cursor = diff_cursor(&state).expect("the diff has the cursor");
    assert_eq!(cursor.at, Some(new_line(3)));
    assert_eq!(cursor.path, Path::new("src/y.rs"));
}

#[test]
fn show_line_puts_the_hunk_heading_on_top() {
    // The cursor's hunk fits under its heading: the heading tops the view.
    let mut state = on_graph();
    let commands = show_line(
        &mut state,
        DetailSpec::Commit(commit(5)),
        Path::new("src/x.rs"),
        Some(LineRef {
            side: Side::Old,
            line: 11,
        }),
        None,
    );
    let (state, commands) = step(&state, files(list_job(&commands), &["src/x.rs"]));
    let (state, _) = step(&state, diff_answer(&commands, sample_diff()));
    assert_eq!(state.detail.body_cursor(), Some(8));
    assert_eq!(state.detail.body_top, 6, "the second hunk's heading");

    // A line too far below its heading to fit: a third of the way down.
    let mut state = on_graph();
    let commands = show_line(
        &mut state,
        DetailSpec::Commit(commit(5)),
        Path::new("src/x.rs"),
        Some(new_line(150)),
        None,
    );
    let (state, commands) = step(&state, files(list_job(&commands), &["src/x.rs"]));
    let (state, _) = step(&state, diff_answer(&commands, long_diff(200)));
    let rows = crate::update::geometry::diff_body_rows(&state);
    assert_eq!(state.detail.body_cursor(), Some(150));
    assert_eq!(state.detail.body_top, 150 - rows / 3);
}

#[test]
fn show_line_focuses_the_diff_body() {
    let mut state = on_graph();
    let commands = show_line(
        &mut state,
        DetailSpec::Commit(commit(3)),
        Path::new("src/x.rs"),
        Some(new_line(2)),
        None,
    );
    assert_eq!(
        selected(&state),
        Some(commit(3)),
        "on hand: the graph is there at once"
    );
    let (state, commands) = step(&state, files(list_job(&commands), &["src/x.rs"]));
    assert_eq!(
        state.detail.focus,
        DetailFocus::Files,
        "the diff has not arrived"
    );
    let (state, _) = step(&state, diff_answer(&commands, sample_diff()));
    assert_eq!(state.detail.focus, DetailFocus::Diff);
    assert_eq!(state.detail.body_cursor(), Some(3));
}

#[test]
fn show_line_working_tree_lands_on_the_uncommitted_row() {
    let mut state = on_graph();
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: vec![StatusEntry {
            path: "src/x.rs".into(),
            original_path: None,
            staged: None,
            unstaged: Some(Change::Modified),
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }],
    }));
    let commands = show_line(
        &mut state,
        DetailSpec::WorkingTree,
        Path::new("src/x.rs"),
        Some(new_line(2)),
        None,
    );
    assert!(
        state.on_working_tree,
        "the uncommitted-changes row is selected"
    );
    assert_eq!(state.detail.spec, Some(DetailSpec::WorkingTree));
    let (state, commands) = step(&state, files(list_job(&commands), &["src/x.rs"]));
    let (state, _) = step(&state, diff_answer(&commands, sample_diff()));
    assert!(state.on_working_tree, "still on it after the reads");
    assert_eq!(state.detail.spec, Some(DetailSpec::WorkingTree));
    assert_eq!(state.detail.body_cursor(), Some(3));
    assert_eq!(state.line_landing, None);
}

#[test]
fn show_line_path_missing_says_so() {
    let mut state = on_graph();
    let commands = show_line(
        &mut state,
        DetailSpec::Commit(commit(3)),
        Path::new("src/gone.rs"),
        Some(new_line(2)),
        None,
    );
    let (state, _) = step(&state, files(list_job(&commands), &["src/x.rs"]));
    assert_eq!(state.line_landing, None);
    assert_eq!(
        state.status,
        Status::sticky("src/gone.rs isn't among 0303030's files")
    );
    assert!(state.detail.shown, "the pane stays open on the spec");
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(commit(3))));
    assert_eq!(
        state.detail.selected_entry(),
        Some(0),
        "as it opens by default"
    );
}

#[test]
fn show_line_unreachable_commit_says_so_and_moves_nothing() {
    let (state, find) = landing_far("src/x.rs", Some(new_line(2)));
    // The walk answers the find without the commit: not in this view.
    let (state, _) = step(&state, window(find, 0..30));
    assert_eq!(selected(&state), Some(commit(1)), "the cursor kept its row");
    assert!(!state.detail.shown, "the pane did not open");
    assert_eq!(state.line_landing, None);
    assert_eq!(
        state.status,
        Status::sticky("can't show c8c8c8c on the graph: not in this view")
    );
}

#[test]
fn a_key_during_a_landing_drops_it() {
    let (state, find) = landing_far("src/x.rs", Some(new_line(2)));
    let (state, _) = step(&state, key(KeyCode::Char('j')));
    assert_eq!(state.line_landing, None, "the graph cursor moved");
    let (state, _) = step(&state, window(find, 180..210));
    assert!(!state.detail.shown, "a late answer lands nothing");

    // A key that moves nothing leaves the landing alone. `F20`, not `?`:
    // `?` opens the palette since m8-polish-and-ship/11.
    let (state, _) = landing_far("src/x.rs", Some(new_line(2)));
    let (state, _) = step(&state, key(KeyCode::Function(20)));
    assert!(state.line_landing.is_some());
}

#[test]
fn a_second_show_line_replaces_the_first() {
    let (mut state, _) = landing_far("src/x.rs", Some(new_line(2)));
    let commands = show_line(
        &mut state,
        DetailSpec::Commit(commit(3)),
        Path::new("src/x.rs"),
        Some(new_line(3)),
        None,
    );
    let landing = state.line_landing.clone().expect("the second is pending");
    assert_eq!(landing.spec, DetailSpec::Commit(commit(3)));
    assert_eq!(landing.stage, LandingStage::Selecting);
    let (state, commands) = step(&state, files(list_job(&commands), &["src/x.rs"]));
    let (state, _) = step(&state, diff_answer(&commands, sample_diff()));
    assert_eq!(selected(&state), Some(commit(3)));
    assert_eq!(state.detail.body_cursor(), Some(4));
    assert!(matches!(state.detail.diff, Loading::Ready(_)));
}

/// 20's pinned-base ruling: a `Commit(c)` landing while a base is pinned
/// used to stop silently at `Selecting` (the pane became a compare). Now it
/// says why and leaves the pane, the cursor and the pin as they were.
#[test]
fn show_line_commit_with_a_pinned_base_says_so() {
    let mut state = on_graph();
    state.compare_base = Some(super::super::state::CompareBase {
        oid: commit(9),
        row: 8,
    });
    let before = state.clone();
    let commands = show_line(
        &mut state,
        DetailSpec::Commit(commit(3)),
        Path::new("src/x.rs"),
        Some(new_line(3)),
        None,
    );
    assert!(commands.is_empty(), "nothing is asked for: {commands:?}");
    assert_eq!(state.line_landing, None);
    assert_eq!(
        selected(&state),
        selected(&before),
        "the cursor kept its row"
    );
    assert_eq!(state.detail.shown, before.detail.shown);
    assert_eq!(state.detail.spec, before.detail.spec);
    assert_eq!(state.compare_base, before.compare_base, "still pinned");
    assert_eq!(
        state.status,
        Status::sticky(
            "can't show 0303030 on its own while compare base 0909090 is pinned · x on the base clears it"
        )
    );
}

/// The base's own row shows the base's own diff (`follow_selection` leaves
/// it alone), so a landing on the base itself still lands.
#[test]
fn show_line_on_the_pinned_base_itself_lands() {
    let mut state = on_graph();
    state.compare_base = Some(super::super::state::CompareBase {
        oid: commit(3),
        row: 2,
    });
    let commands = show_line(
        &mut state,
        DetailSpec::Commit(commit(3)),
        Path::new("src/x.rs"),
        Some(new_line(2)),
        None,
    );
    let (state, commands) = step(&state, files(list_job(&commands), &["src/x.rs"]));
    let (state, _) = step(&state, diff_answer(&commands, sample_diff()));
    assert_eq!(state.line_landing, None, "landed");
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(commit(3))));
    assert_eq!(state.detail.body_cursor(), Some(3));
    assert_eq!(state.compare_base.map(|base| base.oid), Some(commit(3)));
}
