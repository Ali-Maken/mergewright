//! Absorb's oracle, its independent reads: the staged hunks as porcelain
//! `git diff` prints them, and each hunk's target by `git log -L` — never
//! the code under test's plumbing diff or blame (review m7-06 M8).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

use graft_core::oid::Oid;
use graft_ops::absorb::{AbsorbMap, StagedHunk, StaysWhy};

use crate::support::TempRepo;

/// One staged hunk as `git diff --cached -U0` prints it.
pub(crate) struct PrintedHunk {
    pub(crate) path: String,
    pub(crate) body: Vec<String>,
    pub(crate) old_start: u32,
    pub(crate) old_lines: u32,
}

/// The staged diff, cut into hunks — read with porcelain `git diff`, not
/// the code under test's plumbing read.
pub(crate) fn staged_hunks(repo: &TempRepo) -> Vec<PrintedHunk> {
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

pub(crate) fn is(hunk: &PrintedHunk, staged: &StagedHunk) -> bool {
    Path::new(&hunk.path) == staged.path
        && hunk.old_start == staged.old.start
        && hunk.old_start + hunk.old_lines == staged.old.end
}

// ── The independent target: `git log -L`, not the code's blame ──────────

/// The commit in `main..feature` that last changed `line` of `path` at
/// `HEAD`, by `git log -L` — git's line-log, a different walk from the
/// blame the code under test reads. `None`: no commit on the branch did.
fn last_to_change_line(repo: &TempRepo, path: &str, line: u32) -> Option<Oid> {
    let range = format!("{line},{line}:{path}");
    let shown = repo.git(&["log", "-1", "--format=%H", "-L", &range, "main..feature"]);
    shown
        .lines()
        .next()
        .filter(|first| !first.is_empty())
        .map(|hex| Oid::from_hex(hex).expect("a commit id"))
}

/// Where the ticket's rule puts `hunk`, computed from `git log -L` alone:
/// its old lines (an insertion: the line above and the line below) all last
/// changed by one commit on the branch → that commit; else nowhere.
fn expected_target(repo: &TempRepo, hunk: &PrintedHunk) -> Option<Oid> {
    let exists = repo
        .try_git(&["cat-file", "-e", &format!("HEAD:{}", hunk.path)])
        .is_ok();
    if !exists {
        return None; // a new file
    }
    let count = u32::try_from(
        repo.git(&["cat-file", "blob", &format!("HEAD:{}", hunk.path)])
            .lines()
            .count(),
    )
    .expect("a small file");
    let lines: Vec<u32> = if hunk.old_lines == 0 {
        vec![hunk.old_start, hunk.old_start + 1]
    } else {
        (hunk.old_start..hunk.old_start + hunk.old_lines).collect()
    };
    if lines.iter().any(|line| !(1..=count).contains(line)) {
        return None; // an insertion at either end
    }
    let mut commits = lines
        .iter()
        .map(|&line| last_to_change_line(repo, &hunk.path, line));
    let first = commits.next().flatten()?;
    commits.all(|other| other == Some(first)).then_some(first)
}

/// Each placed hunk's target is the one `git log -L` names; a hunk it names
/// no target for is never placed; one it names a target for and the map
/// keeps must have failed the commutation check (`TwoCommitsChangedIt`).
pub(crate) fn each_target_last_changed_its_lines(
    repo: &TempRepo,
    printed: &[PrintedHunk],
    map: &AbsorbMap,
) {
    for printed in printed {
        let expected = expected_target(repo, printed);
        let placed = map
            .placed
            .iter()
            .find(|(staged, _)| is(printed, staged))
            .map(|(_, target)| *target);
        let stays = map.stays.iter().find(|(staged, _)| is(printed, staged));
        match (expected, placed) {
            (Some(want), Some(got)) => assert_eq!(got, want, "{}: the target", printed.path),
            (None, Some(got)) => panic!("{}: placed into {got}, but no one commit", printed.path),
            (Some(want), None) => assert!(
                matches!(stays, Some((_, StaysWhy::TwoCommitsChangedIt))),
                "{}: {want} changed it, yet it stays as {stays:?}",
                printed.path
            ),
            (None, None) => {}
        }
    }
}
