//! The trace parsers on git's own bytes, and `map_to_head` on real hunks.
//! The 2.54 samples are `git log -L18,29:src/auth/session.rs` and `git log
//! --follow --raw --numstat -z` on the `session` fixture, captured by git
//! 2.54.0; the 2.38 samples are written by hand to the formats 2.38
//! documents (no 2.38 binary on the build machine), and cover what the
//! captures do not: a rename inside `-L`, a quoted path, a binary numstat.

use std::path::PathBuf;

use mergewright_core::oid::Oid;

use super::function::{blob_content, enclosing_function_with, marked_diff};
use super::log::{NO_ADD, finish, parse_line_records};
use super::{TraceEnd, TraceError, map_to_head};
use super::{parse_follow_log, parse_line_log};
use crate::diff::parse_unified;
use crate::repo::RepoHandle;
use crate::run::{DEFAULT_CAP, GitOutput, RawOutput};

const LINE_LOG_2_54: &[u8] = include_bytes!("../testdata/git-2.54/line-log.txt");
const FOLLOW_LOG_2_54: &[u8] = include_bytes!("../testdata/git-2.54/follow-log.txt");

const LINE_LOG_2_38: &[u8] =
    b"\x011111111111111111111111111111111111111111\0Tom\x001552089600\0Move it\n\
\n\
diff --git \"a/\\303\\251ld.rs\" \"b/n\\303\\251w.rs\"\n\
--- \"a/\\303\\251ld.rs\"\n\
+++ \"b/n\\303\\251w.rs\"\n\
@@ -3,2 +5,3 @@ fn f() {\n\
\x20    let a = 1;\n\
-    let b = 2;\n\
+    let b = 3;\n\
+    let c = 4;\n\
\x012222222222222222222222222222222222222222\0Ana\x001541721600\0Add f\n\
\n\
diff --git a/old.rs b/old.rs\n\
new file mode 100644\n\
--- /dev/null\n\
+++ b/old.rs\n\
@@ -0,0 +3,2 @@\n\
+    let a = 1;\n\
+    let b = 2;\n";

const FOLLOW_LOG_2_38: &[u8] =
    b"\x011111111111111111111111111111111111111111\0Tom\x001552089600\0Swap the picture\0\n\
:100644 100644 aaaaaaa bbbbbbb R100\0pic.png\0img/pic.png\0-\t-\t\0pic.png\0img/pic.png\0\
\x012222222222222222222222222222222222222222\0Ana\x001541721600\0Add the picture\0\n\
:000000 100644 0000000 aaaaaaa A\0pic.png\0-\t-\tpic.png\0";

/// Author, added, removed, `range_at`.
type Summary<'a> = (&'a str, u32, u32, Option<(u32, u32)>);

#[test]
fn the_2_54_line_log_reads_ranges_and_counts() {
    let rows = parse_line_log(LINE_LOG_2_54).expect("git 2.54's -L output parses");
    let summary: Vec<Summary<'_>> = rows
        .iter()
        .map(|row| (row.author.as_str(), row.added, row.removed, row.range_at))
        .collect();
    assert_eq!(
        summary,
        [
            ("Fixture Maker", 1, 1, Some((18, 29))),
            ("Lina", 3, 2, Some((17, 28))),
            ("Tom", 11, 0, Some((17, 27))),
        ]
    );
    assert_eq!(rows[0].path, PathBuf::from("src/auth/session.rs"));
    assert_eq!(rows[1].path, PathBuf::from("src/session.rs"));
    assert_eq!(rows[1].subject, "Retry refresh once on 401");
    assert_eq!(rows[1].when, 1_559_865_600);
    assert!(rows.iter().all(|row| row.renamed_from.is_none()));
}

#[test]
fn the_2_38_line_log_reads_a_rename_and_an_added_range() {
    let rows = parse_line_log(LINE_LOG_2_38).expect("the 2.38-shaped sample parses");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].path,
        PathBuf::from("n\u{e9}w.rs"),
        "quoted path undone"
    );
    assert_eq!(rows[0].renamed_from, Some(PathBuf::from("\u{e9}ld.rs")));
    assert_eq!((rows[0].added, rows[0].removed), (2, 1));
    assert_eq!(rows[0].range_at, Some((5, 7)));
    assert_eq!(rows[1].range_at, Some((3, 4)));
    assert_eq!((rows[1].added, rows[1].removed), (2, 0));
}

