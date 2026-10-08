use std::sync::Arc;

use graft_core::job::JobId;
use graft_git::diff::{DiffLine, FileDiff, FileEntry, FileList, FileStatus, Hunk, LineKind};
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
use graft_ui::detail::{FileListView, FileRow, FileRowKind, FileStatusView};

use super::stage_discard::{NotStaged, untracked_size};
use super::*;
use crate::update::detail::DetailSpec;

/// An `Applied` naming `description`, with `refresh` as its scope. Only
/// those two fields are read by `operated`; the rest is a plausible
/// journal entry.
fn applied(description: &str, refresh: Refresh) -> graft_ops::apply::Applied {
    use graft_core::journal::{
        EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches,
    };
    graft_ops::apply::Applied {
        came_back: Vec::new(),
        stopped_at: None,
        entry: JournalEntry {
            id: EntryId::new(1, [0; 10]),
            at_unix_ms: 1,
            kind: OperationKind::Stage,
            description: description.to_owned(),
            status: EntryStatus::Committed,
            touches: Touches::default(),
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            head_after: None,
            refs_before: std::collections::BTreeMap::new(),
            refs_after: std::collections::BTreeMap::new(),
            index_before: None,
            index_after: None,
            worktree_before: None,
            worktree_after: None,
            worktree_raw_before: None,
            backup_ref: None,
            pins: Vec::new(),
            commands: Vec::new(),
            reverses: None,
            merge_before: None,
            merge_after: None,
            config_before: std::collections::BTreeMap::new(),
            config_after: std::collections::BTreeMap::new(),
            put_back: None,
        },
        refresh,
        step_backup: None,
        replaced: graft_ops::undo::Replaced::default(),
    }
}

/// A changes pane holding one section with one file row under it, the
/// cursor on the file row, and `entry` behind `AppState::working_tree` —
/// the shape `selected_paths`/`status_entry` read.
fn state_on(path: &str, entry: StatusEntry, section: WorkingTreeSection) -> AppState {
    // Row 0 is the section heading, row 1 the file.
    state_with(&[(path.to_string(), section)], vec![entry], 1)
}

/// A changes pane built row by row: `rows` is `(path, section)` in draw
/// order, a section heading inserted whenever the section changes.
/// `cursor` is the row the pane's cursor sits on.
pub(in crate::update) fn state_with(
    rows: &[(String, WorkingTreeSection)],
    entries: Vec<StatusEntry>,
    cursor: usize,
) -> AppState {
    let (files, view_rows) = sectioned_rows(rows);
    let count = u32::try_from(files.len()).expect("a test list is short");
    let list = FileList {
        files,
        added: 1,
        removed: 0,
    };
    let view = FileListView {
        rows: view_rows,
        cursor,
        top: 0,
        files: count,
        added: 1,
        removed: 0,
        message: Vec::new(),
        compare: None,
    };
    let mut state = AppState {
        working_tree: Some(Arc::new(WorkingTreeStatus { entries })),
        ..AppState::default()
    };
    state.detail.spec = Some(DetailSpec::WorkingTree);
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    state.detail.files = crate::update::detail::Loading::Ready(Arc::new(view));
    state.detail.list = Some(Arc::new(list));
    state
}

/// The git file list and the drawn rows for `rows`, a section heading
/// inserted wherever the section changes.
fn sectioned_rows(rows: &[(String, WorkingTreeSection)]) -> (Vec<FileEntry>, Vec<FileRow>) {
    let mut files = Vec::new();
    let mut view_rows = Vec::new();
    let mut current: Option<WorkingTreeSection> = None;
    for (path, section) in rows {
        if current != Some(*section) {
            view_rows.push(FileRow {
                depth: 0,
                name: graft_term::sanitize::sanitize_line(section.title()),
                kind: FileRowKind::Section {
                    section: *section,
                    count: rows.iter().filter(|(_, s)| s == section).count(),
                },
                entry: None,
            });
            current = Some(*section);
        }
        let index = u32::try_from(files.len()).expect("a test list is short");
        files.push(FileEntry {
            path: path.as_bytes().to_vec(),
            old_path: None,
            status: FileStatus::Modified,
            added: Some(1),
            removed: Some(0),
        });
        view_rows.push(FileRow {
            depth: 1,
            name: graft_term::sanitize::sanitize_line(path),
            kind: FileRowKind::File {
                status: FileStatusView::Modified,
                added: Some(1),
                removed: Some(0),
            },
            entry: Some(index),
        });
    }
    (files, view_rows)
}

pub(in crate::update) fn unstaged_entry(path: &str) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: Some(Change::Modified),
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    }
}

fn staged_entry(path: &str) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: Some(Change::Modified),
        unstaged: None,
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    }
}

fn untracked_entry(path: &str) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: true,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    }
}

fn ids() -> impl FnMut() -> JobId {
    let mut next = 1u64;
    move || {
        let id = JobId::new(next);
        next += 1;
        id
    }
}

fn line(kind: LineKind, text: &str, old_no: Option<u32>, new_no: Option<u32>) -> DiffLine {
    DiffLine {
        kind,
        text: text.as_bytes().to_vec(),
        old_no,
        new_no,
        no_newline_at_eof: false,
    }
}

/// `src/a.rs` with one hunk: a context line, a `-`/`+` pair, a context
/// line. Body rows: `0` the `@@` heading, `1` context, `2` the removed
/// line, `3` the added line, `4` context.
pub(in crate::update) fn one_pair_diff() -> FileDiff {
    FileDiff {
        entry: FileEntry {
            path: b"src/a.rs".to_vec(),
            old_path: None,
            status: FileStatus::Modified,
            added: Some(1),
            removed: Some(1),
        },
        hunks: vec![Hunk {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
            heading: Vec::new(),
            lines: vec![
                line(LineKind::Context, "one", Some(1), Some(1)),
                line(LineKind::Removed, "old", Some(2), None),
                line(LineKind::Added, "new", None, Some(2)),
                line(LineKind::Context, "two", Some(3), Some(3)),
            ],
        }],
        binary: None,
        truncated: None,
        old_mode: None,
        new_mode: None,
    }
}

