//! A cherry-pick over changed tracked files sets them aside and puts them
//! back, in one journal entry (ADR-0045 §4 extended to the pick,
//! m7-power-tools/11a, the note's "Where I'd differ" 4).
//!
//! Every repository is real; every claim is read back from git, the files
//! on disk or the journal file. `u` must hold for the new kind: undo leaves
//! the repository, index, tree and stash list exactly as before the pick,
//! redo exactly as after.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::PathBuf;

use mergewright_core::journal::{EntryStatus, PutBackWhy};
use mergewright_core::oid::Oid;
use mergewright_ops::apply::{ApplyError, StoppedAt};
use mergewright_ops::facts::read_facts;
use mergewright_ops::operation::{Operation, PickParams, Refusal, SetAside, Tip};
use mergewright_ops::preview::{self, ConflictForecast, PreviewError, PreviewPlan};

use crate::set_aside::{
    journal_entries, last_entry, porcelain, set_aside_of, stash_commits, whole,
};
use crate::stop_hooks::{Hooked, StopOnceRunning};
use crate::support::{
    NOW, Never, TempRepo, git_version, run_redo, run_undo, run_with_stop, try_run,
};

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a full id")
}

/// Seven lines, so a pick and an edit can change the same file far enough
/// apart that git merges them without a conflict.
const SEVEN: &[u8] = b"1\n2\n3\n4\n5\n6\n7\n";

/// `main` and `feature` share `a.txt` (seven lines) and `base.txt`;
/// `feature`'s one commit, "fix line one", changes `a.txt`'s first line.
/// Left on `main`, clean.
fn with_a_fix_on_feature() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", SEVEN);
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("a.txt", b"one, fixed\n2\n3\n4\n5\n6\n7\n");
    repo.commit_all("fix line one");
    repo.git(&["switch", "-q", "main"]);
    repo
}

/// Take `feature`'s tip onto `main`, as `o` builds it.
fn take_the_fix(repo: &TempRepo) -> Operation {
    Operation::CherryPick(PickParams {
        commits: vec![id(repo, "refs/heads/feature")],
        subject: "fix line one".to_owned(),
        head: Tip {
            name: "refs/heads/main".to_owned(),
            oid: id(repo, "refs/heads/main"),
        },
    })
}

fn plan_of(repo: &TempRepo, op: &Operation) -> PreviewPlan {
    let handle = repo.open();
    let facts = read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("facts");
    preview::plan(&handle, &git_version(), op, &facts, &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
}

/// An unsaved edit to the file the pick changes (line 7, the pick's line
/// 1). Set aside, picked, put back: both changes in the file, one entry, the
/// stash list as before — and `u` then `U` exact.
#[test]
fn picking_over_touched_unsaved_edits_sets_them_aside() {
    let repo = with_a_fix_on_feature();
    repo.write("a.txt", b"1\n2\n3\n4\n5\n6\nseven, mine\n");
    let op = take_the_fix(&repo);

    let plan = plan_of(&repo, &op);
    assert_eq!(plan.set_aside, Some(SetAside { files: 1 }));
    assert_eq!(
        plan.put_back,
        Some(ConflictForecast::Clean),
        "the preview forecasts a clean put-back"
    );

    let before = whole(&repo);
    let status_before = porcelain(&repo);
    let main_before = id(&repo, "refs/heads/main");
    let applied = try_run(&repo, &op, NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    let stash = set_aside_of(&applied.entry);
    assert!(applied.entry.pins.contains(&stash), "S is pinned");
    assert_eq!(last_entry(&repo), applied.entry, "one entry, on file");

    assert_eq!(
        repo.read("a.txt"),
        b"one, fixed\n2\n3\n4\n5\n6\nseven, mine\n",
        "the pick and the edit, both"
    );
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "fix line one");
    assert_eq!(id(&repo, "refs/heads/main^"), main_before, "one commit on");
    assert_eq!(porcelain(&repo), status_before, "the edit is unsaved again");
    let after = whole(&repo);
    assert_eq!(after.1, before.1, "the stash list is as before");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(&repo), before, "u put it back exactly");

    run_redo(&repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("redo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(&repo), after, "U put it forward exactly");
}

/// The edit and the pick change the same line: the pick lands, the
/// put-back clashes — `Clashed { why: SetAside }` on `a.txt`, the entry
/// `Committed`, `S` still listed, so the resolver takes it from there
/// (ADR-0045 §2). The preview said so first. `u` puts everything back.
#[test]
fn a_pick_put_back_that_clashes_stops_as_a_clash() {
    let repo = with_a_fix_on_feature();
    repo.write("a.txt", b"one, mine\n2\n3\n4\n5\n6\n7\n");
    let op = take_the_fix(&repo);

    let plan = plan_of(&repo, &op);
    assert_eq!(
        plan.put_back,
        Some(ConflictForecast::Likely {
            paths: vec![PathBuf::from("a.txt")]
        }),
        "the preview forecasts the clash"
    );

    let before = whole(&repo);
    let (entry, clash) = match try_run(&repo, &op, NOW) {
        Err(ApplyError::Clashed { entry, clash, .. }) => (*entry, clash),
        other => panic!("the put-back should clash: {other:?}"),
    };
    assert_eq!(clash.why, PutBackWhy::SetAside);
    assert_eq!(clash.paths, vec![PathBuf::from("a.txt")]);
    let stash = set_aside_of(&entry);
    assert_eq!(clash.stash, stash);
    assert_eq!(entry.status, EntryStatus::Committed);
    assert_eq!(stash_commits(&repo), vec![stash], "S kept");
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "fix line one");
    assert!(
        !repo.git(&["ls-files", "-u"]).is_empty(),
        "git says unmerged"
    );

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(&repo), before, "u put it back exactly");
}

