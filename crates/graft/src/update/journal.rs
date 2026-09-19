//! The undo journal's state: what `J` shows, and the read that keeps it
//! current (m3-working-tree-undo-and-stash/06).
//!
//! Pure, like `update::map`, whose open/close/key/event shape this copies.
//! The journal itself is a file, so reading it is a job; what lives here is
//! which entry the cursor is on, which one is expanded, and whether the
//! interrupted-entry notice has been looked at.

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;

use graft_ui::detail::DetailFocus;

use super::detail::Loading;
use super::events::{Command, Product};
use super::state::{AppState, Status};
use crate::jobs::undo::JournalRead;

/// What `Enter` says on an entry older than the one `u` would take back.
/// ADR-0031: the time machine is M5's, behind Ghost Preview, and saying so
/// is better than a key that half works.
pub const OLDER_ENTRY: &str = "multi-step revert comes with Ghost Preview (M5)";

/// The journal browser's own state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JournalState {
    /// `J` opened it.
    pub shown: bool,
    /// The row under the cursor, newest first.
    pub selected: usize,
    /// The row whose Command Log is open, if any. One at a time: the log is
    /// several lines and two open at once pushes the cursor off screen.
    pub expanded: Option<usize>,
    /// The interrupted-entry notice has been looked at, so the footer stops
    /// saying so. One press of `J` is enough.
    pub notice_seen: bool,
    /// The journal as it was last read.
    pub entries: Loading<std::sync::Arc<JournalRead>>,
}

impl JournalState {
    /// Ask for the journal, under a fresh id. Idempotent in the sense that
    /// matters: a second read simply replaces the first, and the older
    /// answer is dropped by id.
    pub(super) fn read(&mut self, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        let id = next_id();
        self.entries = Loading::Pending(id);
        vec![Command::ReadJournal(id)]
    }

    /// The entries the browser draws, newest first. Empty while loading or
    /// after a failure — [`JournalState::entries`] says which.
    #[must_use]
    pub fn newest_first(&self) -> Vec<&graft_core::journal::JournalEntry> {
        match &self.entries {
            Loading::Ready(read) => read.entries.iter().rev().collect(),
            Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => Vec::new(),
        }
    }
}

/// The launch read: asks once, and never again on its own. Called at the
/// tail of every `update` (`super::settle`), which is where the radar's own
/// idempotent requests live.
pub(super) fn request(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    // Nothing to read until the repository is open: the journal is a file
    // inside it, and `journal_path` asks git where. The radar's own request
    // waits on a fact about the repository for the same reason.
    if state.repository.is_some() && state.journal.entries == Loading::Idle {
        commands.extend(state.journal.read(next_id));
    }
}

/// `Product::Journal` landed. Stale answers are dropped by id, the same way
/// every other `Loading` field drops them.
pub(super) fn journal_event(state: &mut AppState, id: JobId, product: &Product) -> bool {
    let Product::Journal(result) = product else {
        return false;
    };
    if state.journal.entries != Loading::Pending(id) {
        return true;
    }
    state.journal.entries = match result {
        Ok(read) => Loading::Ready(std::sync::Arc::clone(read)),
        Err(why) => Loading::Failed(why.clone()),
    };
    // A shorter journal must not leave the cursor past its end.
    let rows = state.journal.newest_first().len();
    state.journal.selected = state.journal.selected.min(rows.saturating_sub(1));
    if state.journal.expanded.is_some_and(|row| row >= rows) {
        state.journal.expanded = None;
    }
    true
}

/// `J`, and every key while the browser is open. `true` when the key was
/// this view's, so the caller sends it nowhere else — the same shape as
/// `update::map::map_key`, including the wildcard: an open overlay swallows
/// the keys it does not use rather than letting them move the graph
/// underneath it.
pub(super) fn journal_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if key.modifiers.control || key.modifiers.alt {
        return false;
    }
    if next.journal.shown {
        return shown_key(next, commands, key, next_id);
    }
    if key.code == KeyCode::Char('J') && next.detail.focus == DetailFocus::Graph {
        next.journal.shown = true;
        next.journal.selected = 0;
        next.journal.expanded = None;
        // Looking at the browser is what the interrupted-entry notice asks
        // for, so the notice stops once it has been taken up.
        next.journal.notice_seen = true;
        // Read it fresh: another Graft on the same worktree may have
        // appended since the last read.
        commands.extend(next.journal.read(next_id));
        return true;
    }
    false
}

