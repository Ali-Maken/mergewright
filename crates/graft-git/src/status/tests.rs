use std::fs;
use std::path::{Path, PathBuf};

use graft_fixtures::Shape;

use super::{
    Change, StatusEntry, StatusError, WorkingTreeCounts, WorkingTreeStatus, counts,
    parse_porcelain_v2, read, status_from,
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

/// The paths [`script_dirty_states`] touched, so the test can look each one
/// back up in the status it reads. Each tracked state needs a tracked file of
/// its own, and the corpus shapes have between zero and five, so every one is
/// an `Option`: what the shape can carry is scripted, what it cannot is
/// `None` and is not asserted. The untracked file needs nothing and is
/// always there.
#[derive(Default)]
struct DirtyPaths {
    staged_edit: Option<String>,
    unstaged_edit: Option<String>,
    staged_and_unstaged_edit: Option<String>,
    rename_from: Option<String>,
    rename_to: Option<String>,
    deleted: Option<String>,
    untracked: &'static str,
}

impl DirtyPaths {
    /// How many of the five tracked states were actually scripted.
    fn tracked_states(&self) -> usize {
        [
            self.staged_edit.is_some(),
            self.unstaged_edit.is_some(),
            self.staged_and_unstaged_edit.is_some(),
            self.rename_to.is_some(),
            self.deleted.is_some(),
        ]
        .iter()
        .filter(|scripted| **scripted)
        .count()
    }
}

/// Scripts one staged edit, one unstaged edit, one file with both, one staged
/// rename and one unstaged delete across the first five of `root`'s tracked
/// files, plus one new untracked file — as many of the five as `files` has
/// entries for, in that order.
fn script_dirty_states(root: &Path, files: &[String]) -> DirtyPaths {
    let mut paths = DirtyPaths {
        untracked: "oracle-untracked.txt",
        ..DirtyPaths::default()
    };
    fs::write(root.join(paths.untracked), b"new\n").expect("untracked file written");

    if let Some(path) = files.first() {
        fs::write(root.join(path), b"staged edit\n").expect("staged edit written");
        graft_fixtures::git::run(root, &["add", path]).expect("git adds the staged edit");
        paths.staged_edit = Some(path.clone());
    }
    if let Some(path) = files.get(1) {
        fs::write(root.join(path), b"unstaged edit\n").expect("unstaged edit written");
        paths.unstaged_edit = Some(path.clone());
    }
    if let Some(path) = files.get(2) {
        fs::write(root.join(path), b"staged half\n").expect("staged half written");
        graft_fixtures::git::run(root, &["add", path]).expect("git adds the staged half");
        fs::write(root.join(path), b"unstaged half\n").expect("unstaged half written");
        paths.staged_and_unstaged_edit = Some(path.clone());
    }
    if let Some(path) = files.get(3) {
        let renamed = format!("{path}-renamed");
        graft_fixtures::git::run(root, &["mv", path, &renamed]).expect("git mv stages a rename");
        paths.rename_from = Some(path.clone());
        paths.rename_to = Some(renamed);
    }
    if let Some(path) = files.get(4) {
        fs::remove_file(root.join(path)).expect("the deleted file is removed");
        paths.deleted = Some(path.clone());
    }
    paths
}

/// `read(repo)`'s counts must match an independent count of `git status`'s
/// own raw output, and each scripted path must carry the [`Change`] its
/// scripted state implies.
fn assert_status_matches_the_dirty_states(label: &str, root: &Path, paths: &DirtyPaths) {
    let repo = RepoHandle::open(root).expect("the repository opens");
    let status = read(&repo, &()).unwrap_or_else(|error| {
        panic!("{label}: status::read failed: {error}");
    });
    assert_counts_match_the_oracle(label, root, &status);
    assert_letters_match_the_scripted_states(label, &status, paths);
}

/// `read(repo).counts()` against an independent count of `git status`'s own
/// raw output.
fn assert_counts_match_the_oracle(label: &str, root: &Path, status: &WorkingTreeStatus) {
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
        "{label}: read().counts() must match git's own record letters"
    );
}