/// `src/a.rs` whose only line changed and whose file ends without a
/// newline on both sides — the shape `hunk_patch` refuses a single-line
/// selection on.
fn no_trailing_newline_diff() -> FileDiff {
    let mut removed = line(LineKind::Removed, "old", Some(1), None);
    removed.no_newline_at_eof = true;
    let mut added = line(LineKind::Added, "new", None, Some(1));
    added.no_newline_at_eof = true;
    FileDiff {
        hunks: vec![Hunk {
            old_start: 1,
            old_lines: 1,
            new_start: 1,
            new_lines: 1,
            heading: Vec::new(),
            lines: vec![removed, added],
        }],
        ..one_pair_diff()
    }
}

/// The changes pane with the diff region focused, `diff` on show and the
/// line cursor on body row `cursor`.
pub(in crate::update) fn state_on_diff(
    section: WorkingTreeSection,
    diff: FileDiff,
    cursor: usize,
) -> AppState {
    let path = String::from_utf8(diff.entry.path.clone()).expect("a test path is utf-8");
    let entry = match section {
        WorkingTreeSection::Staged => staged_entry(&path),
        WorkingTreeSection::Unstaged => unstaged_entry(&path),
        WorkingTreeSection::Untracked => untracked_entry(&path),
        WorkingTreeSection::Unmerged => unmerged_entry(&path),
    };
    let mut state = state_on(&path, entry, section);
    state.detail.focus = DetailFocus::Diff;
    let view = crate::jobs::detail::assemble_diff(&diff, false);
    state.detail.diff = Loading::Ready(Arc::new(view));
    state.detail.diff_raw = Some(Arc::new(diff));
    state.detail.body_cursor = cursor;
    state
}

/// The `Target::Patch` the one queued command carries.
fn queued_patch(commands: &[Command]) -> (&PathBuf, &Vec<u8>, PatchSide) {
    let [Command::Operate(_, op, ..)] = commands else {
        panic!("expected exactly one Operate, got {commands:?}");
    };
    let (Operation::Stage(target) | Operation::Unstage(target) | Operation::Discard(target)) =
        &**op
    else {
        panic!("expected a stage/unstage/discard");
    };
    let Target::Patch { path, patch, side } = target else {
        panic!("expected a patch target, got {target:?}");
    };
    (path, patch, *side)
}

/// `Space` on a conflicted row **marks it resolved**: staging an
/// unmerged path is exactly how git records a resolution, and that is
/// the key the design note gives it ("`space` marks resolved",
/// m5-history-and-ghost-preview/09).
///
/// Group 04 left this refusing, because the operation that makes it safe
/// did not exist yet. It does now: `graft_ops`' `check` admits `Stage`
/// while an operation is in progress and refuses `MarkersRemain` while
/// `<<<<<<< ` is still in the file, so the refusal moved from "this key
/// is not for that" to "that file is not finished" — a fact about the
/// file rather than about the key.
#[test]
fn space_on_a_conflicted_file_stages_it_as_the_resolution() {
    let mut state = state_on(
        "src/auth.rs",
        unmerged_entry("src/auth.rs"),
        WorkingTreeSection::Unmerged,
    );
    let mut commands = Vec::new();
    assert!(
        stage_selected(&mut state, &mut commands, &mut ids()),
        "the key is answered, not ignored"
    );
    let [Command::Operate(_, op, ..)] = commands.as_slice() else {
        panic!("expected one staging operation: {commands:?}");
    };
    assert!(
        matches!(&**op, Operation::Stage(Target::Paths(paths))
            if paths == &[PathBuf::from("src/auth.rs")]),
        "{op:?}"
    );
    assert!(state.operating.is_some(), "the write is in flight");
}

/// `d` on a conflicted **file row** is refused too. A `UU` entry is both
/// staged and unstaged, so the discard box would otherwise offer "both
/// go back to HEAD" and `git restore --staged --worktree
/// --source=HEAD` — throwing away the resolution in progress. The key
/// is answered and says why (M5 group 04 review, Major 3).
#[test]
fn d_on_a_conflicted_file_row_says_why_instead_of_offering_a_reset_to_head() {
    let mut state = state_on(
        "src/auth.rs",
        unmerged_entry("src/auth.rs"),
        WorkingTreeSection::Unmerged,
    );
    assert!(
        request_discard(&mut state),
        "the key is answered, not ignored"
    );
    assert!(state.confirm.is_none(), "no discard box opened");
    let Status::Sticky { reason, .. } = &state.status else {
        panic!("expected a sticky reason, got {:?}", state.status);
    };
    assert!(reason.contains("conflicted file"), "{reason}");
}

/// The same for `d` on a hunk of a conflicted file: there is no clean
/// side to cut a patch against, so the box never opens and the status
/// line says so.
#[test]
fn d_in_the_diff_region_of_a_conflicted_file_says_why_instead_of_asking() {
    let mut state = state_on_diff(WorkingTreeSection::Unmerged, one_pair_diff(), 0);
    assert!(
        request_discard_at_diff_cursor(&mut state),
        "the key is answered, not ignored"
    );
    assert!(state.confirm.is_none(), "no box opened");
    let Status::Sticky { reason, .. } = &state.status else {
        panic!("expected a sticky reason, got {:?}", state.status);
    };
    assert!(reason.contains("conflicted file"), "{reason}");
}

