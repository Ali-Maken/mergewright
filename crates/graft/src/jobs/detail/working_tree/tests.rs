use graft_ui::detail::WorkingTreeSection::{Staged, Unmerged, Unstaged, Untracked};

use super::*;

fn file(path: &str, status: FileStatus) -> FileEntry {
    FileEntry {
        path: path.into(),
        old_path: None,
        status,
        added: Some(1),
        removed: Some(1),
    }
}

fn list(files: Vec<FileEntry>) -> FileList {
    let added = files.len() as u64;
    FileList {
        files,
        added,
        removed: added,
    }
}

/// Every row as `(name, depth, section)` — the shape the pane draws.
fn rows(view: &FileListView) -> Vec<(&str, u8, Option<WorkingTreeSection>)> {
    view.rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.name.as_str(), row.depth, view.section_at(index)))
        .collect()
}

#[test]
fn three_sections_count_their_own_files_and_a_shared_path_is_in_two() {
    // `both.rs` is staged *and* changed again since: two changes, two rows.
    let staged = list(vec![
        file("both.rs", FileStatus::Modified),
        file("s1.rs", FileStatus::Added),
    ]);
    let unstaged = list(vec![
        file("both.rs", FileStatus::Modified),
        file("u1.rs", FileStatus::Modified),
        file("new.txt", FileStatus::Untracked),
    ]);
    let (view, merged) = assemble_working_tree(&staged, &unstaged, &UnmergedShapes::new());
    assert_eq!(
        rows(&view),
        vec![
            ("STAGED", 0, Some(Staged)),
            ("both.rs", 1, Some(Staged)),
            ("s1.rs", 1, Some(Staged)),
            ("UNSTAGED", 0, Some(Unstaged)),
            ("both.rs", 1, Some(Unstaged)),
            ("u1.rs", 1, Some(Unstaged)),
            ("UNTRACKED", 0, Some(Untracked)),
            ("new.txt", 1, Some(Untracked)),
        ]
    );
    let counts: Vec<usize> = view
        .rows
        .iter()
        .filter_map(|row| match row.kind {
            FileRowKind::Section { count, .. } => Some(count),
            _ => None,
        })
        .collect();
    assert_eq!(counts, vec![2, 2, 1]);
    assert_eq!(merged.files.len(), 5, "every row indexes its own entry");
    // The cursor opens on a file, never on a heading.
    assert_eq!(view.cursor, 1);
    assert_eq!(view.selected_section(), Some(Staged));
}

#[test]
fn an_untracked_file_is_a_row_of_its_own_section_not_an_unstaged_one() {
    let unstaged = list(vec![file("new.txt", FileStatus::Untracked)]);
    let (view, _) = assemble_working_tree(&list(Vec::new()), &unstaged, &UnmergedShapes::new());
    assert_eq!(
        rows(&view),
        vec![
            ("UNTRACKED", 0, Some(Untracked)),
            ("new.txt", 1, Some(Untracked))
        ]
    );
}

#[test]
fn an_empty_section_gets_no_heading_and_a_clean_tree_gets_no_rows() {
    let (view, merged) =
        assemble_working_tree(&list(Vec::new()), &list(Vec::new()), &UnmergedShapes::new());
    assert!(view.rows.is_empty(), "{:?}", view.rows);
    assert!(merged.files.is_empty());
}

#[test]
fn a_sections_entries_are_its_own_and_stop_at_the_next_heading() {
    let staged = list(vec![file("a.rs", FileStatus::Modified)]);
    let unstaged = list(vec![
        file("b.rs", FileStatus::Modified),
        file("c.rs", FileStatus::Modified),
    ]);
    let (view, _) = assemble_working_tree(&staged, &unstaged, &UnmergedShapes::new());
    assert_eq!(view.section_entries(0), vec![0]);
    assert_eq!(view.section_entries(2), vec![1, 2]);
    // A file row is not a section row: asking is safe and answers empty.
    assert!(view.section_entries(1).is_empty());
}

#[test]
fn files_nest_under_their_section_one_level_deeper() {
    let unstaged = list(vec![file("src/a/b.rs", FileStatus::Modified)]);
    let (view, _) = assemble_working_tree(&list(Vec::new()), &unstaged, &UnmergedShapes::new());
    assert_eq!(
        rows(&view),
        vec![
            ("UNSTAGED", 0, Some(Unstaged)),
            ("src/", 1, Some(Unstaged)),
            ("a/", 2, Some(Unstaged)),
            ("b.rs", 3, Some(Unstaged)),
        ]
    );
}

#[test]
fn a_staged_row_reads_the_index_and_every_other_row_the_working_tree() {
    assert_eq!(
        diff_spec_for(DetailSpec::WorkingTree, Some(Staged)),
        DiffSpec::Staged
    );
    assert_eq!(
        diff_spec_for(DetailSpec::WorkingTree, Some(Unstaged)),
        DiffSpec::Unstaged
    );
    assert_eq!(
        diff_spec_for(DetailSpec::WorkingTree, Some(Untracked)),
        DiffSpec::Unstaged
    );
    let commit =
        DetailSpec::Commit(graft_core::oid::Oid::from_bytes(&[7; 20]).expect("20 bytes is an oid"));
    assert_eq!(diff_spec_for(commit, None), commit.diff_spec());
}

/// m5/21: an unmerged path against the index has no two-way answer —
/// git replies with a combined diff and the body drew nothing. Against
/// `HEAD` it is an ordinary diff of the file as it stands.
/// `conflict_interim::an_unmerged_file_draws_its_conflict_in_the_body`
/// is the same fact read off a real stopped merge.
#[test]
fn a_conflicted_row_reads_against_head_not_the_index() {
    assert_eq!(
        diff_spec_for(DetailSpec::WorkingTree, Some(Unmerged)),
        DiffSpec::WorkingTreeVsHead
    );
}
