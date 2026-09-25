//! A pop or apply that clashes is recorded and answered as a clash, and
//! finishing one drops the stash by its commit id (ADR-0045 §1–3,
//! m6-conflicts/72). Every fact is read back from git or the journal file,
//! never from a returned value alone.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use graft_core::journal::{EntryStatus, JournalEntry, OperationKind, PutBack, PutBackWhy};
use graft_core::oid::Oid;
use graft_ops::apply::ApplyError;
use graft_ops::clash::Clash;
use graft_ops::operation::{Operation, Refusal};
use graft_ops::snapshot;

use crate::support::{
    NOW, Never, TempRepo, journal_for, run, run_undo, stash_list, state_of, try_run,
};
use crate::unmerged_index::about_to_pop_a_conflict;

/// The stash list as commit ids, newest first.
fn stash_commits(repo: &TempRepo) -> Vec<String> {
    repo.git(&["stash", "list", "--format=%H"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The journal as the file holds it now.
fn journal_entries(repo: &TempRepo) -> Vec<JournalEntry> {
    let handle = repo.open();
    journal_for(&handle).entries().to_vec()
}

/// Run a put-back that must clash; hand back the entry and the clash.
fn clashed(repo: &TempRepo, op: &Operation) -> (JournalEntry, Clash) {
    match try_run(repo, op, NOW) {
        Err(ApplyError::Clashed { entry, clash }) => (*entry, clash),
        other => panic!("the put-back should have clashed: {other:?}"),
    }
}

/// ADR-0045 §1–2: a clashed pop is `Committed` with the stage tree as its
/// index-after and the put-back on its line; git kept the stash.
#[test]
fn a_pop_that_clashes_answers_clashed_and_keeps_the_stash() {
    let (repo, stashed) = about_to_pop_a_conflict();
    let listed = stash_commits(&repo);

    let (entry, clash) = clashed(
        &repo,
        &Operation::StashPop {
            index: 0,
            commit: stashed,
        },
    );

    assert_eq!(
        clash,
        Clash {
            stash: stashed,
            why: PutBackWhy::Pop,
            paths: vec![PathBuf::from("p.txt")],
        }
    );
    assert_eq!(stash_commits(&repo), listed, "git kept the stash");

    // What the journal *file* says, not the value handed back.
    let on_disk = journal_entries(&repo);
    let written = on_disk
        .iter()
        .find(|written| written.id == entry.id)
        .expect("the entry is in the journal file");
    assert_eq!(written.status, EntryStatus::Committed);
    assert_eq!(
        written.put_back,
        Some(PutBack {
            stash: stashed,
            why: PutBackWhy::Pop,
        })
    );
    // The pin (acceptance 2): `finish` records the stage tree as it stands.
    // Nothing touched the index since, so reading it now must agree.
    let handle = repo.open();
    let scratch = repo.path().join(".git/graft");
    let now = snapshot::index_state(&handle, &scratch, &Never).expect("the index reads");
    assert!(now.is_unmerged(), "the pop left the index unmerged");
    assert_eq!(written.index_after, Some(now));
    assert!(
        written
            .index_before
            .is_some_and(|index| !index.is_unmerged()),
        "the index before was merged: {:?}",
        written.index_before
    );
}

#[test]
fn an_apply_that_clashes_records_apply() {
    let (repo, stashed) = about_to_pop_a_conflict();
    let listed = stash_commits(&repo);

    let (entry, clash) = clashed(
        &repo,
        &Operation::StashApply {
            index: 0,
            commit: stashed,
        },
    );

    assert_eq!(clash.why, PutBackWhy::Apply);
    assert_eq!(clash.stash, stashed);
    assert_eq!(clash.paths, vec![PathBuf::from("p.txt")]);
    assert_eq!(stash_commits(&repo), listed);
    let written = journal_entries(&repo)
        .into_iter()
        .find(|written| written.id == entry.id)
        .expect("the entry is in the journal file");
    assert_eq!(written.status, EntryStatus::Committed);
    assert_eq!(
        written.put_back,
        Some(PutBack {
            stash: stashed,
            why: PutBackWhy::Apply,
        })
    );
    assert!(written.index_after.is_some_and(|index| index.is_unmerged()));
}

/// A stash whose **staged** change no longer applies to the index: `apply
/// --index` refuses ("Conflicts in index. Try without --index.") and leaves
/// nothing unmerged. That is a failure, not a clash (ADR-0045 §7).
#[test]
fn a_failed_apply_with_nothing_unmerged_is_not_a_clash() {
    let repo = TempRepo::init();
    repo.write("a.txt", b"one\n");
    repo.commit_all("one");
    repo.write("a.txt", b"staged\n");
    repo.git(&["add", "a.txt"]);
    repo.git(&["stash", "push", "-q", "-m", "staged"]);
    let stashed = Oid::from_hex(&repo.git(&["rev-parse", "stash@{0}"])).expect("a stash id");
    repo.write("a.txt", b"committed\n");
    repo.commit_all("moved on");

    let outcome = try_run(
        &repo,
        &Operation::StashApply {
            index: 0,
            commit: stashed,
        },
        NOW,
    );
    let entry = match outcome {
        Err(ApplyError::Git { entry, .. }) => *entry,
        other => panic!("the apply should have failed plainly: {other:?}"),
    };
    assert!(
        matches!(entry.status, EntryStatus::Aborted { .. }),
        "{:?}",
        entry.status
    );
    assert_eq!(repo.git(&["ls-files", "-u"]), "", "nothing unmerged");
    assert_eq!(
        entry.put_back.map(|put_back| put_back.why),
        Some(PutBackWhy::Apply),
        "the put-back is on the line whatever the outcome"
    );
}

/// Two stashes, the target below the top: the drop finds it by commit, so
/// the one on top survives.
#[test]
fn drop_put_back_drops_by_commit_id_not_position() {
    let (repo, stashed) = about_to_pop_a_conflict();
    repo.write("other.txt", b"stashed on top\n");
    repo.git(&["stash", "push", "-q", "-u", "-m", "on top"]);
    let on_top = repo.git(&["rev-parse", "stash@{0}"]);

    let entry = run(&repo, &Operation::DropPutBack { stash: stashed }, NOW);

    assert_eq!(stash_commits(&repo), [on_top], "only the put-back went");
    assert_eq!(entry.kind, OperationKind::StashDrop);
    assert_eq!(
        entry.pins,
        vec![stashed],
        "pinned, so `u` can store it back"
    );
    assert!(
        entry.commands.iter().any(|line| line.contains("stash@{1}")),
        "dropped at the position the facts gave: {:?}",
        entry.commands
    );
}

#[test]
fn drop_put_back_refuses_when_the_stash_is_gone() {
    let (repo, stashed) = about_to_pop_a_conflict();
    repo.git(&["stash", "drop", "-q"]);

    let outcome = try_run(&repo, &Operation::DropPutBack { stash: stashed }, NOW);

    assert!(
        matches!(outcome, Err(ApplyError::Refused(Refusal::NoSuchStash))),
        "{outcome:?}"
    );
    assert!(
        journal_entries(&repo).is_empty(),
        "a refusal writes nothing"
    );
}

/// `stash::plan_drop_by_commit`'s fallback, should the facts lack the
/// stash: `stash drop <commit hex>`. It is only safe because git refuses a
/// bare commit id and drops nothing — proven here, not assumed.
#[test]
fn git_refuses_a_stash_drop_by_bare_commit_id() {
    let (repo, stashed) = about_to_pop_a_conflict();
    let listed = stash_list(&repo);

    let refused = repo.try_git(&["stash", "drop", &stashed.to_string()]);

    assert!(refused.is_err(), "git dropped by bare id: {refused:?}");
    assert_eq!(stash_list(&repo), listed, "nothing was dropped");
}

#[test]
fn undo_of_drop_put_back_stores_it_again() {
    let (repo, stashed) = about_to_pop_a_conflict();
    let listed = stash_list(&repo);

    run(&repo, &Operation::DropPutBack { stash: stashed }, NOW);
    assert_eq!(stash_commits(&repo), Vec::<String>::new());

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(
        stash_list(&repo),
        listed,
        "the stash is back, message and all"
    );
}

/// ADR-0045 §6: `u` during a clash is the abort — the tree before the pop,
/// the stash still listed. The entry is `Committed`, so it is in force and
/// `u` reaches it with no force.
#[test]
fn undo_of_a_clashed_pop_restores_the_tree_and_keeps_the_stash() {
    let (repo, stashed) = about_to_pop_a_conflict();
    let before = state_of(&repo);
    let listed = stash_list(&repo);

    clashed(
        &repo,
        &Operation::StashPop {
            index: 0,
            commit: stashed,
        },
    );
    assert_ne!(state_of(&repo), before, "the clash changed the tree");

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain()));

    assert_eq!(state_of(&repo), before, "the tree before the pop");
    assert_eq!(stash_list(&repo), listed, "the stash still listed, once");
}
