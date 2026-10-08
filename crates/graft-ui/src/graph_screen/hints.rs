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
//! (`docs/design-notes/m5-history-and-ghost-preview.md`, "Hint bar"). The
//! journal, stash and Map draw their own footers (`journal_screen`,
//! `stash_screen`, `map_screen`), which take the doorway from
//! [`doorway_words`] and shed by [`fit_place_footer`] (m8/11e).
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
mod keys;
mod lists;

use crate::keymap::{ActionId, Keymap};
use keys as hint_keys;

pub use catalogue::{every_context, every_hint};
pub use context::{ConfirmYes, HintContext, LiveC, RowVerbs, TabWord};
pub use fitting::{
    HINT_SEPARATOR, StopWording, fit_status_line, hints_text, stopped_row, stopped_row_before_keys,
};
use fitting::{Words, fit_list};
pub(crate) use fitting::{fit_place_footer, place_footer_text};
use lists::{TAIL, context_list};
pub use lists::{blame_place_list, history_place_list};

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

/// One key a hint names (ticket m8/10, decision 1): an action spelled from
/// his keymap, or a word that is not a binding at all ([`NOT_A_BINDING`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintKey {
    /// The action's first live chord (`Keymap::spell_keys`).
    Key(ActionId),
    /// Every live chord of one action, `/`-joined (`⏎/space`).
    EveryChord(ActionId),
    /// A text field's key: its default chord, which no remap moves — the
    /// field passes every press as typed (ADR-0055). Spelled from
    /// [`Keymap::shipped`] whatever keymap the bar is drawn with.
    Fixed(ActionId),
    /// A word, never a key string: one of [`NOT_A_BINDING`].
    Word(&'static str),
}

/// The words a hint may name that are not a binding, each with why
/// (ticket m8/10, decision 4). Everything else a hint names is a row of
/// `keymap::defaults()`, spelled from his keymap.
pub const NOT_A_BINDING: &[(&str, &str)] = &[
    (
        "type",
        "typed text in a field: every printable key, never remapped (ADR-0055)",
    ),
    (
        "release",
        "the mouse button coming up at the end of a badge drag, not a key",
    ),
    (
        "↑/↓",
        "the branch picker's own arrows: a text field moves its rows with them while \
         `j`/`k` type, so they are not `app.down`/`app.up`",
    ),
];

/// One hint: the keys that do it, the verb it does, whether it does it yet,
/// and how much of it the bar drew.
#[derive(Clone, Copy, Debug)]
pub struct Hint {
    /// The keys, as actions (`app.open`) or words; drawn `/`-joined.
    pub keys: &'static [HintKey],
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
    /// Drawn in a words-only bar (ADR-0058 §4): [`hints_for`] stamps every
    /// hint of a bar outside a modal context, and [`hints_text`] then joins
    /// them with ` · `. A modal bar's hints, and any list built straight from
    /// a table, are joined with two spaces — a run of bare keys reads as keys,
    /// not as a sentence. Not part of a hint's identity: equality ignores it.
    pub words_only: bool,
}

impl PartialEq for Hint {
    fn eq(&self, other: &Self) -> bool {
        (
            self.keys,
            self.verb,
            self.state,
            self.rung,
            self.keeps_words,
        ) == (
            other.keys,
            other.verb,
            other.state,
            other.rung,
            other.keeps_words,
        )
    }
}

