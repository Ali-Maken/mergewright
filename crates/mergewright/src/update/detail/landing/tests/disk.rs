//! `disk_line_in`: where a line of the file on disk lives, read off the
//! Unstaged diff of a partly staged file (m7-power-tools/20m).

use mergewright_git::diff::{DiffLine, FileDiff, FileEntry, FileStatus, Hunk, LineKind, Truncated};

use super::super::{DiskLine, disk_line_in};

/// A hunk with its lines, numbered from `old_start`/`new_start` as git
/// writes them: `' '` context, `'+'` added, `'-'` removed.
fn hunk(old_start: u32, new_start: u32, marks: &str) -> Hunk {
    let (mut old, mut new) = (old_start, new_start);
    let mut lines = Vec::new();
    for mark in marks.chars() {
        let (kind, old_no, new_no) = match mark {
            '+' => (LineKind::Added, None, Some(new)),
            '-' => (LineKind::Removed, Some(old), None),
            _ => (LineKind::Context, Some(old), Some(new)),
        };
        if old_no.is_some() {
            old += 1;
        }
        if new_no.is_some() {
            new += 1;
        }
        lines.push(DiffLine {
            kind,
            text: Vec::new(),
            old_no,
            new_no,
            no_newline_at_eof: false,
        });
    }
    Hunk {
        old_start,
        old_lines: old - old_start,
        new_start,
        new_lines: new - new_start,
        heading: Vec::new(),
        lines,
    }
}

fn diff(hunks: Vec<Hunk>) -> FileDiff {
    FileDiff {
        entry: FileEntry {
            path: b"f.txt".to_vec(),
            old_path: None,
            status: FileStatus::Modified,
            added: Some(1),
            removed: Some(1),
        },
        hunks,
        binary: None,
        truncated: None,
        old_mode: None,
        new_mode: None,
    }
}

/// The ticket's fixture, index → disk: `U` inserted above index line 3,
/// index line 6 (`l6`) → `U6`, one hunk at git's three lines of context.
fn fixture() -> FileDiff {
    // Index lines 1–9 against disk 1–10: ` ` l1 l2, `+` U, ` ` l3 l4 l5,
    // `-` l6, `+` U6, ` ` l7 S8 l9.
    diff(vec![hunk(1, 1, "  +   -+   ")])
}

#[test]
fn disk_line_in_an_added_line_is_unstaged() {
    assert_eq!(disk_line_in(&fixture(), 3), DiskLine::Unstaged(3));
    assert_eq!(disk_line_in(&fixture(), 7), DiskLine::Unstaged(7));
}

#[test]
fn disk_line_in_a_context_line_is_its_index_number() {
    // Disk 9 (`S8`) is index 8; disk 2 (`l2`) is index 2.
    assert_eq!(disk_line_in(&fixture(), 9), DiskLine::Index(8));
    assert_eq!(disk_line_in(&fixture(), 2), DiskLine::Index(2));
}

#[test]
fn disk_line_in_above_every_hunk_is_unshifted() {
    let diff = diff(vec![hunk(20, 20, "   +   ")]);
    assert_eq!(disk_line_in(&diff, 5), DiskLine::Index(5));
}

#[test]
fn disk_line_in_below_the_hunks_shifts_by_them() {
    // +1 at 20, −2 at 40: disk 60 is index 61.
    let diff = diff(vec![hunk(20, 20, "   +   "), hunk(40, 41, "   --   ")]);
    assert_eq!(disk_line_in(&diff, 60), DiskLine::Index(61));
}

#[test]
fn disk_line_in_between_hunks_shifts_by_those_above() {
    let diff = diff(vec![hunk(20, 20, "   +   "), hunk(40, 41, "   --   ")]);
    assert_eq!(disk_line_in(&diff, 30), DiskLine::Index(29));
}

/// git writes a deletion of index 5–6 as `@@ -5,2 +4,0 @@`: its end is the
/// disk line before it, which is unshifted; the line after is shifted.
#[test]
fn disk_line_in_a_pure_deletion_shifts_only_what_follows() {
    let deletion = Hunk {
        old_start: 5,
        old_lines: 2,
        new_start: 4,
        new_lines: 0,
        heading: Vec::new(),
        lines: vec![
            DiffLine {
                kind: LineKind::Removed,
                text: Vec::new(),
                old_no: Some(5),
                new_no: None,
                no_newline_at_eof: false,
            },
            DiffLine {
                kind: LineKind::Removed,
                text: Vec::new(),
                old_no: Some(6),
                new_no: None,
                no_newline_at_eof: false,
            },
        ],
    };
    let diff = diff(vec![deletion]);
    assert_eq!(disk_line_in(&diff, 4), DiskLine::Index(4));
    assert_eq!(disk_line_in(&diff, 5), DiskLine::Index(7));
}

/// `@@ -0,0 +1,3 @@`: three lines added at the top.
#[test]
fn disk_line_in_a_pure_add_at_line_1() {
    let diff = diff(vec![hunk(0, 1, "+++")]);
    assert_eq!(disk_line_in(&diff, 1), DiskLine::Unstaged(1));
    assert_eq!(disk_line_in(&diff, 3), DiskLine::Unstaged(3));
    assert_eq!(disk_line_in(&diff, 4), DiskLine::Index(1));
}

#[test]
fn disk_line_in_a_truncated_diff_below_what_it_shows_is_unstaged() {
    let mut cut = diff(vec![hunk(20, 20, "   +   ")]);
    cut.truncated = Some(Truncated {
        shown_lines: 7,
        total_lines: 90,
    });
    assert_eq!(disk_line_in(&cut, 50), DiskLine::Unstaged(50));
    // Above what it shows, it still reads.
    assert_eq!(disk_line_in(&cut, 5), DiskLine::Index(5));
}
