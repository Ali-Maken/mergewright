//! Tests for `list_files`/`file_diff` against real git, on corpus fixtures
//! and a few hand-built repositories for cases no shape carries (a big
//! truncated diff, a whitespace-only change, a rename).

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use graft_core::oid::Oid;
use graft_fixtures::Shape;

use super::*;
use crate::fixture::Fixture;
use crate::repo::{RepoHandle, SharedRepo};

fn oid_of(repo_path: &Path, rev: &str) -> Oid {
    let stdout =
        graft_fixtures::git::run(repo_path, &["rev-parse", rev]).expect("git rev-parse runs");
    let text = String::from_utf8(stdout).expect("git prints hex ids as ascii");
    Oid::from_hex(text.trim()).expect("git prints a full object id")
}

fn write_file(fixture: &Fixture, name: &str, content: &str) {
    std::fs::write(fixture.root().join(name), content).expect("a fixture file can be written");
}

/// `git show --numstat`'s own view of one commit against its first parent
/// (unlike `diff-tree` with a single revision, `show` diffs an ordinary
/// merge commit against its first parent instead of printing nothing), as
/// the independent oracle `list_files` is checked against. `show` also
/// prints a commit header and message before the numstat lines, so only
/// lines shaped like a numstat record (`<count-or-dash>\t<count-or-dash>\t
/// <path>`) are kept.
fn numstat_oracle(repo_path: &Path, rev: &str) -> (BTreeSet<Vec<u8>>, u64, u64) {
    let stdout = graft_fixtures::git::run(
        repo_path,
        &[
            "show",
            "--no-color",
            "--no-ext-diff",
            "-M",
            "-C",
            "--numstat",
            rev,
        ],
    )
    .expect("git show runs");
    let text = String::from_utf8(stdout).expect("this fixture's paths are ascii");
    let mut paths = BTreeSet::new();
    let (mut added, mut removed) = (0u64, 0u64);
    for line in text.lines() {
        let mut fields = line.splitn(3, '\t');
        let (Some(a), Some(r), Some(path)) = (fields.next(), fields.next(), fields.next()) else {
            continue;
        };
        let is_count = |field: &str| field == "-" || field.parse::<u64>().is_ok();
        if !is_count(a) || !is_count(r) {
            continue;
        }
        if a != "-" {
            added += a.parse::<u64>().expect("a decimal added count");
        }
        if r != "-" {
            removed += r.parse::<u64>().expect("a decimal removed count");
        }
        paths.insert(path.as_bytes().to_vec());
    }
    (paths, added, removed)
}

#[test]
fn list_files_commit_head_matches_git_oracle_on_feature_branches() {
    let path = graft_fixtures::borrow(Shape::FeatureBranches);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let tip = oid_of(&path, "main");

    let list = list_files(
        &handle,
        &DiffSpec::Commit {
            commit: tip,
            against: None,
        },
        &DiffOptions::default(),
        &(),
    )
    .expect("list_files succeeds");
    let (oracle_paths, oracle_added, oracle_removed) = numstat_oracle(&path, "main");

    let paths: BTreeSet<Vec<u8>> = list.files.iter().map(|entry| entry.path.clone()).collect();
    assert_eq!(paths, oracle_paths);
    assert_eq!(list.added, oracle_added);
    assert_eq!(list.removed, oracle_removed);
}

#[test]
fn octopus_merge_lists_a_different_set_against_each_parent() {
    let path = graft_fixtures::borrow(Shape::OctopusMerge);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let tip = oid_of(&path, "main");

    let stdout = graft_fixtures::git::run(&path, &["rev-list", "--parents", "-1", "main"])
        .expect("git rev-list runs");
    let text = String::from_utf8(stdout).expect("ids are ascii");
    let ids: Vec<&str> = text.trim().split(' ').collect();
    assert_eq!(ids.len(), 5, "the octopus tip has four parents");
    let second_parent = Oid::from_hex(ids[2]).expect("a full object id");

    let against_first = list_files(
        &handle,
        &DiffSpec::Commit {
            commit: tip,
            against: None,
        },
        &DiffOptions::default(),
        &(),
    )
    .expect("list_files succeeds");
    let against_second = list_files(
        &handle,
        &DiffSpec::Commit {
            commit: tip,
            against: Some(second_parent),
        },
        &DiffOptions::default(),
        &(),
    )
    .expect("list_files succeeds");

    let first_paths: BTreeSet<Vec<u8>> =
        against_first.files.iter().map(|e| e.path.clone()).collect();
    let second_paths: BTreeSet<Vec<u8>> = against_second
        .files
        .iter()
        .map(|e| e.path.clone())
        .collect();
    assert_ne!(first_paths, second_paths);
    assert_eq!(
        first_paths,
        BTreeSet::from([b"t1.txt".to_vec(), b"t2.txt".to_vec(), b"t3.txt".to_vec()])
    );
    assert_eq!(
        second_paths,
        BTreeSet::from([b"t2.txt".to_vec(), b"t3.txt".to_vec()])
    );
}

