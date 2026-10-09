//! `update` tests: hint coverage. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// The other half: a hint the table still calls planned must really be
/// unbound. Binding `m` to merge without flipping its hint to live fails
/// here — which is how the footer learns about a verb the moment it exists
/// (`hint-bar-redesign` scope item 6; m5-03 review, Major 6).
///
/// Over actions since m8/10: a planned hint names the row it will do, so
/// the banner's planned `e edit` is `operation.edit` and the changes pane's
/// bound `e edit` is `changes.edit` — two rows, told apart. An action a
/// planned hint names that another context draws live (its key planned in
/// one place, built in another) is held by the Live half instead.
#[test]
fn every_planned_hint_is_still_unbound() {
    use graft_ui::graph_screen::{HintState, every_hint};

    let bound = super::super::keys::bound_actions();
    let live: Vec<ActionId> = every_hint()
        .iter()
        .filter(|hint| hint.state == HintState::Live)
        .flat_map(actions_of)
        .collect();
    for hint in every_hint() {
        for action in actions_of(&hint) {
            match hint.state {
                HintState::Planned => {
                    if live.contains(&action) {
                        continue;
                    }
                    assert!(
                        !bound.contains(&action),
                        "`{} {}` ({}) is bound now: flip it to `hint(..)` in \
                         graph_screen::hints",
                        hint.key(),
                        hint.verb,
                        action.0
                    );
                }
                HintState::Live => assert!(
                    bound.contains(&action),
                    "`{} {}` ({}) is hinted as live but nothing binds it",
                    hint.key(),
                    hint.verb,
                    action.0
                ),
            }
        }
    }
}

/// Bound actions the hint bar draws **nowhere**, each with the reason it is
/// not a defect — the named exemption list
/// [`every_bound_action_is_hinted_live_or_is_named_here`] walks against
/// (m5/33, review M4).
///
/// All are `docs/design/02-keymap.md` §2.2 globals: keys live in every pane,
/// which the bar does not list because the bar lists the *focused context's*
/// keys. (Before m8/10 the list held `esc dismiss` too; as an action that is
/// `app.back`, which the bar draws as `esc close`/`esc cancel`.)
const NEVER_HINTED: &[(ActionId, &str)] = &[
    (
        ActionId("app.top"),
        "keymap §2.2: global movement, live in every pane",
    ),
    (
        ActionId("app.bottom"),
        "keymap §2.2: global movement, live in every pane",
    ),
    (
        ActionId("app.page_up"),
        "keymap §2.2: global movement, live in every pane",
    ),
    (
        ActionId("app.page_down"),
        "keymap §2.2: global movement, live in every pane",
    ),
    (ActionId("app.refresh"), "keymap §2.2: global"),
    (
        ActionId("app.redo"),
        "keymap §2.2: global, and §2.1's Shift pair of the `u undo` the bar does draw",
    ),
];

/// **The converse direction** (m5/33, review M4).
/// [`every_planned_hint_is_still_unbound`] walks the hint tables and asks
/// whether each entry is bound; it cannot see an action that is bound and
/// is in **no table at all**, and [`super::super::keys::bound_actions`] is
/// hand-written, so that gap is real rather than theoretical.
///
/// This walks the other way: every action the app binds is drawn
/// `HintState::Live` in some context, or is named in [`NEVER_HINTED`] with
/// its reason. The exemption list is walked too — a row that stopped being
/// bound, or that is drawn now, fails here rather than sitting stale.
#[test]
fn every_bound_action_is_hinted_live_or_is_named_here() {
    use graft_ui::graph_screen::{HintState, every_hint};

    let bound = super::super::keys::bound_actions();
    // `every_hint` is the union of every context's own list plus the tail
    // and the doorway hints `whole_bar` adds, so "live in here" is "live on
    // some bar". The journal, stash and Map footers are no hint contexts
    // since m8/11e: what they draw is read off the drawn frame and resolved
    // by the keymap in the place's context.
    let footers = place_footers::place_footer_states()
        .into_iter()
        .flat_map(|(_, state)| {
            let (state, frame) =
                place_footers::drawn_at(&state, place_footers::WIDE.0, place_footers::WIDE.1);
            place_footers::footer_actions(&state, &place_footers::footer_row(&frame))
        });
    let live: Vec<ActionId> = every_hint()
        .iter()
        .filter(|hint| hint.state == HintState::Live)
        .flat_map(actions_of)
        .chain(footers)
        .collect();
    let exempt: Vec<ActionId> = NEVER_HINTED.iter().map(|(action, _)| *action).collect();

    let unhinted: Vec<&str> = bound
        .iter()
        .filter(|action| !live.contains(action) && !exempt.contains(action))
        .map(|action| action.0)
        .collect();
    assert!(
        unhinted.is_empty(),
        "bound, and drawn on no bar in any context: {unhinted:?}. Either hint it, or add \
         it to NEVER_HINTED with the reason it is never drawn"
    );

    for (action, why) in NEVER_HINTED {
        assert!(
            bound.contains(action),
            "`{}` is on the never-hinted list ({why}) but nothing binds it any more — take \
             the row off",
            action.0
        );
        assert!(
            !live.contains(action),
            "`{}` is drawn live now, so the never-hinted list is wrong about it ({why}) — \
             take the row off",
            action.0
        );
    }
}

// The vertical axis of ticket 19's principle: count from what you were
// given, not from the screen (review M1).
