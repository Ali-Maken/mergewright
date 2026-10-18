//! The edit-stop fact (m7-power-tools/07a): what git writes at an `edit`
//! stop, how `rewrite::edit_stop` and `read_facts` read it, and P14's one
//! exception — an amend at an edit stop, on the commit git stopped on.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use mergewright_core::oid::Oid;
use mergewright_git::state::InProgress;
use mergewright_ops::apply::ApplyError;
use mergewright_ops::facts::read_facts;
use mergewright_ops::operation::{CommitParams, FactsScope, Operation, Refusal, RepoFacts};
use mergewright_ops::rewrite::{EditStop, RewriteRequest, TodoAction, edit_stop, plan};

use crate::support::{NOW, Never, TempRepo, git_version, try_run};

pub(super) fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a commit id")
}

/// Three commits, each on its own file.
fn three() -> TempRepo {
    let repo = TempRepo::init();
    for name in ["c1", "c2", "c3"] {
        repo.write(&format!("{name}.txt"), format!("{name}\n").as_bytes());
        repo.commit_all(name);
    }
    repo
}

/// `git rebase -i HEAD~2` with its first row turned to `edit`, as he would
/// run it in the shell. Stops on `c2`.
fn edit_c2_in_the_shell(repo: &TempRepo) {
    repo.git(&[
        "-c",
        "sequence.editor=sed -i.bak -e 1s/^pick/edit/",
        "rebase",
        "-q",
        "-i",
        "HEAD~2",
    ]);
}

/// The premise the whole fact rests on, checked against the system git
/// (measured 2026-09-27, git 2.54.0 (Apple Git-157)): at an `edit` stop git
/// writes `rebase-merge/amend` holding the stopped commit's id, keeps it
/// through an amend, and removes it with the rest of `rebase-merge/` on
/// `--continue`.
#[test]
fn git_writes_rebase_merge_amend_at_an_edit_stop() {
    let repo = three();
    eprintln!("measured against {}", repo.git(&["--version"]));
    edit_c2_in_the_shell(&repo);

    let amend = repo.path().join(".git/rebase-merge/amend");
    let written = std::fs::read_to_string(&amend).expect("git wrote rebase-merge/amend");
    assert_eq!(
        Oid::from_hex(written.trim()).expect("it holds a commit id"),
        id(&repo, "HEAD"),
        "it holds the commit git stopped on"
    );

    repo.write("c2.txt", b"c2 amended\n");
    repo.git(&["commit", "-q", "--amend", "--no-edit", "-a"]);
    let after = std::fs::read_to_string(&amend).expect("the amend keeps it");
    assert_eq!(after, written, "an amend does not rewrite it");

    repo.git(&["rebase", "--continue"]);
    assert!(!amend.exists(), "--continue removes it");
}

pub(super) fn facts(repo: &TempRepo) -> RepoFacts {
    read_facts(&repo.open(), git_version(), &Never, &FactsScope::default())
        .unwrap_or_else(|why| panic!("the facts read: {}", why.explain()))
}

fn amend_verdict(repo: &TempRepo) -> Result<(), Refusal> {
    Operation::Amend(CommitParams {
        message: "c2".to_owned(),
        sign_off: false,
        no_verify: false,
        allow_empty: true,
        include: mergewright_ops::operation::Include::Staged,
    })
    .check(&facts(repo))
}

#[test]
fn edit_stop_reads_the_stopped_commit_and_whether_head_is_still_on_it() {
    let repo = three();
    edit_c2_in_the_shell(&repo);
    let stopped = id(&repo, "HEAD");
    let read = || edit_stop(&repo.open(), &Never).expect("it reads");

    let expected = EditStop {
        stopped,
        head_on_it: true,
    };
    assert_eq!(read(), Some(expected.clone()), "at the stop");
    assert_eq!(
        facts(&repo).edit_stop,
        Some(expected.clone()),
        "read_facts fills it"
    );

    repo.write("c2.txt", b"c2 amended\n");
    repo.git(&["commit", "-q", "--amend", "--no-edit", "-a"]);
    assert_eq!(read(), Some(expected), "amended once: still on it");

    repo.write("new.txt", b"new\n");
    repo.commit_all("on top");
    assert_eq!(
        read(),
        Some(EditStop {
            stopped,
            head_on_it: false,
        }),
        "a commit on top of it"
    );
}

