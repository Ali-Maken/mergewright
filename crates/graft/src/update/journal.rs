//! The undo journal's state: what `J` shows, and the read that keeps it
//! current (m3-working-tree-undo-and-stash/06).
//!
//! Pure, like `update::map`, whose open/close/key/event shape this copies.
//! The journal itself is a file, so reading it is a job; what lives here is
//! which row the cursor is on, which one is expanded, and whether the
//! interrupted-entry notice has been looked at.
//!
//! Since m7-power-tools/24 the rows are `graft_ops::journal_list`'s list —
//! Graft's own steps (`●`), the shell's (`$`) and what nothing holds any
//! more (`✗`) — built again each time one of its three reads lands
//! ([`rebuild`]). The cursor is a [`RowKey`], not an index, so it stays on
//! its row while rows arrive above it.

use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::journal::{EntryId, OperationKind};
use graft_core::oid::Oid;
use graft_ops::journal_list::{JournalList, ListRow, LostScan, OutsideRecord, Recover};
use graft_ops::operation::Operation;

use super::detail::Loading;
use super::events::{Command, Product};
use super::state::{AppState, Status};
use crate::jobs::journal_list::LostFound;
use crate::jobs::undo::JournalRead;

/// A row of J's list by what it stands for, so the cursor keeps its row
/// when the list is built again with more rows in it (note J1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RowKey {
    /// A journal entry.
    Graft(EntryId),
    /// A step the shell took: its reflog record's own stable fields (V5).
    /// `old` is in it because two records of one ref in one second can
    /// share `new`.
    Outside {
        /// The ref the record is in.
        refname: String,
        /// Its old value.
        old: Option<Oid>,
        /// Its new value.
        new: Option<Oid>,
        /// When, in seconds.
        at: i64,
    },
    /// A lost run, by its tip.
    Lost(Oid),
    /// A lost edit, by the undo that replaced it.
    LostEdit(EntryId),
}

/// The key of `list.rows[index]`.
pub(crate) fn row_key(list: &JournalList, index: usize) -> Option<RowKey> {
    Some(match list.rows.get(index)? {
        ListRow::Graft(id) => RowKey::Graft(id.clone()),
        ListRow::Outside(step) => RowKey::Outside {
            refname: step.record.refname.clone(),
            old: step.record.old,
            new: step.record.new,
            at: step.record.at,
        },
        ListRow::Lost { commit, .. } => RowKey::Lost(commit.run.tip),
        ListRow::LostEdit { edit, .. } => RowKey::LostEdit(edit.entry.clone()),
    })
}

/// Where the row `key` stands in `list`, if it is there.
pub(crate) fn index_of(list: &JournalList, key: &RowKey) -> Option<usize> {
    (0..list.rows.len()).find(|&index| row_key(list, index).as_ref() == Some(key))
}

/// The commit `b` names on a row: ✗ its tip; $ `new`, else `old`; ● and a
/// lost edit none (the list alone names no commit for a journal entry).
pub(crate) fn row_commit(list: &JournalList, index: usize) -> Option<Oid> {
    match list.rows.get(index)? {
        ListRow::Lost { commit, .. } => Some(commit.run.tip),
        ListRow::Outside(step) => step.record.new.or(step.record.old),
        ListRow::Graft(_) | ListRow::LostEdit { .. } => None,
    }
}

/// The row the cursor starts on once the lost commits are in, when no key
/// has been pressed since `J` (planner's ruling 2026-09-26): the newest `✗`
/// row under a `$` row, when that `$` row is newer than every `●` row but a
/// fetch. `None` leaves the cursor on the newest row.
///
/// A lost edit (always under a `●`) and a commit lost under a `●` are never
/// chosen; `n` reaches them. Pure: no state, no terminal.
pub(crate) fn preselect(list: &JournalList, is_fetch: impl Fn(EntryId) -> bool) -> Option<usize> {
    for (index, row) in list.rows.iter().enumerate() {
        match row {
            // Graft worked since: the shell's loss is not the news.
            ListRow::Graft(id) if !is_fetch(id.clone()) => return None,
            ListRow::Lost { event, .. }
                if matches!(list.rows.get(*event), Some(ListRow::Outside(_))) =>
            {
                return Some(index);
            }
            ListRow::Graft(_)
            | ListRow::Outside(_)
            | ListRow::Lost { .. }
            | ListRow::LostEdit { .. } => {}
        }
    }
    None
}

