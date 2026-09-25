//! The design note's priority table: one list of hints per context, and
//! [`TAIL`], what the whole screen answers after them.

use super::{Hint, HintContext, LiveC, hint};
use crate::conflict_resolver::SideRoles;
use crate::finish_summary::SummaryFocus;

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
        } => banner_list(*skippable, *conflicted),
        HintContext::ChangesPane { unmerged, live } => changes_pane_list(*unmerged, *live),
        HintContext::Resolver { .. } => resolver_context_list(context),
        HintContext::Finish { focus, expandable } => finish_list(*focus, *expandable),
        // Keymap §2.11: the journal browser answers `⏎` and `Esc` only.
        HintContext::Journal => vec![hint("⏎", "revert"), hint("esc", "close")],
        // Keymap §2.9: `b` (branch from this stash) is named there as
        // planned, so it is listed and not drawn.
        HintContext::Stash => vec![
            hint("⏎", "apply"),
            hint("p", "pop"),
            hint("d", "drop"),
            hint("b", "branch"),
            hint("esc", "close"),
        ],
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

/// The resolver's table for `context`: the done file's, or the bar over a
/// file still unmerged, each offering `c` while it would act.
fn resolver_context_list(context: &HintContext) -> Vec<Hint> {
    let HintContext::Resolver {
        base,
        restart,
        whole_file,
        roles,
        done,
        finishable,
    } = *context
    else {
        // Only `context_list`'s `Resolver` arm calls this; no other
        // context has a resolver table to draw.
        return Vec::new();
    };
    let list = if done {
        resolver_done_list(restart)
    } else {
        resolver_bar(base, restart, whole_file, roles)
    };
    offering_c(list, roles, finishable)
}

/// A resolver table with `c` added **first** while `c` would act
/// (m6-conflicts/31, P8; first since m6-conflicts/60: it is the next action,
/// UX 1.4): `c finish` in a merge, which opens the summary, else
/// `c continue`. Unchanged while it would refuse.
fn offering_c(mut list: Vec<Hint>, roles: SideRoles, finishable: bool) -> Vec<Hint> {
    if finishable {
        let verb = if roles == SideRoles::Merge {
            "finish"
        } else {
            "continue"
        };
        list.insert(0, hint("c", verb));
    }
    list
}

