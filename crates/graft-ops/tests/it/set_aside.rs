//! A rebase or checkout over changed tracked files sets them aside and puts
//! them back, in one journal entry (ADR-0045 §4–§7, m6-conflicts/73).
//!
//! Every claim is read back from git, the files on disk or the journal file:
//! the stash list by commit id, the index with `ls-files -s`, the tree byte
//! for byte (`support::state_of`). The one promise under test is that the
//! edits are always in the tree or in the stash list, never in neither.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use graft_core::journal::{EntryStatus, HeadRef, JournalEntry, PutBack, PutBackWhy};
use graft_core::oid::Oid;
use graft_git::run::Stop;
use graft_git::state::InProgress;
use graft_ops::apply::{
    Applied, ApplyError, Resume, came_back_from_index, came_back_words, recover_interrupted, resume,
};
use graft_ops::clash::live_clash;
use graft_ops::facts::read_facts;
use graft_ops::journal::journal_path;
use graft_ops::operation::{CheckoutParams, CheckoutTarget, Operation, RebaseParams, Source, Tip};

use crate::support::{
    NOW, Never, RepoState, TempRepo, context, git_version, journal_for, run_redo, run_undo,
    run_with_stop, state_of, try_run,
};

// ── repositories and operations ─────────────────────────────────────────

/// `main` and `feature` both hold `base.txt` and `a.txt`; `main` adds
/// `main.txt`, `feature` adds `feature.txt`. Nothing the edits below touch
/// differs between the two, so a set-aside puts back cleanly.
fn diverged() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("main.txt", b"main\n");
    repo.commit_all("two on main");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("feature.txt", b"feature\n");
    repo.commit_all("one on feature");
    repo.git(&["switch", "-q", "main"]);
    repo
}

pub(crate) fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a full id")
}

fn head_tip(repo: &TempRepo) -> Tip {
    let name = repo.git(&["symbolic-ref", "HEAD"]);
    Tip {
        oid: id(repo, &name),
        name,
    }
}

/// Rebase the current branch onto `onto`.
fn rebase_onto(repo: &TempRepo, onto: &str) -> Operation {
    Operation::Rebase(RebaseParams::new(
        head_tip(repo),
        Source::Branch(onto.to_owned()),
        id(repo, onto),
    ))
}

/// Check out `to` from the current branch.
fn checkout_to(repo: &TempRepo, to: &str) -> Operation {
    Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch(to.to_owned()),
        head_before: HeadRef::Branch(repo.git(&["symbolic-ref", "HEAD"])),
        on_edits: graft_ops::operation::OnEdits::SetAside,
    })
}

/// The two kinds that set aside, each on `diverged()` ready to run: a
/// rebase of `feature` onto `main`, a checkout of `feature` from `main`.
#[derive(Clone, Copy, Debug)]
enum Kind {
    Rebase,
    Checkout,
}

impl Kind {
    const BOTH: [Kind; 2] = [Kind::Rebase, Kind::Checkout];

    /// Put the repository on the branch this kind starts from.
    fn start(self, repo: &TempRepo) {
        if let Kind::Rebase = self {
            repo.git(&["switch", "-q", "feature"]);
        }
    }

    fn op(self, repo: &TempRepo) -> Operation {
        match self {
            Kind::Rebase => rebase_onto(repo, "refs/heads/main"),
            Kind::Checkout => checkout_to(repo, "refs/heads/feature"),
        }
    }
}

/// One shape of uncommitted work, as the round-trip table names it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Tree {
    /// `base.txt` staged one way and edited further unstaged, plus an
    /// untracked file.
    StagedUnstagedUntracked,
    /// A new file, staged.
    StagedNewFile,
    /// `a.txt` deleted with `git rm`.
    Deletion,
    /// `a.txt` renamed with `git mv`.
    Rename,
    /// `git rm --cached a.txt`: untracked from the index, kept on disk
    /// (review m7-28 Q1).
    RmCached,
    /// `base.txt` staged, then the worktree taken back to `HEAD`'s: only
    /// the index holds the change (review m7-06b C2).
    IndexOnly,
    /// `a.txt` made executable in the index alone.
    ModeOnly,
}

impl Tree {
    /// Every shape, in the table's order.
    pub(crate) const ALL: [Tree; 7] = [
        Tree::StagedUnstagedUntracked,
        Tree::StagedNewFile,
        Tree::Deletion,
        Tree::Rename,
        Tree::RmCached,
        Tree::IndexOnly,
        Tree::ModeOnly,
    ];

    pub(crate) fn make(self, repo: &TempRepo) {
        match self {
            Tree::StagedUnstagedUntracked => {
                repo.write("base.txt", b"staged\n");
                repo.git(&["add", "base.txt"]);
                repo.write("base.txt", b"staged\nand unstaged\n");
                repo.write("junk.tmp", b"untracked\n");
            }
            Tree::StagedNewFile => {
                repo.write("n.txt", b"new\n");
                repo.git(&["add", "n.txt"]);
            }
            Tree::Deletion => {
                repo.git(&["rm", "-q", "a.txt"]);
            }
            Tree::Rename => {
                repo.git(&["mv", "a.txt", "b.txt"]);
            }
            Tree::RmCached => {
                repo.git(&["rm", "-q", "--cached", "a.txt"]);
            }
            Tree::IndexOnly => {
                repo.write("base.txt", b"staged\n");
                repo.git(&["add", "base.txt"]);
                repo.write("base.txt", b"base\n");
            }
            Tree::ModeOnly => {
                repo.git(&["update-index", "--chmod=+x", "--", "a.txt"]);
            }
        }
    }
}

// ── reading the repository back ─────────────────────────────────────────

/// The stash list as commit ids, newest first.
pub(crate) fn stash_commits(repo: &TempRepo) -> Vec<Oid> {
    repo.git(&["stash", "list", "--format=%H"])
        .lines()
        .map(|line| Oid::from_hex(line).expect("a full id"))
        .collect()
}

/// The stash list with its messages, for exact before/after comparisons.
fn stash_list(repo: &TempRepo) -> String {
    repo.git(&["stash", "list", "--format=%H %gs"])
}

/// Everything a round trip must put back: refs, `HEAD`, the index (staged
/// split included), every file's bytes, and the stash list.
pub(crate) fn whole(repo: &TempRepo) -> (RepoState, String) {
    (state_of(repo), stash_list(repo))
}