/// Edits to a file the pick never touches are set aside too (`sets_aside`
/// reads no commit) and come back byte for byte, staged part staged: the
/// preview counts them, the status after is the status before, `u` exact.
#[test]
fn edits_the_pick_leaves_alone_are_set_aside_and_come_back_as_they_were() {
    let repo = with_a_fix_on_feature();
    repo.write("base.txt", b"staged\n");
    repo.git(&["add", "base.txt"]);
    repo.write("base.txt", b"staged\nand unstaged\n");
    let op = take_the_fix(&repo);
    assert_eq!(plan_of(&repo, &op).set_aside, Some(SetAside { files: 1 }));

    let before = whole(&repo);
    let status_before = porcelain(&repo);
    let staged_before = repo.git(&["diff", "--cached", "--", "base.txt"]);
    let applied = try_run(&repo, &op, NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    set_aside_of(&applied.entry);
    assert_eq!(repo.read("base.txt"), b"staged\nand unstaged\n");
    assert_eq!(
        repo.git(&["diff", "--cached", "--", "base.txt"]),
        staged_before
    );
    assert_eq!(porcelain(&repo), status_before);
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "fix line one");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(&repo), before, "u put it back exactly");
}

// ── a pick over edits, stopped in its hook (19d-a2: the shared close) ───

/// `with_a_fix_on_feature`, with an edit the pick leaves alone, staged and
/// unstaged: `base.txt` (`MM`).
fn with_a_fix_and_edits() -> TempRepo {
    let repo = with_a_fix_on_feature();
    repo.write("base.txt", b"staged\n");
    repo.git(&["add", "base.txt"]);
    repo.write("base.txt", b"staged\nand unstaged\n");
    repo
}

/// The pick committed before its `post-commit` hook was stopped: it landed,
/// and the set-aside close keeps that mark (D1) — `Committed`, his edits
/// back with their split, `S` dropped, and `u` exact.
#[test]
fn a_pick_over_edits_stopped_after_it_committed_has_landed() {
    let repo = with_a_fix_and_edits();
    let hooked = Hooked::stuck(&repo, "post-commit");
    let before = whole(&repo);
    let split = porcelain(&repo);
    let main_before = id(&repo, "refs/heads/main");

    let landed = run_with_stop(
        &repo,
        &take_the_fix(&repo),
        NOW,
        &StopOnceRunning(hooked.running()),
    );

    let Ok(applied) = landed else {
        panic!("a pick committed before its post-commit hook landed: {landed:?}");
    };
    assert_eq!(applied.stopped_at, Some(StoppedAt::Landed));
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    set_aside_of(&applied.entry);
    assert_eq!(id(&repo, "refs/heads/main^"), main_before, "one commit on");
    assert_eq!(repo.read("base.txt"), b"staged\nand unstaged\n");
    assert_eq!(porcelain(&repo), split, "the split kept");
    assert!(stash_commits(&repo).is_empty(), "S dropped");
    hooked.assert_gone();

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(&repo), before, "u put it back exactly");
}

