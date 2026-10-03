//! The pure half of the rewrite engine: squash groups, the default message,
//! validation, autosquash marks and the todo text git is handed.

use std::collections::BTreeMap;

use graft_core::oid::Oid;

use super::todo::autosquash;
use super::{
    ForkSection, SquashGroup, TodoAction, TodoLine, TodoProblem, default_message, git_todo,
    squash_groups, validate,
};
use crate::operation::Tip;

fn oid(byte: u8) -> Oid {
    Oid::from_hex(&format!("{byte:02x}").repeat(20)).expect("a hex oid")
}

fn row(action: TodoAction, byte: u8, subject: &str) -> TodoLine {
    TodoLine {
        action,
        commit: oid(byte),
        subject: subject.to_owned(),
    }
}

use TodoAction::{Drop, Edit, Fixup, Pick, Reword, Squash};

#[test]
fn squash_groups_meld_into_the_row_below() {
    // Oldest first: A ← s B ← s C, then D alone, then E ← f F.
    let todo = [
        row(Pick, 1, "A"),
        row(Squash, 2, "B"),
        row(Squash, 3, "C"),
        row(Pick, 4, "D"),
        row(Edit, 5, "E"),
        row(Drop, 6, "dropped"),
        row(Fixup, 7, "F"),
    ];
    assert_eq!(
        squash_groups(&todo),
        [
            SquashGroup {
                kept: 0,
                melded: vec![2, 1],
            },
            SquashGroup {
                kept: 4,
                melded: vec![6],
            },
        ],
        "melded newest first; a dropped row is passed over, as git does; D is no group"
    );
}

#[test]
fn default_message_leads_with_the_kept_row_then_newest_first() {
    let todo = [
        row(Pick, 1, "Add OAuth callback"),
        row(Squash, 2, "wip"),
        row(Squash, 3, "Handle expired tokens"),
    ];
    let messages = [
        "Add OAuth callback\n\nThe body.\n",
        "wip\n",
        "Handle expired tokens\n",
    ];
    let group = &squash_groups(&todo)[0];
    let message = default_message(group, &|row| messages[row].to_owned(), &todo);
    assert_eq!(
        message,
        "Add OAuth callback\n\nThe body.\n\nHandle expired tokens\n\nwip\n"
    );
}

#[test]
fn fixup_adds_no_message() {
    let todo = [
        row(Pick, 1, "Add OAuth callback"),
        row(Fixup, 2, "fixup! Add OAuth callback"),
        row(Squash, 3, "More"),
    ];
    let messages = [
        "Add OAuth callback\n",
        "fixup! Add OAuth callback\n",
        "More\n",
    ];
    let group = &squash_groups(&todo)[0];
    assert_eq!(
        default_message(group, &|row| messages[row].to_owned(), &todo),
        "Add OAuth callback\n\nMore\n"
    );
}

#[test]
fn validate_refuses_a_bottom_squash() {
    let todo = [row(Squash, 1, "A"), row(Pick, 2, "B")];
    assert_eq!(
        validate(&todo),
        Err(TodoProblem::MeldsIntoNothing { row: 0 })
    );
    // Only drops below it is nothing below it too.
    let todo = [row(Drop, 1, "A"), row(Fixup, 2, "B"), row(Pick, 3, "C")];
    assert_eq!(
        validate(&todo),
        Err(TodoProblem::MeldsIntoNothing { row: 1 })
    );
    assert_eq!(
        TodoProblem::MeldsIntoNothing { row: 1 }.explain(),
        "nothing below to meld into"
    );
    assert_eq!(validate(&[row(Pick, 1, "A"), row(Squash, 2, "B")]), Ok(()));
}

#[test]
fn validate_refuses_everything_dropped() {
    assert_eq!(
        validate(&[row(Drop, 1, "A"), row(Drop, 2, "B")]),
        Err(TodoProblem::EverythingDropped)
    );
    assert!(
        TodoProblem::EverythingDropped
            .explain()
            .starts_with("every commit dropped")
    );
}

#[test]
fn autosquash_rows_are_marked_and_moved_above_their_target() {
    let todo = vec![
        row(Pick, 1, "Add OAuth callback"),
        row(Pick, 2, "Settings page"),
        row(Pick, 3, "fixup! Add OAuth callback"),
        row(Pick, 4, "squash! Settings page"),
        row(Pick, 5, "fixup! fixup! Add OAuth callback"),
        row(Pick, 6, "fixup! Nothing like this"),
        row(Pick, 7, &format!("fixup! {}", &oid(2).to_string()[..7])),
    ];
    let marked = autosquash(todo);
    let shape: Vec<(TodoAction, u8)> = marked
        .iter()
        .map(|line| (line.action, line.commit.as_bytes()[0]))
        .collect();
    assert_eq!(
        shape,
        [
            (Pick, 1),
            (Fixup, 3),
            (Fixup, 5),
            (Pick, 2),
            (Squash, 4),
            (Fixup, 7),
            (Pick, 6),
        ],
        "each marker directly above its target, after the ones already there; \
         one naming nothing stays a pick"
    );
}