pub(crate) fn journal_entries(repo: &TempRepo) -> Vec<JournalEntry> {
    let handle = repo.open();
    journal_for(&handle).entries().to_vec()
}

pub(crate) fn last_entry(repo: &TempRepo) -> JournalEntry {
    journal_entries(repo).pop().expect("an entry was written")
}

/// m8/14 item 4: a staged new file he deleted from disk comes back with the
/// put-back (git's `stash apply --index` reinstates the index entry and
/// checks it out). Nothing is lost; the line must say it. The read names it,
/// and only it: an added file still on disk is not "came back".
#[test]
fn a_staged_new_file_deleted_from_disk_is_named_as_back_from_the_index() {
    for kind in Kind::BOTH {
        let repo = diverged();
        kind.start(&repo);
        repo.write("notes.md", b"notes\n");
        repo.write("kept.md", b"kept\n");
        repo.git(&["add", "notes.md", "kept.md"]);
        std::fs::remove_file(repo.path().join("notes.md")).expect("removed");

        let applied = try_run(&repo, &kind.op(&repo), NOW)
            .unwrap_or_else(|why| panic!("{kind:?}: {}", why.explain()));

        assert_eq!(
            repo.read("notes.md"),
            b"notes\n",
            "{kind:?}: git put it back"
        );
        let stash = set_aside_of(&applied.entry);
        let before = applied.entry.worktree_before.expect("a before-snapshot");
        let came_back = came_back_from_index(&repo.open(), stash, before)
            .unwrap_or_else(|why| panic!("{kind:?}: {why}"));
        assert_eq!(came_back, [PathBuf::from("notes.md")], "{kind:?}");
        // The operation's own record carries them, for the landing line.
        assert_eq!(applied.came_back, came_back, "{kind:?}: on Applied");
        assert_eq!(
            came_back_words(&came_back).as_deref(),
            Some("notes.md came back from the index"),
        );
    }
}

/// The set-aside stash an entry recorded.
pub(crate) fn set_aside_of(entry: &JournalEntry) -> Oid {
    match entry.put_back {
        Some(PutBack {
            stash,
            why: PutBackWhy::SetAside,
        }) => stash,
        ref other => panic!("the entry records no set-aside: {other:?}"),
    }
}

/// Whether `path` holds `bytes` in the working tree, or in a stash still in
/// the list (a stash commit's own tree is the working tree it saved,
/// staged new files included).
pub(crate) fn edit_is_somewhere(repo: &TempRepo, path: &str, bytes: &[u8]) -> bool {
    let in_tree = repo.path().join(path).exists() && repo.read(path) == bytes;
    let expected = String::from_utf8_lossy(bytes).trim_end().to_owned();
    in_tree
        || stash_commits(repo).iter().any(|commit| {
            repo.try_git(&["show", &format!("{commit}:{path}")])
                .is_ok_and(|shown| shown == expected)
        })
}

pub(crate) fn resume_with(repo: &TempRepo, how: Resume, at: i64) -> Result<Applied, ApplyError> {
    let handle = repo.open();
    // A fresh journal read: `put_back` must come from the Pending line on
    // disk, not from anything kept in memory since the stop.
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, at, &never);
    resume(&mut ctx, how)
}

// ── the sequence, clean ─────────────────────────────────────────────────

