use std::fs;
use std::path::{Path, PathBuf};

use graft_fixtures::Shape;

use super::{
    Change, StatusEntry, WorkingTreeCounts, WorkingTreeStatus, counts, parse_porcelain_v2, read,
};
use crate::fixture::Fixture;
use crate::repo::RepoHandle;

fn open(fixture: &Fixture) -> RepoHandle {
    RepoHandle::open(fixture.root()).expect("a fixture repository opens")
}

#[test]
fn a_clean_tree_reports_nothing_and_is_clean() {
    let fixture = Fixture::with_two_commits();
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads");

    assert_eq!(result, WorkingTreeCounts::default());
    assert!(result.is_clean());
}

#[test]
fn a_staged_file_counts_as_staged_only() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("staged.txt"), b"content").expect("file written");
    fixture.git(&["add", "staged.txt"]);
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads");

    assert_eq!(
        result,
        WorkingTreeCounts {
            staged: 1,
            unstaged: 0,
            untracked: 0,
        }
    );
    assert!(!result.is_clean());
}

#[test]
fn a_modified_tracked_file_counts_as_unstaged_only() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("tracked.txt"), b"first").expect("file written");
    fixture.git(&["add", "tracked.txt"]);
    fixture.commit("add tracked.txt");
    fs::write(fixture.root().join("tracked.txt"), b"changed").expect("file rewritten");
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads");

    assert_eq!(
        result,
        WorkingTreeCounts {
            staged: 0,
            unstaged: 1,
            untracked: 0,
        }
    );
}

#[test]
fn a_new_file_counts_as_untracked_only() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("new.txt"), b"content").expect("file written");
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads");

    assert_eq!(
        result,
        WorkingTreeCounts {
            staged: 0,
            unstaged: 0,
            untracked: 1,
        }
    );
}

#[test]
fn staged_unstaged_and_untracked_all_count_together() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join("tracked.txt"), b"first").expect("file written");
    fixture.git(&["add", "tracked.txt"]);
    fixture.commit("add tracked.txt");
    fs::write(fixture.root().join("tracked.txt"), b"changed").expect("unstaged edit");
    fs::write(fixture.root().join("staged.txt"), b"content").expect("staged file written");
    fixture.git(&["add", "staged.txt"]);
    fs::write(fixture.root().join("untracked.txt"), b"content").expect("untracked file written");
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads");

    assert_eq!(
        result,
        WorkingTreeCounts {
            staged: 1,
            unstaged: 1,
            untracked: 1,
        }
    );
}

#[test]
fn an_unborn_branch_with_a_staged_file_counts_it_as_staged() {
    // No commits at all: `HEAD` does not resolve to a tree. gix's status
    // falls back to the empty tree for the staged (tree-vs-index) side, so a
    // file added to the index before the first commit still counts.
    let fixture = Fixture::empty_repo();
    fs::write(fixture.root().join("first.txt"), b"content").expect("file written");
    fixture.git(&["add", "first.txt"]);
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads on an unborn branch");

    assert_eq!(
        result,
        WorkingTreeCounts {
            staged: 1,
            unstaged: 0,
            untracked: 0,
        }
    );
}

#[test]
fn a_bare_repository_is_reported_clean() {
    let fixture = Fixture::bare_repo();
    let repo = open(&fixture);

    let result = counts(&repo).expect("a bare repository short-circuits, not errors");

    assert_eq!(result, WorkingTreeCounts::default());
    assert!(result.is_clean());
}

#[test]
fn parse_porcelain_v2_reads_a_modified_and_an_untracked_record() {
    let bytes = b"1 M. N... 100644 100644 100644 aaaa bbbb tracked.txt\0? new.txt\0";

    let status = parse_porcelain_v2(bytes).expect("well-formed input parses");

    assert_eq!(
        status.entries,
        vec![
            StatusEntry {
                path: PathBuf::from("tracked.txt"),
                original_path: None,
                staged: Some(Change::Modified),
                unstaged: None,
                untracked: false,
                unmerged: false,
                submodule: false,
            },
            StatusEntry {
                path: PathBuf::from("new.txt"),
                original_path: None,
                staged: None,
                unstaged: None,
                untracked: true,
                unmerged: false,
                submodule: false,
            },
        ]
    );
}

#[test]
fn parse_porcelain_v2_pairs_a_rename_record_with_the_next_record_as_its_origin() {
    let bytes = b"2 R. N... 100644 100644 100644 aaaa bbbb R100 new.txt\0old.txt\0";

    let status = parse_porcelain_v2(bytes).expect("a rename record parses");

    assert_eq!(status.entries.len(), 1);
    let entry = &status.entries[0];
    assert_eq!(entry.path, PathBuf::from("new.txt"));
    assert_eq!(entry.original_path, Some(PathBuf::from("old.txt")));
    assert_eq!(entry.staged, Some(Change::Renamed));
}

