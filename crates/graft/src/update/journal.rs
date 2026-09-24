//! The undo journal's state: what `J` shows, and the read that keeps it
//! current (m3-working-tree-undo-and-stash/06).
//!
//! Pure, like `update::map`, whose open/close/key/event shape this copies.
//! The journal itself is a file, so reading it is a job; what lives here is
//! which entry the cursor is on, which one is expanded, and whether the
//! interrupted-entry notice has been looked at.

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::journal::OperationKind;

use super::detail::Loading;
use super::events::{Command, Product};
use super::state::AppState;
use crate::jobs::undo::JournalRead;

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
    /// The read in flight, if any. Held apart from [`JournalState::entries`]
    /// so a re-read does not blank the list it is refreshing — the marks and
    /// the interrupted notice are drawn from that list, and they must not
    /// blink off for the duration of every `Ctrl+R` (m3/06 review, Major 1).
    pub reading: Option<JobId>,
    /// Interrupted entries whose after-state could not be read, and why,
    /// **accumulated across reads**. Only the read that recovered an entry
    /// can report it, so a later read that recovers nothing must not erase
    /// the warning.
    pub unreadable: Vec<String>,
}

impl JournalState {
    /// Ask for the journal, under a fresh id. Idempotent in the sense that
    /// matters: a second read simply replaces the first, and the older
    /// answer is dropped by id.
    pub(super) fn read(&mut self, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        let id = next_id();
        self.reading = Some(id);
        // A list already on hand stays on screen while the fresh one is
        // fetched: the browser's marks and the interrupted notice come from
        // it, and a journal read happens after every write.
        if !matches!(self.entries, Loading::Ready(_)) {
            self.entries = Loading::Pending(id);
        }
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
pub(super) fn journal_event(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    product: &Product,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Product::Journal(result) = product else {
        return false;
    };
    if state.journal.reading != Some(id) {
        return true;
    }
    // Read before the assignment below: the launch read is the one that
    // finds `entries` anything but `Ready`. See `update::relaunch::journal_read_landed`.
    let first_read = !matches!(state.journal.entries, Loading::Ready(_));
    state.journal.reading = None;
    if let Ok(read) = result {
        // Only a successful read knows what is live. A failed one is left
        // holding the last answer rather than silently claiming "nothing is
        // live", which would put the interrupted notice back on a merge that
        // is standing right there.
        state.live_entries.clone_from(&read.live);
        // Merged, not replaced: see `JournalState::unreadable`.
        for line in &read.unreadable {
            if !state.journal.unreadable.contains(line) {
                state.journal.unreadable.push(line.clone());
            }
        }
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
    if first_read {
        super::relaunch::journal_read_landed(state, commands, next_id);
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
    if key.code == KeyCode::Char('J') && next.graph_has_focus() {
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

/// `Esc`/`J`: the browser goes away, and a read it asked for that is only
/// refreshing a list already on hand goes with it — nothing on screen is
/// waiting for that one.
///
/// A read with **nothing** behind it stays. It is the launch read, and the
/// footer's interrupted-entry notice and `u` are both waiting on it whether
/// the browser is open or not; cancelling it used to blank the field back to
/// `Idle`, which is exactly the condition [`request`] re-reads on, so `settle`
/// asked for the same read again in the same pass. That churn is
/// `KNOWN_GAPS.md` row 217, and the cause is here rather than in `settle`:
/// `Esc` was stopping work the application still wanted
/// (m5-history-and-ghost-preview/05).
fn close(next: &mut AppState, commands: &mut Vec<Command>) {
    next.journal.shown = false;
    next.journal.selected = 0;
    next.journal.expanded = None;
    if let Some(id) = next.journal.reading
        && next.journal.entries != Loading::Pending(id)
    {
        next.journal.reading = None;
        commands.push(Command::CancelJob(id));
    }
}

/// `Enter`: `u` on the entry `u` would take back, `U` on the one `U` would
/// put back, and on anything older **the time machine** — revert to just
/// before that entry (`docs/design/02-keymap.md` §2.11,
/// `docs/design/06-undo-journal-design.md` §6.6).
///
/// The time machine is several restores in a row, so it is the one thing
/// here that never runs from this key: it opens a Ghost Preview and the
/// preview's `Confirm` runs it (ADR-0031, PROMPT.md §1.5). The browser is
/// **hidden, not closed** — `selected` is left where it is, so `Esc` on the
/// preview puts the browser back on the same row
/// (`preview::close_preview`'s `return_to_journal`).
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
    let id = entry.id.clone();
    let kind = entry.kind;
    let redo = if graft_ops::journal::next_undo(&read.entries).is_some_and(|at| at.id == id) {
        false
    } else if graft_ops::journal::next_redo(&read.entries).is_some_and(|at| at.id == id) {
        true
    } else {
        // The time machine aims at an **operation**. An `Undo` or `Redo`
        // row is a move along the journal, not a state the repository was
        // in, and `plan_revert_to` reaches its answer by inverting the
        // operations in force since the target — it has no way to put back
        // what an undo took away. Aimed at `undo(B)` in `[A, B, undo(B),
        // C]` it plans `inverse(C)` alone and leaves `B` undone, while the
        // box's title reads `revert to before "undo: B"`. Refusing in
        // words is the honest answer: the row you pick is the state you get
        // (review 08 Major 2), or no row is picked at all.
        let step = match kind {
            OperationKind::Undo => Some("an undo"),
            OperationKind::Redo => Some("a redo"),
            _ => None,
        };
        if let Some(step) = step {
            next.status = super::state::Status::sticky(format!(
                "the time machine reverts to before an operation · this row is {step}"
            ));
            return;
        }
        if super::preview::open_preview(
            next,
            commands,
            super::preview::PreviewSubject::RevertTo(id),
            true,
            next_id,
        ) {
            next.journal.shown = false;
        }
        // A refusal left its own sentence on the status line and the
        // browser stays where it is; there is nothing else to say.
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

    use graft_core::journal::{EntryId, HeadRef, IndexState, JournalEntry, Touches};

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
            index_before: Some(IndexState::Merged(oid(1))),
            index_after: Some(IndexState::Merged(oid(2))),
            worktree_before: None,
            worktree_after: None,
            backup_ref: Some("refs/graft/backup/main/x".to_owned()),
            pins: Vec::new(),
            commands: vec!["git add -- src/a.rs".to_owned()],
            reverses: None,
            merge_before: None,
            merge_after: None,
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
            unreadable: Vec::new(),
            live: Vec::new(),
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
    fn escape_closes_it_and_cancels_a_read_that_was_only_refreshing_the_list() {
        let mut state = state_with_two();
        press(&mut state, KeyCode::Char('J'));
        let refreshing = state
            .journal
            .reading
            .expect("the open asked for a fresh read");
        let (claimed, commands) = press(&mut state, KeyCode::Escape);
        assert!(claimed);
        assert!(!state.journal.shown);
        assert_eq!(commands, vec![Command::CancelJob(refreshing)]);
        assert_eq!(state.journal.reading, None);
    }

    /// `KNOWN_GAPS.md` row 217: `Esc` on a read nothing is standing on used
    /// to cancel it and blank the field, which is the one condition
    /// [`request`] re-reads on — so the same read went straight back out.
    ///
    /// The read itself is what the footer's interrupted-entry notice and `u`
    /// are waiting for, so the cure is to leave it running, not to stop
    /// `settle` from asking.
    #[test]
    fn esc_on_a_pending_journal_read_does_not_rerequest() {
        let mut state = AppState {
            repository: Some(crate::repository::RepositoryFacts {
                path: std::path::PathBuf::from("/code/api"),
                bare: false,
                head: crate::repository::Head::Branch(b"refs/heads/main".to_vec()),
                mailmap: graft_git::mailmap::Mailmap::default(),
                commit_graph: crate::repository::CommitGraphFact::Present { commits: 1 },
            }),
            ..AppState::default()
        };
        press(&mut state, KeyCode::Char('J'));
        let launch_read = match state.journal.entries {
            Loading::Pending(id) => id,
            ref other => panic!("the open asked for a read with nothing behind it: {other:?}"),
        };

        let (claimed, on_escape) = press(&mut state, KeyCode::Escape);
        // And then the tail of `update`, which is where the re-request was.
        let mut settled = Vec::new();
        request(&mut state, &mut settled, &mut ids());

        assert!(claimed);
        assert!(!state.journal.shown, "`Esc` closes the browser");
        assert!(
            on_escape.is_empty(),
            "the read the notice is waiting for is left alone: {on_escape:?}"
        );
        assert!(
            settled.is_empty(),
            "and nothing asks for it a second time: {settled:?}"
        );
        assert_eq!(state.journal.reading, Some(launch_read));
        assert_eq!(state.journal.entries, Loading::Pending(launch_read));
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

    /// The time machine: `Enter` on an older row previews, and **only**
    /// previews. Nothing is launched from this key — `state.operating` is
    /// still free — and the plan job is the only command that goes out.
    #[test]
    fn enter_on_an_older_row_opens_a_preview_of_reverting_to_it() {
        let mut state = state_with_two();
        state.journal.shown = true;
        state.journal.selected = 1;
        let older = entry(1, OperationKind::Stage, "stage src/a.rs").id;
        let (claimed, commands) = press(&mut state, KeyCode::Enter);
        assert!(claimed);
        let preview = state.preview.as_ref().expect("the preview is open");
        assert_eq!(
            preview.subject,
            crate::update::preview::PreviewSubject::RevertTo(older),
            "the preview is of the row under the cursor, not of `u`'s entry"
        );
        assert!(preview.return_to_journal, "esc must put the browser back");
        assert!(
            matches!(commands.as_slice(), [Command::PlanPreview(_, _)]),
            "a plan and nothing else: {commands:?}"
        );
        assert!(
            state.operating.is_none(),
            "the time machine runs from Confirm, never from this key"
        );
    }

    /// Review 08 Major 2. `[A, B, undo(B), C]`, cursor on `undo(B)`: that
    /// row is neither `u`'s nor `U`'s, so it used to open a time machine
    /// titled `revert to before "undo: B"` whose plan was `inverse(C)`
    /// alone — a picture of a state with `B` in force, and an act that
    /// leaves `B` undone. The row you pick has to be the state you get, so
    /// the key refuses in words instead.
    #[test]
    fn enter_on_an_undo_row_refuses_rather_than_reverting_somewhere_else() {
        let undone = entry(2, OperationKind::Commit, "commit \"x\"").id;
        let mut undo_of_b = entry(3, OperationKind::Undo, "undo: commit \"x\"");
        undo_of_b.reverses = Some(undone);
        let mut state = AppState::default();
        state.journal.entries = Loading::Ready(Arc::new(JournalRead {
            entries: vec![
                entry(1, OperationKind::Stage, "stage src/a.rs"),
                entry(2, OperationKind::Commit, "commit \"x\""),
                undo_of_b,
                entry(4, OperationKind::Stage, "stage src/b.rs"),
            ],
            unreadable: Vec::new(),
            live: Vec::new(),
        }));
        state.journal.shown = true;
        // Newest first: [C, undo(B), B, A].
        state.journal.selected = 1;

        let (claimed, commands) = press(&mut state, KeyCode::Enter);

        assert!(claimed);
        assert!(
            state.preview.is_none(),
            "no picture is drawn of a state this key cannot reach"
        );
        assert!(commands.is_empty(), "nothing is planned: {commands:?}");
        assert!(state.journal.shown, "the browser stays where it is");
        assert_eq!(
            state.status,
            crate::update::state::Status::sticky(
                "the time machine reverts to before an operation · this row is an undo"
            ),
            "and it says why"
        );
    }

    /// The same row when it *is* `U`'s: `Enter` redoes, as it always did.
    /// The refusal above must not swallow the one undo row that has a
    /// meaning.
    #[test]
    fn enter_on_the_next_redo_row_is_still_capital_u() {
        let undone = entry(2, OperationKind::Commit, "commit \"x\"").id;
        let mut undo_of_b = entry(3, OperationKind::Undo, "undo: commit \"x\"");
        undo_of_b.reverses = Some(undone);
        let mut state = AppState::default();
        state.journal.entries = Loading::Ready(Arc::new(JournalRead {
            entries: vec![
                entry(1, OperationKind::Stage, "stage src/a.rs"),
                entry(2, OperationKind::Commit, "commit \"x\""),
                undo_of_b,
            ],
            unreadable: Vec::new(),
            live: Vec::new(),
        }));
        state.journal.shown = true;
        state.journal.selected = 0;

        let (claimed, commands) = press(&mut state, KeyCode::Enter);

        assert!(claimed);
        assert!(
            matches!(commands.as_slice(), [Command::Redo(_, _)]),
            "a redo, not a refusal: {commands:?}"
        );
    }

    /// Hidden, not closed: `Esc` on the preview reopens the browser on the
    /// very row that opened it.
    #[test]
    fn the_browser_comes_back_on_the_same_row_when_the_preview_is_cancelled() {
        let mut state = state_with_two();
        state.journal.shown = true;
        state.journal.selected = 1;
        press(&mut state, KeyCode::Enter);
        assert!(!state.journal.shown, "the preview has the screen");
        let mut commands = Vec::new();
        crate::update::preview::close_preview(&mut state, &mut commands);
        assert!(state.journal.shown);
        assert_eq!(state.journal.selected, 1, "the same row, not the newest");
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
