//! The design note's priority table: one list of hints per context, and
//! [`TAIL`], what the whole screen answers after them.

use super::keys::{
    ABORT, AMEND, BACK, BANNER_EDIT, BANNER_MERGETOOL, BRANCH, COMMIT, COMPARE, CONFIRM_CHOOSE,
    CONFIRM_MOVE, CONFIRM_YES, CONTINUE, COPY_SHA, CREDENTIAL_SEND, DISCARD, DOWN_UP, DROP, EDIT,
    FETCH, FIELD_EDITOR, FIELD_ENTER, FIELD_ESC, FIELD_TAB, FIND, FIRST_PARENT, FIXUP, FOCUS, FOLD,
    HEATMAP, HIDE_STASH, IGNORE_REVS, JOURNAL, LANES, LINE_HISTORY, MAP, MERGE, MERGETOOL,
    MOVE_ROW, NEXT_PANE, NEXT_PREV_CHANGE, NEXT_PREV_MATCH, OPEN, OVERVIEW, PARENT, PICK,
    PICKER_ROWS, PICKER_SHOW, PICKER_SWITCH, PULL, PUSH, QUIT, REBASE, RELEASE, RENDERER, REWORD,
    REWRITE as REWRITE_KEY, REWRITE_CHANGES, REWRITE_EDIT, REWRITE_START, SKIP, SQUASH, STAGE,
    STAGE_ALL, STASH, STATUS, SWITCH, TAG, TAKE, TYPE, UNDO, WHITESPACE,
};
use super::{Hint, HintContext, HintKey, LiveC, RowVerbs, TabWord, hint};
use crate::keymap::ActionId;

mod resolver;

use resolver::{finish_list, resolver_context_list};

/// A confirm box's bar: `y <acting label>` first when `y` takes a button.
fn confirm_box_list(yes: Option<super::ConfirmYes>) -> Vec<Hint> {
    let mut list = Vec::new();
    if let Some(yes) = yes {
        list.push(hint(CONFIRM_YES, yes.word()));
    }
    list.extend([
        hint(CONFIRM_MOVE, "move"),
        hint(CONFIRM_CHOOSE, "choose"),
        hint(BACK, "cancel"),
    ]);
    list
}

/// The name prompt's bar; `⇥ toggle` only where the prompt has a switch.
fn name_prompt_list(toggle: bool) -> Vec<Hint> {
    let mut list = vec![hint(FIELD_ENTER, "create")];
    if toggle {
        list.push(hint(FIELD_TAB, "toggle"));
    }
    list.push(hint(FIELD_ESC, "cancel"));
    list
}

