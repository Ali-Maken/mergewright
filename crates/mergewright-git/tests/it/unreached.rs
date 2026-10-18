//! `history_reads::unreached` against real `git` (m7-power-tools/19): which
//! reflog candidates nothing holds any more, folded into runs.
//!
//! The oracle is computed separately from the reader: membership in
//! `git rev-list --branches --tags --remotes` plus every stash entry's own
//! `rev-list`, each run as its own command here.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::process::{Command, Output};
use std::sync::atomic::AtomicBool;

use mergewright_core::oid::Oid;
use mergewright_fixtures::Scratch;
use mergewright_git::history_reads::{LostRun, unreached};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::RunError;

fn open(path: &Path) -> RepoHandle {
    RepoHandle::open(path).unwrap_or_else(|why| panic!("opening {}: {why}", path.display()))
}

/// Runs `git` hermetically in `dir`; asserts nothing about the result.
fn git(dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("LC_ALL", "C")
        .env("GIT_AUTHOR_NAME", "Graft Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Graft Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .output()
        .unwrap_or_else(|why| {
            panic!("running git (mergewright-git does not run without it): {why}")
        })
}

/// [`git`], asserting success and returning stdout.
fn git_ok(dir: &Path, args: &[&str]) -> String {
    let output = git(dir, args);
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap_or_else(|why| panic!("git's output is UTF-8: {why}"))
}

fn ids(text: &str) -> Vec<Oid> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            Oid::from_hex(line.trim()).unwrap_or_else(|why| panic!("{line:?} is not an id: {why}"))
        })
        .collect()
}

fn rev_parse(dir: &Path, revision: &str) -> Oid {
    ids(&git_ok(dir, &["rev-parse", revision]))[0]
}

/// Blank ground to build a repository in (see `history_reads.rs`'s own
/// `scratch` for why a corpus copy is used as the directory).
fn scratch() -> Scratch {
    mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial)
}

/// A fresh repository with one commit on `main`, in its own subdirectory so
/// the corpus copy's own `.git` is never touched.
fn repo_in(scratch: &Scratch) -> std::path::PathBuf {
    let dir = scratch.path().join("repo");
    std::fs::create_dir_all(&dir).unwrap_or_else(|why| panic!("mkdir: {why}"));
    git_ok(&dir, &["init", "-q", "-b", "main"]);
    commit(&dir, "base");
    dir
}

/// A commit changing `f.txt` to `message`, returning its id.
fn commit(dir: &Path, message: &str) -> Oid {
    std::fs::write(dir.join("f.txt"), format!("{message}\n"))
        .unwrap_or_else(|why| panic!("a fixture file writes: {why}"));
    git_ok(dir, &["add", "f.txt"]);
    git_ok(dir, &["commit", "-q", "-m", message]);
    rev_parse(dir, "HEAD")
}

/// Every lost candidate the answer accounts for: each run's tip and the
/// `under` first-parent ancestors below it.
fn covered(dir: &Path, runs: &[LostRun]) -> BTreeSet<Oid> {
    let mut all = BTreeSet::new();
    for run in runs {
        for depth in 0..=run.under {
            all.insert(rev_parse(dir, &format!("{}~{depth}", run.tip)));
        }
    }
    all
}

#[test]
fn unreached_matches_rev_list_all_on_candidates() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    script_every_lost_and_held_form(&dir);

    // Candidates: every commit HEAD's reflog and the branches' name.
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for reflog in ["HEAD", "main", "kept"] {
        for id in ids(&git_ok(&dir, &["log", "-g", "--format=%H", reflog])) {
            if seen.insert(id) {
                candidates.push(id);
            }
        }
    }

    // The oracle, computed apart from the reader.
    let mut held: HashSet<Oid> = ids(&git_ok(
        &dir,
        &["rev-list", "--branches", "--tags", "--remotes"],
    ))
    .into_iter()
    .collect();
    for entry in ids(&git_ok(&dir, &["log", "-g", "--format=%H", "refs/stash"])) {
        held.extend(ids(&git_ok(&dir, &["rev-list", &entry.to_string()])));
    }
    let expected: BTreeSet<Oid> = candidates
        .iter()
        .copied()
        .filter(|id| !held.contains(id))
        .collect();
    assert!(
        expected.len() >= 5,
        "the script makes at least five lost candidates, got {}",
        expected.len()
    );

    let got = unreached(&open(&dir), &candidates, usize::MAX, &()).expect("unreached reads");
    assert!(!got.capped);
    assert_eq!(got.checked, candidates.len());
    for run in &got.lost {
        assert!(expected.contains(&run.tip), "{} is held", run.tip);
    }
    assert_eq!(covered(&dir, &got.lost), expected);
}

