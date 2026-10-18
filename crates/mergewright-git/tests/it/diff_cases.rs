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

use mergewright_git::diff::{BinaryNote, FileStatus, parse_unified};

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

// --- What the parser must refuse rather than report as an empty success
// (m5-history-and-ghost-preview/22). Every line below that is skipped has to
// be one named case; anything else is an error naming its line.

/// Copied from what `git diff-index -p HEAD -- conflicted.txt` prints while
/// the path is unmerged: the combined format, with no `diff --git` line
/// anywhere in it. Before this ticket the whole reply fell through the
/// top-level skip and came back as `Ok(vec![])`.
const COMBINED_REPLY: &[&str] = &[
    "diff --cc conflicted.txt",
    "index 0f0d4b1,7b8d2c5..0000000",
    "--- a/conflicted.txt",
    "+++ b/conflicted.txt",
    "@@@ -1,1 -1,1 +1,5 @@@",
    "++<<<<<<< HEAD",
    " +ours",
    "++=======",
    "+ theirs",
    "++>>>>>>> other",
];

#[test]
fn a_combined_diff_is_an_error_not_an_empty_success() {
    let err = parse_unified(&input(COMBINED_REPLY)).unwrap_err();
    assert_eq!(
        err.what,
        "combined diff (`diff --cc`) is not supported here"
    );
    assert_eq!(err.line, 1);
}

#[test]
fn a_combined_section_after_a_normal_file_is_an_error_too() {
    let mut lines = vec![
        "diff --git a/one.txt b/one.txt",
        "index aaa..bbb 100644",
        "--- a/one.txt",
        "+++ b/one.txt",
        "@@ -1 +1 @@",
        "-a",
        "+b",
    ];
    lines.extend_from_slice(COMBINED_REPLY);
    let err = parse_unified(&input(&lines)).unwrap_err();
    assert_eq!(
        err.what,
        "combined diff (`diff --cc`) is not supported here"
    );
    assert_eq!(err.line, 8);
}

#[test]
fn an_unreadable_line_before_the_first_file_is_an_error() {
    let err = parse_unified(&input(&[
        "fatal: something we have no grammar for",
        "diff --git a/f.txt b/f.txt",
        "index aaa..bbb 100644",
        "--- a/f.txt",
        "+++ b/f.txt",
        "@@ -1 +1 @@",
        "-a",
        "+b",
    ]))
    .unwrap_err();
    assert_eq!(err.what, "unreadable line outside any `diff --git` section");
    assert_eq!(err.line, 1);
}

#[test]
fn the_diff_tree_commit_id_line_is_still_skipped_and_the_diff_still_parses() {
    for id in [
        "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678",
        "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678a1b2c3d4e5f60718293a4b5c",
    ] {
        let diffs = parse_unified(&input(&[
            id,
            "diff --git a/f.txt b/f.txt",
            "index aaa..bbb 100644",
            "--- a/f.txt",
            "+++ b/f.txt",
            "@@ -1 +1 @@",
            "-a",
            "+b",
        ]))
        .unwrap_or_else(|why| panic!("{id}: {why:?}"));
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].entry.path, b"f.txt");
    }
    // A commit whose diff is empty (a merge, shown without `-c`) is the one
    // input that legitimately parses to nothing at all.
    assert!(
        parse_unified(&input(&["a1b2c3d4e5f60718293a4b5c6d7e8f9012345678"]))
            .unwrap()
            .is_empty()
    );
}

mod properties {
    use mergewright_git::diff::parse_unified;
    use proptest::prelude::*;

    use super::input;

    /// The case count for whichever tier this was compiled for: 64 in the
    /// fast tier, 1000 with `--features heavy-tests` (PROMPT.md §7.1).
    fn tier() -> ProptestConfig {
        ProptestConfig::with_cases(mergewright_fixtures::proptest_cases())
    }

    /// The test's own statement of the only line the parser may skip, kept
    /// independent of the parser's: a bare object id, 40 hex digits (sha1)
    /// or 64 (sha256).
    ///
    /// **This mirrors `diff::unified::is_bare_object_id` on purpose, and
    /// that is also its limit** (M5 review Minor 3). A property whose
    /// predicate is a re-implementation of the code's agrees with the code
    /// by construction: if both are wrong the same way, this stays green.
    /// What it does catch is the shape — `Ok(vec![])` for an input the
    /// parser did not understand — over inputs nobody wrote by hand.
    ///
    /// The real expected strings live in the four hand-written cases above
    /// this module, and **they are what must not be "simplified" away into
    /// this property**: `a_combined_diff_is_an_error_not_an_empty_success`
    /// and `a_combined_section_after_a_normal_file_is_an_error_too` pin
    /// `"combined diff (`diff --cc`) is not supported here"` with its line
    /// number, `an_unreadable_line_before_the_first_file_is_an_error` pins
    /// `"unreadable line outside any `diff --git` section"`, and
    /// `the_diff_tree_commit_id_line_is_still_skipped_and_the_diff_still_parses`
    /// pins the one skip at both id lengths. Delete those and nothing
    /// checks the wording the user reads.
    fn is_a_bare_object_id(line: &str) -> bool {
        matches!(line.len(), 40 | 64) && line.bytes().all(|b| b.is_ascii_hexdigit())
    }

    /// Lines from every family a reply can be built out of: ones the parser
    /// reads, ones it must refuse, the id it may skip, and noise.
    fn line() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("diff --git a/f.txt b/f.txt"),
            Just("index aaa..bbb 100644"),
            Just("--- a/f.txt"),
            Just("+++ b/f.txt"),
            Just("@@ -1 +1 @@"),
            Just("-a"),
            Just("+b"),
            Just("diff --cc f.txt"),
            Just("diff --combined f.txt"),
            Just("@@@ -1,1 -1,1 +1,5 @@@"),
            Just("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678"),
            Just("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678a1b2c3d4e5f60718293a4b5c"),
            Just("fatal: not a git repository"),
            Just("warning: LF will be replaced by CRLF"),
            Just(""),
            Just("Binary files a/f.bin and b/f.bin differ"),
        ]
        .prop_map(str::to_owned)
    }

    proptest! {
        #![proptest_config(tier())]

        /// The property the `diff --cc` case is one instance of:
        /// `parse_unified` never reports success with nothing in it for an
        /// input it did not understand. An empty `Ok` is only honest when
        /// every line was one the parser explicitly recognises as benign.
        #[test]
        fn an_empty_ok_means_every_line_was_a_bare_object_id(
            lines in prop::collection::vec(line(), 1..8),
        ) {
            let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
            let bytes = input(&refs);
            if let Ok(diffs) = parse_unified(&bytes) {
                prop_assert!(
                    !diffs.is_empty() || lines.iter().all(|l| is_a_bare_object_id(l)),
                    "Ok(vec![]) for an input it did not understand: {refs:?}",
                );
            }
        }
    }
}
