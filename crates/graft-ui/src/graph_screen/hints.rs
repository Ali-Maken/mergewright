//! The footer's key hints, by context (`hint-bar-redesign`, ADR-0037 §4: the
//! bar is how anything is discoverable; a feature nobody can see doesn't
//! exist). Shows what the cursor or focus can do, filled **from the most
//! relevant end**: as many hints as the bar's own room holds, each on the
//! best of its three rungs — `c commit` → `c cmt` → `c` — that still leaves
//! room for the hints ahead of it ([`ladder`], ADR-0039's third amendment).
//! Never all hidden, never a hint cut in half, and never below the three
//! most relevant at their **floor rung** (ADR-0037 §4).
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
//! `Journal`, `Stash` and `Map`, which the note leaves out, taken from
//! `docs/design/02-keymap.md` §2.11, §2.9 and §2.3a.
//!
//! **A hint is only drawn once its key is bound.** Every entry carries a
//! [`HintState`]: the table stays as the design wrote it, but
//! [`HintState::Planned`] hints are dropped before the width rule
//! runs, so the bar never teaches a key that does nothing (m5-03 review,
//! Critical 2). `graft` proves both halves —
//! `update::tests::every_key_the_bar_draws_is_answered` (Live ⇒ bound, key
//! *and* verb, **and answered in the state the bar is drawn in**)
//! and `update::tests::every_planned_hint_is_still_unbound` (Planned ⇒ not
//! bound, so binding one fails the build until the table is flipped).

mod catalogue;
mod context;
mod fitting;
mod lists;

pub use catalogue::{every_context, every_hint};
pub use context::{ConfirmYes, HintContext, LiveC, TabWord};
use fitting::fit_list;
pub use fitting::{StopWording, fit_status_line, hints_text, stopped_row, stopped_row_before_keys};
pub use lists::blame_place_list;
use lists::{TAIL, context_list};

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

/// How much of a hint the bar had room to draw — ADR-0039's ladder
/// ("shrink before dropping"), corrected by its third amendment: a hint
/// steps down its own rungs, never the whole list together.
///
/// Ordered worst to best, so `<` means "less of the hint".
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// The key on its own: `S`. What the doorway recovers.
    Key,
    /// The key and the user's short label: `c cmt`. A verb with no short
    /// form draws its whole verb here — its middle rung *is* its full form.
    Short,
    /// The key and the whole verb: `c commit`.
    Full,
}

/// One hint: a key, the verb it does, whether it does it yet, and how much
/// of it the bar drew.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hint {
    /// The key as it reads on screen (`m`, `⏎`, `⌃A`, `.`).
    pub key: &'static str,
    /// The verb it does (`merge`, `open`, `more`).
    pub verb: &'static str,
    /// Bound today, or still to be built.
    pub state: HintState,
    /// The rung [`hints_for`] drew it at. Every table entry is [`Rung::Full`];
    /// only the width rule steps one down, so `verb` always names the whole
    /// verb whatever the rung (which is what `graft`'s
    /// `update::tests::every_key_the_bar_draws_is_answered` compares against the
    /// keymap).
    pub rung: Rung,
    /// Keeps its words ahead of the other hints' words (ADR-0044 §4): once
    /// every hint is placed at its bare key, the ladder raises this one to
    /// [`Rung::Full`] before any other, so a narrow bar keeps the way out
    /// (`d start over`, `esc close`) in words without hiding a key
    /// (m6-conflicts/64). Only the resolver's lists mark one.
    pub keeps_words: bool,
}

impl Hint {
    /// The same hint, marked to keep its words ([`Hint::keeps_words`]).
    #[must_use]
    pub const fn keeps_words(self) -> Self {
        Self {
            keeps_words: true,
            ..self
        }
    }

    /// The same hint, drawn at `rung`.
    const fn at(mut self, rung: Rung) -> Self {
        self.rung = rung;
        self
    }