/// Each lost form (shell reset, deleted branch, amended tip) and each held
/// one (branch, annotated tag, remote-tracking ref, stash).
fn script_every_lost_and_held_form(dir: &Path) {
    let dir = dir.to_path_buf();
    // A shell reset: two commits dropped off main.
    commit(&dir, "kept 1");
    commit(&dir, "reset away 1");
    commit(&dir, "reset away 2");
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~2"]);
    // A deleted branch with two commits of its own.
    git_ok(&dir, &["checkout", "-q", "-b", "doomed"]);
    commit(&dir, "doomed 1");
    commit(&dir, "doomed 2");
    git_ok(&dir, &["checkout", "-q", "main"]);
    git_ok(&dir, &["branch", "-q", "-D", "doomed"]);
    // An amended tip.
    commit(&dir, "before amend");
    git_ok(&dir, &["commit", "-q", "--amend", "-m", "after amend"]);
    // Held forms: a branch, a tag, a remote-tracking ref, a stash.
    git_ok(&dir, &["checkout", "-q", "-b", "kept"]);
    commit(&dir, "on kept");
    git_ok(&dir, &["checkout", "-q", "main"]);
    commit(&dir, "tagged");
    git_ok(&dir, &["tag", "-a", "-m", "t", "v1"]);
    commit(&dir, "remote only");
    let remote_only = rev_parse(&dir, "HEAD");
    git_ok(
        &dir,
        &[
            "update-ref",
            "refs/remotes/origin/x",
            &remote_only.to_string(),
        ],
    );
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~2"]);
    std::fs::write(dir.join("f.txt"), "stashed\n").unwrap_or_else(|why| panic!("write: {why}"));
    git_ok(&dir, &["stash", "-q"]);
    commit(&dir, "last");
}

/// A commit only one stash entry reaches: dropped from `main` by a reset,
/// but the stash made on top of it keeps it.
fn stash_on_a_dropped_commit(dir: &Path) -> Oid {
    let dropped = commit(dir, "dropped");
    std::fs::write(dir.join("f.txt"), "dirty\n").unwrap_or_else(|why| panic!("write: {why}"));
    git_ok(dir, &["stash", "-q"]);
    git_ok(dir, &["reset", "-q", "--hard", "HEAD~1"]);
    dropped
}

#[test]
fn a_commit_only_a_stash_holds_is_held() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let dropped = stash_on_a_dropped_commit(&dir);

    let got = unreached(&open(&dir), &[dropped], 10, &()).expect("unreached reads");
    assert_eq!(got.lost, vec![]);
    assert_eq!(got.checked, 1);
}

#[test]
fn an_older_stash_entry_holds_too() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let dropped = stash_on_a_dropped_commit(&dir);
    // A newer stash, on a different commit: `dropped` is now `stash@{1}`'s.
    std::fs::write(dir.join("f.txt"), "dirty again\n").unwrap_or_else(|why| panic!("write: {why}"));
    git_ok(&dir, &["stash", "-q"]);
    let entries =
        ids(&git_ok(&dir, &["log", "-g", "--format=%P", "refs/stash"]).replace(' ', "\n"));
    assert!(entries.contains(&dropped), "stash@{{1}} sits on it");

    let got = unreached(&open(&dir), &[dropped], 10, &()).expect("unreached reads");
    assert_eq!(got.lost, vec![]);
}

