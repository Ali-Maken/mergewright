//! Hand-written `parse_unified` cases, one per shape of git output the
//! fixture round-trip test cannot be relied on to produce (02-diff-parser).
//! Each input is copied from what real `git diff-tree -p -M -C --no-color
//! --no-ext-diff` actually prints for that shape (checked by hand against a
//! scratch repository while writing this ticket), kept under 40 lines.
//!
//! Built from `&str` lines joined with `\n`, not a multi-line byte-string
//! literal: Rust's line-continuation (`\` at end of line) strips *all*
//! leading whitespace on the next line, which would eat the leading space a
//! context line's marker depends on.

use graft_git::diff::{BinaryNote, FileStatus, parse_unified};

fn input(lines: &[&str]) -> Vec<u8> {
    let mut s = lines.join("\n");
    s.push('\n');
    s.into_bytes()
}

#[test]
fn rename_with_hunks() {
    let diffs = parse_unified(&input(&[
        "diff --git a/file1.txt b/sub/file1.txt",
        "similarity index 54%",
        "rename from file1.txt",
        "rename to sub/file1.txt",
        "index de98044..29b2afc 100644",
        "--- a/file1.txt",
        "+++ b/sub/file1.txt",
        "@@ -1,3 +1,4 @@",
        " a",
        " b",
        " c",
        "+more",
    ]))
    .unwrap();
    assert_eq!(diffs.len(), 1);
    let d = &diffs[0];
    assert_eq!(d.entry.path, b"sub/file1.txt");
    assert_eq!(d.entry.old_path, Some(b"file1.txt".to_vec()));
    assert_eq!(d.entry.status, FileStatus::Renamed { similarity: 54 });
    assert_eq!(d.hunks.len(), 1);
    assert_eq!(d.entry.added, Some(1));
    assert_eq!(d.entry.removed, Some(0));
}

#[test]
fn copy_with_no_content_change() {
    let diffs = parse_unified(&input(&[
        "diff --git a/orig.txt b/copy.txt",
        "similarity index 100%",
        "copy from orig.txt",
        "copy to copy.txt",
    ]))
    .unwrap();
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].entry.path, b"copy.txt");
    assert_eq!(diffs[0].entry.old_path, Some(b"orig.txt".to_vec()));
    assert_eq!(
        diffs[0].entry.status,
        FileStatus::Copied { similarity: 100 }
    );
    assert!(diffs[0].hunks.is_empty());
}

#[test]
fn mode_only_change_has_no_hunks() {
    let diffs = parse_unified(&input(&[
        "diff --git a/script.sh b/script.sh",
        "old mode 100644",
        "new mode 100755",
    ]))
    .unwrap();
    assert_eq!(diffs.len(), 1);
    let d = &diffs[0];
    assert_eq!(d.entry.path, b"script.sh");
    assert_eq!(d.entry.status, FileStatus::Modified);
    assert_eq!(d.old_mode, Some(100_644));
    assert_eq!(d.new_mode, Some(100_755));
    assert!(d.hunks.is_empty());
}

#[test]
fn binary_file() {
    let diffs = parse_unified(&input(&[
        "diff --git a/image.png b/image.png",
        "index abc1234..def5678 100644",
        "Binary files a/image.png and b/image.png differ",
    ]))
    .unwrap();
    assert_eq!(diffs[0].binary, Some(BinaryNote::Differ));
    assert_eq!(diffs[0].entry.added, None);
    assert_eq!(diffs[0].entry.removed, None);
}

#[test]
fn git_binary_patch_body_is_skipped() {
    let diffs = parse_unified(&input(&[
        "diff --git a/blob.bin b/blob.bin",
        "index abc1234..def5678 100644",
        "GIT binary patch",
        "literal 12",
        "zc%1Q$0ssI200000",
        "",
        "diff --git a/next.txt b/next.txt",
        "index aaa..bbb 100644",
        "--- a/next.txt",
        "+++ b/next.txt",
        "@@ -1 +1 @@",
        "-a",
        "+b",
    ]))
    .unwrap();
    assert_eq!(diffs.len(), 2);
    assert_eq!(diffs[0].binary, Some(BinaryNote::Patch));
    assert!(diffs[0].hunks.is_empty());
    assert_eq!(diffs[1].entry.path, b"next.txt");
    assert_eq!(diffs[1].hunks.len(), 1);
}

