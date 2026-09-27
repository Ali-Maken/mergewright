//! The design note's priority table: one list of hints per context, and
//! [`TAIL`], what the whole screen answers after them.

use super::{Hint, HintContext, LiveC, TabWord, hint};

mod resolver;

use resolver::{finish_list, resolver_context_list};

/// The design note's priority table, one context at a time, including the
/// entries no group has built yet ([`drawable`] filters those out).
/// `Marked` and `Preview` are handled in [`hints_for`], not here.
pub(super) fn context_list(context: &HintContext) -> Vec<Hint> {
    match context {
        HintContext::WorkingTreeRow => working_tree_row_list(),
        HintContext::BranchRow { current: false } => branch_row_list(),
        HintContext::BranchRow { current: true } => current_branch_row_list(),
        HintContext::PlainRow => plain_row_list(),
        HintContext::Banner {
            skippable,
            conflicted,
            merge,
        } => banner_list(*skippable, *conflicted, *merge),
        HintContext::ChangesPane {
            unmerged,
            live,
            tab,
        } => changes_pane_list(*unmerged, *live, *tab),
        HintContext::Resolver { .. } => resolver_context_list(context),
        HintContext::Finish { focus, expandable } => finish_list(*focus, *expandable),
        HintContext::Blame {
            finding,
            ignore_file,
            lines,
        } => blame_bar(*finding, *ignore_file, *lines),
        HintContext::History { rows, on_file } => history_place_list(*rows, *on_file),
        // Keymap §2.11: the journal browser answers `⏎` and `Esc` only.
        HintContext::Journal => vec![hint("⏎", "revert"), hint("esc", "close")],
        HintContext::Stash => stash_list(),
        // Keymap §2.3a: the Map view answers `Enter`, the movement keys and
        // `M`/`Esc`. `M` is left out for the reason the journal leaves `J`
        // out — one way of saying "close" is enough for a bar this short.
        HintContext::Map => vec![hint("⏎", "seek"), hint("j/k", "move"), hint("esc", "close")],
        HintContext::Picker => picker_list(),
        // Only ever drawn in the sub-100-column `Tab` layout, which has no
        // divider to drag: everywhere else the overlay draws its own footer
        // beside the pane (`graft::view::frame::split_for_detail`, which
        // draws a hint row only for `DetailLayout::Tab`).
        HintContext::PaneOverOverlay { opens } => pane_over_overlay_list(*opens),
        // The drag is a mouse gesture: `release` is not a key, it is what
        // the user's hand has to do, and `esc` is the one key that answers
        // while a badge is in the air (`graft::update::drag::drag_cancel`).
        // Nothing else is offered because nothing else acts — every other
        // key cancels the drag and then means what it always means.
        HintContext::Dragging => vec![hint("release", "drop on a row"), hint("esc", "cancel")],
        // `y` leads, as it leads the preview's keys (ADR-0044 §3.1), and
        // names the acting button's own label: it takes that button from
        // wherever the cursor is. A box `y` does not take draws no `y`.
        HintContext::ConfirmBox { yes } => {
            let mut list = Vec::new();
            if let Some(yes) = yes {
                list.push(hint("y", yes.word()));
            }
            list.extend([
                hint("←/→", "move"),
                hint("⏎", "choose"),
                hint("esc", "cancel"),
            ]);
            list
        }
        HintContext::NamePrompt { toggle } => {
            let mut list = vec![hint("⏎", "create")];
            if *toggle {
                list.push(hint("⇥", "toggle"));
            }
            list.push(hint("esc", "cancel"));
            list
        }
        HintContext::Credential => vec![hint("⏎", "send"), hint("esc", "cancel")],
        // All three are answered in `whole_bar` before this is reached; a
        // `RowContext` sub-enum would say so in the type, at the cost of
        // wrapping every row context at every call site.
        HintContext::Marked(_)
        | HintContext::PaneFocused { .. }
        | HintContext::BackToBlame(_)
        | HintContext::BackToHistory(_)
        | HintContext::OperationLive { .. }
        | HintContext::Preview => Vec::new(),
    }
}

