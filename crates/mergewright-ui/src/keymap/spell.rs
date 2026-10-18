//! How a key is printed: every surface spells it from the live table
//! (ADR-0055), never from a literal.

use super::resolve::{Over, index_of};
use super::{ActionId, KeyChord, KeyLabel, Keymap, defaults};

impl Keymap {
    /// The action's key as `?` prints it: its first live chord, or `—`
    /// when he unbound it (or it is not an action, or every chord it had
    /// was taken by another in a collision).
    #[must_use]
    pub fn spell(&self, action: &ActionId) -> KeyLabel {
        self.live_labels(action).first().map_or_else(
            || KeyLabel("—".to_string()),
            |label| KeyLabel(label.clone()),
        )
    }

    /// The hint bar's and the footers' spelling of several actions: each
    /// one's first live chord, joined with `/`, an unbound action left out
    /// (`j/k` for `app.down`, `app.up`). `None` when none is bound — the
    /// caller draws nothing then, never `—` (ticket m8/10, decision 2).
    #[must_use]
    pub fn spell_keys(&self, actions: &[ActionId]) -> Option<String> {
        join(
            actions
                .iter()
                .filter_map(|action| self.live_labels(action).first().cloned()),
        )
    }

    /// Every live chord of one action, joined with `/` (`⏎/space` for
    /// `finish.toggle_diff`); `None` when it has none.
    #[must_use]
    pub fn spell_every(&self, action: &ActionId) -> Option<String> {
        join(self.live_labels(action).iter().cloned())
    }

    /// A footer's `<keys> <verb>` (`esc back`, `j/k move`): the keys as
    /// [`Keymap::spell_keys`] spells them, then the verb. `None` when every
    /// key is unbound, so the caller leaves the clause out — a footer never
    /// names a key that does nothing.
    #[must_use]
    pub fn key_verb(&self, actions: &[ActionId], verb: &str) -> Option<String> {
        self.spell_keys(actions)
            .map(|keys| format!("{keys} {verb}"))
    }

    /// A footer's `<key> <verb>` pairs, two spaces apart, each spelled by
    /// [`Keymap::key_verb`] from one action id, an unbound one left out
    /// (`⏎ apply  p pop  esc close`). Empty when none is bound.
    #[must_use]
    pub fn key_verbs(&self, parts: &[(&'static str, &str)]) -> String {
        parts
            .iter()
            .filter_map(|(action, verb)| self.key_verb(&[ActionId(action)], verb))
            .collect::<Vec<_>>()
            .join("  ")
    }

    /// `words` with ` · <key> <verb>` after it (`finish needs 60x20 · esc
    /// back`), or `words` alone when the key is unbound.
    #[must_use]
    pub fn after(&self, words: &str, action: ActionId, verb: &str) -> String {
        match self.key_verb(&[action], verb) {
            Some(hint) => format!("{words} · {hint}"),
            None => words.to_owned(),
        }
    }

    /// The action's live chords, spelled: every chord it holds, less any
    /// another action of its own context took in a collision, and less any
    /// a collision anywhere names it as losing (m8/09d D7: after `merge =
    /// "j"`, move spells `↓` alone). Empty when unbound or not an action.
    #[must_use]
    pub fn live_labels(&self, action: &ActionId) -> &[String] {
        index_of(action.0).map_or(&[], |row| self.labels[row].as_slice())
    }

    /// Works out [`Keymap::labels`] from the chords as they now stand,
    /// leaving out each `(chord, action)` in `taken` — a chord a clash says
    /// that action lost.
    pub(super) fn relabel(&mut self, taken: &[(KeyChord, ActionId)]) {
        self.labels = (0..self.chords.len())
            .map(|row| {
                let action = defaults()[row].action;
                self.live_chords(row)
                    .iter()
                    .filter(|chord| !taken.contains(&(**chord, action)))
                    .map(|chord| chord.label().0)
                    .collect()
            })
            .collect();
    }

    /// Row `row`'s chords that still do it. A text field's rows are fixed,
    /// so all of theirs.
    fn live_chords(&self, row: usize) -> Vec<KeyChord> {
        let ctx = defaults()[row].context;
        self.chords[row]
            .iter()
            .copied()
            .filter(|chord| {
                ctx.is_text_field() || self.holder(ctx, *chord, Over::Effective) == Some(row)
            })
            .collect()
    }
}

/// Labels joined with `/`, the one joiner (ux-principles row 4), with a
/// repeated `<word>+` modifier written once (ticket m8/10, ruling R2):
/// `alt+↑`, `alt+↓` → `alt+↑/↓`. The repeated word earns no cell (row 8).
/// Only `+`-joined words are shared; a glyph modifier keeps its own
/// (`⌃O/⌃T` — the resolver's spelling since m6). `None` when there are
/// no labels.
pub(crate) fn join(labels: impl IntoIterator<Item = String>) -> Option<String> {
    let mut out = String::new();
    let mut previous: Option<String> = None;
    for label in labels {
        let prefix = word_prefix(&label);
        let shared = prefix.is_some() && previous.as_deref() == prefix;
        if !out.is_empty() {
            out.push('/');
        }
        match prefix {
            Some(prefix) if shared => out.push_str(&label[prefix.len()..]),
            _ => out.push_str(&label),
        }
        previous = prefix.map(str::to_owned);
    }
    (!out.is_empty()).then_some(out)
}

/// `label`'s modifier words with their trailing `+` (`alt+` of `alt+↑`,
/// `ctrl+shift+` of `ctrl+shift+x`), when it has any and a key follows them.
fn word_prefix(label: &str) -> Option<&str> {
    let end = label.rfind('+')? + 1;
    let prefix = &label[..end];
    let words = prefix
        .split('+')
        .filter(|word| !word.is_empty())
        .all(|word| word.chars().all(|c| c.is_ascii_alphabetic()));
    (end > 1 && end < label.len() && words && !prefix.starts_with('+')).then_some(prefix)
}
