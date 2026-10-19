//! The whole bar per context before the width rule: the list filters and wrappers (`whole_bar`, `drawable`) (m8/21b-1 split from `hints.rs`).

use super::{
    ACTIONS, DOORWAY, Divider, EXPLAIN, Hint, HintContext, HintKey, HintState, Keymap, LiveC,
    RESIZE, TAIL, WayToBanner, context_list, doorway_hint, hint, hint_keys, lists,
};

/// The bar `context` would draw on a row with no end to it, and the doorway
/// that leads whatever the width drops. [`hints_for`] is this plus the width
/// rule — the split exists so that a context which *filters* another
/// ([`HintContext::PaneFocused`]) can take the whole list away and hand back
/// less **before** the ladder runs. Filtering after it would leave a gap in
/// a row the ladder had already fitted.
pub(super) fn whole_bar(
    context: &HintContext,
    divider: Divider,
    keys: &Keymap,
) -> (Vec<Hint>, Option<Hint>) {
    match context {
        HintContext::Preview => (Vec::new(), None),
        // No width tier for the banner: every key it answers draws while
        // the width allows it, `⇥ graph` — the way out of the mode —
        // included. Three keys at 80 columns would have hidden the exit of
        // a modal that swallows every other key, which is the failure
        // `banner_key`'s own doc comment warns of.
        // A drag takes no doorway either: `.` cancels the drag. The bars
        // where the keys are the point (ADR-0058 §4) swallow `?` or type it,
        // so a doorway there would be a dead key.
        HintContext::Banner { .. }
        | HintContext::BannerPaused { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Picker
        | HintContext::PickerOnCurrent
        | HintContext::Resolver { .. }
        | HintContext::Clash(_)
        | HintContext::Finish { .. }
        | HintContext::Review(_)
        // The rewrite editor is modal (keymap §2.7): no doorway.
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage => (drawable(context, divider), None),
        // The places, and the pane over an overlay, are modal for keys but
        // not for the bar (m8/11c decision 1): `?` answers before any of
        // them (`mergewright::update::keys`, `palette::open_key`) and lists the
        // place's own keys, so they end with the doorway. No `. more`: `.`
        // is swallowed there. The journal, stash and Map draw their own
        // footers, which take the doorway from [`doorway_words`] (m8/11e).
        HintContext::PaneOverOverlay { .. }
        | HintContext::Blame { .. }
        | HintContext::History { .. } => (drawable(context, divider), Some(DOORWAY)),
        HintContext::Marked(inner) => (marked_list(inner, divider, keys), None),
        HintContext::Searching(inner) => searching_bar(inner, divider, keys),
        // The inner context's own bar, minus the key the focused pane has
        // nothing left to do with. Nothing else changes: every other key on
        // a row context is still answered with the pane up — the graph's
        // own keys fall through to it, and the pane answers the motions
        // itself (m5/31).
        HintContext::PaneFocused {
            row,
            opens,
            blamable,
            folds,
            graph_hidden,
        } => off_the_graph(
            pane_focused_bar(row, *opens, *blamable, *folds, divider, keys),
            *graph_hidden,
        ),
        HintContext::BackToBlame(inner) => led_by(BACK_TO_BLAME, inner, divider, keys),
        HintContext::BackToHistory(inner) => led_by(BACK_TO_HISTORY, inner, divider, keys),
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow { .. }
        | HintContext::GraphRow { .. }
        | HintContext::StashRow
        | HintContext::Rewritable(_)
        | HintContext::ChangesPane { .. } => (
            actions_unless_over_overlay(context, drawable(context, divider)),
            Some(DOORWAY),
        ),
        // The row's own bar with its `c` said the way it acts now, and —
        // while one `⇥` reaches the banner — the way back leading, so the
        // ladder's floor always holds it. Then the row's own `⇥` goes: one
        // key, one meaning, and there `Tab` lands on the banner. Without
        // a one-press way the row's `⇥` (if any) stays as the row says it.
        // One `⇧⇥` back (graph, pane open, m8/20d) leads with `⇧⇥ banner`.
        HintContext::OperationLive {
            row,
            c,
            way_to_banner,
            space_stages,
        } => live_bar(row, *c, *way_to_banner, *space_stages, divider, keys),
    }
}

/// [`whole_bar`]'s [`HintContext::OperationLive`] arm (moved out whole,
/// ease-pass/12, to keep `whole_bar` under clippy's line limit).
fn live_bar(
    row: &HintContext,
    c: LiveC,
    way_to_banner: Option<WayToBanner>,
    space_stages: bool,
    divider: Divider,
    keys: &Keymap,
) -> (Vec<Hint>, Option<Hint>) {
    let (list, trailer) = whole_bar(row, divider, keys);
    let list = lists::said_live_on_the_graph(list, c, space_stages);
    let way = match way_to_banner {
        None => return (list, trailer),
        Some(WayToBanner::Forward) => WAY_TO_BANNER_FORWARD,
        Some(WayToBanner::Back) => WAY_TO_BANNER_BACK,
    };
    let mut led: Vec<Hint> = list.into_iter().filter(|hint| !hint.is(way.keys)).collect();
    led.insert(0, way);
    (led, trailer)
}

/// A marked row's list: `. explain`, the row's own list, and `? keys`.
///
/// `.` is already on screen as `. explain`, so no `. more`; it is prepended
/// before fitting and not after, because a list fitted first and prepended
/// to second overflows by one hint. `? keys` is an ordinary candidate, not
/// a trailer (m8/11d R2): ranked right after the later of `u undo` and
/// `/ search`, so it never costs either (a trailer cost `u undo` at 83
/// cells, 11b), and the hints ranked after it are what `?` recovers.
/// [`hints_for`] then draws it last ([`doorway_last`]).
///
/// The doorway is [`doorway_hint`]'s, spelled before ranking: `⌃P keys`
/// when `help` is unbound ranks where `? keys` did; both unbound, none.
pub(super) fn marked_list(inner: &HintContext, divider: Divider, keys: &Keymap) -> Vec<Hint> {
    let mut list = vec![EXPLAIN];
    list.extend(drawable(inner, divider));
    let at = list
        .iter()
        .rposition(|hint| hint.is(hint_keys::UNDO) || hint.is(hint_keys::FIND))
        .map_or(list.len(), |at| at + 1);
    if let Some(door) = doorway_hint(keys) {
        list.insert(at, door);
    }
    list
}

/// A focused pane's bar on the Detail tab (`graph_hidden`) less
/// `1 first-parent`, whose effect is on the hidden graph (m8/11b decision 3,
/// ADR-0041 carve-out).
pub(super) fn off_the_graph(
    (mut list, trailer): (Vec<Hint>, Option<Hint>),
    graph_hidden: bool,
) -> (Vec<Hint>, Option<Hint>) {
    if graph_hidden {
        list.retain(|hint| !hint.is(hint_keys::FIRST_PARENT));
    }
    (list, trailer)
}

/// `list` with `. actions` last: ranked after every other candidate, so the
/// width drops it first (m8/11b decision 1).
pub(super) fn with_actions(mut list: Vec<Hint>) -> Vec<Hint> {
    list.push(ACTIONS);
    list
}

/// [`with_actions`], except for the changes pane over an overlay (m8/09e,
/// 11b row 553): there `.` is the graph's menu key, reached through the
/// graph, which the overlay swallows (`mergewright::update::keys::graph_key`) —
/// no `. actions`. `?` answers first (`open_key`), so the doorway stays.
pub(super) fn actions_unless_over_overlay(context: &HintContext, list: Vec<Hint>) -> Vec<Hint> {
    match context {
        HintContext::ChangesPane {
            over_overlay: true, ..
        } => list,
        _ => with_actions(list),
    }
}

/// [`whole_bar`] for [`HintContext::Searching`] (m8/15b): `n next  N prev`
/// lead the row's own bar, since the search is what he is moving through
/// now. Any `n`/`N` the row had goes, so one key draws once.
pub(super) fn searching_bar(
    inner: &HintContext,
    divider: Divider,
    keys: &Keymap,
) -> (Vec<Hint>, Option<Hint>) {
    let (list, trailer) = whole_bar(inner, divider, keys);
    let mut led = vec![SEARCH_NEXT, SEARCH_PREV];
    led.extend(list.into_iter().filter(|hint| {
        !hint.is(hint_keys::NEXT_MATCH)
            && !hint.is(hint_keys::PREV_MATCH)
            && !hint.is(hint_keys::NEXT_PREV_MATCH)
    }));
    (led, trailer)
}

/// `inner`'s whole bar with `lead` in front: the way back to a place.
pub(super) fn led_by(
    lead: Hint,
    inner: &HintContext,
    divider: Divider,
    keys: &Keymap,
) -> (Vec<Hint>, Option<Hint>) {
    let (list, trailer) = whole_bar(inner, divider, keys);
    let mut bar = vec![lead];
    bar.extend(list);
    (bar, trailer)
}

/// [`whole_bar`] for [`HintContext::PaneFocused`]: the row's own bar minus
/// the keys the focused pane leaves dead, plus the pane's own `B` and
/// `space`.
pub(super) fn pane_focused_bar(
    row: &HintContext,
    opens: bool,
    blamable: bool,
    folds: bool,
    divider: Divider,
    keys: &Keymap,
) -> (Vec<Hint>, Option<Hint>) {
    let (list, trailer) = whole_bar(row, divider, keys);
    // With a pane focused (its files list or its diff) `space` is the pane's,
    // so the graph's merge fold is not live (m7-power-tools/20h, 20l). Only
    // that hint goes: another list's `space` is its own.
    let mut list: Vec<Hint> = list
        .into_iter()
        .filter(|hint| {
            !GRAPH_ONLY.contains(&hint.keys)
                && (opens || !hint.is(hint_keys::OPEN))
                && *hint != lists::GRAPH_FOLD
        })
        .collect();
    if blamable {
        // On a file `F` is the pane's history, not the graph's Focus
        // (`mergewright::update::place::f_key`, guarded on the same `blamable`),
        // so [`TAIL`]'s `F focus` goes and `F history` follows `B blame`
        // (m7-power-tools/21b).
        list.retain(|hint| !hint.is(hint_keys::FOCUS));
        // After `⏎` when it is there, else first: the pane's own key.
        let at = usize::from(list.first().is_some_and(|hint| hint.is(hint_keys::OPEN)));
        list.insert(at, BLAME);
        list.insert(at + 1, F_HISTORY);
    }
    if folds {
        // After `⏎` and `B` when present, else first.
        let at = list
            .iter()
            .rposition(|hint| hint.is(hint_keys::OPEN) || *hint == BLAME)
            .map_or(0, |at| at + 1);
        list.insert(at, FOLD);
    }
    (list, trailer)
}

/// `B blame`: the pane's line blamed ([`HintContext::PaneFocused`]).
pub(super) const BLAME: Hint = hint(hint_keys::BLAME, "blame");

/// `space fold`: the graph's merge fold ([`TAIL`]) and, with the same word
/// for the same act, a focused files list's directory fold
/// ([`HintContext::PaneFocused`]'s `folds`, m7-power-tools/20h).
pub(super) const FOLD: Hint = hint(hint_keys::FOLD_DIR, "fold");

/// `esc back to blame`: the way back to a hidden blame trail
/// ([`HintContext::BackToBlame`]).
pub(super) const BACK_TO_BLAME: Hint = hint(hint_keys::BACK, "back to blame");

/// `esc back to history`: the way back to a hidden history place
/// ([`HintContext::BackToHistory`], m7-power-tools/21b).
pub(super) const BACK_TO_HISTORY: Hint = hint(hint_keys::BACK, "back to history");

/// `F history`: the focused pane's file traced
/// ([`HintContext::PaneFocused`]'s `blamable`, m7-power-tools/21b).
pub(super) const F_HISTORY: Hint = hint(hint_keys::FILE_HISTORY, "history");

/// `n next`: the search's next match ([`HintContext::Searching`], 15b).
pub(super) const SEARCH_NEXT: Hint = hint(hint_keys::NEXT_MATCH, "next");

/// `N prev`: the search's previous match ([`HintContext::Searching`], 15b).
pub(super) const SEARCH_PREV: Hint = hint(hint_keys::PREV_MATCH, "prev");

/// `⇥ banner`: the way forward to a live operation's banner
/// ([`HintContext::OperationLive`]).
pub(super) const WAY_TO_BANNER_FORWARD: Hint = hint(hint_keys::NEXT_PANE, "banner");

/// `⇧⇥ banner`: the way to the banner one `⇧⇥` away — the graph with the
/// detail pane open (m8/20d), where `⇥` takes three.
pub(super) const WAY_TO_BANNER_BACK: Hint = hint(hint_keys::PREV_PANE, "banner");

/// The keys only the graph screen answers: each opens something that stands
/// *where the graph stands*, and each is guarded on the graph having the
/// keyboard, so each does nothing while the detail pane has it
/// (`mergewright::update::state::AppState::graph_has_focus`, the one function all
/// five handlers read).
pub(super) const GRAPH_ONLY: [&[HintKey]; 5] = [
    hint_keys::STASH,
    hint_keys::JOURNAL,
    hint_keys::MAP,
    hint_keys::OVERVIEW,
    hint_keys::SWITCH,
];

/// The keys that need a commit under the cursor, and so do nothing on the
/// uncommitted-changes row: `y` copies a sha there is none of and `x` marks
/// a compare base there is no oid for. Both return at their first line on
/// that row (`mergewright::update::keys::copy_selected_sha`,
/// `toggle_compare_selected`), which the design note already said and the
/// bar drew anyway (m5/31). `space` folds the merge under the cursor, and
/// the uncommitted row is no merge (gap row 553).
pub(super) const NEEDS_A_COMMIT: [&[HintKey]; 3] =
    [hint_keys::COPY_SHA, hint_keys::COMPARE, hint_keys::FOLD];

/// What an ease-pass row bar ([`HintContext::GraphRow`], bare or under
/// [`HintContext::Rewritable`]) draws of [`TAIL`]:
/// `u undo`, the mock's bar end (03's Handoff).
const ROW_BAR_TAIL: [Hint; 1] = [hint(hint_keys::UNDO, "undo")];

/// Every hint the footer could draw for `context`, in priority order, with
/// the ones nothing has built yet already dropped: the context's own keys,
/// then — on the graph's row contexts only — the keys the whole screen
/// answers ([`TAIL`]), then the divider's when a pane is open.
///
/// A pane keeps to its own list: while the changes pane, an overlay or the
/// banner has focus it answers keys the graph does not, and the graph
/// answers keys it does not, so [`TAIL`] would be advertising the wrong
/// screen.
///
/// The focused banner goes further and takes no divider hint either: it is
/// modal, so `<>` is as dead as every other key it swallows. (Under 100
/// columns the angle keys are a no-op in any case —
/// `mergewright::update::detail`'s own rule for the tab layout.)
pub(super) fn drawable(context: &HintContext, divider: Divider) -> Vec<Hint> {
    let mut list: Vec<Hint> = context_list(context)
        .into_iter()
        .filter(|hint| hint.state == HintState::Live)
        .collect();
    let rows = matches!(
        context,
        HintContext::WorkingTreeRow
            | HintContext::BranchRow { .. }
            | HintContext::PlainRow { .. }
            | HintContext::GraphRow { .. }
            | HintContext::StashRow
            | HintContext::Rewritable(_)
    );
    if rows {
        let commits = *context != HintContext::WorkingTreeRow;
        // An ease-pass row bar is the mock's (G13): its own words, then
        // `u undo`, then the doorway — the screen's other keys are `?`'s,
        // so a named verb is not crowded to its short rung by them.
        // Under `R rewrite`'s wrapper too (ease-pass/07b): the row is the
        // same row, so its bar is the same mock bar.
        let tail: &[Hint] = if context.is_graph_row_bar() {
            &ROW_BAR_TAIL
        } else {
            &TAIL
        };
        for extra in tail.iter().cloned() {
            let dead = !commits && NEEDS_A_COMMIT.contains(&extra.keys);
            if !dead && !list.iter().any(|hint| hint.is(extra.keys)) {
                list.push(extra);
            }
        }
    }
    if divider == Divider::Shown
        && !matches!(
            context,
            HintContext::Banner { .. }
                | HintContext::BannerPaused { .. }
                | HintContext::Dragging
                | HintContext::Picker
                | HintContext::PickerOnCurrent
                | HintContext::PaneOverOverlay { .. }
                | HintContext::Resolver { .. }
                | HintContext::Clash(_)
                | HintContext::Finish { .. }
                | HintContext::Review(_)
                | HintContext::Blame { .. }
                | HintContext::History { .. }
                | HintContext::RewriteEditor { .. }
                | HintContext::RewriteMessage
        )
    {
        // Last of the bar's own keys, after [`TAIL`] (ticket m8/10, ruling
        // R1): a layout key earns its cell last, so it is the first the
        // width drops and nothing else pays for its width — not `u undo`,
        // not `/ search` (02-keymap §2.1 "ranks by relevance at the
        // cursor"; ux-principles row 8). It used to sit fourth so the old
        // six-hint tier would reach it; that tier is gone, and fourth let one
        // cell of its own spelling push undo off at 100 columns.
        list.push(RESIZE);
    }
    list
}