/// The design note's priority table, one context at a time, including the
/// entries no group has built yet ([`drawable`] filters those out).
/// `Marked` and `Preview` are handled in [`hints_for`], not here.
pub(super) fn context_list(context: &HintContext) -> Vec<Hint> {
    match context {
        HintContext::WorkingTreeRow => working_tree_row_list(),
        HintContext::BranchRow {
            current: false,
            verbs,
        } => with_verbs(branch_row_list(), *verbs),
        HintContext::BranchRow {
            current: true,
            verbs,
        } => with_verbs(current_branch_row_list(), *verbs),
        HintContext::PlainRow { verbs } => with_verbs(plain_row_list(), *verbs),
        HintContext::StashRow => stash_row_list(),
        HintContext::Banner {
            skippable,
            conflicted,
            merge,
        } => banner_list(*skippable, *conflicted, *merge),
        HintContext::ChangesPane {
            unmerged,
            live,
            tab,
            over_overlay,
        } => changes_pane_list(*unmerged, *live, *tab, *over_overlay),
        HintContext::Resolver { .. } => resolver_context_list(context),
        HintContext::Finish { focus, expandable } => finish_list(*focus, *expandable),
        HintContext::Blame {
            finding,
            ignore_file,
            lines,
        } => blame_bar(*finding, *ignore_file, *lines),
        HintContext::History { rows, on_file } => history_place_list(*rows, *on_file),
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
        HintContext::Dragging => vec![hint(RELEASE, "drop on a row"), hint(BACK, "cancel")],
        // `y` leads, as it leads the preview's keys (ADR-0044 §3.1), and
        // names the acting button's own label: it takes that button from
        // wherever the cursor is. A box `y` does not take draws no `y`.
        HintContext::ConfirmBox { yes } => confirm_box_list(*yes),
        HintContext::NamePrompt { toggle } => name_prompt_list(*toggle),
        HintContext::Credential => vec![hint(CREDENTIAL_SEND, "send"), hint(FIELD_ESC, "cancel")],
        HintContext::Rewritable(row) => with_rewrite(context_list(row)),
        HintContext::RewriteEditor { narrow, can_start } => {
            rewrite_editor_list(*narrow, *can_start)
        }
        // Keymap §2.7: the field is a text field; these three are what the
        // non-letter keys do (`graft::update::rewrite_editor` `field_key`).
        HintContext::RewriteMessage => vec![
            hint(FIELD_ENTER, "keep"),
            hint(FIELD_EDITOR, "$EDITOR"),
            hint(FIELD_ESC, "drop the edit"),
        ],
        // All three are answered in `whole_bar` before this is reached; a
        // `RowContext` sub-enum would say so in the type, at the cost of
        // wrapping every row context at every call site.
        HintContext::Marked(_)
        | HintContext::Searching(_)
        | HintContext::PaneFocused { .. }
        | HintContext::BackToBlame(_)
        | HintContext::BackToHistory(_)
        | HintContext::OperationLive { .. }
        | HintContext::Preview => Vec::new(),
    }
}

/// A commit row's list with the verbs that act on this row
/// (m7-power-tools/40): `o take` right after `⏎ open`, `a amend` right after
/// the last of `c`/`P`/`p`/`f` (after `⏎ open` where the list has none). Both are
/// worked out by `graft` (`view::rows::row_verbs`), from the same checks the
/// keys run, so the bar never hints a row the key refuses.
pub(super) fn with_verbs(mut list: Vec<Hint>, verbs: RowVerbs) -> Vec<Hint> {
    let after = |list: &[Hint], keys| list.iter().position(|h| h.is(keys)).map(|i| i + 1);
    if verbs.take {
        let at = after(&list, OPEN).unwrap_or(0);
        list.insert(at, hint(TAKE, "take"));
    }
    if verbs.amend {
        // After the last of the daily verbs the row holds (planner ruling
        // 2026-10-01): `c commit`, `P push`, `p pull`, `f fetch`.
        let at = list
            .iter()
            .rposition(|h| [COMMIT, PUSH, PULL, FETCH].contains(&h.keys))
            .map(|i| i + 1)
            .or_else(|| after(&list, OPEN))
            .unwrap_or(0);
        list.insert(at, hint(AMEND, "amend"));
    }
    list
}

/// `R rewrite`, on a commit row of his branch (m7-power-tools/08).
const REWRITE: Hint = hint(REWRITE_KEY, "rewrite");

/// A commit row's list with [`REWRITE`] last among the row's own keys,
/// ahead of the whole screen's ([`TAIL`]): a new key takes no rung from the
/// verbs the row already leads with, so the bar's floor (ADR-0037 §4) and
/// every narrower bar stay as they were, and wider ones gain `R rewrite`.
fn with_rewrite(mut list: Vec<Hint>) -> Vec<Hint> {
    list.push(REWRITE);
    list
}

