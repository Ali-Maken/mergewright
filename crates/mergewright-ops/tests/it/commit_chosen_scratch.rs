//! A commit of chosen files runs on a scratch index of its own (review
//! ease-pass-04): two windows on one worktree never share one, a dead run's
//! is never read and recovery sweeps it, there is one with no real index
//! yet, and a folded edit-stop amend gets one too.

#![allow(clippy::expect_used, reason = "a failing test should panic")]

use std::path::Path;
use std::time::{Duration, Instant};

use mergewright_core::journal::EntryStatus;
use mergewright_ops::apply::recover_interrupted;
use mergewright_ops::operation::{Chosen, CommitParams, Include, Operation, Part};
use mergewright_ops::rewrite::{RewriteRequest, TodoAction, plan};

use crate::commit_chosen::{
    at_head, commit_of, hook, names_dir, no_scratch_left, partly_staged, repo, scratch_copies,
    whole,
};
use crate::edit_stop::{facts, feature, id};
use crate::support::{NOW, Never, TempRepo, context, git_version, journal_for, run, try_run};

/// Wait for `path` to exist, or fail the test after 30 s.
fn wait_for(path: &Path) {
    let started = Instant::now();
    while !path.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "{} never appeared",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Window B is inside its `git commit` (its pre-commit hook waits) while
/// window A runs a chosen commit that fails at the same hook and cleans up.
/// B's commit must still be B's choice on top of every other tracked file:
/// with one shared scratch name, A's copy overwrote B's index and A's
/// clean-up removed it, and B committed an empty tree.
#[test]
fn another_window_cleaning_up_mid_commit_leaves_this_commit_whole() {
    let repo = repo();
    repo.write("a.rs", b"window b\n");
    repo.write("b.rs", b"window a\n");
    let marks = repo.path().join(".git");
    let b_waiting = marks.join("b-waiting");
    let a_done = marks.join("a-done");
    // The first commit to reach the hook is B's: it waits for A. A's fails.
    hook(
        &repo,
        "pre-commit",
        &format!(
            "if [ ! -e '{waiting}' ]; then touch '{waiting}'; \
             while [ ! -e '{done}' ]; do sleep 0.02; done; exit 0; \
             else echo window-a-refused >&2; exit 1; fi",
            waiting = b_waiting.display(),
            done = a_done.display(),
        ),
    );

    let b = std::thread::scope(|scope| {
        let b = scope.spawn(|| try_run(&repo, &commit_of(vec![whole("a.rs")]), NOW));
        wait_for(&b_waiting);
        let a = try_run(&repo, &commit_of(vec![whole("b.rs")]), NOW + 1);
        assert!(a.is_err(), "window A's hook refuses: {a:?}");
        std::fs::write(&a_done, b"").expect("the mark writes");
        b.join().expect("window B's thread")
    });

    let b = b.expect("window B's commit runs");
    assert_eq!(b.entry.status, EntryStatus::Committed);
    assert_eq!(
        repo.git(&["ls-tree", "--name-only", "HEAD"]),
        "a.rs\nb.rs\ngone.rs"
    );
    assert_eq!(repo.git(&["show", "HEAD:a.rs"]), "window b");
    assert_eq!(
        repo.git(&["show", "HEAD:b.rs"]),
        "two",
        "A's choice stays out"
    );
    no_scratch_left(&repo);
}

/// What a dead run left is never read: the next commit copies to its own
/// name, and recovery, holding the journal alone, sweeps the leftovers and
/// nothing else in the names dir (review ease-pass-04, Minor 4).
#[test]
fn a_dead_runs_scratch_index_is_never_read_and_recovery_sweeps_it() {
    let repo = repo();
    partly_staged(&repo);
    let names = names_dir(&repo);
    std::fs::write(names.join("commit-index"), b"junk").expect("an old leftover");
    std::fs::write(names.join("commit-index-1-0"), b"junk").expect("a dead run's");
    std::fs::write(names.join("tmp-index-1-0"), b"not ours").expect("a snapshot's");

    run(&repo, &commit_of(vec![whole("a.rs")]), NOW);
    assert_eq!(at_head(&repo, "a.rs"), "a3");

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    let recovered = recover_interrupted(&mut ctx).expect("recovery runs");
    assert!(
        recovered.scratch_left.is_empty(),
        "{:?}",
        recovered.scratch_left
    );
    no_scratch_left(&repo);
    assert!(
        names.join("tmp-index-1-0").exists(),
        "only commit copies go"
    );
}

/// While another `Journal` holds the writer's lock, a copy may be a live
/// run's: recovery sweeps nothing.
#[test]
fn recovery_sweeps_nothing_while_a_writer_holds_the_journal() {
    let repo = repo();
    let names = names_dir(&repo);
    std::fs::write(names.join("commit-index-1-0"), b"live").expect("a live run's");
    let handle = repo.open();
    let mut writer = journal_for(&handle);
    writer.claim().expect("the writer takes its lock");

    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW, &never);
    recover_interrupted(&mut ctx).expect("recovery runs");

    assert_eq!(scratch_copies(&repo), vec!["commit-index-1-0".to_owned()]);
}

/// No real index yet (a fresh `git init`): the copy is written as an empty
/// index, so the commit sees its file, and nothing is read as missing.
#[test]
fn a_first_commit_with_no_index_yet_commits_the_chosen_file_alone() {
    let repo = TempRepo::init();
    assert!(!repo.path().join(".git/index").exists(), "no index yet");
    repo.write("n.rs", b"new\n");
    repo.write("other.rs", b"left out\n");

    run(&repo, &commit_of(vec![whole("n.rs")]), NOW);

    assert_eq!(repo.git(&["ls-tree", "--name-only", "HEAD"]), "n.rs");
    assert_eq!(repo.git(&["status", "--porcelain"]), "?? other.rs");
    no_scratch_left(&repo);
}

/// A chosen-files amend at an edit stop of Mergewright's rewrite folds, on a
/// scratch index of its own that is gone after, holding the journal while it
/// exists (review ease-pass-04, Minor 1): only the chosen file goes in, the
/// other stays staged.
#[test]
fn a_chosen_amend_inside_mergewright_s_rewrite_folds_on_its_own_index() {
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
    assert!(
        try_run(&repo, &Operation::Rewrite(params), NOW).is_err(),
        "a stop"
    );
    repo.write("a1.txt", b"a1 chosen\n");
    repo.write("staged.txt", b"staged\n");
    repo.git(&["add", "staged.txt"]);
    let chosen = Operation::Amend(CommitParams {
        message: "a1".to_owned(),
        sign_off: false,
        no_verify: false,
        allow_empty: false,
        include: Include::Chosen(vec![Chosen {
            path: "a1.txt".into(),
            part: Part::Whole,
        }]),
    });

    try_run(&repo, &chosen, NOW + 1)
        .unwrap_or_else(|why| panic!("the amend runs: {}", why.explain(&crate::support::Letters)));

    assert_eq!(repo.git(&["show", "HEAD:a1.txt"]), "a1 chosen");
    assert_eq!(repo.git(&["diff", "--cached", "--name-only"]), "staged.txt");
    no_scratch_left(&repo);
}
