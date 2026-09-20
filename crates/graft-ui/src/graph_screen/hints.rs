//! The footer's key hints, by context (`hint-bar-redesign`, ADR-0037 §4: the
//! bar is how anything is discoverable; a feature nobody can see doesn't
//! exist). Shows the three most relevant keys for wherever the cursor or
//! focus is, at 80 columns, six at 120, and from 160 as many as fit — never
//! all hidden, and never a hint cut in half.
//!
//! Replaces the old fixed nineteen-entry list (`crates/graft-ui/MAP.md`'s
//! `graph_screen` entry: 144/155-column thresholds): that list answered
//! "what can this screen ever do", which grew too long to read at 80 or 120
//! columns and buried the keys that mattered right now. This one answers
//! "what does the *cursor* do right now" first, and only then what the whole
//! screen answers ([`TAIL`], the old list's leftovers in the old order, so a
//! wide terminal still teaches `q quit` or `x compare`).
//!
//! The per-context lists are the design note's priority table
//! (`docs/design-notes/m5-history-and-ghost-preview.md`, "Hint bar") — with
//! `Journal` and `Stash`, which the note leaves out, taken from
//! `docs/design/02-keymap.md` §2.11 and §2.9.
//!
//! **A hint is only drawn once its key is bound.** Every entry carries a
//! [`HintState`]: the table stays as the design wrote it, but
//! [`HintState::Planned`] hints are dropped before the width rule
//! runs, so the bar never teaches a key that does nothing (m5-03 review,
//! Critical 2). `graft` proves both halves —
//! `update::tests::every_hinted_key_is_bound` (Live ⇒ bound, key *and* verb)
//! and `update::tests::every_planned_hint_is_still_unbound` (Planned ⇒ not
//! bound, so binding one fails the build until the table is flipped).

use unicode_width::UnicodeWidthStr;

use crate::fit::fit;

/// Whether the key a hint names actually does the verb it names, today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintState {
    /// Bound in the app: pressing the key does the verb.
    Live,
    /// Named by `docs/design/02-keymap.md` for this context, but no group
    /// has built it yet. Never drawn; kept here so the table stays the
    /// design's and the group that binds it has one word to change.
    Planned,
}

/// One hint: a key, the verb it does, and whether it does it yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hint {
    /// The key as it reads on screen (`m`, `⏎`, `⌃A`, `.`).
    pub key: &'static str,
    /// The verb it does (`merge`, `open`, `more`).
    pub verb: &'static str,
    /// Bound today, or still to be built.
    pub state: HintState,
}

/// A hint whose key is bound today.
const fn hint(key: &'static str, verb: &'static str) -> Hint {
    Hint {
        key,
        verb,
        state: HintState::Live,
    }
}

/// A hint the keymap names for this context that no group has built yet.
/// Never drawn (see the module comment).
const fn planned(key: &'static str, verb: &'static str) -> Hint {
    Hint {
        key,
        verb,
        state: HintState::Planned,
    }
}

/// Leads a truncated list: press `.` for the rest (the context-actions key,
/// `docs/design/02-keymap.md` §2.1 "Rare verbs").
const MORE: Hint = hint(".", "more");
/// Leads a marked row's list: press `.` to explain the Mistake Radar finding
/// (m2-details-and-diffs/17, design note "Marker column").
const EXPLAIN: Hint = hint(".", "explain");
/// The detail pane's divider (ADR-0028): shown by every context while a
/// pane is open, not only the changes pane (m5-03 review, Major 4).
const RESIZE: Hint = hint("<>", "resize");

/// Whether a detail pane is on screen, so its divider can be dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Divider {
    /// A detail pane is open: `<> resize` applies.
    Shown,
    /// No pane, no divider, no hint.
    Hidden,
}

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
        /// The file under the cursor is unmerged (conflicted): the keys
        /// that write swap for the conflict-resolution keys.
        unmerged: bool,
    },
    /// The undo journal browser has focus (overlay; draws its own footer
    /// today — reserved for the `?` help overlay, M8).
    Journal,
    /// The stash list has focus (overlay; draws its own footer today —
    /// reserved for the `?` help overlay, M8).
    Stash,
}

/// Below this width: the first three hints. `100..160`: six. `160` and
/// above: as many as fit.
const NARROW: u16 = 100;
/// Below this width (and at or above [`NARROW`]): six hints.
const MEDIUM: u16 = 160;