#[test]
fn space_on_a_hunk_heading_stages_the_whole_hunk() {
    let diff = one_pair_diff();
    let mut state = state_on_diff(WorkingTreeSection::Unstaged, diff.clone(), 0);
    let mut commands = Vec::new();
    assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
    let (target_path, patch, side) = queued_patch(&commands);
    assert_eq!(target_path, &PathBuf::from("src/a.rs"));
    assert_eq!(side, PatchSide::Worktree);
    assert_eq!(
        patch,
        &graft_git::diff::hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward)
            .expect("the whole hunk always builds")
    );
}

#[test]
fn space_on_an_added_line_stages_that_line_alone() {
    let diff = one_pair_diff();
    // Body row 3 is the `+` line; raw hunk line 2.
    let mut state = state_on_diff(WorkingTreeSection::Unstaged, diff.clone(), 3);
    let mut commands = Vec::new();
    assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
    let (_, patch, _) = queued_patch(&commands);
    assert_eq!(
        patch,
        &graft_git::diff::hunk_patch(
            &diff,
            0,
            &Selection::Lines(vec![2]),
            PatchDirection::Forward
        )
        .expect("one added line builds")
    );
}

#[test]
fn space_on_a_context_line_stages_the_hunk_around_it() {
    let diff = one_pair_diff();
    // Body row 1 is the leading context line.
    let mut state = state_on_diff(WorkingTreeSection::Unstaged, diff.clone(), 1);
    let mut commands = Vec::new();
    assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
    let (_, patch, _) = queued_patch(&commands);
    assert_eq!(
        patch,
        &graft_git::diff::hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward)
            .expect("the whole hunk always builds")
    );
}

#[test]
fn a_hunk_under_staged_unstages_in_reverse_against_the_index() {
    let diff = one_pair_diff();
    let mut state = state_on_diff(WorkingTreeSection::Staged, diff.clone(), 0);
    let mut commands = Vec::new();
    assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
    let [Command::Operate(_, op, ..)] = commands.as_slice() else {
        panic!("expected one Operate");
    };
    assert!(
        matches!(&**op, Operation::Unstage(_)),
        "the STAGED section unstages: {op:?}"
    );
    let (_, patch, side) = queued_patch(&commands);
    assert_eq!(side, PatchSide::Index);
    assert_eq!(
        patch,
        &graft_git::diff::hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Reverse)
            .expect("the whole hunk always builds")
    );
}

/// The one refusal a user meets in ordinary work. It must read as a
/// refusal with a way out, never as a key that did nothing.
#[test]
fn a_single_line_at_a_missing_final_newline_is_refused_in_words() {
    // Body row 2 is the `+` line of a file with no trailing newline.
    let mut state = state_on_diff(WorkingTreeSection::Unstaged, no_trailing_newline_diff(), 2);
    let mut commands = Vec::new();
    assert!(
        stage_at_diff_cursor(&mut state, &mut commands, &mut ids()),
        "the key is answered, not ignored"
    );
    assert!(commands.is_empty(), "nothing is written");
    assert_eq!(state.operating, None);
    let Status::Sticky { reason: text, .. } = &state.status else {
        panic!(
            "expected a failure on the status line, got {:?}",
            state.status
        );
    };
    assert!(
        text.contains("no newline at the end") && text.contains("whole hunk"),
        "the reason names the cause and the way out: {text}"
    );
}

#[test]
fn the_whole_hunk_still_stages_where_a_single_line_cannot() {
    let mut state = state_on_diff(WorkingTreeSection::Unstaged, no_trailing_newline_diff(), 0);
    let mut commands = Vec::new();
    assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
    assert_eq!(commands.len(), 1, "the heading row still writes");
}

#[test]
fn d_in_the_diff_region_confirms_before_discarding_the_hunk() {
    let mut state = state_on_diff(WorkingTreeSection::Unstaged, one_pair_diff(), 0);
    assert!(request_discard_at_diff_cursor(&mut state));
    let pending = state.confirm.expect("confirm box opened");
    assert_eq!(pending.view.selected(), 0);
    assert!(
        pending.view.title.as_str().contains("hunk of src/a.rs"),
        "the question names what goes: {:?}",
        pending.view.title
    );
    assert_eq!(pending.view.command.as_str(), "git apply -R -");
    // The question and the button name the same act (m3/04 review M6):
    // "Discard this hunk of …?" over a `Confirm` button leaves the user
    // to remember what they are confirming.
    assert_eq!(pending.view.choices[0].as_str(), "Cancel");
    assert_eq!(pending.view.choices[1].as_str(), "Discard");
    assert!(
        matches!(
            &pending.then,
            ConfirmThen::Operate(op) if matches!(**op, Operation::Discard(Target::Patch {
                side: PatchSide::Worktree,
                ..
            }))
        ),
        "the working tree's own side is reversed: {:?}",
        pending.then
    );
}

#[test]
fn d_on_one_line_names_the_line_and_counts_it() {
    // Body row 3, the `+` line.
    let mut state = state_on_diff(WorkingTreeSection::Unstaged, one_pair_diff(), 3);
    assert!(request_discard_at_diff_cursor(&mut state));
    let pending = state.confirm.expect("confirm box opened");
    assert!(
        pending.view.title.as_str().contains("line of src/a.rs"),
        "{:?}",
        pending.view.title
    );
    let said: Vec<String> = pending
        .view
        .message
        .iter()
        .map(|line| line.as_str().to_string())
        .collect();
    assert!(
        said.iter()
            .any(|line| line.starts_with("1 added / 0 removed")),
        "the consequence counts the lines actually in the patch: {said:?}"
    );
}

/// ADR-0034: untracked files have no index side to cut a hunk against,
/// so the diff region's `Space` means the whole file there.
#[test]
fn space_in_an_untracked_files_diff_stages_the_whole_file() {
    let mut state = state_on_diff(WorkingTreeSection::Untracked, one_pair_diff(), 0);
    let mut commands = Vec::new();
    assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
    let [Command::Operate(_, op, ..)] = commands.as_slice() else {
        panic!("expected one Operate");
    };
    assert_eq!(
        **op,
        Operation::Stage(Target::Paths(vec!["src/a.rs".into()]))
    );
}

