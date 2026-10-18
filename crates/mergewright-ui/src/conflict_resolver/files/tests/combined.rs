//! The column's `Combined by git` section (ease-pass/11b): its states, its
//! names packed and cut, and no click landing on it.

use super::*;

const COLUMN: Placement = Placement::Column { width: 30 };

fn with(combined: CombinedList) -> FileListView {
    FileListView {
        combined,
        ..two_files()
    }
}

fn names(items: &[&str]) -> CombinedList {
    CombinedList::Files(items.iter().map(|s| sanitize_line(s)).collect())
}

fn drawn(view: &FileListView, height: u16) -> Vec<String> {
    let buf = draw_list(view, COLUMN, 30, height);
    (0..height).map(|y| line(&buf, y)).collect()
}

/// `Combined by git (2)` one blank row under the files, the names after
/// `✓`, `, `-separated; rows 0..4 are the heading, blank and two files.
#[test]
fn two_combined_draw_heading_and_names() {
    let rows = drawn(&with(names(&["clean.txt", "new.txt"])), 10);
    assert_eq!(rows[4], "");
    assert_eq!(rows[5], "Combined by git (2)");
    assert_eq!(rows[6], "✓ clean.txt, new.txt");
    assert_eq!(rows[7], "");
}

#[test]
fn reading_and_failed_say_so() {
    let rows = drawn(&with(CombinedList::Reading), 10);
    assert_eq!(rows[5], "Combined by git · reading…");
    let failed = CombinedList::Failed(sanitize_line("git exited 128"));
    let rows = drawn(&with(failed), 10);
    assert_eq!(rows[5], "Combined by git · can't tell:");
    assert_eq!(rows[6], "  git exited 128");
}

/// Status rule: nothing combined, and any operation but a merge, draw no
/// section at all.
#[test]
fn none_combined_and_not_a_merge_draw_nothing() {
    let bare = drawn(&with(CombinedList::NotMerge), 10);
    assert_eq!(drawn(&with(names(&[])), 10), bare);
    assert!(bare.iter().all(|row| !row.contains("Combined")), "{bare:?}");
}

/// More names than rows: the rows that fit, then `… K more` on the last.
#[test]
fn more_names_than_rows_end_in_more() {
    let many: Vec<String> = (0..12).map(|i| format!("file{i:02}.rs")).collect();
    let refs: Vec<&str> = many.iter().map(String::as_str).collect();
    let rows = drawn(&with(names(&refs)), 9);
    assert_eq!(rows[5], "Combined by git (12)");
    assert_eq!(rows[6], "✓ file00.rs, file01.rs,");
    assert_eq!(rows[7], "  file02.rs, file03.rs,");
    assert_eq!(rows[8], "  … 8 more");
}

/// Information only: no click on the section picks a file or the review.
#[test]
fn the_section_is_not_clickable() {
    let view = with(names(&["clean.txt", "new.txt"]));
    for dy in 4..10 {
        for dx in 0..30 {
            assert_eq!(file_at(&view, COLUMN, 30, 10, dx, dy), None, "{dx},{dy}");
            assert!(!review_row_at(&view, COLUMN, 30, 10, dx, dy));
        }
    }
}

/// Under `✓ Review the merge  y` when the list ends with it.
#[test]
fn it_sits_under_the_review_row() {
    let view = FileListView {
        tail: ListTail::Review { key: "y".into() },
        ..with(names(&["clean.txt"]))
    };
    let rows = drawn(&view, 10);
    assert_eq!(rows[4], "✓ Review the merge  y");
    assert_eq!(rows[5], "");
    assert_eq!(rows[6], "Combined by git (1)");
    assert_eq!(rows[7], "✓ clean.txt");
}
