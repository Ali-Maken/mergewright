//! The table with his overrides applied, and the translate the app runs on
//! every key press (ADR-0055 §3, made exact by ticket m8/09 Scope 5).

use super::{ActionId, Binding, ContextId, KeyChord, KeyLabel, KeySwitches, KeyWarning, defaults};

/// What the app does with a pressed key, decided before any pane sees it.
#[derive(Debug, PartialEq, Eq)]
pub enum Translated {
    /// Hand the panes this chord instead: the default chord of the action
    /// he moved onto the pressed key.
    Default(KeyChord),
    /// Drop the key, doing nothing: its default action moved elsewhere.
    Unbound,
    /// Hand the panes the key as pressed.
    Pass,
}

/// [`defaults()`] with his overrides applied, resolved per context.
///
/// Two keymaps are equal when every action has the same chords and the
/// same override lines decide its collisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keymap {
    /// Each row's effective chords, by index into [`defaults()`].
    pub(super) chords: Vec<Vec<KeyChord>>,
    /// Each row's chords before his lines: the table, with the switches
    /// applied (D2). What translate's "default" means.
    pub(super) base: Vec<Vec<KeyChord>>,
    /// The override line that set a row's chords; `None` keeps its base. A
    /// follower ([`FOLLOWS`]) carries its leader's line.
    pub(super) line: Vec<Option<usize>>,
    /// Each row's live chords, spelled ([`Keymap::live_labels`]): worked
    /// out once, because every bar spells its keys many times over while
    /// its width rule measures.
    labels: Vec<Vec<String>>,
}

impl Default for Keymap {
    /// The table as shipped: no overrides, every switch off.
    fn default() -> Self {
        Self::base_map(KeySwitches::default())
    }
}

/// Which chords a resolve reads.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Over {
    /// What he has now, overrides applied.
    Effective,
    /// Before his lines: the table with the switches applied.
    Base,
}

/// Rows whose chords are another row's, never his to set (D3): the
/// preview's "opening key again" confirms with whatever opens it.
pub(super) const FOLLOWS: &[(ActionId, ActionId)] = &[
    (ActionId("preview.confirm_merge"), ActionId("graph.merge")),
    (ActionId("preview.confirm_rebase"), ActionId("graph.rebase")),
];

const SUSPEND_ID: ActionId = ActionId("app.suspend");
const UNDO_ID: ActionId = ActionId("app.undo");

impl Keymap {
    /// Apply his `[keys.<context>]` lines, in the order his file has them:
    /// `("graph.merge", ["M"])`, `("app.down", ["n", "down"])`,
    /// `("graph.push", ["none"])`. An override replaces the action's whole
    /// default list. A line with a problem is not applied and says why;
    /// nothing here is fatal.
    ///
    /// `switches` set the chords before his lines (D2): with
    /// `suspend_on_ctrl_z`, `app.suspend` is `⌃Z` and `app.undo` keeps `u`.
    #[must_use]
    pub fn with_overrides(
        lines: &[(String, Vec<String>)],
        switches: KeySwitches,
    ) -> (Keymap, Vec<KeyWarning>) {
        let mut map = Keymap::base_map(switches);
        let mut warned: Vec<(usize, KeyWarning)> = Vec::new();
        for (at, (name, texts)) in lines.iter().enumerate() {
            match map.apply(at, name, texts, switches) {
                Ok(()) => {}
                Err(warning) => warned.push((at, warning)),
            }
        }
        map.follow();
        let collisions = map.collisions();
        // D7 (m8/09d): a chord any place took from an action is not
        // spelled for it anywhere — a hint naming it would be a lie there.
        let taken: Vec<(KeyChord, ActionId)> = collisions
            .iter()
            .flat_map(|(_, warning)| match warning {
                KeyWarning::Collision { places, .. } => places.as_slice(),
                _ => &[],
            })
            .map(|clash| (clash.key, clash.lost))
            .collect();
        warned.extend(collisions);
        map.relabel(&taken);
        // Stable: within one line, the order they were found in.
        warned.sort_by_key(|(at, _)| *at);
        (map, warned.into_iter().map(|(_, w)| w).collect())
    }

    /// The table with `switches` applied and no lines.
    fn base_map(switches: KeySwitches) -> Self {
        let rows = defaults();
        let mut base: Vec<Vec<KeyChord>> = rows.iter().map(|r| r.default.to_vec()).collect();
        if switches.suspend_on_ctrl_z {
            let suspend = KeyChord::ctrl('z');
            if let (Some(s), Some(u)) = (index_of(SUSPEND_ID.0), index_of(UNDO_ID.0)) {
                base[s] = vec![suspend];
                base[u].retain(|chord| *chord != suspend);
            }
        }
        let mut map = Self {
            chords: base.clone(),
            base,
            line: vec![None; rows.len()],
            labels: Vec::new(),
        };
        map.relabel(&[]);
        map
    }

