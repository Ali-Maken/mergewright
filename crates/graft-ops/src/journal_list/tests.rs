//! The list's pure half: `build`, `words` and `recover_for` over records and
//! entries written by hand. What real git writes is proven by the oracle
//! (`tests/it/journal_list_oracle.rs`); the shapes here are copied from it.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod build;
mod recover;
mod words;

use std::collections::BTreeMap;

use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_git::history_reads::{LostRun, Unreached};
use graft_git::reflog::ReflogRecord;

use super::{JournalList, LostEdit, LostScan, OutsideRecord, build};

/// A commit id made of one repeated byte, so a test can name it `oid(4)`.
fn oid(byte: u8) -> Oid {
    Oid::from_sha1([byte; 20])
}

fn value(byte: u8) -> Option<Oid> {
    (byte != 0).then(|| oid(byte))
}

/// One record: `0` stands for git's zero id.
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

fn with_back(mut outside: OutsideRecord, back: usize) -> OutsideRecord {
    outside.back = Some(back);
    outside
}

/// A committed entry at `at_s` seconds that moved each `(ref, before,
/// after)`; `HEAD` on `main` throughout.
fn entry(
    at_s: i64,
    kind: OperationKind,
    description: &str,
    refs: &[(&str, u8, u8)],
) -> JournalEntry {
    let before: BTreeMap<String, Option<Oid>> = refs
        .iter()
        .map(|(name, old, _)| ((*name).to_owned(), value(*old)))
        .collect();
    let after: BTreeMap<String, Option<Oid>> = refs
        .iter()
        .map(|(name, _, new)| ((*name).to_owned(), value(*new)))
        .collect();
    let at_unix_ms = at_s * 1000;
    JournalEntry {
        id: EntryId::new(at_unix_ms, [7; 10]),
        at_unix_ms,
        kind,
        description: description.to_owned(),
        status: EntryStatus::Committed,
        touches: Touches {
            refs: refs.iter().map(|(name, _, _)| (*name).to_owned()).collect(),
            index: false,
            worktree: false,
        },
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: Some(HeadRef::Branch("refs/heads/main".to_owned())),
        refs_before: before,
        refs_after: after,
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        backup_ref: None,
        pins: vec![],
        commands: vec![],
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: std::collections::BTreeMap::new(),
        config_after: std::collections::BTreeMap::new(),
        put_back: None,
    }
}

fn run(tip: u8, under: usize) -> LostRun {
    LostRun {
        tip: oid(tip),
        under,
    }
}

fn lost(runs: &[LostRun]) -> Unreached {
    Unreached {
        lost: runs.to_vec(),
        checked: runs.len(),
        capped: false,
    }
}

/// `build` with every lost tip named `subject <byte>`, every branch present
/// but those in `gone`, and no lost edits.
fn list(
    entries: &[JournalEntry],
    outside: &[OutsideRecord],
    found: Option<&Unreached>,
    gone: &[&str],
) -> JournalList {
    list_with_edits(entries, outside, found, gone, &[])
}

fn list_with_edits(
    entries: &[JournalEntry],
    outside: &[OutsideRecord],
    found: Option<&Unreached>,
    gone: &[&str],
    edits: &[LostEdit],
) -> JournalList {
    let subject_of = |tip: Oid| Some(format!("subject {}", tip.as_bytes()[0]));
    let exists = |name: &str| !gone.contains(&name);
    build(
        entries,
        outside,
        found,
        &subject_of,
        &exists,
        edits,
        LostScan::Done,
    )
}