#[test]
fn stage_selected_stages_an_unstaged_file() {
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    let mut commands = Vec::new();
    assert!(stage_selected(&mut state, &mut commands, &mut ids()));
    assert!(state.operating.is_some());
    let Command::Operate(_, op, ..) = &commands[0] else {
        panic!("expected Operate");
    };
    assert_eq!(
        **op,
        Operation::Stage(Target::Paths(vec!["src/a.rs".into()]))
    );
}

#[test]
fn stage_selected_unstages_a_row_under_the_staged_section() {
    let mut state = state_on(
        "src/a.rs",
        staged_entry("src/a.rs"),
        WorkingTreeSection::Staged,
    );
    let mut commands = Vec::new();
    assert!(stage_selected(&mut state, &mut commands, &mut ids()));
    let Command::Operate(_, op, ..) = &commands[0] else {
        panic!("expected Operate");
    };
    assert_eq!(
        **op,
        Operation::Unstage(Target::Paths(vec!["src/a.rs".into()]))
    );
}

/// A path staged *and* unstaged draws a row under both sections. The
/// status flags are the same for both rows, so only the section can say
/// which way the write goes.
#[test]
fn section_not_status_decides_direction_for_a_file_in_both() {
    let both = StatusEntry {
        path: "src/a.rs".into(),
        original_path: None,
        staged: Some(Change::Modified),
        unstaged: Some(Change::Modified),
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    };
    let rows = vec![
        ("src/a.rs".to_string(), WorkingTreeSection::Staged),
        ("src/a.rs".to_string(), WorkingTreeSection::Unstaged),
    ];

    // Row 1 is the file under STAGED, row 3 the same file under UNSTAGED.
    let mut staged_row = state_with(&rows, vec![both.clone()], 1);
    let mut commands = Vec::new();
    assert!(stage_selected(&mut staged_row, &mut commands, &mut ids()));
    let Command::Operate(_, op, ..) = &commands[0] else {
        panic!("expected Operate");
    };
    assert_eq!(
        **op,
        Operation::Unstage(Target::Paths(vec!["src/a.rs".into()])),
        "the STAGED row unstages"
    );

    let mut unstaged_row = state_with(&rows, vec![both], 3);
    let mut commands = Vec::new();
    assert!(stage_selected(&mut unstaged_row, &mut commands, &mut ids()));
    let Command::Operate(_, op, ..) = &commands[0] else {
        panic!("expected Operate");
    };
    assert_eq!(
        **op,
        Operation::Stage(Target::Paths(vec!["src/a.rs".into()])),
        "the UNSTAGED row stages"
    );
}

#[test]
fn stage_selected_on_a_section_row_takes_every_file_under_it() {
    let rows = vec![
        ("src/a.rs".to_string(), WorkingTreeSection::Unstaged),
        ("src/b.rs".to_string(), WorkingTreeSection::Unstaged),
        ("new.txt".to_string(), WorkingTreeSection::Untracked),
    ];
    let entries = vec![
        unstaged_entry("src/a.rs"),
        unstaged_entry("src/b.rs"),
        untracked_entry("new.txt"),
    ];
    // Row 0 is the UNSTAGED heading; its two files follow.
    let mut state = state_with(&rows, entries, 0);
    let mut commands = Vec::new();
    assert!(stage_selected(&mut state, &mut commands, &mut ids()));
    let Command::Operate(_, op, ..) = &commands[0] else {
        panic!("expected Operate");
    };
    assert_eq!(
        **op,
        Operation::Stage(Target::Paths(vec!["src/a.rs".into(), "src/b.rs".into()])),
        "the untracked file below the next heading is not swept in"
    );
}

#[test]
fn stage_selected_is_a_noop_while_a_write_is_already_running() {
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    // The first `Space` launched the write and said so; the second must
    // change nothing, and the line it left standing is what tells the
    // user why (m3/04 review M3).
    let mut commands = Vec::new();
    assert!(stage_selected(&mut state, &mut commands, &mut ids()));
    assert_eq!(state.status, Status::Busy("stage src/a.rs".into()));

    let mut again = Vec::new();
    assert!(!stage_selected(&mut state, &mut again, &mut ids()));
    assert!(again.is_empty());
    assert_eq!(state.status, Status::Busy("stage src/a.rs".into()));
}

#[test]
fn stage_all_stages_every_path_in_status() {
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    let mut commands = Vec::new();
    assert_eq!(stage_all(&mut state, &mut commands, &mut ids()), Ok(()));
    let Command::Operate(_, op, ..) = &commands[0] else {
        panic!("expected Operate");
    };
    assert_eq!(
        **op,
        Operation::Stage(Target::Paths(vec!["src/a.rs".into()]))
    );
}

#[test]
fn stage_all_is_a_noop_on_a_clean_tree() {
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    state.working_tree = Some(Arc::new(WorkingTreeStatus::default()));
    let mut commands = Vec::new();
    assert_eq!(
        stage_all(&mut state, &mut commands, &mut ids()),
        Err(NotStaged::NothingToStage)
    );
}

#[test]
fn request_discard_names_clean_for_an_untracked_file() {
    let mut state = state_on(
        "new.txt",
        untracked_entry("new.txt"),
        WorkingTreeSection::Untracked,
    );
    assert!(request_discard(&mut state));
    let pending = state.confirm.expect("confirm box opened");
    assert_eq!(pending.view.command.as_str(), "git clean -f -- new.txt");
    assert_eq!(
        pending.then,
        ConfirmThen::operate(Operation::Discard(Target::Paths(vec!["new.txt".into()])))
    );
}

