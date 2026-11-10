//! m6-conflicts/80 (review m8): the whole-tree case of an undo forced past
//! drift, driven on a real repository through `y` and then `U`.
//!
//! An entry whose after-state was never recorded is inverted whole: its
//! index step restores every path (`IndexPaths::Everything`), its
//! working-tree step every file (`PathScope::Everything`). No narrower list
//! of what the run replaces exists, so [`Replaced`] says which of the two it
//! restores whole, and the words follow it.
//!
//! **Decided (the wording for an index-only restore):** "the whole index".
//! An interrupted `stage` touches the index only, so `u` rewrites every
//! index entry and not one file on disk. Calling that "the whole working
//! tree" — what 77 said for either — would tell the user their files are
//! about to be overwritten when none is. `whole_tree` is the working tree's
//! flag alone now, `whole_index` the index's; the app names each by what it
//! is (`update::undo::consequence`).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::PathBuf;

use mergewright_ops::operation::{Operation, Target};
use mergewright_ops::undo::{self, IndexPaths, Replaced, Step, UndoError};

use crate::support::{NOW, TempRepo, journal_for, run, run_redo, run_undo, state_of};
use mergewright_core::on_disk::DIR;

/// Cut the journal back to the one entry's pending line and close it with
/// nothing known about what it left behind: the process died mid-run, and
/// recovery could not read the repository either.
fn interrupt_the_only_entry(repo: &TempRepo) {
    let path = repo.path().join(format!(".git/{DIR}/undo-journal.jsonl"));
    let text = std::fs::read_to_string(&path).expect("the journal reads");
    let pending = text.lines().next().expect("a pending line");
    std::fs::write(&path, format!("{pending}\n")).expect("the journal is truncated");
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let id = journal.entries()[0].id.clone();
    journal
        .mark_interrupted(&id, None)
        .expect("the entry is closed");
}

#[test]
fn an_index_only_whole_restore_names_the_whole_index_and_u_redo_puts_it_back() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"one\n");
    repo.write("b.rs", b"b\n");
    repo.commit_all("one");
    repo.write("a.rs", b"two\n");
    run(
        &repo,
        &Operation::Stage(Target::Paths(vec![PathBuf::from("a.rs")])),
        NOW,
    );
    interrupt_the_only_entry(&repo);

    // The inverse is the index's alone, and whole.
    let plan = undo::plan_undo(journal_for(&repo.open()).entries()).expect("still invertible");
    assert!(
        plan.steps.iter().any(|step| matches!(
            step,
            Step::RestoreIndex {
                paths: IndexPaths::Everything,
                ..
            }
        )) && !plan
            .steps
            .iter()
            .any(|step| matches!(step, Step::RestoreWorktree { .. })),
        "an index-only whole restore: {:?}",
        plan.steps
    );

    // Since then the user staged an edit of their own.
    repo.write("b.rs", b"b, staged\n");
    repo.git(&["add", "b.rs"]);
    let pre = state_of(&repo);
    let whole_index = Replaced {
        paths: Vec::new(),
        whole_tree: false,
        whole_index: true,
    };

    // The box.
    match run_undo(&repo, NOW + 1, false) {
        Err(UndoError::Drift { drift, .. }) => {
            assert!(drift.unknown.contains(&"the index"), "{drift:?}");
            assert_eq!(drift.replaced, whole_index, "the box names the index");
        }
        other => panic!("the rail should have fired: {other:?}"),
    }
    assert_eq!(state_of(&repo), pre, "the box wrote nothing");

    // `y`.
    let forced = run_undo(&repo, NOW + 2, true)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(forced.replaced, whole_index, "the landing names the index");
    assert_eq!(
        repo.git(&["diff", "--cached", "--name-only"]),
        "",
        "the whole index went back to before the stage"
    );
    assert_eq!(
        repo.read("b.rs"),
        b"b, staged\n",
        "no file on disk was touched"
    );
    assert_eq!(repo.read("a.rs"), b"two\n", "no file on disk was touched");

    // `U`: byte-exact, index and all.
    run_redo(&repo, NOW + 3, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(state_of(&repo), pre, "U put the whole index back");
}