/// git killed before it could pause the pick. A pick runs no hook before
/// it writes `CHERRY_PICK_HEAD`, so the stop lands in a merge driver: one
/// that left git's group, so the stop had no helper to end and stopped git
/// itself, mid-merge — `HEAD` unmoved, no pick in progress, only the
/// driver's temp files left in the tree. The set-aside close keeps the
/// reason the stop was read as (D2): `stopped before git could pause it`,
/// not `git was stopped`. His edits come back with their split, `S` is
/// dropped, and `u` takes git's debris away.
#[test]
fn a_pick_over_edits_stopped_before_git_paused_it_says_so() {
    let repo = with_a_fix_on_feature();
    // `main` changes `a.txt` too (line 7), so the pick's merge of it is a
    // real three-way merge and runs the driver.
    repo.write("a.txt", b"1\n2\n3\n4\n5\n6\nseven, main\n");
    repo.commit_all("main's line seven");
    repo.write("base.txt", b"staged\n");
    repo.git(&["add", "base.txt"]);
    repo.write("base.txt", b"staged\nand unstaged\n");
    let hooked = Hooked::stuck_outside_the_group(&repo, "merge-driver");
    let driver = hooked.script("merge-driver");
    repo.git(&[
        "config",
        "merge.stuck.driver",
        &format!("{} %A %O %B", driver.to_str().expect("utf-8")),
    ]);
    std::fs::write(
        repo.path().join(".git/info/attributes"),
        b"a.txt merge=stuck\n",
    )
    .expect("the attributes are written");
    let before = whole(&repo);
    let split = porcelain(&repo);

    let stopped = run_with_stop(
        &repo,
        &take_the_fix(&repo),
        NOW,
        &StopOnceRunning(hooked.running()),
    );

    let Err(error) = stopped else {
        panic!("a pick git did not pause does not succeed");
    };
    // git left its driver's temp files (`.merge_file_*`) behind: changed.
    assert_eq!(
        error.stopped_at(),
        Some(StoppedAt::BeforePause { changed: true }),
        "{error:?}"
    );
    let ApplyError::Git { entry, .. } = &error else {
        panic!("today's variant, not {error:?}");
    };
    assert_eq!(
        entry.status,
        EntryStatus::Aborted {
            reason: "stopped before git could pause it".to_owned()
        }
    );
    set_aside_of(entry);
    assert!(!repo.path().join(".git/CHERRY_PICK_HEAD").exists());
    assert_eq!(id(&repo, "HEAD"), id(&repo, "refs/heads/main"));
    assert_eq!(before.0.head, whole(&repo).0.head, "HEAD unmoved");
    assert_eq!(repo.read("base.txt"), b"staged\nand unstaged\n");
    let tracked: Vec<String> = porcelain(&repo)
        .into_iter()
        .filter(|line| !line.starts_with("??"))
        .collect();
    assert_eq!(tracked, split, "his edits are back, split kept");
    assert!(stash_commits(&repo).is_empty(), "S dropped");
    // The driver left git's group, so the stop does not end it; `hooked`'s
    // drop does.

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(&repo), before, "u put it back exactly");
}

// ── what a pick cannot take: the preview and `apply` refuse ─────────────

/// Pick `rev` onto `main`, as `o` builds it on that row.
fn take(repo: &TempRepo, rev: &str) -> Operation {
    Operation::CherryPick(PickParams {
        commits: vec![id(repo, rev)],
        subject: "picked".to_owned(),
        head: Tip {
            name: "refs/heads/main".to_owned(),
            oid: id(repo, "refs/heads/main"),
        },
    })
}

