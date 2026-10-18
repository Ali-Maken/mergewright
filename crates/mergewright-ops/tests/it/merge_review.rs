//! A merge stages its solved files once, inside the finishing operation
//! (ease-pass/05b, hard choice 3): the review reads them from a copy of the
//! index, `Resume::Continue { solved }` stages them in the live entry's span,
//! and a merge begun in the shell stages them as `FinishMerge`'s first line.
//! Truth is git's own answer in a throwaway repository.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::PathBuf;

use mergewright_core::journal::{EntryStatus, OperationKind};
use mergewright_ops::apply::{ApplyError, Resume};
use mergewright_ops::finish::{ByGit, ByYou, Change, ResultIs, SummaryError, merge_review};
use mergewright_ops::operation::Refusal;

use crate::conflict_fixtures::{id, mergewright_merge_stops, paths, run_resume, two_file_conflict};
use crate::finish::refuse_commits;
use crate::support::{NOW, TempRepo, journal_for, run_undo};

/// Both files picked the way the resolver writes them: no markers left,
/// still unmerged in the index.
fn solve_both(repo: &TempRepo) {
    repo.write("a.txt", b"1\nF\n3\n");
    repo.write("b.txt", b"M\nF\n");
}

fn unmerged(repo: &TempRepo) -> String {
    repo.git(&["ls-files", "-u"])
}

fn index_bytes(repo: &TempRepo) -> Vec<u8> {
    std::fs::read(repo.path().join(".git/index")).expect("the index reads")
}

/// Every review copy left in the journal's directory.
fn review_copies(repo: &TempRepo) -> Vec<String> {
    let journal = mergewright_ops::journal::journal_path(&repo.open()).expect("the journal path");
    let Ok(dir) = std::fs::read_dir(journal.parent().expect("a directory")) else {
        return Vec::new();
    };
    dir.map(|entry| {
        entry
            .expect("an entry")
            .file_name()
            .to_string_lossy()
            .into_owned()
    })
    .filter(|name| name.starts_with("tmp-review-index"))
    .collect()
}

fn entry_count(repo: &TempRepo) -> usize {
    journal_for(&repo.open()).entries().len()
}

fn explain(why: &ApplyError) -> String {
    why.explain(&crate::support::Letters)
}

#[test]
fn the_review_reads_solved_files_from_a_copy_and_leaves_the_real_index_alone() {
    let repo = two_file_conflict();
    mergewright_merge_stops(&repo);
    solve_both(&repo);
    let index = index_bytes(&repo);
    let stages = unmerged(&repo);

    let review = merge_review(&repo.open(), &()).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(review.solved, paths(&["a.txt", "b.txt"]));
    let summary = review.summary;

    assert_eq!(
        summary.by_you,
        vec![
            ByYou {
                path: PathBuf::from("a.txt"),
                result: ResultIs::Theirs,
            },
            ByYou {
                path: PathBuf::from("b.txt"),
                result: ResultIs::Combined,
            },
        ]
    );
    assert_eq!(
        summary.by_git,
        vec![ByGit {
            path: PathBuf::from("clean.txt"),
            change: Change::Modified,
            added: 2,
            removed: 1,
        }]
    );
    assert_eq!(index_bytes(&repo), index, "the real index, byte for byte");
    assert_eq!(unmerged(&repo), stages, "git's conflict stages are kept");
    assert!(
        review_copies(&repo).is_empty(),
        "{:?}",
        review_copies(&repo)
    );
}

/// One file staged already (a whole-file choice, or the shell), one solved
/// and still unmerged: the review lists them in git's path order, as it
/// did when every file was staged before it opened.
#[test]
fn the_review_lists_staged_and_solved_files_in_path_order() {
    let repo = two_file_conflict();
    mergewright_merge_stops(&repo);
    solve_both(&repo);
    repo.git(&["add", "a.txt"]);

    let review = merge_review(&repo.open(), &()).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(review.solved, paths(&["b.txt"]), "a.txt is staged already");
    let summary = review.summary;

    let listed: Vec<PathBuf> = summary.by_you.into_iter().map(|you| you.path).collect();
    assert_eq!(listed, paths(&["a.txt", "b.txt"]));
}

