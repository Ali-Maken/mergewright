//! The trace parsers on git's own bytes, and `map_to_head` on real hunks.
//! The 2.54 samples are `git log -L18,29:src/auth/session.rs` and `git log
//! --follow --raw --numstat -z` on the `session` fixture, captured by git
//! 2.54.0; the 2.38 samples are written by hand to the formats 2.38
//! documents (no 2.38 binary on the build machine), and cover what the
//! captures do not: a rename inside `-L`, a quoted path, a binary numstat.

use std::path::PathBuf;

use super::{parse_follow_log, parse_line_log};
use super::map_to_head;
use crate::diff::parse_unified;

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
            ("Graft Fixture", 1, 1, Some((18, 29))),
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
