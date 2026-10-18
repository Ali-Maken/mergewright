use super::*;
use crate::update::Size;
use crate::view::tests::{counts, history_of, oid};

fn state_on_working_tree(
    counts: std::sync::Arc<mergewright_git::status::WorkingTreeStatus>,
) -> AppState {
    AppState {
        history: Some(history_of(3, 0)),
        working_tree: Some(counts),
        on_working_tree: true,
        size: Size {
            columns: 60,
            rows: 5,
        },
        ..AppState::default()
    }
}

/// HEAD's id comes from the refs, not from the row order: row 0 here is
/// `oid(3)` — another branch's tip, or a stash commit sorted on top —
/// while HEAD is elsewhere. Unread refs name no id at all
/// (review m6-usability-2 m-b).
#[test]
fn the_uncommitted_header_names_head_not_the_top_row() {
    for head in [oid(1), oid(2)] {
        let mut state = state_on_working_tree(counts(0, 1, 0));
        state.refs = Some(std::sync::Arc::new(crate::view::tests::refs_with_head(
            Some(head),
        )));
        let against = detail_view(&state).header.against.expect("a HEAD line");
        let sha = short_sha(&head.to_string(), usize::from(SHA_WIDTH));
        assert_eq!(against.as_str(), format!("HEAD {sha}"));
    }
    let unread = state_on_working_tree(counts(0, 1, 0));
    let against = detail_view(&unread).header.against.expect("a HEAD line");
    assert_eq!(against.as_str(), "HEAD", "refs unread: no id");
}

#[test]
fn working_tree_header_reads_counts() {
    let state = state_on_working_tree(counts(2, 1, 3));
    let view = detail_view(&state);
    assert_eq!(
        view.header.subject,
        Subject::WorkingTree {
            staged: 2,
            unstaged: 1,
            untracked: 3,
        }
    );
    assert_eq!(view.header.summary.as_str(), "Uncommitted changes");
    assert!(view.header.author.as_str().is_empty());
    assert!(view.header.parents.is_empty());
    assert_eq!(view.header.contained, ContainedIn::NotAsked);
}

#[test]
fn commit_header_reads_the_selected_rows_facts() {
    let state = AppState {
        history: Some(history_of(3, 0)),
        size: Size {
            columns: 60,
            rows: 5,
        },
        ..AppState::default()
    };
    let view = detail_view(&state);
    assert_eq!(
        view.header.subject,
        Subject::Commit {
            sha: sanitize_line(&oid(3).to_string()[..7])
        }
    );
    assert_eq!(view.header.author.as_str(), "Test <test@example.invalid>");
    // Author and committer are identical in this fixture: no line shown.
    assert_eq!(view.header.committer, None);
    assert_eq!(view.header.parents.len(), 1);
}
