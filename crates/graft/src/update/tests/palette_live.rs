//! `?` lists **live** keys here (m8 design note :56; m8/09f Major 2): every
//! key a place's help prints, pressed in that place, changes what he sees —
//! the frame, the commands, `quit`, the Map's row ([`Seen`], the sweep's
//! projection, which holds 09e's frame-or-command oracle), and which pane
//! has the keyboard.
//!
//! At the Map, the stash list and the journal this holds outright. Every
//! other place `?` opens is held to [`DEAD_ELSEWHERE`] exactly: a new dead
//! row fails, and so does a fixed one until its line is removed.

mod dead;

use graft_ui::keymap::{ActionId, ContextId, KeyChord};

use dead::DEAD_ELSEWHERE;

use super::keymap_edge::{Seen, fixture, seen, sized_to_draw, unanswered, walk};
use super::*;
use crate::update::palette::{PaletteTarget, rows};

/// One place `?` is opened on: its name in a failure, and the state.
struct Place {
    name: String,
    state: AppState,
}

/// `?` pressed on `state`: the place it opened on and each row it lists
/// with a key — the action and the chord the row spells. `None` where `?`
/// opens nothing (a confirm box, a text field).
fn listed_keys(state: &AppState) -> Option<(ContextId, Vec<(ActionId, KeyChord)>)> {
    let help = Keymap::shipped().chords(&ActionId("app.help"))[0];
    let (asked, _) = update(state, AppEvent::Core(Event::Key(help.to_press())));
    let palette = asked.palette.as_deref()?;
    let keys = rows(&asked, palette)
        .into_iter()
        .flat_map(|(_, group)| group)
        .filter_map(|(row, target)| {
            let PaletteTarget::Action(id) = target else {
                return None;
            };
            let label = row.key?.0;
            let chord = state
                .keymap
                .chords(&id)
                .iter()
                .copied()
                .find(|chord| chord.label().0 == label);
            chord.map(|chord| (id, chord))
        })
        .collect();
    Some((palette.opened_from, keys))
}

/// What a press did: [`Seen`], and which pane has the keyboard — drawn as
/// a border colour only, which a text frame cannot show (rule 5), so read
/// from the state as `Seen` reads the Map's row. `⇥` under the stash list
/// moves it and nothing else (`stash.rs:199`, `detail/keys.rs:158`).
fn did(state: &AppState, press: KeyPress) -> (Seen, DetailFocus) {
    let focus = update(state, AppEvent::Core(Event::Key(press)))
        .0
        .detail
        .focus;
    (seen(state, press), focus)
}

/// Where `?` opened on `place`, and the rows it lists there whose key,
/// pressed there, changes nothing; `None` where `?` opens nothing.
fn dead_at(place: &Place) -> Option<(ContextId, Vec<ActionId>)> {
    let (ctx, keys) = listed_keys(&place.state)?;
    assert!(!keys.is_empty(), "{}: `?` lists keys", place.name);
    let idle = did(&place.state, unanswered());
    let dead = keys
        .into_iter()
        .filter(|(_, chord)| did(&place.state, chord.to_press()) == idle)
        .map(|(id, _)| id)
        .collect();
    Some((ctx, dead))
}

fn place(ctx: ContextId) -> Place {
    Place {
        name: format!("{ctx:?}"),
        state: fixture(ctx).expect("a fixture"),
    }
}

/// A state of the same place where `id` has something to act on, for the
/// rows the sweep's fixture gives nothing (its `NO_EFFECT_IN_FIXTURE` rows
/// at the three overlays): `⇥`/`⇧⇥` with a pane open to focus, `n`/`N`
/// with a lost row above and below the cursor.
fn something_to_act_on(ctx: ContextId, id: ActionId) -> Option<AppState> {
    use graft_git::history_reads::LostRun;
    use graft_ops::journal_list::{HeldBy, ListRow, LostCommit};
    let overlay = match ctx {
        ContextId::Stash => Shown::Stash,
        ContextId::Journal => Shown::Journal,
        _ => return None,
    };
    match id.0 {
        "app.next_pane" | "app.prev_pane" => {
            let mut state = commit_pane(walk(), false);
            state.detail.focus = DetailFocus::Graph;
            Some(sized_to_draw(overlay_over(state, overlay)))
        }
        "app.next_match" | "app.prev_match" if ctx == ContextId::Journal => {
            let mut state = fixture(ctx)?;
            let lost = |tip: u8| ListRow::Lost {
                event: 0,
                commit: LostCommit {
                    run: LostRun {
                        tip: Oid::from_bytes(&[tip; Oid::SHA1_BYTES]).expect("20 bytes"),
                        under: 0,
                    },
                    subject: Some(format!("lost {tip}")),
                    held_by: HeldBy::Head,
                },
            };
            let rows = &mut Arc::make_mut(state.journal.list.as_mut()?).rows;
            rows.insert(0, lost(7));
            rows.push(lost(8));
            Some(state)
        }
        _ => None,
    }
}

