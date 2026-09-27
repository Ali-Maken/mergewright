//! The edit-stop fact (m7-power-tools/07a): what git writes at an `edit`
//! stop, how `rewrite::edit_stop` and `read_facts` read it, and P14's one
//! exception — an amend at an edit stop, on the commit git stopped on.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use graft_core::oid::Oid;
use graft_git::state::InProgress;
use graft_ops::apply::ApplyError;
use graft_ops::facts::read_facts;
use graft_ops::operation::{CommitParams, FactsScope, Operation, Refusal, RepoFacts};
use graft_ops::rewrite::{EditStop, RewriteRequest, TodoAction, edit_stop, plan};

use crate::support::{NOW, Never, TempRepo, git_version, try_run};

fn id(repo: &TempRepo, rev: &str) -> Oid {
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

fn facts(repo: &TempRepo) -> RepoFacts {
    read_facts(&repo.open(), git_version(), &Never, &FactsScope::default())
        .unwrap_or_else(|why| panic!("the facts read: {}", why.explain()))
}

fn amend_verdict(repo: &TempRepo) -> Result<(), Refusal> {
    Operation::Amend(CommitParams {
        message: "c2".to_owned(),
        sign_off: false,
        no_verify: false,
        allow_empty: true,
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
        graft_git::state::in_progress(&repo.open()).expect("state reads"),
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
fn feature() -> TempRepo {
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

/// Graft's own rewrite of `feature` with `a1` set to `edit` stops as a
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
        Err(other) => panic!("expected a stop, got {}", other.explain()),
        Ok(applied) => panic!("expected a stop, it finished: {:?}", applied.entry.status),
    }
    let stop = facts(&repo).edit_stop.expect("an edit stop");
    assert_eq!(stop.stopped, id(&repo, "HEAD"));
    assert!(stop.head_on_it);
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "a1");
}
