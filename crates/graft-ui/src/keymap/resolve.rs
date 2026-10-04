//! The table with his overrides applied, and the translate the app runs on
//! every key press (ADR-0055 §3, made exact by ticket m8/09 Scope 5).

use super::{ActionId, Binding, ContextId, KeyChord, KeyLabel, KeyWarning, defaults};

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
    chords: Vec<Vec<KeyChord>>,
    /// The override line that set a row's chords; `None` keeps its defaults.
    line: Vec<Option<usize>>,
    /// Each row's live chords, spelled ([`Keymap::live_labels`]): worked
    /// out once, because every bar spells its keys many times over while
    /// its width rule measures.
    labels: Vec<Vec<String>>,
}

impl Default for Keymap {
    /// The table as shipped: no overrides.
    fn default() -> Self {
        let rows = defaults();
        let mut map = Self {
            chords: rows.iter().map(|r| r.default.to_vec()).collect(),
            line: vec![None; rows.len()],
            labels: Vec::new(),
        };
        map.relabel();
        map
    }
}

/// Which chords a resolve reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Over {
    /// What he has now, overrides applied.
    Effective,
    /// The table as shipped.
    Defaults,
}

impl Keymap {
    /// Apply his `[keys.<context>]` lines, in the order his file has them:
    /// `("graph.merge", ["M"])`, `("app.down", ["n", "down"])`,
    /// `("graph.push", ["none"])`. An override replaces the action's whole
    /// default list. A line with a problem is not applied and says why;
    /// nothing here is fatal.
    #[must_use]
    pub fn with_overrides(lines: &[(String, Vec<String>)]) -> (Keymap, Vec<KeyWarning>) {
        let mut map = Keymap::default();
        let mut warned: Vec<(usize, KeyWarning)> = Vec::new();
        for (at, (name, texts)) in lines.iter().enumerate() {
            match map.apply(at, name, texts) {
                Ok(()) => {}
                Err(warning) => warned.push((at, warning)),
            }
        }
        warned.extend(map.collisions());
        map.relabel();
        // Stable: within one line, the order they were found in.
        warned.sort_by_key(|(at, _)| *at);
        (map, warned.into_iter().map(|(_, w)| w).collect())
    }