    /// Each follower takes its leader's chords and line.
    fn follow(&mut self) {
        for (follower, leader) in FOLLOWS {
            if let (Some(f), Some(l)) = (index_of(follower.0), index_of(leader.0)) {
                self.chords[f] = self.chords[l].clone();
                self.line[f] = self.line[l];
            }
        }
    }

    /// One override line, applied, or the reason it isn't.
    fn apply(
        &mut self,
        at: usize,
        name: &str,
        texts: &[String],
        switches: KeySwitches,
    ) -> Result<(), KeyWarning> {
        let Some(row) = index_of(name) else {
            return Err(KeyWarning::UnknownAction(name.to_string()));
        };
        let binding = &defaults()[row];
        if binding.context.is_text_field() {
            return Err(KeyWarning::Fixed(binding.action));
        }
        if let Some((_, leader)) = FOLLOWS.iter().find(|(f, _)| *f == binding.action) {
            return Err(KeyWarning::Follows {
                action: binding.action,
                leader: *leader,
            });
        }
        if binding.action == SUSPEND_ID && !switches.suspend_on_ctrl_z {
            return Err(KeyWarning::SwitchOff {
                action: binding.action,
                switch: "suspend_on_ctrl_z",
            });
        }
        let chords = parse_list(name, texts)?;
        if binding.action == QUIT_NOW_ID {
            return Err(KeyWarning::Reserved {
                action: name.to_string(),
                key: chords.first().copied().unwrap_or(KeyChord::QUIT_NOW),
            });
        }
        if chords.contains(&KeyChord::QUIT_NOW) {
            return Err(KeyWarning::Reserved {
                action: name.to_string(),
                key: KeyChord::QUIT_NOW,
            });
        }
        self.chords[row] = chords;
        self.line[row] = Some(at);
        Ok(())
    }

    /// What the app hands the panes for `pressed` in `ctx` (module docs of
    /// [`Translated`]). `Ctrl+C` and every key in a text field pass.
    #[must_use]
    pub fn translate(&self, ctx: ContextId, pressed: KeyChord) -> Translated {
        if pressed == KeyChord::QUIT_NOW || ctx.is_text_field() {
            return Translated::Pass;
        }
        let Some(action) = self.resolve(ctx, pressed, Over::Effective) else {
            let is_default = reachable_rows(ctx)
                .iter()
                .any(|r| self.base[*r].contains(&pressed));
            return if is_default {
                Translated::Unbound
            } else {
                Translated::Pass
            };
        };
        let acts_as_action =
            |chord: &KeyChord| self.resolve(ctx, *chord, Over::Base) == Some(action);
        let own = &self.base[action];
        let chosen = if own.contains(&pressed) && acts_as_action(&pressed) {
            Some(pressed)
        } else {
            own.iter().copied().find(|d| acts_as_action(d))
        };
        match chosen {
            None => Translated::Unbound,
            Some(d) if d == pressed => Translated::Pass,
            Some(d) => Translated::Default(d),
        }
    }