#[test]
fn range_root_to_tip_lists_every_file_the_tip_has() {
    let path = graft_fixtures::borrow(Shape::Linear);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let root = oid_of(&path, "main~199");
    let tip = oid_of(&path, "main");

    let list = list_files(
        &handle,
        &DiffSpec::Range {
            base: root,
            head: tip,
        },
        &DiffOptions::default(),
        &(),
    )
    .expect("list_files succeeds");

    assert_eq!(list.files.len(), 1);
    assert_eq!(list.files[0].path, b"log.txt");
}

#[test]
fn working_tree_vs_head_reports_modified_added_and_untracked() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Linear);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();

    std::fs::write(scratch.path().join("log.txt"), "modified\n")
        .expect("overwrite the tracked file");
    std::fs::write(scratch.path().join("staged.txt"), "new\n").expect("write a new file");
    graft_fixtures::git::run(scratch.path(), &["add", "staged.txt"])
        .expect("git add stages the new file");
    std::fs::write(scratch.path().join("untracked.txt"), "u\n").expect("write an untracked file");

    let list = list_files(
        &handle,
        &DiffSpec::WorkingTreeVsHead,
        &DiffOptions::default(),
        &(),
    )
    .expect("list_files succeeds");

    let mut statuses: Vec<(Vec<u8>, FileStatus)> = list
        .files
        .iter()
        .map(|e| (e.path.clone(), e.status))
        .collect();
    statuses.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        statuses,
        vec![
            (b"log.txt".to_vec(), FileStatus::Modified),
            (b"staged.txt".to_vec(), FileStatus::Added),
            (b"untracked.txt".to_vec(), FileStatus::Untracked),
        ]
    );
}

