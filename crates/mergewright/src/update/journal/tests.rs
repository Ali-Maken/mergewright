//! J's state and keys (m3/06, m7-power-tools/24).

use std::sync::Arc;

use mergewright_core::journal::{EntryId, HeadRef, IndexState, JournalEntry, Touches};

use super::*;

fn ids() -> impl FnMut() -> JobId {
    let mut next = 0;
    move || {
        next += 1;
        JobId::new(next)
    }
}

fn oid(byte: u8) -> mergewright_core::oid::Oid {
    mergewright_core::oid::Oid::from_bytes(&[byte; 20]).expect("20 bytes is a sha-1 oid")
}

fn entry(at: i64, kind: OperationKind, description: &str) -> JournalEntry {
    JournalEntry {
        id: EntryId::new(at, [0; 10]),
        at_unix_ms: at,
        kind,
        description: description.to_owned(),
        status: mergewright_core::journal::EntryStatus::Committed,
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
        worktree_raw_before: None,
        backup_ref: Some("refs/graft/backup/main/x".to_owned()),
        pins: Vec::new(),
        commands: vec!["git add -- src/a.rs".to_owned()],
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: std::collections::BTreeMap::new(),
        config_after: std::collections::BTreeMap::new(),
        put_back: None,
        fetched_refs: None,
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
        held: Vec::new(),
        live: Vec::new(),
        interrupted_edits: Vec::new(),
    }));
    rebuild(&mut state);
    state
}

/// The cursor on list row `index`, the list built from what is on hand.
fn put_cursor(state: &mut AppState, index: usize) {
    rebuild(state);
    let list = state.journal.list.clone().expect("the journal is read");
    state.journal.cursor = row_key(&list, index);
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
    assert!(
        matches!(
            commands.as_slice(),
            [Command::ReadJournal(_), Command::ReadOutside { .. }]
        ),
        "the journal again, and stage 2: {commands:?}"
    );
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
    let Loading::Pending(outside) = state.journal.outside else {
        panic!("the open asked for stage 2: {:?}", state.journal.outside);
    };
    let (claimed, commands) = press(&mut state, KeyCode::Escape);
    assert!(claimed);
    assert!(!state.journal.shown);
    assert_eq!(
        commands,
        vec![Command::CancelJob(outside), Command::CancelJob(refreshing)]
    );
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
        repository: Some(Box::new(crate::repository::RepositoryFacts {
            path: std::path::PathBuf::from("/code/api"),
            bare: false,
            head: crate::repository::Head::Branch(b"refs/heads/main".to_vec()),
            head_commit: None,
            mailmap: mergewright_git::mailmap::Mailmap::default(),
            commit_graph: crate::repository::CommitGraphFact::Present { commits: 1 },
        })),
        ..AppState::default()
    };
    press(&mut state, KeyCode::Char('J'));
    let launch_read = match state.journal.entries {
        Loading::Pending(id) => id,
        ref other => panic!("the open asked for a read with nothing behind it: {other:?}"),
    };
    let Loading::Pending(stage_two) = state.journal.outside else {
        panic!("the open asked for stage 2: {:?}", state.journal.outside);
    };

    let (claimed, on_escape) = press(&mut state, KeyCode::Escape);
    // And then the tail of `update`, which is where the re-request was.
    let mut settled = Vec::new();
    request(&mut state, &mut settled, &mut ids());

    assert!(claimed);
    assert!(!state.journal.shown, "`Esc` closes the browser");
    assert_eq!(
        on_escape,
        vec![Command::CancelJob(stage_two)],
        "only the open browser's own read goes; the one the notice is waiting for is left alone"
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
    assert_eq!(state.journal.selected(), 1);
    press(&mut state, KeyCode::Char('j'));
    assert_eq!(state.journal.selected(), 1, "two rows, no third");
    press(&mut state, KeyCode::Char('k'));
    press(&mut state, KeyCode::Char('k'));
    assert_eq!(state.journal.selected(), 0);
}

#[test]
fn space_opens_one_rows_command_log_and_closes_it_again() {
    let mut state = state_with_two();
    state.journal.shown = true;
    press(&mut state, KeyCode::Char(' '));
    let newest = entry(2, OperationKind::Commit, "commit \"x\"").id;
    assert_eq!(state.journal.expanded, Some(RowKey::Mergewright(newest)));
    press(&mut state, KeyCode::Char(' '));
    assert_eq!(state.journal.expanded, None);
    press(&mut state, KeyCode::Char('j'));
    press(&mut state, KeyCode::Char(' '));
    let older = entry(1, OperationKind::Stage, "stage src/a.rs").id;
    assert_eq!(
        state.journal.expanded,
        Some(RowKey::Mergewright(older)),
        "one at a time, by row"
    );
}