#[test]
fn request_discard_names_restore_staged_for_a_staged_file() {
    let mut state = state_on(
        "src/a.rs",
        staged_entry("src/a.rs"),
        WorkingTreeSection::Staged,
    );
    assert!(request_discard(&mut state));
    let pending = state.confirm.expect("confirm box opened");
    assert_eq!(
        pending.view.command.as_str(),
        "git restore --staged --worktree --source=HEAD -- src/a.rs"
    );
}

#[test]
fn request_discard_opens_with_cancel_under_the_cursor() {
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    assert!(request_discard(&mut state));
    let pending = state.confirm.expect("confirm box opened");
    assert_eq!(pending.view.selected(), 0);
}

fn deleted_and_staged(path: &str) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: Some(Change::Deleted),
        unstaged: None,
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    }
}

pub(in crate::update) fn unmerged_entry(path: &str) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged: true,
        unmerged_how: None,
        submodule: None,
    }
}

#[test]
fn stage_all_skips_the_paths_git_add_would_refuse() {
    // m3/04 review M2: `git add` on a staged deletion is
    // `fatal: pathspec ... did not match any files`, exit 128, nothing
    // staged — so one such entry used to make the whole `Ctrl+A` fail.
    // An unmerged path is skipped too: `git add` there marks the conflict
    // resolved with the markers still in the file.
    let mut state = state_with(
        &[("src/a.rs".into(), WorkingTreeSection::Unstaged)],
        vec![
            deleted_and_staged("gone.rs"),
            unmerged_entry("conflict.rs"),
            unstaged_entry("src/a.rs"),
            untracked_entry("new.txt"),
        ],
        1,
    );
    let mut commands = Vec::new();
    assert_eq!(stage_all(&mut state, &mut commands, &mut ids()), Ok(()));
    let Command::Operate(_, op, ..) = &commands[0] else {
        panic!("expected Operate");
    };
    assert_eq!(
        **op,
        Operation::Stage(Target::Paths(vec!["src/a.rs".into(), "new.txt".into()]))
    );
}

#[test]
fn stage_all_refuses_when_nothing_left_is_stageable() {
    let mut state = state_with(
        &[("gone.rs".into(), WorkingTreeSection::Staged)],
        vec![deleted_and_staged("gone.rs")],
        1,
    );
    let mut commands = Vec::new();
    assert_eq!(
        stage_all(&mut state, &mut commands, &mut ids()),
        Err(NotStaged::NothingToStage)
    );
    assert!(commands.is_empty());
    assert_eq!(state.operating, None);
}

#[test]
fn side_by_side_refuses_a_hunk_write_instead_of_acting_unseen() {
    // m3/04 review M5: the side-by-side view draws no line cursor, so a
    // `Space` there would stage a hunk the user cannot see.
    for section in [WorkingTreeSection::Unstaged, WorkingTreeSection::Staged] {
        let mut state = state_on_diff(section, one_pair_diff(), 1);
        state.detail.mode = graft_ui::detail::DiffMode::SideBySide;
        let mut commands = Vec::new();
        assert!(stage_at_diff_cursor(&mut state, &mut commands, &mut ids()));
        assert!(commands.is_empty(), "{commands:?}");
        assert_eq!(state.operating, None);
        let Status::Sticky { reason: said, .. } = &state.status else {
            panic!("expected a refusal, got {:?}", state.status);
        };
        assert!(said.contains("press v"), "{said}");

        let mut state = state_on_diff(section, one_pair_diff(), 1);
        state.detail.mode = graft_ui::detail::DiffMode::SideBySide;
        assert!(request_discard_at_diff_cursor(&mut state));
        assert!(state.confirm.is_none());
        let Status::Sticky { reason: said, .. } = &state.status else {
            panic!("expected a refusal, got {:?}", state.status);
        };
        assert!(said.contains("press v"), "{said}");
    }
}

#[test]
fn the_discard_box_names_deletion_for_an_untracked_file() {
    // m3/04 review M6: `git clean -f` deletes it. The box never said so.
    let mut state = state_on(
        "new.txt",
        untracked_entry("new.txt"),
        WorkingTreeSection::Untracked,
    );
    assert!(request_discard(&mut state));
    let pending = state.confirm.as_ref().expect("a box opened");
    assert_eq!(pending.view.title.as_str(), "Delete new.txt?");
    // The question and the button say the same thing (m3/04 review
    // round; the box used to ask "Delete" over a `Discard` button).
    assert_eq!(pending.view.choices[1].as_str(), "Delete");
    assert_eq!(pending.view.choices[0].as_str(), "Cancel");
    let said = pending
        .view
        .message
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    assert!(said.contains("Deletes the file"), "{said}");
    assert!(said.contains("backup"), "{said}");
}

#[test]
fn a_tracked_file_still_asks_to_discard_and_says_discard_on_the_button() {
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    assert!(request_discard(&mut state));
    let pending = state.confirm.as_ref().expect("a box opened");
    assert_eq!(pending.view.title.as_str(), "Discard changes to src/a.rs?");
    assert_eq!(pending.view.choices[1].as_str(), "Discard");
}

#[test]
fn the_discard_box_says_both_changes_go_for_a_file_staged_and_unstaged() {
    // m3/04 review M6: from the UNSTAGED row the pane shows only the
    // unstaged diff, but `plan_discard` runs
    // `restore --staged --worktree --source=HEAD` and both go.
    let both = StatusEntry {
        path: "src/a.rs".into(),
        original_path: None,
        staged: Some(Change::Modified),
        unstaged: Some(Change::Modified),
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    };
    let mut state = state_on("src/a.rs", both, WorkingTreeSection::Unstaged);
    assert!(request_discard(&mut state));
    let pending = state.confirm.as_ref().expect("a box opened");
    let said = pending
        .view
        .message
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        said.contains("Both the staged and the unstaged change"),
        "{said}"
    );
    // The counts come from the pane's own file list, not from prose.
    assert!(said.contains("(+1 \u{2212}0)"), "{said}");
}