/// The branch picker's bar.
/// Keymap §2.16: the branch picker is a text field, so every letter
/// types; the bar names what the non-letter keys do
/// (`graft::update::picker::shown_key`). `esc close` is third, not
/// last as the ticket spelled it: the first three are the floor drawn
/// at every width (ADR-0037 §4), none of these verbs has a short rung
/// in the user's table, and with `⇥` third the floor is 52 cells — a
/// 40-column bar would draw nothing. With `esc` third it is 39, and
/// the way out stays in words (ADR-0044 §4's reason).
fn picker_list() -> Vec<Hint> {
    vec![
        hint("type", "to find"),
        hint("⏎", "switch to it"),
        hint("esc", "close"),
        hint("⇥", "show it on the graph"),
        hint("↑↓", "row"),
    ]
}

/// The keys the graph screen answers wherever the cursor is, in the order
/// the old fixed list dropped them (PROMPT.md principle 4: the hint bar
/// teaches the tool). Appended after a row context's own keys, so a wide
/// terminal still shows `q quit`, `x compare` and `+ lanes` — which the
/// per-context table leaves out entirely (m5-03 review, Major 3). Whatever
/// the context already hints is not repeated ([`drawable`] dedupes by key).
pub(super) const TAIL: [Hint; 21] = [
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
    hint("C", "switch"),
];

/// The blame place's bar (design note pass 2b B1; m7-power-tools/20c), in
/// the note's order. `n N match` only while a find is active (2b proposal
/// 7); `i ignore list` only when the repo has `.git-blame-ignore-revs`.
///
/// `L history of these lines` (m7-power-tools/21) is third, where the note's
/// mock-up (`:132`) draws it: in ADR-0037 §4's floor it steps down to its
/// short form, `L history` (`hints.rs` `short_form`, 21b), so the floor's
/// three are 37 cells, inside the 48-cell bar
/// (`the_floor_fits_at_forty_eight_cells_of_bar`).
/// [`HintContext::Blame`]'s list (m7-power-tools/20).
#[must_use]
pub fn blame_place_list(finding: bool, ignore_file: bool) -> Vec<Hint> {
    let mut list = vec![
        hint("⏎", "that commit"),
        hint("^", "before it"),
        hint("L", "history of these lines"),
        hint("/", "find"),
    ];
    if finding {
        list.push(hint("n N", "match"));
    }
    list.extend([hint("space", "ages"), hint("w", "whitespace")]);
    if ignore_file {
        list.push(hint("i", "ignore list"));
    }
    list.push(hint("esc", "back"));
    list
}

/// [`blame_place_list`] as the bar draws it: without `lines` (a read in
/// flight, or a failed one) the keys that act on a line — `⏎`, `^`, `L`,
/// `n N` — are left out, because the place has no lines for them
/// (m7-power-tools/20k, 20n, 21).
fn blame_bar(finding: bool, ignore_file: bool, lines: bool) -> Vec<Hint> {
    let mut list = blame_place_list(finding, ignore_file);
    if !lines {
        list.retain(|hint| !matches!(hint.key, "⏎" | "^" | "L" | "n N"));
    }
    list
}

/// [`HintContext::History`]'s list: the history place's bar (design note
/// pass 1b "History", m7-power-tools/21b), in the note's order. `rows`: the
/// trace answered with at least one row — without one (tracing, failed,
/// zero rows) only `esc back` acts, so it is the whole bar. `on_file`: the
/// file pane has the keys, so `j/k` scrolls and `⇥` goes to the list.
///
/// `v` (side by side) and `L` (the whole file) are answered but left to
/// `?`, as the detail pane's bar leaves `v`.
#[must_use]
pub fn history_place_list(rows: bool, on_file: bool) -> Vec<Hint> {
    if !rows {
        return vec![hint("esc", "back")];
    }
    let (moves, tab) = if on_file {
        (hint("j/k", "scroll"), hint("⇥", "list"))
    } else {
        (hint("j/k", "older·newer"), hint("⇥", "file"))
    };
    let mut list = vec![
        moves,
        tab,
        hint("n/N", "next change"),
        hint("B", "blame here"),
        hint("⏎", "that commit"),
    ];
    // No form offers `v`, so `w` sits in the file's form, before the way
    // back (21c); it answers from either focus.
    if on_file {
        list.push(hint("w", "whitespace"));
    }
    list.push(hint("esc", "back"));
    list
}

