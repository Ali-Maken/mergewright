//! `show_line`'s `Selecting` stage (m7-power-tools/20m): a path inside a
//! folded directory (B), and a changes-pane section that has gone (A).

use std::path::Path;
use std::sync::Arc;

use graft_ui::detail::{FileRow, FileRowKind};

use crate::update::events::{AppEvent, Product};
use crate::update::state::Status;

use super::super::super::cursor::tests::{files_of, sample_diff};
use super::super::super::state::{DetailSpec, Loading};
use super::super::show_line;
use super::{commit, diff_answer, list_job, new_line, on_graph, step};

/// `src/` (row 0) holding `a.rs` (row 1), as the job answers it.
fn tree(id: graft_core::job::JobId) -> AppEvent {
    let (mut view, list) = files_of(&["src/a.rs"], None, false);
    view.rows[0].depth = 1;
    let mut dir = view.rows[0].clone();
    dir.depth = 0;
    dir.kind = FileRowKind::Dir { collapsed: false };
    dir.entry = None;
    view.rows.insert(0, FileRow { ..dir });
    view.cursor = 1;
    AppEvent::Produced(id, Product::Files(Ok((Arc::new(view), Arc::new(list)))))
}

/// A section asked for that the path no longer has a row in (the tree
/// changed from outside): the path's first row, at its first change.
#[test]
fn show_line_section_gone_lands_on_the_first_row_first_change() {
    use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
    use graft_ui::detail::WorkingTreeSection;

    let mut state = on_graph();
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: vec![StatusEntry {
            path: "src/x.rs".into(),
            original_path: None,
            staged: Some(Change::Modified),
            unstaged: None,
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }],
    }));
    let commands = show_line(
        &mut state,
        DetailSpec::WorkingTree,
        Path::new("src/x.rs"),
        Some(new_line(9)),
        Some(WorkingTreeSection::Unstaged),
    );
    let (view, list) = super::super::super::tests::changes_pane(&[(
        WorkingTreeSection::Staged,
        &["src/w.rs", "src/x.rs"],
    )]);
    let answer = AppEvent::Produced(
        list_job(&commands),
        Product::Files(Ok((Arc::new(view), Arc::new(list)))),
    );
    let (state, _) = step(&state, answer);
    assert_eq!(
        state.detail.selected_section(),
        Some(WorkingTreeSection::Staged)
    );
    assert_eq!(state.detail.selected_entry(), Some(1), "src/x.rs's row");
    let landing = state.line_landing.as_ref().expect("waiting for the diff");
    assert_eq!(landing.at, None, "its first change, not line 9");
}

#[test]
fn show_line_unfolds_a_folded_directory_to_reach_its_path() {
    let mut state = on_graph();
    let spec = DetailSpec::Commit(commit(1));
    let commands = show_line(
        &mut state,
        spec,
        Path::new("src/a.rs"),
        Some(new_line(2)),
        None,
    );
    let (state, commands) = step(&state, tree(list_job(&commands)));
    let (mut state, _) = step(&state, diff_answer(&commands, sample_diff()));
    assert_eq!(state.line_landing, None, "the first landing is in");

    // Fold `src/`: the cursor moves up onto it.
    if let Loading::Ready(files) = &mut state.detail.files {
        Arc::make_mut(files).toggle_dir(0);
        assert_eq!(files.cursor, 0);
    }
    // The diff is cached from the first landing: nothing to ask for.
    let commands = show_line(
        &mut state,
        spec,
        Path::new("src/a.rs"),
        Some(new_line(2)),
        None,
    );
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(state.status, Status::Ready, "no `isn't among`");
    let Loading::Ready(files) = &state.detail.files else {
        panic!("the list is still in");
    };
    assert_eq!(
        files.rows[0].kind,
        FileRowKind::Dir { collapsed: false },
        "src/ is open"
    );
    assert_eq!(files.cursor, 1, "src/a.rs selected");
    assert_eq!(state.line_landing, None, "landed (the diff was cached)");
    assert_eq!(state.detail.body_cursor(), Some(3));
}
