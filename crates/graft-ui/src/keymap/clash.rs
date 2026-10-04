//! Which of his lines changed what a key does somewhere, worked out by
//! comparing every place's resolution before and after his lines (ticket
//! m8/09c, D1) — not pairwise between rows, so a graph verb put on `j` is
//! seen wherever `j` stops moving, and nowhere it still moves.

use super::resolve::{FOLLOWS, Over, index_of, reachable_rows};
use super::{ActionId, Clash, ContextId, KeyChord, KeyWarning, Keymap, defaults};

/// One clash, with the line it is about.
struct Found {
    line: usize,
    clash: Clash,
}

impl Keymap {
    /// One [`KeyWarning::Collision`] per line that took a key from a row
    /// still listing it, or lost a key where its action lives; each with
    /// the line it is about, for ordering.
    pub(super) fn collisions(&self) -> Vec<(usize, KeyWarning)> {
        let mut found: Vec<Found> = Vec::new();
        for &ctx in ContextId::ALL {
            if ctx.is_text_field() {
                continue; // nothing in a text field is remapped
            }
            for chord in self.chords_reaching(ctx) {
                self.clashes_at(ctx, chord, &mut found);
            }
        }
        let found = once_each(found);
        let mut out: Vec<(usize, KeyWarning)> = Vec::new();
        for Found { line, clash } in found {
            match out.iter_mut().find(|(at, _)| *at == line) {
                Some((_, KeyWarning::Collision { places, .. })) => places.push(clash),
                _ => out.push((
                    line,
                    KeyWarning::Collision {
                        line: self.line_action(line, &clash),
                        places: vec![clash],
                    },
                )),
            }
        }
        out
    }

    /// The take and the lose clashes of `chord` in `ctx` (D1 (i), (ii)).
    fn clashes_at(&self, ctx: ContextId, chord: KeyChord, found: &mut Vec<Found>) {
        let now = self.resolve(ctx, chord, Over::Effective);
        let before = self.resolve(ctx, chord, Over::Base);
        // (i) take: his line's row now answers a chord another row still
        // lists, where that row acts. A row only reached by falling through
        // (the graph reaching a diff's `w`) is the over-include `scope`
        // allows, not a place it acts — the same rule as (ii).
        if let (Some(kept), Some(lost)) = (now, before)
            && kept != lost
            && let Some(line) = self.line[kept]
            && self.has(lost, chord, Over::Effective)
            && lives_in(lost, ctx)
        {
            found.push(Found {
                line,
                clash: self.clash(chord, ctx, kept, lost),
            });
        }
        // (ii) lose: a row his line set holds the chord where it lives, but
        // something else answers it there — unless that was so before his
        // lines too (the table's own design).
        let Some(kept) = now else { return };
        for row in reachable_rows(ctx) {
            let Some(line) = self.line[row] else { continue };
            let unchanged = self.has(row, chord, Over::Base) && before == now;
            if row == kept || !self.has(row, chord, Over::Effective) || unchanged {
                continue;
            }
            if lives_in(row, ctx) {
                found.push(Found {
                    line,
                    clash: Clash {
                        key: chord,
                        context: ctx,
                        kept: named(kept),
                        lost: named(row),
                        keeps: None,
                    },
                });
            }
        }
    }

    fn clash(&self, key: KeyChord, ctx: ContextId, kept: usize, lost: usize) -> Clash {
        Clash {
            key,
            context: ctx,
            kept: named(kept),
            lost: named(lost),
            keeps: self.left_in(&defaults()[lost].action, ctx, key),
        }
    }

    /// The action of his line `at`: the row it set that is not a follower,
    /// else the clash's own.
    fn line_action(&self, at: usize, clash: &Clash) -> ActionId {
        let follower = |row: usize| FOLLOWS.iter().any(|(f, _)| *f == defaults()[row].action);
        self.line
            .iter()
            .enumerate()
            .find(|(row, line)| **line == Some(at) && !follower(*row))
            .map_or(clash.kept, |(row, _)| defaults()[row].action)
    }

    /// Every distinct chord any row reachable in `ctx` holds, before or
    /// after his lines, in table order.
    fn chords_reaching(&self, ctx: ContextId) -> Vec<KeyChord> {
        let mut out: Vec<KeyChord> = Vec::new();
        for row in reachable_rows(ctx) {
            for chord in self.chords[row].iter().chain(&self.base[row]) {
                if !out.contains(chord) {
                    out.push(*chord);
                }
            }
        }
        out
    }

    /// What else the losing action still answers to in `ctx`, after his
    /// lines.
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
            .find(|c| self.resolve(ctx, *c, Over::Effective) == Some(row))
    }
}

/// The row's action as his config names it: a follower is its leader's
/// (`preview.confirm_merge` is `graph.merge`'s key in a preview), so its
/// clash is the leader's and is named once.
fn named(row: usize) -> ActionId {
    let action = defaults()[row].action;
    FOLLOWS
        .iter()
        .find(|(follower, _)| *follower == action)
        .map_or(action, |(_, leader)| *leader)
}

/// Where `row`'s action acts: its own place, a place that borrows it, or
/// anywhere for a `Global` one. A line losing in a place it only falls
/// through to is not a problem (J8).
fn lives_in(row: usize, ctx: ContextId) -> bool {
    let binding = &defaults()[row];
    binding.context == ctx
        || binding.context == ContextId::Global
        || ctx.borrows().contains(&binding.action)
}

/// One clash per line, key, keeper and loser: a place that only falls
/// through to another inherits its clash, so the clash is named once, in
/// the keeper's own place when it is among them, else the first.
fn once_each(found: Vec<Found>) -> Vec<Found> {
    let mut out: Vec<Found> = Vec::new();
    for f in found {
        let same = out.iter_mut().find(|o| {
            o.line == f.line
                && o.clash.key == f.clash.key
                && o.clash.kept == f.clash.kept
                && o.clash.lost == f.clash.lost
        });
        match same {
            None => out.push(f),
            Some(o) => {
                let home = |c: &Clash| index_of(c.kept.0).map(|r| defaults()[r].context);
                if home(&f.clash) == Some(f.clash.context)
                    && home(&o.clash) != Some(o.clash.context)
                {
                    *o = f;
                }
            }
        }
    }
    out
}