#[test]
fn new_file() {
    let diffs = parse_unified(&input(&[
        "diff --git a/new.txt b/new.txt",
        "new file mode 100644",
        "index 0000000..de98044",
        "--- /dev/null",
        "+++ b/new.txt",
        "@@ -0,0 +1,2 @@",
        "+line one",
        "+line two",
    ]))
    .unwrap();
    let d = &diffs[0];
    assert_eq!(d.entry.path, b"new.txt");
    assert_eq!(d.entry.old_path, None);
    assert_eq!(d.entry.status, FileStatus::Added);
    assert_eq!(d.entry.added, Some(2));
    assert_eq!(d.entry.removed, Some(0));
}

#[test]
fn deleted_file() {
    let diffs = parse_unified(&input(&[
        "diff --git a/old.txt b/old.txt",
        "deleted file mode 100644",
        "index de98044..0000000",
        "--- a/old.txt",
        "+++ /dev/null",
        "@@ -1,2 +0,0 @@",
        "-line one",
        "-line two",
    ]))
    .unwrap();
    let d = &diffs[0];
    assert_eq!(d.entry.path, b"old.txt");
    assert_eq!(d.entry.status, FileStatus::Deleted);
    assert_eq!(d.entry.added, Some(0));
    assert_eq!(d.entry.removed, Some(2));
}

#[test]
fn no_newline_at_eof_marks_the_line_before_it() {
    let diffs = parse_unified(&input(&[
        "diff --git a/nofinal.txt b/nofinal.txt",
        "index aaa..bbb 100644",
        "--- a/nofinal.txt",
        "+++ b/nofinal.txt",
        "@@ -1 +1 @@",
        "-old",
        "\\ No newline at end of file",
        "+new",
        "\\ No newline at end of file",
    ]))
    .unwrap();
    let lines = &diffs[0].hunks[0].lines;
    assert_eq!(lines.len(), 2);
    assert!(lines[0].no_newline_at_eof, "removed line: {lines:?}");
    assert!(lines[1].no_newline_at_eof, "added line: {lines:?}");
}

#[test]
fn crlf_content_keeps_its_carriage_return() {
    let mut bytes = input(&[
        "diff --git a/crlf.txt b/crlf.txt",
        "index aaa..bbb 100644",
        "--- a/crlf.txt",
        "+++ b/crlf.txt",
        "@@ -1 +1 @@",
    ]);
    bytes.extend_from_slice(b"-old\r\n+new\r\n");
    let diffs = parse_unified(&bytes).unwrap();
    let lines = &diffs[0].hunks[0].lines;
    assert_eq!(lines[0].text, b"old\r");
    assert_eq!(lines[1].text, b"new\r");
}

#[test]
fn quoted_non_utf8_path_unquotes_to_the_right_bytes() {
    let diffs = parse_unified(&input(&[
        "diff --git \"a/\\377\" \"b/\\377\"",
        "new file mode 100644",
        "index 0000000..abc1234",
        "--- /dev/null",
        "+++ \"b/\\377\"",
        "@@ -0,0 +1 @@",
        "+x",
    ]))
    .unwrap();
    assert_eq!(diffs[0].entry.path, vec![0xFF]);
}

#[test]
fn hunk_heading_may_contain_at_at() {
    let diffs = parse_unified(&input(&[
        "diff --git a/f.rs b/f.rs",
        "index aaa..bbb 100644",
        "--- a/f.rs",
        "+++ b/f.rs",
        "@@ -1,2 +1,2 @@ fn main() { // @@ marker",
        " a",
        "-b",
        "+c",
    ]))
    .unwrap();
    assert_eq!(diffs[0].hunks[0].heading, b"fn main() { // @@ marker");
}

#[test]
fn two_files_in_one_stream() {
    let diffs = parse_unified(&input(&[
        "diff --git a/one.txt b/one.txt",
        "index aaa..bbb 100644",
        "--- a/one.txt",
        "+++ b/one.txt",
        "@@ -1 +1 @@",
        "-a",
        "+b",
        "diff --git a/two.txt b/two.txt",
        "index ccc..ddd 100644",
        "--- a/two.txt",
        "+++ b/two.txt",
        "@@ -1 +1 @@",
        "-c",
        "+d",
    ]))
    .unwrap();
    assert_eq!(diffs.len(), 2);
    assert_eq!(diffs[0].entry.path, b"one.txt");
    assert_eq!(diffs[1].entry.path, b"two.txt");
}

#[test]
fn hunk_line_count_mismatch_is_a_parse_error() {
    let err = parse_unified(&input(&[
        "diff --git a/f.txt b/f.txt",
        "index aaa..bbb 100644",
        "--- a/f.txt",
        "+++ b/f.txt",
        "@@ -1,2 +1,2 @@",
        " a",
    ]))
    .unwrap_err();
    assert_eq!(
        err.what,
        "hunk ended before its declared line counts were reached"
    );
    assert_eq!(err.line, 6);
}
