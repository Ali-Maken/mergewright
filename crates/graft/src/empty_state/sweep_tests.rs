//! The start screen's behavioural sweep (m8/09d Major 4): every row a key
//! can reach there, pressed through `outside_edge` by its default chord, in
//! three fixtures — the list (cursor mid-list), the init box open (a
//! confirm box: `Confirm`'s rows, D6), and the open prompt (a text field:
//! every key typed). The oracle is what the press does — the step and the
//! state after — never a copy of `translate`.
//!
//! Under a sweep keymap (every row the place reaches moved to its own `alt`
//! chord), the new chord does what the default did, and the default does
//! what `F20` (a key nobody answers) does; in the prompt both are typed.

use std::path::PathBuf;

use graft_core::event::{KeyCode, KeyPress};
use graft_ui::keymap::{ContextId, KeyChord, KeySwitches, Keymap};

use super::{OutsideState, OutsideStep, Prompt, outside_edge, outside_key};
use crate::recent::Recent;
use graft_ui::text_input::TextField;

/// `(place, id, why)`: the place answers the key itself and does nothing
/// with it.
const LIST: &str = "empty_state.rs:302 the list answers q o i ⏎ j/↓ k/↑ alone (and ? ⌃P :, act)";
const BOX: &str = "empty_state.rs:259 the init box keeps all but y esc (draws y, esc)";

const SWALLOWED_BY_DESIGN: &[(ContextId, &str, &str)] = &[
    (ContextId::EmptyState, "app.next_pane", LIST),
    (ContextId::EmptyState, "app.prev_pane", LIST),
    (ContextId::EmptyState, "app.back", LIST),
    (ContextId::EmptyState, "app.undo", LIST),
    (ContextId::EmptyState, "app.redo", LIST),
    (ContextId::EmptyState, "app.refresh", LIST),
    (ContextId::EmptyState, "app.top", LIST),
    (ContextId::EmptyState, "app.bottom", LIST),
    (ContextId::EmptyState, "app.page_down", LIST),
    (ContextId::EmptyState, "app.page_up", LIST),
    (ContextId::EmptyState, "app.find", LIST),
    (ContextId::EmptyState, "app.next_match", LIST),
    (ContextId::EmptyState, "app.prev_match", LIST),
    (ContextId::EmptyState, "app.menu", LIST),
    (ContextId::Confirm, "confirm.choose", BOX),
    (ContextId::Confirm, "confirm.left", BOX),
    (ContextId::Confirm, "confirm.right", BOX),
    (ContextId::Confirm, "confirm.cycle", BOX),
    (ContextId::Confirm, "app.next_pane", BOX),
    (ContextId::Confirm, "app.prev_pane", BOX),
    (ContextId::Confirm, "app.undo", BOX),
    (ContextId::Confirm, "app.redo", BOX),
    (ContextId::Confirm, "app.refresh", BOX),
    (ContextId::Confirm, "app.quit", BOX),
    (ContextId::Confirm, "app.down", BOX),
    (ContextId::Confirm, "app.up", BOX),
    (ContextId::Confirm, "app.top", BOX),
    (ContextId::Confirm, "app.bottom", BOX),
    (ContextId::Confirm, "app.page_down", BOX),
    (ContextId::Confirm, "app.page_up", BOX),
    (ContextId::Confirm, "app.find", BOX),
    (ContextId::Confirm, "app.prev_match", BOX),
    // m8/09e: `n` no longer declines raw; swallowed like every other key.
    (ContextId::Confirm, "app.next_match", BOX),
    (ContextId::Confirm, "app.open", BOX),
    (ContextId::Confirm, "app.menu", BOX),
    // m8/11 decision 7: a confirm box swallows `?`, `⌃P` and `:` too.
    (ContextId::Confirm, "app.help", BOX),
    (ContextId::Confirm, "app.palette", BOX),
];

/// `(place, id, why)`: answered, but the fixture gives it nothing to change.
/// None: the list is mid-list, so every answered key changes something.
const NO_EFFECT_IN_FIXTURE: &[(ContextId, &str, &str)] = &[];

/// Rows the table holds that a later ticket answers; each deletes its own.
/// None since m8/19b answered `t`.
const NOT_YET_ANSWERED: &[(&str, &str)] = &[];

/// Cases that acted when this sweep landed (2026-10-07: 13; 16 after
/// m8/18c answered `c`; 17 after m8/19b answered `t`). It may grow; a drop
/// means a key stopped doing something.
const ACTING_AT_LANDING: usize = 17;

/// Never moved by the sweep: `⌃C` is never his to move; suspend is off
/// unless his switch is on.
const UNMOVED: &[&str] = &["app.quit_now", "app.suspend"];

fn state() -> OutsideState {
    let recent = |path: &str| Recent {
        path: PathBuf::from(path),
        opened: std::time::UNIX_EPOCH,
    };
    OutsideState {
        folder: PathBuf::from("/me/code"),
        missing: false,
        home: Some(PathBuf::from("/me")),
        recents: vec![
            recent("/me/code/payments-service"),
            recent("/me/code/web-dashboard"),
            recent("/me/dotfiles"),
        ],
        // Mid-list, so both `j` and `k` have somewhere to go.
        cursor: 1,
        prompt: None,
        problem: None,
        palette: None,
        config_problems: Vec::new(),
        first_launch: crate::first_launch::FirstLaunch::default(),
        credential: None,
    }
}

/// The three fixtures, each with the place its keys translate in.
fn fixtures() -> Vec<(&'static str, ContextId, OutsideState)> {
    let mut boxed = state();
    boxed.prompt = Some(Prompt::ConfirmInit);
    let mut prompt = state();
    prompt.prompt = Some(Prompt::Open(TextField::new("/me/code/")));
    vec![
        ("list", ContextId::EmptyState, state()),
        ("init box", ContextId::Confirm, boxed),
        ("open prompt", ContextId::EmptyState, prompt),
    ]
}