#[test]
fn amend_passes_check_at_a_real_edit_stop_until_a_commit_lands_on_top() {
    let repo = three();
    edit_c2_in_the_shell(&repo);
    assert_eq!(amend_verdict(&repo), Ok(()), "HEAD is the stopped commit");

    repo.write("c2.txt", b"c2 amended\n");
    repo.git(&["commit", "-q", "--amend", "--no-edit", "-a"]);
    assert_eq!(amend_verdict(&repo), Ok(()), "amended once");

    repo.write("new.txt", b"new\n");
    repo.commit_all("on top");
    assert_eq!(
        amend_verdict(&repo),
        Err(Refusal::InProgress(InProgress::Rebase)),
        "a new commit on top"
    );
}

/// `c2` and `c3` both change `shared.txt`; replaying `c3` onto `c1` stops.
fn conflict_stop() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("shared.txt", b"1\n");
    repo.commit_all("c1");
    repo.write("shared.txt", b"2\n");
    repo.commit_all("c2");
    repo.write("shared.txt", b"3\n");
    repo.commit_all("c3");
    // Drop c2: c3's change no longer applies cleanly.
    let stopped = repo.try_git(&[
        "-c",
        "sequence.editor=sed -i.bak -e 1s/^pick/drop/",
        "rebase",
        "-q",
        "-i",
        "HEAD~2",
    ]);
    assert!(stopped.is_err(), "the rebase stops on a conflict");
    assert_eq!(
        mergewright_git::state::in_progress(&repo.open()).expect("state reads"),
        Some(InProgress::Rebase)
    );
    repo
}

#[test]
fn edit_stop_is_none_outside_a_rebase() {
    let repo = three();
    assert_eq!(edit_stop(&repo.open(), &Never).expect("it reads"), None);
    assert_eq!(facts(&repo).edit_stop, None);
}

#[test]
fn edit_stop_is_none_at_a_conflict_stop() {
    let repo = conflict_stop();
    assert_eq!(edit_stop(&repo.open(), &Never).expect("it reads"), None);
    assert_eq!(facts(&repo).edit_stop, None);
    // Resolved and staged, the amend is still P14's refusal.
    repo.write("shared.txt", b"3\n");
    repo.git(&["add", "shared.txt"]);
    assert_eq!(
        amend_verdict(&repo),
        Err(Refusal::InProgress(InProgress::Rebase))
    );
}

/// A `line()` of `rewrite_run`, cut to one branch: `main` then `feature`
/// with two commits on it.
pub(super) fn feature() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("m1.txt", b"m1\n");
    repo.commit_all("m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    for name in ["a1", "a2"] {
        repo.write(&format!("{name}.txt"), format!("{name}\n").as_bytes());
        repo.commit_all(name);
    }
    repo
}

/// Mergewright's own rewrite of `feature` with `a1` set to `edit` stops as a
/// conflict stop does (`ApplyError::Conflicted`, the arm the app's
/// `operated` lands a stop through), with no paths — and the stop reads as
/// an edit stop.
#[test]
fn an_edit_stop_comes_back_as_a_stop() {
    let repo = feature();
    let handle = repo.open();
    let mut params = plan(
        &handle,
        &git_version(),
        &RewriteRequest::Open {
            from: id(&repo, "feature~1"),
        },
        &facts(&repo),
        None,
        &Never,
    )
    .unwrap_or_else(|why| panic!("it plans: {why:?}"));
    assert_eq!(params.todo[0].subject, "a1");
    params.todo[0].action = TodoAction::Edit;

    match try_run(&repo, &Operation::Rewrite(params), NOW) {
        Err(ApplyError::Conflicted { paths, .. }) => {
            assert!(paths.is_empty(), "an edit stop lists nothing: {paths:?}");
        }
        Err(other) => panic!(
            "expected a stop, got {}",
            other.explain(&crate::support::Letters)
        ),
        Ok(applied) => panic!("expected a stop, it finished: {:?}", applied.entry.status),
    }
    let stop = facts(&repo).edit_stop.expect("an edit stop");
    assert_eq!(stop.stopped, id(&repo, "HEAD"));
    assert!(stop.head_on_it);
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "a1");
}

// ── An amend at an edit stop, through `apply` (m7-power-tools/07b) ───────

fn keep_message(message: &str) -> Operation {
    Operation::Amend(CommitParams {
        message: message.to_owned(),
        sign_off: false,
        no_verify: false,
        allow_empty: false,
        include: mergewright_ops::operation::Include::Staged,
    })
}

fn journal(repo: &TempRepo) -> Vec<mergewright_core::journal::JournalEntry> {
    crate::support::journal_for(&repo.open()).entries().to_vec()
}

