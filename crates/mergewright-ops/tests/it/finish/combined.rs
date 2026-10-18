//! `finish::combined_by_git` (ease-pass/11b): the clash screen's
//! `Combined by git (N)`, against git's own answer.

use std::collections::BTreeSet;
use std::path::PathBuf;

use mergewright_ops::finish::{SummaryError, combined_by_git};

use crate::conflict_fixtures::{mergewright_merge_stops, mergewright_rebase_stops};
use crate::support::TempRepo;

/// A merge of `feature` into `main` that stops on `a.txt` and `b.txt` and
/// combines `clean.txt` (modified) and `new.txt` (added) on its own.
fn two_combined_two_clashing() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"1\n2\n3\n");
    repo.write("b.txt", b"x\n");
    repo.write("clean.txt", b"k\nl\n");
    repo.commit_all("base");
    repo.git(&["branch", "feature"]);
    repo.write("a.txt", b"1\nM\n3\n");
    repo.write("b.txt", b"M\n");
    repo.commit_all("main's side");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("a.txt", b"1\nF\n3\n");
    repo.write("b.txt", b"F\n");
    repo.write("clean.txt", b"k2\nl\n");
    repo.write("new.txt", b"n\n");
    repo.commit_all("feature's side");
    repo.git(&["switch", "-q", "main"]);
    repo
}

/// Paths from git's output, one per line; `ls-files -u` puts its path
/// after a tab.
fn names(out: &str) -> BTreeSet<PathBuf> {
    out.lines()
        .filter(|l| !l.is_empty())
        .map(|l| PathBuf::from(l.rsplit('\t').next().unwrap_or(l)))
        .collect()
}

#[test]
fn combined_by_git_is_staged_minus_unmerged() {
    let repo = two_combined_two_clashing();
    mergewright_merge_stops(&repo);

    let got = combined_by_git(&repo.open(), &()).unwrap_or_else(|why| panic!("{why}"));

    let staged = names(&repo.git(&["diff", "--cached", "--name-only"]));
    let unmerged = names(&repo.git(&["ls-files", "-u"]));
    let want: BTreeSet<PathBuf> = staged.difference(&unmerged).cloned().collect();
    assert_eq!(
        want,
        [PathBuf::from("clean.txt"), PathBuf::from("new.txt")].into(),
        "git's own answer"
    );
    assert_eq!(
        got,
        vec![PathBuf::from("clean.txt"), PathBuf::from("new.txt")]
    );
}

#[test]
fn combined_by_git_is_merge_only() {
    let repo = two_combined_two_clashing();
    mergewright_rebase_stops(&repo);
    assert!(matches!(
        combined_by_git(&repo.open(), &()),
        Err(SummaryError::NotMerging)
    ));
}