/// What the review reads with nothing named: every unmerged file sorted by
/// its bytes on disk — a file with no clash left is solved, one with a clash
/// still open is counted — so a relaunch or the banner's `c` needs no picks
/// in memory.
#[test]
fn the_merge_review_sorts_unmerged_files_by_what_is_on_disk() {
    let repo = two_file_conflict();
    mergewright_merge_stops(&repo);
    repo.write("a.txt", b"1\nF\n3\n");
    let why = merge_review(&repo.open(), &()).expect_err("b.txt is open");
    assert!(
        matches!(
            why,
            SummaryError::ClashesOpen {
                files: 1,
                clashes: 1
            }
        ),
        "{why:?}"
    );

    repo.write("b.txt", b"M\nF\n");
    let review = merge_review(&repo.open(), &()).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(review.solved, paths(&["a.txt", "b.txt"]));
    assert_eq!(review.summary.by_you.len(), 2);
    assert!(review_copies(&repo).is_empty());
}

/// The read counts clashes, not files: the resolver's unit, so his `y`
/// and the read's refusal say the same number.
#[test]
fn a_file_with_two_open_clashes_is_refused_with_its_clash_count() {
    let repo = TempRepo::init();
    repo.write("two.txt", b"1\n2\n3\n4\n5\n6\n7\n");
    repo.commit_all("base");
    repo.git(&["branch", "feature"]);
    repo.write("two.txt", b"M\n2\n3\n4\n5\n6\nM\n");
    repo.commit_all("main's side");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("two.txt", b"F\n2\n3\n4\n5\n6\nF\n");
    repo.commit_all("feature's side");
    repo.git(&["switch", "-q", "main"]);
    mergewright_merge_stops(&repo);

    let why = merge_review(&repo.open(), &()).expect_err("two.txt has two clashes");
    assert!(
        matches!(
            why,
            SummaryError::ClashesOpen {
                files: 1,
                clashes: 2
            }
        ),
        "{why:?}"
    );
    assert!(review_copies(&repo).is_empty());
}