#[test]
fn the_git_todo_asks_where_git_asks_and_the_line_follows() {
    let todo = [
        row(Pick, 1, "A"),
        row(Squash, 2, "B"),
        row(Fixup, 3, "C"),
        row(Reword, 4, "D"),
        row(Pick, 5, "E"),
        row(Fixup, 6, "F"),
        row(Drop, 7, "G"),
    ];
    let mut messages = BTreeMap::new();
    messages.insert(3, "D, reworded\n".to_owned());
    messages.insert(4, "E and F\n".to_owned());
    let whole = |row: usize| format!("message {row}\n");
    let follower = vec![row(Pick, 8, "H")];
    let top = vec![row(Pick, 9, "I")];
    let built = git_todo(
        &todo,
        &messages,
        &whole,
        "refs/heads/a",
        &[
            ("refs/heads/b".to_owned(), follower),
            ("refs/heads/c".to_owned(), top),
        ],
        &[],
    );
    let expected = [
        format!("pick {} A", oid(1)),
        format!("squash {} B", oid(2)),
        format!("squash {} C", oid(3)),
        format!("reword {} D", oid(4)),
        format!("pick {} E", oid(5)),
        format!("squash {} F", oid(6)),
        format!("drop {} G", oid(7)),
        "update-ref refs/heads/a".to_owned(),
        format!("pick {} H", oid(8)),
        "update-ref refs/heads/b".to_owned(),
        format!("pick {} I", oid(9)),
    ];
    assert_eq!(built.text, expected.join("\n") + "\n");
    assert_eq!(
        built.messages,
        [
            (oid(3), "message 0\n\nmessage 1\n".to_owned()),
            (oid(6), "E and F\n".to_owned()),
            (oid(4), "D, reworded\n".to_owned()),
        ]
    );
}

#[test]
fn an_empty_git_todo_is_a_noop() {
    let built = git_todo(
        &[],
        &BTreeMap::new(),
        &|_| String::new(),
        "refs/heads/a",
        &[],
        &[],
    );
    assert_eq!(built.text, "noop\n");
}

fn tip(name: &str, byte: u8) -> Tip {
    Tip {
        name: format!("refs/heads/{name}"),
        oid: oid(byte),
    }
}

/// ADR-0059 §3: each fork that follows is replayed inline, right after the
/// row that makes its new base — a squash group's last row, a dependent's
/// pick — as `label`, its picks with an `update-ref` after each branch on
/// it, `reset`; one with no commits of its own is its `update-ref` alone.
#[test]
fn a_fork_section_follows_the_row_that_makes_its_base() {
    let todo = [row(Pick, 1, "A"), row(Fixup, 2, "A2"), row(Pick, 3, "B")];
    let dependent = vec![row(Pick, 4, "C"), row(Pick, 5, "D")];
    let forks = [
        ForkSection {
            after_row: 1,
            branch: tip("x", 7),
            picks: vec![row(Pick, 6, "X1"), row(Pick, 7, "X2")],
            inside: vec![tip("x-wip", 6)],
        },
        ForkSection {
            after_row: 3,
            branch: tip("park", 4),
            picks: Vec::new(),
            inside: Vec::new(),
        },
        ForkSection {
            after_row: 3,
            branch: tip("y", 8),
            picks: vec![row(Pick, 8, "Y1")],
            inside: Vec::new(),
        },
    ];
    let built = git_todo(
        &todo,
        &BTreeMap::new(),
        &|_| String::new(),
        "refs/heads/a",
        &[("refs/heads/c".to_owned(), dependent)],
        &forks,
    );
    let expected = [
        format!("pick {} A", oid(1)),
        format!("fixup {} A2", oid(2)),
        "label graft-fork-1".to_owned(),
        format!("pick {} X1", oid(6)),
        "update-ref refs/heads/x-wip".to_owned(),
        format!("pick {} X2", oid(7)),
        "update-ref refs/heads/x".to_owned(),
        "reset graft-fork-1".to_owned(),
        format!("pick {} B", oid(3)),
        "update-ref refs/heads/a".to_owned(),
        format!("pick {} C", oid(4)),
        "update-ref refs/heads/park".to_owned(),
        "label graft-fork-3".to_owned(),
        format!("pick {} Y1", oid(8)),
        "update-ref refs/heads/y".to_owned(),
        "reset graft-fork-3".to_owned(),
        format!("pick {} D", oid(5)),
    ];
    assert_eq!(built.text, expected.join("\n") + "\n");
}