/// A rebase begun in the shell has no entry of Mergewright's to fold into, so an
/// amend at its edit stop is **its own entry, with its own backup**, like any
/// amend: `u` can take it back (review m7-09-to-21c, Major 1). It used to run
/// folded into nothing — no backup, no line.
#[test]
fn a_shell_begun_edit_stop_amend_is_recorded_with_a_backup() {
    let repo = three();
    edit_c2_in_the_shell(&repo);
    let stopped = id(&repo, "HEAD");
    repo.write("c2.txt", b"c2 amended\n");
    repo.git(&["add", "c2.txt"]);

    let applied = try_run(&repo, &keep_message("c2"), NOW)
        .unwrap_or_else(|why| panic!("the amend runs: {}", why.explain(&crate::support::Letters)));
    assert_eq!(
        applied.refresh,
        mergewright_ops::apply::Refresh::Everything,
        "HEAD moved: the graph re-reads"
    );
    let amended = id(&repo, "HEAD");
    assert_ne!(amended, stopped, "HEAD is the amended commit");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        "c2",
        "message kept"
    );

    let entries = journal(&repo);
    assert_eq!(entries.len(), 1, "one entry, the amend's: {entries:?}");
    assert_eq!(
        entries[0].kind,
        mergewright_core::journal::OperationKind::Amend
    );
    assert!(entries[0].backup_ref.is_some(), "with a backup");
    assert!(
        repo.path().join(".git/rebase-merge").is_dir(),
        "the rebase is still stopped"
    );
    repo.git(&["rebase", "--continue"]);
    assert_eq!(repo.git(&["show", "HEAD~1:c2.txt"]), "c2 amended");
}

/// Mergewright's own rewrite keeps one entry for the span: the amend folds into
/// the live `Pending` rewrite entry (M5 hard choice 2), which `c` then still
/// finds and finishes — and the graph re-reads, since `HEAD` moved.
#[test]
fn an_amend_inside_graft_s_rewrite_folds_and_c_still_continues_it() {
    let repo = feature();
    let handle = repo.open();
    let mut params = plan(
        &handle,
        &git_version(),
        &RewriteRequest::Open {
            from: id(&repo, "feature~1"),
        },
        &facts(&repo),
        None,
        &Never,
    )
    .unwrap_or_else(|why| panic!("it plans: {why:?}"));
    params.todo[0].action = TodoAction::Edit;
    let entry = match try_run(&repo, &Operation::Rewrite(params), NOW) {
        Err(ApplyError::Conflicted { entry, .. }) => entry.id,
        Err(other) => panic!(
            "expected a stop, got {}",
            other.explain(&crate::support::Letters)
        ),
        Ok(_) => panic!("expected a stop"),
    };
    repo.write("a1.txt", b"a1 amended\n");
    repo.git(&["add", "a1.txt"]);

    let applied = try_run(&repo, &keep_message("a1"), NOW + 1)
        .unwrap_or_else(|why| panic!("the amend runs: {}", why.explain(&crate::support::Letters)));
    assert_eq!(applied.refresh, mergewright_ops::apply::Refresh::Everything);
    // Twice, as for a forgotten file.
    repo.write("late.txt", b"late\n");
    repo.git(&["add", "late.txt"]);
    try_run(&repo, &keep_message("a1"), NOW + 2).unwrap_or_else(|why| {
        panic!(
            "the second amend runs: {}",
            why.explain(&crate::support::Letters)
        )
    });

    let ids: Vec<_> = journal(&repo).iter().map(|e| e.id.clone()).collect();
    assert_eq!(ids, vec![entry.clone()], "no entry of the amends' own");

    let handle = repo.open();
    let mut journal = crate::support::journal_for(&handle);
    let mut ctx = crate::support::context(&handle, &mut journal, NOW + 3, &Never);
    let finished = mergewright_ops::apply::resume(
        &mut ctx,
        mergewright_ops::apply::Resume::Continue { solved: Vec::new() },
    )
    .unwrap_or_else(|why| panic!("c continues: {}", why.explain(&crate::support::Letters)));
    assert_eq!(finished.entry.id, entry, "the live entry was the one found");
    assert!(!repo.path().join(".git/rebase-merge").exists());
    assert_eq!(repo.git(&["show", "HEAD~1:a1.txt"]), "a1 amended");
    assert_eq!(repo.git(&["show", "HEAD~1:late.txt"]), "late");
    assert_eq!(repo.git(&["log", "-1", "--format=%s", "HEAD~1"]), "a1");
}