/// The browser's own keys.
fn shown_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    // `Tab` belongs to the detail pane's focus cycle, always: an overlay
    // that swallowed it would be one the user could never `Tab` out of
    // (m3-working-tree-undo-and-stash/06).
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        return false;
    }
    let rows = next.journal.newest_first().len();
    match key.code {
        KeyCode::Char('J') | KeyCode::Escape => {
            close(next, commands);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            next.journal.selected = next.journal.selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            next.journal.selected = next
                .journal
                .selected
                .saturating_add(1)
                .min(rows.saturating_sub(1));
        }
        KeyCode::Char(' ') => {
            if rows > 0 {
                next.journal.expanded = match next.journal.expanded {
                    Some(row) if row == next.journal.selected => None,
                    _ => Some(next.journal.selected),
                };
            }
        }
        KeyCode::Enter => enter(next, commands, next_id),
        _ => {}
    }
    true
}

/// `Esc`/`J`: the browser goes away and the read it asked for, if it is
/// still in flight, is cancelled — nothing on screen is waiting for it.
fn close(next: &mut AppState, commands: &mut Vec<Command>) {
    next.journal.shown = false;
    next.journal.selected = 0;
    next.journal.expanded = None;
    if let Loading::Pending(id) = next.journal.entries {
        commands.push(Command::CancelJob(id));
        next.journal.entries = Loading::Idle;
    }
}