/// `main` two commits deep; `side` holds a merge of `x` and `y`, both off
/// `main`'s first commit. Left on `main`, clean.
fn with_a_merge_on_side() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.write("two.txt", b"two\n");
    repo.commit_all("two");
    repo.git(&["switch", "-q", "-c", "x", "HEAD~1"]);
    repo.write("x.txt", b"x\n");
    repo.commit_all("x");
    repo.git(&["switch", "-q", "-c", "side", "HEAD~1"]);
    repo.write("y.txt", b"y\n");
    repo.commit_all("y");
    repo.git(&["merge", "-q", "--no-ff", "--no-edit", "x"]);
    repo.git(&["switch", "-q", "main"]);
    repo
}

/// What the preview answers: its refusal, or a panic naming what it drew.
fn preview_refusal(repo: &TempRepo, op: &Operation) -> Refusal {
    let handle = repo.open();
    let facts = read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("facts");
    match preview::plan(&handle, &git_version(), op, &facts, &Never) {
        Err(PreviewError::Refused(refused)) => refused,
        other => panic!("the preview should stay shut: {other:?}"),
    }
}

/// What `apply` answers: its refusal — with nothing written: no journal
/// line, the repository and stash list exactly as before.
fn apply_refusal(repo: &TempRepo, op: &Operation) -> Refusal {
    let before = whole(repo);
    let refused = match try_run(repo, op, NOW) {
        Err(ApplyError::Refused(refused)) => refused,
        other => panic!("apply should refuse: {other:?}"),
    };
    assert!(journal_entries(repo).is_empty(), "no journal line");
    assert_eq!(whole(repo), before, "nothing written");
    refused
}

fn sha7(repo: &TempRepo, rev: &str) -> String {
    id(repo, rev).to_string()[..7].to_owned()
}

/// `HEAD`'s own commit: the preview never opens, `apply` refuses, in the
/// words `<sha7> is already on main` — over unsaved edits too, which are
/// never set aside for it.
#[test]
fn picking_head_is_refused_in_the_preview_and_in_apply() {
    let repo = with_a_merge_on_side();
    repo.write("base.txt", b"my edit\n");
    let op = take(&repo, "main");
    let words = format!("{} is already on main", sha7(&repo, "main"));
    assert_eq!(
        preview_refusal(&repo, &op).explain(&crate::support::Letters),
        words
    );
    assert_eq!(
        apply_refusal(&repo, &op).explain(&crate::support::Letters),
        words
    );
}

/// An ancestor of `HEAD`: the same refusal, in both places.
#[test]
fn picking_an_ancestor_is_refused_in_the_preview_and_in_apply() {
    let repo = with_a_merge_on_side();
    let op = take(&repo, "main~1");
    let words = format!("{} is already on main", sha7(&repo, "main~1"));
    assert_eq!(
        preview_refusal(&repo, &op).explain(&crate::support::Letters),
        words
    );
    assert_eq!(
        apply_refusal(&repo, &op).explain(&crate::support::Letters),
        words
    );
}

/// A merge `HEAD` lacks: refused in both places, before git is asked.
#[test]
fn picking_a_merge_is_refused_in_the_preview_and_in_apply() {
    let repo = with_a_merge_on_side();
    let op = take(&repo, "side");
    let words = format!(
        "{} is a merge · taking a merge isn't in Mergewright yet",
        sha7(&repo, "side")
    );
    assert_eq!(
        preview_refusal(&repo, &op).explain(&crate::support::Letters),
        words
    );
    assert_eq!(
        apply_refusal(&repo, &op).explain(&crate::support::Letters),
        words
    );
}

/// What git itself does with `cherry-pick <an ancestor of HEAD>`: the case
/// the refusal now keeps from it (`KNOWN_GAPS` :806). Checked against the
/// system git this suite runs; its version is in every failure message.
#[test]
fn git_alone_stops_on_a_pick_of_an_ancestor() {
    let repo = with_a_merge_on_side();
    let version = repo.git(&["--version"]);
    let head = id(&repo, "HEAD");
    let said = repo
        .try_git(&["cherry-pick", "--no-edit", "main~1"])
        .expect_err(&format!("{version}: git should not exit 0"));
    assert!(said.contains("empty"), "{version}: {said}");
    assert_eq!(id(&repo, "HEAD"), head, "{version}: no commit made");
    assert!(
        repo.path().join(".git/CHERRY_PICK_HEAD").exists(),
        "{version}: git leaves a cherry-pick in progress"
    );
}
