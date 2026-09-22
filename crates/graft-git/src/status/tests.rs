use std::fs;
use std::path::{Path, PathBuf};

use graft_fixtures::Shape;

use super::{
    Change, StatusEntry, StatusError, WorkingTreeCounts, WorkingTreeStatus, parse_porcelain_v2,
    read, status_from,
};
use crate::fixture::Fixture;
use crate::repo::RepoHandle;

fn open(fixture: &Fixture) -> RepoHandle {
    RepoHandle::open(fixture.root()).expect("a fixture repository opens")
}

/// The gix-walking `status::counts` these seven tests covered went in
/// m5-history-and-ghost-preview/22 with its last caller. What it counted is
/// now counted from `read`'s own entries, and
/// `read_matches_gits_own_counts_and_letters_on_every_shape` checks those
/// against git's own output on every corpus shape. The one case that walk
/// answered without spawning git at all is kept here.
#[test]
fn a_bare_repository_is_reported_clean() {
    let fixture = Fixture::bare_repo();
    let repo = open(&fixture);

    let status = read(&repo, &()).expect("a bare repository short-circuits, not errors");

    assert!(status.is_clean());
    assert_eq!(status.counts(), WorkingTreeCounts::default());
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
                unmerged_how: None,
                submodule: false,
            },
            StatusEntry {
                path: PathBuf::from("new.txt"),
                original_path: None,
                staged: None,
                unstaged: None,
                untracked: true,
                unmerged: false,
                unmerged_how: None,
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

    let counts = read(&repo, &()).expect("status reads").counts();

    assert_eq!(
        counts,
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

/// `UnmergedHow` as git would print it, for comparing with a raw `u` record.
fn xy_of(how: super::UnmergedHow) -> &'static str {
    use super::UnmergedHow;
    match how {
        UnmergedHow::BothModified => "UU",
        UnmergedHow::BothAdded => "AA",
        UnmergedHow::BothDeleted => "DD",
        UnmergedHow::AddedByUs => "AU",
        UnmergedHow::AddedByThem => "UA",
        UnmergedHow::DeletedByUs => "DU",
        UnmergedHow::DeletedByThem => "UD",
    }
}

#[test]
fn unmerged_how_from_porcelain() {
    use super::UnmergedHow;

    // Every pair, by hand, with git's words.
    for (xy, how, words, markers) in [
        ("UU", UnmergedHow::BothModified, "both modified", true),
        ("AA", UnmergedHow::BothAdded, "both added", true),
        ("DD", UnmergedHow::BothDeleted, "both deleted", false),
        ("AU", UnmergedHow::AddedByUs, "added by us", false),
        ("UA", UnmergedHow::AddedByThem, "added by them", false),
        ("DU", UnmergedHow::DeletedByUs, "deleted by us", false),
        ("UD", UnmergedHow::DeletedByThem, "deleted by them", false),
    ] {
        let record = format!("u {xy} N... 100644 100644 100644 100644 aaaa bbbb cccc p.txt\0");
        let status = parse_porcelain_v2(record.as_bytes()).expect("parses");
        let entry = &status.entries[0];
        assert!(entry.unmerged);
        assert_eq!(entry.unmerged_how, Some(how), "{xy}");
        assert_eq!(how.words(), words);
        assert_eq!(how.has_markers(), markers, "{xy}");
        assert_eq!(xy_of(how), xy);
    }
    let bad = b"u MM N... 100644 100644 100644 100644 aaaa bbbb cccc p.txt\0";
    assert!(matches!(
        parse_porcelain_v2(bad),
        Err(StatusError::Malformed(_))
    ));

    // Every file in both stopped corpus shapes, against git's own XY, read
    // independently of the parser under test.
    for shape in [Shape::StoppedRebase, Shape::StoppedMerge] {
        stopped_shape_unmerged_how_matches_git(shape);
    }
}

/// `(path, XY)` for every `u` record `git status --porcelain=v2 -z` prints
/// in `dir`, sorted by path — split by hand, not by the parser under test.
fn gits_own_xy(dir: &Path) -> Vec<(String, String)> {
    let raw = graft_fixtures::git::run(dir, &["status", "--porcelain=v2", "-z"])
        .unwrap_or_else(|why| panic!("{why}"));
    let mut found: Vec<(String, String)> = raw
        .split(|&b| b == 0)
        .filter(|record| record.first() == Some(&b'u'))
        .map(|record| {
            let text = String::from_utf8_lossy(record).into_owned();
            let fields: Vec<&str> = text.splitn(11, ' ').collect();
            (fields[10].to_owned(), fields[1].to_owned())
        })
        .collect();
    found.sort();
    found
}

fn stopped_shape_unmerged_how_matches_git(shape: Shape) {
    use super::UnmergedHow;

    let expected: Vec<(String, UnmergedHow)> = [
        ("added-by-them.txt", UnmergedHow::AddedByThem),
        ("added-by-us.txt", UnmergedHow::AddedByUs),
        ("both-added.txt", UnmergedHow::BothAdded),
        ("both-deleted.txt", UnmergedHow::BothDeleted),
        ("both.txt", UnmergedHow::BothModified),
        ("crlf.txt", UnmergedHow::BothModified),
        ("md.md", UnmergedHow::BothModified),
        ("nofinal.txt", UnmergedHow::BothModified),
        ("ours-deleted.txt", UnmergedHow::DeletedByUs),
        ("pic.bin", UnmergedHow::BothModified),
        ("theirs-deleted.txt", UnmergedHow::DeletedByThem),
        ("three.txt", UnmergedHow::BothModified),
        ("zdiff3.txt", UnmergedHow::BothModified),
    ]
    .into_iter()
    .map(|(path, how)| (path.to_owned(), how))
    .collect();
    let dir = graft_fixtures::borrow(shape);
    let repo = RepoHandle::open(&dir).expect("the fixture opens");
    let status = read(&repo, &()).unwrap_or_else(|why| panic!("{shape}: {why}"));
    let mut found: Vec<(String, UnmergedHow)> = status
        .entries
        .iter()
        .filter(|entry| entry.unmerged)
        .map(|entry| {
            (
                entry.path.to_string_lossy().into_owned(),
                entry.unmerged_how.expect("an unmerged entry says how"),
            )
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(found, expected, "{shape}");
    assert!(
        status
            .entries
            .iter()
            .all(|entry| entry.unmerged == entry.unmerged_how.is_some())
    );

    let ours: Vec<(String, String)> = found
        .iter()
        .map(|(path, how)| (path.clone(), xy_of(*how).to_owned()))
        .collect();
    assert_eq!(ours, gits_own_xy(&dir), "{shape}: XY as git printed it");
}
