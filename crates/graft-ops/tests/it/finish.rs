//! `finish::finish_summary` and `Resume::ContinueWith` (m6-conflicts/09):
//! what a merge will commit, split into what the user resolved and what git
//! combined on its own, and an edited message reaching git through
//! `commit -F`. Truth is git's own answer in a throwaway repository.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use graft_core::journal::EntryStatus;
use graft_git::state::InProgress;
use graft_ops::apply::{ApplyError, Resume};
use graft_ops::finish::{ByGit, ByYou, Change, ResultIs, SummaryError, finish_summary};
use graft_ops::operation::Refusal;

use crate::conflict_fixtures::{
    downstream_of, graft_merge_stops, graft_rebase_stops, graft_stage, id, run_resume,
    two_file_conflict,
};
use crate::support::{NOW, TempRepo, state_of};

/// `git diff --cached --numstat` for one path: git's own line counts.
fn numstat(repo: &TempRepo, path: &str) -> (usize, usize) {
    let line = repo.git(&["diff", "--cached", "--numstat", "--", path]);
    let mut fields = line.split('\t');
    let added = fields.next().expect("added").parse().expect("a count");
    let removed = fields.next().expect("removed").parse().expect("a count");
    (added, removed)
}

#[test]
fn summary_splits_yours_from_gits() {
    let repo = two_file_conflict();
    graft_merge_stops(&repo);
    repo.git(&["checkout", "--theirs", "a.txt", "b.txt"]);
    repo.git(&["add", "a.txt"]);
    repo.write("b.txt", b"M\nF\n");
    repo.git(&["add", "b.txt"]);

    let summary = finish_summary(&repo.open()).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(summary.into, b"main");
    assert_eq!(summary.from, b"feature");
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
    let (added, removed) = numstat(&repo, "clean.txt");
    assert_eq!(
        summary.by_git,
        vec![ByGit {
            path: PathBuf::from("clean.txt"),
            change: Change::Modified,
            added,
            removed,
        }]
    );
    assert_eq!(summary.message, b"Merge branch 'feature' into main\n");
    assert!(summary.recorded);
}

#[test]
fn yours_and_deleted_are_read_from_the_index() {
    let repo = two_file_conflict();
    graft_merge_stops(&repo);
    repo.git(&["checkout", "--ours", "a.txt"]);
    repo.git(&["add", "a.txt"]);
    repo.git(&["rm", "-q", "b.txt"]);
    let summary = finish_summary(&repo.open()).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(
        summary.by_you,
        vec![
            ByYou {
                path: PathBuf::from("a.txt"),
                result: ResultIs::Yours,
            },
            ByYou {
                path: PathBuf::from("b.txt"),
                result: ResultIs::Deleted,
            },
        ]
    );
}

#[test]
fn a_shell_merge_names_its_branch_and_a_pull_its_source() {
    let repo = two_file_conflict();
    assert!(repo.try_git(&["merge", "feature"]).is_err());
    repo.git(&["checkout", "--theirs", "a.txt", "b.txt"]);
    repo.git(&["add", "a.txt", "b.txt"]);
    let summary = finish_summary(&repo.open()).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(summary.from, b"feature");
    assert_eq!(summary.message, b"Merge branch 'feature'\n");

    let upstream = two_file_conflict();
    let repo = downstream_of(&upstream, false);
    let from = upstream.path().to_str().expect("utf-8 temp path");
    assert!(
        repo.try_git(&["pull", "--no-rebase", "--no-edit", from, "feature"])
            .is_err()
    );
    assert!(repo.exists(".git/MERGE_HEAD"), "the pull stopped mid-merge");
    repo.git(&["checkout", "--theirs", "a.txt"]);
    repo.git(&["add", "a.txt"]);
    let summary = finish_summary(&repo.open()).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(summary.from, format!("feature of {from}").into_bytes());
}

#[test]
fn detached_into_is_a_short_id() {
    let repo = two_file_conflict();
    repo.git(&["switch", "-q", "--detach", "main"]);
    assert!(repo.try_git(&["merge", "feature"]).is_err());
    repo.git(&["checkout", "--theirs", "a.txt", "b.txt"]);
    repo.git(&["add", "a.txt", "b.txt"]);
    let summary = finish_summary(&repo.open()).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(
        summary.into,
        repo.git(&["rev-parse", "--short", "HEAD"]).into_bytes()
    );
}

#[test]
fn unrecorded_puts_every_change_under_git() {
    let repo = two_file_conflict();
    assert!(repo.try_git(&["merge", "feature"]).is_err());
    repo.git(&["checkout", "--theirs", "a.txt", "b.txt"]);
    repo.git(&["add", "a.txt", "b.txt"]);
    // A MERGE_MSG with no list, and no Graft chain: nothing says which were
    // conflicts.
    repo.write(".git/MERGE_MSG", b"Merge branch 'feature'\n");
    let summary = finish_summary(&repo.open()).unwrap_or_else(|why| panic!("{why}"));
    assert!(!summary.recorded);
    assert!(summary.by_you.is_empty());
    let named: Vec<PathBuf> = summary.by_git.iter().map(|row| row.path.clone()).collect();
    assert_eq!(
        named,
        vec![
            PathBuf::from("a.txt"),
            PathBuf::from("b.txt"),
            PathBuf::from("clean.txt"),
        ]
    );
}

