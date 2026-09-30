//! J's find: which rows its words name (m7-power-tools/30o-c R4).

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_git::history_reads::LostRun;
use graft_git::reflog::ReflogRecord;
use graft_ops::journal_list::{
    HeldBy, JournalList, ListRow, LostCommit, LostEdit, LostScan, OutsideStep,
};

use super::super::JournalState;
use super::{RowKey, hits};
use crate::jobs::undo::JournalRead;
use crate::update::detail::Loading;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).expect("20 bytes is a sha-1 oid")
}

fn entry(at: i64, description: &str, status: EntryStatus) -> JournalEntry {
    JournalEntry {
        id: EntryId::new(at, [0; 10]),
        at_unix_ms: at,
        kind: OperationKind::Discard,
        description: description.to_owned(),
        status,
        touches: Touches {
            refs: Vec::new(),
            index: true,
            worktree: true,
        },
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: Some(HeadRef::Branch("refs/heads/main".to_owned())),
        refs_before: std::collections::BTreeMap::new(),
        refs_after: std::collections::BTreeMap::new(),
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        worktree_raw_before: None,
        backup_ref: None,
        pins: Vec::new(),
        commands: vec!["git restore -- aborted.txt".to_owned()],
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: std::collections::BTreeMap::new(),
        config_after: std::collections::BTreeMap::new(),
        put_back: None,
    }
}

/// Four rows, newest first: an aborted `●` discard, a `$` commit on
/// `spike`, a `✗` commit `spike` held, and a lost edit to `src/a.rs`.
fn journal() -> JournalState {
    let discard = entry(
        1,
        "discard notes.txt",
        EntryStatus::Aborted {
            reason: "aborted by you".to_owned(),
        },
    );
    let id = discard.id.clone();
    let rows = vec![
        ListRow::Graft(id.clone()),
        ListRow::Outside(OutsideStep {
            record: ReflogRecord {
                refname: "refs/heads/spike".to_owned(),
                old: Some(oid(3)),
                new: Some(oid(4)),
                at: 1004,
                subject: "commit: Try redis cache".to_owned(),
            },
            words: "committed \"Try redis cache\"".to_owned(),
            folded: 0,
        }),
        ListRow::Lost {
            event: 1,
            commit: LostCommit {
                run: LostRun {
                    tip: oid(0xab),
                    under: 0,
                },
                subject: Some("Add rate limiter".to_owned()),
                held_by: HeldBy::Branch {
                    name: "spike".to_owned(),
                    gone: true,
                },
            },
        },
        ListRow::LostEdit {
            event: 0,
            edit: LostEdit {
                entry: id,
                backup_tree: oid(9),
                paths: vec![PathBuf::from("src/a.rs")],
            },
        },
    ];
    JournalState {
        shown: true,
        entries: Loading::Ready(Arc::new(JournalRead {
            entries: vec![discard],
            unreadable: Vec::new(),
            live: Vec::new(),
            interrupted_edits: Vec::new(),
        })),
        list: Some(Arc::new(JournalList {
            rows,
            scan: LostScan::Done,
        })),
        ..JournalState::default()
    }
}

fn keys_at(journal: &JournalState, indices: &[usize]) -> Vec<RowKey> {
    let list = journal.list.as_ref().expect("built");
    indices
        .iter()
        .map(|&index| super::row_key(list, index).expect("a row"))
        .collect()
}

#[test]
fn hits_find_a_lost_subject_a_branch_and_a_path() {
    let journal = journal();
    // The lost commit's subject, any case.
    assert_eq!(hits(&journal, "RATE limiter"), keys_at(&journal, &[2]));
    // The branch: the `$` row's ref short name, and the `✗` row it held.
    assert_eq!(hits(&journal, "spike"), keys_at(&journal, &[1, 2]));
    // The lost edit's path.
    assert_eq!(hits(&journal, "src/a.rs"), keys_at(&journal, &[3]));
    // The sha7 of the lost commit.
    assert_eq!(hits(&journal, "abababa"), keys_at(&journal, &[2]));
    // A `●` row's description; git's subject on a `$` row.
    assert_eq!(hits(&journal, "notes"), keys_at(&journal, &[0]));
    assert_eq!(hits(&journal, "commit: try"), keys_at(&journal, &[1]));
}

#[test]
fn hits_skip_verdict_words() {
    let journal = journal();
    // The `●` row was aborted, and its command names `aborted.txt`: neither
    // is a word that names the row.
    assert_eq!(hits(&journal, "aborted"), Vec::new());
}

#[test]
fn a_blank_find_has_no_hits() {
    let journal = journal();
    assert_eq!(hits(&journal, ""), Vec::new());
    assert_eq!(hits(&journal, "   "), Vec::new());
}