#[test]
fn the_2_54_follow_log_reads_the_rename() {
    let rows = parse_follow_log(FOLLOW_LOG_2_54).expect("git 2.54's --follow output parses");
    let summary: Vec<(&str, u32, u32)> = rows
        .iter()
        .map(|row| (row.subject.as_str(), row.added, row.removed))
        .collect();
    assert_eq!(
        summary,
        [
            ("Handle expired tokens", 2, 1),
            ("Move session TTL to config", 6, 6),
            ("rustfmt", 6, 2),
            ("Retry refresh once on 401", 3, 2),
            ("Add token refresh", 12, 0),
            ("Add session", 42, 0),
        ]
    );
    assert_eq!(rows[1].path, PathBuf::from("src/auth/session.rs"));
    assert_eq!(rows[1].renamed_from, Some(PathBuf::from("src/session.rs")));
    assert_eq!(rows[2].path, PathBuf::from("src/session.rs"));
    assert!(rows.iter().all(|row| row.range_at.is_none()));
}

#[test]
fn the_2_38_follow_log_reads_a_binary_rename() {
    let rows = parse_follow_log(FOLLOW_LOG_2_38).expect("the 2.38-shaped sample parses");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].path, PathBuf::from("img/pic.png"));
    assert_eq!(rows[0].renamed_from, Some(PathBuf::from("pic.png")));
    assert_eq!(
        (rows[0].added, rows[0].removed),
        (0, 0),
        "binary counts as 0"
    );
    assert_eq!(rows[1].path, PathBuf::from("pic.png"));
}

#[test]
fn garbage_is_a_parse_error_not_a_panic() {
    assert!(parse_line_log(b"not a log").is_err());
    assert!(parse_follow_log(b"not a log").is_err());
    assert!(parse_line_log(b"\x01zz\0a\x001\0s\n").is_err());
    assert!(parse_follow_log(b"\x011111111111111111111111111111111111111111\0a\0x\0s\0").is_err());
    assert_eq!(parse_line_log(b"").map(|rows| rows.len()), Ok(0));
    let cut = &LINE_LOG_2_54[..LINE_LOG_2_54.len() / 2];
    // Cut mid-record: rows or an error, never a panic.
    if let Ok(rows) = parse_line_log(cut) {
        assert!(rows.len() <= 3);
    }
}

/// Worktree-vs-`HEAD` hunks: `HEAD` has lines 1–10; the working tree adds
/// two lines after 2, removes 5–6, and changes 9 (its line 9 too).
const WORKTREE: &[u8] = b"diff --git a/f b/f\n\
--- a/f\n\
+++ b/f\n\
@@ -2,0 +3,2 @@\n\
+new one\n\
+new two\n\
@@ -5,2 +6,0 @@\n\
-five\n\
-six\n\
@@ -9 +9 @@\n\
-nine\n\
+NINE\n";

#[test]
fn map_to_head_skips_added_lines() {
    let hunks = &parse_unified(WORKTREE).expect("parses")[0].hunks;
    assert_eq!(map_to_head(hunks, 1), Some(1));
    assert_eq!(map_to_head(hunks, 2), Some(2));
    assert_eq!(map_to_head(hunks, 3), None, "added");
    assert_eq!(map_to_head(hunks, 4), None, "added");
    assert_eq!(map_to_head(hunks, 5), Some(3), "shifted by the two adds");
    assert_eq!(map_to_head(hunks, 8), Some(8));
    assert_eq!(
        map_to_head(hunks, 9),
        None,
        "a changed line is an added one"
    );
}

#[test]
fn map_to_head_shifts_past_a_removal() {
    let hunks = &parse_unified(WORKTREE).expect("parses")[0].hunks;
    assert_eq!(map_to_head(hunks, 6), Some(4), "just before the removal");
    assert_eq!(map_to_head(hunks, 7), Some(7), "after it: +2 −2");
    assert_eq!(map_to_head(hunks, 10), Some(10), "past every hunk");
    assert_eq!(map_to_head(&[], 4), Some(4), "no changes");
}

/// Review 18/19/27 Minor 6 (mutation M17): a log whose oldest commit only
/// changed the range, not added it, ends as "git stops", not as a start.
#[test]
fn a_log_ending_on_a_change_not_an_add_is_not_a_start() {
    // The 2.38 sample's first record alone: `@@ -3,2 +5,3 @@`, a change.
    let cut = LINE_LOG_2_38
        .iter()
        .skip(1)
        .position(|&b| b == 0x01)
        .expect("two records");
    let raw = RawOutput {
        output: GitOutput {
            stdout: LINE_LOG_2_38[..=cut].to_vec(),
            stderr: Vec::new(),
            truncated: false,
        },
        code: Some(0),
    };
    let mut rows = Vec::new();
    let end = finish(
        Ok(raw),
        1000,
        &mut |batch| rows.extend(batch),
        parse_line_records,
        "none",
        &|| Ok(std::collections::BTreeSet::new()),
    )
    .expect("finishes");
    assert_eq!(rows.len(), 1);
    assert_eq!(end, TraceEnd::GitStops(NO_ADD.to_owned()));
}

