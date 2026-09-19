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

use super::confirm::PendingConfirm;
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

/// `Space`/`s` on the changes pane: stage the file under the cursor unless
/// it is already staged with nothing left unstaged, in which case unstage
/// it. A no-op — `false` — while a write is running, the pane is not on a
/// file, or the file's status is not known yet, so the caller can fall back
/// to the key's other meaning (folding a directory row).
pub fn stage_selected(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if operation_in_flight(next) {
        return false;
    }
    let Some(path) = selected_path(next) else {
        return false;
    };
    let Some(entry) = status_entry(next, &path) else {
        return false;
    };
    let op = if entry.is_staged() && !entry.is_unstaged() {
        Operation::Unstage(Target::Paths(vec![path]))
    } else {
        Operation::Stage(Target::Paths(vec![path]))
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

    /// A pane on the changes pane, cursor on one file row backed by `entry`,
    /// with `status` behind `AppState::working_tree` — the shape
    /// `selected_path`/`status_entry` read.
    fn state_on(path: &str, entry: StatusEntry) -> AppState {
        let list = FileList {
            files: vec![FileEntry {
                path: path.as_bytes().to_vec(),
                old_path: None,
                status: FileStatus::Modified,
                added: Some(1),
                removed: Some(0),
            }],
            added: 1,
            removed: 0,
        };
        let view = FileListView {
            rows: vec![FileRow {
                depth: 0,
                name: graft_term::sanitize::sanitize_line(path),
                kind: FileRowKind::File {
                    status: FileStatusView::Modified,
                    added: Some(1),
                    removed: Some(0),
                },
                entry: Some(0),
            }],
            cursor: 0,
            top: 0,
            files: 1,
            added: 1,
            removed: 0,
            message: Vec::new(),
        };
        let mut state = AppState {
            working_tree: Some(Arc::new(WorkingTreeStatus {
                entries: vec![entry],
            })),
            ..AppState::default()
        };
        state.detail.spec = Some(DetailSpec::WorkingTree);
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Files;
        state.detail.files = crate::update::detail::Loading::Ready(Arc::new(view));
        state.detail.list = Some(Arc::new(list));
        state
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
        let mut state = state_on("src/a.rs", unstaged_entry("src/a.rs"));
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
    fn stage_selected_unstages_an_already_staged_file() {
        let mut state = state_on("src/a.rs", staged_entry("src/a.rs"));
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

    #[test]
    fn stage_selected_is_a_noop_while_a_write_is_already_running() {
        let mut state = state_on("src/a.rs", unstaged_entry("src/a.rs"));
        state.operating = Some(JobId::new(9));
        let mut commands = Vec::new();
        assert!(!stage_selected(&mut state, &mut commands, &mut ids()));
        assert!(commands.is_empty());
    }

    #[test]
    fn stage_all_stages_every_path_in_status() {
        let mut state = state_on("src/a.rs", unstaged_entry("src/a.rs"));
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
        let mut state = state_on("src/a.rs", unstaged_entry("src/a.rs"));
        state.working_tree = Some(Arc::new(WorkingTreeStatus::default()));
        let mut commands = Vec::new();
        assert!(!stage_all(&mut state, &mut commands, &mut ids()));
    }

    #[test]
    fn request_discard_names_clean_for_an_untracked_file() {
        let mut state = state_on("new.txt", untracked_entry("new.txt"));
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
        let mut state = state_on("src/a.rs", staged_entry("src/a.rs"));
        assert!(request_discard(&mut state));
        let pending = state.confirm.expect("confirm box opened");
        assert_eq!(
            pending.view.command,
            "git restore --staged --worktree --source=HEAD -- src/a.rs"
        );
    }

    #[test]
    fn request_discard_opens_with_cancel_under_the_cursor() {
        let mut state = state_on("src/a.rs", unstaged_entry("src/a.rs"));
        assert!(request_discard(&mut state));
        let pending = state.confirm.expect("confirm box opened");
        assert_eq!(pending.view.cursor, graft_ui::confirm::Choice::Cancel);
    }

    #[test]
    fn operated_drops_a_stale_answer() {
        let mut state = state_on("src/a.rs", unstaged_entry("src/a.rs"));
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
        let mut state = state_on("src/a.rs", unstaged_entry("src/a.rs"));
        let id = JobId::new(5);
        state.operating = Some(id);
        let commands = operated(&mut state, id, Operated::Failed("boom".into()), &mut ids());
        assert!(commands.is_empty());
        assert_eq!(state.operating, None);
        assert_eq!(state.status, Status::Failed("boom".into()));
    }
}
