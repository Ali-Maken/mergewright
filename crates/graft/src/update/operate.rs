//! Working-tree writes from the changes pane: `Space`/`s` stage or unstage
//! the file under the cursor, `Ctrl+A` stages every changed and untracked
//! path, `d` opens the discard confirm box
//! (m3-working-tree-undo-and-stash/04, design note "Changes pane").
//!
//! One write in flight at a time (`AppState::operating`); every other write
//! key is a no-op while it is `Some`.
//!
//! Seam rule (ARCHITECTURE.md, `crates/graft/tests/it/seam_rule.rs`): this
//! module only builds an [`Operation`] and a [`Command::Operate`] from it —
//! `graft_ops` decides what the write does, never `app`.

use std::path::{Path, PathBuf};

use graft_core::job::JobId;
use graft_git::status::StatusEntry;
use graft_ops::apply::Refresh;
use graft_ops::operation::{Operation, Target};
use graft_ui::detail::WorkingTreeSection;

use super::confirm::PendingConfirm;
use super::detail::Loading;
use super::events::Command;
use super::state::{AppState, Status};
use crate::jobs::operate::Operated;

/// A write is already running: every other write key is a no-op.
#[must_use]
pub fn operation_in_flight(state: &AppState) -> bool {
    state.operating.is_some()
}

/// The path under the changes-pane cursor, read from the file list the pane
/// already has — the same list its diff reads use
/// (`DetailState::selected_entry`).
fn selected_path(state: &AppState) -> Option<PathBuf> {
    let list = state.detail.list.as_ref()?;
    let entry = state.detail.selected_entry()?;
    let file = list.files.get(usize::try_from(entry).ok()?)?;
    Some(bytes_to_pathbuf(&file.path))
}

/// Every path the changes-pane cursor names: the one file of a file row,
/// every file of a section row, nothing for any other row. A section row's
/// files come out in the order the list draws them.
fn selected_paths(state: &AppState) -> Vec<PathBuf> {
    let Some(list) = state.detail.list.as_ref() else {
        return Vec::new();
    };
    let Loading::Ready(files) = &state.detail.files else {
        return Vec::new();
    };
    let path_of = |entry: u32| {
        usize::try_from(entry)
            .ok()
            .and_then(|index| list.files.get(index))
            .map(|file| bytes_to_pathbuf(&file.path))
    };
    match files.selected_entry() {
        Some(entry) => path_of(entry).into_iter().collect(),
        None => files
            .section_entries(files.cursor)
            .into_iter()
            .filter_map(path_of)
            .collect(),
    }
}

#[cfg(unix)]
fn bytes_to_pathbuf(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
fn bytes_to_pathbuf(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

/// `path`'s status entry, read from `AppState::working_tree` — the same read
/// that drives the uncommitted-changes row and the pane's header counts.
fn status_entry<'a>(state: &'a AppState, path: &Path) -> Option<&'a StatusEntry> {
    state.working_tree.as_ref()?.entry(path)
}

/// `Space`/`s` on the changes pane's file list: stage or unstage the file
/// under the cursor — or every file of the section row under it.
///
/// **Section membership decides direction**, not the file's status flags: a
/// path that is staged *and* unstaged has a row under both groups, and the
/// two rows mean opposite writes (design note, "Changes pane"). Reading the
/// flags instead would make the STAGED row of such a file stage it again.
///
/// A no-op — `false` — while a write is running, off the changes pane, or on
/// a row that names no file, so the caller can fall back to the key's other
/// meaning (folding a directory row).
pub fn stage_selected(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if operation_in_flight(next) {
        return false;
    }
    let Some(section) = next.detail.selected_section() else {
        return false;
    };
    let paths = selected_paths(next);
    if paths.is_empty() {
        return false;
    }
    let target = Target::Paths(paths);
    let op = match section {
        WorkingTreeSection::Staged => Operation::Unstage(target),
        WorkingTreeSection::Unstaged | WorkingTreeSection::Untracked => Operation::Stage(target),
    };
    let id = next_id();
    next.operating = Some(id);
    commands.push(Command::Operate(id, Box::new(op)));
    true
}

/// `Ctrl+A`: stage every path `AppState::working_tree` lists — staged,
/// unstaged and untracked alike (`git add` on an already-staged path is a
/// no-op). `false` while a write is running or nothing is dirty.
pub fn stage_all(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if operation_in_flight(next) {
        return false;
    }
    let Some(status) = next.working_tree.as_ref() else {
        return false;
    };
    if status.is_clean() {
        return false;
    }
    let paths: Vec<PathBuf> = status
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect();
    let id = next_id();
    next.operating = Some(id);
    commands.push(Command::Operate(
        id,
        Box::new(Operation::Stage(Target::Paths(paths))),
    ));
    true
}

/// `d`: opens the confirm box for discarding the file under the cursor —
/// the consequence in plain words, the exact command, `Cancel` under the
/// cursor. `false` under the same conditions as [`stage_selected`], and the
/// key is free to mean something else.
pub fn request_discard(next: &mut AppState) -> bool {
    if operation_in_flight(next) {
        return false;
    }
    let Some(path) = selected_path(next) else {
        return false;
    };
    let Some(entry) = status_entry(next, &path) else {
        return false;
    };
    let command = discard_command(&path, entry);
    let shown = path.display();
    let view = graft_ui::confirm::ConfirmView::new(
        format!("Discard changes to {shown}?"),
        vec![
            "This throws away the change in the working tree (and the".to_string(),
            "index, if staged). Graft keeps a backup first, so `u` can".to_string(),
            "undo it afterwards.".to_string(),
        ],
        command,
    );
    next.confirm = Some(PendingConfirm {
        view,
        then: Operation::Discard(Target::Paths(vec![path])),
    });
    true
}

