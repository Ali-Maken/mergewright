//! Absorb's oracle: plan and apply, and judge the result by the trees git
//! holds afterwards.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use graft_core::oid::Oid;
use graft_ops::absorb::AbsorbMap;
use graft_ops::operation::Operation;
use graft_ops::rewrite::TodoAction;

use crate::absorb::{branch, map, oid, planned, stage};
use crate::absorb_printed::{PrintedHunk, each_target_last_changed_its_lines, is, staged_hunks};
use crate::support::{NOW, TempRepo, try_run};

// ── The oracle ──────────────────────────────────────────────────────────

/// `git` with a temporary index, for building the expected tree.
fn git_with_index(repo: &TempRepo, index: &Path, args: &[&str], stdin: Option<&[u8]>) -> String {
    use std::io::Write as _;
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(repo.path())
        .env("GIT_INDEX_FILE", index)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("git runs");
    if let Some(bytes) = stdin {
        child
            .stdin
            .take()
            .expect("a stdin")
            .write_all(bytes)
            .expect("stdin writes");
    } else {
        drop(child.stdin.take());
    }
    let output = child.wait_with_output().expect("git ends");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// The old `HEAD` tree plus `hunks`, each applied alone with
/// `--unidiff-zero`, bottom of each file first so the line numbers above
/// still hold.
fn tree_plus(repo: &TempRepo, commit: &str, hunks: &[&PrintedHunk]) -> String {
    let index = repo.path().join(".git/oracle-index");
    git_with_index(repo, &index, &["read-tree", commit], None);
    let mut ordered: Vec<&&PrintedHunk> = hunks.iter().collect();
    ordered.sort_by(|a, b| a.path.cmp(&b.path).then(b.old_start.cmp(&a.old_start)));
    for hunk in ordered {
        // Alone, the new side starts where the old one does (after the
        // line an insertion follows).
        let new_start = if hunk.old_lines == 0 {
            hunk.old_start + 1
        } else {
            hunk.old_start
        };
        let new_count = hunk
            .body
            .iter()
            .filter(|line| line.starts_with('+'))
            .count();
        let header = format!(
            "@@ -{},{} +{},{} @@",
            hunk.old_start, hunk.old_lines, new_start, new_count
        );
        let patch = format!(
            "diff --git a/{p} b/{p}\n--- a/{p}\n+++ b/{p}\n{header}\n{}\n",
            hunk.body.join("\n"),
            p = hunk.path
        );
        git_with_index(
            repo,
            &index,
            &["apply", "--cached", "--unidiff-zero", "-"],
            Some(patch.as_bytes()),
        );
    }
    let tree = git_with_index(repo, &index, &["write-tree"], None);
    std::fs::remove_file(&index).expect("the oracle's index goes");
    tree
}

fn subjects(repo: &TempRepo) -> Vec<String> {
    repo.git(&["log", "--format=%s", "main..feature"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The repository before the absorb, as the oracle compares against it.
struct Before {
    printed: Vec<PrintedHunk>,
    head: String,
    index: String,
    subjects: Vec<String>,
    subject_of: BTreeMap<Oid, String>,
}

impl Before {
    fn read(repo: &TempRepo) -> Before {
        let subjects = subjects(repo);
        let subject_of = subjects
            .iter()
            .enumerate()
            .map(|(back, subject)| (oid(repo, &format!("feature~{back}")), subject.clone()))
            .collect();
        Before {
            printed: staged_hunks(repo),
            head: repo.git(&["rev-parse", "HEAD"]),
            index: repo.git(&["write-tree"]),
            subjects,
            subject_of,
        }
    }
}

/// Plan `absorb`, apply it, and judge it by the trees: the new `HEAD` tree
/// is the old one plus the placed hunks; the index is the new `HEAD` plus
/// the hunks that stay (or, when the map says the put-back loses the split,
/// the working tree has them); each target's new commit carries its hunks.
fn absorb_agrees_with_the_trees(repo: &TempRepo, unsaved: &[(&str, &[u8])]) -> AbsorbMap {
    let before = Before::read(repo);
    let map = map(repo);
    each_target_last_changed_its_lines(repo, "main..feature", &before.printed, &map);
    for (path, bytes) in unsaved {
        repo.write(path, bytes);
    }

    let params = planned(repo).unwrap_or_else(|why| panic!("it plans: {why}"));
    let fixups = params
        .todo
        .iter()
        .filter(|line| line.action == TodoAction::Fixup)
        .count();
    assert_eq!(fixups, params.made.len(), "one fixup row per commit made");
    try_run(repo, &Operation::Rewrite(params), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(subjects(repo), before.subjects, "no commit added or lost");
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/feature");
    head_is_old_head_plus_placed(repo, &before, &map);
    index_and_worktree_hold_the_rest(repo, &before, unsaved);
    each_target_carries_its_hunks(repo, &before, &map);
    map
}

/// The new `HEAD` tree = the old `HEAD` tree + the placed hunks.
fn head_is_old_head_plus_placed(repo: &TempRepo, before: &Before, map: &AbsorbMap) {
    let placed: Vec<&PrintedHunk> = before
        .printed
        .iter()
        .filter(|hunk| map.placed.iter().any(|(staged, _)| is(hunk, staged)))
        .collect();
    assert_eq!(placed.len(), map.placed.len(), "every placed hunk printed");
    assert_eq!(
        repo.git(&["rev-parse", "HEAD^{tree}"]),
        tree_plus(repo, &before.head, &placed),
        "new HEAD = old HEAD + placed"
    );
}

/// The index = the new `HEAD` + the hunks that stay — the old index's tree
/// exactly, since the new `HEAD` is the old one plus the placed hunks (06b:
/// what stays is still staged) — and the working tree holds every staged
/// line.
fn index_and_worktree_hold_the_rest(repo: &TempRepo, before: &Before, unsaved: &[(&str, &[u8])]) {
    assert_eq!(
        repo.git(&["write-tree"]),
        before.index,
        "the index kept the split"
    );
    for hunk in &before.printed {
        let path = hunk.path.as_str();
        if unsaved.iter().any(|(edited, _)| *edited == path) {
            continue;
        }
        let mut staged = repo
            .git(&["cat-file", "blob", &format!("{}:{path}", before.index)])
            .into_bytes();
        staged.push(b'\n');
        assert_eq!(
            repo.read(path),
            staged,
            "{path}: the working tree holds every staged line"
        );
    }
    for (path, bytes) in unsaved {
        assert_eq!(repo.read(path), *bytes, "{path}: the unsaved edit is back");
    }
}

/// Each target's new commit carries its hunks: every added line, and every
/// removed line either removed now or no longer added.
fn each_target_carries_its_hunks(repo: &TempRepo, before: &Before, map: &AbsorbMap) {
    let has = |text: &str, line: &str| text.lines().any(|shown_line| shown_line == line);
    let diff = |from: &str, to: &str, path: &Path| {
        repo.git(&[
            "diff",
            "-U0",
            "--no-color",
            from,
            to,
            "--",
            &path.to_string_lossy(),
        ])
    };
    for (staged, target) in &map.placed {
        let subject = &before.subject_of[target];
        let back = subjects(repo)
            .iter()
            .position(|now| now == subject)
            .expect("the target is still there");
        let now = format!("feature~{back}");
        let shown = diff(&format!("{now}^"), &now, &staged.path);
        let was_shown = diff(&format!("{target}^"), &target.to_string(), &staged.path);
        let printed = before
            .printed
            .iter()
            .find(|hunk| is(hunk, staged))
            .expect("printed");
        for line in &printed.body {
            let carried = match line.strip_prefix('-') {
                Some(text) => {
                    let added = format!("+{text}");
                    has(&shown, line) || (has(&was_shown, &added) && !has(&shown, &added))
                }
                None => has(&shown, line),
            };
            assert!(carried, "{subject} carries {line:?}: {shown}");
        }
    }
}

#[test]
fn absorb_agrees_with_the_trees_one_target() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");

    let map = absorb_agrees_with_the_trees(&repo, &[]);

    assert_eq!(map.placed.len(), 1);
}

#[test]
fn absorb_agrees_with_the_trees_two_targets() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");
    stage(&repo, "a.txt", b"a1\nA2\na3\na4\na5 changed\n");

    let map = absorb_agrees_with_the_trees(&repo, &[]);

    let targets: Vec<Oid> = map.placed.iter().map(|(_, target)| *target).collect();
    assert_eq!(targets.len(), 2);
    assert_ne!(targets[0], targets[1], "two different commits gain");
}

#[test]
fn absorb_agrees_with_the_trees_mixed_placed_and_stays() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");
    stage(&repo, "old.txt", b"o1\no2\nO3\no4\no5\n");
    stage(&repo, "new.txt", b"n1\n");
    stage(&repo, "a.txt", b"a1\na2\na3\nA4\nA5\n");

    let map = absorb_agrees_with_the_trees(&repo, &[]);

    assert_eq!(map.placed.len(), 1);
    assert_eq!(map.stays.len(), 3);
}

#[test]
fn absorb_agrees_with_the_trees_three_targets_one_file_twice() {
    let repo = branch();
    // a2 is f1's, a5 is f3's, b1 is f2's.
    stage(&repo, "a.txt", b"a1\nA2\na3\na4\nA5 changed\n");
    stage(&repo, "b.txt", b"B1\nb2\nb3\nb4\nb5\n");

    let map = absorb_agrees_with_the_trees(&repo, &[]);

    assert_eq!(map.placed.len(), 3);
}

#[test]
fn absorb_agrees_with_the_trees_an_insertion() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nnew\nb3\nb4\nb5\n");

    let map = absorb_agrees_with_the_trees(&repo, &[]);

    assert_eq!(map.placed.len(), 1);
}

#[test]
fn absorb_agrees_with_the_trees_an_unsaved_edit_alongside() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");

    let map =
        absorb_agrees_with_the_trees(&repo, &[("old.txt", b"o1 not saved\no2\no3\no4\no5\n")]);

    assert_eq!(map.placed.len(), 1);
}
