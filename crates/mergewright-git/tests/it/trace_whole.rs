//! `whole_file`'s whitespace flag and `blob_sizes`, on real throwaway repos
//! (m7-power-tools/21c).

use std::path::Path;

use mergewright_core::oid::Oid;
use mergewright_fixtures::Shape;
use mergewright_git::diff::{FileDiff, LineKind};
use mergewright_git::trace::{BlobSizes, blob_sizes, whole_file};

use crate::session::{OLD_FILE, commits, git_ok, open, rev};
use crate::trace::with_file;

/// A copy of the fixture where `re.rs` was added, then re-indented with one
/// real line added at its end.
fn reindented() -> (mergewright_fixtures::Scratch, Oid) {
    let (scratch, _) = with_file("re.rs", "fn a() {\nx();\ny();\nz();\n}\n");
    let dir = scratch.path();
    std::fs::write(
        dir.join("re.rs"),
        "fn a() {\n    x();\n    y();\n    z();\n}\n// end\n",
    )
    .unwrap_or_else(|why| panic!("writing re.rs: {why}"));
    git_ok(dir, &["commit", "-q", "-am", "reindent"]);
    let head = rev(dir, "HEAD");
    (scratch, head)
}

fn changed_rows(diff: &FileDiff) -> Vec<(LineKind, Vec<u8>)> {
    diff.hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| line.kind != LineKind::Context)
        .map(|line| (line.kind, line.text.clone()))
        .collect()
}

#[test]
fn whole_file_ignoring_whitespace_marks_a_reindent_unchanged() {
    let (scratch, head) = reindented();
    let repo = open(scratch.path());
    let path = Path::new("re.rs");
    let seen =
        whole_file(&repo, head, path, None, usize::MAX, false, &()).expect("with whitespace");
    assert_eq!(changed_rows(&seen).len(), 7, "3 removed, 3 added, 1 added");
    let ignored =
        whole_file(&repo, head, path, None, usize::MAX, true, &()).expect("ignoring whitespace");
    assert_eq!(
        changed_rows(&ignored),
        [(LineKind::Added, b"// end".to_vec())]
    );
    let lines: usize = ignored.hunks.iter().map(|hunk| hunk.lines.len()).sum();
    assert_eq!(lines, 6, "every line still shown");
}

#[test]
fn whole_file_ignoring_whitespace_of_a_whitespace_only_change_is_all_context() {
    let (scratch, _) = with_file("ws.rs", "a\nb\n");
    let dir = scratch.path();
    std::fs::write(dir.join("ws.rs"), "  a\nb\n").expect("writes");
    git_ok(dir, &["commit", "-q", "-am", "indent"]);
    let head = rev(dir, "HEAD");
    let diff = whole_file(
        &open(dir),
        head,
        Path::new("ws.rs"),
        None,
        usize::MAX,
        true,
        &(),
    )
    .expect("ignoring whitespace");
    assert_eq!(diff.hunks.len(), 1);
    assert_eq!(diff.hunks[0].lines.len(), 2);
    assert!(changed_rows(&diff).is_empty());
}

#[test]
fn blob_sizes_reads_both_sides() {
    let (scratch, _) = with_file("bin.dat", &"x".repeat(10));
    let dir = scratch.path();
    std::fs::write(dir.join("bin.dat"), "y".repeat(2000)).expect("writes");
    git_ok(dir, &["commit", "-q", "-am", "grow"]);
    let head = rev(dir, "HEAD");
    let sizes = blob_sizes(&open(dir), head, Path::new("bin.dat"), None, &()).expect("sizes");
    assert_eq!(
        sizes,
        BlobSizes {
            old: Some(10),
            new: Some(2000)
        }
    );
}

#[test]
fn blob_sizes_of_an_added_file_has_no_old() {
    let (scratch, head) = with_file("new.dat", "abc");
    let sizes =
        blob_sizes(&open(scratch.path()), head, Path::new("new.dat"), None, &()).expect("sizes");
    assert_eq!(
        sizes,
        BlobSizes {
            old: None,
            new: Some(3)
        }
    );
}

#[test]
fn blob_sizes_of_a_deleted_file_has_no_new() {
    let (scratch, _) = with_file("gone.dat", "abcd");
    let dir = scratch.path();
    git_ok(dir, &["rm", "-q", "gone.dat"]);
    git_ok(dir, &["commit", "-q", "-m", "delete"]);
    let head = rev(dir, "HEAD");
    let sizes = blob_sizes(&open(dir), head, Path::new("gone.dat"), None, &()).expect("sizes");
    assert_eq!(
        sizes,
        BlobSizes {
            old: Some(4),
            new: None
        }
    );
}

#[test]
fn blob_sizes_follows_a_rename() {
    let (scratch, _) = with_file("before.dat", "12345");
    let dir = scratch.path();
    git_ok(dir, &["mv", "before.dat", "after.dat"]);
    std::fs::write(dir.join("after.dat"), "123456").expect("writes");
    git_ok(dir, &["add", "after.dat"]);
    git_ok(dir, &["commit", "-q", "-m", "rename"]);
    let head = rev(dir, "HEAD");
    let sizes = blob_sizes(
        &open(dir),
        head,
        Path::new("after.dat"),
        Some(Path::new("before.dat")),
        &(),
    )
    .expect("sizes");
    assert_eq!(
        sizes,
        BlobSizes {
            old: Some(5),
            new: Some(6)
        }
    );
}

#[test]
fn blob_sizes_on_a_root_commit() {
    let dir = mergewright_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let sizes = blob_sizes(&open(&dir), ids.ana, Path::new(OLD_FILE), None, &()).expect("sizes");
    assert_eq!(sizes.old, None);
    let git = git_ok(
        &dir,
        &["cat-file", "-s", &format!("{}:{OLD_FILE}", ids.ana)],
    );
    let git: u64 = String::from_utf8_lossy(&git)
        .trim()
        .parse()
        .expect("a size");
    assert_eq!(sizes.new, Some(git), "git's own count");
}