/// `git status --porcelain` as its lines, one per path (v1: a staged
/// rename reads `R  a.txt -> b.txt`).
pub(crate) fn porcelain(repo: &TempRepo) -> Vec<String> {
    repo.git(&["status", "--porcelain"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// How many staged entries lost their staging, read from git alone: of the
/// paths staged `before` (a rename's old and new path both), those that
/// `after` lists changed in the working tree with nothing staged.
pub(crate) fn staging_lost(before: &[String], after: &[String]) -> usize {
    let staged_before: Vec<&str> = before
        .iter()
        .filter(|line| !line.starts_with(' ') && !line.starts_with('?'))
        .flat_map(|line| line[3..].split(" -> "))
        .collect();
    after
        .iter()
        .filter(|line| line.starts_with(' '))
        .filter(|line| staged_before.contains(&&line[3..]))
        .count()
}

impl Tree {
    /// Each edited file as the put-back must leave it: `Some(bytes)` on
    /// disk, `None` gone.
    pub(crate) fn edits(self) -> &'static [(&'static str, Option<&'static [u8]>)] {
        match self {
            Tree::StagedUnstagedUntracked => &[
                ("base.txt", Some(b"staged\nand unstaged\n")),
                ("junk.tmp", Some(b"untracked\n")),
            ],
            Tree::StagedNewFile => &[("n.txt", Some(b"new\n"))],
            Tree::Deletion => &[("a.txt", None)],
            Tree::Rename => &[("a.txt", None), ("b.txt", Some(b"a\n"))],
            Tree::RmCached | Tree::ModeOnly => &[("a.txt", Some(b"a\n"))],
            Tree::IndexOnly => &[("base.txt", Some(b"base\n"))],
        }
    }
}

/// What he staged and what he did not, as diffs against `HEAD` (blob ids
/// and modes included): the same before and after a put-back that keeps
/// the index, since neither branch touches the edited files.
fn split(repo: &TempRepo) -> (String, String) {
    (
        repo.git(&["diff", "--cached", "--no-renames"]),
        repo.git(&["diff", "--no-renames"]),
    )
}

/// ADR-0045 §7, for one kind over one tree:
/// - after the put-back, before any undo: every edited file's bytes are
///   back, the staged split is as before (`git status`, and [`split`]'s
///   diffs — every put-back keeps the index, the 2026-09-26 b amendment),
///   the staging git lost is none ([`staging_lost`], 83 M1; the always-0
///   `SetAside::staged` it was once checked against went in 30c), and the
///   stash list is as before;
/// - undo leaves refs, `HEAD`, index, files and stash list exactly as
///   before; redo exactly as after.
fn round_trip(kind: Kind, tree: Tree) {
    let repo = diverged();
    kind.start(&repo);
    tree.make(&repo);
    let what = format!("{kind:?} over {tree:?}");
    let op = kind.op(&repo);
    let before = whole(&repo);
    let status_before = porcelain(&repo);
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("the facts read");
    assert!(op.sets_aside(&facts).is_some(), "{what}: the tree is dirty");
    let split_before = split(&repo);

    let applied =
        try_run(&repo, &op, NOW).unwrap_or_else(|why| panic!("{what}: {}", why.explain()));
    assert_eq!(applied.entry.status, EntryStatus::Committed, "{what}");
    set_aside_of(&applied.entry);
    let after = whole(&repo);
    assert_ne!(after.0, before.0, "{what}: the operation changed nothing");
    assert_eq!(after.1, before.1, "{what}: the stash list is as before");
    for (path, bytes) in tree.edits() {
        match bytes {
            Some(bytes) => assert_eq!(repo.read(path), *bytes, "{what}: {path} is back"),
            None => assert!(!repo.exists(path), "{what}: {path} is gone again"),
        }
    }
    let status_after = porcelain(&repo);
    assert_eq!(status_after, status_before, "{what}: the status as before");
    assert_eq!(
        split(&repo),
        split_before,
        "{what}: the staged split as before"
    );
    assert_eq!(
        staging_lost(&status_before, &status_after),
        0,
        "{what}: no staging lost ({status_before:?} → {status_after:?})"
    );

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{what}: undo: {}", why.explain()));
    assert_eq!(whole(&repo), before, "{what}: undo put it back exactly");

    run_redo(&repo, NOW + 2, false).unwrap_or_else(|why| panic!("{what}: redo: {}", why.explain()));
    assert_eq!(whole(&repo), after, "{what}: redo put it forward exactly");
}

// One test per kind and tree, so they run in parallel (73 review, m9).

#[test]
fn rebase_round_trips_staged_unstaged_untracked() {
    round_trip(Kind::Rebase, Tree::StagedUnstagedUntracked);
}

#[test]
fn rebase_round_trips_a_staged_new_file() {
    round_trip(Kind::Rebase, Tree::StagedNewFile);
}

#[test]
fn rebase_round_trips_a_deletion() {
    round_trip(Kind::Rebase, Tree::Deletion);
}

#[test]
fn rebase_round_trips_a_rename() {
    round_trip(Kind::Rebase, Tree::Rename);
}

#[test]
fn rebase_round_trips_an_rm_cached() {
    round_trip(Kind::Rebase, Tree::RmCached);
}

#[test]
fn rebase_round_trips_index_only_content() {
    round_trip(Kind::Rebase, Tree::IndexOnly);
}

#[test]
fn rebase_round_trips_a_staged_mode_only_change() {
    round_trip(Kind::Rebase, Tree::ModeOnly);
}

#[test]
fn checkout_round_trips_an_rm_cached() {
    round_trip(Kind::Checkout, Tree::RmCached);
}

#[test]
fn checkout_round_trips_index_only_content() {
    round_trip(Kind::Checkout, Tree::IndexOnly);
}

#[test]
fn checkout_round_trips_a_staged_mode_only_change() {
    round_trip(Kind::Checkout, Tree::ModeOnly);
}

#[test]
fn checkout_round_trips_staged_unstaged_untracked() {
    round_trip(Kind::Checkout, Tree::StagedUnstagedUntracked);
}

#[test]
fn checkout_round_trips_a_staged_new_file() {
    round_trip(Kind::Checkout, Tree::StagedNewFile);
}

#[test]
fn checkout_round_trips_a_deletion() {
    round_trip(Kind::Checkout, Tree::Deletion);
}

#[test]
fn checkout_round_trips_a_rename() {
    round_trip(Kind::Checkout, Tree::Rename);
}

/// Each command line of `entry`, named by the set-aside step it is, or "the
/// operation" for the operation's own lines.
pub(crate) fn step_names(entry: &JournalEntry) -> Vec<&'static str> {
    const STEPS: [&str; 5] = [
        "stash create",
        "stash store",
        "reset --hard",
        "stash apply",
        "stash drop",
    ];
    entry
        .commands
        .iter()
        .map(|line| {
            STEPS
                .into_iter()
                .find(|step| line.contains(step))
                .unwrap_or("the operation")
        })
        .collect()
}

/// A clean put-back drops the set-aside stash by its commit id — an older
/// stash of the user's stays exactly where it was — and the entry is
/// `Committed`, pinning `S`, with every step's command on it in order.
#[test]
fn a_clean_put_back_drops_the_set_aside_stash() {
    for kind in Kind::BOTH {
        let repo = diverged();
        kind.start(&repo);
        repo.write("a.txt", b"an older stash\n");
        repo.git(&["stash", "push", "-q", "-m", "mine"]);
        repo.write("base.txt", b"edited\n");
        let before = stash_list(&repo);

        let applied = try_run(&repo, &kind.op(&repo), NOW)
            .unwrap_or_else(|why| panic!("{kind:?}: {}", why.explain()));
        let entry = last_entry(&repo);
        assert_eq!(
            entry, applied.entry,
            "{kind:?}: the file holds what came back"
        );
        assert_eq!(entry.status, EntryStatus::Committed);
        let stash = set_aside_of(&entry);
        assert!(entry.pins.contains(&stash), "{kind:?}: S is pinned");
        assert!(entry.touches.refs.contains(&"refs/stash".to_owned()));
        assert_eq!(stash_list(&repo), before, "{kind:?}: S dropped, mine kept");
        assert_eq!(
            repo.read("base.txt"),
            b"edited\n",
            "{kind:?}: the edit is back"
        );

        assert_eq!(
            step_names(&entry),
            [
                "stash create",
                "stash store",
                "reset --hard",
                "the operation",
                "stash apply",
                "stash drop"
            ],
            "{kind:?}: {:?}",
            entry.commands
        );
        let stored = entry
            .commands
            .iter()
            .find(|line| line.contains("stash store"))
            .unwrap();
        assert!(
            stored.contains("graft: your edits, set aside for"),
            "{stored}"
        );
    }
}

// ── the sequence, when the put-back does not go cleanly ─────────────────

/// `main` and `feature` both change `base.txt`, and the user has edited it
/// on `main`: checking out `feature` puts the edit back over feature's line.
pub(crate) fn clashing_checkout() -> (TempRepo, Operation) {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("base.txt", b"feature's line\n");
    repo.commit_all("feature changes base");
    repo.git(&["switch", "-q", "main"]);
    repo.write("base.txt", b"my edit\n");
    let op = checkout_to(&repo, "refs/heads/feature");
    (repo, op)
}

/// The put-back clashed: `Clashed { why: SetAside }`, the entry `Committed`
/// with an unmerged index after, `S` still listed — and `live_clash` over
/// the journal the operation wrote finds it.
#[test]
fn a_clashing_put_back_is_clashed_and_keeps_the_stash() {
    let (repo, op) = clashing_checkout();
    let (entry, clash) = match try_run(&repo, &op, NOW) {
        Err(ApplyError::Clashed { entry, clash }) => (*entry, clash),
        other => panic!("the put-back should clash: {other:?}"),
    };
    assert_eq!(clash.why, PutBackWhy::SetAside);
    assert_eq!(clash.paths, vec![PathBuf::from("base.txt")]);
    let stash = set_aside_of(&entry);
    assert_eq!(clash.stash, stash);
    assert_eq!(entry.status, EntryStatus::Committed);
    assert!(entry.index_after.is_some_and(|index| index.is_unmerged()));
    assert!(
        !repo.git(&["ls-files", "-u"]).is_empty(),
        "git says unmerged"
    );
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/feature");
    assert_eq!(stash_commits(&repo), vec![stash], "S kept");

    let entries = journal_entries(&repo);
    let handle = repo.open();
    let scratch = repo.path().join(".git/graft");
    let index_now =
        graft_ops::snapshot::index_state(&handle, &scratch, &Never).expect("the index reads");
    let found = live_clash(&entries, None, &index_now, &[(stash, 0)]).expect("the clash is live");
    assert_eq!(found.entry, entry.id);
    assert_eq!(found.why, PutBackWhy::SetAside);
    assert_eq!(found.index, Some(0));
}

/// Install `script` as a one-shot `post-checkout` hook: it runs in the
/// working tree's root while the checkout (step 5) runs, after the set-aside
/// and before the put-back, then deletes itself — so an undo's own checkout
/// does not run it again. It lives in the git dir, out of the tree's files.
pub(crate) fn post_checkout(repo: &TempRepo, script: &str) {
    let hooks = repo.path().join(".git/test-hooks");
    std::fs::create_dir_all(&hooks).expect("the hooks dir is made");
    let hook = hooks.join("post-checkout");
    std::fs::write(&hook, format!("#!/bin/sh\nrm -f \"$0\"\n{script}\n"))
        .expect("the hook is written");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
        .expect("the hook is executable");
    repo.git(&["config", "core.hooksPath", hooks.to_str().expect("utf-8")]);
}

/// A checkout whose put-back git refuses with nothing unmerged: the user's
/// staged new `n.txt`, and a hook that drops an untracked `n.txt` in the
/// way after the switch.
pub(crate) fn put_back_refused() -> (TempRepo, Operation) {
    let repo = diverged();
    repo.write("n.txt", b"new\n");
    repo.git(&["add", "n.txt"]);
    post_checkout(&repo, "echo in the way > n.txt");
    let op = checkout_to(&repo, "refs/heads/feature");
    (repo, op)
}

/// A put-back git refuses with nothing unmerged — an untracked file where
/// the edits bring back a staged new file — keeps `S` and says where the
/// edits are. The index decides, not the exit code (ADR-0045 §7).
#[test]
fn a_put_back_that_fails_without_conflict_keeps_the_stash() {
    let (repo, op) = put_back_refused();
    let (entry, words) = match try_run(&repo, &op, NOW) {
        Err(error @ ApplyError::Git { .. }) => {
            let words = error.explain();
            let ApplyError::Git { entry, .. } = error else {
                unreachable!()
            };
            (*entry, words)
        }
        other => panic!("the put-back should fail: {other:?}"),
    };
    let stash = set_aside_of(&entry);
    assert_eq!(stash_commits(&repo), vec![stash], "S kept");
    assert!(repo.git(&["ls-files", "-u"]).is_empty(), "nothing unmerged");
    assert!(words.contains("stash@{0}"), "{words}");
    assert!(
        matches!(&entry.status, EntryStatus::Aborted { reason } if reason.contains("stash@{0}")),
        "{:?}",
        entry.status
    );
    assert!(edit_is_somewhere(&repo, "n.txt", b"new\n"));
}

/// `u` after that refused put-back (73 review, m8): the tree, the index, the
/// branch and the stash list exactly as before the checkout (ADR-0045 §5,
/// "`u` restores the backup").
#[test]
fn undo_after_a_refused_put_back_restores_everything() {
    let (repo, op) = put_back_refused();
    let before = whole(&repo);
    let Err(ApplyError::Git { .. }) = try_run(&repo, &op, NOW) else {
        panic!("the put-back should fail");
    };
    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("undo: {}", why.explain()));
    assert_eq!(whole(&repo), before, "undo put it back exactly");
}