/// What a press does: the step, and the state after it.
fn did(state: &OutsideState, keys: &Keymap, press: KeyPress) -> (OutsideStep, OutsideState) {
    let mut after = state.clone();
    let step = outside_edge(&mut after, keys, press);
    (step, after)
}

/// Every movable row of the start screen and the init box on its own
/// `alt` chord: the keymap, and per id its new chord.
fn swept() -> (Keymap, Vec<(&'static str, KeyChord)>) {
    let shipped = Keymap::shipped();
    let mut ids: Vec<&'static str> = Vec::new();
    for ctx in [ContextId::EmptyState, ContextId::Confirm] {
        for (row, _) in shipped.reachable(ctx) {
            let id = row.action.0;
            if !row.context.is_text_field() && !UNMOVED.contains(&id) && !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    let free = ('a'..='z').chain('0'..='9');
    let moved: Vec<(&'static str, KeyChord)> = ids
        .into_iter()
        .zip(free)
        .map(|(id, c)| (id, KeyChord::alt(KeyCode::Char(c))))
        .collect();
    let lines: Vec<(String, Vec<String>)> = moved
        .iter()
        .map(|(id, chord)| ((*id).to_owned(), vec![chord.label().0]))
        .collect();
    let (keys, warnings) = Keymap::with_overrides(&lines, KeySwitches::default());
    assert_eq!(warnings, [], "the sweep keymap is clean");
    (keys, moved)
}

fn unanswered() -> KeyPress {
    KeyPress::plain(KeyCode::Function(20))
}

fn listed(ctx: ContextId, id: &str) -> bool {
    SWALLOWED_BY_DESIGN
        .iter()
        .chain(NO_EFFECT_IN_FIXTURE)
        .any(|(at, listed, _)| *at == ctx && *listed == id)
        || NOT_YET_ANSWERED.iter().any(|(listed, _)| *listed == id)
}

/// The counts and failures of one sweep.
#[derive(Default)]
struct Tally {
    cases: usize,
    acting: usize,
    listed: usize,
    failures: Vec<String>,
}

/// One default press: typed in the prompt, else it acts or is listed.
fn judge(tally: &mut Tally, name: &str, ctx: ContextId, id: &str, label: &str, acted: bool) {
    match (acted, listed(ctx, id)) {
        (true, true) => tally
            .failures
            .push(format!("{name}: {id} ({label}) listed, but acts")),
        (false, false) => tally
            .failures
            .push(format!("{name}: {id} ({label}) does nothing")),
        (true, false) => tally.acting += 1,
        (false, true) => tally.listed += 1,
    }
}

/// Every reachable row of `ctx`, pressed in `fixture`.
fn sweep_fixture(
    (name, ctx, fixture): (&str, ContextId, OutsideState),
    (swept, moved): (&Keymap, &[(&'static str, KeyChord)]),
    tally: &mut Tally,
) {
    let shipped = Keymap::shipped();
    let new_chord = |id: &str| moved.iter().find(|(m, _)| *m == id).map(|(_, c)| *c);
    let typed = matches!(fixture.prompt, Some(Prompt::Open(_)));
    let nothing = did(&fixture, shipped, unanswered());
    for (row, _) in shipped.reachable(ctx) {
        let id = row.action.0;
        let mut first = None;
        for &chord in row.default {
            tally.cases += 1;
            let (press, label) = (chord.to_press(), chord.label().0);
            let shipped_did = did(&fixture, shipped, press);
            if typed {
                // A text field passes every key on as typed (R4).
                let mut raw = fixture.clone();
                let step = outside_key(&mut raw, press);
                if shipped_did != (step, raw) {
                    tally
                        .failures
                        .push(format!("{name}: {id} ({label}) not typed"));
                }
            } else {
                judge(tally, name, ctx, id, &label, shipped_did != nothing);
            }
            // Moved, the default does what `F20` does; typed or never moved,
            // what it did.
            let keeps = typed || new_chord(id).is_none();
            let want_old = if keeps { &shipped_did } else { &nothing };
            if did(&fixture, swept, press) != *want_old {
                tally.failures.push(format!("{name}: {id}'s old {label}"));
            }
            first.get_or_insert(shipped_did);
        }
        if let Some(to) = new_chord(id) {
            let want = if typed {
                did(&fixture, shipped, to.to_press())
            } else {
                first.unwrap_or_else(|| nothing.clone())
            };
            if did(&fixture, swept, to.to_press()) != want {
                tally
                    .failures
                    .push(format!("{name}: {id}'s new {}", to.label().0));
            }
        }
    }
}

#[test]
fn every_row_the_start_screen_reaches_acts_or_is_listed_and_follows_its_remap() {
    let (swept, moved) = swept();
    let mut tally = Tally::default();
    for fixture in fixtures() {
        sweep_fixture(fixture, (&swept, &moved), &mut tally);
    }
    let Tally {
        cases,
        acting,
        listed,
        failures,
    } = tally;
    println!("{cases} cases: {acting} act, {listed} listed");
    assert_eq!(failures, Vec::<String>::new());
    assert!(
        acting >= ACTING_AT_LANDING,
        "{acting} act, {ACTING_AT_LANDING} did at landing"
    );
    for (ctx, id, _) in SWALLOWED_BY_DESIGN.iter().chain(NO_EFFECT_IN_FIXTURE) {
        assert!(
            Keymap::shipped()
                .reachable(*ctx)
                .iter()
                .any(|(b, _)| b.action.0 == *id),
            "{ctx:?}: {id} is not reachable there"
        );
    }
}