#[test]
fn parse_porcelain_v2_reads_an_unmerged_record_as_unmerged_not_a_change() {
    let bytes = b"u UU N... 100644 100644 100644 100644 aaaa bbbb cccc conflict.txt\0";

    let status = parse_porcelain_v2(bytes).expect("an unmerged record parses");

    assert_eq!(status.entries.len(), 1);
    let entry = &status.entries[0];
    assert!(entry.unmerged);
    assert_eq!(entry.staged, None);
    assert_eq!(entry.unstaged, None);
}

#[test]
fn parse_porcelain_v2_rejects_garbage_instead_of_panicking() {
    for garbage in [
        &b""[..],
        b"\0",
        b"x garbage\0",
        b"1 too few fields\0",
        b"1 XY N... 100644 100644 100644 aaaa bbbb path.txt\0", // X is not a status letter
        b"2 R. N... 100644 100644 100644 aaaa bbbb R100 new.txt", // no origin record follows: buffer ends mid-record
        b"2 R. N... 100644 100644 100644 aaaa bbbb R100 new.txt\0", // ends right after, same as a capped/truncated stream
    ] {
        let result = parse_porcelain_v2(garbage);
        assert!(
            garbage.is_empty() || garbage == b"\0" || result.is_err(),
            "{garbage:?} should not parse: {result:?}"
        );
    }
}

#[test]
fn empty_or_all_nul_input_parses_to_an_empty_status() {
    assert_eq!(
        parse_porcelain_v2(b"").expect("empty input parses"),
        WorkingTreeStatus::default()
    );
    assert_eq!(
        parse_porcelain_v2(b"\0\0").expect("bare NULs parse"),
        WorkingTreeStatus::default()
    );
}

/// Counts a raw `-z` porcelain v2 buffer directly, without going through
/// this crate's own [`parse_porcelain_v2`] or its helpers — an oracle
/// `WorkingTreeStatus::counts` must independently agree with.
fn oracle_counts(bytes: &[u8]) -> (usize, usize, usize) {
    let mut staged = 0;
    let mut unstaged = 0;
    let mut untracked = 0;
    let mut records = bytes.split(|&b| b == 0);
    while let Some(record) = records.next() {
        if record.is_empty() {
            continue;
        }
        match record[0] {
            b'1' | b'2' => {
                let (x, y) = (record[2], record[3]);
                if x != b'.' {
                    staged += 1;
                }
                if y != b'.' {
                    unstaged += 1;
                }
                if record[0] == b'2' {
                    records.next(); // the origin path, its own record
                }
            }
            b'?' => untracked += 1,
            _ => {}
        }
    }
    (staged, unstaged, untracked)
}

/// The fixture, or `None` for a shape this machine cannot check out (a
/// non-UTF-8 ref name on APFS), said rather than passed over.
fn available<T, E: std::fmt::Display>(shape: Shape, fixture: Result<T, E>) -> Option<T> {
    match fixture {
        Ok(fixture) => Some(fixture),
        Err(why) => {
            let reason = why.to_string();
            assert!(reason.contains("not valid UTF-8"), "{shape}: {reason}");
            None
        }
    }
}

/// The paths `git ls-files` reports at `root`.
fn tracked_files(root: &Path) -> Vec<String> {
    let output = graft_fixtures::git::run(root, &["ls-files", "-z"]).expect("ls-files runs");
    output
        .split(|&b| b == 0)
        .filter(|piece| !piece.is_empty())
        .map(|piece| String::from_utf8(piece.to_vec()).expect("fixture paths are UTF-8"))
        .collect()
}

/// The six paths [`script_six_dirty_states`] touched, so the test can look
/// each one back up in the status it reads.
struct SixDirtyPaths {
    staged_edit: String,
    unstaged_edit: String,
    staged_and_unstaged_edit: String,
    rename_from: String,
    rename_to: String,
    deleted: String,
    untracked: &'static str,
}