    /// One override line, applied, or the reason it isn't.
    fn apply(&mut self, at: usize, name: &str, texts: &[String]) -> Result<(), KeyWarning> {
        let Some(row) = index_of(name) else {
            return Err(KeyWarning::UnknownAction(name.to_string()));
        };
        let binding = &defaults()[row];
        if binding.context.is_text_field() {
            return Err(KeyWarning::Fixed(binding.action));
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
        let scope = ctx.scope();
        let Some(action) = self.resolve(scope, pressed, Over::Effective) else {
            let is_default = rows_in(scope).any(|r| defaults()[r].default.contains(&pressed));
            return if is_default {
                Translated::Unbound
            } else {
                Translated::Pass
            };
        };
        let acts_as_action =
            |chord: &KeyChord| self.resolve(scope, *chord, Over::Defaults) == Some(action);
        let own = defaults()[action].default;
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

    /// The action's live chords, spelled: every chord it holds, less any
    /// another action of its own context took in a collision. Empty when
    /// unbound or not an action.
    #[must_use]
    pub fn live_labels(&self, action: &ActionId) -> &[String] {
        index_of(action.0).map_or(&[], |row| self.labels[row].as_slice())
    }

    /// Works out [`Keymap::labels`] from the chords as they now stand.
    fn relabel(&mut self) {
        self.labels = (0..self.chords.len())
            .map(|row| {
                self.live_chords(row)
                    .iter()
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

    /// The row a chord acts as in `scope`: inner context first; inside one
    /// context an override beats a default, and of two overrides the
    /// earlier line keeps it.
    fn resolve(&self, scope: &[ContextId], chord: KeyChord, over: Over) -> Option<usize> {
        scope.iter().find_map(|ctx| self.holder(*ctx, chord, over))
    }

    /// The row of `ctx` itself that keeps `chord`, by the same rule.
    fn holder(&self, ctx: ContextId, chord: KeyChord, over: Over) -> Option<usize> {
        self.holders(ctx, chord, over)
            .min_by_key(|row| self.rank(*row))
    }

    /// Every row of `ctx` itself holding `chord`.
    fn holders(
        &self,
        ctx: ContextId,
        chord: KeyChord,
        over: Over,
    ) -> impl Iterator<Item = usize> + '_ {
        rows_of(ctx).filter(move |row| match over {
            Over::Effective => self.chords[*row].contains(&chord),
            Over::Defaults => defaults()[*row].default.contains(&chord),
        })
    }

    /// Lower keeps a shared chord: overrides by line, then defaults by
    /// table order.
    fn rank(&self, row: usize) -> (bool, usize, usize) {
        match self.line[row] {
            Some(at) => (false, at, row),
            None => (true, 0, row),
        }
    }

    /// Every chord that ends up on two actions in one context, or on a
    /// context's action and a `Global` one, where an override put it there.
    /// Each with the line it is about, for ordering.
    fn collisions(&self) -> Vec<(usize, KeyWarning)> {
        let mut found = Vec::new();
        for &ctx in ContextId::ALL {
            if ctx.is_text_field() {
                continue; // nothing in a text field is remapped
            }
            for chord in self.chords_in(ctx) {
                let Some(kept) = self.holder(ctx, chord, Over::Effective) else {
                    continue;
                };
                let mut lost: Vec<usize> = self
                    .holders(ctx, chord, Over::Effective)
                    .filter(|row| *row != kept)
                    .collect();
                if ctx != ContextId::Global {
                    lost.extend(self.holders(ContextId::Global, chord, Over::Effective));
                }
                for row in lost {
                    let lines = [self.line[kept], self.line[row]];
                    let Some(at) = lines.iter().flatten().max() else {
                        continue; // two defaults: the table's own design
                    };
                    found.push((
                        *at,
                        KeyWarning::Collision {
                            key: chord,
                            context: ctx,
                            kept: defaults()[kept].action,
                            lost: defaults()[row].action,
                        },
                    ));
                }
            }
        }
        found
    }

    /// The distinct chords `ctx`'s own rows hold now, in table order.
    fn chords_in(&self, ctx: ContextId) -> Vec<KeyChord> {
        let mut out: Vec<KeyChord> = Vec::new();
        for row in rows_of(ctx) {
            for chord in &self.chords[row] {
                if !out.contains(chord) {
                    out.push(*chord);
                }
            }
        }
        out
    }

    /// Words for a lost chord: what else the losing action still answers to
    /// in `ctx`, by the effective resolve.
    pub(super) fn left_in(
        &self,
        action: &ActionId,
        ctx: ContextId,
        lost: KeyChord,
    ) -> Option<KeyChord> {
        let row = index_of(action.0)?;
        self.chords[row]
            .iter()
            .copied()
            .filter(|c| *c != lost)
            .find(|c| self.resolve(ctx.scope(), *c, Over::Effective) == Some(row))
    }
}

const QUIT_NOW_ID: ActionId = ActionId("app.quit_now");

/// `labels` joined with `/`, or `None` when there are none.
fn join(labels: impl Iterator<Item = String>) -> Option<String> {
    let joined = labels.collect::<Vec<_>>().join("/");
    (!joined.is_empty()).then_some(joined)
}

/// The row index of an action id.
fn index_of(name: &str) -> Option<usize> {
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

/// The rows bound in any of `contexts`.
fn rows_in(contexts: &[ContextId]) -> impl Iterator<Item = usize> + '_ {
    defaults()
        .iter()
        .enumerate()
        .filter(|(_, b)| contexts.contains(&b.context))
        .map(|(row, _)| row)
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
