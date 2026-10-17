//! The graph rows' lists: the working tree, a branch, the current branch, a
//! plain commit, a stash — and the ease pass's row bars ([`RowBar`]).

use super::super::keys::{
    BRANCH, COMMIT, COMPARE, COPY_SHA, DOWN_UP, FETCH, FIND, FOLD, JOURNAL, MERGE, OPEN, PULL,
    PUSH, REBASE, STASH, SWITCH, TAG, UNDO,
};
use super::super::{Hint, RowBar, bar_name, hint, named};

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

/// A graph row's bar in the ease pass's words (ticket ease-pass/03b,
/// design note G13 and M1): each kind of row says its own next steps, a
/// branch by its [`bar_name`], a count pluralised. Words exactly the
/// mocks'; the bar end stays today's `. more` (Q11 a).
pub(super) fn graph_row_list(row: &RowBar) -> Vec<Hint> {
    match row {
        RowBar::Own => vec![
            hint(OPEN, "open"),
            hint(PUSH, "push"),
            hint(COMMIT, "commit"),
        ],
        // The merge mock leads with `m` (M1); the main mock's three follow.
        RowBar::Elsewhere { branch } => {
            let name = bar_name(branch);
            vec![
                named(MERGE, format!("merge {name} into yours"), "merge"),
                hint(OPEN, "open"),
                named(SWITCH, format!("switch to {name}"), "switch"),
                hint(COMPARE, "compare"),
            ]
        }
        RowBar::FoldedMerge { inside } => vec![
            named(FOLD, show_inside(*inside), "show"),
            hint(OPEN, "open"),
            hint(COMPARE, "compare"),
        ],
        RowBar::OpenedMerge => vec![
            named(FOLD, "hide the commits inside", "hide"),
            hint(OPEN, "open"),
            hint(COMPARE, "compare"),
        ],
        RowBar::JustCommitted => vec![
            named(UNDO, "undo the commit, your edits come back", "undo"),
            hint(PUSH, "push"),
            named(COMMIT, "commit again", "commit"),
        ],
        RowBar::JustMerged { inside } => vec![
            named(UNDO, "undo the merge", "undo"),
            hint(PUSH, "push"),
            named(FOLD, show_inside(*inside), "show"),
        ],
    }
}

/// `show the 1 commit inside`, `show the 3 commits inside`.
fn show_inside(inside: u32) -> String {
    let noun = if inside == 1 { "commit" } else { "commits" };
    format!("show the {inside} {noun} inside")
}