    /// What the bar draws for this hint at its rung, key included.
    #[must_use]
    pub fn text(&self) -> String {
        match self.drawn_verb() {
            Some(verb) => format!("{} {verb}", self.key),
            None => self.key.to_owned(),
        }
    }

    /// The verb as the bar draws it at this hint's rung: `None` at
    /// [`Rung::Key`], where only the key is drawn. What [`Hint::text`] puts
    /// after the key and its space, for a caller that styles the two apart.
    #[must_use]
    pub fn drawn_verb(&self) -> Option<&'static str> {
        match self.rung {
            Rung::Key => None,
            Rung::Short => Some(short_form(self.verb).unwrap_or(self.verb)),
            Rung::Full => Some(self.verb),
        }
    }

    /// The rungs this hint really has, worst to best. A verb with no short
    /// form has two, not three: dropping the middle rung is what keeps the
    /// ladder from spending a step on nothing, and what keeps a rung from
    /// meaning two different amounts of text.
    fn rungs(&self) -> &'static [Rung] {
        if short_form(self.verb).is_some() {
            &[Rung::Key, Rung::Short, Rung::Full]
        } else {
            &[Rung::Key, Rung::Full]
        }
    }

    /// The lowest rung ADR-0037 §4 lets one of the three most relevant hints
    /// fall to: its short form where the user's table gives one, and its
    /// whole verb where it does not.
    fn floor_rung(&self) -> Rung {
        if short_form(self.verb).is_some() {
            Rung::Short
        } else {
            Rung::Full
        }
    }
}

/// The user's short-form table, restated 2026-09-21 (m5/27, correcting
/// m5/24): **the short form is a prefix of the full label, or there is no
/// short form.**
///
/// This table is also what the floor is measured against. **A hint's floor
/// rung is its short rung where this table names one, and its full verb
/// where it does not** ([`Hint::floor_rung`]) — so since m5/27 deleted
/// thirteen short forms, "the three most relevant at their short rung" is
/// no longer the floor for most verbs, and the prose says *floor rung*
/// where it means the floor.
///
/// Three kinds were on the table before this pass, and only one survives:
///
/// - a **prefix** keeps the label's beginning (`cont` for `continue`) — the
///   reader recovers the word from the fragment, the way `git`'s own
///   abbreviated flags do;
/// - a **squeeze** removes letters from the middle (`stsh` for `stash`,
///   `cmt` for `commit`) — the reader cannot recover the word from the
///   fragment, only recognise it once told;
/// - a **substitution** replaces the word outright (`find` for `search`,
///   `tool` for `mergetool`) — it gives one verb two names, which
///   `docs/design/02-keymap.md:12`'s "one meaning per verb" forbids
///   outright, and two of them (`a stop`, `M tool`) also broke
///   `docs/design/02-keymap.md:45`'s "match the buttons' initials exactly".
///
/// Squeezes and substitutions are gone. A verb this function does not name
/// simply has no short rung — adding a row is never required to make a
/// label fit, because the ladder's bare-key rung is already the mechanism
/// for "no room" (ADR-0039).
///
/// `stash`, `merge` and `rebase` are live and absent because a squeeze was
/// the only short form that ever fit them and the user ruled squeezes out;
/// `branch`, `tag` and `resolved` are [`HintState::Planned`] and never
/// drawn. All six keep their full form: the table is the user's, and a
/// label it does not name is reported, not replaced.
fn short_form(verb: &str) -> Option<&'static str> {
    Some(match verb {
        "first-parent" => "first",
        "continue" => "cont",
        "discard" => "disc",
        "hide stash" => "hide",
        "overview" => "over",
        "renderer" => "rend",
        "status" => "stat",
        "revert" => "rev",
        "lanes" => "lane",
        _ => return None,
    })
}

/// A hint whose key is bound today.
const fn hint(key: &'static str, verb: &'static str) -> Hint {
    Hint {
        key,
        verb,
        state: HintState::Live,
        rung: Rung::Full,
        keeps_words: false,
    }
}