/// The outcome line cannot be written after the put-back clashed (the
/// journal file replaced by a directory while the checkout runs): the error
/// still says where the edits are (73 review, m1).
#[test]
fn a_journal_failure_after_a_clash_names_the_stash() {
    let (repo, op) = clashing_checkout();
    post_checkout(
        &repo,
        "j=$(git rev-parse --git-path graft/undo-journal.jsonl) && rm -f \"$j\" && mkdir \"$j\"",
    );
    let error = match try_run(&repo, &op, NOW) {
        Err(error @ ApplyError::Journal { .. }) => error,
        other => panic!("the outcome line should fail: {other:?}"),
    };
    let words = error.explain();
    assert!(words.contains("stash@{0}"), "{words}");
    assert!(words.contains("clash"), "{words}");
    assert_eq!(stash_commits(&repo).len(), 1, "S kept");
}

/// The outcome line cannot be written after a clean put-back that brought
/// a staged-new file he deleted back from the index: the error names it,
/// since no landing line will (review m8-14 Major 3).
#[test]
fn a_journal_failure_after_a_clean_put_back_names_what_came_back() {
    let repo = diverged();
    repo.write("notes.md", b"notes\n");
    repo.git(&["add", "notes.md"]);
    std::fs::remove_file(repo.path().join("notes.md")).expect("removed");
    post_checkout(
        &repo,
        "j=$(git rev-parse --git-path graft/undo-journal.jsonl) && rm -f \"$j\" && mkdir \"$j\"",
    );
    let op = checkout_to(&repo, "refs/heads/feature");
    let error = match try_run(&repo, &op, NOW) {
        Err(error @ ApplyError::Journal { .. }) => error,
        other => panic!("the outcome line should fail: {other:?}"),
    };
    let words = error.explain();
    assert!(
        words.contains("notes.md came back from the index"),
        "{words}"
    );
    assert_eq!(repo.read("notes.md"), b"notes\n", "git put it back");
}

