//! The line cursor in every diff, and `diff_cursor` (m7-power-tools/20b).

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::oid::Oid;
use graft_git::diff::{FileEntry, FileList, FileStatus};
use graft_term::sanitize::sanitize_line;
use graft_ui::detail::{
    DetailFocus, DiffMode, DiffView, FileListView, FileRow, FileRowKind, FileStatusView, HunkView,
    LineKindView, LineView, WorkingTreeSection,
};

use crate::update::events::Command;
use crate::update::state::{AppState, Size};

use super::super::keys::detail_key;
use super::super::state::{DetailSpec, DetailState, Loading};
use super::super::tests::{empty_diff_view, pane_on};
use super::{DiffCursor, LineRef, Side, diff_cursor};

pub(crate) fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn line(
    kind: LineKindView,
    old_no: Option<u32>,
    new_no: Option<u32>,
    pair: Option<u32>,
) -> LineView {
    LineView {
        kind,
        old_no,
        new_no,
        text: sanitize_line("x"),
        words: Vec::new(),
        syntax: Vec::new(),
        pair,
    }
}

fn heading(text: &str) -> LineView {
    LineView {
        text: sanitize_line(text),
        ..line(LineKindView::HunkHeading, None, None, None)
    }
}

/// Two hunks. Body rows:
/// `0` heading · `1` ctx 1/1 · `2` − old 2 · `3` + new 2 · `4` + new 3 ·
/// `5` ctx 3/4 · `6` heading · `7` ctx 10/11 · `8` − old 11 · `9` ctx 12/12.
pub(crate) fn sample_diff() -> DiffView {
    use LineKindView::{Added, Context, Removed};
    DiffView {
        hunks: vec![
            HunkView {
                heading: sanitize_line("@@ -1,3 +1,4 @@"),
                lines: vec![
                    heading("@@ -1,3 +1,4 @@"),
                    line(Context, Some(1), Some(1), None),
                    line(Removed, Some(2), None, Some(3)),
                    line(Added, None, Some(2), Some(2)),
                    line(Added, None, Some(3), None),
                    line(Context, Some(3), Some(4), None),
                ],
            },
            HunkView {
                heading: sanitize_line("@@ -10,3 +11,2 @@"),
                lines: vec![
                    heading("@@ -10,3 +11,2 @@"),
                    line(Context, Some(10), Some(11), None),
                    line(Removed, Some(11), None, None),
                    line(Context, Some(12), Some(12), None),
                ],
            },
        ],
        ..empty_diff_view()
    }
}

/// One hunk of `count` context lines, numbered from 1 on both sides.
pub(crate) fn long_diff(count: u32) -> DiffView {
    let mut lines = vec![heading("@@ -1 +1 @@")];
    lines.extend((1..=count).map(|n| line(LineKindView::Context, Some(n), Some(n), None)));
    DiffView {
        hunks: vec![HunkView {
            heading: sanitize_line("@@ -1 +1 @@"),
            lines,
        }],
        ..empty_diff_view()
    }
}

/// A file list of `paths`, one row each, no sections; `old` renames the
/// first from it, `binary` drops the first's counts.
pub(crate) fn files_of(
    paths: &[&str],
    old: Option<&str>,
    binary: bool,
) -> (FileListView, FileList) {
    let mut rows = Vec::new();
    let mut files = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let counts = if binary && index == 0 { None } else { Some(1) };
        rows.push(FileRow {
            depth: 0,
            name: sanitize_line(path),
            kind: FileRowKind::File {
                status: FileStatusView::Modified,
                added: counts,
                removed: counts,
            },
            entry: Some(u32::try_from(index).expect("short")),
        });
        files.push(FileEntry {
            path: path.as_bytes().to_vec(),
            old_path: (index == 0)
                .then_some(old)
                .flatten()
                .map(|old| old.as_bytes().to_vec()),
            status: FileStatus::Modified,
            added: counts,
            removed: counts,
        });
    }
    let view = FileListView {
        rows,
        cursor: 0,
        top: 0,
        files: u32::try_from(paths.len()).expect("short"),
        added: 1,
        removed: 1,
        message: Vec::new(),
        compare: None,
    };
    (
        view,
        FileList {
            files,
            added: 1,
            removed: 1,
        },
    )
}