/// The journal browser's own state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JournalState {
    /// `J` opened it.
    pub shown: bool,
    /// The row under the cursor; `None` is the newest row.
    pub(crate) cursor: Option<RowKey>,
    /// The row whose Command Log is open, if any. One at a time: the log is
    /// several lines and two open at once pushes the cursor off screen.
    pub(crate) expanded: Option<RowKey>,
    /// A key was pressed since `J`: the lost rows' preselect is off.
    pub(crate) touched: bool,
    /// The interrupted-entry notice has been looked at, so the footer stops
    /// saying so. One press of `J` is enough.
    pub notice_seen: bool,
    /// The journal as it was last read.
    pub entries: Loading<Arc<JournalRead>>,
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
    /// Stage 2: git's reflog records, read once per `J`.
    pub(crate) outside: Loading<Arc<Vec<OutsideRecord>>>,
    /// Stage 3: the lost commits and edits, found after stage 2.
    pub(crate) lost: Loading<Arc<LostFound>>,
    /// The list as last built from what has landed ([`rebuild`]).
    pub(crate) list: Option<Arc<JournalList>>,
    /// The name the last `⏎` offered on a row (`OfferBranch`): `b` on that
    /// row prefills it.
    pub(crate) offered: Option<(RowKey, String)>,
    /// A `⏎` asking what a row puts back, and the row it asked for. An
    /// answer under any other id is stale.
    pub(crate) recovering: Option<(JobId, RowKey)>,
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

    /// The cursor's index in the list; 0 (the newest) when it names none.
    #[must_use]
    pub(crate) fn selected(&self) -> usize {
        match (&self.list, &self.cursor) {
            (Some(list), Some(key)) => index_of(list, key).unwrap_or(0),
            _ => 0,
        }
    }

    /// The cursor as an index into [`JournalState::newest_first`]: the
    /// cursor's `●` row, or the nearest `●` above it. What the browser's
    /// drawing of the `●` rows alone selects (m3/06; 24-b draws the rest).
    #[must_use]
    pub fn selected_graft_row(&self) -> usize {
        let Some(list) = &self.list else {
            return 0;
        };
        let upto = self.selected().min(list.rows.len().saturating_sub(1));
        list.rows
            .iter()
            .take(upto + 1)
            .filter(|row| matches!(row, ListRow::Graft(_)))
            .count()
            .saturating_sub(1)
    }

    /// Whether entry `id`'s Command Log is open.
    #[must_use]
    pub fn is_expanded(&self, id: &EntryId) -> bool {
        matches!(&self.expanded, Some(RowKey::Graft(open)) if open == id)
    }

    /// Stage 2 or stage 3 is still out: the title says `finding lost
    /// commits…` while this holds (J1).
    #[must_use]
    pub fn finding_lost(&self) -> bool {
        matches!(self.outside, Loading::Pending(_)) || matches!(self.lost, Loading::Pending(_))
    }

    fn rows(&self) -> usize {
        self.list.as_ref().map_or(0, |list| list.rows.len())
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

/// The local branch names on hand. No refs read yet is no branch at all,
/// which reads every branch as gone; the lost rows need two reads that land
/// after the launch's refs read, so that case is not reached on the way
/// through `J`.
fn local_branches(state: &AppState) -> std::collections::HashSet<Vec<u8>> {
    state
        .refs
        .as_deref()
        .map(|refs| {
            refs.branches()
                .map(|(_, name, _, _)| name.to_vec())
                .collect()
        })
        .unwrap_or_default()
}

/// Builds J's list again from what has landed: the journal (stage 1), the
/// reflog records (stage 2), the lost commits and edits (stage 3).
///
/// The cursor keeps its [`RowKey`]; a key that is gone falls to the newest
/// row. Once stage 3 is in, and no key was pressed since `J`, the cursor
/// goes where [`preselect`] says.
pub(crate) fn rebuild(state: &mut AppState) {
    let Loading::Ready(read) = &state.journal.entries else {
        state.journal.list = None;
        return;
    };
    let outside: &[OutsideRecord] = match &state.journal.outside {
        Loading::Ready(records) => records,
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => &[],
    };
    let found = match &state.journal.lost {
        Loading::Ready(found) => Some(Arc::clone(found)),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
    };
    let scan = match (&state.journal.lost, &found) {
        (_, Some(found)) => found.scan.clone(),
        (Loading::Failed(why), None) => LostScan::Failed(why.clone()),
        // Not asked yet, or still out: the list has no lost rows so far.
        _ => LostScan::Running,
    };
    // Whether a lost commit's branch is gone is asked of the refs on hand.
    let branches = local_branches(state);
    let subject_of = |tip: Oid| found.as_ref().and_then(|found| found.subject_of(tip));
    let branch_exists = |name: &str| branches.contains(name.as_bytes());
    let list = graft_ops::journal_list::build(
        &read.entries,
        outside,
        found.as_ref().map(|found| &found.unreached),
        &subject_of,
        &branch_exists,
        found
            .as_ref()
            .map_or(&[][..], |found| found.edits.as_slice()),
        scan,
    );
    let preselected = if found.is_some() && !state.journal.touched {
        let entries = &read.entries;
        preselect(&list, |id| {
            entries
                .iter()
                .any(|entry| entry.id == id && entry.kind == OperationKind::Fetch)
        })
    } else {
        None
    };
    let journal = &mut state.journal;
    if let Some(index) = preselected {
        journal.cursor = row_key(&list, index);
    } else if let Some(key) = &journal.cursor
        && index_of(&list, key).is_none()
    {
        journal.cursor = row_key(&list, 0);
    }
    if let Some(key) = &journal.expanded
        && index_of(&list, key).is_none()
    {
        journal.expanded = None;
    }
    journal.list = Some(Arc::new(list));
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
        // Merged too: only the read that closed an entry knows where its
        // edits were (`AppState::interrupted_edits`).
        for edits in &read.interrupted_edits {
            if !state.interrupted_edits.iter().any(|(id, _)| *id == edits.0) {
                state.interrupted_edits.push(edits.clone());
            }
        }
    }
    state.journal.entries = match result {
        Ok(read) => Loading::Ready(Arc::clone(read)),
        Err(why) => Loading::Failed(why.clone()),
    };
    // Stage 1: the `●` rows. A shorter journal must not leave the cursor
    // past its end; `rebuild` keeps it on its row or falls to the newest.
    rebuild(state);
    if first_read {
        super::relaunch::journal_read_landed(state, commands, next_id);
    }
    // Whether a stash put back is still clashing is read out of the journal
    // (ADR-0045 §2, m6-conflicts/72b).
    super::clash::refresh_and_land(state, commands, next_id);
    true
}