#[test]
fn a_dead_operate_job_does_not_take_the_write_keys_with_it() {
    // m3/04 review M4: nothing but `Product::Operated` cleared
    // `operating`, so one panicked or cancelled write left every write
    // key a no-op until the user quit.
    for ended in [true, false] {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let id = JobId::new(3);
        state.operating = Some(id);
        let commands = write_job_ended(
            &mut state,
            if ended { id } else { JobId::new(9) },
            &mut ids(),
        );
        if ended {
            assert_eq!(state.operating, None);
            assert!(
                commands
                    .iter()
                    .any(|command| matches!(command, Command::ReadWorkingTree(_))),
                "{commands:?}"
            );
            // Every write key's first line is this check, so clearing it
            // is exactly what brings them back.
            assert!(!operation_in_flight(&state));
        } else {
            // A different job ending says nothing about the write.
            assert_eq!(state.operating, Some(id));
            assert!(commands.is_empty());
        }
    }
}

#[test]
fn operated_drops_a_stale_answer() {
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    state.operating = Some(JobId::new(1));
    let commands = operated(
        &mut state,
        JobId::new(2),
        Operated::Failed {
            error: "boom".into(),
            refresh: Some(Refresh::WorkingTree),
            remote: None,
            refused: None,
            edits: None,
        },
        &mut ids(),
    );
    assert!(commands.is_empty());
    assert_eq!(state.operating, Some(JobId::new(1)));
}

/// The two refusals a confirmed Ghost Preview can come back with are
/// worded as a problem **and a key**, not as a bare sentence
/// (m5-history-and-ghost-preview/08, piece 8; design note hard choices
/// 3 and 4).
///
/// `Moved` names the key that drew the picture, which is the key that
/// redraws it — that is the whole point of carrying `opened_by` through
/// the launch. `DirtyTree` names `S` and how many files it would take.
#[test]
fn a_refused_history_operation_says_which_key_answers_it() {
    let rebase = Operation::Rebase(graft_ops::operation::RebaseParams::new(
        graft_ops::operation::Tip {
            name: "refs/heads/feature".to_owned(),
            oid: graft_core::oid::Oid::from_bytes(&[1; 20]).expect("20 bytes"),
        },
        graft_ops::operation::Source::Branch("main".to_owned()),
        graft_core::oid::Oid::from_bytes(&[2; 20]).expect("20 bytes"),
    ));
    let cases = [
        (
            Refusal::Moved {
                name: "refs/heads/main".to_owned(),
            },
            Status::Sticky {
                reason: "main moved since the preview".to_owned(),
                keys: Some("r again".to_owned()),
            },
        ),
        (
            Refusal::DirtyTree { count: 3 },
            Status::Sticky {
                reason: "rebase needs a clean tree".to_owned(),
                keys: Some("S stash 3 changed files".to_owned()),
            },
        ),
    ];
    for (refusal, want) in cases {
        let mut state = AppState::default();
        let mut commands = Vec::new();
        launch_unstoppable(
            &mut state,
            &mut commands,
            rebase.clone(),
            crate::update::preview::HISTORY_CANNOT_STOP,
            Some("r"),
            &mut || JobId::new(9),
        )
        .expect("no write is running");
        let commands = operated(
            &mut state,
            JobId::new(9),
            Operated::Failed {
                error: "the sentence nobody should see here".into(),
                refresh: None,
                remote: None,
                refused: Some(refusal.clone()),
                edits: None,
            },
            &mut ids(),
        );
        assert!(commands.is_empty(), "{refusal:?}: {commands:?}");
        assert_eq!(state.status, want, "{refusal:?}");
    }
}

/// Every other refusal keeps `graft_ops`' own sentence: the app invents
/// no key hint it cannot honour.
#[test]
fn a_refusal_with_no_key_to_offer_keeps_the_plain_sentence() {
    let mut state = AppState::default();
    let id = JobId::new(4);
    state.operating = Some(id);
    let commands = operated(
        &mut state,
        id,
        Operated::Failed {
            error: "nothing staged".into(),
            refresh: None,
            remote: None,
            refused: Some(Refusal::NothingStaged),
            edits: None,
        },
        &mut ids(),
    );
    assert!(commands.is_empty());
    assert_eq!(state.status, Status::sticky("nothing staged"));
}

#[test]
fn operated_failed_part_way_reports_the_error_and_still_refreshes() {
    // m3/04 review M1. `ApplyError::Git` is "git ran part-way, journaled
    // `Aborted`": the file list and the diff bytes on screen no longer
    // describe the repository, so the next `Space` would cut a patch from
    // stale bytes. The failure is shown *and* the pane re-reads.
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    let id = JobId::new(5);
    state.operating = Some(id);
    let commands = operated(
        &mut state,
        id,
        Operated::Failed {
            error: "boom".into(),
            refresh: Some(Refresh::WorkingTree),
            remote: None,
            refused: None,
            edits: None,
        },
        &mut ids(),
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        "{commands:?}"
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ReadHistory(..))),
        "a working-tree failure must not restart the walk: {commands:?}"
    );
    assert_eq!(state.operating, None);
    assert_eq!(state.status, Status::sticky("boom"));
}

#[test]
fn operated_failed_before_anything_ran_leaves_the_screen_alone() {
    // A refusal writes nothing, so there is nothing to re-read; the job
    // says so with `refresh: None` rather than the pane guessing.
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    let id = JobId::new(5);
    state.operating = Some(id);
    let commands = operated(
        &mut state,
        id,
        Operated::Failed {
            error: "nothing to do".into(),
            refresh: None,
            remote: None,
            refused: None,
            edits: None,
        },
        &mut ids(),
    );
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(state.operating, None);
    assert_eq!(state.status, Status::sticky("nothing to do"));
}