/// The rewrite editor's bar (note R1, keymap §2.7): `y rewrite` leads while
/// it would start, then `tab after` below 120 (the one way to the AFTER
/// side there), then the row keys in R1's order.
fn rewrite_editor_list(narrow: bool, can_start: bool) -> Vec<Hint> {
    let mut list = Vec::new();
    if can_start {
        list.push(hint(REWRITE_START, "rewrite"));
    }
    if narrow {
        list.push(hint(NEXT_PANE, "after"));
    }
    list.extend([
        hint(REWRITE_CHANGES, "changes"),
        hint(PICK, "pick"),
        hint(REWORD, "reword"),
        hint(REWRITE_EDIT, "edit"),
        hint(SQUASH, "squash"),
        hint(FIXUP, "fixup"),
        hint(DROP, "drop"),
        hint(MOVE_ROW, "move"),
        hint(BACK, "cancel"),
    ]);
    list
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
        hint(TYPE, "to find"),
        hint(PICKER_SWITCH, "switch to it"),
        hint(FIELD_ESC, "close"),
        hint(PICKER_SHOW, "show it on the graph"),
        hint(PICKER_ROWS, "row"),
    ]
}

/// The keys the graph screen answers wherever the cursor is, in the order
/// the old fixed list dropped them (PROMPT.md principle 4: the hint bar
/// teaches the tool). Appended after a row context's own keys, so a wide
/// terminal still shows `q quit`, `x compare` and `+ lanes` — which the
/// per-context table leaves out entirely (m5-03 review, Major 3). Whatever
/// the context already hints is not repeated ([`drawable`] dedupes by key).
/// `space fold`: the graph's merge fold, which a focused pane takes back
/// (`hints::pane_focused_bar`).
pub(super) const GRAPH_FOLD: Hint = hint(FOLD, "fold");

pub(super) const TAIL: [Hint; 21] = [
    hint(QUIT, "quit"),
    hint(STATUS, "status"),
    hint(RENDERER, "renderer"),
    hint(UNDO, "undo"),
    hint(COMPARE, "compare"),
    hint(LANES, "lanes"),
    hint(FIND, "search"),
    hint(DOWN_UP, "move"),
    GRAPH_FOLD,
    hint(FIRST_PARENT, "first-parent"),
    hint(FOCUS, "focus"),
    hint(STASH, "stash"),
    hint(HIDE_STASH, "hide stash"),
    hint(FETCH, "fetch"),
    hint(PULL, "pull"),
    hint(PUSH, "push"),
    hint(JOURNAL, "journal"),
    hint(COPY_SHA, "sha"),
    hint(MAP, "map"),
    hint(OVERVIEW, "overview"),
    hint(SWITCH, "switch"),
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
        hint(OPEN, "that commit"),
        hint(PARENT, "before it"),
        hint(LINE_HISTORY, "history of these lines"),
        hint(FIND, "find"),
    ];
    if finding {
        list.push(hint(NEXT_PREV_MATCH, "match"));
    }
    list.extend([hint(HEATMAP, "ages"), hint(WHITESPACE, "whitespace")]);
    if ignore_file {
        list.push(hint(IGNORE_REVS, "ignore list"));
    }
    list.push(hint(BACK, "back"));
    list
}

/// [`blame_place_list`] as the bar draws it: without `lines` (a read in
/// flight, or a failed one) the keys that act on a line — `⏎`, `^`, `L`,
/// `n N` — are left out, because the place has no lines for them
/// (m7-power-tools/20k, 20n, 21).
fn blame_bar(finding: bool, ignore_file: bool, lines: bool) -> Vec<Hint> {
    let mut list = blame_place_list(finding, ignore_file);
    if !lines {
        list.retain(|hint| ![OPEN, PARENT, LINE_HISTORY, NEXT_PREV_MATCH].contains(&hint.keys));
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
        return vec![hint(BACK, "back")];
    }
    let (moves, tab) = if on_file {
        (hint(DOWN_UP, "scroll"), hint(NEXT_PANE, "list"))
    } else {
        (hint(DOWN_UP, "older·newer"), hint(NEXT_PANE, "file"))
    };
    let mut list = vec![
        moves,
        tab,
        hint(NEXT_PREV_CHANGE, "next change"),
        hint(super::keys::BLAME, "blame here"),
        hint(OPEN, "that commit"),
    ];
    // No form offers `v`, so `w` sits in the file's form, before the way
    // back (21c); it answers from either focus.
    if on_file {
        list.push(hint(WHITESPACE, "whitespace"));
    }
    list.push(hint(BACK, "back"));
    list
}