/// Stage 2 landed: git's reflog records. The `$` rows join the list; the
/// cursor keeps its row. Stale answers are dropped by id.
pub(crate) fn outside_landed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    result: &Result<Arc<Vec<OutsideRecord>>, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.journal.outside != Loading::Pending(id) {
        return;
    }
    match result {
        Ok(records) => {
            state.journal.outside = Loading::Ready(Arc::clone(records));
            // Stage 3 looks for what nothing holds among these records' old
            // values too, so it waits for them.
            let id = next_id();
            state.journal.lost = Loading::Pending(id);
            commands.push(Command::FindLost {
                id,
                outside: Arc::clone(records),
            });
        }
        Err(why) => {
            state.journal.outside = Loading::Failed(why.clone());
            // No scan over records never read: the title says why there
            // are no lost rows rather than claiming there are none.
            state.journal.lost = Loading::Failed(format!("couldn't read git's own records: {why}"));
        }
    }
    rebuild(state);
}

/// Stage 3 landed: the lost commits and edits. The `✗` rows join the list
/// under their events, and the preselect runs (ruling 2026-09-26). A failed
/// scan is said in the title (`LostScan::Failed`), never dropped.
pub(crate) fn lost_landed(
    state: &mut AppState,
    id: JobId,
    result: &Result<Arc<LostFound>, String>,
) {
    if state.journal.lost != Loading::Pending(id) {
        return;
    }
    state.journal.lost = match result {
        Ok(found) => Loading::Ready(Arc::clone(found)),
        Err(why) => Loading::Failed(why.clone()),
    };
    rebuild(state);
}

/// `recover_for`'s answer to a `⏎` on a `✗`, `$` or lost-edit row.
///
/// `Run` opens the put-back's preview **with J left shown** under it, so
/// `Esc` is J on the same row (V3); `y` confirms (ADR-0044 §1).
/// `OfferBranch` opens nothing: the status says why and what `b` makes, and
/// `b` on the row prefills the name. A refusal (`Err`, already worded) is
/// the status line's.
pub(crate) fn recover_answered(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    result: &Result<Recover, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(row) = state
        .journal
        .recovering
        .take_if(|(asked, _)| *asked == id)
        .map(|(_, row)| row)
    else {
        return;
    };
    match result {
        Ok(Recover::Run(params)) => {
            // A refusal to open left its own sentence on the status line,
            // and J stays where it is.
            let _opened: bool = super::preview::open_preview(
                state,
                commands,
                super::preview::PreviewSubject::Operation {
                    op: Box::new(Operation::Recover(params.clone())),
                    opened_by: "\u{23ce}",
                },
                false,
                next_id,
            );
        }
        Ok(Recover::OfferBranch { at, name, why }) => {
            state.status = Status::Sticky {
                reason: why.clone(),
                keys: Some(format!("b makes {name} at {}", super::place::short(*at))),
            };
            state.journal.offered = Some((row, name.clone()));
        }
        Err(why) => state.status = Status::sticky(why.clone()),
    }
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
    // From the graph or from the changes pane, as `c` and `p` are: a
    // refusal that names `J` must name a key that works where he is
    // (m6-conflicts/85 m2). The overlay stands where the graph does and
    // takes the keys the graph would, so the keyboard goes to the graph.
    if key.code == KeyCode::Char('J') {
        commands.extend(journal_enter(next, next_id));
        return true;
    }
    false
}

