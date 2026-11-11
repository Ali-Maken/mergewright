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
        .env("GIT_AUTHOR_NAME", "Maker Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_AUTHOR_DATE", "2020-01-01T00:00:00+00:00")
        .env("GIT_COMMITTER_NAME", "Maker Fixture")
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
        .env("GIT_AUTHOR_NAME", "Maker Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_AUTHOR_DATE", "2020-01-01T00:00:00+00:00")
        .env("GIT_COMMITTER_NAME", "Maker Fixture")
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

/// A `main`/`side` pair that conflicts on `f.txt`, checked out on `main`.
fn diverged() -> Fixture {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("f.txt"), "base\n").expect("file written");
    fixture.git(&["add", "f.txt"]);
    fixture.commit("add f");
    fixture.git(&["checkout", "-q", "-b", "side"]);
    fs::write(fixture.root().join("f.txt"), "side\n").expect("file written");
    fixture.git(&["add", "f.txt"]);
    fixture.commit("side change");
    fixture.git(&["checkout", "-q", "main"]);
    fs::write(fixture.root().join("f.txt"), "main\n").expect("file written");
    fixture.git(&["add", "f.txt"]);
    fixture.commit("main change");
    fixture
}

fn incoming_of(fixture: &Fixture) -> Option<(Oid, String)> {
    in_progress_detail(&open(fixture))
        .expect("state reads")
        .expect("an operation is in progress")
        .theirs
        .expect("theirs is named")
        .map(|summary| (summary.id, summary.subject))
}

#[test]
fn in_progress_detail_reads_the_incoming_commit() {
    let side = |fixture: &Fixture| Oid::from_hex(&fixture.rev_parse("side")).expect("an id");

    let merge = diverged();
    assert!(!run_expecting_maybe_failure(&merge, &["merge", "side", "-m", "m"]).success());
    assert_eq!(
        incoming_of(&merge),
        Some((side(&merge), "side change".into()))
    );

    let pick = diverged();
    assert!(!run_expecting_maybe_failure(&pick, &["cherry-pick", "side"]).success());
    assert_eq!(
        incoming_of(&pick),
        Some((side(&pick), "side change".into()))
    );

    let revert = diverged();
    revert.git(&["checkout", "-q", "side"]);
    fs::write(revert.root().join("f.txt"), "later\n").expect("file written");
    revert.git(&["add", "f.txt"]);
    revert.commit("later change");
    let reverted = Oid::from_hex(&revert.rev_parse("HEAD~1")).expect("an id");
    assert!(!run_expecting_maybe_failure(&revert, &["revert", "--no-edit", "HEAD~1"]).success());
    assert_eq!(incoming_of(&revert), Some((reverted, "side change".into())));

    for backend in ["--merge", "--apply"] {
        let rebase = diverged();
        rebase.git(&["checkout", "-q", "side"]);
        assert!(!run_expecting_maybe_failure(&rebase, &["rebase", backend, "main"]).success());
        assert_eq!(
            incoming_of(&rebase),
            Some((side(&rebase), "side change".into())),
            "{backend}"
        );
    }

    let idle = diverged();
    assert_eq!(in_progress_detail(&open(&idle)).expect("state reads"), None);
}

/// A head file naming a commit git cannot read (Minor 2 of
/// `docs/reviews/m6-amend-10-22.md`): theirs' name is optional detail, so
/// the operation itself must still read — it is what offers `--abort`.
#[test]
fn an_unreadable_incoming_commit_still_reads_the_operation() {
    let merge = diverged();
    assert!(!run_expecting_maybe_failure(&merge, &["merge", "side", "-m", "m"]).success());
    let missing = "1111111111111111111111111111111111111111";
    fs::write(merge.root().join(".git/MERGE_HEAD"), format!("{missing}\n")).expect("written");

    let detail = in_progress_detail(&open(&merge))
        .expect("the operation reads though theirs cannot be named")
        .expect("a merge is in progress");
    assert_eq!(detail.kind, InProgress::Merge);
    // Not swallowed: the failure is carried where the name would be.
    let error = detail.theirs.expect_err("theirs cannot be named");
    assert_eq!(error.file, "MERGE_HEAD");
    assert!(error.reason.contains(missing), "{error}");
}

/// A stop ends the one git run the detail makes, and says so on theirs.
#[test]
fn a_stopped_incoming_read_is_carried_on_theirs() {
    let merge = diverged();
    assert!(!run_expecting_maybe_failure(&merge, &["merge", "side", "-m", "m"]).success());
    let stopped = std::sync::atomic::AtomicBool::new(true);

    let detail = in_progress_detail_with_stop(&open(&merge), &stopped)
        .expect("the operation reads")
        .expect("a merge is in progress");
    let error = detail.theirs.expect_err("the listing was stopped");
    assert_eq!(error.reason, RunError::Stopped.to_string());
}
