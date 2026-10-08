//! The graph rows' lists: the working tree, a branch, the current branch, a
//! plain commit, a stash.

use super::super::keys::{
    BRANCH, COMMIT, COPY_SHA, DOWN_UP, FETCH, FIND, JOURNAL, MERGE, OPEN, PULL, PUSH, REBASE,
    STASH, TAG, UNDO,
};
use super::super::{Hint, hint};

pub(super) fn working_tree_row_list() -> Vec<Hint> {
    vec![
        hint(OPEN, "changes"),
        hint(COMMIT, "commit"),
        hint(STASH, "stash"),
        hint(PULL, "pull"),
        hint(PUSH, "push"),
        hint(FETCH, "fetch"),
        hint(UNDO, "undo"),
        hint(JOURNAL, "journal"),
        hint(FIND, "search"),
        hint(DOWN_UP, "move"),
    ]
}

pub(super) fn branch_row_list() -> Vec<Hint> {
    vec![
        hint(MERGE, "merge"),
        hint(REBASE, "rebase"),
        hint(OPEN, "open"),
        hint(BRANCH, "branch"),
        hint(TAG, "tag"),
        hint(PUSH, "push"),
        hint(COMMIT, "commit"),
        hint(PULL, "pull"),
        hint(FETCH, "fetch"),
        hint(UNDO, "undo"),
        hint(STASH, "stash"),
        hint(FIND, "search"),
    ]
}

pub(super) fn current_branch_row_list() -> Vec<Hint> {
    vec![
        hint(OPEN, "open"),
        hint(COMMIT, "commit"),
        hint(PUSH, "push"),
        hint(PULL, "pull"),
        hint(FETCH, "fetch"),
        hint(BRANCH, "branch"),
        hint(TAG, "tag"),
        hint(UNDO, "undo"),
        hint(STASH, "stash"),
        hint(JOURNAL, "journal"),
        hint(FIND, "search"),
    ]
}

pub(super) fn plain_row_list() -> Vec<Hint> {
    vec![
        hint(OPEN, "open"),
        hint(BRANCH, "branch"),
        hint(REBASE, "rebase"),
        hint(TAG, "tag"),
        hint(COMMIT, "commit"),
        hint(COPY_SHA, "sha"),
        hint(MERGE, "merge"),
        hint(UNDO, "undo"),
        hint(STASH, "stash"),
        hint(JOURNAL, "journal"),
        hint(FIND, "search"),
    ]
}

/// A stash's row (m8-polish-and-ship/15 A4): [`plain_row_list`] less
/// `m merge`, `r rebase` (both refuse on a stash, `preview/history.rs:108`),
/// with `S stash` right after `⏎ open` — the stash's own verbs (`p d b ⏎`,
/// keymap §2.9) live in the `S` list, not on the graph row. `R rewrite` and
/// `o take` are never added: `R` is not offered here (`Rewritable` never
/// wraps a stash row) and `o` refuses on a stash (`can_take`). [`TAIL`]'s
/// `S stash` is deduped by key, so it is drawn once, here.
pub(super) fn stash_row_list() -> Vec<Hint> {
    let mut list: Vec<Hint> = plain_row_list()
        .into_iter()
        .filter(|hint| !hint.is(MERGE) && !hint.is(REBASE) && !hint.is(STASH))
        .collect();
    let at = list
        .iter()
        .position(|hint| hint.is(OPEN))
        .map_or(0, |i| i + 1);
    list.insert(at, hint(STASH, "stash"));
    list
}