/// Review Minor 4: a blob or a marked diff cut at the runner's cap is
/// refused as too large — not read short, and not reported as a stop.
#[test]
fn enclosing_function_refuses_output_cut_at_the_cap() {
    let cut = GitOutput {
        stdout: b"fn a() {\n".to_vec(),
        stderr: Vec::new(),
        truncated: true,
    };
    match blob_content(cut.clone(), 9) {
        Err(TraceError::TooLarge { cap }) => assert_eq!(cap, 9),
        other => panic!("a cut blob must be refused: {other:?}"),
    }
    // The runner reports its own kill at the cap as `code: None`.
    let capped = RawOutput {
        output: cut,
        code: None,
    };
    match marked_diff(capped, 9) {
        Err(TraceError::TooLarge { cap }) => assert_eq!(cap, 9),
        other => panic!("a capped diff is too large, not stopped: {other:?}"),
    }
}

/// A one-commit repository holding `a.rs` with `content`; its `HEAD`.
fn one_file_repo(content: &str) -> (tempfile::TempDir, Oid) {
    let temp = tempfile::tempdir().expect("a temp dir");
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(temp.path())
            .env("GIT_AUTHOR_NAME", "Test Maker")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Test Maker")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .output()
            .expect("git runs");
        assert!(output.status.success(), "git {args:?}");
        output.stdout
    };
    git(&["init", "-q"]);
    std::fs::write(temp.path().join("a.rs"), content).expect("writes");
    git(&["add", "a.rs"]);
    git(&["commit", "-q", "-m", "add"]);
    let head =
        Oid::from_hex(String::from_utf8_lossy(&git(&["rev-parse", "HEAD"])).trim()).expect("an id");
    (temp, head)
}

fn function_capped(content: &str, cap: usize) -> Result<Option<super::LineRange>, TraceError> {
    let (temp, head) = one_file_repo(content);
    let repo = RepoHandle::open(temp.path()).expect("opens");
    enclosing_function_with(&repo, head, std::path::Path::new("a.rs"), 1, &(), cap)
}

/// Review 18bcd Minor 3: each cap guard is reached through the real spawns.
/// A file of one long line: the blob passes the cap, the marked diff (one
/// short hunk) would not. Replacing the `blob_content(…)` call in
/// `enclosing_function_with` by the run's bare `.stdout` turns this red.
#[test]
fn enclosing_function_refuses_a_blob_over_the_cap() {
    let long_line = format!("fn a() {{ {} }}\n", "x".repeat(5000));
    match function_capped(&long_line, 1000) {
        Err(TraceError::TooLarge { cap }) => assert_eq!(cap, 1000),
        other => panic!("a capped blob must not read as a function: {other:?}"),
    }
}

/// Many short lines: the blob fits, the marked diff (a hunk per line) does
/// not. Replacing `marked_diff(raw, cap)?` in `headers_by_line` by the raw
/// stdout turns this red.
#[test]
fn enclosing_function_refuses_a_marked_diff_over_the_cap() {
    let short_lines = "x\n".repeat(200);
    match function_capped(&short_lines, 1000) {
        Err(TraceError::TooLarge { cap }) => assert_eq!(cap, 1000),
        other => panic!("a capped diff must not read as headers: {other:?}"),
    }
}

/// Review 18bcd Minor 2: a log cut at the runner's cap is too large, in its
/// own words, not a parse failure.
#[test]
fn a_log_cut_at_the_cap_is_too_large() {
    let raw = RawOutput {
        output: GitOutput {
            stdout: LINE_LOG_2_54[..100].to_vec(),
            stderr: Vec::new(),
            truncated: true,
        },
        code: None,
    };
    let error = finish(
        Ok(raw),
        1000,
        &mut |_| {},
        parse_line_records,
        "none",
        &|| Ok(std::collections::BTreeSet::new()),
    )
    .expect_err("refused");
    assert!(
        matches!(error, TraceError::TooLarge { cap } if cap == DEFAULT_CAP),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "too large to trace: git's output is over 64 MiB"
    );
}