/// The command the confirm box shows: what `graft_ops::operation::stage`
/// actually runs for a whole-file discard of this path's class
/// (`crates/graft-ops/src/operation/stage.rs::plan_discard`).
fn discard_command(path: &Path, entry: &StatusEntry) -> String {
    let shown = path.display();
    if entry.untracked {
        format!("git clean -f -- {shown}")
    } else if entry.is_staged() {
        format!("git restore --staged --worktree --source=HEAD -- {shown}")
    } else {
        format!("git restore --worktree -- {shown}")
    }
}

/// An operate job answered. Stale answers (a job that is not the one
/// `AppState::operating` is waiting for) are dropped. `Operated::Incomplete`
/// still refreshes — the write happened even though part of its after-state
/// could not be read (`ApplyError::AfterState`'s contract,
/// `.scratch/m3-working-tree-undo-and-stash/as-built.md`).
pub fn operated(
    state: &mut AppState,
    id: JobId,
    outcome: Operated,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.operating != Some(id) {
        return Vec::new();
    }
    state.operating = None;
    match outcome {
        Operated::Done(applied) => {
            state.status = Status::Done(applied.entry.description.clone());
            refresh(state, applied.refresh, next_id)
        }
        Operated::Incomplete { applied, error } => {
            state.status = Status::Failed(format!(
                "{} \u{2014} could not confirm it fully: {error}",
                applied.entry.description
            ));
            refresh(state, applied.refresh, next_id)
        }
        Operated::Failed(error) => {
            state.status = Status::Failed(error);
            Vec::new()
        }
    }
}

fn refresh(
    state: &mut AppState,
    what: Refresh,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let mut commands = vec![Command::ReadWorkingTree];
    commands.extend(state.detail.refresh(next_id));
    if what == Refresh::Everything {
        commands.push(Command::ReadHistory(graft_git::history::Order::Topological));
        commands.push(Command::ReadRefs);
    }
    commands
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use graft_core::job::JobId;
    use graft_git::diff::{FileEntry, FileList, FileStatus};
    use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
    use graft_ui::detail::{DetailFocus, FileListView, FileRow, FileRowKind, FileStatusView};

    use super::*;
    use crate::update::detail::DetailSpec;

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
    fn state_with(
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

    fn unstaged_entry(path: &str) -> StatusEntry {
        StatusEntry {
            path: path.into(),
            original_path: None,
            staged: None,
            unstaged: Some(Change::Modified),
            untracked: false,
            unmerged: false,
            submodule: false,
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
            submodule: false,
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
            submodule: false,
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
        let Command::Operate(_, op) = &commands[0] else {
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
        let Command::Operate(_, op) = &commands[0] else {
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
            submodule: false,
        };
        let rows = vec![
            ("src/a.rs".to_string(), WorkingTreeSection::Staged),
            ("src/a.rs".to_string(), WorkingTreeSection::Unstaged),
        ];

        // Row 1 is the file under STAGED, row 3 the same file under UNSTAGED.
        let mut staged_row = state_with(&rows, vec![both.clone()], 1);
        let mut commands = Vec::new();
        assert!(stage_selected(&mut staged_row, &mut commands, &mut ids()));
        let Command::Operate(_, op) = &commands[0] else {
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
        let Command::Operate(_, op) = &commands[0] else {
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
        let Command::Operate(_, op) = &commands[0] else {
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
        state.operating = Some(JobId::new(9));
        let mut commands = Vec::new();
        assert!(!stage_selected(&mut state, &mut commands, &mut ids()));
        assert!(commands.is_empty());
    }

    #[test]
    fn stage_all_stages_every_path_in_status() {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let mut commands = Vec::new();
        assert!(stage_all(&mut state, &mut commands, &mut ids()));
        let Command::Operate(_, op) = &commands[0] else {
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
        assert!(!stage_all(&mut state, &mut commands, &mut ids()));
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
        assert_eq!(pending.view.command, "git clean -f -- new.txt");
        assert_eq!(
            pending.then,
            Operation::Discard(Target::Paths(vec!["new.txt".into()]))
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
            pending.view.command,
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
        assert_eq!(pending.view.cursor, graft_ui::confirm::Choice::Cancel);
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
            Operated::Failed("boom".into()),
            &mut ids(),
        );
        assert!(commands.is_empty());
        assert_eq!(state.operating, Some(JobId::new(1)));
    }

    #[test]
    fn operated_failed_reports_the_error_and_asks_for_no_refresh() {
        let mut state = state_on(
            "src/a.rs",
            unstaged_entry("src/a.rs"),
            WorkingTreeSection::Unstaged,
        );
        let id = JobId::new(5);
        state.operating = Some(id);
        let commands = operated(&mut state, id, Operated::Failed("boom".into()), &mut ids());
        assert!(commands.is_empty());
        assert_eq!(state.operating, None);
        assert_eq!(state.status, Status::Failed("boom".into()));
    }
}
