//! The behavioural sweep (m8/09d, Major 3, D4): every row a key can reach
//! in each context with a fixture (`Keymap::reachable`, not own + Global),
//! pressed by its default chord under the default keymap. The oracle is
//! what that press does — never a copy of `resolve`:
//!
//! - it **acts** (differs from `F20`, a key nobody answers), or the row is
//!   listed with its reason: [`SWALLOWED_BY_DESIGN`], [`NO_EFFECT_IN_FIXTURE`],
//!   [`NOT_YET_ANSWERED`]; a listed row that acts fails;
//! - or the chord is [`SHADOWED`] there by another row, written by hand:
//!   the shadow must hold (the other row's swept chord does what the
//!   default chord did; this row's swept chord does nothing);
//! - and under the sweep keymap the row's new chord does what its default
//!   did, while the default does what `F20` does (in a text field, both go
//!   on as typed).

use super::*;

use super::sweep_swallowed::SWALLOWED_BY_DESIGN;
use super::sweep_tables::{DEFECTS, NO_EFFECT_IN_FIXTURE, ONE_CHORD_IDLE, SHADOWED};

/// In the table, answered by a later ticket; each deletes its own.
const NOT_YET_ANSWERED: &[(&str, &str)] = &[
    ("app.help", "ticket 11: `?`"),
    ("app.palette", "ticket 11: the palette"),
    ("palette.run", "ticket 11: the palette"),
    ("empty_state.clone", "ticket 18: clone"),
    ("empty_state.tour", "ticket 19: the practice tour"),
];

/// Cases that acted when this sweep landed (2026-10-07). It may grow; a
/// drop means a key stopped doing something.
const ACTING_AT_LANDING: usize = 396;

/// Rows whose chords are another row's (`Keymap`'s followers): swept with
/// their leader.
fn leader_of(id: &str) -> &str {
    match id {
        "preview.confirm_merge" => "graph.merge",
        "preview.confirm_rebase" => "graph.rebase",
        other => other,
    }
}

fn listed(ctx: ContextId, id: &str, label: &str) -> bool {
    ONE_CHORD_IDLE
        .iter()
        .chain(DEFECTS)
        .any(|(at, row, chord, _)| *at == ctx && *row == id && *chord == label)
        || SWALLOWED_BY_DESIGN
        .iter()
        .chain(NO_EFFECT_IN_FIXTURE)
        .any(|(at, listed, _)| *at == ctx && *listed == id)
        // In a text field a key not yet answered is still typed.
        || (!ctx.is_text_field() && NOT_YET_ANSWERED.iter().any(|(listed, _)| *listed == id))
}

fn shadow(ctx: ContextId, id: &str, chord: KeyChord) -> Option<&'static str> {
    let label = chord.label().0;
    SHADOWED
        .iter()
        .find(|(at, row, spelled, _, _)| *at == ctx && *row == id && *spelled == label)
        .map(|(_, _, _, by, _)| *by)
}

/// The counts and failures of one sweep.
#[derive(Default)]
struct Tally {
    cases: usize,
    acting: usize,
    listed: usize,
    shadowed: usize,
    failures: Vec<String>,
}

impl Tally {
    /// A default press that is not shadowed: it acts, or it is listed.
    fn judge(&mut self, ctx: ContextId, id: &str, label: &str, acts: bool) {
        match (acts, listed(ctx, id, label)) {
            (true, true) => self.fail(format!("{ctx:?}: {id} ({label}) listed, but acts")),
            (false, false) => self.fail(format!("{ctx:?}: {id} ({label}) does nothing")),
            (true, false) => self.acting += 1,
            (false, true) => self.listed += 1,
        }
    }

    fn fail(&mut self, why: String) {
        self.failures.push(why);
    }
}

