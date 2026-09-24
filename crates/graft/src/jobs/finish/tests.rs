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

    let missing = staged_diff(&store, Path::new("nowhere.md"), &())
        .expect_err("a path with no staged change");
    assert_eq!(missing, "nowhere.md is not staged");
}