/// The state a conflicted merge leaves: one journaled `Pending` entry,
/// git's own state live, two files unmerged.
fn conflicted(paths: &[&str], stalled: Option<&str>) -> Operated {
    Operated::Conflicted {
        rewrite: None,
        description: "merge feature into main".to_owned(),
        kind: InProgress::Merge,
        paths: paths.iter().map(PathBuf::from).collect(),
        stalled: stalled.map(ToOwned::to_owned),
        entry: graft_core::journal::EntryId::new(42, [4; 10]),
    }
}

/// **Gap row 505.** The conflict's answer names its journal entry, and
/// that id reaches the state the banner is drawn from — `live_entries`
/// — unchanged and at once, not only when the journal re-read lands.
#[test]
fn conflicted_entry_id_survives() {
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    let id = JobId::new(7);
    state.operating = Some(id);
    operated(&mut state, id, conflicted(&["src/a.rs"], None), &mut ids());
    assert_eq!(
        state.live_entries,
        vec![graft_core::journal::EntryId::new(42, [4; 10])]
    );
}

#[test]
fn a_conflicted_merge_is_shown_as_a_state_and_re_reads_everything() {
    // m5/07 seam. `ApplyError::Conflicted` is not a failure: git stopped,
    // the entry is `Pending`, and the user is now *in* the merge. The
    // line has to say so, survive the refresh it triggers, and the
    // refresh has to be the wide one — it is what puts the unmerged
    // paths under CONFLICTS and fills `AppState::in_progress`.
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    let id = JobId::new(7);
    state.operating = Some(id);
    let commands = operated(
        &mut state,
        id,
        conflicted(&["src/a.rs", "src/b.rs"], None),
        &mut ids(),
    );
    assert_eq!(state.operating, None);
    // m6-conflicts/12: the stop is state, not a status, so it outlives
    // its own refresh and anything else that takes the status slot.
    assert_eq!(state.status, Status::Ready);
    let stop = state.stopped.as_ref().expect("a live conflict is a stop");
    assert_eq!(stop.kind, InProgress::Merge);
    assert!(!stop.again);
    let reason = crate::update::banner::stop_reason(stop, None);
    assert_eq!(reason, "merge stopped \u{b7} 2 files conflicted");
    assert!(
        !reason.contains("failed") && !reason.contains("error"),
        "a conflict is a state, not an error: {reason}"
    );
    // The wide refresh, which is `Ctrl+R`'s own path: the working tree
    // (unmerged flags and `in_progress`) and the graph both.
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        "{commands:?}"
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadHistory(..))),
        "refs moved: {commands:?}"
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadRefs(_))),
        "{commands:?}"
    );
}

#[test]
fn a_conflict_in_one_file_counts_one_file() {
    let mut state = AppState::default();
    let id = JobId::new(7);
    state.operating = Some(id);
    operated(&mut state, id, conflicted(&["src/a.rs"], None), &mut ids());
    let stop = state.stopped.as_ref().expect("a stop");
    assert_eq!(
        crate::update::banner::stop_reason(stop, None),
        "merge stopped \u{b7} 1 file conflicted"
    );
}

#[test]
fn a_stop_with_nothing_unmerged_does_not_claim_a_conflict() {
    // A rebase can stop with no unmerged path at all. Saying "0 files
    // conflict" would be a lie about why it stopped.
    let mut state = AppState::default();
    let id = JobId::new(7);
    state.operating = Some(id);
    operated(&mut state, id, conflicted(&[], None), &mut ids());
    let stop = state.stopped.as_ref().expect("a stop");
    let reason = crate::update::banner::stop_reason(stop, None);
    assert_eq!(reason, "merge stopped");
    assert!(!reason.contains("conflict"), "{reason}");
}

/// m6-conflicts/18 (P5): a stop with something unmerged lands in the
/// resolver on the answer's first path — git's status order, the same
/// source `first_conflicted` reads — opened from the banner.
#[test]
fn a_conflicted_merge_lands_in_the_resolver() {
    let mut state = AppState::default();
    let id = JobId::new(7);
    state.operating = Some(id);
    let commands = operated(
        &mut state,
        id,
        conflicted(&["src/a.rs", "src/b.rs"], None),
        &mut ids(),
    );
    let open = state
        .resolver
        .as_ref()
        .expect("the stop lands in the resolver");
    assert_eq!(open.path, PathBuf::from("src/a.rs"));
    assert_eq!(
        open.opened_from,
        crate::update::resolver::OpenedFrom::Banner
    );
    // Block 0 is set when the opening read lands (`answers.rs`), as for
    // any open; before it, no block is claimed.
    assert_eq!(open.cursor_block, None);
    assert!(state.banner_focus, "Esc goes back to the banner");
    assert!(state.stopped.is_some());
    assert_eq!(state.status, Status::Ready, "no refusal was worded");
    assert!(
        commands.iter().any(|command| matches!(
            command,
            Command::OpenConflict(_, path) if path == &PathBuf::from("src/a.rs")
        )),
        "{commands:?}"
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        "the wide refresh still runs: {commands:?}"
    );
    // Before the file list lands, `file_cursor` is 0 — the first row,
    // which is where `src/a.rs` will sit once it does.
    assert_eq!(crate::update::resolver::file_cursor(&state), 0);
}

/// m6-conflicts/29 (review-18 Major 1): `c` opens the commit editor
/// while the merge runs; the stop then lands where it lands without a
/// resolver — no refusal words, the box untouched.
#[test]
fn a_box_opened_mid_merge_keeps_the_stop_on_the_banner_without_words() {
    let mut state = AppState::default();
    let id = JobId::new(7);
    state.operating = Some(id);
    let mut editor_ids = ids();
    // The editor's template read is not under test here.
    let _template_read = crate::update::commit_editor::open(&mut state, false, &mut editor_ids);
    assert!(state.commit_editor.is_some(), "c opens the box mid-merge");
    let before = state.commit_editor.clone();
    operated(&mut state, id, conflicted(&["src/a.rs"], None), &mut ids());
    assert!(state.resolver.is_none(), "no resolver over the box");
    assert_eq!(state.status, Status::Ready, "no refusal was worded");
    assert!(state.stopped.is_some());
    assert_eq!(state.commit_editor, before, "the box is untouched");
}