#[test]
fn truncation_keeps_whole_hunks_and_reports_totals() {
    let fixture = Fixture::empty_repo();
    let mut lines: Vec<String> = (1..=40).map(|n| format!("line {n:02}\n")).collect();
    write_file(&fixture, "big.txt", &lines.concat());
    fixture.git(&["add", "-A"]);
    fixture.commit("base");

    for &n in &[2usize, 20, 38] {
        lines[n - 1] = format!("line {n:02} changed\n");
    }
    write_file(&fixture, "big.txt", &lines.concat());
    fixture.git(&["add", "-A"]);
    fixture.commit("edit three spots");

    let shared = SharedRepo::open(fixture.root()).expect("the fixture opens");
    let handle = shared.handle();
    let tip = oid_of(fixture.root(), "HEAD");
    let entry = FileEntry {
        path: b"big.txt".to_vec(),
        old_path: None,
        status: FileStatus::Modified,
        added: Some(3),
        removed: Some(3),
    };
    let spec = DiffSpec::Commit {
        commit: tip,
        against: None,
    };

    let full = file_diff(
        &handle,
        &spec,
        &entry,
        &DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff succeeds");
    assert!(full.truncated.is_none());
    assert!(
        full.hunks.len() >= 2,
        "the three edits are far enough apart to form separate hunks"
    );
    let total_lines: usize = full.hunks.iter().map(|h| h.lines.len()).sum();

    let cut_after = full.hunks[0].lines.len();
    let capped = file_diff(
        &handle,
        &spec,
        &entry,
        &DiffOptions::default(),
        cut_after,
        &(),
    )
    .expect("file_diff succeeds");

    assert_eq!(capped.hunks.len(), 1, "truncation ends on a hunk boundary");
    assert_eq!(capped.hunks[0], full.hunks[0]);
    let truncated = capped
        .truncated
        .expect("more hunks remained than the cap allowed");
    assert_eq!(truncated.shown_lines, cut_after);
    assert_eq!(truncated.total_lines, total_lines);
}

#[test]
fn context_option_changes_hunk_line_counts_like_git_dash_u() {
    let fixture = Fixture::empty_repo();
    let lines: Vec<String> = (1..=30).map(|n| format!("line {n:02}\n")).collect();
    write_file(&fixture, "ctx.txt", &lines.concat());
    fixture.git(&["add", "-A"]);
    fixture.commit("base");

    let mut edited = lines.clone();
    edited[15] = "line 16 changed\n".to_string();
    write_file(&fixture, "ctx.txt", &edited.concat());
    fixture.git(&["add", "-A"]);
    fixture.commit("edit line 16");

    let shared = SharedRepo::open(fixture.root()).expect("the fixture opens");
    let handle = shared.handle();
    let tip = oid_of(fixture.root(), "HEAD");
    let entry = FileEntry {
        path: b"ctx.txt".to_vec(),
        old_path: None,
        status: FileStatus::Modified,
        added: Some(1),
        removed: Some(1),
    };
    let spec = DiffSpec::Commit {
        commit: tip,
        against: None,
    };

    let zero = file_diff(
        &handle,
        &spec,
        &entry,
        &DiffOptions {
            context: 0,
            ignore_whitespace: false,
        },
        usize::MAX,
        &(),
    )
    .expect("file_diff succeeds");
    let eight = file_diff(
        &handle,
        &spec,
        &entry,
        &DiffOptions {
            context: 8,
            ignore_whitespace: false,
        },
        usize::MAX,
        &(),
    )
    .expect("file_diff succeeds");

    assert_eq!(zero.hunks.len(), 1);
    assert_eq!(eight.hunks.len(), 1);
    assert_eq!(
        zero.hunks[0].lines.len(),
        2,
        "no context: just the two changed lines"
    );
    assert_eq!(
        eight.hunks[0].lines.len(),
        2 + 16,
        "8 lines of context on each side"
    );
}

#[test]
fn ignore_whitespace_on_a_whitespace_only_change_gives_no_hunks() {
    let fixture = Fixture::empty_repo();
    write_file(&fixture, "ws.txt", "alpha\nbeta\ngamma\n");
    fixture.git(&["add", "-A"]);
    fixture.commit("base");
    write_file(&fixture, "ws.txt", "alpha\nbeta  \ngamma\n");
    fixture.git(&["add", "-A"]);
    fixture.commit("trailing whitespace only");

    let shared = SharedRepo::open(fixture.root()).expect("the fixture opens");
    let handle = shared.handle();
    let tip = oid_of(fixture.root(), "HEAD");
    let entry = FileEntry {
        path: b"ws.txt".to_vec(),
        old_path: None,
        status: FileStatus::Modified,
        added: Some(1),
        removed: Some(1),
    };
    let spec = DiffSpec::Commit {
        commit: tip,
        against: None,
    };

    let normal = file_diff(
        &handle,
        &spec,
        &entry,
        &DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff succeeds");
    assert!(!normal.hunks.is_empty());

    let ignoring = file_diff(
        &handle,
        &spec,
        &entry,
        &DiffOptions {
            context: 3,
            ignore_whitespace: true,
        },
        usize::MAX,
        &(),
    )
    .expect("file_diff succeeds");
    assert!(ignoring.hunks.is_empty());
}

#[test]
fn renamed_file_carries_old_path_and_file_diff_parses_both_sides() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Linear);
    graft_fixtures::git::run(scratch.path(), &["mv", "log.txt", "renamed.txt"])
        .expect("git mv renames the file");
    graft_fixtures::git::run(scratch.path(), &["commit", "-m", "rename log.txt"])
        .expect("git commit runs");

    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();
    let tip = oid_of(scratch.path(), "HEAD");
    let spec = DiffSpec::Commit {
        commit: tip,
        against: None,
    };

    let list =
        list_files(&handle, &spec, &DiffOptions::default(), &()).expect("list_files succeeds");
    let entry = list
        .files
        .iter()
        .find(|e| e.path == b"renamed.txt")
        .expect("the renamed file is listed");
    assert_eq!(entry.old_path, Some(b"log.txt".to_vec()));
    assert!(matches!(entry.status, FileStatus::Renamed { .. }));

    let diff = file_diff(
        &handle,
        &spec,
        entry,
        &DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff parses a rename's two paths");
    assert_eq!(diff.entry.path, b"renamed.txt");
}

#[test]
fn stopped_before_running_gives_run_stopped_with_no_parse_attempt() {
    let path = graft_fixtures::borrow(Shape::Trivial);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let tip = oid_of(&path, "main");
    let stop = AtomicBool::new(true);

    let result = list_files(
        &handle,
        &DiffSpec::Commit {
            commit: tip,
            against: None,
        },
        &DiffOptions::default(),
        &stop,
    );
    assert!(matches!(result, Err(DiffError::Run(RunError::Stopped))));
}

#[test]
fn non_utf8_path_round_trips_through_git_arg_path() {
    let path = graft_fixtures::borrow(Shape::NonUtf8Path);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let tip = oid_of(&path, "refs/heads/latin1");
    let spec = DiffSpec::Commit {
        commit: tip,
        against: None,
    };

    let list =
        list_files(&handle, &spec, &DiffOptions::default(), &()).expect("list_files succeeds");
    let entry = list.files.first().expect("the commit adds one file");
    assert_eq!(entry.path, b"caf\xe9.txt");

    let diff = file_diff(
        &handle,
        &spec,
        entry,
        &DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff parses the non-UTF-8 path");
    assert_eq!(diff.entry.path, b"caf\xe9.txt");
    assert!(!diff.hunks.is_empty());
}

/// `diff-index` is plumbing: it trusts the index's stat cache and never
/// refreshes it, so a tracked file whose mtime moved without its content
/// changing — a branch switch, an editor save, a build — came back as `M`
/// with a `0 0` numstat while the pane's header (from `git status`, which
/// does refresh) said nothing was unstaged. The plumbing assertion is the
/// point of the test: without it the setup could silently stop being
/// stat-dirty and the test would pass for the wrong reason.
#[test]
fn a_touched_but_unchanged_file_is_not_listed_as_modified() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Linear);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();

    let tracked = scratch.path().join("log.txt");
    let content = std::fs::read(&tracked).expect("the tracked file reads");
    std::fs::write(&tracked, &content).expect("the same bytes go back with a new mtime");

    let plumbing = graft_fixtures::git::run(scratch.path(), &["diff-index", "--raw", "HEAD"])
        .expect("git diff-index runs");
    assert!(
        !plumbing.is_empty(),
        "the file is not stat-dirty, so this test would prove nothing"
    );

    let list = list_files(
        &handle,
        &DiffSpec::WorkingTreeVsHead,
        &DiffOptions::default(),
        &(),
    )
    .expect("list_files succeeds");
    let listed: Vec<String> = list
        .files
        .iter()
        .map(|e| String::from_utf8_lossy(&e.path).into_owned())
        .collect();
    assert!(
        list.files.is_empty(),
        "nothing changed, yet the pane would list {listed:?}"
    );
}