/// The pane open on `spec`, `src/x.rs` selected, `diff` arrived, the diff
/// focused, the screen 120x40.
pub(crate) fn pane_with(spec: DetailSpec, diff: DiffView) -> AppState {
    let (view, list) = files_of(&["src/x.rs"], None, false);
    AppState {
        size: Size {
            columns: 120,
            rows: 40,
        },
        detail: DetailState {
            shown: true,
            spec: Some(spec),
            focus: DetailFocus::Diff,
            files: Loading::Ready(Arc::new(view)),
            list: Some(Arc::new(list)),
            diff: Loading::Ready(Arc::new(diff)),
            ..DetailState::default()
        },
        ..AppState::default()
    }
}

pub(crate) fn raw_diff() -> Arc<graft_git::diff::FileDiff> {
    Arc::new(graft_git::diff::FileDiff {
        entry: FileEntry {
            path: b"src/x.rs".to_vec(),
            old_path: None,
            status: FileStatus::Modified,
            added: Some(1),
            removed: Some(1),
        },
        hunks: Vec::new(),
        binary: None,
        truncated: None,
        old_mode: None,
        new_mode: None,
    })
}

fn press(state: &mut AppState, code: KeyCode) -> Vec<Command> {
    let mut commands = Vec::new();
    assert!(detail_key(state, &mut commands, KeyPress::plain(code)));
    commands
}

fn at(side: Side, line: u32) -> LineRef {
    LineRef { side, line }
}

// ---- the cursor --------------------------------------------------------

#[test]
fn commit_diff_j_moves_the_line_cursor() {
    let mut state = pane_with(DetailSpec::Commit(oid(1)), sample_diff());
    for _ in 0..3 {
        press(&mut state, KeyCode::Char('j'));
    }
    assert_eq!(state.detail.body_cursor(), Some(3));
    assert_eq!(
        state.detail.body_top, 0,
        "the cursor moved, the view did not"
    );
    press(&mut state, KeyCode::Char('k'));
    assert_eq!(state.detail.body_cursor(), Some(2));
}

#[test]
fn commit_diff_view_scrolls_only_when_the_cursor_leaves_it() {
    let mut state = pane_with(DetailSpec::Commit(oid(1)), long_diff(200));
    let rows = crate::update::geometry::diff_body_rows(&state);
    assert!(rows > 3, "a 120x40 screen shows a body: {rows}");
    for _ in 0..rows - 1 {
        press(&mut state, KeyCode::Char('j'));
    }
    assert_eq!(state.detail.body_cursor(), Some(rows - 1));
    assert_eq!(state.detail.body_top, 0, "still on screen: no scroll");
    press(&mut state, KeyCode::Down);
    assert_eq!(
        state.detail.body_top, 1,
        "one row past the bottom: one row of scroll"
    );
    press(&mut state, KeyCode::Char('G'));
    assert_eq!(state.detail.body_cursor(), Some(200));
    press(&mut state, KeyCode::Char('g'));
    assert_eq!(
        (state.detail.body_cursor(), state.detail.body_top),
        (Some(0), 0)
    );
    press(&mut state, KeyCode::PageDown);
    assert_eq!(state.detail.body_cursor(), Some(10));
}

#[test]
fn compare_diff_has_the_line_cursor() {
    let spec = DetailSpec::Compare {
        base: oid(1),
        head: oid(2),
    };
    let mut state = pane_with(spec, sample_diff());
    press(&mut state, KeyCode::Char('j'));
    press(&mut state, KeyCode::Up);
    press(&mut state, KeyCode::Down);
    assert_eq!(state.detail.body_cursor(), Some(1));
    let view = crate::view::detail_view(&state);
    assert_eq!(view.body_cursor, Some(1), "the view draws it");
}