#[test]
fn summary_refuses_with_anything_unmerged() {
    let repo = two_file_conflict();
    graft_merge_stops(&repo);
    let refused = finish_summary(&repo.open()).expect_err("two files are unmerged");
    assert!(
        matches!(refused, SummaryError::StillUnmerged { count: 2 }),
        "{refused:?}"
    );
}

#[test]
fn summary_is_merge_only() {
    let repo = two_file_conflict();
    let refused = finish_summary(&repo.open()).expect_err("nothing in progress");
    assert!(matches!(refused, SummaryError::NotMerging), "{refused:?}");

    let repo = two_file_conflict();
    graft_rebase_stops(&repo);
    repo.git(&["checkout", "--theirs", "a.txt", "b.txt"]);
    repo.git(&["add", "a.txt", "b.txt"]);
    let refused = finish_summary(&repo.open()).expect_err("a rebase");
    assert!(matches!(refused, SummaryError::NotMerging), "{refused:?}");
}

/// A `pre-commit` hook that refuses, installed through `core.hooksPath`.
pub(crate) fn refuse_commits(repo: &TempRepo) -> PathBuf {
    let hooks = repo.path().join("hooks");
    std::fs::create_dir_all(&hooks).expect("the hooks directory");
    let hook = hooks.join("pre-commit");
    std::fs::write(&hook, b"#!/bin/sh\necho refused by hook >&2\nexit 1\n").expect("the hook");
    let mut permissions = std::fs::metadata(&hook)
        .expect("its metadata")
        .permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
    std::fs::set_permissions(&hook, permissions).expect("executable");
    repo.git(&[
        "config",
        "core.hooksPath",
        hooks.to_str().expect("utf-8 path"),
    ]);
    hooks
}

/// Graft's merge, stopped, both files hand-combined and marked done through
/// Graft.
fn graft_merge_resolved() -> TempRepo {
    let repo = two_file_conflict();
    graft_merge_stops(&repo);
    repo.write("a.txt", b"1\nboth\n3\n");
    graft_stage(&repo, "a.txt", NOW + 1);
    repo.write("b.txt", b"both\n");
    graft_stage(&repo, "b.txt", NOW + 2);
    repo
}

fn merge_msg_mtime(repo: &TempRepo) -> std::time::SystemTime {
    std::fs::metadata(repo.path().join(".git/MERGE_MSG"))
        .and_then(|meta| meta.modified())
        .expect("MERGE_MSG's mtime")
}

/// `git stripspace`: what `--cleanup=whitespace` makes of a message, from
/// git itself. Comment lines are kept.
pub(crate) fn stripspace(message: &[u8]) -> String {
    use std::io::Write;
    let mut child = std::process::Command::new("git")
        .arg("stripspace")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("stripspace runs");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(message)
        .expect("written");
    let output = child.wait_with_output().expect("stripspace ends");
    String::from_utf8(output.stdout).expect("utf-8")
}

/// `HEAD`'s message, byte for byte: `git cat-file commit`, everything after
/// the header's blank line. `%B` would add a newline of its own.
pub(crate) fn head_message(repo: &TempRepo) -> Vec<u8> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(["cat-file", "commit", "HEAD"])
        .output()
        .expect("cat-file runs");
    assert!(output.status.success(), "cat-file commit HEAD");
    let raw = output.stdout;
    let body = raw
        .windows(2)
        .position(|pair| pair == b"\n\n")
        .expect("a header ends in a blank line");
    raw[body + 2..].to_vec()
}

#[test]
fn an_edited_message_is_the_commit_message() {
    let repo = graft_merge_resolved();

    let message = b"Merge feature, edited  \n\n# a comment line\nwhy it was merged\n\n\n".to_vec();
    let bytes_before = repo.read(".git/MERGE_MSG");
    let mtime_before = merge_msg_mtime(&repo);

    // A hook refuses first: git keeps its state, so MERGE_MSG is still there
    // to prove it was never written.
    let hooks = refuse_commits(&repo);
    let refused = run_resume(
        &repo,
        Resume::ContinueWith {
            message: message.clone(),
        },
        NOW + 3,
    )
    .expect_err("the hook refuses");
    assert!(
        matches!(
            refused,
            ApplyError::ResumeFailed {
                kind: InProgress::Merge,
                ..
            }
        ),
        "{refused:?}"
    );
    assert_eq!(repo.read(".git/MERGE_MSG"), bytes_before);
    assert_eq!(merge_msg_mtime(&repo), mtime_before);
    std::fs::remove_dir_all(hooks).expect("the hook goes");
    repo.git(&["config", "core.hooksPath", "/dev/null"]);

    let applied = run_resume(
        &repo,
        Resume::ContinueWith {
            message: message.clone(),
        },
        NOW + 4,
    )
    .unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert_eq!(
        applied.entry.commands.last().map(String::as_str),
        Some("git commit --cleanup=whitespace -F -")
    );
    // `# a comment line` is kept: git's own comments were removed when the
    // summary read MERGE_MSG, so any left are the user's (m6-conflicts/22).
    let body = repo.git(&["log", "-1", "--format=%B"]);
    assert_eq!(format!("{body}\n"), stripspace(&message));
    assert!(body.lines().any(|line| line == "# a comment line"));
    assert_eq!(
        repo.git(&["rev-parse", "HEAD^2"]),
        id(&repo, "feature").to_string()
    );
    assert!(!repo.exists(".git/MERGE_HEAD"), "the merge is over");
    assert!(
        !repo.exists(".git/MERGE_MSG"),
        "git removed its own message"
    );
}