/// Each scripted path carries the [`Change`] its scripted state implies.
fn assert_letters_match_the_scripted_states(
    label: &str,
    status: &WorkingTreeStatus,
    paths: &DirtyPaths,
) {
    let entry = |path: &str| {
        status
            .entry(Path::new(path))
            .unwrap_or_else(|| panic!("{label}: {path} must be in the status"))
    };

    let sides = [
        (&paths.staged_edit, Some(Change::Modified), None),
        (&paths.unstaged_edit, None, Some(Change::Modified)),
        (
            &paths.staged_and_unstaged_edit,
            Some(Change::Modified),
            Some(Change::Modified),
        ),
    ];
    for (scripted, staged, unstaged) in sides {
        let Some(path) = scripted else { continue };
        let found = entry(path);
        assert_eq!(found.staged, staged, "{label}: {path} staged side");
        assert_eq!(found.unstaged, unstaged, "{label}: {path} unstaged side");
    }
    if let (Some(to), Some(from)) = (&paths.rename_to, &paths.rename_from) {
        let renamed = entry(to);
        assert_eq!(renamed.staged, Some(Change::Renamed), "{label}: {to}");
        assert_eq!(
            renamed.original_path,
            Some(PathBuf::from(from)),
            "{label}: {to}"
        );
    }
    if let Some(path) = &paths.deleted {
        assert_eq!(
            entry(path).unstaged,
            Some(Change::Deleted),
            "{label}: {path}"
        );
    }
    assert!(entry(paths.untracked).untracked, "{label}");
}

#[test]
fn read_matches_gits_own_counts_and_letters_on_every_shape() {
    let mut shapes_checked = 0usize;
    let mut best_states = 0usize;
    for &shape in Shape::ALL {
        let Some(scratch) = available(shape, graft_fixtures::try_copy_into_temp(shape)) else {
            continue;
        };
        let root = scratch.path();
        let files = tracked_files(root);
        let paths = script_dirty_states(root, &files);
        assert_status_matches_the_dirty_states(&shape.to_string(), root, &paths);
        shapes_checked += 1;
        best_states = best_states.max(paths.tracked_states());
    }
    // A loop that can skip every iteration proves nothing; this one says out
    // loud how much it actually ran. The corpus's widest shape carries five
    // tracked files, so all five tracked states are scripted somewhere —
    // `all_six_dirty_states_on_a_repository_built_for_them` guarantees it
    // even if the corpus ever shrinks.
    // Only the non-UTF-8-ref shape may drop out (`available` says so), so
    // anything short of "all but one" means the loop started skipping again.
    assert!(
        shapes_checked + 1 >= Shape::ALL.len(),
        "only {shapes_checked} of {} shapes were checked",
        Shape::ALL.len()
    );
    assert!(
        best_states == 5,
        "no shape carried all five tracked states; the widest one scripted {best_states}"
    );
}

#[test]
fn all_six_dirty_states_on_a_repository_built_for_them() {
    // The corpus decides how many tracked files a shape has; this repository
    // is built with exactly the five the six states need, so the rename and
    // delete letters are asserted against real git whatever the corpus holds.
    let fixture = Fixture::empty_repo();
    let files: Vec<String> = (0..5).map(|index| format!("f{index}.txt")).collect();
    for path in &files {
        fs::write(fixture.root().join(path), b"original\n").expect("file written");
    }
    fixture.git(&["add", "-A"]);
    fixture.commit("five files");

    let paths = script_dirty_states(fixture.root(), &files);
    assert_eq!(paths.tracked_states(), 5, "all five states are scripted");
    assert_status_matches_the_dirty_states("built-for-six", fixture.root(), &paths);
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

#[test]
fn a_capped_status_is_an_error_not_a_shorter_list() {
    // A cut at a record boundary parses perfectly well into fewer changes
    // than the tree really has; that is exactly the answer that must not be
    // returned.
    let whole = b"1 M. N... 100644 100644 100644 \
0000000000000000000000000000000000000000 \
0000000000000000000000000000000000000000 a.txt\0? b.txt\0";
    let full = status_from(&crate::run::GitOutput {
        stdout: whole.to_vec(),
        stderr: Vec::new(),
        truncated: false,
    })
    .expect("an uncapped status parses");
    assert_eq!(full.entries.len(), 2);

    let cut = whole
        .iter()
        .position(|&byte| byte == 0)
        .expect("the record separator is there")
        + 1;
    let capped = status_from(&crate::run::GitOutput {
        stdout: whole[..cut].to_vec(),
        stderr: Vec::new(),
        truncated: true,
    });
    assert!(
        matches!(capped, Err(StatusError::TooLarge { .. })),
        "{capped:?}"
    );
}
