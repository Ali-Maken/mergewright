//! `jobs::finish` against real git.

use std::path::Path;
use std::process::Command as Shell;

use graft_ui::detail::LineKindView;

use super::staged_diff;

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Shell::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Graft Test",
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

/// The lines of `git diff --cached <path>` that start with `prefix` (`-` or
/// `+`), headers left out, prefix stripped.
fn git_lines(repo: &Path, path: &str, prefix: char) -> Vec<String> {
    git(repo, &["diff", "--cached", "--", path])
        .lines()
        .filter(|line| !line.starts_with("---") && !line.starts_with("+++"))
        .filter_map(|line| line.strip_prefix(prefix).map(str::to_owned))
        .collect()
}

#[test]
fn the_diff_is_the_staged_change() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("README.md"), "one\ntwo\nthree\nfour\n").expect("writable");
    git(repo, &["add", "README.md"]);
    git(repo, &["commit", "-q", "-m", "readme"]);
    std::fs::write(repo.join("README.md"), "one\nTWO\nthree\nfour\n").expect("writable");
    git(repo, &["add", "README.md"]);
    // An unstaged edit on top: the staged diff must not show it.
    std::fs::write(repo.join("README.md"), "one\nTWO\nthree\nFOUR\n").expect("writable");

    let store = crate::repos::RepoStore::new(repo.to_path_buf(), None);
    let view = staged_diff(&store, Path::new("README.md"), &()).expect("the diff reads");
    assert_eq!(view.hunks.len(), 1, "{view:?}");
    let ours = |kind: LineKindView| -> Vec<String> {
        view.hunks[0]
            .lines
            .iter()
            .filter(|line| line.kind == kind)
            .map(|line| line.text.as_str().to_owned())
            .collect()
    };
    assert_eq!(
        ours(LineKindView::Removed),
        git_lines(repo, "README.md", '-')
    );
    assert_eq!(ours(LineKindView::Added), git_lines(repo, "README.md", '+'));
    assert_eq!(ours(LineKindView::Added), vec!["TWO".to_owned()]);

    // Staged but equal to `HEAD` (what keeping yours leaves): no change,
    // not an error.
    let same =
        staged_diff(&store, Path::new("nowhere.md"), &()).expect("no change is not an error");
    assert!(same.hunks.is_empty() && same.binary.is_none(), "{same:?}");
}

/// A merge stopped on `notes.txt`, solved on disk the way the resolver
/// writes it, and nothing staged.
fn solved_merge() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
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

/// With a clash still open the review is refused with the count of open
/// files, so the app can close it and say so.
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
        Err(crate::update::finish::ReviewFailed::StillOpen { count: 1 })
    );
}

/// `space` on a solved row shows what the merge will commit for it — the
/// file on disk against `HEAD` — though git's index still holds its
/// conflict stages until `y`.
#[test]
fn a_solved_row_s_diff_is_the_file_on_disk_against_head() {
    let scratch = solved_merge();
    let store = crate::repos::RepoStore::new(scratch.path().to_path_buf(), None);
    let view = staged_diff(&store, Path::new("notes.txt"), &()).expect("the diff reads");
    let added: Vec<String> = view
        .hunks
        .iter()
        .flat_map(|hunk| hunk.lines.iter())
        .filter(|line| line.kind == LineKindView::Added)
        .map(|line| line.text.as_str().to_owned())
        .collect();
    assert_eq!(added, vec!["THEIRS".to_owned()], "{view:?}");
}