#[test]
fn enter_on_the_next_undo_row_is_u_and_closes_the_browser() {
    let mut state = state_with_two();
    state.journal.shown = true;
    let (claimed, commands) = press(&mut state, KeyCode::Enter);
    assert!(claimed);
    assert!(!state.journal.shown, "the result is on the graph");
    assert!(
        matches!(commands.as_slice(), [Command::Undo(_, false, ..)]),
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
    put_cursor(&mut state, 1);
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
        held: Vec::new(),
        live: Vec::new(),
        interrupted_edits: Vec::new(),
    }));
    state.journal.shown = true;
    // Newest first: [C, undo(B), B, A].
    put_cursor(&mut state, 1);

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
        held: Vec::new(),
        live: Vec::new(),
        interrupted_edits: Vec::new(),
    }));
    state.journal.shown = true;
    put_cursor(&mut state, 0);

    let (claimed, commands) = press(&mut state, KeyCode::Enter);

    assert!(claimed);
    assert!(
        matches!(commands.as_slice(), [Command::Redo(..)]),
        "a redo, not a refusal: {commands:?}"
    );
}

/// Hidden, not closed: `Esc` on the preview reopens the browser on the
/// very row that opened it.
#[test]
fn the_browser_comes_back_on_the_same_row_when_the_preview_is_cancelled() {
    let mut state = state_with_two();
    state.journal.shown = true;
    put_cursor(&mut state, 1);
    press(&mut state, KeyCode::Enter);
    assert!(!state.journal.shown, "the preview has the screen");
    let mut commands = Vec::new();
    crate::update::preview::close_preview(&mut state, &mut commands);
    assert!(state.journal.shown);
    assert_eq!(state.journal.selected(), 1, "the same row, not the newest");
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

// ---- m7-power-tools/24-a: the three-stage list, the row-keyed cursor ----

use mergewright_git::history_reads::{LostRun, Unreached};
use mergewright_git::reflog::ReflogRecord;
use mergewright_ops::journal_list::{Fate, HeldBy, LostCommit, LostEdit, OutsideStep};

fn value(byte: u8) -> Option<mergewright_core::oid::Oid> {
    (byte != 0).then(|| oid(byte))
}

fn record(refname: &str, old: u8, new: u8, at: i64, subject: &str) -> OutsideRecord {
    OutsideRecord {
        record: ReflogRecord {
            refname: refname.to_owned(),
            old: value(old),
            new: value(new),
            at,
            subject: subject.to_owned(),
        },
        back: None,
    }
}

/// The shell's `reset --hard HEAD~1` on `main` at 1005 s, D (`0x04`) lost
/// under it: newer than [`state_with_two`]'s entries (1 ms and 2 ms).
fn shell_reset() -> Vec<OutsideRecord> {
    vec![
        record("HEAD", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("refs/heads/main", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("HEAD", 3, 4, 1004, "commit: D"),
        record("refs/heads/main", 3, 4, 1004, "commit: D"),
    ]
}

fn found(tips: &[u8]) -> LostFound {
    LostFound {
        unreached: Unreached {
            lost: tips
                .iter()
                .map(|tip| LostRun {
                    tip: oid(*tip),
                    under: 0,
                })
                .collect(),
            checked: tips.len(),
            capped: false,
        },
        edits: Vec::new(),
        scan: LostScan::Done,
        subjects: tips
            .iter()
            .map(|tip| (oid(*tip), format!("subject {tip}")))
            .collect(),
    }
}

/// Stage 2 lands, as its job would answer: asked, then answered.
fn outside_lands(state: &mut AppState, records: Vec<OutsideRecord>) {
    let id = JobId::new(900);
    state.journal.outside = Loading::Pending(id);
    let mut commands = Vec::new();
    outside_landed(state, &mut commands, id, &Ok(Arc::new(records)), &mut ids());
}

/// Stage 3 lands.
fn lost_lands(state: &mut AppState, lost: LostFound) {
    let id = JobId::new(901);
    state.journal.lost = Loading::Pending(id);
    lost_landed(state, id, &Ok(Arc::new(lost)));
}

/// J open on [`state_with_two`], no key pressed yet.
fn opened() -> AppState {
    let mut state = state_with_two();
    let _read = journal_enter(&mut state, &mut ids());
    state
}

fn older_entry() -> EntryId {
    entry(1, OperationKind::Stage, "stage src/a.rs").id
}

fn newest_entry() -> EntryId {
    entry(2, OperationKind::Commit, "commit \"x\"").id
}

fn index_of_lost(state: &AppState) -> usize {
    let list = state.journal.list.as_ref().expect("built");
    list.rows
        .iter()
        .position(|row| matches!(row, ListRow::Lost { .. }))
        .expect("a lost row")
}

#[test]
fn j_opens_with_mergewright_rows_at_once() {
    let mut state = state_with_two();
    let commands = journal_enter(&mut state, &mut ids());
    let list = state.journal.list.as_ref().expect("stage 1 is on hand");
    assert_eq!(
        list.rows,
        vec![
            ListRow::Mergewright(newest_entry()),
            ListRow::Mergewright(older_entry())
        ]
    );
    assert_eq!(state.journal.selected(), 0, "on the newest");
    let [Command::ReadJournal(_), Command::ReadOutside { id }] = commands.as_slice() else {
        panic!("the journal again, and stage 2 asked at once: {commands:?}");
    };
    assert_eq!(state.journal.outside, Loading::Pending(*id));
    assert!(state.journal.finding_lost(), "the title says it is looking");
}

#[test]
fn the_cursor_stays_on_its_row_when_outside_rows_arrive() {
    let mut state = opened();
    press(&mut state, KeyCode::Char('j'));
    assert_eq!(
        state.journal.cursor,
        Some(RowKey::Mergewright(older_entry()))
    );
    outside_lands(&mut state, shell_reset());
    assert_eq!(
        state.journal.cursor,
        Some(RowKey::Mergewright(older_entry()))
    );
    let list = state.journal.list.as_ref().expect("built");
    assert_eq!(
        list.rows.get(state.journal.selected()),
        Some(&ListRow::Mergewright(older_entry())),
        "the $ rows came in above, and the cursor went down with its row"
    );
    assert!(state.journal.selected() > 1);
}

#[test]
fn the_cursor_stays_on_its_row_when_lost_rows_arrive() {
    let mut state = opened();
    press(&mut state, KeyCode::Char('j'));
    outside_lands(&mut state, shell_reset());
    lost_lands(&mut state, found(&[4]));
    assert_eq!(
        state.journal.cursor,
        Some(RowKey::Mergewright(older_entry())),
        "a key was pressed: no preselect moves it"
    );
}

#[test]
fn a_vanished_row_falls_to_the_newest() {
    let mut state = opened();
    state.journal.cursor = Some(RowKey::Lost(oid(9)));
    rebuild(&mut state);
    assert_eq!(
        state.journal.cursor,
        Some(RowKey::Mergewright(newest_entry()))
    );
    assert_eq!(state.journal.selected(), 0);
}

#[test]
fn space_expands_by_row_not_by_index() {
    let mut state = opened();
    press(&mut state, KeyCode::Char(' '));
    outside_lands(&mut state, shell_reset());
    assert_eq!(
        state.journal.expanded,
        Some(RowKey::Mergewright(newest_entry()))
    );
    assert!(state.journal.is_expanded(&newest_entry()));
    assert!(
        !matches!(
            state.journal.list.as_ref().expect("built").rows.first(),
            Some(ListRow::Mergewright(_))
        ),
        "row 0 is a $ row now, and it is not the one open"
    );
}

#[test]
fn two_steps_of_one_ref_in_one_second_are_two_rows() {
    let mut state = opened();
    outside_lands(
        &mut state,
        vec![
            record("refs/heads/main", 4, 5, 1004, "commit: E"),
            record("refs/heads/main", 3, 4, 1004, "commit: D"),
        ],
    );
    let list = state.journal.list.clone().expect("built");
    let (first, second) = (row_key(&list, 0), row_key(&list, 1));
    assert!(
        matches!(&first, Some(RowKey::Outside { at: 1004, .. })),
        "{first:?}"
    );
    assert!(
        matches!(&second, Some(RowKey::Outside { at: 1004, .. })),
        "{second:?}"
    );
    assert_ne!(first, second, "same ref, same second, two keys");
    press(&mut state, KeyCode::Char('j'));
    assert_eq!(
        state.journal.selected(),
        1,
        "and the cursor tells them apart"
    );
}

#[test]
fn the_newest_lost_row_is_preselected() {
    let mut state = opened();
    outside_lands(&mut state, shell_reset());
    lost_lands(&mut state, found(&[4]));
    assert_eq!(state.journal.selected(), index_of_lost(&state));
    assert_eq!(state.journal.cursor, Some(RowKey::Lost(oid(4))));
}

#[test]
fn no_preselect_after_a_key() {
    let mut state = opened();
    outside_lands(&mut state, shell_reset());
    press(&mut state, KeyCode::Char('k'));
    lost_lands(&mut state, found(&[4]));
    assert_eq!(state.journal.selected(), 0);
}

/// Entries at these milliseconds are newer than the shell's 1005 s.
fn newer_entry(kind: OperationKind) -> JournalEntry {
    entry(2_000_000, kind, "after the shell")
}

fn opened_with(extra: JournalEntry) -> AppState {
    let mut state = state_with_two();
    if let Loading::Ready(read) = &state.journal.entries {
        let mut read = (**read).clone();
        read.entries.push(extra);
        state.journal.entries = Loading::Ready(Arc::new(read));
    }
    let _read = journal_enter(&mut state, &mut ids());
    state
}

#[test]
fn no_preselect_when_a_mergewright_row_is_newer() {
    let mut state = opened_with(newer_entry(OperationKind::Commit));
    outside_lands(&mut state, shell_reset());
    lost_lands(&mut state, found(&[4]));
    assert_eq!(state.journal.selected(), 0, "Mergewright worked since");
}

#[test]
fn a_fetch_above_does_not_stop_the_preselect() {
    let mut state = opened_with(newer_entry(OperationKind::Fetch));
    outside_lands(&mut state, shell_reset());
    lost_lands(&mut state, found(&[4]));
    assert_eq!(state.journal.selected(), index_of_lost(&state));
}

fn lost_row(event: usize, tip: u8) -> ListRow {
    row_with_fate(event, tip, Fate::Lost)
}

fn row_with_fate(event: usize, tip: u8, fate: Fate) -> ListRow {
    ListRow::Lost {
        event,
        commit: LostCommit {
            run: LostRun {
                tip: oid(tip),
                under: 0,
            },
            subject: Some(format!("subject {tip}")),
            held_by: HeldBy::Head,
            fate,
        },
    }
}

#[test]
fn a_lost_row_under_a_mergewright_row_is_never_preselected() {
    let list = JournalList {
        rows: vec![ListRow::Mergewright(newest_entry()), lost_row(0, 4)],
        scan: LostScan::Done,
    };
    // Even a fetch: the lost row's own event is Mergewright's.
    assert_eq!(preselect(&list, |_| true), None);
    let shell = JournalList {
        rows: vec![
            ListRow::Outside(OutsideStep {
                record: record("HEAD", 4, 3, 1005, "reset: moving to HEAD~1").record,
                words: "moved main".to_owned(),
                folded: 0,
            }),
            lost_row(0, 4),
        ],
        scan: LostScan::Done,
    };
    assert_eq!(preselect(&shell, |_| false), Some(1), "the contrast");
}

#[test]
fn n_goes_to_the_next_lost_row() {
    let mut state = opened();
    outside_lands(&mut state, shell_reset());
    lost_lands(&mut state, found(&[4]));
    press(&mut state, KeyCode::Char('k'));
    press(&mut state, KeyCode::Char('k'));
    assert_eq!(state.journal.selected(), 0);
    press(&mut state, KeyCode::Char('n'));
    assert_eq!(state.journal.selected(), index_of_lost(&state));
    press(&mut state, KeyCode::Char('n'));
    assert_eq!(
        state.journal.selected(),
        index_of_lost(&state),
        "none older: stays"
    );
    press(&mut state, KeyCode::Char('N'));
    assert_eq!(
        state.journal.selected(),
        index_of_lost(&state),
        "none newer: stays"
    );
}

#[test]
fn b_on_any_row_opens_the_name_prompt() {
    let mut state = opened();
    outside_lands(&mut state, shell_reset());
    lost_lands(&mut state, found(&[4]));
    press(&mut state, KeyCode::Char('b'));
    let prompt = state.name_prompt.as_ref().expect("the name prompt");
    assert_eq!(
        prompt.kind,
        crate::update::name_prompt::NameKind::Branch {
            at: oid(4),
            checkout: false
        }
    );
    assert!(state.journal.shown, "J stays under it (V1)");
    // A `$` row: its new value.
    let mut state = opened();
    outside_lands(&mut state, shell_reset());
    press(&mut state, KeyCode::Char('b'));
    assert_eq!(
        state.name_prompt.as_ref().expect("the name prompt").kind,
        crate::update::name_prompt::NameKind::Branch {
            at: oid(3),
            checkout: false
        }
    );
}

#[test]
fn b_on_a_step_with_no_new_names_its_old() {
    let mut state = opened();
    outside_lands(
        &mut state,
        vec![record("refs/heads/spike", 7, 0, 1005, "branch: deleted")],
    );
    let list = state.journal.list.clone().expect("built");
    assert!(
        matches!(list.rows.first(), Some(ListRow::Outside(_))),
        "{list:?}"
    );
    assert_eq!(row_commit(&list, 0), Some(oid(7)));
    press(&mut state, KeyCode::Char('b'));
    assert_eq!(
        state.name_prompt.as_ref().expect("the name prompt").kind,
        crate::update::name_prompt::NameKind::Branch {
            at: oid(7),
            checkout: false
        }
    );
}

#[test]
fn b_on_a_step_with_no_commit_says_so() {
    // A `$` step whose record names no commit (m7/30o Minor 3: a `●` row
    // now says where `b` works instead).
    let mut state = opened();
    state.journal.list = Some(Arc::new(JournalList {
        rows: vec![ListRow::Outside(
            mergewright_ops::journal_list::OutsideStep {
                record: record("refs/heads/main", 0, 0, 1005, "update by push").record,
                words: "moved main".to_owned(),
                folded: 0,
            },
        )],
        scan: LostScan::Done,
    }));
    state.journal.cursor = row_key(state.journal.list.as_ref().expect("set"), 0);
    press(&mut state, KeyCode::Char('b'));
    assert!(state.name_prompt.is_none());
    assert_eq!(state.status, Status::sticky("this step names no commit"));
    // A lost edit: its own words and the key that does put it back.
    state.journal.list = Some(Arc::new(JournalList {
        rows: vec![ListRow::LostEdit {
            event: 0,
            edit: LostEdit {
                entry: newest_entry(),
                backup_tree: oid(8),
                paths: vec!["src/a.rs".into()],
            },
        }],
        scan: LostScan::Done,
    }));
    state.journal.cursor = Some(RowKey::LostEdit(newest_entry()));
    press(&mut state, KeyCode::Char('b'));
    assert!(state.name_prompt.is_none());
    assert_eq!(
        state.status,
        Status::sticky_keys(
            "a lost edit has no commit".to_owned(),
            Some("\u{23ce} puts it back".to_owned())
        )
    );
}

/// J open, the cursor on the lost row, and a `⏎` on it out under `id`.
fn asked_on_the_lost_row(id: JobId) -> AppState {
    let mut state = opened();
    outside_lands(&mut state, shell_reset());
    lost_lands(&mut state, found(&[4]));
    state.journal.recovering = Some((id, RowKey::Lost(oid(4))));
    state
}

#[test]
fn moved_on_shows_the_words_and_b_prefills_the_name() {
    let id = JobId::new(950);
    let mut state = asked_on_the_lost_row(id);
    let mut commands = Vec::new();
    recover_answered(
        &mut state,
        &mut commands,
        id,
        &Ok(Recover::OfferBranch {
            at: oid(4),
            name: "main-restored".to_owned(),
            why: "main has moved on since".to_owned(),
        }),
        &mut ids(),
    );
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(
        state.status,
        Status::sticky_keys(
            "main has moved on since".to_owned(),
            Some("b makes branch \"main-restored\" at 0404040".to_owned())
        )
    );
    press(&mut state, KeyCode::Char('b'));
    let prompt = state.name_prompt.as_ref().expect("the name prompt");
    assert_eq!(prompt.field.text(), "main-restored");
}

#[test]
fn a_refusal_in_j_sets_the_status() {
    let id = JobId::new(951);
    let mut state = asked_on_the_lost_row(id);
    let mut commands = Vec::new();
    recover_answered(
        &mut state,
        &mut commands,
        id,
        &Err("that commit is on main again".to_owned()),
        &mut ids(),
    );
    assert_eq!(state.status, Status::sticky("that commit is on main again"));
    assert!(state.journal.shown);
    assert!(state.preview.is_none());
    // A stale answer changes nothing.
    state.status = Status::default();
    recover_answered(
        &mut state,
        &mut commands,
        JobId::new(1),
        &Err("stale".to_owned()),
        &mut ids(),
    );
    assert_eq!(state.status, Status::default());
}

fn put_back_main() -> Recover {
    Recover::Run(mergewright_ops::operation::RecoverParams {
        target: mergewright_ops::operation::RecoverTarget::MoveRef {
            name: "main".to_owned(),
            to: oid(4),
            expected: oid(3),
        },
        because: "reset: moving to HEAD~1".to_owned(),
    })
}

#[test]
fn enter_on_a_lost_commit_opens_the_put_back_preview() {
    let id = JobId::new(952);
    let mut state = asked_on_the_lost_row(id);
    let mut commands = Vec::new();
    recover_answered(
        &mut state,
        &mut commands,
        id,
        &Ok(put_back_main()),
        &mut ids(),
    );
    let preview = state.preview.as_ref().expect("the put-back's preview");
    assert!(
        matches!(
            &preview.subject,
            crate::update::preview::PreviewSubject::Operation { op, opened_by: "\u{23ce}" }
                if matches!(**op, Operation::Recover(_))
        ),
        "{:?}",
        preview.subject
    );
    assert!(state.journal.shown, "J stays under it");
    assert!(state.operating.is_none(), "y confirms; nothing runs yet");
}

#[test]
fn esc_on_the_put_back_preview_returns_to_j_on_the_same_row() {
    let id = JobId::new(953);
    let mut state = asked_on_the_lost_row(id);
    state.journal.cursor = Some(RowKey::Lost(oid(4)));
    let mut commands = Vec::new();
    recover_answered(
        &mut state,
        &mut commands,
        id,
        &Ok(put_back_main()),
        &mut ids(),
    );
    crate::update::preview::close_preview(&mut state, &mut commands);
    assert!(state.preview.is_none());
    assert!(state.journal.shown);
    assert_eq!(state.journal.cursor, Some(RowKey::Lost(oid(4))));
}

#[test]
fn esc_on_the_name_prompt_returns_to_j_on_the_same_row() {
    let mut state = opened();
    outside_lands(&mut state, shell_reset());
    lost_lands(&mut state, found(&[4]));
    press(&mut state, KeyCode::Char('b'));
    let mut commands = Vec::new();
    crate::update::name_prompt::name_prompt_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Escape),
        &mut ids(),
    );
    assert!(state.name_prompt.is_none());
    assert!(state.journal.shown);
    assert_eq!(state.journal.cursor, Some(RowKey::Lost(oid(4))));
}

#[test]
fn switched_from_reads_the_branch_a_shell_switch_left() {
    use crate::update::switch::SwitchTarget;
    assert_eq!(
        switched_from("checkout: moving from feature to main"),
        Some(SwitchTarget::Branch("refs/heads/feature".to_owned()))
    );
    let hex = "04".repeat(20);
    assert_eq!(
        switched_from(&format!("checkout: moving from {hex} to main")),
        Some(SwitchTarget::Detached(oid(4)))
    );
    assert_eq!(switched_from("reset: moving to HEAD~1"), None);
}

/// `main` (`0x02`, checked out) and `feature` (`0x03`), clean: the switch
/// module's own fixture, which is private to its tests.
fn on_main_with_feature() -> AppState {
    let name = |full: &str| {
        mergewright_core::ref_name::RefName::new(full.as_bytes().to_vec())
            .expect("a valid ref name")
    };
    let refs = crate::refs::badges(
        &[
            (name("refs/heads/main"), oid(2)),
            (name("refs/heads/feature"), oid(3)),
        ],
        &mergewright_git::repo::HeadState::Branch(name("refs/heads/main")),
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(mergewright_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("no ahead/behind walk can fail here");
    AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        refs: Some(Arc::new(refs)),
        working_tree: Some(Arc::new(mergewright_git::status::WorkingTreeStatus {
            entries: vec![],
        })),
        ..AppState::default()
    }
}

#[test]
fn enter_on_a_switch_row_switches_back() {
    let mut state = on_main_with_feature();
    state.journal.entries = state_with_two().journal.entries;
    let _read = journal_enter(&mut state, &mut ids());
    outside_lands(
        &mut state,
        vec![record(
            "HEAD",
            3,
            2,
            1005,
            "checkout: moving from feature to main",
        )],
    );
    let (claimed, commands) = press(&mut state, KeyCode::Enter);
    assert!(claimed);
    assert!(
        state.operating.is_some(),
        "the switch back to feature launched: {:?} {commands:?}",
        state.status
    );
    assert!(!state.journal.shown, "the result is on the graph");
}

// ---- 24-a remainder: the jobs dispatched ----

/// Stage 2's answer to J's own `ReadOutside`, and the commands it asks.
fn stage_two(state: &mut AppState, records: Vec<OutsideRecord>) -> Vec<Command> {
    let Loading::Pending(id) = state.journal.outside else {
        panic!("J asked for stage 2: {:?}", state.journal.outside);
    };
    let mut commands = Vec::new();
    outside_landed(state, &mut commands, id, &Ok(Arc::new(records)), &mut ids());
    commands
}

#[test]
fn outside_rows_arrive_second() {
    let mut state = opened();
    let commands = stage_two(&mut state, shell_reset());
    let list = state.journal.list.as_ref().expect("built");
    assert!(
        matches!(list.rows.first(), Some(ListRow::Outside(_))),
        "the shell's steps joined the list: {:?}",
        list.rows
    );
    // Stage 3 is asked with stage 2's own records, under the id it waits on.
    let [Command::FindLost { id, outside }] = commands.as_slice() else {
        panic!("stage 3 asked: {commands:?}");
    };
    assert_eq!(**outside, shell_reset());
    assert_eq!(state.journal.lost, Loading::Pending(*id));
    // A stale stage-2 answer asks nothing and changes nothing.
    let mut after_stale = Vec::new();
    outside_landed(
        &mut state,
        &mut after_stale,
        JobId::new(77),
        &Ok(Arc::new(Vec::new())),
        &mut ids(),
    );
    assert!(after_stale.is_empty());
}

#[test]
fn lost_rows_arrive_third_and_the_title_says_while() {
    let mut state = opened();
    let commands = stage_two(&mut state, shell_reset());
    let [Command::FindLost { id, .. }] = commands.as_slice() else {
        panic!("stage 3 asked: {commands:?}");
    };
    let list = state.journal.list.clone().expect("built");
    assert_eq!(
        list.scan,
        LostScan::Running,
        "the title's `finding lost commits…`"
    );
    assert!(state.journal.finding_lost());
    assert!(
        !list
            .rows
            .iter()
            .any(|row| matches!(row, ListRow::Lost { .. })),
        "no lost rows before stage 3"
    );
    lost_landed(&mut state, *id, &Ok(Arc::new(found(&[4]))));
    let list = state.journal.list.clone().expect("built");
    assert_eq!(list.scan, LostScan::Done, "nothing in the title once done");
    assert!(!state.journal.finding_lost());
    assert!(
        list.rows
            .iter()
            .any(|row| matches!(row, ListRow::Lost { .. }))
    );
}

#[test]
fn a_failed_scan_says_why_in_the_title() {
    let mut state = opened();
    let commands = stage_two(&mut state, shell_reset());
    let [Command::FindLost { id, .. }] = commands.as_slice() else {
        panic!("stage 3 asked: {commands:?}");
    };
    lost_landed(&mut state, *id, &Err("fatal: bad object".to_owned()));
    let list = state.journal.list.clone().expect("built");
    assert_eq!(list.scan, LostScan::Failed("fatal: bad object".to_owned()));
    assert!(!state.journal.finding_lost());

    // Stage 2 failing is said the same way: the scan never ran, and why.
    let mut state = opened();
    let Loading::Pending(id) = state.journal.outside else {
        panic!("J asked for stage 2");
    };
    let mut commands = Vec::new();
    outside_landed(
        &mut state,
        &mut commands,
        id,
        &Err("fatal: reflog".to_owned()),
        &mut ids(),
    );
    assert!(commands.is_empty(), "no stage 3 over records never read");
    let list = state.journal.list.clone().expect("built");
    assert_eq!(
        list.scan,
        LostScan::Failed("couldn't read git's own records: fatal: reflog".to_owned())
    );
}

/// J open over the shell's reset, every stage in, and `⏎` pressed on the
/// row `pick` finds: the state, the `RecoverFor` id, and the row's key.
fn enter_on(landed: LostFound, pick: impl Fn(&ListRow) -> bool) -> (AppState, JobId, RowKey) {
    let mut state = opened();
    let _find = stage_two(&mut state, shell_reset());
    lost_lands(&mut state, landed);
    let list = state.journal.list.clone().expect("built");
    let row = list.rows.iter().position(pick).expect("the row is listed");
    state.journal.cursor = row_key(&list, row);
    let (claimed, commands) = press(&mut state, KeyCode::Enter);
    assert!(claimed);
    let [
        Command::RecoverFor {
            id,
            list: asked,
            row: asked_row,
            ..
        },
    ] = commands.as_slice()
    else {
        panic!(
            "⏎ asks what the row puts back: {commands:?} {:?}",
            state.status
        );
    };
    assert!(
        Arc::ptr_eq(asked, &list),
        "the list on screen, not a rebuilt one"
    );
    assert_eq!(*asked_row, row);
    let key = row_key(&list, row).expect("a row");
    assert_eq!(state.journal.recovering, Some((*id, key.clone())));
    assert!(state.journal.shown, "J stays while it is asked");
    (state, *id, key)
}

/// `recover_for`'s `Run` answered, and the preview it opened over J.
fn answered_run(
    state: &mut AppState,
    id: JobId,
    target: &mergewright_ops::operation::RecoverTarget,
) {
    let mut commands = Vec::new();
    recover_answered(
        state,
        &mut commands,
        id,
        &Ok(Recover::Run(mergewright_ops::operation::RecoverParams {
            target: target.clone(),
            because: "reset: moving to HEAD~1".to_owned(),
        })),
        &mut ids(),
    );
    let preview = state.preview.as_ref().expect("the put-back's preview");
    assert!(
        matches!(
            &preview.subject,
            crate::update::preview::PreviewSubject::Operation { op, opened_by: "\u{23ce}" }
                if matches!(&**op, Operation::Recover(params) if params.target == *target)
        ),
        "{:?}",
        preview.subject
    );
    assert!(state.journal.shown, "J stays under it");
    assert!(state.operating.is_none(), "y confirms; nothing runs yet");
}

#[test]
fn enter_on_a_lost_commit_asks_what_it_puts_back() {
    let (mut state, id, key) = enter_on(found(&[4]), |row| matches!(row, ListRow::Lost { .. }));
    assert_eq!(key, RowKey::Lost(oid(4)));
    answered_run(
        &mut state,
        id,
        &mergewright_ops::operation::RecoverTarget::MoveRef {
            name: "main".to_owned(),
            to: oid(4),
            expected: oid(3),
        },
    );
}

#[test]
fn enter_on_a_gone_branch_opens_make_it_again() {
    let (mut state, id, _) = enter_on(found(&[4]), |row| matches!(row, ListRow::Lost { .. }));
    answered_run(
        &mut state,
        id,
        &mergewright_ops::operation::RecoverTarget::Recreate {
            name: "spike".to_owned(),
            at: oid(4),
        },
    );
}

#[test]
fn enter_on_an_outside_row_puts_that_ref_back() {
    let (mut state, id, key) = enter_on(
        found(&[]),
        |row| matches!(row, ListRow::Outside(step) if step.record.refname == "refs/heads/main"),
    );
    assert!(matches!(key, RowKey::Outside { .. }), "{key:?}");
    answered_run(
        &mut state,
        id,
        &mergewright_ops::operation::RecoverTarget::MoveRef {
            name: "main".to_owned(),
            to: oid(4),
            expected: oid(3),
        },
    );
}

#[test]
fn enter_on_a_lost_edit_puts_the_file_back() {
    let mut lost = found(&[]);
    lost.edits = vec![LostEdit {
        entry: newest_entry(),
        backup_tree: oid(8),
        paths: vec!["src/a.rs".into()],
    }];
    let (mut state, id, key) = enter_on(lost, |row| matches!(row, ListRow::LostEdit { .. }));
    assert_eq!(key, RowKey::LostEdit(newest_entry()));
    answered_run(
        &mut state,
        id,
        &mergewright_ops::operation::RecoverTarget::Files {
            entry: newest_entry(),
            backup_tree: oid(8),
            paths: vec!["src/a.rs".into()],
        },
    );
}

#[test]
fn closing_j_cancels_a_put_back_still_being_asked() {
    let (mut state, id, _) = enter_on(found(&[4]), |row| matches!(row, ListRow::Lost { .. }));
    let (_, commands) = press(&mut state, KeyCode::Escape);
    assert!(commands.contains(&Command::CancelJob(id)), "{commands:?}");
    assert_eq!(state.journal.recovering, None);
}

mod minors;
mod one_u;
mod replaced;