/// `J`: the browser opens at once on what is on hand — stage 1, the `●`
/// rows, on the newest — the journal is read again, and stage 2 (git's
/// reflog records) is asked; its answer asks stage 3 ([`outside_landed`]).
pub(crate) fn journal_enter(
    state: &mut AppState,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    state.detail.focus = graft_ui::detail::DetailFocus::Graph;
    let journal = &mut state.journal;
    journal.shown = true;
    journal.cursor = None;
    journal.expanded = None;
    journal.touched = false;
    journal.offered = None;
    journal.recovering = None;
    journal.outside = Loading::Idle;
    journal.lost = Loading::Idle;
    // Looking at the browser is what the interrupted-entry notice asks
    // for, so the notice stops once it has been taken up.
    journal.notice_seen = true;
    rebuild(state);
    // Read it fresh: another Graft on the same worktree may have appended
    // since the last read.
    let mut commands = state.journal.read(next_id);
    let id = next_id();
    state.journal.outside = Loading::Pending(id);
    commands.push(Command::ReadOutside { id });
    commands
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
    // Any key since `J` means he has chosen where to look: the lost rows'
    // preselect must not move the cursor out from under him.
    next.journal.touched = true;
    match key.code {
        KeyCode::Char('J') | KeyCode::Escape => {
            close(next, commands);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let above = next.journal.selected().checked_sub(1);
            move_to(next, above);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let below = next.journal.selected().saturating_add(1);
            let below = (below < next.journal.rows()).then_some(below);
            move_to(next, below);
        }
        KeyCode::Char('n') => next_lost(next, true),
        KeyCode::Char('N') => next_lost(next, false),
        KeyCode::Char(' ') => {
            if let Some(list) = &next.journal.list {
                let key = row_key(list, next.journal.selected());
                next.journal.expanded = if next.journal.expanded == key {
                    None
                } else {
                    key
                };
            }
        }
        KeyCode::Char('b') => branch_here(next),
        KeyCode::Enter => enter(next, commands, next_id),
        _ => {}
    }
    true
}

/// The cursor to row `index`, when there is one.
fn move_to(next: &mut AppState, index: Option<usize>) {
    if let (Some(list), Some(index)) = (&next.journal.list, index)
        && let Some(key) = row_key(list, index)
    {
        next.journal.cursor = Some(key);
    }
}

/// `n`/`N` with no find active: the next (older) or previous (newer) `✗`
/// row, lost edits included. Nowhere to go stays put.
fn next_lost(next: &mut AppState, older: bool) {
    let Some(list) = &next.journal.list else {
        return;
    };
    let from = next.journal.selected();
    let is_lost = |index: &usize| {
        matches!(
            list.rows.get(*index),
            Some(ListRow::Lost { .. } | ListRow::LostEdit { .. })
        )
    };
    let found = if older {
        (from + 1..list.rows.len()).find(is_lost)
    } else {
        (0..from).rev().find(is_lost)
    };
    move_to(next, found);
}

