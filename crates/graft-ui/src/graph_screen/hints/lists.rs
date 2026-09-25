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
        // Keymap §2.11: the journal browser answers `⏎` and `Esc` only.
        HintContext::Journal => vec![hint("⏎", "revert"), hint("esc", "close")],
        HintContext::Stash => stash_list(),
        // Keymap §2.3a: the Map view answers `Enter`, the movement keys and
        // `M`/`Esc`. `M` is left out for the reason the journal leaves `J`
        // out — one way of saying "close" is enough for a bar this short.
        HintContext::Map => vec![hint("⏎", "seek"), hint("j/k", "move"), hint("esc", "close")],
        // Only ever drawn in the sub-100-column `Tab` layout, which has no
        // divider to drag: everywhere else the overlay draws its own footer
        // beside the pane (`graft::view::frame::split_for_detail`, which
        // draws a hint row only for `DetailLayout::Tab`).
        HintContext::PaneOverOverlay { opens } => {
            let mut list = Vec::new();
            if *opens {
                list.push(hint("⏎", "open"));
            }
            list.extend([hint("j/k", "move"), hint("u", "undo"), hint("/", "search")]);
            list
        }
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
        | HintContext::OperationLive { .. }
        | HintContext::Preview => Vec::new(),
    }
}

/// The keys the graph screen answers wherever the cursor is, in the order
/// the old fixed list dropped them (PROMPT.md principle 4: the hint bar
/// teaches the tool). Appended after a row context's own keys, so a wide
/// terminal still shows `q quit`, `x compare` and `+ lanes` — which the
/// per-context table leaves out entirely (m5-03 review, Major 3). Whatever
/// the context already hints is not repeated ([`drawable`] dedupes by key).
pub(super) const TAIL: [Hint; 20] = [
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