#[test]
fn load_whole_file_keeps_the_commit_diff_cursor_line() {
    let mut state = pane_with(DetailSpec::Commit(oid(1)), sample_diff());
    for _ in 0..8 {
        press(&mut state, KeyCode::Char('j'));
    }
    assert_eq!(
        diff_cursor(&state).and_then(|c| c.at),
        Some(at(Side::Old, 11))
    );
    let commands = press(&mut state, KeyCode::Char('L'));
    let Some(Command::ReadFileDiff(id, key, _)) = commands
        .iter()
        .find(|command| matches!(command, Command::ReadFileDiff(..)))
    else {
        panic!("L reads the file again: {commands:?}");
    };
    // The whole file: one more hunk above the first, so every row moves.
    let mut whole = sample_diff();
    whole.hunks.insert(0, long_diff(0).hunks.remove(0));
    let raw = raw_diff();
    state
        .detail
        .diff_arrived(*id, *key, Ok((Arc::new(whole), raw)));
    assert_eq!(state.detail.body_cursor(), Some(9));
    assert_eq!(
        diff_cursor(&state).and_then(|c| c.at),
        Some(at(Side::Old, 11))
    );
}

#[test]
fn w_keeps_the_cursor_on_its_line_from_the_cache() {
    let mut state = pane_with(DetailSpec::Commit(oid(1)), sample_diff());
    for _ in 0..3 {
        press(&mut state, KeyCode::Char('j'));
    }
    let commands = press(&mut state, KeyCode::Char('w'));
    let Some(Command::ReadFileDiff(id, key, _)) = commands.first() else {
        panic!("w reads the file again: {commands:?}");
    };
    // Whitespace ignored: the first context line goes, every row moves up.
    let mut fewer = sample_diff();
    fewer.hunks[0].lines.remove(1);
    let (id, key) = (*id, *key);
    state
        .detail
        .diff_arrived(id, key, Ok((Arc::new(fewer), raw_diff())));
    assert_eq!(state.detail.body_cursor(), Some(2));
    assert_eq!(
        diff_cursor(&state).and_then(|c| c.at),
        Some(at(Side::New, 2))
    );
    // Back, read again, then forward once more: that read is cached, and
    // the cursor keeps its line without waiting.
    let commands = press(&mut state, KeyCode::Char('w'));
    let Some(Command::ReadFileDiff(id, key, _)) = commands.first() else {
        panic!("w reads the file again: {commands:?}");
    };
    let (id, key) = (*id, *key);
    state
        .detail
        .diff_arrived(id, key, Ok((Arc::new(sample_diff()), raw_diff())));
    assert_eq!(state.detail.body_cursor(), Some(3));
    let commands = press(&mut state, KeyCode::Char('w'));
    assert!(commands.is_empty(), "a cache hit: {commands:?}");
    assert_eq!(state.detail.body_cursor(), Some(2));
    assert_eq!(
        diff_cursor(&state).and_then(|c| c.at),
        Some(at(Side::New, 2))
    );
}

#[test]
fn changes_pane_cursor_is_unchanged() {
    let mut detail = pane_on(&[(WorkingTreeSection::Unstaged, &["a.rs"])], 1, 0);
    detail.focus = DetailFocus::Diff;
    detail.diff = Loading::Ready(Arc::new(sample_diff()));
    let mut state = AppState {
        size: Size {
            columns: 120,
            rows: 40,
        },
        detail,
        ..AppState::default()
    };
    press(&mut state, KeyCode::Char('j'));
    press(&mut state, KeyCode::Char('j'));
    assert_eq!(state.detail.body_cursor(), Some(2));
    assert_eq!(state.detail.body_top, 0);
}

// ---- diff_cursor -------------------------------------------------------

fn cursor_on(row: usize) -> Option<DiffCursor> {
    let mut state = pane_with(DetailSpec::Commit(oid(1)), sample_diff());
    state.detail.body_cursor = row;
    diff_cursor(&state)
}

#[test]
fn diff_cursor_added_line_is_new_side() {
    let cursor = cursor_on(4).expect("a cursor");
    assert_eq!(cursor.at, Some(at(Side::New, 3)));
    assert_eq!(cursor.path, PathBuf::from("src/x.rs"));
    assert_eq!(cursor.spec, DetailSpec::Commit(oid(1)));
    assert_eq!(cursor.section, None);
    assert!(!cursor.binary);
    assert_eq!(
        cursor_on(1).and_then(|c| c.at),
        Some(at(Side::New, 1)),
        "context: new side"
    );
}