/// Map, stash list, journal open, the graph focused: `?` lists only keys
/// that act there.
#[test]
fn every_key_help_lists_at_an_overlay_changes_what_he_sees() {
    let mut dead: Vec<String> = Vec::new();
    for ctx in [ContextId::Map, ContextId::Stash, ContextId::Journal] {
        let at = place(ctx);
        let (opened, idle) = dead_at(&at).expect("`?` opens over an overlay");
        assert_eq!(opened, ctx, "{}", at.name);
        for id in idle {
            let acts_elsewhere = something_to_act_on(ctx, id).is_some_and(|state| {
                let richer = Place {
                    name: format!("{ctx:?} ({})", id.0),
                    state,
                };
                let (opened, idle) = dead_at(&richer).expect("`?` opens");
                assert_eq!(opened, ctx, "{}", richer.name);
                !idle.contains(&id)
            });
            if !acts_elsewhere {
                dead.push(format!("{}: {}", at.name, id.0));
            }
        }
    }
    assert!(
        dead.is_empty(),
        "`?` lists {} key(s) that change nothing:\n  {}",
        dead.len(),
        dead.join("\n  ")
    );
}

/// Every other place `?` opens on: each place with a sweep fixture, and a
/// pane focused over each overlay (`context_of` → `Diff`/`Changes`, which
/// keep the graph's section — the ratchet m8/09f leaves).
fn elsewhere() -> Vec<Place> {
    let overlays = [ContextId::Map, ContextId::Stash, ContextId::Journal];
    let mut places: Vec<Place> = ContextId::ALL
        .iter()
        .filter(|ctx| !overlays.contains(ctx))
        .filter_map(|&ctx| {
            fixture(ctx).map(|state| Place {
                name: format!("{ctx:?}"),
                state,
            })
        })
        .collect();
    for overlay in EVERY_OVERLAY {
        places.push(Place {
            name: format!("diff over {overlay:?}"),
            state: sized_to_draw(overlay_over(commit_pane(walk(), false), overlay)),
        });
        places.push(Place {
            name: format!("changes over {overlay:?}"),
            state: sized_to_draw(overlay_over(changes_pane_open(walk(), false), overlay)),
        });
    }
    places
}

/// `?` elsewhere lists the keys in [`DEAD_ELSEWHERE`], which change nothing
/// where it lists them. Exact: a new dead row fails, and a fixed one fails
/// until its line goes. The conductor's gap row `M8/help-lists-dead-keys`
/// is that table.
#[test]
fn dead_rows_help_lists_elsewhere_match_the_known_table() {
    let mut found: Vec<(ContextId, ActionId)> = Vec::new();
    let mut opened = 0;
    for at in elsewhere() {
        let Some((ctx, idle)) = dead_at(&at) else {
            continue;
        };
        opened += 1;
        for id in idle {
            if !found.contains(&(ctx, id)) {
                found.push((ctx, id));
            }
        }
    }
    assert!(opened > 0, "`?` opened somewhere");
    let known: Vec<(ContextId, ActionId)> = DEAD_ELSEWHERE
        .iter()
        .map(|(ctx, id, _)| (*ctx, *id))
        .collect();
    let new: Vec<String> = found
        .iter()
        .filter(|row| !known.contains(row))
        .map(|(ctx, id)| format!("(ContextId::{ctx:?}, ActionId(\"{}\"), \"\"),", id.0))
        .collect();
    let fixed: Vec<String> = known
        .iter()
        .filter(|row| !found.contains(row))
        .map(|(ctx, id)| format!("{ctx:?} {}", id.0))
        .collect();
    assert!(
        new.is_empty() && fixed.is_empty(),
        "new dead rows:\n{}\nfixed (remove from DEAD_ELSEWHERE):\n{}",
        new.join("\n"),
        fixed.join("\n")
    );
}