/// The status cannot be read after the put-back (a config value `git
/// status` refuses, set while the checkout runs): the error names
/// `stash@{n}`, not only the entry's reason (73 review, m2).
#[test]
fn an_unreadable_status_after_the_put_back_names_the_stash() {
    let repo = diverged();
    repo.write("base.txt", b"edited\n");
    post_checkout(&repo, "git config status.showUntrackedFiles bogus");
    let op = checkout_to(&repo, "refs/heads/feature");
    let error = try_run(&repo, &op, NOW).expect_err("the status should not read");
    let words = error.explain();
    assert!(words.contains("stash@{0}"), "{error:?}: {words}");
    assert!(words.contains("could not be read"), "{words}");
    assert!(edit_is_somewhere(&repo, "base.txt", b"edited\n"));
}

/// The stash list cannot be read to drop `S` after a clean put-back (its
/// reflog made unreadable while the checkout runs): the words say the edits
/// are back and the copy could not be found to drop — not that the status
/// was unread (73 review, m3).
#[test]
fn an_unlisted_set_aside_after_a_clean_put_back_says_the_edits_are_back() {
    let repo = diverged();
    repo.write("base.txt", b"edited\n");
    post_checkout(
        &repo,
        "chmod 000 \"$(git rev-parse --git-path logs/refs/stash)\"",
    );
    let op = checkout_to(&repo, "refs/heads/feature");
    let error = try_run(&repo, &op, NOW).expect_err("the drop should not happen");
    let words = error.explain();
    assert!(
        words.contains("your edits are back; the set-aside copy could not be found to drop"),
        "{error:?}: {words}"
    );
    assert!(!words.contains("status"), "{words}");
    assert_eq!(repo.read("base.txt"), b"edited\n", "the edit is back");
}

// ── where the edits are, when it did not go cleanly (ADR-0045 §5 amended) ─

/// The drop after a clean put-back fails (the stash reflog's directory made
/// read-only while the checkout runs, so `stash drop` cannot lock it): the
/// edits are back in the tree **and** the copy is still listed — `Back`, not
/// `Stash` (review m6-final-fixes-74 M1, path 1).
#[test]
fn a_drop_that_fails_after_a_clean_put_back_is_edits_back() {
    let repo = diverged();
    repo.write("base.txt", b"edited\n");
    post_checkout(&repo, "chmod 555 \"$(git rev-parse --git-path logs/refs)\"");
    let op = checkout_to(&repo, "refs/heads/feature");
    let answer = try_run(&repo, &op, NOW);
    let logs = repo.path().join(".git/logs/refs");
    std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o755))
        .expect("the reflog dir is writable again");
    let error = answer.expect_err("the drop should fail");
    assert_eq!(
        error.edits_at(),
        Some(graft_ops::EditsAt::Back { copy: Some(0) }),
        "{error:?}"
    );
    assert_eq!(
        repo.read("base.txt"),
        b"edited\n",
        "the edit is in the tree"
    );
    assert_eq!(stash_commits(&repo).len(), 1, "the copy is still listed");
    assert!(
        !error.to_string().contains("S p"),
        "no key in graft-ops' words: {error}"
    );
}

/// A stop after `stash store` and before `reset --hard` (step 4): the entry
/// closes, `S` is listed, and the backup has the tree — `Backup`, whatever
/// the list says (review M1, path 2).
#[test]
fn a_stop_after_the_store_is_edits_in_the_backup() {
    for kind in Kind::BOTH {
        let (repo, op) = dirty_for(kind);
        let stop = stop_before(&repo, 4);
        let error = run_with_stop(&repo, &op, NOW, &stop).expect_err("stopped at step 4");
        assert_eq!(
            error.edits_at(),
            Some(graft_ops::EditsAt::Backup { copy: Some(0) }),
            "{kind:?}: {error:?}"
        );
        assert_eq!(stash_commits(&repo).len(), 1, "{kind:?}: S listed");
        assert_eq!(
            repo.read("base.txt"),
            b"edited\n",
            "{kind:?}: tree untouched"
        );
    }
}

/// The operation itself fails (a `post-checkout` hook exits 1) **and** the
/// stash list cannot be read to drop the copy (the same hook makes the stash
/// reflog unreadable): the put-back ran clean, so the edits are back and the
/// copy was not found to drop — `Back { copy: None }` (`Kept::Unlisted`,
/// review M1, path 3). The injected fault: that hook.
#[test]
fn an_unfound_copy_after_a_failed_operation_is_edits_back() {
    let repo = diverged();
    repo.write("base.txt", b"edited\n");
    post_checkout(
        &repo,
        "chmod 000 \"$(git rev-parse --git-path logs/refs/stash)\"; exit 1",
    );
    let op = checkout_to(&repo, "refs/heads/feature");
    let answer = try_run(&repo, &op, NOW);
    let reflog = repo.path().join(".git/logs/refs/stash");
    std::fs::set_permissions(&reflog, std::fs::Permissions::from_mode(0o644))
        .expect("the reflog reads again");
    let error = answer.expect_err("the checkout's hook failed");
    assert!(matches!(error, ApplyError::Git { .. }), "{error:?}");
    assert_eq!(
        error.edits_at(),
        Some(graft_ops::EditsAt::Back { copy: None }),
        "{error:?}"
    );
    assert_eq!(repo.read("base.txt"), b"edited\n", "the edit is back");
    assert_eq!(stash_commits(&repo).len(), 1, "the copy was never dropped");
}

/// git refuses the put-back with nothing unmerged (an untracked file in the
/// way of a staged new one): the edits are **not** in the tree, only in the
/// listed copy — `Stash { n: 0 }`.
#[test]
fn a_put_back_that_fails_without_a_clash_is_edits_in_stash() {
    let (repo, op) = put_back_refused();
    let error = try_run(&repo, &op, NOW).expect_err("the put-back should fail");
    assert_eq!(
        error.edits_at(),
        Some(graft_ops::EditsAt::Stash { n: 0 }),
        "{error:?}"
    );
    assert_eq!(
        repo.read("n.txt"),
        b"in the way\n",
        "the tree lacks the edit"
    );
    assert_eq!(stash_commits(&repo).len(), 1, "S listed");
    // The error's own line is git's; where the edits are is `edits_at`, and
    // `explain` adds it in graft-ops' words.
    assert!(!error.to_string().contains("stash@{0}"), "{error}");
    assert!(error.explain().contains("stash@{0}"), "{}", error.explain());
}

