//! The table with his overrides applied, and the translate the app runs on
//! every key press (ADR-0055 §3, made exact by ticket m8/09 Scope 5).

use super::{ActionId, Binding, ContextId, KeyChord, KeySwitches, KeyWarning, defaults};

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
    pub(super) labels: Vec<Vec<String>>,
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

pub(super) const SUSPEND_ID: ActionId = ActionId("app.suspend");
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

    /// The action's chords as he has them; empty when unbound or unknown.
    #[must_use]
    pub fn chords(&self, action: &ActionId) -> &[KeyChord] {
        index_of(action.0).map_or(&[], |row| self.chords[row].as_slice())
    }

    /// Every row a key pressed in `ctx` may land on, in the order a chord
    /// resolves there: its own rows, then [`ContextId::borrows`], then the
    /// rest of its scope (D1). Each row once, with its chords as he has them;
    /// less the rows the place stands down ([`ContextId::stands_down`]).
    #[must_use]
    pub fn reachable(&self, ctx: ContextId) -> Vec<(&Binding, &[KeyChord])> {
        reachable_rows(ctx)
            .into_iter()
            .map(|row| (&defaults()[row], self.chords[row].as_slice()))
            .filter(|(binding, _)| !ctx.stands_down().contains(&binding.action))
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

    /// The action `chord` acts as when pressed in `ctx`, his lines applied:
    /// the palette's per-place spelling (m8/11 decision 3 — after `merge =
    /// "j"`, `j` on the graph is merge, not move). `None` when no row a key
    /// there reaches holds it.
    #[must_use]
    pub fn acting(&self, ctx: ContextId, chord: KeyChord) -> Option<ActionId> {
        self.resolve(ctx, chord, Over::Effective)
            .map(|row| defaults()[row].action)
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
    pub(super) fn holder(&self, ctx: ContextId, chord: KeyChord, over: Over) -> Option<usize> {
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

pub(super) const QUIT_NOW_ID: ActionId = ActionId("app.quit_now");

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
