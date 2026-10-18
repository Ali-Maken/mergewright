use mergewright_term::sanitize::sanitize_line;

use super::*;

fn row(depth: u8, name: &str, kind: FileRowKind, entry: Option<u32>) -> FileRow {
    FileRow {
        depth,
        name: sanitize_line(name),
        kind,
        entry,
    }
}

fn file() -> FileRowKind {
    FileRowKind::File {
        status: FileStatusView::Modified,
        added: Some(1),
        removed: Some(1),
    }
}

fn tree() -> FileListView {
    FileListView {
        rows: vec![
            row(0, "a/", FileRowKind::Dir { collapsed: false }, None),
            row(1, "b/", FileRowKind::Dir { collapsed: false }, None),
            row(2, "c.rs", file(), Some(0)),
            row(1, "d.rs", file(), Some(1)),
            row(0, "e.rs", file(), Some(2)),
        ],
        ..FileListView::default()
    }
}

fn visible_names(list: &FileListView) -> Vec<&str> {
    list.visible().map(|(_, row)| row.name.as_str()).collect()
}

#[test]
fn folding_a_directory_hides_only_its_children() {
    let mut list = tree();
    list.toggle_dir(1);
    assert_eq!(visible_names(&list), ["a/", "b/", "d.rs", "e.rs"]);
    list.toggle_dir(0);
    assert_eq!(visible_names(&list), ["a/", "e.rs"]);
}

#[test]
fn the_cursor_skips_folded_rows_and_stops_at_the_ends() {
    let mut list = tree();
    list.toggle_dir(1);
    list.move_cursor(2);
    assert_eq!(list.cursor, 3);
    assert_eq!(list.selected_entry(), Some(1));
    list.move_cursor(10);
    assert_eq!(list.cursor, 4);
    list.move_cursor(-10);
    assert_eq!(list.cursor, 0);
    assert_eq!(list.selected_entry(), None);
}

#[test]
fn folding_over_the_cursor_moves_it_to_the_directory() {
    let mut list = tree();
    list.cursor = 2;
    list.toggle_dir(1);
    assert_eq!(list.cursor, 1);
}