#[test]
fn a_mergewright_merge_stages_its_solved_files_and_commits_in_one_entry() {
    let repo = two_file_conflict();
    let main = id(&repo, "main");
    mergewright_merge_stops(&repo);
    let entries = entry_count(&repo);
    solve_both(&repo);

    let applied = run_resume(
        &repo,
        Resume::Continue {
            solved: paths(&["a.txt", "b.txt"]),
        },
        NOW + 1,
    )
    .unwrap_or_else(|why| panic!("{}", explain(&why)));

    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert_eq!(
        entry_count(&repo),
        entries,
        "the live entry closed, no new one"
    );
    assert_eq!(
        applied.entry.commands[applied.entry.commands.len() - 2..],
        [
            "git --literal-pathspecs add -- a.txt b.txt".to_owned(),
            "git merge --continue".to_owned(),
        ]
    );
    assert_eq!(repo.git(&["rev-parse", "HEAD^1"]), main.to_string());
    assert_eq!(
        repo.git(&["rev-parse", "HEAD^2"]),
        id(&repo, "feature").to_string()
    );
    assert_eq!(repo.git(&["status", "--porcelain"]), "", "index clean");
    assert_eq!(repo.read("b.txt"), b"M\nF\n");

    run_undo(&repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(id(&repo, "HEAD"), main, "u puts the merge back");
}

#[test]
fn a_shell_merge_stages_its_solved_files_as_finish_merge_s_first_line() {
    let repo = two_file_conflict();
    assert!(
        repo.try_git(&["merge", "feature"]).is_err(),
        "the merge stops"
    );
    solve_both(&repo);
    let before_stages = unmerged(&repo);

    let applied = run_resume(
        &repo,
        Resume::Continue {
            solved: paths(&["a.txt", "b.txt"]),
        },
        NOW,
    )
    .unwrap_or_else(|why| panic!("{}", explain(&why)));

    assert_eq!(applied.entry.kind, OperationKind::FinishMerge);
    assert_eq!(entry_count(&repo), 1, "one entry");
    assert_eq!(
        applied.entry.commands,
        [
            "git --literal-pathspecs add -- a.txt b.txt".to_owned(),
            "git merge --continue".to_owned(),
        ]
    );
    assert_eq!(
        repo.git(&["rev-parse", "HEAD^2"]),
        id(&repo, "feature").to_string()
    );
    assert_eq!(repo.git(&["status", "--porcelain"]), "");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(
        unmerged(&repo),
        before_stages,
        "u brings the conflict stages back"
    );
    assert_eq!(repo.read("b.txt"), b"M\nF\n", "and the solved file");
}

#[test]
fn a_file_named_solved_with_a_clash_still_open_is_refused_and_nothing_is_staged() {
    for shell in [false, true] {
        let repo = two_file_conflict();
        if shell {
            assert!(repo.try_git(&["merge", "feature"]).is_err());
        } else {
            mergewright_merge_stops(&repo);
        }
        repo.write("a.txt", b"1\nF\n3\n");
        let stages = unmerged(&repo);
        let entries = entry_count(&repo);

        let why = run_resume(
            &repo,
            Resume::Continue {
                solved: paths(&["a.txt", "b.txt"]),
            },
            NOW + 1,
        )
        .expect_err("b.txt still has its clash");

        assert!(
            matches!(&why, ApplyError::Refused(Refusal::MarkersRemain { path }) if path == &PathBuf::from("b.txt")),
            "shell {shell}: {}",
            explain(&why)
        );
        assert_eq!(unmerged(&repo), stages, "shell {shell}: nothing staged");
        assert_eq!(
            entry_count(&repo),
            entries,
            "shell {shell}: nothing journaled"
        );
    }
}

#[test]
fn an_unmerged_file_not_named_solved_is_refused_by_name() {
    let repo = two_file_conflict();
    mergewright_merge_stops(&repo);
    solve_both(&repo);
    let why = run_resume(
        &repo,
        Resume::Continue {
            solved: paths(&["a.txt"]),
        },
        NOW + 1,
    )
    .expect_err("b.txt is not named");
    assert!(
        matches!(&why, ApplyError::Refused(Refusal::Unmerged { paths: open }) if open == &paths(&["b.txt"])),
        "{}",
        explain(&why)
    );
}

/// `git merge --continue` concludes a merge through `git commit`, which runs
/// `pre-commit` (not `pre-merge-commit`): that is the hook refused here.
#[test]
fn a_hook_refusal_keeps_the_merge_and_the_same_finish_again_commits() {
    for shell in [false, true] {
        let repo = two_file_conflict();
        if shell {
            assert!(repo.try_git(&["merge", "feature"]).is_err());
        } else {
            mergewright_merge_stops(&repo);
        }
        solve_both(&repo);
        refuse_commits(&repo);
        let how = Resume::Continue {
            solved: paths(&["a.txt", "b.txt"]),
        };

        let why = run_resume(&repo, how.clone(), NOW + 1).expect_err("the hook refuses");
        assert!(
            explain(&why).contains("refused by hook"),
            "shell {shell}: {}",
            explain(&why)
        );
        assert!(
            repo.exists(".git/MERGE_HEAD"),
            "shell {shell}: still merging"
        );

        repo.git(&["config", "--unset", "core.hooksPath"]);
        run_resume(&repo, how, NOW + 2)
            .unwrap_or_else(|why| panic!("shell {shell}: {}", explain(&why)));
        assert!(!repo.exists(".git/MERGE_HEAD"), "shell {shell}: merged");
        assert_eq!(
            repo.git(&["rev-parse", "HEAD^2"]),
            id(&repo, "feature").to_string()
        );
    }
}
