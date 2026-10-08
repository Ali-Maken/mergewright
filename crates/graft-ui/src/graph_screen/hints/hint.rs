//! A hint and its parts: state, rung, key, the constructors and the shared hints the bar draws (m8/21b-1 split from `hints.rs`).

use super::{ActionId, Keymap, hint_keys};

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
    pub(super) const fn at(mut self, rung: Rung) -> Self {
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
    pub(super) fn labels(&self, keys: &Keymap) -> Vec<String> {
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
    pub(super) fn rungs(&self) -> &'static [Rung] {
        if short_form(self.verb).is_some() {
            &[Rung::Key, Rung::Short, Rung::Full]
        } else {
            &[Rung::Key, Rung::Full]
        }
    }

    /// The lowest rung ADR-0037 §4 lets one of the three most relevant hints
    /// fall to: its short form where the user's table gives one, and its
    /// whole verb where it does not.
    pub(super) fn floor_rung(&self) -> Rung {
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
pub(super) fn short_form(verb: &str) -> Option<&'static str> {
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
pub(super) const fn hint(keys: &'static [HintKey], verb: &'static str) -> Hint {
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
pub(super) const fn planned(keys: &'static [HintKey], verb: &'static str) -> Hint {
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
pub(super) const EXPLAIN: Hint = hint(hint_keys::MENU, "explain");
/// The detail pane's divider (ADR-0028): shown by every context while a
/// pane is open, not only the changes pane (m5-03 review, Major 4).
pub(super) const RESIZE: Hint = hint(hint_keys::RESIZE, "resize");