/// M2 is a read-only milestone: listing the working tree must leave
/// `.git/index` byte for byte as it was. Porcelain `git diff` does not —
/// it refreshes the stat cache and writes the index, taking `index.lock`
/// on the way, which can make the user's own `git commit` fail — and
/// neither `GIT_OPTIONAL_LOCKS=0` nor `--no-optional-locks` stops it
/// (only `git status` honours those). The index bytes are the assertion
/// because they are the thing that must not change; the stat-dirty setup
/// is what makes a refresh tempting in the first place.
#[test]
fn listing_the_working_tree_does_not_write_the_index() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Linear);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();

    let tracked = scratch.path().join("log.txt");
    let content = std::fs::read(&tracked).expect("the tracked file reads");
    std::fs::write(&tracked, &content).expect("the same bytes go back with a new mtime");

    let plumbing = graft_fixtures::git::run(scratch.path(), &["diff-index", "--raw", "HEAD"])
        .expect("git diff-index runs");
    assert!(
        !plumbing.is_empty(),
        "the file is not stat-dirty, so nothing would want to refresh the index"
    );

    let index = scratch.path().join(".git").join("index");
    let before = std::fs::read(&index).expect("the index reads");

    list_files(
        &handle,
        &DiffSpec::WorkingTreeVsHead,
        &DiffOptions::default(),
        &(),
    )
    .expect("list_files succeeds");

    let after = std::fs::read(&index).expect("the index reads again");
    assert!(
        before == after,
        "listing the working tree rewrote .git/index ({} bytes -> {} bytes)",
        before.len(),
        after.len()
    );
}

