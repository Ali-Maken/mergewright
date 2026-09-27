//! A cherry-pick over changed tracked files sets them aside and puts them
//! back, in one journal entry (ADR-0045 §4 extended to the pick,
//! m7-power-tools/11a, the note's "Where I'd differ" 4).
//!
//! Every repository is real; every claim is read back from git, the files
//! on disk or the journal file. `u` must hold for the new kind: undo leaves
//! the repository, index, tree and stash list exactly as before the pick,
//! redo exactly as after.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use graft_core::journal::{EntryStatus, PutBackWhy};
use graft_core::oid::Oid;
use graft_ops::apply::ApplyError;
use graft_ops::facts::read_facts;
use graft_ops::operation::{Operation, PickParams, SetAside, Tip};
use graft_ops::preview::{self, ConflictForecast, PreviewPlan};

use crate::set_aside::{last_entry, porcelain, set_aside_of, stash_commits, whole};
use crate::support::{NOW, Never, TempRepo, git_version, run_redo, run_undo, try_run};

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
        .unwrap_or_else(|why| panic!("{}", why.explain()))
}

/// An unsaved edit to the file the pick changes (line 7, the pick's line
/// 1): git alone would refuse ("your local changes would be overwritten").
/// Set aside, picked, put back: both changes in the file, one entry, the
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
    let applied = try_run(&repo, &op, NOW).unwrap_or_else(|why| panic!("{}", why.explain()));
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

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("undo: {}", why.explain()));
    assert_eq!(whole(&repo), before, "u put it back exactly");

    run_redo(&repo, NOW + 2, false).unwrap_or_else(|why| panic!("redo: {}", why.explain()));
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
        Err(ApplyError::Clashed { entry, clash }) => (*entry, clash),
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

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("undo: {}", why.explain()));
    assert_eq!(whole(&repo), before, "u put it back exactly");
}
