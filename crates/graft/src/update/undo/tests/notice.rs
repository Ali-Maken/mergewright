//! `update::undo` tests: the interrupted notice. Moved out of the inline module (m8/21b-5,
//! move-only).

use super::*;

/// A journal holding one entry per description, each one torn the way
/// recovery leaves it: `Aborted { reason: "interrupted" }`, which is what
/// the view is actually handed (`Pending` never survives a read).
fn read(interrupted: &[&str], unreadable: &[&str]) -> JournalState {
    let entries = interrupted
        .iter()
        .enumerate()
        .map(|(index, description)| torn_entry(index, description))
        .collect();
    JournalState {
        entries: Loading::Ready(Arc::new(JournalRead {
            entries,
            unreadable: Vec::new(),
            live: Vec::new(),
            interrupted_edits: Vec::new(),
        })),
        unreadable: unreadable.iter().map(|&s| s.to_owned()).collect(),
        ..JournalState::default()
    }
}

/// One entry a previous run left open and recovery then closed.
pub(super) fn torn_entry(index: usize, description: &str) -> graft_core::journal::JournalEntry {
    use graft_core::journal::{EntryId, EntryStatus, HeadRef, OperationKind, Touches};
    let oid = |byte: u8| graft_core::oid::Oid::from_bytes(&[byte; 20]).expect("20 bytes");
    let number = i64::try_from(index).expect("a handful of entries") + 1;
    graft_core::journal::JournalEntry {
        id: EntryId::new(number, [0; 10]),
        at_unix_ms: number,
        kind: OperationKind::Commit,
        description: description.to_owned(),
        status: EntryStatus::Aborted {
            reason: graft_core::journal::INTERRUPTED.to_owned(),
        },
        touches: Touches {
            refs: Vec::new(),
            index: false,
            worktree: true,
        },
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: None,
        refs_before: std::collections::BTreeMap::new(),
        refs_after: std::collections::BTreeMap::new(),
        index_before: None,
        index_after: None,
        // A backup of the tree it changed, so `in_force` counts it: an
        // entry `u` can no longer reach is not one to announce.
        worktree_before: Some(oid(3)),
        worktree_after: None,
        worktree_raw_before: None,
        backup_ref: Some("refs/graft/backup/main/x".to_owned()),
        pins: Vec::new(),
        commands: Vec::new(),
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: std::collections::BTreeMap::new(),
        config_after: std::collections::BTreeMap::new(),
        put_back: None,
        fetched_refs: None,
    }
}

#[test]
fn the_notice_names_the_interrupted_entry_until_it_has_been_seen() {
    let mut state = AppState {
        journal: read(&["commit \"fix\""], &[]),
        ..AppState::default()
    };
    let notice = interrupted_notice(&state).expect("there is one to report");
    assert!(notice.contains("commit \"fix\""), "{notice}");
    assert!(notice.contains("J to inspect"), "{notice}");

    state.journal.notice_seen = true;
    assert_eq!(interrupted_notice(&state), None);
}

#[test]
fn a_clean_journal_has_no_notice() {
    let state = AppState {
        journal: read(&[], &[]),
        ..AppState::default()
    };
    assert_eq!(interrupted_notice(&state), None);
}

#[test]
fn an_unreadable_after_state_says_undo_would_restore_everything() {
    let state = AppState {
        journal: read(&["stash pop"], &["stash pop: could not read HEAD"]),
        ..AppState::default()
    };
    let notice = interrupted_notice(&state).expect("there is one to report");
    assert!(notice.contains("restores the whole tree"), "{notice}");
}

/// Review 08 Minor 4. A revert whose plan could not be made at all
/// used to say `revert stopped after 0 of 0 steps`, which describes
/// something that ran and gave up. Nothing ran.
#[test]
fn a_revert_that_never_started_is_worded_as_a_refusal() {
    let mut state = AppState::default();
    let running = JobId::new(7);
    state.operating = Some(running);
    let mut next = 0;
    let mut next_id = || {
        next += 1;
        JobId::new(next)
    };

    let commands = reverted_to(
        &mut state,
        running,
        crate::jobs::undo::RevertedTo::Refused {
            description: "commit one".to_owned(),
            error: "there is nothing to revert".to_owned(),
        },
        &mut next_id,
    );

    assert_eq!(
        state.status,
        Status::sticky(
            "cannot revert to before \u{201c}commit one\u{201d}: there is nothing to revert"
        )
    );
    assert!(commands.is_empty(), "nothing ran, so nothing is re-read");
}

/// Review 08 Minor 2. The drift box's `[ Revert ]` used to launch under
/// `revert to before "01J..."` — the entry's ULID, because the
/// description was not carried through `ConfirmThen::RevertTo`. A ULID
/// is a fact about the journal file, not about what the user asked for.
#[test]
fn the_time_machines_busy_line_names_the_entry_not_its_id() {
    let target = EntryId::new(1, [0; 10]);
    let mut state = AppState::default();
    let mut commands = Vec::new();
    let mut next = 0;
    let mut next_id = || {
        next += 1;
        JobId::new(next)
    };
    // `reverted_to` drops an outcome that is not the job in flight.
    let running = JobId::new(99);
    state.operating = Some(running);
    reverted_to(
        &mut state,
        running,
        crate::jobs::undo::RevertedTo::Drift {
            description: "commit one".to_owned(),
            total: 1,
            drift: drift(),
            target,
        },
        &mut next_id,
    );
    let confirm = state.confirm.as_mut().expect("the drift box is open");
    // Index 0 is `Stop`; `[ Revert ]` is the acting choice.
    confirm.view.right();

    assert!(crate::update::confirm::confirm_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Enter),
        &mut next_id,
    ));

    assert_eq!(
        state.status,
        Status::Busy("revert to before \u{201c}commit one\u{201d}".to_owned()),
        "the entry's own words, not its ULID"
    );
    assert!(
        matches!(
            commands.as_slice(),
            [Command::Unstoppable(..), Command::RevertTo(_, _, true, ..)]
        ),
        "{commands:?}"
    );
}

// ── ticket 41: the landing after an undo that puts an operation back ──