/// `list_files(Staged)` lists exactly the paths a real `git diff --cached
/// --numstat` oracle does — an independent porcelain command, not plumbing,
/// since this is checking correctness, not the index-write guarantee (the
/// dedicated tests for that use plumbing throughout).
#[test]
fn list_files_staged_matches_a_cached_diff_oracle() {
    let scratch = graft_fixtures::copy_into_temp(Shape::FeatureBranches);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();
    let root = scratch.path();

    std::fs::write(root.join("a.txt"), "staged change\n").expect("a.txt written");
    graft_fixtures::git::run(root, &["add", "a.txt"]).expect("git adds a.txt");
    std::fs::write(root.join("b.txt"), "unstaged change, not staged\n").expect("b.txt written");

    let list = list_files(&handle, &DiffSpec::Staged, &DiffOptions::default(), &())
        .expect("list_files(Staged) succeeds");
    let paths: BTreeSet<Vec<u8>> = list.files.iter().map(|entry| entry.path.clone()).collect();

    assert_eq!(paths, BTreeSet::from([b"a.txt".to_vec()]));
}

/// `list_files(Unstaged)` lists a tracked edit and an untracked file
/// together, the same as [`DiffSpec::WorkingTreeVsHead`] does — but not the
/// staged one, which only [`DiffSpec::Staged`] shows.
#[test]
fn list_files_unstaged_lists_a_tracked_edit_and_an_untracked_file() {
    let scratch = graft_fixtures::copy_into_temp(Shape::FeatureBranches);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();
    let root = scratch.path();

    std::fs::write(root.join("a.txt"), "staged change\n").expect("a.txt written");
    graft_fixtures::git::run(root, &["add", "a.txt"]).expect("git adds a.txt");
    std::fs::write(root.join("b.txt"), "unstaged change\n").expect("b.txt written");
    std::fs::write(root.join("new.txt"), "untracked\n").expect("new.txt written");

    let list = list_files(&handle, &DiffSpec::Unstaged, &DiffOptions::default(), &())
        .expect("list_files(Unstaged) succeeds");
    let paths: BTreeSet<Vec<u8>> = list.files.iter().map(|entry| entry.path.clone()).collect();

    assert_eq!(
        paths,
        BTreeSet::from([b"b.txt".to_vec(), b"new.txt".to_vec()])
    );
    let untracked = list
        .files
        .iter()
        .find(|entry| entry.path == b"new.txt")
        .expect("new.txt is listed");
    assert_eq!(untracked.status, FileStatus::Untracked);
}

/// `list_files(Staged)` on an unborn branch diffs the index against the
/// empty tree, so a file staged before the first commit still shows —
/// design note, `DiffSpec::Staged`.
#[test]
fn list_files_staged_on_an_unborn_branch_diffs_against_the_empty_tree() {
    let fixture = Fixture::empty_repo();
    write_file(&fixture, "first.txt", "hello\n");
    fixture.git(&["add", "first.txt"]);
    let repo = RepoHandle::open(fixture.root()).expect("the fixture opens");

    let list = list_files(&repo, &DiffSpec::Staged, &DiffOptions::default(), &())
        .expect("list_files(Staged) succeeds on an unborn branch");

    assert_eq!(list.files.len(), 1);
    assert_eq!(list.files[0].path, b"first.txt");
    assert_eq!(list.files[0].status, FileStatus::Added);
}

