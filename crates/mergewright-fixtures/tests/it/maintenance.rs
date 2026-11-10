//! No fixture command leaves git working in the background
//! (pre-publish-cleanup/02).
//!
//! A `git commit` (and a merge, rebase, fetch …) ends by starting
//! `git maintenance run --auto --detach`. Git 2.54's geometric strategy can
//! then repack the fixture's loose objects and delete them, empty fan-out
//! folders included, while the fixture's next command writes an object into
//! one of those folders: `unable to create temporary file` on a `git add`,
//! intermittent and only under load (the `fork_rows` walk flake). The
//! fixture runner switches auto maintenance off for every command it runs;
//! this proves no background maintenance is started, which is deterministic
//! where the race itself is not.

use std::path::Path;

use mergewright_fixtures::git::{isolated, run_at};

const DATE: &str = "2020-01-01T00:00:00+00:00";

fn fixture_git(repo: &Path, args: &[&str]) {
    run_at(repo, args, DATE).unwrap_or_else(|why| panic!("git {args:?}: {why}"));
}

/// The `child_start` events of the trace2 event stream at `trace`: every
/// process the traced git started, detached ones included.
fn children(trace: &Path) -> Vec<String> {
    let events = std::fs::read_to_string(trace)
        .unwrap_or_else(|why| panic!("the trace {}: {why}", trace.display()));
    events
        .lines()
        .filter(|line| line.contains("\"event\":\"child_start\""))
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_commit_through_the_fixture_runner_starts_no_background_maintenance() {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let repo = dir.path();
    fixture_git(repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("a.txt"), "a\n").unwrap_or_else(|why| panic!("write: {why}"));
    fixture_git(repo, &["add", "-A"]);

    // The fixture runner's own command, traced: `run_at` is `isolated` plus
    // a clock and an identity, and the trace needs an environment variable
    // `run_at` does not take.
    let trace = repo.join(".git").join("pp-02-trace.json");
    let out = isolated(
        repo,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-q",
            "-m",
            "a",
        ],
    )
    .env("GIT_TRACE2_EVENT", &trace)
    .output()
    .unwrap_or_else(|why| panic!("git commit: {why}"));
    assert!(
        out.status.success(),
        "git commit: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let maintenance: Vec<String> = children(&trace)
        .into_iter()
        .filter(|event| event.contains("\"maintenance\""))
        .collect();
    assert!(
        maintenance.is_empty(),
        "a fixture commit started background maintenance: {maintenance:?}"
    );
}