    /// The table as shipped, built once: what a text field spells (its keys
    /// pass untranslated, so they do what their defaults do) and what tests
    /// spell against.
    #[must_use]
    pub fn shipped() -> &'static Keymap {
        static SHIPPED: std::sync::LazyLock<Keymap> = std::sync::LazyLock::new(Keymap::default);
        &SHIPPED
    }

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
    fn relabel(&mut self, taken: &[(KeyChord, ActionId)]) {
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

    /// The action's chords as he has them; empty when unbound or unknown.
    #[must_use]
    pub fn chords(&self, action: &ActionId) -> &[KeyChord] {
        index_of(action.0).map_or(&[], |row| self.chords[row].as_slice())
    }

    /// Every row a key pressed in `ctx` may land on, in the order a chord
    /// resolves there: its own rows, then [`ContextId::borrows`], then the
    /// rest of its scope (D1). Each row once, with its chords as he has them.
    #[must_use]
    pub fn reachable(&self, ctx: ContextId) -> Vec<(&Binding, &[KeyChord])> {
        reachable_rows(ctx)
            .into_iter()
            .map(|row| (&defaults()[row], self.chords[row].as_slice()))
            .collect()
    }

    /// The context's own rows, in table order, with their chords as he has
    /// them. Not its scope: `?` lists a place's own keys, then `Global`'s.
    #[must_use]
    pub fn in_context(&self, ctx: ContextId) -> Vec<(&Binding, &[KeyChord])> {
        defaults()
            .iter()
            .enumerate()
            .filter(|(_, b)| b.context == ctx)
            .map(|(row, b)| (b, self.chords[row].as_slice()))
            .collect()
    }

    /// The row a chord acts as in `ctx`: its own rows, then the rows it
    /// borrows, then the rest of its scope in order (D1). Inside one tier
    /// an override beats a default, and of two overrides the earlier line
    /// keeps it.
    pub(super) fn resolve(&self, ctx: ContextId, chord: KeyChord, over: Over) -> Option<usize> {
        self.holder(ctx, chord, over)
            .or_else(|| self.best(borrowed_rows(ctx), chord, over))
            .or_else(|| {
                ctx.scope()
                    .iter()
                    .filter(|inner| **inner != ctx)
                    .find_map(|inner| self.holder(*inner, chord, over))
            })
    }

    /// The row of `ctx` itself that keeps `chord`, by the same rule.
    fn holder(&self, ctx: ContextId, chord: KeyChord, over: Over) -> Option<usize> {
        self.best(rows_of(ctx), chord, over)
    }

    /// Of `rows`, the one holding `chord` that keeps it.
    fn best(
        &self,
        rows: impl Iterator<Item = usize>,
        chord: KeyChord,
        over: Over,
    ) -> Option<usize> {
        rows.filter(|row| self.has(*row, chord, over))
            .min_by_key(|row| self.rank(*row))
    }

    /// Whether `row` holds `chord`, as he has it or before his lines.
    pub(super) fn has(&self, row: usize, chord: KeyChord, over: Over) -> bool {
        match over {
            Over::Effective => self.chords[row].contains(&chord),
            Over::Base => self.base[row].contains(&chord),
        }
    }

    /// Lower keeps a shared chord: overrides by line, then defaults by
    /// table order.
    fn rank(&self, row: usize) -> (bool, usize, usize) {
        match self.line[row] {
            Some(at) => (false, at, row),
            None => (true, 0, row),
        }
    }
}

const QUIT_NOW_ID: ActionId = ActionId("app.quit_now");

/// `labels` joined with `/`, or `None` when there are none.
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

/// The row index of an action id.
pub(super) fn index_of(name: &str) -> Option<usize> {
    static ROWS: std::sync::LazyLock<std::collections::HashMap<&'static str, usize>> =
        std::sync::LazyLock::new(|| {
            defaults()
                .iter()
                .enumerate()
                .map(|(row, b)| (b.action.0, row))
                .collect()
        });
    ROWS.get(name).copied()
}

/// The rows bound in `ctx` itself.
fn rows_of(ctx: ContextId) -> impl Iterator<Item = usize> {
    defaults()
        .iter()
        .enumerate()
        .filter(move |(_, b)| b.context == ctx)
        .map(|(row, _)| row)
}

/// The rows `ctx` borrows ([`ContextId::borrows`]).
fn borrowed_rows(ctx: ContextId) -> impl Iterator<Item = usize> {
    ctx.borrows().iter().filter_map(|action| index_of(action.0))
}

/// Every row reachable in `ctx`, in resolution order, each once.
pub(super) fn reachable_rows(ctx: ContextId) -> Vec<usize> {
    let mut out: Vec<usize> = rows_of(ctx).chain(borrowed_rows(ctx)).collect();
    for inner in ctx.scope().iter().filter(|inner| **inner != ctx) {
        for row in rows_of(*inner) {
            if !out.contains(&row) {
                out.push(row);
            }
        }
    }
    out
}

/// His chord list: `["none"]` alone is no chord; `none` inside a longer
/// list, or text naming no key, is a [`KeyWarning::BadKey`].
fn parse_list(action: &str, texts: &[String]) -> Result<Vec<KeyChord>, KeyWarning> {
    let bad = |text: &str| KeyWarning::BadKey {
        action: action.to_string(),
        text: text.to_string(),
    };
    match texts {
        [] => Err(bad("")),
        [only] if only.trim().eq_ignore_ascii_case("none") => Ok(Vec::new()),
        _ => {
            let mut out = Vec::new();
            for text in texts {
                if text.trim().eq_ignore_ascii_case("none") {
                    return Err(bad(text));
                }
                let chord = KeyChord::parse(text).map_err(|e| bad(&e.text))?;
                if !out.contains(&chord) {
                    out.push(chord);
                }
            }
            Ok(out)
        }
    }
}
