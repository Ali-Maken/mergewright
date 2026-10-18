//! What a landing leaves for the next `c` (ease-pass/08 C13, Q9 a; 08s
//! Q9, m1, m2, m6): the commit just made, `u` giving its message back
//! without replacing typed words, and a redo taking it back while unedited.

use graft_core::journal::OperationKind;
use graft_ui::graph_screen::hints::RowBar;

use super::draft_tests::{esc, last, oid, other_write_lands, undo_lands, words};
use super::screen_tests::{editor, key, mock_tree, opened, text, walk};
use super::*;

#[test]
fn landed_bar_is_just_committed_only_on_the_commit() {
    assert_eq!(landed_bar(None, Some(oid(1))), None);
    assert_eq!(landed_bar(Some(&last(oid(1))), Some(oid(2))), None);
    assert_eq!(
        landed_bar(Some(&last(oid(1))), Some(oid(1))),
        Some(RowBar::JustCommitted)
    );
}

#[test]
fn c_opening_clears_the_last_commit() {
    let mut state = mock_tree();
    state.last_commit = Some(Box::new(last(oid(1))));
    assert!(opened(&state).last_commit.is_none());
}

/// m6: a write that is not `c`'s commit lands through `update`.
#[test]
fn another_write_landing_clears_the_last_commit() {
    let mut state = mock_tree();
    state.last_commit = Some(Box::new(last(oid(1))));
    let landed = other_write_lands(&state, OperationKind::Stage);
    assert!(landed.last_commit.is_none());
}

#[test]
fn undoing_the_commit_gives_its_message_back_as_the_draft() {
    let mut state = mock_tree();
    state.last_commit = Some(Box::new(last(oid(2))));
    let undone = undo_lands(&state, false, oid(2), oid(1), true);
    assert!(undone.last_commit.is_none());
    assert_eq!(undone.drafts.commit.as_deref(), Some(&words("fix toc", "")));
    let reopened = opened(&undone);
    assert_eq!(editor(&reopened).subject.text(), "fix toc");
    assert_eq!(editor(&reopened).going_in(), 3, "files chosen afresh");
}

/// m6: seeded with words, which an undo of something else leaves alone.
#[test]
fn undoing_something_else_leaves_the_draft_alone() {
    let mut state = mock_tree();
    state.last_commit = Some(Box::new(last(oid(2))));
    state.drafts.commit = Some(Box::new(words("half", "")));
    let undone = undo_lands(&state, false, oid(5), oid(4), true);
    assert_eq!(undone.drafts.commit.as_deref(), Some(&words("half", "")));
}

#[test]
fn undo_gives_back_without_replacing_typed_words() {
    let mut state = mock_tree();
    state.last_commit = Some(Box::new(LastCommit {
        commit: oid(2),
        message: words("fix toc", "the long story"),
    }));
    state.drafts.commit = Some(Box::new(words("half", "")));
    let reopened = opened(&undo_lands(&state, false, oid(2), oid(1), true));
    assert_eq!(editor(&reopened).subject.text(), "half");
    assert_eq!(editor(&reopened).body.text(), "fix toc\n\nthe long story");
}

#[test]
fn redo_takes_back_only_the_unchanged_returned_words() {
    let mut state = mock_tree();
    state.last_commit = Some(Box::new(last(oid(2))));
    let undone = undo_lands(&state, false, oid(2), oid(1), true);
    let redone = undo_lands(&undone, true, oid(1), oid(2), true);
    assert_eq!(
        redone.drafts.commit, None,
        "`u`, `U`: the words are the commit again"
    );
    let edited = esc(&walk(
        &undone,
        [vec![key(KeyCode::Char('c'))], text(" more")].concat(),
    )
    .0);
    let redone = undo_lands(&edited, true, oid(1), oid(2), true);
    assert_eq!(
        redone.drafts.commit.as_deref(),
        Some(&words("fix toc more", ""))
    );
}

#[test]
fn an_incomplete_undo_gives_the_words_back() {
    let mut state = mock_tree();
    state.last_commit = Some(Box::new(last(oid(2))));
    let undone = undo_lands(&state, false, oid(2), oid(1), false);
    assert!(undone.last_commit.is_none());
    assert_eq!(undone.drafts.commit.as_deref(), Some(&words("fix toc", "")));
    assert!(
        matches!(undone.status, Status::Sticky { .. }),
        "its warning stands"
    );
}
