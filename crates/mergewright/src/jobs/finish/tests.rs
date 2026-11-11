//! `jobs::finish` against real git.

use std::path::Path;
use std::process::Command as Shell;

pub(super) fn git(repo: &Path, args: &[&str]) -> String {
    let out = Shell::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Maker Test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A merge stopped on `notes.txt`, solved on disk the way the resolver
/// writes it, and nothing staged.
pub(super) fn solved_merge() -> mergewright_fixtures::Scratch {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("notes.txt"), "a\nbase\nc\n").expect("writable");
    git(repo, &["add", "notes.txt"]);
    git(repo, &["commit", "-q", "-m", "base"]);
    git(repo, &["checkout", "-q", "-b", "feature"]);
    std::fs::write(repo.join("notes.txt"), "a\nTHEIRS\nc\n").expect("writable");
    git(repo, &["commit", "-q", "-am", "theirs"]);
    git(repo, &["checkout", "-q", "-"]);
    std::fs::write(repo.join("notes.txt"), "a\nOURS\nc\n").expect("writable");
    git(repo, &["commit", "-q", "-am", "ours"]);
    let merged = Shell::new("git")
        .arg("-C")
        .arg(repo)
        .args(["merge", "-q", "feature"])
        .output()
        .expect("git runs");
    assert!(!merged.status.success(), "the merge stops on notes.txt");
    std::fs::write(repo.join("notes.txt"), "a\nTHEIRS\nc\n").expect("writable");
    scratch
}

/// The review reads what is solved from disk (ease-pass/05b): the solved
/// file comes back named, and nothing is staged by reading.
#[test]
fn the_review_names_the_files_solved_on_disk_and_stages_nothing() {
    let scratch = solved_merge();
    let store = crate::repos::RepoStore::new(scratch.path().to_path_buf(), None);
    let review = super::summary(&store).expect("the review reads");
    assert_eq!(review.solved, vec![std::path::PathBuf::from("notes.txt")]);
    assert_eq!(review.summary.by_you.len(), 1);
    assert!(
        git(scratch.path(), &["ls-files", "-u"]).contains("notes.txt"),
        "still unmerged"
    );
}

/// With a clash still open the review is refused with the open files and
/// their clashes, so the app can say so.
#[test]
fn the_review_with_a_clash_still_open_says_how_many_files() {
    let scratch = solved_merge();
    std::fs::write(
        scratch.path().join("notes.txt"),
        "a\n<<<<<<< HEAD\nOURS\n=======\nTHEIRS\n>>>>>>> feature\nc\n",
    )
    .expect("writable");
    let store = crate::repos::RepoStore::new(scratch.path().to_path_buf(), None);
    assert_eq!(
        super::summary(&store),
        Err(crate::update::finish::ReviewFailed::StillOpen {
            files: 1,
            clashes: 1
        })
    );
}