/// `git diff [--cached] --no-color --no-ext-diff -M -C -U3 -- <path>`,
/// parsed the same way [`file_diff`]'s own output is, as the independent
/// oracle for [`file_diff_staged_and_unstaged_match_porcelain_diff_oracles`].
fn porcelain_diff_oracle(root: &Path, cached: bool, path: &str) -> FileDiff {
    let mut args = vec!["diff"];
    if cached {
        args.push("--cached");
    }
    args.extend(["--no-color", "--no-ext-diff", "-M", "-C", "-U3", "--", path]);
    let stdout = graft_fixtures::git::run(root, &args).expect("the diff oracle runs");
    parse_unified(&stdout)
        .expect("the oracle parses")
        .pop()
        .expect("one file")
}

/// `file_diff(Staged, …)`/`file_diff(Unstaged, …)` reproduce the same hunks
/// a porcelain `git diff --cached`/`git diff` oracle prints for the same
/// file — the parser is shared with M2 ([`parse_unified`]); this checks the
/// command each spec builds picks up the right side.
#[test]
fn file_diff_staged_and_unstaged_match_porcelain_diff_oracles() {
    let scratch = graft_fixtures::copy_into_temp(Shape::FeatureBranches);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();
    let root = scratch.path();

    std::fs::write(root.join("a.txt"), "staged change\nsecond line\n").expect("a.txt written");
    graft_fixtures::git::run(root, &["add", "a.txt"]).expect("git adds a.txt");
    std::fs::write(root.join("b.txt"), "unstaged change\n").expect("b.txt written");

    let staged_list = list_files(&handle, &DiffSpec::Staged, &DiffOptions::default(), &())
        .expect("list_files(Staged) succeeds");
    let staged_entry = staged_list
        .files
        .iter()
        .find(|entry| entry.path == b"a.txt")
        .expect("a.txt is staged");
    let staged_diff = file_diff(
        &handle,
        &DiffSpec::Staged,
        staged_entry,
        &DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff(Staged) succeeds");
    assert_eq!(
        staged_diff.hunks,
        porcelain_diff_oracle(root, true, "a.txt").hunks
    );

    let unstaged_list = list_files(&handle, &DiffSpec::Unstaged, &DiffOptions::default(), &())
        .expect("list_files(Unstaged) succeeds");
    let unstaged_entry = unstaged_list
        .files
        .iter()
        .find(|entry| entry.path == b"b.txt")
        .expect("b.txt is unstaged");
    let unstaged_diff = file_diff(
        &handle,
        &DiffSpec::Unstaged,
        unstaged_entry,
        &DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff(Unstaged) succeeds");
    assert_eq!(
        unstaged_diff.hunks,
        porcelain_diff_oracle(root, false, "b.txt").hunks
    );
}

/// M3: `status::read` and the two new `DiffSpec`s must never write
/// `.git/index` either — the guarantee M2 established for
/// `list_files(WorkingTreeVsHead)` above ("the index-untouched test from
/// round 1e is extended to both specs"). One test, five spawns, on a tree
/// stat-dirty everywhere so a refresh would be tempting if any of the five
/// ever did one.
#[test]
fn staging_reads_do_not_write_the_index() {
    let scratch = graft_fixtures::copy_into_temp(Shape::FeatureBranches);
    let repo = RepoHandle::open(scratch.path()).expect("the fixture opens");
    let root = scratch.path();

    // One staged edit, one unstaged edit — so list_files(Staged)/(Unstaged)
    // and file_diff have something real to read, not an empty answer.
    std::fs::write(root.join("a.txt"), "staged change\n").expect("a.txt written");
    graft_fixtures::git::run(root, &["add", "a.txt"]).expect("git adds a.txt");
    std::fs::write(root.join("b.txt"), "unstaged change\n").expect("b.txt written");

    // Every other tracked file gets a new mtime with the same content: the
    // stat-cache lie plumbing must not act on.
    for path in graft_fixtures::git::run(root, &["ls-files", "-z"])
        .expect("ls-files runs")
        .split(|&b| b == 0)
        .filter(|piece| !piece.is_empty())
    {
        let path = root.join(std::str::from_utf8(path).expect("fixture paths are ascii"));
        let content = std::fs::read(&path).expect("the tracked file reads");
        std::fs::write(&path, &content).expect("the same bytes go back with a new mtime");
    }

    let index = root.join(".git").join("index");
    let before = std::fs::read(&index).expect("the index reads");

    crate::status::read(&repo, &()).expect("status::read succeeds");
    let staged = list_files(&repo, &DiffSpec::Staged, &DiffOptions::default(), &())
        .expect("list_files(Staged) succeeds");
    let unstaged = list_files(&repo, &DiffSpec::Unstaged, &DiffOptions::default(), &())
        .expect("list_files(Unstaged) succeeds");
    let staged_entry = staged
        .files
        .iter()
        .find(|entry| entry.path == b"a.txt")
        .expect("a.txt is staged");
    let unstaged_entry = unstaged
        .files
        .iter()
        .find(|entry| entry.path == b"b.txt")
        .expect("b.txt is unstaged");
    file_diff(
        &repo,
        &DiffSpec::Staged,
        staged_entry,
        &DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff(Staged) succeeds");
    file_diff(
        &repo,
        &DiffSpec::Unstaged,
        unstaged_entry,
        &DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff(Unstaged) succeeds");

    let after = std::fs::read(&index).expect("the index reads again");
    assert!(
        before == after,
        "a staging read rewrote .git/index ({} bytes -> {} bytes)",
        before.len(),
        after.len()
    );
}

/// An untracked file's diff, read back from a real repo's
/// [`DiffSpec::Unstaged`] listing (m6-conflicts/81).
fn untracked_diff(content: &[u8], max_lines: usize) -> FileDiff {
    let fixture = Fixture::empty_repo();
    std::fs::write(fixture.root().join("new.bin"), content).expect("file written");
    let repo = RepoHandle::open(fixture.root()).expect("the fixture opens");
    let list = list_files(&repo, &DiffSpec::Unstaged, &DiffOptions::default(), &())
        .expect("list_files succeeds");
    let entry = list
        .files
        .iter()
        .find(|entry| entry.path == b"new.bin")
        .expect("the untracked file is listed");
    assert_eq!(entry.status, FileStatus::Untracked);
    // `git diff` porcelain refreshes `.git/index` (module doc); `--no-index`
    // must not. A tracked file makes the index exist to compare.
    std::fs::write(fixture.root().join("t.txt"), "t\n").expect("t.txt written");
    fixture.git(&["add", "t.txt"]);
    let index = fixture.root().join(".git").join("index");
    let before = std::fs::read(&index).expect("the index reads");
    let diff = file_diff(
        &repo,
        &DiffSpec::Unstaged,
        entry,
        &DiffOptions::default(),
        max_lines,
        &(),
    )
    .expect("file_diff reads an untracked file");
    let after = std::fs::read(&index).expect("the index reads again");
    assert!(before == after, "reading an untracked file wrote the index");
    assert_eq!(diff.entry, *entry, "the entry is kept as listed");
    diff
}

#[test]
fn untracked_file_diff_is_one_all_added_hunk() {
    let diff = untracked_diff(b"one\ntwo\nthree\n", usize::MAX);
    assert_eq!(diff.hunks.len(), 1);
    let lines = &diff.hunks[0].lines;
    let texts: Vec<&[u8]> = lines.iter().map(|line| line.text.as_slice()).collect();
    assert_eq!(texts, [&b"one"[..], b"two", b"three"]);
    assert!(
        lines
            .iter()
            .all(|line| line.kind == super::super::LineKind::Added)
    );
    assert!(diff.binary.is_none());
}

#[test]
fn untracked_binary_file_gives_the_binary_note() {
    let diff = untracked_diff(b"\0\x01\x02binary\0", usize::MAX);
    assert!(diff.hunks.is_empty());
    assert!(diff.binary.is_some());
}

#[test]
fn untracked_empty_file_gives_no_hunks_and_no_error() {
    let diff = untracked_diff(b"", usize::MAX);
    assert!(diff.hunks.is_empty());
    assert!(diff.binary.is_none());
}

#[test]
fn untracked_file_over_the_line_cap_is_truncated() {
    let body = (0..50).fold(String::new(), |mut body, n| {
        body.push_str(&format!("line {n}\n"));
        body
    });
    let diff = untracked_diff(body.as_bytes(), 10);
    let truncated = diff.truncated.expect("the cap's line is set");
    assert_eq!(truncated.total_lines, 50);
}
