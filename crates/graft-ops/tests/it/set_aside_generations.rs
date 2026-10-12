//! The messages written into git across generations (ADR-0057 amendment
//! 2026-10-11 §1, ticket m8/52): this build writes `mergewright`, and a
//! set-aside an earlier build left (`graft:`, `tributary:`) is still found
//! and put back. The set-aside is found by its id on the journal entry,
//! never by its message, so retagging the stash must change nothing.
//!
//! The older stash is the app's own set-aside, interrupted by a kill (K1 in
//! `set_aside_kill`), then re-listed under an older tag with `git stash
//! store -m` — the same commit, the message an older build would have
//! written. Truth from `git stash list` and the file bytes.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use graft_core::on_disk::{Generation, MESSAGE_TAG};
use graft_ops::EditsAt;
use graft_ops::apply::recover_interrupted;

use crate::put_back_index::staged_near_their_change;
use crate::set_aside::{journal_entries, set_aside_of, stash_commits, whole};
use crate::set_aside_kill::{Shim, run_child};
use crate::support::{Letters, NOW, Never, TempRepo, context, journal_for, run_undo};

/// A set-aside interrupted between its two applies: the edits are only in
/// the stash. Returns the repository, its state before the switch, and the
/// stash's id.
fn interrupted_set_aside() -> (TempRepo, (crate::support::RepoState, String), String) {
    let repo = staged_near_their_change();
    let before = whole(&repo);
    let (killed, _) = run_child(&repo, "refs/heads/other", Shim::KillOnPlainApply);
    assert!(killed, "the shim killed the child");
    let entries = journal_entries(&repo);
    let [entry] = entries.as_slice() else {
        panic!("one entry: {entries:?}");
    };
    let stash = set_aside_of(entry).to_string();
    (repo, before, stash)
}

/// The stash list's messages, newest first.
fn stash_messages(repo: &TempRepo) -> Vec<String> {
    repo.git(&["stash", "list", "--format=%gs"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// An older generation's set-aside, after a relaunch: recovery names it,
/// `u` puts the edits back, and the stash list is empty after.
#[test]
fn an_older_tag_is_still_ours() {
    for older in Generation::CURRENT.older() {
        let (repo, before, stash) = interrupted_set_aside();
        let message = format!("{}: your edits, set aside for switch to other", older.dir());
        repo.git(&["stash", "drop", "-q"]);
        repo.git(&["stash", "store", "-m", &message, &stash]);
        assert_eq!(
            stash_messages(&repo),
            std::slice::from_ref(&message),
            "{older:?}: the stash is listed under the older tag"
        );
        assert_eq!(
            repo.git(&["stash", "list", "--format=%H"]).trim(),
            stash,
            "{older:?}: the same commit"
        );

        let handle = repo.open();
        let mut journal = journal_for(&handle);
        let never = Never;
        let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
        let recovered = recover_interrupted(&mut ctx)
            .unwrap_or_else(|why| panic!("{}", why.explain(&Letters)))
            .set_aside;
        assert_eq!(
            recovered.iter().map(|(_, at)| at).collect::<Vec<_>>(),
            [&EditsAt::Stash { n: 0 }],
            "{older:?}: the relaunch offers it"
        );
        drop(ctx);
        drop(journal);

        run_undo(&repo, NOW + 2, false).unwrap_or_else(|why| panic!("{}", why.explain(&Letters)));
        assert_eq!(whole(&repo), before, "{older:?}: the edits are back");
        assert_eq!(
            repo.git(&["stash", "list"]),
            "",
            "{older:?}: the stash list is empty"
        );
        assert!(
            repo.read("f.txt").windows(5).any(|w| w == b"five\n"),
            "{older:?}: the edit's bytes are on disk"
        );
    }
}

/// A set-aside and a backup made by this build carry `mergewright`.
#[test]
fn new_writes_say_mergewright() {
    assert_eq!(MESSAGE_TAG, "mergewright");
    let (repo, _before, _stash) = interrupted_set_aside();
    let messages = stash_messages(&repo);
    let [listed] = messages.as_slice() else {
        panic!("one stash: {messages:?}");
    };
    assert!(
        listed.starts_with("mergewright: your edits, set aside for "),
        "{listed}"
    );
    let entries = journal_entries(&repo);
    let backup = entries[0]
        .backup_ref
        .as_deref()
        .expect("the switch took a backup");
    let subject = repo.git(&["log", "-1", "--format=%s", backup]);
    assert!(subject.starts_with("mergewright "), "{subject}");
    assert_eq!(stash_commits(&repo).len(), 1);
}
