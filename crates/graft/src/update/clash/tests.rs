//! `update::clash` tests (m6-conflicts/72b): the refresh rule over the
//! app's reads, the words, and the answer's landing.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use graft_core::journal::{
    EntryId, EntryStatus, HeadRef, IndexState, JournalEntry, OperationKind, PutBack, Touches,
};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::HeadState;
use graft_git::status::{StatusEntry, WorkingTreeStatus};

use super::*;
use crate::jobs::operate::Operated;
use crate::jobs::undo::JournalRead;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).expect("20 bytes is a sha-1 oid")
}

fn ids() -> impl FnMut() -> JobId {
    let mut next = 100u64;
    move || {
        next += 1;
        JobId::new(next)
    }
}

const STASH: u8 = 0x5a;

/// A `Committed` pop from a merged index into a clash, carrying its
/// put-back.
fn pop_entry() -> JournalEntry {
    JournalEntry {
        id: EntryId::new(7, [7; 10]),
        at_unix_ms: 7,
        kind: OperationKind::StashPop,
        description: "pop stash@{0}".to_owned(),
        status: EntryStatus::Committed,
        touches: Touches {
            refs: vec!["refs/stash".to_owned()],
            index: true,
            worktree: true,
        },
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: Some(HeadRef::Branch("refs/heads/main".to_owned())),
        refs_before: BTreeMap::new(),
        refs_after: BTreeMap::new(),
        index_before: Some(IndexState::Merged(oid(1))),
        index_after: Some(IndexState::Unmerged { unmerged: oid(2) }),
        worktree_before: None,
        worktree_after: None,
        backup_ref: None,
        pins: vec![oid(STASH)],
        commands: Vec::new(),
        reverses: None,
        merge_before: None,
        merge_after: None,
        put_back: Some(PutBack {
            stash: oid(STASH),
            why: PutBackWhy::Pop,
        }),
    }
}

fn entry(path: &str, unmerged: bool) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged,
        unmerged_how: None,
        submodule: false,
    }
}

fn refs_with_stash() -> crate::refs::RefBadges {
    let main = RefName::new(b"refs/heads/main".to_vec()).expect("a valid ref name");
    crate::refs::badges(
        &[(main.clone(), oid(1))],
        &HeadState::Branch(main),
        &HashMap::new(),
        &[(oid(STASH), 0)],
        |_, _| unreachable!("no upstream is configured"),
    )
    .expect("badges builds from plain data")
}

/// Every fact a clash is read from, nothing in flight: the journal holding
/// the pop, `notes.md` unmerged (or not), the stash listed.
fn facts(unmerged: bool) -> AppState {
    let mut state = AppState::default();
    state.journal.entries = Loading::Ready(Arc::new(JournalRead {
        entries: vec![pop_entry()],
        unreadable: Vec::new(),
        live: Vec::new(),
    }));
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: vec![entry("notes.md", unmerged)],
    }));
    state.refs = Some(Arc::new(refs_with_stash()));
    state
}

#[test]
fn refresh_clash_follows_the_reads() {
    // Set on a journal read: the pop is in the journal, the tree unmerged.
    let mut state = facts(true);
    state.clash_facts_seen = true;
    let mut commands = Vec::new();
    let reading = JobId::new(9);
    state.journal.reading = Some(reading);
    let read = state.journal.entries.clone();
    let Loading::Ready(read) = read else {
        unreachable!("facts() holds a ready journal")
    };
    let product = crate::update::Product::Journal(Ok(read));
    crate::update::journal::journal_event(&mut state, &mut commands, reading, &product, &mut ids());
    let clash = state
        .clash
        .clone()
        .expect("the journal read found the clash");
    assert_eq!(clash.clash.index, Some(0));
    assert_eq!(clash.clash.why, PutBackWhy::Pop);
    assert_eq!(clash.origin, LiveOrigin::ThisRun, "found after the launch");

    // Cleared by a working-tree read with nothing unmerged.
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: vec![entry("notes.md", false)],
    }));
    refresh_clash(&mut state);
    assert_eq!(state.clash, None, "nothing unmerged, no clash");
}