#[test]
fn an_edit_stop_with_nothing_unmerged_keeps_the_banner() {
    let mut state = AppState::default();
    let id = JobId::new(7);
    state.operating = Some(id);
    let commands = operated(&mut state, id, conflicted(&[], None), &mut ids());
    assert!(state.resolver.is_none(), "nothing to resolve");
    assert!(state.stopped.is_some());
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(..))),
        "{commands:?}"
    );
}

#[test]
fn a_continue_git_refused_repeats_gits_own_words() {
    // `ApplyError::ResumeFailed`: a hook said no, the state is still
    // live, and nothing is unmerged — git's line is the only thing that
    // can say what to do, so it is passed through rather than replaced.
    let mut state = AppState::default();
    let id = JobId::new(7);
    state.operating = Some(id);
    operated(
        &mut state,
        id,
        conflicted(&[], Some("pre-commit hook refused")),
        &mut ids(),
    );
    let Status::Sticky { reason, .. } = &state.status else {
        panic!("{:?}", state.status)
    };
    assert!(reason.contains("pre-commit hook refused"), "{reason}");
    assert!(
        reason.contains("the merge is still in progress"),
        "{reason}"
    );
}

#[test]
fn operated_done_refreshes_the_working_tree_without_restarting_the_walk() {
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    let id = JobId::new(7);
    state.operating = Some(id);
    let commands = operated(
        &mut state,
        id,
        Operated::Done(applied("stage src/a.rs", Refresh::WorkingTree)),
        &mut ids(),
    );
    assert_eq!(state.status, Status::done("stage src/a.rs"));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        "{commands:?}"
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ReadHistory(..))),
        "{commands:?}"
    );
}

/// The id rewind that made `u` after a commit answer under another
/// job's name (m3-working-tree-undo-and-stash/06): a caller borrows
/// `local_jobs` out into a closure and writes its copy back when the
/// callee returns, so any id the callee minted from the state's own
/// counter is thrown away and the next mint reissues an id still in
/// flight. Both refresh paths must mint from the closure only.
#[test]
fn a_refresh_never_mints_from_the_states_own_counter() {
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    let before = state.local_jobs;

    let mut commands = Vec::new();
    crate::update::keys::refresh_key(&mut state, &mut commands, &mut ids());
    assert!(!commands.is_empty(), "the refresh asked for something");
    assert_eq!(state.local_jobs, before, "Ctrl+R's path");

    let id = JobId::new(9);
    state.operating = Some(id);
    let commands = write_job_ended(&mut state, id, &mut ids());
    assert!(!commands.is_empty(), "the ended write asked for something");
    assert_eq!(state.local_jobs, before, "a write job that ended badly");
}

#[test]
fn operated_done_on_everything_emits_the_same_commands_as_ctrl_r() {
    // m3/04 review M8: `Everything` used to be a hand-written subset that
    // had drifted from `Ctrl+R` — no walk cancellation, no repository
    // re-open, no Mistake Radar reset.
    let mut state = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    let id = JobId::new(9);
    state.operating = Some(id);
    let after_write = operated(
        &mut state,
        id,
        Operated::Done(applied("commit", Refresh::Everything)),
        &mut ids(),
    );

    let mut fresh = state_on(
        "src/a.rs",
        unstaged_entry("src/a.rs"),
        WorkingTreeSection::Unstaged,
    );
    let mut after_ctrl_r = Vec::new();
    crate::update::keys::refresh_key(&mut fresh, &mut after_ctrl_r, &mut ids());

    assert_eq!(after_write, after_ctrl_r, "{after_write:?}");
    assert!(
        after_write.contains(&Command::OpenRepository),
        "{after_write:?}"
    );
}
/// Major 2 of the m3/05 review: `e` had no test at all, though the
/// ticket's NOTES said it was tested.
#[test]
fn e_on_a_file_asks_for_that_files_porcelain_path() {
    let state = state_with(
        &[("src/a.rs".to_owned(), WorkingTreeSection::Unstaged)],
        vec![unstaged_entry("src/a.rs")],
        1,
    );

    let mut commands = Vec::new();
    let handled = crate::update::detail::detail_key(
        &mut state.clone(),
        &mut commands,
        graft_core::event::KeyPress {
            code: graft_core::event::KeyCode::Char('e'),
            modifiers: graft_core::event::Modifiers::default(),
        },
    );

    assert!(handled, "`e` is the changes pane's key");
    assert_eq!(
        commands,
        vec![Command::OpenExternalEditor {
            path: PathBuf::from("src/a.rs"),
            line: None,
        }],
        "the path is the porcelain's, relative to the worktree root — \
         `Handover` is what joins it onto that root"
    );
}

/// The cursor's line travels with it when the diff region has focus, so
/// the editor opens where the user was reading.
#[test]
fn e_in_the_diff_carries_the_cursors_new_side_line() {
    let state = state_on_diff(WorkingTreeSection::Unstaged, one_pair_diff(), 3);

    let target = edit_target(&state).expect("a file is under the cursor");
    assert_eq!(target.0, PathBuf::from("src/a.rs"));
    assert!(
        target.1.is_some(),
        "the added line has a new-side number: {target:?}"
    );
}

/// m6-conflicts/46: an untracked file of one line says `1 line`.
#[test]
fn untracked_size_is_singular_at_one() {
    assert_eq!(untracked_size(1), " \u{2014} 1 line");
    assert_eq!(untracked_size(7), " \u{2014} 7 lines");
}

mod edits_words;
mod landing;
mod lands_on_head;
mod one_write;
mod staging_not_kept;
