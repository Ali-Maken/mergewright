//! `update::undo` tests: launch and refusals. Moved out of the inline module (m8/21b-5,
//! move-only).

use std::sync::Arc;

use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_core::journal::{
    EntryId, EntryStatus, HeadRef, IndexState, JournalEntry, OperationKind, Touches,
};
use mergewright_ops::apply::Applied;

use super::*;
use crate::jobs::undo::JournalRead;
use crate::update::detail::Loading;
use crate::update::journal::JournalState;

fn ids() -> impl FnMut() -> JobId {
    let mut next = 0;
    move || {
        next += 1;
        JobId::new(next)
    }
}

/// An `Applied` naming `description`, with `refresh` as its scope: the
/// two fields `undone` reads.
fn applied(description: &str, refresh: Refresh) -> Applied {
    Applied {
        came_back: Vec::new(),
        stopped_at: None,
        entry: JournalEntry {
            id: EntryId::new(1, [0; 10]),
            at_unix_ms: 1,
            kind: OperationKind::Undo,
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
            fetched_refs: None,
        },
        refresh,
        step_backup: None,
        replaced: mergewright_ops::undo::Replaced::default(),
    }
}

/// Two different merged trees: index drift that still explains itself as
/// "the index changed", the wording the m3 captures pin.
fn index_drift() -> (IndexState, IndexState) {
    let oid = |byte: u8| mergewright_core::oid::Oid::from_bytes(&[byte; 20]).expect("20 bytes");
    (IndexState::Merged(oid(1)), IndexState::Merged(oid(2)))
}

fn drift() -> mergewright_ops::undo::Drift {
    mergewright_ops::undo::Drift {
        index: Some(index_drift()),
        ..mergewright_ops::undo::Drift::default()
    }
}

#[test]
fn u_asks_for_an_undo_and_says_so() {
    let mut state = AppState::default();
    let commands = undo_key(&mut state, false, &mut ids());
    assert_eq!(
        commands,
        vec![Command::Undo(
            JobId::new(1),
            false,
            std::sync::Arc::new(mergewright_ui::keymap::Keymap::shipped().clone())
        )]
    );
    assert_eq!(state.operating, Some(JobId::new(1)));
    // The busy line offers the cancel, because the cancel is now real
    // (m5-history-and-ghost-preview/06): `restore::execute` rolls back
    // the steps it had applied. `update::remote::tests::
    // esc_cancels_an_undo_in_flight` is the other half.
    assert_eq!(
        state.status,
        Status::Busy(format!(
            "undo{}",
            super::super::remote::esc_hint(mergewright_ui::keymap::Keymap::shipped())
        ))
    );
}

#[test]
fn capital_u_asks_for_a_redo() {
    let mut state = AppState::default();
    let commands = undo_key(&mut state, true, &mut ids());
    assert_eq!(
        commands,
        vec![Command::Redo(
            JobId::new(1),
            false,
            std::sync::Arc::new(mergewright_ui::keymap::Keymap::shipped().clone())
        )]
    );
}

/// Piece 6. A stopped merge, rebase, cherry-pick or revert has exactly
/// two ends, and `u` is neither. Refused **in words**, with no job — and
/// the words name where the two ends are.
#[test]
fn u_refused_during_an_operation() {
    for (kind, word) in [
        (mergewright_git::state::InProgress::Merge, "merge"),
        (mergewright_git::state::InProgress::Rebase, "rebase"),
        (
            mergewright_git::state::InProgress::CherryPick,
            "cherry-pick",
        ),
        (mergewright_git::state::InProgress::Revert, "revert"),
    ] {
        let mut state = AppState {
            in_progress: Some(mergewright_git::state::InProgressDetail {
                kind,
                step: None,
                head_name: None,
                onto: None,
                theirs: Ok(None),
            }),
            ..AppState::default()
        };
        for redo in [false, true] {
            let commands = undo_key(&mut state, redo, &mut ids());
            assert!(commands.is_empty(), "{word}: {commands:?}");
            assert_eq!(state.operating, None, "{word}: a job was claimed");
            assert_eq!(
                state.status,
                Status::Sticky {
                    reason: format!("finish or abort the {word} first"),
                    keys: Some("\u{21e5} banner".to_owned()),
                },
                "{word}"
            );
        }
    }
}

/// The drift box's second, forcing `Enter` is the same key again, so it
/// meets the same refusal: `[ Undo ]` must not be a way around a live
/// operation.
#[test]
fn forcing_past_the_drift_box_is_refused_during_an_operation_too() {
    let mut state = AppState {
        confirm: Some(drift_box("stage src/a.rs", &drift(), false)),
        in_progress: Some(mergewright_git::state::InProgressDetail {
            kind: mergewright_git::state::InProgress::Rebase,
            step: None,
            head_name: None,
            onto: None,
            theirs: Ok(None),
        }),
        ..AppState::default()
    };
    state
        .confirm
        .as_mut()
        .expect("the box is open")
        .view
        .toggle();
    let mut commands = Vec::new();
    assert!(super::super::confirm::confirm_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Enter),
        &mut ids(),
    ));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(state.operating, None);
}

/// A stopped merge, and the journal as the app last read it: `Some`
/// holds the entries git is still inside (`JournalRead::live`), `None`
/// is the launch read still out.
fn merging(journal: Option<Vec<EntryId>>) -> AppState {
    let (entries, live_entries) = match journal {
        Some(live) => (
            Loading::Ready(Arc::new(JournalRead {
                entries: Vec::new(),
                unreadable: Vec::new(),
                held: Vec::new(),
                live: live.clone(),
                interrupted_edits: Vec::new(),
            })),
            live,
        ),
        None => (Loading::Pending(JobId::new(40)), Vec::new()),
    };
    AppState {
        in_progress: Some(mergewright_git::state::InProgressDetail {
            kind: mergewright_git::state::InProgress::Merge,
            step: None,
            head_name: None,
            onto: None,
            theirs: Ok(None),
        }),
        live_entries,
        journal: JournalState {
            entries,
            ..JournalState::default()
        },
        ..AppState::default()
    }
}

