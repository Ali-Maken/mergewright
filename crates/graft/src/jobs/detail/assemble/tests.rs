use graft_git::diff::{DiffLine, FileEntry, Hunk};

use super::super::body_of;
use super::*;

fn entry(path: &str) -> FileEntry {
    FileEntry {
        path: path.into(),
        old_path: None,
        status: FileStatus::Modified,
        added: Some(1),
        removed: Some(1),
    }
}

fn names(view: &FileListView) -> Vec<(&str, u8)> {
    view.rows
        .iter()
        .map(|row| (row.name.as_str(), row.depth))
        .collect()
}

#[test]
fn files_become_a_tree_with_directories_before_their_children() {
    let list = FileList {
        files: vec![entry("e.rs"), entry("a/b/c.rs"), entry("a/d.rs")],
        added: 3,
        removed: 3,
    };
    let view = assemble_files(&list, None);
    assert_eq!(
        names(&view),
        [("a/", 0), ("b/", 1), ("c.rs", 2), ("d.rs", 1), ("e.rs", 0)]
    );
    assert_eq!(view.rows[2].entry, Some(1));
    let mut folded = view.clone();
    folded.toggle_dir(1);
    assert_eq!(
        folded
            .visible()
            .map(|(_, r)| r.name.as_str())
            .collect::<Vec<_>>(),
        ["a/", "b/", "d.rs", "e.rs"]
    );
}

#[test]
fn a_body_with_prose_leads_with_a_message_row_and_trailers_alone_do_not() {
    let list = FileList {
        files: vec![entry("e.rs")],
        added: 1,
        removed: 1,
    };
    let view = assemble_files(&list, Some(b"First line.\nSecond line.\n"));
    assert_eq!(view.rows[0].kind, FileRowKind::Message { lines: 2 });
    assert_eq!(view.message.len(), 2);
    let trailers = assemble_files(&list, Some(b"Signed-off-by: A <a@b>\n"));
    assert_eq!(names(&trailers), [("e.rs", 0)]);
    assert_eq!(body_of(b"Subject\n\nBody\n"), b"Body\n");
}

fn line(kind: LineKind, text: &[u8], old_no: Option<u32>, new_no: Option<u32>) -> DiffLine {
    DiffLine {
        kind,
        text: text.to_vec(),
        old_no,
        new_no,
        no_newline_at_eof: false,
    }
}

#[test]
fn a_paired_hunk_gets_a_heading_pairs_words_syntax_and_safe_text() {
    let diff = FileDiff {
        entry: entry("src/lib.rs"),
        hunks: vec![Hunk {
            old_start: 1,
            old_lines: 2,
            new_start: 1,
            new_lines: 2,
            heading: b"fn main()".to_vec(),
            lines: vec![
                line(LineKind::Removed, b"let x = 1;\x1b[31m", Some(1), None),
                line(LineKind::Removed, b"let y = 2;\r", Some(2), None),
                line(LineKind::Added, b"let x = 5;", None, Some(1)),
                line(LineKind::Added, b"let y = 2; // same\r", None, Some(2)),
            ],
        }],
        binary: None,
        truncated: None,
        old_mode: None,
        new_mode: None,
    };
    let view = assemble_diff(&diff, true);
    let lines = &view.hunks[0].lines;
    assert_eq!(lines[0].kind, LineKindView::HunkHeading);
    assert_eq!(lines[1].pair, Some(3));
    assert_eq!(lines[3].pair, Some(1));
    assert!(!lines[3].words.is_empty(), "the changed number is tinted");
    assert!(
        lines[1].syntax.iter().any(|s| !s.range.is_empty()),
        "rust is highlighted"
    );
    assert!(
        !lines[1].text.as_str().contains('\x1b'),
        "ESC never reaches the view"
    );
    assert!(
        lines[2].text.as_str().ends_with('\u{240D}'),
        "CRLF shows as ␍"
    );
    assert!(!view.untracked);
}
