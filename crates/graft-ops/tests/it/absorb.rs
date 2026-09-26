//! Absorb (M7 absorb-map): each staged hunk goes to the commit on his branch
//! that last changed its lines, and one that fits none stays staged, with
//! the reason. Then the oracle: plan and apply, and judge the result by the
//! trees git holds afterwards.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use graft_core::oid::Oid;
use graft_git::run::Stop;
use graft_ops::absorb::{AbsorbMap, StagedHunk, StaysWhy, absorb_map};
use graft_ops::facts::{FactsError, read_facts};
use graft_ops::operation::{FactsScope, Operation, Refusal, Tip};
use graft_ops::rewrite::{RewriteError, RewriteParams, RewriteRequest, TodoAction, plan};

use crate::support::{NOW, Never, TempRepo, git_version, run_undo, state_of, try_run};

// ── The branch every test stands on ─────────────────────────────────────

/// `main`: `m1` (`old.txt`). `feature`, `HEAD`: `f1` makes `a.txt`, `f2`
/// makes `b.txt`, `f3` changes `a.txt`'s last line.
fn branch() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("old.txt", b"o1\no2\no3\no4\no5\n");
    repo.commit_all("m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("a.txt", b"a1\na2\na3\na4\na5\n");
    repo.commit_all("f1");
    repo.write("b.txt", b"b1\nb2\nb3\nb4\nb5\n");
    repo.commit_all("f2");
    repo.write("a.txt", b"a1\na2\na3\na4\na5 changed\n");
    repo.commit_all("f3");
    repo
}

fn stage(repo: &TempRepo, path: &str, bytes: &[u8]) {
    repo.write(path, bytes);
    repo.git(&["add", "--", path]);
}

fn oid(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a commit id")
}

fn tip(repo: &TempRepo) -> Tip {
    Tip {
        name: "refs/heads/feature".to_owned(),
        oid: oid(repo, "feature"),
    }
}

fn map_with(repo: &TempRepo, stop: &dyn Stop) -> Result<AbsorbMap, FactsError> {
    absorb_map(
        &repo.open(),
        &git_version(),
        &tip(repo),
        oid(repo, "main"),
        stop,
    )
}

fn map(repo: &TempRepo) -> AbsorbMap {
    map_with(repo, &Never).unwrap_or_else(|why| panic!("the map reads: {why}"))
}

fn hunk(path: &str, old: std::ops::Range<u32>, new: std::ops::Range<u32>) -> StagedHunk {
    StagedHunk {
        path: PathBuf::from(path),
        old,
        new,
    }
}

fn planned(repo: &TempRepo) -> Result<RewriteParams, RewriteError> {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    plan(
        &handle,
        &git_version(),
        &RewriteRequest::Absorb,
        &facts,
        None,
        &Never,
    )
}

struct Always;

impl Stop for Always {
    fn should_stop(&self) -> bool {
        true
    }
}

// ── Placing each hunk ───────────────────────────────────────────────────

#[test]
fn a_hunk_goes_to_the_commit_that_last_changed_its_lines() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");

    let map = map(&repo);

    assert_eq!(
        map.placed,
        vec![(hunk("b.txt", 3..4, 3..4), oid(&repo, "feature~1"))]
    );
    assert_eq!(map.stays, vec![]);
}

#[test]
fn an_insertion_between_one_commits_lines_goes_there() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nnew\nb3\nb4\nb5\n");

    let map = map(&repo);

    // git's header for it: `@@ -2,0 +3 @@`, after line 2.
    assert_eq!(
        map.placed,
        vec![(hunk("b.txt", 2..2, 3..4), oid(&repo, "feature~1"))]
    );
    assert_eq!(map.stays, vec![]);
}