/// The hints for `context` that fit in `columns` cells: as many whole hints
/// as the width tier allows (three below 100, six from 100 to 159, as many
/// as fit from 160), dropped further from the right if even that does not
/// fit — never below three (or the context's own count, if it has fewer)
/// plus `. more`. Never empty except for [`HintContext::Preview`].
///
/// `divider` adds `<> resize` when a detail pane is open, whichever context
/// the cursor is in (ADR-0028: the pane being shown is what makes the
/// divider exist).
#[must_use]
pub fn hints_for(context: &HintContext, divider: Divider, columns: u16) -> Vec<Hint> {
    match context {
        HintContext::Preview => Vec::new(),
        HintContext::Marked(inner) => {
            // `.` is already on screen as `. explain`, so no `. more`; the
            // explain hint counts as one of the width tier's own, which is
            // why it is prepended before fitting and not after (a list
            // fitted first and prepended to second overflows by one hint).
            let mut list = vec![EXPLAIN];
            list.extend(drawable(inner, divider));
            fit_list(&list, None, columns)
        }
        _ => fit_list(&drawable(context, divider), Some(MORE), columns),
    }
}

/// Every hint the footer could draw for `context`, in priority order, with
/// the ones nothing has built yet already dropped: the context's own keys,
/// the divider's when a pane is open, then — on the graph's row contexts
/// only — the keys the whole screen answers ([`TAIL`]).
///
/// A pane keeps to its own list: while the changes pane, an overlay or the
/// banner has focus it answers keys the graph does not, and the graph
/// answers keys it does not, so [`TAIL`] would be advertising the wrong
/// screen. The one exception is a context whose keys are all still
/// [`HintState::Planned`] (the banner today): its list would otherwise be
/// empty, and a keyless bar is the one outcome ADR-0037 §4 rules out, so it
/// falls back to the keys that are live everywhere.
fn drawable(context: &HintContext, divider: Divider) -> Vec<Hint> {
    let mut list: Vec<Hint> = context_list(context)
        .into_iter()
        .filter(|hint| hint.state == HintState::Live)
        .collect();
    if divider == Divider::Shown {
        // Fourth, not last: appended, it sat behind ten context keys and the
        // six-hint tier never reached it, so the hint existed and was never
        // drawn (found by rendering a real frame —
        // `graft::it::hint_bar::the_changes_pane_hints_the_keys_that_write`).
        // Fourth keeps the three verbs that do work ahead of it and still
        // shows the divider from 100 columns, which is where a pane and a
        // graph first fit side by side.
        list.insert(3.min(list.len()), RESIZE);
    }
    let rows = matches!(
        context,
        HintContext::WorkingTreeRow | HintContext::BranchRow { .. } | HintContext::PlainRow
    );
    if rows || list.is_empty() {
        for extra in TAIL {
            if !list.iter().any(|hint| hint.key == extra.key) {
                list.push(extra);
            }
        }
    }
    list
}

/// As many of `list`'s leading hints as the width tier and `columns` allow,
/// plus `trailer` (`. more`, unless the caller has already shown `.`).
/// Never fewer than three (or `list.len()` if it is shorter).
fn fit_list(list: &[Hint], trailer: Option<Hint>, columns: u16) -> Vec<Hint> {
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
        candidate.extend(trailer);
        if text_width(&hints_text(&candidate)) <= usize::from(columns) || count <= floor {
            return candidate;
        }
        count -= 1;
    }
}

fn text_width(text: &str) -> usize {
    text.width()
}

/// The design note's priority table, one context at a time, including the
/// entries no group has built yet ([`drawable`] filters those out).
/// `Marked` and `Preview` are handled in [`hints_for`], not here.
fn context_list(context: &HintContext) -> Vec<Hint> {
    match context {
        HintContext::WorkingTreeRow => working_tree_row_list(),
        HintContext::BranchRow { current: false } => branch_row_list(),
        HintContext::BranchRow { current: true } => current_branch_row_list(),
        HintContext::PlainRow => plain_row_list(),
        HintContext::Banner { skippable } => banner_list(*skippable),
        HintContext::ChangesPane { unmerged } => changes_pane_list(*unmerged),
        // Keymap §2.11: the journal browser answers `⏎` and `Esc` only.
        HintContext::Journal => vec![hint("⏎", "revert"), hint("esc", "close")],
        // Keymap §2.9: `b` (branch from this stash) is named there as
        // planned, so it is listed and not drawn.
        HintContext::Stash => vec![
            hint("⏎", "apply"),
            hint("p", "pop"),
            hint("d", "drop"),
            hint("esc", "close"),
            planned("b", "branch"),
        ],
        // Both are answered in `hints_for` before this is reached; a
        // `RowContext` sub-enum would say so in the type, at the cost of
        // wrapping every row context at every call site.
        HintContext::Marked(_) | HintContext::Preview => Vec::new(),
    }
}