impl Eq for Hint {}

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

    /// The hint's keys as `keys` spells them, `/`-joined; an unbound
    /// action is left out, and `None` when nothing is left — the hint names
    /// no key that acts, so it is not drawn (ticket m8/10, decision 2).
    #[must_use]
    pub fn key_text(&self, keys: &Keymap) -> Option<String> {
        crate::keymap::join(self.labels(keys))
    }

    /// Each key the hint draws, one label per chord or word.
    fn labels(&self, keys: &Keymap) -> Vec<String> {
        let mut out = Vec::new();
        for key in self.keys {
            match key {
                HintKey::Key(id) => out.extend(keys.spell_keys(&[*id])),
                HintKey::EveryChord(id) => {
                    out.extend(keys.live_labels(id).iter().cloned());
                }
                HintKey::Fixed(id) => out.extend(Keymap::shipped().spell_keys(&[*id])),
                HintKey::Word(word) => out.push((*word).to_owned()),
            }
        }
        out
    }

    /// What the bar draws for this hint at its rung, key included; `None`
    /// when [`Hint::key_text`] is.
    #[must_use]
    pub fn text(&self, keys: &Keymap) -> Option<String> {
        let key = self.key_text(keys)?;
        Some(match self.drawn_verb() {
            Some(verb) => format!("{key} {verb}"),
            None => key,
        })
    }

    /// Whether this hint names exactly `keys` — how a table finds one of
    /// its own entries (`⏎`'s, `c`'s) to place another beside it.
    #[must_use]
    pub fn is(&self, keys: &[HintKey]) -> bool {
        self.keys == keys
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
        // Blame's `L`, third in its bar (m7-power-tools/21b): the floor's
        // three fit 48 cells only with this prefix.
        "history of these lines" => "history",
        // The history place's bar (conductor ruling 2026-09-27, 21b): its
        // floor at full words is 43 cells; with these two it is 30, so a
        // 40-column terminal still draws it. `next` gives way first.
        "next change" => "next",
        "older·newer" => "older",
        // The way back to a hidden place leads its bar (ADR-0058, 06): with
        // the ` · ` separator its floor is 50 cells whole, and the narrowest
        // bar is 47. Prefixes, so the reader still recovers the verb.
        "back to blame" | "back to history" => "back",
        _ => return None,
    })
}