/// `Enter`: `u` on the entry `u` would take back, `U` on the one `U` would
/// put back, and on anything else the M5 line (ADR-0031). Acting on an
/// older entry is a multi-step revert, which needs a Ghost Preview to be
/// safe — a key that half works would be the worst of the three.
fn enter(next: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId) {
    let Loading::Ready(read) = &next.journal.entries else {
        return;
    };
    let Some(selected) = read.entries.len().checked_sub(next.journal.selected + 1) else {
        return;
    };
    let Some(entry) = read.entries.get(selected) else {
        return;
    };
    let id = &entry.id;
    let redo = if graft_ops::journal::next_undo(&read.entries).is_some_and(|at| at.id == *id) {
        false
    } else if graft_ops::journal::next_redo(&read.entries).is_some_and(|at| at.id == *id) {
        true
    } else {
        next.status = Status::Done(OLDER_ENTRY.to_owned());
        return;
    };
    // The result is a change to the graph and the working tree, so the
    // browser gets out of the way to show it.
    close(next, commands);
    commands.extend(super::undo::undo_key(next, redo, next_id));
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use graft_core::journal::{EntryId, HeadRef, JournalEntry, OperationKind, Touches};

    use super::*;

    fn ids() -> impl FnMut() -> JobId {
        let mut next = 0;
        move || {
            next += 1;
            JobId::new(next)
        }
    }

    fn oid(byte: u8) -> graft_core::oid::Oid {
        graft_core::oid::Oid::from_bytes(&[byte; 20]).expect("20 bytes is a sha-1 oid")
    }

    fn entry(at: i64, kind: OperationKind, description: &str) -> JournalEntry {
        JournalEntry {
            id: EntryId::new(at, [0; 10]),
            at_unix_ms: at,
            kind,
            description: description.to_owned(),
            status: graft_core::journal::EntryStatus::Committed,
            touches: Touches {
                refs: Vec::new(),
                index: true,
                worktree: false,
            },
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            head_after: Some(HeadRef::Branch("refs/heads/main".to_owned())),
            refs_before: std::collections::BTreeMap::new(),
            refs_after: std::collections::BTreeMap::new(),
            index_before: Some(oid(1)),
            index_after: Some(oid(2)),
            worktree_before: None,
            worktree_after: None,
            backup_ref: Some("refs/graft/backup/main/x".to_owned()),
            pins: Vec::new(),
            commands: vec!["git add -- src/a.rs".to_owned()],
            reverses: None,
        }
    }

    /// Two entries, a stage then a commit: `u` reaches the commit.
    fn state_with_two() -> AppState {
        let mut state = AppState::default();
        state.journal.entries = Loading::Ready(Arc::new(JournalRead {
            entries: vec![
                entry(1, OperationKind::Stage, "stage src/a.rs"),
                entry(2, OperationKind::Commit, "commit \"x\""),
            ],
            interrupted: Vec::new(),
            unreadable: Vec::new(),
        }));
        state
    }

    fn press(state: &mut AppState, code: KeyCode) -> (bool, Vec<Command>) {
        let mut commands = Vec::new();
        let claimed = journal_key(state, &mut commands, KeyPress::plain(code), &mut ids());
        (claimed, commands)
    }

    #[test]
    fn capital_j_opens_the_browser_and_reads_the_journal_again() {
        let mut state = AppState::default();
        let (claimed, commands) = press(&mut state, KeyCode::Char('J'));
        assert!(claimed);
        assert!(state.journal.shown);
        assert!(matches!(commands.as_slice(), [Command::ReadJournal(_)]));
    }

    #[test]
    fn opening_it_is_what_the_interrupted_notice_asked_for() {
        let mut state = AppState::default();
        assert!(!state.journal.notice_seen);
        press(&mut state, KeyCode::Char('J'));
        assert!(state.journal.notice_seen);
    }

    #[test]
    fn escape_closes_it_and_cancels_a_read_still_in_flight() {
        let mut state = AppState::default();
        press(&mut state, KeyCode::Char('J'));
        let pending = match state.journal.entries {
            Loading::Pending(id) => id,
            ref other => panic!("the open asked for a read: {other:?}"),
        };
        let (claimed, commands) = press(&mut state, KeyCode::Escape);
        assert!(claimed);
        assert!(!state.journal.shown);
        assert_eq!(commands, vec![Command::CancelJob(pending)]);
    }

    #[test]
    fn j_and_k_move_within_the_rows_and_never_past_them() {
        let mut state = state_with_two();
        state.journal.shown = true;
        press(&mut state, KeyCode::Char('j'));
        assert_eq!(state.journal.selected, 1);
        press(&mut state, KeyCode::Char('j'));
        assert_eq!(state.journal.selected, 1, "two rows, no third");
        press(&mut state, KeyCode::Char('k'));
        press(&mut state, KeyCode::Char('k'));
        assert_eq!(state.journal.selected, 0);
    }

    #[test]
    fn space_opens_one_rows_command_log_and_closes_it_again() {
        let mut state = state_with_two();
        state.journal.shown = true;
        press(&mut state, KeyCode::Char(' '));
        assert_eq!(state.journal.expanded, Some(0));
        press(&mut state, KeyCode::Char(' '));
        assert_eq!(state.journal.expanded, None);
        press(&mut state, KeyCode::Char('j'));
        press(&mut state, KeyCode::Char(' '));
        assert_eq!(state.journal.expanded, Some(1), "one at a time, by row");
    }

    #[test]
    fn enter_on_the_next_undo_row_is_u_and_closes_the_browser() {
        let mut state = state_with_two();
        state.journal.shown = true;
        let (claimed, commands) = press(&mut state, KeyCode::Enter);
        assert!(claimed);
        assert!(!state.journal.shown, "the result is on the graph");
        assert!(
            matches!(commands.as_slice(), [Command::Undo(_, false)]),
            "{commands:?}"
        );
        assert!(state.operating.is_some());
    }

    #[test]
    fn enter_on_an_older_row_says_the_time_machine_is_m5s_and_asks_for_nothing() {
        let mut state = state_with_two();
        state.journal.shown = true;
        state.journal.selected = 1;
        let (claimed, commands) = press(&mut state, KeyCode::Enter);
        assert!(claimed);
        assert!(commands.is_empty(), "{commands:?}");
        assert_eq!(state.status, Status::Done(OLDER_ENTRY.to_owned()));
        assert!(state.journal.shown, "the browser stays open to say so");
        assert!(state.operating.is_none());
    }

    #[test]
    fn an_open_browser_swallows_the_graphs_own_keys() {
        let mut state = state_with_two();
        state.journal.shown = true;
        let (claimed, commands) = press(&mut state, KeyCode::Char('q'));
        assert!(claimed, "the graph must not quit under an open overlay");
        assert!(commands.is_empty());
        assert!(state.quit.is_none());
    }
}
