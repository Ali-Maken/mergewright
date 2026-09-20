//! The footer's key hints, by context (`hint-bar-redesign`, ADR-0037 §4: the
//! bar is how anything is discoverable; a feature nobody can see doesn't
//! exist). Shows the three most relevant keys for wherever the cursor or
//! focus is, at 80 columns, six at 120, all of them at 160 — never all
//! hidden, and never a hint cut in half.
//!
//! Replaces the old fixed nineteen-entry list (`docs/design/02-keymap.md`
//! `graph_screen` MAP entry's 144/155-column thresholds): that list answered
//! "what can this screen ever do", which grew too long to read at 80 or 120
//! columns and buried the keys that mattered right now. This one answers
//! "what does the *cursor* do right now" — the design note's priority table,
//! verbatim.

use unicode_width::UnicodeWidthStr;

use crate::fit::fit;

/// One hint: a key and the verb it does, e.g. `m merge`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hint {
    /// The key as it reads on screen (`m`, `⏎`, `⌃A`, `.`).
    pub key: &'static str,
    /// The verb it does (`merge`, `open`, `more`).
    pub verb: &'static str,
}

const fn hint(key: &'static str, verb: &'static str) -> Hint {
    Hint { key, verb }
}

/// Leads a truncated list: press `.` for the rest (the context-actions key,
/// `docs/design/02-keymap.md` §2.1 "Rare verbs").
const MORE: Hint = hint(".", "more");
/// Leads a marked row's list: press `.` to explain the Mistake Radar finding
/// (m2-details-and-diffs/17, design note "Marker column").
const EXPLAIN: Hint = hint(".", "explain");

/// Where the cursor or focus is, for the footer's hint list. `Preview` is
/// the one context allowed an empty list (the Ghost Preview box carries its
/// own footer); every other variant's list is non-empty by construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HintContext {
    /// The cursor is on the collapsed Uncommitted-changes row.
    WorkingTreeRow,
    /// The cursor is on a row with a local branch badge; `current` is the
    /// branch `HEAD` points at.
    BranchRow {
        /// The badge is the current branch (`HEAD`).
        current: bool,
    },
    /// The cursor is on a row with no branch badge.
    PlainRow,
    /// The cursor row carries a Mistake Radar marker: `. explain` leads,
    /// then the inner context's own list, `. more` omitted (`.` is already
    /// shown).
    Marked(Box<HintContext>),
    /// The operation banner has focus (a conflict just landed, or the user
    /// tabbed to it); `skippable` is false for a plain merge, which has no
    /// `s` skip.
    Banner {
        /// `s` (skip) applies — a rebase/cherry-pick/revert sequence, not a
        /// plain merge.
        skippable: bool,
    },
    /// The Ghost Preview box has focus: its own border draws the footer,
    /// so the main footer's list is empty.
    Preview,
    /// The changes pane (working tree, not a commit) has focus.
    ChangesPane {
        /// The working tree has an unmerged (conflicted) entry: the keys
        /// that write swap for the conflict-resolution keys.
        unmerged: bool,
        /// The detail pane's divider is on screen and draggable.
        detail_shown: bool,
    },
    /// The undo journal browser has focus (overlay; draws its own footer
    /// today — reserved for the `?` help overlay, M8).
    Journal,
    /// The stash list has focus (overlay; draws its own footer today —
    /// reserved for the `?` help overlay, M8).
    Stash,
}

/// Below this width: the first three hints. `100..160`: six. `160` and
/// above: every hint the context has.
const NARROW: u16 = 100;
/// Below this width (and at or above [`NARROW`]): six hints.
const MEDIUM: u16 = 160;

/// The hints for `context` that fit in `columns` cells: as many whole hints
/// as the width tier allows (three below 100, six from 100 to 159, every
/// hint from 160), dropped further from the right if even that does not
/// fit — never below three (or the context's own count, if it has fewer)
/// plus `. more`. Never empty except for [`HintContext::Preview`].
#[must_use]
pub fn hints_for(context: &HintContext, columns: u16) -> Vec<Hint> {
    match context {
        HintContext::Preview => Vec::new(),
        HintContext::Marked(inner) => {
            let mut rest = hints_for(inner, columns);
            if rest.last() == Some(&MORE) {
                rest.pop();
            }
            let mut out = Vec::with_capacity(rest.len() + 1);
            out.push(EXPLAIN);
            out.extend(rest);
            out
        }
        _ => fit_list(&context_list(context), columns),
    }
}

/// As many of `list`'s leading hints as the width tier and `columns` allow,
/// plus `. more`. Never fewer than three (or `list.len()` if it is shorter).
fn fit_list(list: &[Hint], columns: u16) -> Vec<Hint> {
    let tier = if columns < NARROW {
        3
    } else if columns < MEDIUM {
        6
    } else {
        list.len()
    };
    let floor = list.len().min(3);
    let mut count = tier.min(list.len());
    loop {
        let mut candidate = list[..count].to_vec();
        candidate.push(MORE);
        if text_width(&hints_text(&candidate)) <= usize::from(columns) || count <= floor {
            return candidate;
        }
        count -= 1;
    }
}