/// A hint whose key is bound today.
const fn hint(keys: &'static [HintKey], verb: &'static str) -> Hint {
    Hint {
        keys,
        verb,
        state: HintState::Live,
        rung: Rung::Full,
        keeps_words: false,
        words_only: false,
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
const fn planned(keys: &'static [HintKey], verb: &'static str) -> Hint {
    Hint {
        keys,
        verb,
        state: HintState::Planned,
        rung: Rung::Full,
        keeps_words: false,
        words_only: false,
    }
}

/// The bar's doorway (m8/11b decision 1, §2.12): `? keys` opens the palette
/// on this place, which lists every key the width dropped. Drawn last, never
/// dropped. Spelled from his keymap; with `app.help` unbound it falls back to
/// [`DOORWAY_BY_PALETTE`], and with both unbound there is no doorway.
pub(crate) const DOORWAY: Hint = hint(hint_keys::HELP, "keys");
/// The doorway when `app.help` is unbound: `⌃P keys` (`app.palette`).
pub(crate) const DOORWAY_BY_PALETTE: Hint = hint(hint_keys::PALETTE, "keys");
/// `. actions`: the context menu, an ordinary candidate since m8/11b
/// (§2.1: everything bound is a candidate). Ranked last before the doorway,
/// so it is the first hint the width drops.
pub(crate) const ACTIONS: Hint = hint(hint_keys::MENU, "actions");
/// Leads a marked row's list: press `.` to explain the Mistake Radar finding
/// (m2-details-and-diffs/17, design note "Marker column").
const EXPLAIN: Hint = hint(hint_keys::MENU, "explain");
/// The detail pane's divider (ADR-0028): shown by every context while a
/// pane is open, not only the changes pane (m5-03 review, Major 4).
const RESIZE: Hint = hint(hint_keys::RESIZE, "resize");

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
///
/// Keys are spelled from `keys`, his keymap (ticket m8/10): a hint whose
/// every key is unbound is left out before the width rule runs, and so is a
/// hint whose key an earlier hint on the same bar already draws — after a
/// remap two actions can spell one chord, and the bar never draws a chord
/// twice ([`spelled`]).
#[must_use]
pub fn hints_for(
    context: &HintContext,
    divider: Divider,
    columns: u16,
    keys: &Keymap,
) -> Vec<Hint> {
    // The resolver below its own floor is the one context whose *keys*
    // depend on the width: under `conflict_resolver::MIN_COLUMNS` the
    // widget draws one line of text instead of the document, and the app
    // answers nothing but `Esc` — there is no document on screen to take a
    // side of. So the bar says the one thing that is true there, which is
    // also what that line itself says (m6-conflicts/04).
    if matches!(context, HintContext::Resolver { .. })
        && columns < crate::conflict_resolver::MIN_COLUMNS
    {
        return spelled(vec![hint(hint_keys::BACK, "close")], keys);
    }
    // The finish summary has a floor too (`finish_summary::FINISH_MIN`), but
    // in both dimensions, and this is told only the width: the app draws
    // `esc back` alone below it (`graft::view::finish`), so the width sweep
    // here stays monotone.
    let (list, trailer) = spelled_bar(context, divider, keys);
    let words = words_for(context);
    let fitted = fit_list(&list, trailer, columns, words, keys);
    if words == Words::Only {
        doorway_last(fitted)
    } else {
        fitted
    }
}

/// A words-only bar with its doorway, if drawn, moved to the end: where a
/// marked row ranks `? keys` mid-list (m8/11d R2) it still reads last, as
/// on every other words-only bar. Width-neutral — every join on a
/// words-only bar is the same ` · `, so a reorder cannot overflow.
fn doorway_last(mut fitted: Vec<Hint>) -> Vec<Hint> {
    if let Some(at) = fitted.iter().position(|hint| *hint == DOORWAY) {
        let door = fitted.remove(at);
        fitted.push(door);
    }
    fitted
}

/// [`whole_bar`] with [`spelled`] applied to the list and its doorway: the
/// doorway goes too when `.` is unbound, or when the list already draws it.
fn spelled_bar(
    context: &HintContext,
    divider: Divider,
    keys: &Keymap,
) -> (Vec<Hint>, Option<Hint>) {
    let (list, trailer) = whole_bar(context, divider);
    let list = spelled(list, keys);
    // `? keys`, or `⌃P keys` when `?` is unbound (m8/11b decision 1): the
    // one fallback, [`doorway_hint`], which the place footers spell too.
    let trailer = trailer.and_then(|door| {
        if door == DOORWAY {
            doorway_hint(keys)
        } else {
            Some(door)
        }
    });
    let trailer = trailer.filter(|door| {
        spelled(list.iter().copied().chain([*door]).collect(), keys).len() > list.len()
    });
    (list, trailer)
}

/// The doorway his keymap can spell: [`DOORWAY`], else
/// [`DOORWAY_BY_PALETTE`] when `app.help` is unbound, else `None`. The one
/// home of that fallback (m8/11e F2): the bar and the place footers both
/// read it.
fn doorway_hint(keys: &Keymap) -> Option<Hint> {
    [DOORWAY, DOORWAY_BY_PALETTE]
        .into_iter()
        .find(|door| !door.labels(keys).is_empty())
}

/// The bar doorway as his keymap spells it: `? keys`, else `⌃P keys` when
/// `help` is unbound, else `None`. The one source: `spelled_bar` and the
/// journal / stash / Map footers all call it (m8/11e F2).
#[must_use]
pub fn doorway_words(keys: &Keymap) -> Option<String> {
    doorway_hint(keys).and_then(|door| door.text(keys))
}

/// The doorway's bare key (`?`, or `⌃P`): the rung below
/// [`doorway_words`] on a footer too narrow for the words (ADR-0039).
pub(crate) fn doorway_key(keys: &Keymap) -> Option<String> {
    doorway_hint(keys).and_then(|door| door.key_text(keys))
}

/// `list` less every hint that names no live key, and less every hint
/// drawing a chord an earlier hint already draws (the earlier one is the
/// more relevant: tables put a place's own keys first).
fn spelled(list: Vec<Hint>, keys: &Keymap) -> Vec<Hint> {
    let mut drawn: Vec<String> = Vec::new();
    let mut kept = Vec::new();
    for hint in list {
        let labels = hint.labels(keys);
        if labels.is_empty() || labels.iter().any(|label| drawn.contains(label)) {
            continue;
        }
        drawn.extend(labels);
        kept.push(hint);
    }
    kept
}

/// Whether `context`'s bar is words-only (ADR-0058 §4): every context that
/// is not modal. The modal ones — the banner, a badge in the air, boxes,
/// prompts, the overlays, the resolver, the places, the rewrite editor —
/// keep the bare-key rung, because there the keys are the point.
fn words_for(context: &HintContext) -> Words {
    match context {
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow { .. }
        | HintContext::StashRow
        | HintContext::Rewritable(_)
        | HintContext::ChangesPane { .. }
        | HintContext::Marked(_)
        | HintContext::PaneFocused { .. } => Words::Only,
        HintContext::BackToBlame(inner) | HintContext::BackToHistory(inner) => words_for(inner),
        HintContext::OperationLive { row, .. } => words_for(row),
        HintContext::Preview
        | HintContext::Banner { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Picker
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Resolver { .. }
        | HintContext::Finish { .. }
        | HintContext::Blame { .. }
        | HintContext::History { .. }
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage => Words::Keys,
    }
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
        // The bars where the keys are the point (ADR-0058 §4): each
        // swallows `?` or types it, so a doorway there would be a dead key.
        HintContext::Banner { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Picker
        | HintContext::Resolver { .. }
        | HintContext::Finish { .. }
        // The rewrite editor is modal (keymap §2.7): no doorway.
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage => (drawable(context, divider), None),
        // The places, and the pane over an overlay, are modal for keys but
        // not for the bar (m8/11c decision 1): `?` answers before any of
        // them (`graft::update::keys`, `palette::open_key`) and lists the
        // place's own keys, so they end with the doorway. No `. more`: `.`
        // is swallowed there. The journal, stash and Map draw their own
        // footers, which take the doorway from [`doorway_words`] (m8/11e).
        HintContext::PaneOverOverlay { .. }
        | HintContext::Blame { .. }
        | HintContext::History { .. } => (drawable(context, divider), Some(DOORWAY)),
        HintContext::Marked(inner) => (marked_list(inner, divider), None),
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
            pane_focused_bar(row, *opens, *blamable, *folds, divider),
            *graph_hidden,
        ),
        HintContext::BackToBlame(inner) => led_by(BACK_TO_BLAME, inner, divider),
        HintContext::BackToHistory(inner) => led_by(BACK_TO_HISTORY, inner, divider),
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow { .. }
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
        // `tab_to_banner` the row's `⇥` (if any) stays as the row says it.
        HintContext::OperationLive {
            row,
            c,
            tab_to_banner,
            space_stages,
        } => {
            let (list, trailer) = whole_bar(row, divider);
            let list = lists::said_live_on_the_graph(list, *c, *space_stages);
            if !*tab_to_banner {
                return (list, trailer);
            }
            let mut led = vec![WAY_BACK];
            led.extend(list.into_iter().filter(|hint| !hint.is(WAY_BACK.keys)));
            (led, trailer)
        }
    }
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
fn marked_list(inner: &HintContext, divider: Divider) -> Vec<Hint> {
    let mut list = vec![EXPLAIN];
    list.extend(drawable(inner, divider));
    let at = list
        .iter()
        .rposition(|hint| hint.is(hint_keys::UNDO) || hint.is(hint_keys::FIND))
        .map_or(list.len(), |at| at + 1);
    list.insert(at, DOORWAY);
    list
}

/// A focused pane's bar on the Detail tab (`graph_hidden`) less
/// `1 first-parent`, whose effect is on the hidden graph (m8/11b decision 3,
/// ADR-0041 carve-out).
fn off_the_graph(
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
fn with_actions(mut list: Vec<Hint>) -> Vec<Hint> {
    list.push(ACTIONS);
    list
}

/// [`with_actions`], except for the changes pane over an overlay (m8/09e,
/// 11b row 553): there `.` is the graph's menu key, reached through the
/// graph, which the overlay swallows (`graft::update::keys::graph_key`) —
/// no `. actions`. `?` answers first (`open_key`), so the doorway stays.
fn actions_unless_over_overlay(context: &HintContext, list: Vec<Hint>) -> Vec<Hint> {
    match context {
        HintContext::ChangesPane {
            over_overlay: true, ..
        } => list,
        _ => with_actions(list),
    }
}

/// `inner`'s whole bar with `lead` in front: the way back to a place.
fn led_by(lead: Hint, inner: &HintContext, divider: Divider) -> (Vec<Hint>, Option<Hint>) {
    let (list, trailer) = whole_bar(inner, divider);
    let mut bar = vec![lead];
    bar.extend(list);
    (bar, trailer)
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
        // (`graft::update::place::f_key`, guarded on the same `blamable`),
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
const BLAME: Hint = hint(hint_keys::BLAME, "blame");

/// `space fold`: the graph's merge fold ([`TAIL`]) and, with the same word
/// for the same act, a focused files list's directory fold
/// ([`HintContext::PaneFocused`]'s `folds`, m7-power-tools/20h).
const FOLD: Hint = hint(hint_keys::FOLD_DIR, "fold");

/// `esc back to blame`: the way back to a hidden blame trail
/// ([`HintContext::BackToBlame`]).
const BACK_TO_BLAME: Hint = hint(hint_keys::BACK, "back to blame");

/// `esc back to history`: the way back to a hidden history place
/// ([`HintContext::BackToHistory`], m7-power-tools/21b).
const BACK_TO_HISTORY: Hint = hint(hint_keys::BACK, "back to history");

/// `F history`: the focused pane's file traced
/// ([`HintContext::PaneFocused`]'s `blamable`, m7-power-tools/21b).
const F_HISTORY: Hint = hint(hint_keys::FILE_HISTORY, "history");

/// `⇥ banner`: the way back to a live operation's banner
/// ([`HintContext::OperationLive`]).
const WAY_BACK: Hint = hint(hint_keys::NEXT_PANE, "banner");

/// The keys only the graph screen answers: each opens something that stands
/// *where the graph stands*, and each is guarded on the graph having the
/// keyboard, so each does nothing while the detail pane has it
/// (`graft::update::state::AppState::graph_has_focus`, the one function all
/// five handlers read).
const GRAPH_ONLY: [&[HintKey]; 5] = [
    hint_keys::STASH,
    hint_keys::JOURNAL,
    hint_keys::MAP,
    hint_keys::OVERVIEW,
    hint_keys::SWITCH,
];

/// The keys that need a commit under the cursor, and so do nothing on the
/// uncommitted-changes row: `y` copies a sha there is none of and `x` marks
/// a compare base there is no oid for. Both return at their first line on
/// that row (`graft::update::keys::copy_selected_sha`,
/// `toggle_compare_selected`), which the design note already said and the
/// bar drew anyway (m5/31). `space` folds the merge under the cursor, and
/// the uncommitted row is no merge (gap row 553).
const NEEDS_A_COMMIT: [&[HintKey]; 3] = [hint_keys::COPY_SHA, hint_keys::COMPARE, hint_keys::FOLD];

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
/// `graft::update::detail`'s own rule for the tab layout.)
fn drawable(context: &HintContext, divider: Divider) -> Vec<Hint> {
    let mut list: Vec<Hint> = context_list(context)
        .into_iter()
        .filter(|hint| hint.state == HintState::Live)
        .collect();
    let rows = matches!(
        context,
        HintContext::WorkingTreeRow
            | HintContext::BranchRow { .. }
            | HintContext::PlainRow { .. }
            | HintContext::StashRow
            | HintContext::Rewritable(_)
    );
    if rows {
        let commits = *context != HintContext::WorkingTreeRow;
        for extra in TAIL {
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
                | HintContext::Dragging
                | HintContext::Picker
                | HintContext::PaneOverOverlay { .. }
                | HintContext::Resolver { .. }
                | HintContext::Finish { .. }
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod spell_sweep;