/// [`HintContext::PaneOverOverlay`]'s list: `⏎ open` unless the diff has
/// focus, then the pane's moves and `u undo`, dispatched above it. Not
/// `/ search`: the graph's search stands down under an overlay (m8/09e).
fn pane_over_overlay_list(opens: bool) -> Vec<Hint> {
    let mut list = Vec::new();
    if opens {
        list.push(hint(OPEN, "open"));
    }
    list.extend([hint(DOWN_UP, "move"), hint(UNDO, "undo")]);
    list
}

fn working_tree_row_list() -> Vec<Hint> {
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

fn branch_row_list() -> Vec<Hint> {
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

fn current_branch_row_list() -> Vec<Hint> {
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

fn plain_row_list() -> Vec<Hint> {
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
fn stash_row_list() -> Vec<Hint> {
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
    let mut list = vec![hint(CONTINUE, if merge { "finish" } else { "continue" })];
    if skippable {
        list.push(hint(SKIP, "skip"));
    }
    list.push(hint(ABORT, "abort"));
    if conflicted {
        // `⏎ resolve` leads (Proposed change 2a): live since
        // m6-conflicts/04, which binds it in `update::banner::banner_key`.
        list.insert(0, hint(OPEN, "resolve"));
        list.push(hint(BANNER_MERGETOOL, "mergetool"));
        list.push(hint(BANNER_EDIT, "edit"));
    }
    list.push(hint(NEXT_PANE, "graph"));
    // Last, so the first dropped on a narrow bar: the banner's own keys
    // matter more than a way out he can always reach by `⇥` (m8-13b).
    // `q` boxes from here (`graft::update::banner::banner_key`).
    list.push(hint(QUIT, "quit"));
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
fn changes_pane_list(
    unmerged: bool,
    live: Option<LiveC>,
    tab: TabWord,
    over_overlay: bool,
) -> Vec<Hint> {
    if over_overlay {
        return changes_pane_over_overlay_list(unmerged, tab);
    }
    let list = changes_pane_table(unmerged, tab);
    match live {
        Some(c) => said_live(list, c),
        None => list,
    }
}

/// The changes pane's list with the Map, journal or stash list up behind
/// it (m8/09e, 11b row 553): the pane's own keys, which it answers before
/// the overlay, without `c` in any form — `c` is the graph's verb reached
/// through the graph, and the overlay swallows it — so `live` is not read.
fn changes_pane_over_overlay_list(unmerged: bool, tab: TabWord) -> Vec<Hint> {
    changes_pane_table(unmerged, tab)
        .into_iter()
        .filter(|hint| !hint.is(COMMIT))
        .collect()
}

/// `list` with its `c commit` said the way `c` acts while an operation is
/// live (m6-conflicts/52), in the same place: `c finish`, `c continue`, or
/// gone where `c` only refuses in words. Every other hint is untouched.
pub(super) fn said_live(list: Vec<Hint>, c: LiveC) -> Vec<Hint> {
    // At an `edit` stop `a` amends the stopped commit, and it leads: it is
    // the step the stop is for (07b, note R4's bar).
    let mut said = match c {
        LiveC::ContinueAmends => vec![AMEND_IT],
        LiveC::Finish | LiveC::Continue | LiveC::Refuses => Vec::new(),
    };
    said.extend(list.into_iter().filter_map(|hint| {
        if hint.is(AMEND_IT.keys) && c == LiveC::ContinueAmends {
            return None;
        }
        if !hint.is(COMMIT) {
            return Some(hint);
        }
        match c {
            LiveC::Finish => Some(Hint {
                verb: "finish",
                ..hint
            }),
            LiveC::Continue | LiveC::ContinueAmends => Some(Hint {
                verb: "continue",
                ..hint
            }),
            LiveC::Refuses => None,
        }
    }));
    said
}

/// `a amend it`, at a rewrite's `edit` stop.
const AMEND_IT: Hint = hint(AMEND, "amend it");

/// `space stage`, at a rewrite's `edit` stop on the rows where `a` amends
/// (m7-power-tools/07d D1).
const SPACE_STAGE: Hint = hint(FOLD, "stage");

/// A graph row's bar while an operation is live: [`said_live`]'s list with
/// `c` ranked first after `a amend it` — above the graph's write keys,
/// which a live operation refuses (review m7-10-to-07c Minor 7), so the
/// width ladder drops them before the key that finishes — and, where
/// `space_stages`, `space stage` right after it in place of `space fold`.
pub(super) fn said_live_on_the_graph(list: Vec<Hint>, c: LiveC, space_stages: bool) -> Vec<Hint> {
    let said = said_live(list, c);
    let (c_hint, rest): (Vec<Hint>, Vec<Hint>) = said.into_iter().partition(|hint| hint.is(COMMIT));
    let mut rest = rest.into_iter().peekable();
    let mut ranked = Vec::new();
    if c == LiveC::ContinueAmends {
        ranked.extend(rest.next_if(|hint| hint.is(AMEND_IT.keys)));
    }
    ranked.extend(c_hint);
    if space_stages {
        ranked.push(SPACE_STAGE);
    }
    ranked.extend(rest.filter(|hint| !(space_stages && hint.is(SPACE_STAGE.keys))));
    ranked.retain(|hint| !refused_while_live(hint));
    ranked
}

/// The graph's write keys that refuse while an operation is live
/// (m8-polish-and-ship/15 A1, keymap §2.1 mode rule: a key that can only
/// say "no" is not offered). Each member's refusing line, in `graft`:
/// - `app.undo` — `update::undo::start_undo` → `in_progress_refusal`
///   (`update/undo.rs:344`, words from `live_refusal_words`). The one
///   exception, a merge begun in the shell (keymap §2.6a), is not told
///   apart by the bar's context, so `u` is not offered there either: a key
///   left out is recoverable through `?`, a key that refuses is a lie.
/// - `graph.merge`, `graph.rebase` — `preview::history_key` →
///   `open_preview` → `preview_refusal` (`update/preview.rs:380`); on a
///   detached `HEAD` (an `edit` stop) `m` refuses earlier
///   (`update/preview/history.rs:112`).
/// - `graph.take` — `preview::take_key` → `open_preview` → `preview_refusal`.
/// - `graph.rewrite` — `rewrite_editor::open_rewrite` → `preview_refusal`
///   (`update/rewrite_editor/open.rs:48`).
///
/// Held against the app by `graft`'s
/// `no_key_a_live_operation_bar_draws_refuses_while_live`.
const REFUSED_WHILE_LIVE: &[ActionId] = &[
    ActionId("app.undo"),
    ActionId("graph.merge"),
    ActionId("graph.rebase"),
    ActionId("graph.take"),
    ActionId("graph.rewrite"),
];

/// Whether `hint` names a [`REFUSED_WHILE_LIVE`] key.
fn refused_while_live(hint: &Hint) -> bool {
    hint.keys.iter().any(|key| {
        matches!(key, HintKey::Key(id) | HintKey::EveryChord(id) if REFUSED_WHILE_LIVE.contains(id))
    })
}

fn changes_pane_table(unmerged: bool, tab: TabWord) -> Vec<Hint> {
    if unmerged {
        vec![
            // `⏎ resolve` leads (Proposed change 2a).
            hint(OPEN, "resolve"),
            hint(MERGETOOL, "mergetool"),
            hint(EDIT, "edit"),
            hint(STAGE, "resolved"),
            hint(COMMIT, "commit"),
            hint(NEXT_PANE, tab.word()),
        ]
    } else {
        vec![
            hint(STAGE, "stage"),
            hint(DISCARD, "discard"),
            hint(COMMIT, "commit"),
            hint(STAGE_ALL, "all"),
            hint(EDIT, "edit"),
            hint(NEXT_PANE, tab.word()),
        ]
    }
}