/// [`HintContext::PaneOverOverlay`]'s list: `⏎ open` unless the diff has
/// focus, then the pane's moves and the two verbs dispatched above it.
fn pane_over_overlay_list(opens: bool) -> Vec<Hint> {
    let mut list = Vec::new();
    if opens {
        list.push(hint("⏎", "open"));
    }
    list.extend([hint("j/k", "move"), hint("u", "undo"), hint("/", "search")]);
    list
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

/// The stash list's keys. Keymap §2.9: `b` (branch from this stash) is
/// named there as planned, so it is listed and not drawn.
fn stash_list() -> Vec<Hint> {
    vec![
        hint("⏎", "apply"),
        hint("p", "pop"),
        hint("d", "drop"),
        hint("b", "branch"),
        hint("esc", "close"),
    ]
}

/// The banner's own keys (design note, "Banner focused"), every one of them
/// answered by `graft::update::banner::banner_key` while the banner has
/// focus.
///
/// The list is the *whole* bar in that context: the banner is modal, so
/// `<> resize` and `. more` are dropped by [`hints_for`] and [`drawable`]
/// rather than advertised at keys the banner swallows. `M`/`e` appear only
/// while something is conflicted — with nothing unmerged they refuse in
/// words, and a hint whose key can only say "no" is a hint that lies
/// (m5-09 review, Major 2).
///
/// On a merge `c` opens the finish summary, so it says `c finish`
/// (m6-conflicts/70, n2).
fn banner_list(skippable: bool, conflicted: bool, merge: bool) -> Vec<Hint> {
    let mut list = vec![hint("c", if merge { "finish" } else { "continue" })];
    if skippable {
        list.push(hint("s", "skip"));
    }
    list.push(hint("a", "abort"));
    if conflicted {
        // `⏎ resolve` leads (Proposed change 2a): live since
        // m6-conflicts/04, which binds it in `update::banner::banner_key`.
        list.insert(0, hint("⏎", "resolve"));
        list.push(hint("M", "mergetool"));
        list.push(hint("e", "edit"));
    }
    list.push(hint("⇥", "graph"));
    list
}

/// The changes pane (design note, "Changes pane"). On an unmerged file the
/// write keys swap for the conflict keys: `⏎` opens the resolver
/// (`update::detail::keys`), `M` launches the mergetool and `space` marks
/// the file resolved — which is what staging an unmerged path *is*
/// (`update::operate::stage_selected`). All three bound by m6-conflicts/04.
///
/// While an operation is live, `c` is said the way it acts ([`said_live`]).
///
/// `⇥` names where it lands (`tab`, m6-conflicts/70).
fn changes_pane_list(unmerged: bool, live: Option<LiveC>, tab: TabWord) -> Vec<Hint> {
    let list = changes_pane_table(unmerged, tab);
    match live {
        Some(c) => said_live(list, c),
        None => list,
    }
}

/// `list` with its `c commit` said the way `c` acts while an operation is
/// live (m6-conflicts/52), in the same place: `c finish`, `c continue`, or
/// gone where `c` only refuses in words. Every other hint is untouched.
pub(super) fn said_live(list: Vec<Hint>, c: LiveC) -> Vec<Hint> {
    list.into_iter()
        .filter_map(|hint| {
            if hint.key != "c" {
                return Some(hint);
            }
            match c {
                LiveC::Finish => Some(Hint {
                    verb: "finish",
                    ..hint
                }),
                LiveC::Continue => Some(Hint {
                    verb: "continue",
                    ..hint
                }),
                LiveC::Refuses => None,
            }
        })
        .collect()
}

fn changes_pane_table(unmerged: bool, tab: TabWord) -> Vec<Hint> {
    if unmerged {
        vec![
            // `⏎ resolve` leads (Proposed change 2a).
            hint("⏎", "resolve"),
            hint("M", "mergetool"),
            hint("e", "edit"),
            hint("space", "resolved"),
            hint("c", "commit"),
            hint("⇥", tab.word()),
        ]
    } else {
        vec![
            hint("space", "stage"),
            hint("d", "discard"),
            hint("c", "commit"),
            hint("⌃A", "all"),
            hint("e", "edit"),
            hint("⇥", tab.word()),
        ]
    }
}