/// The sweep keymap, the keymap holding both chords of every row, and each
/// moved row's new chord.
struct Keys {
    swept: Arc<Keymap>,
    both: Arc<Keymap>,
    moved: Vec<(&'static Binding, KeyChord)>,
}

impl Keys {
    fn new_chord(&self, id: &str) -> Option<KeyChord> {
        self.moved
            .iter()
            .find(|(b, _)| b.action.0 == leader_of(id))
            .map(|(_, chord)| *chord)
    }
}

/// Every reachable row of `ctx`, pressed in `fixture`.
fn sweep_place(ctx: ContextId, fixture: AppState, keys: &Keys, tally: &mut Tally) {
    let remapped = AppState {
        keymap: keys.swept.clone(),
        ..fixture.clone()
    };
    let base = AppState {
        keymap: keys.both.clone(),
        ..fixture
    };
    let nothing = seen_as(&base, unanswered(), &keys.swept);
    let typed = ctx.is_text_field();
    for (row, _) in keys.swept.reachable(ctx) {
        let id = row.action.0;
        let mut first_live: Option<Seen> = None;
        for &chord in row.default {
            tally.cases += 1;
            let label = chord.label().0;
            let did = seen_as(&base, chord.to_press(), &keys.swept);
            if let Some(by) = shadow(ctx, id, chord) {
                // The shadow holds: the row that answers it there does, on
                // its own swept chord, what the default did.
                tally.shadowed += 1;
                let holds = keys
                    .new_chord(by)
                    .is_some_and(|to| seen(&remapped, to.to_press()) == did);
                if !holds {
                    tally.fail(format!("{ctx:?}: {id}'s {label} is not {by}'s"));
                }
                continue;
            }
            tally.judge(ctx, id, &label, did != nothing);
            // Moved, the default does what `F20` does; never moved (`⌃C`)
            // or typed, it does what it did.
            let unmoved = keys.new_chord(id).is_none();
            let old_want = if typed || unmoved { &did } else { &nothing };
            if seen(&remapped, chord.to_press()) != *old_want {
                tally.fail(format!("{ctx:?}: {id}'s old {label}"));
            }
            first_live.get_or_insert(did);
        }
        // The new chord does what the first unshadowed default did, or —
        // every default shadowed — nothing. Typed, it is typed.
        if let Some(to) = keys.new_chord(id) {
            let want = if typed {
                seen_as(&base, to.to_press(), &keys.swept)
            } else {
                first_live.unwrap_or_else(|| nothing.clone())
            };
            if seen(&remapped, to.to_press()) != want {
                tally.fail(format!("{ctx:?}: {id}'s new {}", to.label().0));
            }
        }
    }
}

#[test]
fn every_reachable_row_acts_or_is_listed_and_follows_its_remap() {
    let (swept, moved) = swept();
    let keys = Keys {
        both: both(&moved),
        swept,
        moved,
    };
    let mut tally = Tally::default();
    for &ctx in ContextId::ALL {
        if let Some(fixture) = fixture(ctx) {
            sweep_place(ctx, fixture, &keys, &mut tally);
        }
    }
    let Tally {
        cases,
        acting,
        listed,
        shadowed,
        failures,
    } = tally;
    // The counts ride on every failure message: `update`'s purity scan
    // (`update_cannot_reach_the_world`) reads these files too, and no
    // printing macro may appear under `update/`.
    let counts = format!("{cases} cases: {acting} act, {listed} listed, {shadowed} shadowed");
    assert_eq!(failures, Vec::<String>::new(), "{counts}");
    assert!(
        acting >= ACTING_AT_LANDING,
        "{counts}; {ACTING_AT_LANDING} acted at landing"
    );
    // Every list entry names a reachable row of its place.
    let per_chord = ONE_CHORD_IDLE
        .iter()
        .chain(DEFECTS)
        .map(|(ctx, id, _, why)| (*ctx, *id, *why));
    let per_row = SWALLOWED_BY_DESIGN
        .iter()
        .chain(NO_EFFECT_IN_FIXTURE)
        .copied();
    for (ctx, id, _) in per_row.chain(per_chord) {
        assert!(
            keys.swept
                .reachable(ctx)
                .iter()
                .any(|(b, _)| b.action.0 == id),
            "{ctx:?}: {id} is not reachable there"
        );
    }
}