#[test]
fn a_commit_only_refs_graft_holds_is_lost() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let backed_up = commit(&dir, "backed up");
    git_ok(
        &dir,
        &["update-ref", "refs/graft/backup/1", &backed_up.to_string()],
    );
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~1"]);

    let got = unreached(&open(&dir), &[backed_up], 10, &()).expect("unreached reads");
    assert_eq!(
        got.lost,
        vec![LostRun {
            tip: backed_up,
            under: 0
        }]
    );
}

#[test]
fn a_detached_head_holds() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    git_ok(&dir, &["checkout", "-q", "--detach"]);
    let detached = commit(&dir, "on a detached head");

    let got = unreached(&open(&dir), &[detached], 10, &()).expect("unreached reads");
    assert_eq!(got.lost, vec![]);
}

#[test]
fn a_tag_holds_through_peeling() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let tagged = commit(&dir, "tagged");
    git_ok(&dir, &["tag", "-a", "-m", "annotated", "v1"]);
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~1"]);
    // The tag is packed, so it is read the way a gc'd repository keeps it.
    git_ok(&dir, &["pack-refs", "--all"]);

    let got = unreached(&open(&dir), &[tagged], 10, &()).expect("unreached reads");
    assert_eq!(got.lost, vec![]);
}

#[test]
fn lost_ancestors_fold_into_one_run_with_under() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let first = commit(&dir, "lost 1");
    let second = commit(&dir, "lost 2");
    let third = commit(&dir, "lost 3");
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~3"]);
    let alone = commit(&dir, "lost alone");
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~1"]);

    // Candidates newest first, with an ancestor ahead of its descendant:
    // the fold must not depend on the order.
    let got =
        unreached(&open(&dir), &[alone, first, third, second], 10, &()).expect("unreached reads");
    assert_eq!(
        got.lost,
        vec![
            LostRun {
                tip: alone,
                under: 0
            },
            LostRun {
                tip: third,
                under: 2
            },
        ]
    );
}

#[test]
fn cap_checks_only_the_newest_and_says_capped() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let older = commit(&dir, "older");
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~1"]);
    let newer = commit(&dir, "newer");
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~1"]);

    let got = unreached(&open(&dir), &[newer, older], 1, &()).expect("unreached reads");
    assert!(got.capped);
    assert_eq!(got.checked, 1);
    assert_eq!(
        got.lost,
        vec![LostRun {
            tip: newer,
            under: 0
        }]
    );

    let all = unreached(&open(&dir), &[newer, older], 2, &()).expect("unreached reads");
    assert!(!all.capped);
    assert_eq!(all.checked, 2);
}

#[test]
fn unreached_honours_stop() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let lost = commit(&dir, "lost");
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~1"]);

    let stop = AtomicBool::new(true);
    let got = unreached(&open(&dir), &[lost], 10, &stop);
    assert!(matches!(got, Err(RunError::Stopped)), "{got:?}");
}

#[test]
fn a_candidate_git_no_longer_has_is_not_reported() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let gone = Oid::from_hex("0123456789abcdef0123456789abcdef01234567")
        .unwrap_or_else(|why| panic!("a valid id: {why}"));
    let lost = commit(&dir, "lost");
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~1"]);

    let got = unreached(&open(&dir), &[gone, lost], 10, &()).expect("unreached reads");
    assert_eq!(
        got.lost,
        vec![LostRun {
            tip: lost,
            under: 0
        }]
    );
    assert_eq!(got.checked, 2);
}

#[test]
fn unreached_on_an_unborn_head() {
    let scratch = scratch();
    let dir = repo_in(&scratch);
    let lost = commit(&dir, "lost");
    git_ok(&dir, &["reset", "-q", "--hard", "HEAD~1"]);
    git_ok(&dir, &["checkout", "-q", "--orphan", "fresh"]);

    let got = unreached(&open(&dir), &[lost], 10, &()).expect("unreached reads");
    assert_eq!(
        got.lost,
        vec![LostRun {
            tip: lost,
            under: 0
        }]
    );
}