/// The keys the graph screen answers wherever the cursor is, in the order
/// the old fixed list dropped them (PROMPT.md principle 4: the hint bar
/// teaches the tool). Appended after a row context's own keys, so a wide
/// terminal still shows `q quit`, `x compare` and `+ lanes` — which the
/// per-context table leaves out entirely (m5-03 review, Major 3). Whatever
/// the context already hints is not repeated ([`drawable`] dedupes by key).
const TAIL: [Hint; 20] = [
    hint("q", "quit"),
    hint("F2", "status"),
    hint("F3", "renderer"),
    hint("u", "undo"),
    hint("x", "compare"),
    hint("+", "lanes"),
    hint("/", "search"),
    hint("j/k", "move"),
    hint("space", "fold"),
    hint("1", "first-parent"),
    hint("F", "focus"),
    hint("S", "stash"),
    hint("H", "hide stash"),
    hint("f", "fetch"),
    hint("p", "pull"),
    hint("P", "push"),
    hint("J", "journal"),
    hint("y", "sha"),
    hint("M", "map"),
    hint("O", "overview"),
];

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
        planned("m", "merge"),
        planned("r", "rebase"),
        hint("⏎", "open"),
        planned("b", "branch"),
        planned("t", "tag"),
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
        planned("b", "branch"),
        planned("t", "tag"),
        hint("u", "undo"),
        hint("S", "stash"),
        hint("J", "journal"),
        hint("/", "search"),
    ]
}

fn plain_row_list() -> Vec<Hint> {
    vec![
        hint("⏎", "open"),
        planned("b", "branch"),
        planned("r", "rebase"),
        planned("t", "tag"),
        hint("c", "commit"),
        hint("y", "sha"),
        planned("m", "merge"),
        hint("u", "undo"),
        hint("S", "stash"),
        hint("J", "journal"),
        hint("/", "search"),
    ]
}

/// The banner's own keys (design note, "Banner focused"). None of them is
/// bound yet — the banner is not focusable today — so the whole list is
/// planned, and [`drawable`] falls back to [`TAIL`] rather than leave the
/// bar empty. The group that gives the banner focus flips these.
fn banner_list(skippable: bool) -> Vec<Hint> {
    let mut list = vec![planned("c", "continue")];
    if skippable {
        list.push(planned("s", "skip"));
    }
    list.push(planned("a", "abort"));
    list.push(planned("M", "mergetool"));
    list.push(planned("e", "edit"));
    list.push(planned("⇥", "graph"));
    list
}

/// The changes pane (design note, "Changes pane"). On an unmerged file the
/// write keys swap for the conflict keys — `M mergetool` and `space
/// resolved` are M6's to build (`space` is *refused* on an unmerged entry
/// today, `update::operate`), so until then the pane keeps its own keys,
/// minus the two that would be a lie.
fn changes_pane_list(unmerged: bool) -> Vec<Hint> {
    if unmerged {
        vec![
            planned("M", "mergetool"),
            hint("e", "edit"),
            planned("space", "resolved"),
            hint("c", "commit"),
            hint("⇥", "graph"),
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
    }
}

/// Every hint in every context's table, planned ones included, deduped by
/// `(key, verb, state)` — what `graft`'s guard tests walk so that neither
/// half of the table can drift from what the app binds.
#[must_use]
pub fn every_hint() -> Vec<Hint> {
    let mut all = Vec::new();
    let contexts = [
        HintContext::WorkingTreeRow,
        HintContext::BranchRow { current: false },
        HintContext::BranchRow { current: true },
        HintContext::PlainRow,
        HintContext::Banner { skippable: true },
        HintContext::Banner { skippable: false },
        HintContext::ChangesPane { unmerged: false },
        HintContext::ChangesPane { unmerged: true },
        HintContext::Journal,
        HintContext::Stash,
    ];
    for context in contexts {
        all.extend(context_list(&context));
    }
    all.extend(TAIL);
    all.push(MORE);
    all.push(EXPLAIN);
    all.push(RESIZE);
    let mut seen = Vec::new();
    all.retain(|hint| {
        let fresh = !seen.contains(hint);
        if fresh {
            seen.push(*hint);
        }
        fresh
    });
    all
}

/// `hints` as the footer draws them: `" ⏎ open  c commit  P push"` — a
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
/// When even ` · keys` cannot fit, the keys are cut on their own, without
/// the separator — three cells spent on a `·` with nothing in front of it
/// buy nothing. No `keys`: the whole line is cut like any other footer text.
#[must_use]
pub fn fit_status_line(reason: &str, keys: Option<&str>, columns: u16) -> String {
    let columns = usize::from(columns);
    let Some(keys) = keys else {
        return fit(reason, columns);
    };
    let suffix = format!(" · {keys}");
    let suffix_width = text_width(&suffix);
    if suffix_width >= columns {
        return fit(keys, columns);
    }
    let room = columns - suffix_width;
    if text_width(reason) <= room {
        return format!("{reason}{suffix}");
    }
    format!("{}{suffix}", fit(reason, room))
}

#[cfg(test)]
mod tests;