#[test]
fn an_insertion_at_the_top_of_a_file_stays() {
    // No line above it: the rule needs both neighbours from one commit.
    let repo = branch();
    stage(&repo, "b.txt", b"new\nb1\nb2\nb3\nb4\nb5\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![]);
    assert_eq!(
        map.stays,
        vec![(hunk("b.txt", 0..0, 1..2), StaysWhy::NoCommitChangedIt)]
    );
}

#[test]
fn a_hunk_over_two_commits_stays() {
    let repo = branch();
    // a4 is f1's, a5 is f3's.
    stage(&repo, "a.txt", b"a1\na2\na3\nA4\nA5\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![]);
    assert_eq!(
        map.stays,
        vec![(hunk("a.txt", 4..6, 4..6), StaysWhy::TwoCommitsChangedIt)]
    );
}

#[test]
fn a_hunk_on_lines_older_than_the_branch_stays() {
    let repo = branch();
    stage(&repo, "old.txt", b"o1\no2\nO3\no4\no5\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![]);
    assert_eq!(
        map.stays,
        vec![(hunk("old.txt", 3..4, 3..4), StaysWhy::BeforeYourBranch)]
    );
}

#[test]
fn a_new_file_stays() {
    let repo = branch();
    stage(&repo, "new.txt", b"n1\nn2\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![]);
    assert_eq!(
        map.stays,
        vec![(hunk("new.txt", 0..0, 1..3), StaysWhy::NewFile)]
    );
}

#[test]
fn a_hunk_that_does_not_commute_stays() {
    // `d1` is f1's, but f2 changed the line right below it: put into f1,
    // the change would meet f2's on replay.
    let repo = TempRepo::init();
    repo.write("old.txt", b"o\n");
    repo.commit_all("m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("d.txt", b"d1\nd2\nd3\n");
    repo.commit_all("f1");
    repo.write("d.txt", b"d1\nd2 later\nd3\n");
    repo.commit_all("f2");
    stage(&repo, "d.txt", b"D1\nd2 later\nd3\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![]);
    assert_eq!(
        map.stays,
        vec![(hunk("d.txt", 1..2, 1..2), StaysWhy::TwoCommitsChangedIt)]
    );
}

#[test]
fn absorb_with_nothing_placed_is_refused() {
    let repo = branch();
    stage(&repo, "old.txt", b"o1\no2\nO3\no4\no5\n");

    let refused = planned(&repo);

    let Err(RewriteError::Refused(refusal)) = refused else {
        panic!("refused, got {refused:?}");
    };
    assert_eq!(refusal, Refusal::NothingToAbsorb);
    assert_eq!(
        refusal.explain(),
        "none of your staged lines belong to a commit on your branch · c commits them"
    );
}

#[test]
fn absorb_map_writes_no_ref_index_or_file() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");
    stage(&repo, "a.txt", b"a1\nA2\na3\na4\na5 changed\n");
    repo.write("old.txt", b"unsaved\n");
    let before = state_of(&repo);
    let git_files_before = git_dir_files(&repo);

    map(&repo);
    planned(&repo).expect("it plans");

    assert_eq!(state_of(&repo), before);
    assert_eq!(
        git_dir_files(&repo),
        git_files_before,
        "no file left in .git"
    );
}

#[test]
fn absorb_map_honours_stop() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");

    let stopped = map_with(&repo, &Always);

    assert!(matches!(stopped, Err(FactsError::Stopped)), "{stopped:?}");
}

#[test]
fn undo_after_absorb_restores_index_and_commits() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");
    stage(&repo, "old.txt", b"o1\no2\nO3\no4\no5\n");
    let before = state_of(&repo);

    let params = planned(&repo).expect("it plans");
    try_run(&repo, &Operation::Rewrite(params), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_ne!(state_of(&repo), before, "the absorb changed something");
    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{why:?}"));

    assert_eq!(state_of(&repo), before);
}

/// Every file under `.git` except the ones any git command rewrites
/// (`logs/`, the journal's directory is Graft's and not touched by a read).
fn git_dir_files(repo: &TempRepo) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, found: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("the directory reads") {
            let path = entry.expect("the entry reads").path();
            let relative = path
                .strip_prefix(root)
                .expect("under the root")
                .to_string_lossy()
                .into_owned();
            // New objects are allowed: the plan writes the commits it will
            // put in (as `a` into an older commit does).
            if relative.starts_with("objects") {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, found);
            } else {
                found.push(relative);
            }
        }
    }
    let root = repo.path().join(".git");
    let mut found = Vec::new();
    walk(&root, &root, &mut found);
    found.sort();
    found
}

// ── The oracle ──────────────────────────────────────────────────────────

/// One staged hunk as `git diff --cached -U0` prints it.
struct PrintedHunk {
    path: String,
    body: Vec<String>,
    old_start: u32,
    old_lines: u32,
}

/// The staged diff, cut into hunks — read with porcelain `git diff`, not
/// the code under test's plumbing read.
fn staged_hunks(repo: &TempRepo) -> Vec<PrintedHunk> {
    let text = repo.git(&["diff", "--cached", "-U0", "--no-color", "--no-renames"]);
    let mut hunks: Vec<PrintedHunk> = Vec::new();
    let mut path = String::new();
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("+++ b/") {
            name.clone_into(&mut path);
        } else if line.starts_with("@@ ") {
            let old = line
                .split_whitespace()
                .nth(1)
                .and_then(|range| range.strip_prefix('-'))
                .expect("an old range");
            let (start, count) = match old.split_once(',') {
                Some((start, count)) => (start.parse().unwrap(), count.parse().unwrap()),
                None => (old.parse().unwrap(), 1),
            };
            hunks.push(PrintedHunk {
                path: path.clone(),
                body: Vec::new(),
                old_start: start,
                old_lines: count,
            });
        } else if line.starts_with('+') || line.starts_with('-') || line.starts_with('\\') {
            if line.starts_with("+++") || line.starts_with("---") {
                continue;
            }
            if let Some(last) = hunks.last_mut() {
                last.body.push(line.to_owned());
            }
        }
    }
    hunks
}

fn is(hunk: &PrintedHunk, staged: &StagedHunk) -> bool {
    Path::new(&hunk.path) == staged.path
        && hunk.old_start == staged.old.start
        && hunk.old_start + hunk.old_lines == staged.old.end
}

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
        .unwrap_or_else(|why| panic!("{}", why.explain()));

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
