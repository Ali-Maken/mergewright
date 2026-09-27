//! `space` on the detail pane's bar with a commit open (m7-power-tools/20h):
//! drawn exactly where the key does something — a directory row folds, a
//! file row has nothing to fold, and with the diff focused the pane claims
//! the key and does nothing (20l).

use super::*;
use graft_ui::graph_screen::hints_for;

/// The `space` hints the footer draws over `state`, read through the app's
/// own classifier, on a bar wide enough to hold every hint.
fn space_hints(state: &AppState) -> Vec<&'static str> {
    let banner = crate::update::banner::banner_view(state);
    let context = crate::view::rows::footer_context(state, &[], banner.as_ref());
    hints_for(&context, Divider::Shown, 400)
        .into_iter()
        .filter(|hint| hint.key == "space")
        .map(|hint| hint.verb)
        .collect()
}

fn files_cursor_kind(state: &AppState) -> FileRowKind {
    match &state.detail.files {
        Loading::Ready(files) => files.rows[files.cursor].kind.clone(),
        other => panic!("the file list is loaded, not {other:?}"),
    }
}

#[test]
fn pane_bar_on_a_commits_file_row_draws_no_space() {
    let mut state = commit_pane(JobId::new(701), true);
    super::super::place::tests::on_a_file(&mut state);
    assert!(matches!(
        files_cursor_kind(&state),
        FileRowKind::File { .. }
    ));
    assert_eq!(space_hints(&state), Vec::<&str>::new());
    // And the key does nothing there: the bar is not hiding a live key.
    let (next, _) = update(&state, key(KeyCode::Char(' ')));
    assert_eq!(next.detail.files, state.detail.files);
    assert_eq!(next.history, state.history);
}

#[test]
fn pane_bar_on_a_directory_row_offers_space_fold() {
    let state = commit_pane(JobId::new(701), true);
    assert_eq!(
        files_cursor_kind(&state),
        FileRowKind::Dir { collapsed: false }
    );
    assert_eq!(space_hints(&state), ["fold"]);
}

#[test]
fn space_on_a_directory_row_folds_it_and_the_bar_still_says_fold() {
    let state = commit_pane(JobId::new(701), true);

    let (folded, _) = update(&state, key(KeyCode::Char(' ')));
    assert_eq!(
        files_cursor_kind(&folded),
        FileRowKind::Dir { collapsed: true }
    );
    assert_eq!(folded.history, state.history, "the graph's merge untouched");
    assert_eq!(space_hints(&folded), ["fold"]);

    let (unfolded, _) = update(&folded, key(KeyCode::Char(' ')));
    assert_eq!(
        files_cursor_kind(&unfolded),
        FileRowKind::Dir { collapsed: false }
    );
    assert_eq!(space_hints(&unfolded), ["fold"]);
}

// Replaced `space_with_the_diff_focused_folds_the_graphs_merge_and_the_bar_says_so`:
// spec change M7/20h-diff-space (ticket 20l), the pane now claims `space`.

/// With the diff focused `space` belongs to the pane (it claims the key and
/// does nothing), at every width: the graph's merge is not folded, and the
/// bar offers no `space` (m7-power-tools/20l).
#[test]
fn space_with_the_diff_focused_leaves_the_graph_as_it_was() {
    for (columns, rows) in [(120, 40), (80, 24)] {
        let mut state = commit_pane(JobId::new(701), false);
        state.size.columns = columns;
        state.size.rows = rows;
        let at = format!("{columns}x{rows}");
        assert_eq!(space_hints(&state), Vec::<&str>::new(), "{at}");

        let (next, commands) = update(&state, key(KeyCode::Char(' ')));
        // The evil-merge probe rides along on any event over a merge
        // (`asked_view`); what must be absent is a row request, i.e. a fold.
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::ShowRows(..))),
            "{at}: {commands:?}"
        );
        assert_eq!(next.history, state.history, "{at}");
        assert_eq!(next.detail.files, state.detail.files, "{at}");
        assert_eq!(space_hints(&next), Vec::<&str>::new(), "{at}");
    }
}
