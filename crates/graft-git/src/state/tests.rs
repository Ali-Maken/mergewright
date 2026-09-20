use std::fs;

use super::*;
use crate::fixture::Fixture;

fn open(fixture: &Fixture) -> RepoHandle {
    RepoHandle::open(fixture.root()).expect("a fixture repository opens")
}

#[test]
fn an_ordinary_repository_has_nothing_in_progress() {
    let fixture = Fixture::with_two_commits();
    let repo = open(&fixture);
    assert_eq!(in_progress(&repo).expect("state reads"), None);
}

#[test]
fn a_conflicting_merge_is_detected_and_cleared_by_abort() {
    let fixture = Fixture::with_two_commits();
    fixture.git(&["checkout", "-q", "-b", "side"]);
    fs::write(fixture.root().join("f.txt"), "side\n").expect("file written");
    fixture.git(&["add", "f.txt"]);
    fixture.commit("side change");
    fixture.git(&["checkout", "-q", "main"]);
    fs::write(fixture.root().join("f.txt"), "main\n").expect("file written");
    fixture.git(&["add", "f.txt"]);
    fixture.commit("main change");

    // A merge with a real conflict on `f.txt` fails and leaves `MERGE_HEAD`
    // behind; `git merge` itself exits non-zero here, which is expected.
    let mut command = std::process::Command::new("git");
    command
        .args(["merge", "side", "-m", "merge"])
        .current_dir(fixture.root())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("LC_ALL", "C")
        .env("GIT_AUTHOR_NAME", "Graft Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_AUTHOR_DATE", "2020-01-01T00:00:00+00:00")
        .env("GIT_COMMITTER_NAME", "Graft Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_DATE", "2020-01-01T00:00:00+00:00")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY");
    let status = command.status().expect("git merge runs");
    assert!(!status.success(), "the merge conflicts, as scripted");

    let repo = open(&fixture);
    assert_eq!(
        in_progress(&repo).expect("state reads"),
        Some(InProgress::Merge)
    );

    fixture.git(&["merge", "--abort"]);
    assert_eq!(in_progress(&repo).expect("state reads"), None);
}

/// Runs `git` hermetically in `fixture`'s root, the same way the conflicting
/// merge test above does, but returns the exit status instead of asserting
/// success — for a command scripted to fail, like a conflicting rebase.
fn run_expecting_maybe_failure(fixture: &Fixture, args: &[&str]) -> std::process::ExitStatus {
    std::process::Command::new("git")
        .args(args)
        .current_dir(fixture.root())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("LC_ALL", "C")
        .env("GIT_AUTHOR_NAME", "Graft Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_AUTHOR_DATE", "2020-01-01T00:00:00+00:00")
        .env("GIT_COMMITTER_NAME", "Graft Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_DATE", "2020-01-01T00:00:00+00:00")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .status()
        .expect("git runs")
}

#[test]
fn in_progress_detail_reports_step_head_name_and_onto_mid_rebase() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("f.txt"), "base\n").expect("file written");
    fixture.git(&["add", "f.txt"]);
    fixture.commit("add f");

    fixture.git(&["checkout", "-q", "-b", "feature"]);
    fs::write(fixture.root().join("f.txt"), "feature\n").expect("file written");
    fixture.git(&["add", "f.txt"]);
    fixture.commit("feature change");

    fixture.git(&["checkout", "-q", "main"]);
    fs::write(fixture.root().join("f.txt"), "main\n").expect("file written");
    fixture.git(&["add", "f.txt"]);
    fixture.commit("main change");
    let onto = fixture.rev_parse("main");

    fixture.git(&["checkout", "-q", "feature"]);
    let status = run_expecting_maybe_failure(&fixture, &["rebase", "main"]);
    assert!(!status.success(), "the rebase conflicts, as scripted");

    let repo = open(&fixture);
    let detail = in_progress_detail(&repo)
        .expect("state reads")
        .expect("a rebase is in progress");
    assert_eq!(detail.kind, InProgress::Rebase);
    assert_eq!(detail.step, Some((1, 1)));
    assert_eq!(detail.head_name.as_deref(), Some("refs/heads/feature"));
    assert_eq!(detail.onto, Some(Oid::from_hex(&onto).expect("a valid id")));

    fixture.git(&["rebase", "--abort"]);
    assert_eq!(in_progress_detail(&repo).expect("state reads"), None);
}

#[test]
fn mergetool_is_none_until_configured() {
    let fixture = Fixture::with_two_commits();
    let repo = open(&fixture);
    assert_eq!(mergetool(&repo).expect("config reads"), None);

    fixture.git(&["config", "merge.tool", "vimdiff"]);
    assert_eq!(
        mergetool(&repo).expect("config reads"),
        Some("vimdiff".to_owned())
    );
}

#[test]
fn names_and_commands_are_stable() {
    assert_eq!(InProgress::Merge.name(), "merge");
    assert_eq!(InProgress::Merge.continue_command(), "git commit");
    assert_eq!(InProgress::Merge.abort_command(), "git merge --abort");
    assert_eq!(InProgress::Rebase.name(), "rebase");
    assert_eq!(
        InProgress::Rebase.continue_command(),
        "git rebase --continue"
    );
    assert_eq!(InProgress::Rebase.abort_command(), "git rebase --abort");
    assert_eq!(InProgress::CherryPick.name(), "cherry-pick");
    assert_eq!(InProgress::Revert.name(), "revert");
    assert_eq!(InProgress::Bisect.name(), "bisect");
}