#[test]
fn diff_cursor_removed_line_is_old_side_with_the_old_path() {
    let mut state = pane_with(DetailSpec::Commit(oid(1)), sample_diff());
    let (view, list) = files_of(&["src/x.rs"], Some("src/old.rs"), false);
    state.detail.files = Loading::Ready(Arc::new(view));
    state.detail.list = Some(Arc::new(list));
    state.detail.body_cursor = 8;
    let cursor = diff_cursor(&state).expect("a cursor");
    assert_eq!(cursor.at, Some(at(Side::Old, 11)));
    assert_eq!(cursor.old_path, Some(PathBuf::from("src/old.rs")));
    assert_eq!(cursor.path, PathBuf::from("src/x.rs"));
}

#[test]
fn diff_cursor_side_by_side_removed_row_with_a_pair_is_new_side() {
    let mut state = pane_with(DetailSpec::Commit(oid(1)), sample_diff());
    state.detail.mode = DiffMode::SideBySide;
    state.detail.body_cursor = 2;
    assert_eq!(
        diff_cursor(&state).and_then(|c| c.at),
        Some(at(Side::New, 2))
    );
    state.detail.body_cursor = 8;
    assert_eq!(
        diff_cursor(&state).and_then(|c| c.at),
        Some(at(Side::Old, 11)),
        "a removed-only row has no new side"
    );
}

#[test]
fn diff_cursor_heading_is_the_hunks_first_changed_line() {
    assert_eq!(cursor_on(0).and_then(|c| c.at), Some(at(Side::New, 2)));
    assert_eq!(
        cursor_on(6).and_then(|c| c.at),
        Some(at(Side::Old, 11)),
        "no added line: the first removed one"
    );
}

#[test]
fn diff_cursor_file_row_is_the_files_first_changed_line() {
    let mut state = pane_with(DetailSpec::Commit(oid(1)), sample_diff());
    state.detail.focus = DetailFocus::Files;
    state.detail.body_cursor = 8;
    assert_eq!(
        diff_cursor(&state).and_then(|c| c.at),
        Some(at(Side::New, 2))
    );
}

#[test]
fn diff_cursor_file_row_before_the_diff_arrives_is_none() {
    let mut state = pane_with(DetailSpec::Commit(oid(1)), sample_diff());
    state.detail.focus = DetailFocus::Files;
    state.detail.diff = Loading::Pending(graft_core::job::JobId::new(7));
    let cursor = diff_cursor(&state).expect("a file row still names its file");
    assert_eq!(cursor.at, None);
    assert_eq!(cursor.path, PathBuf::from("src/x.rs"));
}

#[test]
fn diff_cursor_binary_file_says_binary() {
    let mut state = pane_with(DetailSpec::Commit(oid(1)), sample_diff());
    let (view, list) = files_of(&["logo.png"], None, true);
    state.detail.files = Loading::Ready(Arc::new(view));
    state.detail.list = Some(Arc::new(list));
    state.detail.diff = Loading::Pending(graft_core::job::JobId::new(7));
    assert!(
        diff_cursor(&state).is_some_and(|c| c.binary),
        "from the counts"
    );
    state.detail.diff = Loading::Ready(Arc::new(DiffView {
        binary: Some(sanitize_line("Binary files differ")),
        ..empty_diff_view()
    }));
    let cursor = diff_cursor(&state).expect("a cursor");
    assert!(cursor.binary, "from the diff");
    assert_eq!(cursor.at, None);
}

#[test]
fn diff_cursor_untracked_row_has_its_section() {
    let mut detail = pane_on(&[(WorkingTreeSection::Untracked, &["new.rs"])], 1, 0);
    detail.focus = DetailFocus::Files;
    let state = AppState {
        detail,
        ..AppState::default()
    };
    let cursor = diff_cursor(&state).expect("a cursor");
    assert_eq!(cursor.section, Some(WorkingTreeSection::Untracked));
    assert_eq!(cursor.spec, DetailSpec::WorkingTree);
    assert_eq!(cursor.path, PathBuf::from("new.rs"));
}

#[test]
fn diff_cursor_is_none_with_the_graph_focused() {
    let mut state = pane_with(DetailSpec::Commit(oid(1)), sample_diff());
    state.detail.focus = DetailFocus::Graph;
    assert_eq!(diff_cursor(&state), None);
    state.detail.focus = DetailFocus::Diff;
    state.detail.shown = false;
    assert_eq!(diff_cursor(&state), None, "a hidden pane has no cursor");
}