fn text_width(text: &str) -> usize {
    text.width()
}

/// The design note's priority table, verbatim, one context at a time.
/// `Marked` and `Preview` are handled in [`hints_for`], not here.
fn context_list(context: &HintContext) -> Vec<Hint> {
    match context {
        HintContext::WorkingTreeRow => working_tree_row_list(),
        HintContext::BranchRow { current: false } => branch_row_list(),
        HintContext::BranchRow { current: true } => current_branch_row_list(),
        HintContext::PlainRow => plain_row_list(),
        HintContext::Banner { skippable } => banner_list(*skippable),
        HintContext::ChangesPane {
            unmerged,
            detail_shown,
        } => changes_pane_list(*unmerged, *detail_shown),
        HintContext::Journal => vec![hint("⏎", "revert"), hint("esc", "close")],
        HintContext::Stash => vec![hint("⏎", "apply"), hint("p", "pop"), hint("d", "drop")],
        HintContext::Marked(_) | HintContext::Preview => {
            unreachable!("handled in hints_for")
        }
    }
}

fn working_tree_row_list() -> Vec<Hint> {
    vec![
        hint("⏎", "changes"),
        hint("c", "commit"),
        hint("S", "stash"),
        hint("p", "pull"),
        hint("P", "push"),
        hint("f", "fetch"),
        hint("u", "undo"),
        hint("J", "journal"),
        hint("/", "search"),
        hint("j/k", "move"),
    ]
}

fn branch_row_list() -> Vec<Hint> {
    vec![
        hint("m", "merge"),
        hint("r", "rebase"),
        hint("⏎", "open"),
        hint("b", "branch"),
        hint("t", "tag"),
        hint("P", "push"),
        hint("c", "commit"),
        hint("p", "pull"),
        hint("f", "fetch"),
        hint("u", "undo"),
        hint("S", "stash"),
        hint("/", "search"),
    ]
}

fn current_branch_row_list() -> Vec<Hint> {
    vec![
        hint("⏎", "open"),
        hint("c", "commit"),
        hint("P", "push"),
        hint("p", "pull"),
        hint("f", "fetch"),
        hint("b", "branch"),
        hint("t", "tag"),
        hint("u", "undo"),
        hint("S", "stash"),
        hint("J", "journal"),
        hint("/", "search"),
    ]
}

fn plain_row_list() -> Vec<Hint> {
    vec![
        hint("⏎", "open"),
        hint("b", "branch"),
        hint("r", "rebase"),
        hint("t", "tag"),
        hint("c", "commit"),
        hint("y", "sha"),
        hint("m", "merge"),
        hint("u", "undo"),
        hint("S", "stash"),
        hint("J", "journal"),
        hint("/", "search"),
    ]
}

fn banner_list(skippable: bool) -> Vec<Hint> {
    let mut list = vec![hint("c", "continue")];
    if skippable {
        list.push(hint("s", "skip"));
    }
    list.push(hint("a", "abort"));
    list.push(hint("M", "mergetool"));
    list.push(hint("e", "edit"));
    list.push(hint("⇥", "graph"));
    list
}

fn changes_pane_list(unmerged: bool, detail_shown: bool) -> Vec<Hint> {
    let mut list = if unmerged {
        vec![
            hint("M", "mergetool"),
            hint("e", "edit"),
            hint("space", "resolved"),
        ]
    } else {
        vec![
            hint("space", "stage"),
            hint("d", "discard"),
            hint("c", "commit"),
            hint("⌃A", "all"),
            hint("e", "edit"),
            hint("⇥", "graph"),
        ]
    };
    if detail_shown {
        list.push(hint("<>", "resize"));
    }
    list
}

/// `hints` as the footer draws them: `" m merge  r rebase  ⏎ open"` — a
/// leading space (matching the footer's other slots, all built the same
/// way), two spaces between hints, one between a key and its verb.
#[must_use]
pub fn hints_text(hints: &[Hint]) -> String {
    let mut line = String::new();
    for hint in hints {
        line.push_str(if line.is_empty() { " " } else { "  " });
        line.push_str(hint.key);
        line.push(' ');
        line.push_str(hint.verb);
    }
    line
}

/// A status line, worded `reason · keys` (the design note's "Failure line",
/// `remote_failed`'s callers): keeps ` · keys` whole and cuts the reason
/// with `…` first, since the key the message names is the point (ADR-0037
/// §4, acceptance M2 — at 80 columns the old blind cut lost `P force`).
/// When even ` · keys` cannot fit, `keys` is cut too. No `keys`: the whole
/// line is cut like any other footer text.
#[must_use]
pub fn fit_status_line(reason: &str, keys: Option<&str>, columns: u16) -> String {
    let columns = usize::from(columns);
    let Some(keys) = keys else {
        return fit(reason, columns);
    };
    let suffix = format!(" · {keys}");
    let suffix_width = text_width(&suffix);
    if suffix_width >= columns {
        return fit(&suffix, columns);
    }
    let room = columns - suffix_width;
    if text_width(reason) <= room {
        return format!("{reason}{suffix}");
    }
    format!("{}{suffix}", fit(reason, room))
}

#[cfg(test)]
mod tests;