/// Keymap §2.6a's one exception (m6-conflicts/57): in a merge **begun
/// in the shell** — the journal read, and no entry of Mergewright's live —
/// `u` and `U` are sent, and ops' gate answers.
#[test]
fn u_in_a_merge_begun_in_the_shell_is_sent() {
    for redo in [false, true] {
        let mut state = merging(Some(Vec::new()));
        let commands = undo_key(&mut state, redo, &mut ids());
        let sent = if redo {
            Command::Redo(
                JobId::new(1),
                false,
                std::sync::Arc::new(mergewright_ui::keymap::Keymap::shipped().clone()),
            )
        } else {
            Command::Undo(
                JobId::new(1),
                false,
                std::sync::Arc::new(mergewright_ui::keymap::Keymap::shipped().clone()),
            )
        };
        assert_eq!(commands, vec![sent], "redo {redo}: {:?}", state.status);
    }
}

/// A merge Mergewright began folds its steps into its own live entry, so `a`
/// is its way back: `u` refuses as before.
#[test]
fn u_still_refuses_in_a_merge_begun_in_mergewright() {
    let mut state = merging(Some(vec![EntryId::new(7, [7; 10])]));
    for redo in [false, true] {
        assert!(undo_key(&mut state, redo, &mut ids()).is_empty());
        assert_eq!(
            state.status,
            live_refusal(
                mergewright_git::state::InProgress::Merge,
                Some("\u{21e5} banner".to_owned())
            )
        );
    }
}

/// Until the launch's journal read answers, `live_entries` is empty for
/// a merge Mergewright began too, so "begun in the shell" is not known yet:
/// refused as before (decision 1).
#[test]
fn u_refuses_while_the_launch_journal_read_is_out() {
    let mut state = merging(None);
    for redo in [false, true] {
        assert!(undo_key(&mut state, redo, &mut ids()).is_empty());
        assert_eq!(state.operating, None);
        assert_eq!(
            state.status,
            live_refusal(
                mergewright_git::state::InProgress::Merge,
                Some("\u{21e5} banner".to_owned())
            )
        );
    }
}

/// The app's refusal and ops' own answer are the same sentence for
/// every kind, so `u` reads the same whichever side refused it.
#[test]
fn operation_live_words_match_ops() {
    use mergewright_git::state::InProgress;
    use mergewright_ops::undo::{NotInvertible, UndoError};
    for kind in [
        InProgress::Merge,
        InProgress::Rebase,
        InProgress::CherryPick,
        InProgress::Revert,
        InProgress::Bisect,
    ] {
        let ops = UndoError::Nothing(NotInvertible::OperationLive { kind }).explain(
            &crate::update::op_keys::OpKeys(mergewright_ui::keymap::Keymap::shipped()),
        );
        let Status::Sticky { reason, .. } = live_refusal(kind, None) else {
            panic!("a refusal is sticky");
        };
        assert_eq!(reason, ops, "{kind:?}");
    }
}

/// Ops' gate refused (an entry older than the merge): a refusal in the
/// app's words, sticky, pointing at the banner — not the plain `Done`
/// that "nothing to undo" gets.
#[test]
fn an_operation_live_answer_is_a_refusal() {
    let mut state = AppState {
        operating: Some(JobId::new(2)),
        ..merging(Some(Vec::new()))
    };
    let commands = undone(
        &mut state,
        JobId::new(2),
        Undone::OperationLive {
            kind: mergewright_git::state::InProgress::Merge,
        },
        &mut ids(),
    );
    assert!(commands.is_empty(), "nothing ran: {commands:?}");
    assert_eq!(state.operating, None);
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: "finish or abort the merge first".to_owned(),
            keys: Some("\u{21e5} banner".to_owned()),
        }
    );
}

#[test]
fn u_while_a_write_is_running_asks_for_nothing() {
    let mut state = AppState {
        operating: Some(JobId::new(4)),
        ..AppState::default()
    };
    assert!(undo_key(&mut state, false, &mut ids()).is_empty());
    // The running write keeps the slot: a second `u` must not steal it.
    assert_eq!(state.operating, Some(JobId::new(4)));
}

/// `main` committed, then pushed: the journal `u` asks about first.
fn after_a_push() -> AppState {
    let mut commit = applied("commit \"to push\"", Refresh::WorkingTree).entry;
    commit.id = EntryId::new(1, [0; 10]);
    commit.kind = OperationKind::Commit;
    commit.status = EntryStatus::Committed;
    commit.head_before = HeadRef::Branch("refs/heads/main".to_owned());
    commit
        .refs_before
        .insert("refs/heads/main".to_owned(), None);
    let mut push = commit.clone();
    push.id = EntryId::new(2, [0; 10]);
    push.kind = OperationKind::Push;
    push.refs_before.clear();
    let mut pushed = AppState::default();
    pushed.journal.entries = Loading::Ready(Arc::new(JournalRead {
        entries: vec![commit, push],
        unreadable: Vec::new(),
        held: Vec::new(),
        live: Vec::new(),
        interrupted_edits: Vec::new(),
    }));
    pushed
}

mod answers;
mod landing;
mod notice;
mod set_aside;

// `set_aside` reaches these through `tests`, as it did before the split.
use landing::{done_line_of, finish_undone, read_lands};
use notice::torn_entry;
