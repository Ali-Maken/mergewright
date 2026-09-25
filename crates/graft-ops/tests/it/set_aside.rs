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
use graft_ops::apply::{Applied, ApplyError, Resume, recover_interrupted, resume};
use graft_ops::clash::live_clash;
use graft_ops::journal::journal_path;
use graft_ops::operation::{CheckoutParams, CheckoutTarget, Operation, RebaseParams, Source, Tip};

use crate::support::{
    NOW, Never, RepoState, TempRepo, context, journal_for, run_redo, run_undo, run_with_stop,
    state_of, try_run,
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

fn id(repo: &TempRepo, rev: &str) -> Oid {
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
    Operation::Rebase(RebaseParams {
        branch: head_tip(repo),
        onto: Source::Branch(onto.to_owned()),
        onto_tip: id(repo, onto),
    })
}

/// Check out `to` from the current branch.
fn checkout_to(repo: &TempRepo, to: &str) -> Operation {
    Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch(to.to_owned()),
        head_before: HeadRef::Branch(repo.git(&["symbolic-ref", "HEAD"])),
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
enum Tree {
    /// `base.txt` staged one way and edited further unstaged, plus an
    /// untracked file.
    StagedUnstagedUntracked,
    /// A new file, staged.
    StagedNewFile,
    /// `a.txt` deleted with `git rm`.
    Deletion,
    /// `a.txt` renamed with `git mv`.
    Rename,
}

impl Tree {
    const ALL: [Tree; 4] = [
        Tree::StagedUnstagedUntracked,
        Tree::StagedNewFile,
        Tree::Deletion,
        Tree::Rename,
    ];

    fn make(self, repo: &TempRepo) {
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
        }
    }
}

// ── reading the repository back ─────────────────────────────────────────

/// The stash list as commit ids, newest first.
fn stash_commits(repo: &TempRepo) -> Vec<Oid> {
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
fn whole(repo: &TempRepo) -> (RepoState, String) {
    (state_of(repo), stash_list(repo))
}

fn journal_entries(repo: &TempRepo) -> Vec<JournalEntry> {
    let handle = repo.open();
    journal_for(&handle).entries().to_vec()
}

fn last_entry(repo: &TempRepo) -> JournalEntry {
    journal_entries(repo).pop().expect("an entry was written")
}

/// The set-aside stash an entry recorded.
fn set_aside_of(entry: &JournalEntry) -> Oid {
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
fn edit_is_somewhere(repo: &TempRepo, path: &str, bytes: &[u8]) -> bool {
    let in_tree = repo.path().join(path).exists() && repo.read(path) == bytes;
    let expected = String::from_utf8_lossy(bytes).trim_end().to_owned();
    in_tree
        || stash_commits(repo).iter().any(|commit| {
            repo.try_git(&["show", &format!("{commit}:{path}")])
                .is_ok_and(|shown| shown == expected)
        })
}

fn resume_with(repo: &TempRepo, how: Resume, at: i64) -> Result<Applied, ApplyError> {
    let handle = repo.open();
    // A fresh journal read: `put_back` must come from the Pending line on
    // disk, not from anything kept in memory since the stop.
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, at, &never);
    resume(&mut ctx, how)
}

// ── the sequence, clean ─────────────────────────────────────────────────

/// ADR-0045 §7: for every tree in the table, for a rebase and a checkout,
/// apply → undo leaves the index, the files and the stash list exactly as
/// before; redo leaves them exactly as after.
#[test]
fn set_aside_round_trips() {
    for kind in Kind::BOTH {
        for tree in Tree::ALL {
            let repo = diverged();
            kind.start(&repo);
            tree.make(&repo);
            let what = format!("{kind:?} over {tree:?}");
            let op = kind.op(&repo);
            let before = whole(&repo);

            let applied =
                try_run(&repo, &op, NOW).unwrap_or_else(|why| panic!("{what}: {}", why.explain()));
            assert_eq!(applied.entry.status, EntryStatus::Committed, "{what}");
            set_aside_of(&applied.entry);
            let after = whole(&repo);
            assert_ne!(after.0, before.0, "{what}: the operation changed nothing");
            assert_eq!(after.1, before.1, "{what}: the stash list is as before");

            run_undo(&repo, NOW + 1, false)
                .unwrap_or_else(|why| panic!("{what}: undo: {}", why.explain()));
            assert_eq!(whole(&repo), before, "{what}: undo put it back exactly");

            run_redo(&repo, NOW + 2, false)
                .unwrap_or_else(|why| panic!("{what}: redo: {}", why.explain()));
            assert_eq!(whole(&repo), after, "{what}: redo put it forward exactly");
        }
    }
}

/// Each command line of `entry`, named by the set-aside step it is, or "the
/// operation" for the operation's own lines.
fn step_names(entry: &JournalEntry) -> Vec<&'static str> {
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
fn clashing_checkout() -> (TempRepo, Operation) {
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

/// A put-back git refuses with nothing unmerged — an untracked file where
/// the edits bring back a staged new file — keeps `S` and says where the
/// edits are. The index decides, not the exit code (ADR-0045 §7).
#[test]
fn a_put_back_that_fails_without_conflict_keeps_the_stash() {
    let repo = diverged();
    repo.write("n.txt", b"new\n");
    repo.git(&["add", "n.txt"]);
    // After the switch, something drops an untracked `n.txt` in the way.
    let hooks = repo.path().join("hooks");
    std::fs::create_dir_all(&hooks).expect("the hooks dir is made");
    let hook = hooks.join("post-checkout");
    std::fs::write(&hook, "#!/bin/sh\necho in the way > n.txt\n").expect("the hook is written");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
        .expect("the hook is executable");
    repo.git(&["config", "core.hooksPath", hooks.to_str().expect("utf-8")]);

    let op = checkout_to(&repo, "refs/heads/feature");
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

/// git refuses the rebase before it starts (an untracked file in the way of
/// `main`'s tree): the edits go back onto the same tip, then the refusal.
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
    repo.write("u.txt", b"untracked, in the way\n");
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
    assert_eq!(repo.read("u.txt"), b"untracked, in the way\n");
    assert!(stash_commits(&repo).is_empty(), "S dropped");
}

// ── stops ───────────────────────────────────────────────────────────────

/// A `Stop` that latches once `fired` says the step before it has landed —
/// keyed to what the step leaves on disk, never to a poll count (the
/// reasoning of `support::StopWhenGone`).
struct StopWhen {
    fired: Box<dyn Fn() -> bool + Sync>,
    latched: AtomicBool,
}

impl StopWhen {
    fn new(fired: impl Fn() -> bool + Sync + 'static) -> StopWhen {
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
fn loose_objects(git_dir: &Path) -> usize {
    let Ok(fans) = std::fs::read_dir(git_dir.join("objects")) else {
        return 0;
    };
    fans.flatten()
        .filter(|fan| fan.file_name().len() == 2)
        .map(|fan| std::fs::read_dir(fan.path()).map_or(0, Iterator::count))
        .sum()
}

/// A stop that fires just before set-aside step `step` (1–6) runs, keyed
/// to what the step before it leaves on disk.
fn stop_before(repo: &TempRepo, step: u8) -> StopWhen {
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

/// ADR-0045 §5, §7: a stop before each of steps 1–6 in turn leaves every
/// edit in the tree or in the stash list, never in neither — read from both,
/// for a rebase and a checkout.
#[test]
fn a_stop_before_each_step_loses_nothing() {
    for kind in Kind::BOTH {
        for step in 1..=6 {
            let repo = diverged();
            kind.start(&repo);
            repo.write("base.txt", b"edited\n");
            repo.write("n.txt", b"new\n");
            repo.git(&["add", "n.txt"]);
            let op = kind.op(&repo);
            let stop = stop_before(&repo, step);
            let result = run_with_stop(&repo, &op, NOW, &stop);
            let what = format!(
                "{kind:?}, stop before step {step}: {:?}",
                result.as_ref().err()
            );
            assert!(
                stop.latched.load(Ordering::Relaxed) || step == 6,
                "{what}: never fired"
            );
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