/// `b`: the name prompt for a new branch at the row's commit, J left shown
/// under it (V1: as the stash's `b` leaves S). The name is prefilled when
/// the last `⏎` on this row offered one.
fn branch_here(next: &mut AppState) {
    let Some(list) = next.journal.list.clone() else {
        return;
    };
    let index = next.journal.selected();
    if matches!(list.rows.get(index), Some(ListRow::LostEdit { .. })) {
        next.status = Status::Sticky {
            reason: "a lost edit has no commit".to_owned(),
            keys: Some("\u{23ce} puts it back".to_owned()),
        };
        return;
    }
    let Some(at) = row_commit(&list, index) else {
        next.status = Status::sticky("this step names no commit");
        return;
    };
    let key = row_key(&list, index);
    let prefill = next
        .journal
        .offered
        .as_ref()
        .filter(|(offered, _)| Some(offered) == key.as_ref())
        .map_or("", |(_, name)| name.as_str())
        .to_owned();
    // `false`: something modal is up, and it owns the keyboard — nothing
    // this key could say would be seen under it.
    let _opened: bool = super::name_prompt::open_name_prompt_with(
        next,
        super::name_prompt::NameKind::Branch {
            at,
            checkout: false,
        },
        &prefill,
    );
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
pub(super) fn close(next: &mut AppState, commands: &mut Vec<Command>) {
    let journal = &mut next.journal;
    journal.shown = false;
    journal.cursor = None;
    journal.expanded = None;
    journal.offered = None;
    // The list's own reads are for the open browser only.
    if let Loading::Pending(id) = journal.outside {
        commands.push(Command::CancelJob(id));
    }
    if let Loading::Pending(id) = journal.lost {
        commands.push(Command::CancelJob(id));
    }
    journal.outside = Loading::Idle;
    journal.lost = Loading::Idle;
    if let Some((id, _)) = journal.recovering.take() {
        commands.push(Command::CancelJob(id));
    }
    if let Some(id) = journal.reading
        && journal.entries != Loading::Pending(id)
    {
        journal.reading = None;
        commands.push(Command::CancelJob(id));
    }
}

/// `Enter`, by the row under the cursor (keymap §2.11, 2b):
/// - `●`: `u` on the entry `u` would take back, `U` on the one `U` would
///   put back, and on anything older the time machine ([`enter_graft`]).
/// - `$` switch: switch back to the branch it left (the M7 switch rule).
/// - any other `$`, `✗` or lost edit: ask what it puts back
///   (`recover_for`), answered in [`recover_answered`].
fn enter(next: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId) {
    let Some(list) = next.journal.list.clone() else {
        return;
    };
    let index = next.journal.selected();
    match list.rows.get(index) {
        Some(ListRow::Graft(id)) => enter_graft(next, commands, id.clone(), next_id),
        Some(ListRow::Outside(step)) if switched_from(&step.record.subject).is_some() => {
            let Some(target) = switched_from(&step.record.subject) else {
                return;
            };
            if super::switch::switch_into(next, commands, target, "\u{23ce}", next_id) {
                // The result is on the graph.
                close(next, commands);
            }
        }
        // What it puts back needs the refs read fresh, off the loop; the
        // answer lands in `recover_answered`. A second `⏎` replaces the
        // first, whose answer is then stale by id.
        Some(ListRow::Outside(_) | ListRow::Lost { .. } | ListRow::LostEdit { .. }) => {
            let Some(key) = row_key(&list, index) else {
                return;
            };
            if let Some((stale, _)) = next.journal.recovering.take() {
                commands.push(Command::CancelJob(stale));
            }
            let id = next_id();
            next.journal.recovering = Some((id, key));
            commands.push(Command::RecoverFor {
                id,
                list,
                row: index,
            });
        }
        None => {}
    }
}

/// The branch a shell switch left, as a switch target: `checkout: moving
/// from main to spike` → `main`; a detached start is its commit.
fn switched_from(subject: &str) -> Option<super::switch::SwitchTarget> {
    let (from, _) = subject
        .strip_prefix("checkout: moving from ")?
        .split_once(" to ")?;
    let hex = (from.len() == 40 || from.len() == 64) && from.bytes().all(|b| b.is_ascii_hexdigit());
    Some(if hex {
        super::switch::SwitchTarget::Detached(Oid::from_hex(from).ok()?)
    } else {
        super::switch::SwitchTarget::Branch(format!("refs/heads/{from}"))
    })
}

/// `Enter` on a `●` row: `u`, `U` or **the time machine** — revert to just
/// before that entry (`docs/design/02-keymap.md` §2.11,
/// `docs/design/06-undo-journal-design.md` §6.6).
///
/// The time machine is several restores in a row, so it is the one thing
/// here that never runs from this key: it opens a Ghost Preview and the
/// preview's `Confirm` runs it (ADR-0031, PROMPT.md §1.5). The browser is
/// **hidden, not closed** — the cursor is left where it is, so `Esc` on the
/// preview puts the browser back on the same row
/// (`preview::close_preview`'s `return_to_journal`).
fn enter_graft(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: EntryId,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Loading::Ready(read) = &next.journal.entries else {
        return;
    };
    let Some(entry) = read.entries.iter().find(|entry| entry.id == id) else {
        return;
    };
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
            next.status = Status::sticky(format!(
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
mod tests;