/// A hint the keymap names for this context that no group has built yet.
/// Never drawn (see the module comment). No table holds one today — the
/// finish summary's were the last, bound by m6-conflicts/34 — so only the
/// tests' canary builds one until the next unbuilt key does.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "no table has a planned entry today; the next unbuilt key uses it"
    )
)]
const fn planned(key: &'static str, verb: &'static str) -> Hint {
    Hint {
        key,
        verb,
        state: HintState::Planned,
        rung: Rung::Full,
        keeps_words: false,
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

/// The hints for `context` that fit in `columns` cells: **as many whole
/// hints as fit the row the bar actually has**, each at the best rung
/// [`ladder`] can afford it, and the ones that will not fit even at their
/// bare key dropped from the right — never below three (or the context's own
/// count, if it has fewer) plus `. more`. Never empty except for
/// [`HintContext::Preview`].
///
/// `columns` is the bar's own room, not the screen's, and there is no
/// width tier on top of it. The 3/6/all tiers this used to carry were
/// never spec: ADR-0037 §4 sets a floor (three most relevant keys, never
/// all hidden) and says nothing about width. Laid over a `columns` that is
/// already the bar's own region, a tier is a *cap*: at 160 columns with the
/// detail pane beside the graph the bar has about 92 cells and fell into
/// the "narrow" tier, so it drew three hints — while a 120-column screen
/// with the pane below kept its 120 cells and drew six. The wider terminal
/// said less (ADR-0039's 2026-09-21 amendment, m5/20 item 7).
///
/// `divider` adds `<> resize` when a detail pane is open, whichever context
/// the cursor is in (ADR-0028: the pane being shown is what makes the
/// divider exist).
///
/// [`HintContext::Banner`] is the exception to both trailers. The banner is
/// modal — `banner_key` answers every key but `Ctrl+C` — so `.` opens
/// nothing and `<>` moves no divider while it has focus. ADR-0039's doorway
/// is a promise that what the width rule dropped is still reachable; under a
/// modal that reaches nothing, drawing it would be the dead key the ADR's
/// own first condition rules out. The banner's list is short enough to draw
/// whole at 80 columns, so nothing is dropped and nothing needs a doorway.
#[must_use]
pub fn hints_for(context: &HintContext, divider: Divider, columns: u16) -> Vec<Hint> {
    // The resolver below its own floor is the one context whose *keys*
    // depend on the width: under `conflict_resolver::MIN_COLUMNS` the
    // widget draws one line of text instead of the document, and the app
    // answers nothing but `Esc` — there is no document on screen to take a
    // side of. So the bar says the one thing that is true there, which is
    // also what that line itself says (m6-conflicts/04).
    if matches!(context, HintContext::Resolver { .. })
        && columns < crate::conflict_resolver::MIN_COLUMNS
    {
        return vec![hint("esc", "close")];
    }
    // The finish summary has a floor too (`finish_summary::FINISH_MIN`), but
    // in both dimensions, and this is told only the width: the app draws
    // `esc back` alone below it (`graft::view::finish`), so the width sweep
    // here stays monotone.
    let (list, trailer) = whole_bar(context, divider);
    fit_list(&list, trailer, columns)
}

/// The bar `context` would draw on a row with no end to it, and the doorway
/// that leads whatever the width drops. [`hints_for`] is this plus the width
/// rule — the split exists so that a context which *filters* another
/// ([`HintContext::PaneFocused`]) can take the whole list away and hand back
/// less **before** the ladder runs. Filtering after it would leave a gap in
/// a row the ladder had already fitted.
fn whole_bar(context: &HintContext, divider: Divider) -> (Vec<Hint>, Option<Hint>) {
    match context {
        HintContext::Preview => (Vec::new(), None),
        // No width tier for the banner: every key it answers draws while
        // the width allows it, `⇥ graph` — the way out of the mode —
        // included. Three keys at 80 columns would have hidden the exit of
        // a modal that swallows every other key, which is the failure
        // `banner_key`'s own doc comment warns of.
        // A drag takes no doorway either, and for the banner's reason: `.`
        // does not open the menu while a badge is in the air, it cancels the
        // drag. Two hints fit at every width the app draws at.
        // The banner, a badge in the air and the two overlays are all
        // modal: each swallows every key but its own, so `.` opens no menu
        // and `<>` moves no divider while it has focus (`banner_key`,
        // `drag_key`, `journal::shown_key` and `stash::shown_key`, whose
        // catch-all arms return "claimed"). ADR-0039's doorway is a promise
        // that what the width dropped is still reachable; under a modal
        // that reaches nothing it would be the dead key the ADR's own first
        // condition rules out (its first amendment, extended to the
        // overlays in m5/31 — the sweep pressed `.` in the journal and
        // nothing happened).
        // The Map view is the third overlay and swallows keys the same way
        // (`graft::update::map::shown_key`'s catch-all). The detail pane
        // with an overlay behind it takes no doorway either: `.` reaches
        // the overlay, which swallows it — pressed in every one of these
        // states and nothing happened (m5/42).
        HintContext::Banner { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Journal
        | HintContext::Stash
        | HintContext::Map
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Resolver { .. }
        | HintContext::Finish { .. }
        // The blame place is modal too (keymap §2.1): no doorway.
        | HintContext::Blame { .. } => (drawable(context, divider), None),
        HintContext::Marked(inner) => {
            // `.` is already on screen as `. explain`, so no `. more`; it
            // is prepended before fitting and not after, because a list
            // fitted first and prepended to second overflows by one hint.
            let mut list = vec![EXPLAIN];
            list.extend(drawable(inner, divider));
            (list, None)
        }
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
        } => pane_focused_bar(row, *opens, *blamable, *folds, divider),
        HintContext::BackToBlame(inner) => {
            let (list, trailer) = whole_bar(inner, divider);
            let mut led = vec![BACK_TO_BLAME];
            led.extend(list);
            (led, trailer)
        }
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow
        | HintContext::ChangesPane { .. } => (drawable(context, divider), Some(MORE)),
        // The row's own bar with its `c` said the way it acts now, and —
        // while one `⇥` reaches the banner — the way back leading, so the
        // ladder's floor always holds it. Then the row's own `⇥` goes: one
        // key, one meaning, and there `Tab` lands on the banner. Without
        // `tab_to_banner` the row's `⇥` (if any) stays as the row says it.
        HintContext::OperationLive {
            row,
            c,
            tab_to_banner,
        } => {
            let (list, trailer) = whole_bar(row, divider);
            let list = lists::said_live(list, *c);
            if !*tab_to_banner {
                return (list, trailer);
            }
            let mut led = vec![WAY_BACK];
            led.extend(list.into_iter().filter(|hint| hint.key != WAY_BACK.key));
            (led, trailer)
        }
    }
}

/// [`whole_bar`] for [`HintContext::PaneFocused`]: the row's own bar minus
/// the keys the focused pane leaves dead, plus the pane's own `B` and
/// `space`.
fn pane_focused_bar(
    row: &HintContext,
    opens: bool,
    blamable: bool,
    folds: bool,
    divider: Divider,
) -> (Vec<Hint>, Option<Hint>) {
    let (list, trailer) = whole_bar(row, divider);
    // With the files list focused its `space` is the pane's, on every row, so
    // the graph's merge fold is not live (m7-power-tools/20h). Only that hint
    // goes: another list's `space` is its own.
    let mut list: Vec<Hint> = list
        .into_iter()
        .filter(|hint| {
            !GRAPH_ONLY.contains(&hint.key)
                && (opens || hint.key != ENTER)
                && !(opens && *hint == FOLD)
        })
        .collect();
    if blamable {
        // After `⏎` when it is there, else first: the pane's own key.
        let at = usize::from(list.first().is_some_and(|hint| hint.key == ENTER));
        list.insert(at, BLAME);
    }
    if folds {
        // After `⏎` and `B` when present, else first.
        let at = list
            .iter()
            .rposition(|hint| hint.key == ENTER || *hint == BLAME)
            .map_or(0, |at| at + 1);
        list.insert(at, FOLD);
    }
    (list, trailer)
}

/// `B blame`: the pane's line blamed ([`HintContext::PaneFocused`]).
const BLAME: Hint = hint("B", "blame");

/// `space fold`: the graph's merge fold ([`TAIL`]) and, with the same word
/// for the same act, a focused files list's directory fold
/// ([`HintContext::PaneFocused`]'s `folds`, m7-power-tools/20h).
const FOLD: Hint = hint("space", "fold");

/// `esc back to blame`: the way back to a hidden blame trail
/// ([`HintContext::BackToBlame`]).
const BACK_TO_BLAME: Hint = hint("esc", "back to blame");

/// `⇥ banner`: the way back to a live operation's banner
/// ([`HintContext::OperationLive`]).
const WAY_BACK: Hint = hint("⇥", "banner");

/// `Enter`, as every table spells it. Named because
/// [`HintContext::PaneFocused`] filters on it.
const ENTER: &str = "⏎";

/// The keys only the graph screen answers: each opens something that stands
/// *where the graph stands*, and each is guarded on the graph having the
/// keyboard, so each does nothing while the detail pane has it
/// (`graft::update::state::AppState::graph_has_focus`, the one function all
/// four handlers read).
const GRAPH_ONLY: [&str; 4] = ["S", "J", "M", "O"];

/// The keys that need a commit under the cursor, and so do nothing on the
/// uncommitted-changes row: `y` copies a sha there is none of and `x` marks
/// a compare base there is no oid for. Both return at their first line on
/// that row (`graft::update::keys::copy_selected_sha`,
/// `toggle_compare_selected`), which the design note already said and the
/// bar drew anyway (m5/31). `space` folds the merge under the cursor, and
/// the uncommitted row is no merge (gap row 553).
const NEEDS_A_COMMIT: [&str; 3] = ["y", "x", "space"];

/// Every hint the footer could draw for `context`, in priority order, with
/// the ones nothing has built yet already dropped: the context's own keys,
/// the divider's when a pane is open, then — on the graph's row contexts
/// only — the keys the whole screen answers ([`TAIL`]).
///
/// A pane keeps to its own list: while the changes pane, an overlay or the
/// banner has focus it answers keys the graph does not, and the graph
/// answers keys it does not, so [`TAIL`] would be advertising the wrong
/// screen.
///
/// The focused banner goes further and takes no divider hint either: it is
/// modal, so `<>` is as dead as every other key it swallows. (Under 100
/// columns the angle keys are a no-op in any case —
/// `graft::update::detail`'s own rule for the tab layout.)
fn drawable(context: &HintContext, divider: Divider) -> Vec<Hint> {
    let mut list: Vec<Hint> = context_list(context)
        .into_iter()
        .filter(|hint| hint.state == HintState::Live)
        .collect();
    if divider == Divider::Shown
        && !matches!(
            context,
            HintContext::Banner { .. }
                | HintContext::Dragging
                | HintContext::Journal
                | HintContext::Stash
                | HintContext::Map
                | HintContext::PaneOverOverlay { .. }
                | HintContext::Resolver { .. }
                | HintContext::Finish { .. }
                | HintContext::Blame { .. }
        )
    {
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
    if rows {
        let commits = *context != HintContext::WorkingTreeRow;
        for extra in TAIL {
            let dead = !commits && NEEDS_A_COMMIT.contains(&extra.key);
            if !dead && !list.iter().any(|hint| hint.key == extra.key) {
                list.push(extra);
            }
        }
    }
    list
}

#[cfg(test)]
mod tests;