/// `feature` and `main` rewrite the same line of `shared.txt`, so a rebase
/// of `feature` onto `main` stops on it; the user's own edit is to
/// `other.txt`, which neither side touches.
fn stopping_rebase() -> (TempRepo, Operation) {
    let repo = TempRepo::init();
    repo.write("shared.txt", b"base\n");
    repo.write("other.txt", b"other\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("shared.txt", b"main's line\n");
    repo.commit_all("two on main");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("shared.txt", b"feature's line\n");
    repo.commit_all("one on feature");
    repo.write("other.txt", b"my edit\n");
    let op = rebase_onto(&repo, "refs/heads/main");
    (repo, op)
}

/// Run `op`, which must stop on a conflict, and check the edits wait aside:
/// the entry `Pending` with `put_back` on its line, `S` listed, `other.txt`
/// as committed.
fn stopped_with_edits_aside(repo: &TempRepo, op: &Operation) -> JournalEntry {
    let entry = match try_run(repo, op, NOW) {
        Err(ApplyError::Conflicted { entry, kind, .. }) => {
            assert_eq!(kind, InProgress::Rebase);
            *entry
        }
        other => panic!("the rebase should stop: {other:?}"),
    };
    assert_eq!(entry.status, EntryStatus::Pending);
    let stash = set_aside_of(&entry);
    assert_eq!(stash_commits(repo), vec![stash], "S waits in the list");
    assert_eq!(repo.read("other.txt"), b"other\n", "the edit is aside");
    assert_eq!(
        set_aside_of(&last_entry(repo)),
        stash,
        "the Pending line has it"
    );
    entry
}

/// A rebase that stops keeps the edits aside while it is stopped — entry
/// `Pending` with `put_back`, `S` listed — and puts them back when it ends:
/// after `--continue`, after `--abort` (on the original tip) and after a
/// final `--skip`. Each resume reads a freshly opened journal.
#[test]
fn a_rebase_that_stops_keeps_the_edits_aside_until_it_ends() {
    for how in [Resume::Continue, Resume::Abort, Resume::Skip] {
        let (repo, op) = stopping_rebase();
        let original = id(&repo, "refs/heads/feature");
        let entry = stopped_with_edits_aside(&repo, &op);

        if how == Resume::Continue {
            repo.write("shared.txt", b"resolved\n");
            repo.git(&["add", "shared.txt"]);
        }
        let applied = resume_with(&repo, how.clone(), NOW + 1)
            .unwrap_or_else(|why| panic!("{how:?}: {}", why.explain()));
        assert!(
            !repo.exists(".git/rebase-merge"),
            "{how:?}: the rebase ended"
        );
        assert_eq!(
            repo.read("other.txt"),
            b"my edit\n",
            "{how:?}: the edit is back"
        );
        assert!(stash_commits(&repo).is_empty(), "{how:?}: S dropped");
        let closed = last_entry(&repo);
        assert_eq!(closed.id, entry.id, "{how:?}: one entry");
        assert_eq!(closed, applied.entry);
        if how == Resume::Abort {
            assert_eq!(
                id(&repo, "refs/heads/feature"),
                original,
                "on the original tip"
            );
            assert_eq!(
                closed.status,
                EntryStatus::Aborted {
                    reason: "aborted".to_owned()
                }
            );
        } else {
            assert_eq!(closed.status, EntryStatus::Committed, "{how:?}");
            assert_ne!(
                id(&repo, "refs/heads/feature"),
                original,
                "{how:?}: rebased"
            );
        }
        assert!(
            closed
                .commands
                .iter()
                .any(|line| line.contains("stash apply")),
            "{how:?}: {:?}",
            closed.commands
        );
    }
}

/// git refuses the rebase before it starts (a `pre-rebase` hook says no):
/// the edits go back onto the same tip, then the refusal. An untracked file
/// in the way of `main`'s tree no longer gets this far: Graft refuses it
/// before the set-aside (ADR-0049, `in_the_way::rebase_over_an_ignored_file_is_refused`).
#[test]
fn a_refused_rebase_puts_the_edits_back_on_the_same_tip() {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("u.txt", b"main's\n");
    repo.commit_all("main adds u");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("feature.txt", b"f\n");
    repo.commit_all("one on feature");
    repo.write("base.txt", b"edited\n");
    let hooks = repo.path().join(".git/test-hooks");
    std::fs::create_dir_all(&hooks).expect("the hooks dir is made");
    let hook = hooks.join("pre-rebase");
    std::fs::write(&hook, b"#!/bin/sh\nexit 1\n").expect("the hook is written");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
        .expect("the hook is executable");
    repo.git(&["config", "core.hooksPath", hooks.to_str().expect("utf-8")]);
    let tip = id(&repo, "refs/heads/feature");
    let op = rebase_onto(&repo, "refs/heads/main");

    let entry = match try_run(&repo, &op, NOW) {
        Err(ApplyError::Git { entry, .. }) => *entry,
        other => panic!("git should refuse the rebase: {other:?}"),
    };
    assert!(matches!(entry.status, EntryStatus::Aborted { .. }));
    set_aside_of(&entry);
    assert_eq!(id(&repo, "refs/heads/feature"), tip, "the same tip");
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/feature");
    assert_eq!(repo.read("base.txt"), b"edited\n", "the edit is back");
    assert!(stash_commits(&repo).is_empty(), "S dropped");
}

// ── stops ───────────────────────────────────────────────────────────────

/// A `Stop` that latches once `fired` says the step before it has landed —
/// keyed to what the step leaves on disk, never to a poll count (the
/// reasoning of `support::StopWhenGone`).
pub(crate) struct StopWhen {
    fired: Box<dyn Fn() -> bool + Sync>,
    pub(crate) latched: AtomicBool,
}

impl StopWhen {
    pub(crate) fn new(fired: impl Fn() -> bool + Sync + 'static) -> StopWhen {
        StopWhen {
            fired: Box::new(fired),
            latched: AtomicBool::new(false),
        }
    }
}

impl Stop for StopWhen {
    fn should_stop(&self) -> bool {
        if self.latched.load(Ordering::Relaxed) || (self.fired)() {
            self.latched.store(true, Ordering::Relaxed);
            return true;
        }
        false
    }
}

/// How many loose objects the repository holds.
pub(crate) fn loose_objects(git_dir: &Path) -> usize {
    let Ok(fans) = std::fs::read_dir(git_dir.join("objects")) else {
        return 0;
    };
    fans.flatten()
        .filter(|fan| fan.file_name().len() == 2)
        .map(|fan| std::fs::read_dir(fan.path()).map_or(0, Iterator::count))
        .sum()
}

/// Hold git inside its `post-checkout` hook — after it has moved `HEAD`,
/// mid-operation — until the runner's stop kills it (its parent pid gone),
/// so a stop keyed to `HEAD` moving always lands while git is still running
/// rather than racing git to the finish (M7/02b). Bounded at ten seconds,
/// so a stop that never fires fails the test instead of hanging it.
pub(crate) fn hold_git_until_killed(repo: &TempRepo) {
    post_checkout(
        repo,
        "p=$PPID; n=0\n\
         while kill -0 \"$p\" 2>/dev/null && [ \"$n\" -lt 1000 ]; do sleep 0.01; n=$((n + 1)); done",
    );
}

/// A stop that fires just before set-aside step `step` (1–6) runs, keyed
/// to what the step before it leaves on disk. Step 6's is keyed to the
/// operation (step 5) moving `HEAD`, with git held there until it is killed
/// ([`hold_git_until_killed`]): there is no poll between 5 and 6, so "before
/// the put-back" is "during the operation".
fn stop_before(repo: &TempRepo, step: u8) -> StopWhen {
    if step >= 6 {
        hold_git_until_killed(repo);
    }
    let root = repo.path().to_path_buf();
    let git_dir = root.join(".git");
    let journal = journal_path(&repo.open()).expect("the journal path");
    let objects = loose_objects(&git_dir);
    let start_head = std::fs::read(git_dir.join("HEAD")).expect("HEAD reads");
    let start_tip = id(repo, "HEAD");
    match step {
        1 => StopWhen::new(|| true),
        // `stash create` has written its objects.
        2 => StopWhen::new(move || loose_objects(&git_dir) > objects),
        // `begin` has written the Pending line.
        3 => StopWhen::new(move || std::fs::read(&journal).is_ok_and(|bytes| !bytes.is_empty())),
        // `stash store` has moved `refs/stash`.
        4 => StopWhen::new(move || git_dir.join("refs/stash").exists()),
        // `reset --hard` has put `base.txt` back.
        5 => StopWhen::new(move || {
            std::fs::read(root.join("base.txt")).is_ok_and(|bytes| bytes == b"base\n")
        }),
        // The operation has moved `HEAD` or the branch.
        _ => StopWhen::new(move || {
            std::fs::read(root.join(".git/HEAD")).ok() != Some(start_head.clone())
                || std::process::Command::new("git")
                    .arg("-C")
                    .arg(&root)
                    .args(["rev-parse", "HEAD"])
                    .output()
                    .is_ok_and(|out| {
                        String::from_utf8_lossy(&out.stdout).trim() != start_tip.to_string()
                    })
        }),
    }
}

/// What a stop before one step leaves behind (73 review, M2): read from the
/// journal, the stash list and the tree, so a stop drifting to a
/// neighbouring step changes at least one of them.
#[derive(Debug, PartialEq, Eq)]
struct Left {
    /// The entry written, if any: `true` when it closed `Aborted`.
    aborted: Option<bool>,
    /// The step name ([`step_names`]) of the entry's last command.
    last: Option<&'static str>,
    stashes: usize,
    /// `base.txt` still edited on disk (else the tree was reset clean).
    dirty: bool,
}

impl Left {
    fn read(repo: &TempRepo) -> Left {
        let entries = journal_entries(repo);
        assert!(entries.len() <= 1, "one entry at most: {entries:?}");
        let entry = entries.first();
        Left {
            aborted: entry.map(|entry| matches!(entry.status, EntryStatus::Aborted { .. })),
            last: entry.and_then(|entry| step_names(entry).last().copied()),
            stashes: stash_commits(repo).len(),
            dirty: repo.read("base.txt") == b"edited\n",
        }
    }

    /// What each stop leaves, as ADR-0045 §5 and the 73 review's probe
    /// describe it.
    fn expected(kind: Kind, step: u8) -> Left {
        let (aborted, last, stashes, dirty) = match (kind, step) {
            // Before `stash create`, or before `begin`: nothing written.
            (_, 1 | 2) => (None, None, 0, true),
            // Before `stash store`: the store never starts, so the entry is
            // closed after the create, `S` objects only, the tree untouched.
            (_, 3) => (Some(true), Some("stash create"), 0, true),
            // Before `reset --hard`: `S` listed, the tree untouched.
            (_, 4) => (Some(true), Some("stash store"), 1, true),
            // Before the operation: the reset is done and the operation
            // never ran, so the tree is clean and the edits are only in `S`.
            (_, 5) => (Some(true), Some("reset --hard"), 1, false),
            // A rebase killed part-way is git's live rebase: held `Pending`
            // for `resume` (ADR-0045 §4, "stops … at 5"), `S` waiting in the
            // list, the tree as the reset left it. The Pending line on disk
            // carries no commands; they are held in memory.
            (Kind::Rebase, _) => (Some(false), None, 1, false),
            // A checkout killed part-way leaves no live state: the put-back
            // never stops — `S` back and dropped.
            (Kind::Checkout, _) => (Some(true), Some("stash drop"), 0, true),
        };
        Left {
            aborted,
            last,
            stashes,
            dirty,
        }
    }
}

/// A repository dirty for [`Kind`], ready to stop: `base.txt` edited, `n.txt`
/// a staged new file.
fn dirty_for(kind: Kind) -> (TempRepo, Operation) {
    let repo = diverged();
    kind.start(&repo);
    repo.write("base.txt", b"edited\n");
    repo.write("n.txt", b"new\n");
    repo.git(&["add", "n.txt"]);
    let op = kind.op(&repo);
    (repo, op)
}

/// ADR-0045 §5, §7: a stop before each of steps 1–6 in turn leaves every
/// edit in the tree or in the stash list, never in neither — read from both,
/// for a rebase and a checkout — and leaves exactly what that step leaves
/// ([`Left::expected`]).
#[test]
fn a_stop_before_each_step_loses_nothing() {
    for kind in Kind::BOTH {
        for step in 1..=6 {
            let (repo, op) = dirty_for(kind);
            let stop = stop_before(&repo, step);
            let result = run_with_stop(&repo, &op, NOW, &stop);
            let what = format!(
                "{kind:?}, stop before step {step}: {:?}",
                result.as_ref().err()
            );
            assert!(stop.latched.load(Ordering::Relaxed), "{what}: never fired");
            assert_eq!(Left::read(&repo), Left::expected(kind, step), "{what}");
            assert!(
                edit_is_somewhere(&repo, "base.txt", b"edited\n"),
                "{what}: base.txt lost"
            );
            assert!(
                edit_is_somewhere(&repo, "n.txt", b"new\n"),
                "{what}: n.txt lost"
            );
            // No entry is left open unless git itself says the rebase is live.
            let live = repo.exists(".git/rebase-merge") || repo.exists(".git/rebase-apply");
            if !live {
                assert!(
                    journal_entries(&repo)
                        .iter()
                        .all(|entry| entry.status != EntryStatus::Pending),
                    "{what}: an entry was left open"
                );
            }
        }
    }
}

/// A stop that kills a set-aside rebase part-way leaves git's rebase live:
/// `apply` holds it as the stop it is — `Conflicted`, the entry `Pending`
/// with `S` listed — rather than failing the status read after it on the
/// same stop (M7/02b: `Facts(Status(Run(Stopped)))`, the commands never
/// held). `a` then puts the edits back and drops `S`.
#[test]
fn a_stop_mid_rebase_is_held_and_abort_puts_the_edits_back() {
    let (repo, op) = dirty_for(Kind::Rebase);
    let stop = stop_before(&repo, 6);
    let (entry, paths) = match run_with_stop(&repo, &op, NOW, &stop) {
        Err(ApplyError::Conflicted { entry, kind, paths }) => {
            assert_eq!(kind, InProgress::Rebase);
            (*entry, paths)
        }
        other => panic!("the stopped rebase should be held: {other:?}"),
    };
    assert!(paths.is_empty(), "killed, not conflicted: {paths:?}");
    assert!(repo.exists(".git/rebase-merge"), "git's rebase is live");
    assert_eq!(entry.status, EntryStatus::Pending);
    assert!(
        entry.commands.iter().any(|line| line.contains("rebase")),
        "the commands are held: {:?}",
        entry.commands
    );
    let stash = set_aside_of(&entry);
    assert_eq!(stash_commits(&repo), vec![stash], "S waits in the list");
    assert!(edit_is_somewhere(&repo, "base.txt", b"edited\n"));
    assert!(edit_is_somewhere(&repo, "n.txt", b"new\n"));

    resume_with(&repo, Resume::Abort, NOW + 1)
        .unwrap_or_else(|why| panic!("abort: {}", why.explain()));
    assert!(!repo.exists(".git/rebase-merge"), "the rebase is gone");
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/feature");
    assert_eq!(repo.read("base.txt"), b"edited\n", "the edit is back");
    assert_eq!(repo.read("n.txt"), b"new\n", "the new file is back");
    assert!(stash_commits(&repo).is_empty(), "S dropped");
}

/// `u` after a stop at step 4 (`S` listed, tree untouched) or step 5 (tree
/// reset, edits only in `S`): the tree, the index and the stash list exactly
/// as before (73 review, m8; ADR-0045 §5, "4 or later").
#[test]
fn undo_after_a_stop_at_4_or_5_restores_everything() {
    for kind in Kind::BOTH {
        for step in [4, 5] {
            let (repo, op) = dirty_for(kind);
            let before = whole(&repo);
            let stop = stop_before(&repo, step);
            let result = run_with_stop(&repo, &op, NOW, &stop);
            let what = format!("{kind:?}, stop before step {step}");
            assert!(result.is_err(), "{what}: the stop is an error");
            assert_eq!(
                Left::read(&repo),
                Left::expected(kind, step),
                "{what}: the stop landed"
            );
            run_undo(&repo, NOW + 1, false)
                .unwrap_or_else(|why| panic!("{what}: undo: {}", why.explain()));
            assert_eq!(whole(&repo), before, "{what}: undo put it back exactly");
        }
    }
}

/// A relaunch after a kill: `recover_interrupted` closes the entry as it
/// always has, and the closed entry still carries `put_back`, so the app can
/// name the stash (ADR-0045 §5, 74).
#[test]
fn relaunch_after_a_kill_keeps_put_back_on_the_entry() {
    let (repo, op) = stopping_rebase();
    let Err(ApplyError::Conflicted { entry, .. }) = try_run(&repo, &op, NOW) else {
        panic!("the rebase should stop");
    };
    let stash = set_aside_of(&entry);
    // The process died and the rebase was abandoned in the shell: the
    // Pending line is all that is left.
    repo.git(&["rebase", "--abort"]);

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    let recovered = recover_interrupted(&mut ctx).expect("recovery runs");
    assert_eq!(recovered.closed, vec![entry.id.clone()]);
    let closed = last_entry(&repo);
    assert_eq!(closed.id, entry.id);
    assert!(matches!(closed.status, EntryStatus::Aborted { .. }));
    assert_eq!(
        set_aside_of(&closed),
        stash,
        "put_back survives the relaunch"
    );
    assert_eq!(stash_commits(&repo), vec![stash], "S is still listed");
}

// ── undo and redo after a clash ─────────────────────────────────────────

/// `u` after a clashed put-back is the abort A4 asked for: the tree, the
/// branch and the index as before the checkout, the edit back, and `S`
/// dropped as a duplicate of what came back (ADR-0045 §6).
#[test]
fn undo_after_a_clashed_put_back_drops_the_duplicate_stash() {
    let (repo, op) = clashing_checkout();
    let before = whole(&repo);
    let Err(ApplyError::Clashed { .. }) = try_run(&repo, &op, NOW) else {
        panic!("the put-back should clash");
    };
    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("undo: {}", why.explain()));
    assert_eq!(
        whole(&repo),
        before,
        "undo put it back exactly, stash list included"
    );
    assert!(stash_commits(&repo).is_empty(), "the duplicate is gone");
}

/// Redo after that undo puts the clash back and stores `S` again, since the
/// undo's own `refs/stash` moved (ADR-0045 §6, m6-conflicts/07 F2).
#[test]
fn redo_after_that_undo_stores_it_again() {
    let (repo, op) = clashing_checkout();
    let Err(ApplyError::Clashed { entry, .. }) = try_run(&repo, &op, NOW) else {
        panic!("the put-back should clash");
    };
    let stash = set_aside_of(&entry);
    let clashed = state_of(&repo);
    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("undo: {}", why.explain()));
    run_redo(&repo, NOW + 2, false).unwrap_or_else(|why| panic!("redo: {}", why.explain()));
    let redone = state_of(&repo);
    assert_eq!(redone.index, clashed.index, "the unmerged index is back");
    assert_eq!(redone.files, clashed.files, "the markers are back");
    assert_eq!(redone.head, clashed.head);
    assert_eq!(stash_commits(&repo), vec![stash], "S is listed again");
}