/// The resolver's bar over a file still unmerged: a whole-file card or a
/// document.
fn resolver_bar(base: bool, restart: bool, whole_file: bool, roles: SideRoles) -> Vec<Hint> {
    if whole_file {
        resolver_card_list(roles)
    } else {
        resolver_list(base, restart, roles)
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
fn banner_list(skippable: bool, conflicted: bool) -> Vec<Hint> {
    let mut list = vec![hint("c", "continue")];
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
fn changes_pane_list(unmerged: bool, live: Option<LiveC>) -> Vec<Hint> {
    let list = changes_pane_table(unmerged);
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

fn changes_pane_table(unmerged: bool) -> Vec<Hint> {
    if unmerged {
        vec![
            // `⏎ resolve` leads (Proposed change 2a).
            hint("⏎", "resolve"),
            hint("M", "mergetool"),
            hint("e", "edit"),
            hint("space", "resolved"),
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

/// The resolver's own key list (`docs/design-notes/m6-conflicts.md`, "New
/// UI" footer mockups): `o <ours>  t <theirs>  b both  [B base]  e edit
/// O/T file  n next  N prev  ⏎ done  j/k file  M mergetool  [d start over]  esc close`,
/// `B base` after `b both` only while `base` (Proposed change 2b), `d start
/// over` before `esc close` only while `restart` (Proposed change 2c).
/// `o`/`t` read as `roles` ([`SideRoles`]): `o upstream  t yours` in a
/// rebase, never git's own `ours`/`theirs`.
///
/// Live since m6-conflicts/04, which built `update::resolver` — every one
/// of these is answered by `resolver_key`, the two in brackets included
/// since m6-conflicts/05 bound them (`toggle_base`, `restart_box`).
fn resolver_list(base: bool, restart: bool, roles: SideRoles) -> Vec<Hint> {
    let mut list = vec![
        hint("o", roles.ours()),
        hint("t", roles.theirs()),
        hint("b", "both"),
        // `Ctrl+O`/`Ctrl+T` as one hint (P3, user 2026-09-23; review
        // m6-amend-16 M2): `o`/`t` just before already name the roles.
        hint("⌃O/⌃T", "all"),
        hint("O/T", "file"),
    ];
    if base {
        list.push(hint("B", "base"));
    }
    list.extend([
        hint("e", "edit"),
        hint("n", "next"),
        hint("N", "prev"),
        hint("⏎", "done"),
        hint("j/k", "file"),
        hint("M", "mergetool"),
    ]);
    if restart {
        list.push(hint("d", "start over").keeps_words());
    }
    list.push(hint("esc", "close").keeps_words());
    list
}

/// A whole-file card's own list (m6-conflicts/05, m6-05 review Major 1):
/// `o <ours>  t <theirs>  e edit  n next  M mergetool  esc close`, `o`/`t`
/// read as `roles` ([`SideRoles`]), as on [`resolver_list`].
///
/// Only the keys a card answers. `b` is refused in words (there is no
/// "both" of a path), `⏎` has nothing to mark — `o`/`t` already stage —
/// `B` has no blocks to show a base in, `d` has no markers to bring back,
/// and `N` has no previous file to walk to. `n` crosses to the next
/// conflicted file, as it does from a document's last block.
fn resolver_card_list(roles: SideRoles) -> Vec<Hint> {
    vec![
        hint("o", roles.ours()),
        hint("t", roles.theirs()),
        hint("e", "edit"),
        hint("n", "next"),
        hint("M", "mergetool"),
        hint("esc", "close").keeps_words(),
    ]
}

/// A done file, read-only (m6-conflicts/25): `d start over  e edit  esc
/// close`, or `e edit  esc close` where ops cannot start it over
/// (m6-conflicts/28: a key shown must work). Every other resolver key is
/// refused there in words.
fn resolver_done_list(restart: bool) -> Vec<Hint> {
    let mut list = Vec::with_capacity(3);
    if restart {
        list.push(hint("d", "start over").keeps_words());
    }
    list.push(hint("e", "edit"));
    list.push(hint("esc", "close").keeps_words());
    list
}

/// The finish summary's keys (keymap §2.6b), by focus (m6-conflicts/19),
/// drawn in the summary's own bottom border (m6-conflicts/59). `y` commits
/// from the files, the one confirm key of every preview (P12); `⏎` commits
/// from the subject; in the body it is a newline, so `⌃⏎` commits there.
/// `⇥` names where focus goes next. `⏎/space diff` (both the same toggle,
/// named like `^O/^T all`) is drawn only on a row git combined, the only rows whose
/// change he has not seen.
///
/// Bound by `graft`'s `update::finish::finish_key` (m6-conflicts/34, the
/// app half of ticket 19).
fn finish_list(focus: SummaryFocus, expandable: bool) -> Vec<Hint> {
    match focus {
        SummaryFocus::Files if expandable => vec![
            hint("y", "commit"),
            hint("⏎/space", "diff"),
            hint("j/k", "move"),
            hint("⇥", "message"),
            hint("esc", "back"),
        ],
        SummaryFocus::Files => vec![
            hint("y", "commit"),
            hint("j/k", "move"),
            hint("⇥", "message"),
            hint("esc", "back"),
        ],
        SummaryFocus::Subject => vec![
            hint("⏎", "commit"),
            hint("⇥", "body"),
            hint("⌃E", "editor"),
            hint("esc", "back"),
        ],
        SummaryFocus::Body => vec![
            hint("⌃⏎", "commit"),
            hint("⇥", "files"),
            hint("⌃E", "editor"),
            hint("esc", "back"),
        ],
    }
}