#[test]
fn continue_is_unchanged() {
    let repo = two_file_conflict();
    graft_merge_stops(&repo);
    repo.git(&["checkout", "--theirs", "a.txt", "b.txt"]);
    graft_stage(&repo, "a.txt", NOW + 1);
    graft_stage(&repo, "b.txt", NOW + 2);
    let applied = run_resume(&repo, Resume::Continue, NOW + 3)
        .unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert_eq!(
        applied.entry.commands.last().map(String::as_str),
        Some("git merge --continue")
    );
    assert_eq!(
        repo.git(&["log", "-1", "--format=%B"]),
        "Merge branch 'feature' into main"
    );
}

#[test]
fn a_message_on_a_rebase_is_refused_before_anything_runs() {
    let repo = two_file_conflict();
    graft_rebase_stops(&repo);
    repo.git(&["checkout", "--theirs", "a.txt", "b.txt"]);
    repo.git(&["add", "a.txt", "b.txt"]);
    let before = state_of(&repo);
    let backups_before = repo.git(&["for-each-ref", "--format=%(refname) %(objectname)"]);

    let refused = run_resume(
        &repo,
        Resume::ContinueWith {
            message: b"not here\n".to_vec(),
        },
        NOW + 1,
    )
    .expect_err("a rebase keeps its commits' messages");
    let ApplyError::Refused(refusal) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(
        refusal,
        Refusal::MessageNotHere {
            kind: InProgress::Rebase
        }
    );
    assert_eq!(
        refusal.explain(),
        "a rebase keeps each commit's own message · c continues"
    );
    assert_eq!(state_of(&repo), before, "nothing ran");
    assert_eq!(
        repo.git(&["for-each-ref", "--format=%(refname) %(objectname)"]),
        backups_before,
        "no backup was chained"
    );
    assert!(repo.exists(".git/rebase-merge"), "still rebasing");
}

#[test]
fn a_hash_line_the_user_typed_survives() {
    let repo = graft_merge_resolved();
    let message = b"Merge feature\n\n#123 fixes login\n".to_vec();
    let applied = run_resume(&repo, Resume::ContinueWith { message }, NOW + 3)
        .unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    let body = repo.git(&["log", "-1", "--format=%B"]);
    assert!(
        body.lines().any(|line| line == "#123 fixes login"),
        "{body:?}"
    );
}

/// Graft's merge with `core.commentChar` as given (`None` unset), stopped,
/// both files marked done: `MERGE_MSG` still carries git's comment block.
fn resolved_with_comment_char(comment: Option<&str>) -> TempRepo {
    let repo = two_file_conflict();
    if let Some(comment) = comment {
        repo.git(&["config", "core.commentChar", comment]);
    }
    graft_merge_stops(&repo);
    repo.write("a.txt", b"1\nboth\n3\n");
    graft_stage(&repo, "a.txt", NOW + 1);
    repo.write("b.txt", b"both\n");
    graft_stage(&repo, "b.txt", NOW + 2);
    repo
}

#[test]
fn gits_comment_lines_never_reach_the_summary() {
    for (comment, prefix) in [(None, "#"), (Some(";"), ";")] {
        let repo = resolved_with_comment_char(comment);
        let merge_msg = String::from_utf8(repo.read(".git/MERGE_MSG")).expect("utf-8");
        assert!(
            merge_msg.contains(&format!("{prefix} Conflicts:")),
            "git wrote its block under {prefix:?}: {merge_msg:?}"
        );
        let summary = finish_summary(&repo.open()).unwrap_or_else(|why| panic!("{why}"));
        let message = String::from_utf8(summary.message).expect("utf-8");
        assert!(!message.is_empty(), "the subject is kept");
        assert!(
            !message.lines().any(|line| line.starts_with(prefix)),
            "{prefix:?}: {message:?}"
        );
    }
}

#[test]
fn an_unedited_message_lands_as_shown() {
    let repo = graft_merge_resolved();
    let shown = finish_summary(&repo.open())
        .unwrap_or_else(|why| panic!("{why}"))
        .message;
    let applied = run_resume(&repo, Resume::Continue, NOW + 3)
        .unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    // Byte for byte: git ends a message with exactly one newline, and the
    // summary's message already does, so no normalisation is needed.
    assert_eq!(
        String::from_utf8_lossy(&head_message(&repo)),
        String::from_utf8_lossy(&shown)
    );
}