#[test]
fn refresh_clash_waits_while_a_read_is_out() {
    let mut state = facts(false);
    state.clash = Some(ClashState {
        clash: StashClash {
            entry: EntryId::new(7, [7; 10]),
            stash: oid(STASH),
            index: Some(0),
            why: PutBackWhy::Pop,
        },
        origin: LiveOrigin::ThisRun,
    });
    state.journal.reading = Some(JobId::new(3));
    refresh_clash(&mut state);
    assert!(
        state.clash.is_some(),
        "a fresh status against a journal from before the write is not judged"
    );
}

#[test]
fn a_clash_found_by_the_launch_is_from_the_last_run() {
    let mut state = facts(true);
    refresh_clash(&mut state);
    assert_eq!(
        state.clash.as_ref().map(|clash| clash.origin),
        Some(LiveOrigin::PreviousRun)
    );
    assert_eq!(
        crate::update::banner::stop_line(&state).as_deref(),
        Some("stash@{0} clash from last run \u{b7} 1 file conflicted")
    );
    assert!(state.stopped.is_none(), "a clash is not a stop");
}

#[test]
fn the_journal_is_read_after_a_write() {
    for scope in [
        graft_ops::apply::Refresh::WorkingTree,
        graft_ops::apply::Refresh::Everything,
    ] {
        let mut state = AppState::default();
        let commands = crate::update::operate::refresh_scope(&mut state, scope, &mut ids());
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::ReadJournal(_))),
            "{scope:?}: {commands:?}"
        );
    }
}

/// `Running::op` pin (67): the `Clashed` answer reads the stash's position
/// from the op that was launched, so the notice names the popped index —
/// `stash@{2}` here — not `stash@{0}`.
#[test]
fn the_notice_names_the_popped_index_after_a_clashed_answer() {
    let mut state = AppState::default();
    let mut commands = Vec::new();
    let mut next = ids();
    crate::update::operate::launch(
        &mut state,
        &mut commands,
        Operation::StashPop {
            index: 2,
            commit: oid(STASH),
        },
        &mut next,
    )
    .expect("nothing else is running");
    let id = state.operating.expect("the pop is running");
    let answer = Operated::Clashed {
        entry: EntryId::new(7, [7; 10]),
        clash: Clash {
            stash: oid(STASH),
            why: PutBackWhy::Pop,
            paths: vec![PathBuf::from("notes.md"), PathBuf::from("todo.md")],
        },
    };
    crate::update::operate::operated(&mut state, id, answer, &mut next);
    assert_eq!(
        state.clash.as_ref().and_then(|clash| clash.clash.index),
        Some(2)
    );
    assert_eq!(state.status, Status::Ready, "no failure line");
    let open = state.resolver.as_ref().expect("landed in the resolver");
    assert_eq!(open.path, PathBuf::from("notes.md"));
    assert_eq!(
        open.notice
            .as_ref()
            .map(|(words, problem)| (words.as_str(), *problem)),
        Some((
            "stash@{2} clashed in 2 files \u{b7} esc, u puts it all back",
            false
        ))
    );
    assert!(state.stopped.is_none() && state.live_entries.is_empty());
}

#[test]
fn the_stop_ladder_drops_the_origin_then_the_count_then_the_name() {
    let clash = ClashState {
        clash: StashClash {
            entry: EntryId::new(7, [7; 10]),
            stash: oid(STASH),
            index: Some(1),
            why: PutBackWhy::Apply,
        },
        origin: LiveOrigin::PreviousRun,
    };
    assert_eq!(
        stop_wordings(&clash, 2),
        [
            "stash@{1} clash from last run \u{b7} 2 files conflicted",
            "stash@{1} clashed \u{b7} 2 conflicted",
            "stash@{1} clashed",
            "stash clashed",
        ]
    );
}

#[test]
fn the_close_words() {
    assert_eq!(
        kept_status(Some(0)),
        Status::Done {
            what: "every file done \u{b7} stash@{0} kept".to_owned(),
            keys: None,
        }
    );
    assert_eq!(
        dropped_status(Some(0)),
        Status::Done {
            what: "every file done \u{b7} stash@{0} dropped".to_owned(),
            keys: Some("u brings it back".to_owned()),
        }
    );
    assert!(drops(PutBackWhy::Pop) && drops(PutBackWhy::SetAside));
    assert!(!drops(PutBackWhy::Apply));
}