/// Scripts one staged edit, one unstaged edit, one file with both, one
/// staged rename and one unstaged delete across five of `root`'s tracked
/// files, plus one new untracked file.
fn script_six_dirty_states(root: &Path, files: &[String]) -> SixDirtyPaths {
    let paths = SixDirtyPaths {
        staged_edit: files[0].clone(),
        unstaged_edit: files[1].clone(),
        staged_and_unstaged_edit: files[2].clone(),
        rename_from: files[3].clone(),
        rename_to: format!("{}-renamed", files[3]),
        deleted: files[4].clone(),
        untracked: "oracle-untracked.txt",
    };

    fs::write(root.join(&paths.staged_edit), b"staged edit\n").expect("staged edit written");
    graft_fixtures::git::run(root, &["add", &paths.staged_edit]).expect("git adds the staged edit");

    fs::write(root.join(&paths.unstaged_edit), b"unstaged edit\n").expect("unstaged edit written");

    fs::write(root.join(&paths.staged_and_unstaged_edit), b"staged half\n")
        .expect("staged half written");
    graft_fixtures::git::run(root, &["add", &paths.staged_and_unstaged_edit])
        .expect("git adds the staged half");
    fs::write(
        root.join(&paths.staged_and_unstaged_edit),
        b"unstaged half\n",
    )
    .expect("unstaged half written");

    graft_fixtures::git::run(root, &["mv", &paths.rename_from, &paths.rename_to])
        .expect("git mv stages a rename");

    fs::remove_file(root.join(&paths.deleted)).expect("the deleted file is removed");

    fs::write(root.join(paths.untracked), b"new\n").expect("untracked file written");

    paths
}

/// `read(repo)`'s counts must match an independent count of `git status`'s
/// own raw output, and each of the six scripted paths must carry the
/// [`Change`] its scripted state implies.
fn assert_status_matches_the_six_dirty_states(shape: Shape, root: &Path, paths: &SixDirtyPaths) {
    let repo = RepoHandle::open(root).expect("the scratch copy opens");
    let status = read(&repo, &()).unwrap_or_else(|error| {
        panic!("{shape}: status::read failed: {error}");
    });

    let raw = graft_fixtures::git::run(
        root,
        &[
            "-c",
            "core.quotepath=off",
            "status",
            "--porcelain=v2",
            "-z",
            "--untracked-files=all",
            "--renames",
        ],
    )
    .expect("the oracle git status runs");
    let expected = oracle_counts(&raw);
    let actual = status.counts();
    assert_eq!(
        (actual.staged, actual.unstaged, actual.untracked),
        expected,
        "{shape}: read().counts() must match git's own record letters"
    );

    let entry = |path: &str| {
        status
            .entry(Path::new(path))
            .unwrap_or_else(|| panic!("{shape}: {path} must be in the status"))
    };

    let staged = entry(&paths.staged_edit);
    assert_eq!(staged.staged, Some(Change::Modified), "{shape}");
    assert_eq!(staged.unstaged, None, "{shape}");

    let unstaged = entry(&paths.unstaged_edit);
    assert_eq!(unstaged.staged, None, "{shape}");
    assert_eq!(unstaged.unstaged, Some(Change::Modified), "{shape}");

    let both = entry(&paths.staged_and_unstaged_edit);
    assert_eq!(both.staged, Some(Change::Modified), "{shape}");
    assert_eq!(both.unstaged, Some(Change::Modified), "{shape}");

    let renamed = entry(&paths.rename_to);
    assert_eq!(renamed.staged, Some(Change::Renamed), "{shape}");
    assert_eq!(
        renamed.original_path,
        Some(PathBuf::from(&paths.rename_from)),
        "{shape}"
    );

    assert_eq!(
        entry(&paths.deleted).unstaged,
        Some(Change::Deleted),
        "{shape}"
    );
    assert!(entry(paths.untracked).untracked, "{shape}");
}

#[test]
fn read_matches_gits_own_counts_and_letters_on_every_shape() {
    for &shape in Shape::ALL {
        let Some(scratch) = available(shape, graft_fixtures::try_copy_into_temp(shape)) else {
            continue;
        };
        let root = scratch.path();
        let files = tracked_files(root);
        // Six scripted dirty states need six distinct tracked files; a shape
        // too small for that is not this test's business (it still gets
        // covered as `Linear`/`FeatureBranches` and the like, which have
        // plenty).
        if files.len() < 6 {
            continue;
        }

        let paths = script_six_dirty_states(root, &files);
        assert_status_matches_the_six_dirty_states(shape, root, &paths);
    }
}

#[test]
fn ignored_files_are_not_counted_as_untracked() {
    let fixture = Fixture::with_two_commits();
    fs::write(fixture.root().join(".gitignore"), b"ignored.txt\n").expect(".gitignore written");
    fixture.git(&["add", ".gitignore"]);
    fixture.commit("add .gitignore");
    fs::write(fixture.root().join("ignored.txt"), b"content").expect("ignored file written");
    let repo = open(&fixture);

    let result = counts(&repo).expect("status reads");

    assert_eq!(
        result,
        WorkingTreeCounts {
            staged: 0,
            unstaged: 0,
            untracked: 0,
        }
    );
}
