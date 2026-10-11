//! `update` tests: dead keys. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// One cell of [`every_key_the_bar_draws_is_answered`]'s sweep: this
/// context, at this width, with the pane open or not — every hint its bar
/// draws that the handler with focus does nothing with.
pub(super) fn dead_keys_on(
    context: &HintContext,
    divider: Divider,
    columns: u16,
    bound: &[ActionId],
) -> Vec<String> {
    states_for(context, divider, columns)
        .into_iter()
        .flat_map(|state| dead_keys_in(&settled(state), context, divider, columns, bound))
        .collect()
}

/// The hints `bar` draws at `columns`. The finish summary draws its keys
/// in its own frame, and below the widget's floor offers `esc back` alone
/// (`view::finish::finish_hints`): that is the bar to press, not the
/// table's whole list.
pub(super) fn bar_hints(
    state: &AppState,
    bar: &HintContext,
    divider: Divider,
    columns: u16,
) -> Vec<graft_ui::graph_screen::Hint> {
    if matches!(bar, HintContext::Finish { .. }) {
        crate::view::finish::finish_hints(state, columns)
    } else {
        graft_ui::graph_screen::hints_for(
            bar,
            divider,
            columns,
            graft_ui::keymap::Keymap::shipped(),
        )
    }
}

/// Both bars `state` can be under: the one the enum names, and the one
/// `view::rows` actually asks for when handed the state (they differ only in
/// row decoration, but a classifier that reads the state wrongly is exactly
/// how a live key and a drawn key come apart — so the keys it picks are
/// pressed too).
fn bars_for(state: &AppState, context: &HintContext) -> Vec<HintContext> {
    let mut drawn = vec![context.clone()];
    if classified_form(context).is_some() {
        let banner = crate::update::banner::banner_view(state);
        let asked = crate::view::rows::footer_context(state, &[], banner.as_ref());
        if asked != *context {
            drawn.push(asked);
        }
    }
    drawn
}

/// [`dead_keys_on`] against one of the screens that context can be the bar
/// for — every hint the bar draws that this screen's focused handler does
/// nothing with.
fn dead_keys_in(
    state: &AppState,
    context: &HintContext,
    divider: Divider,
    columns: u16,
    bound: &[ActionId],
) -> Vec<String> {
    let state = state.clone();
    let drawn = bars_for(&state, context);
    let idle = nothing_happened(&state);
    // Any key at all ends a drag: `update::drag::drag_key` takes `state.drag`
    // before it reads the key. So while a badge is in the air the inert key
    // moves the state too and no press can be told from another; the sweep
    // says which states it is blind in rather than reading every key there as
    // answered by accident.
    let blind = idle.0 != state;
    // The sweep's own baseline must read as *not* an answer, or every key
    // here would read as one. `nothing_happened` makes that true today by
    // asserting the two inert presses agree outright; pinned again here, at
    // the point that depends on it, so relaxing the baseline cannot quietly
    // turn the sweep green (m5/33, review M2).
    assert!(
        blind
            || !did_something(
                &idle,
                &outcome(&state, Press::Key(KeyPress::plain(ALSO_INERT)))
            ),
        "{context:?} ({divider:?}) at {columns} columns: a key nothing binds reads as an \
         answer, so every key drawn on this bar would"
    );
    // A live confirm box is exempt too: it swallows every key it has no
    // use for, the inert one included, so no key here reaches the bar. Its three hints are pinned
    // one by one where the box is (`confirm::tests`: `←`/`→`
    // `left_on_cancel_stays_on_cancel`, `right_moves_onto_the_acting_choice_and_clamps`;
    // `⏎` `enter_on_confirm_dispatches_the_operation`; `esc`
    // `escape_closes_without_dispatching`).
    assert!(
        !blind || state.drag.is_some() || state.confirm.is_some(),
        "{context:?} ({divider:?}) at {columns} columns: a key nothing binds moved the \
         state, and not by dropping a dragged badge — every key here now reads as answered \
         whether it is or not"
    );
    // The divider the **app** would draw this bar with, not the fixture's
    // knob: under 100 columns the pane is a tab strip and
    // `view::rows::divider_shown` says there is nothing to drag, so asking
    // for the `Shown` bar here would be asking for a bar the app never draws
    // (m5/33, review M6).
    let drawn_divider = if crate::view::divider_shown(&state) {
        Divider::Shown
    } else {
        Divider::Hidden
    };

    let mut dead = Vec::new();
    let mut pressed: Vec<(&[HintKey], &str)> = Vec::new();
    for bar in &drawn {
        for hint in bar_hints(&state, bar, drawn_divider, columns) {
            let unbound: Vec<&str> = actions_of(&hint)
                .into_iter()
                .filter(|action| !bound.contains(action))
                .map(|action| action.0)
                .collect();
            assert!(
                unbound.is_empty(),
                "{bar:?} at {columns} columns hints `{} {}`, whose {unbound:?} nothing binds",
                hint.key(),
                hint.verb
            );
            if pressed.contains(&(hint.keys, hint.verb)) {
                continue;
            }
            pressed.push((hint.keys, hint.verb));
            if !blind && !answered(&state, &idle, &hint) {
                // Both names: the screen the state is, and the bar the key
                // came off — they differ exactly when the classifier reads
                // the state as another screen, and then which one is wrong is
                // the whole question.
                dead.push(format!(
                    "{context:?} ({divider:?}) at {columns} columns, bar of {bar:?}, \
                     draws `{} {}`",
                    hint.key(),
                    hint.verb
                ));
            }
        }
    }
    dead
}

/// The states [`every_key_the_bar_draws_is_answered`] presses keys into are
/// the screens they claim to be — **the app's own classifier says so**, not
/// the fixture. Without this the sweep could press every key into the wrong
/// state and stay green, which is the same fault it exists to catch.
///
/// With no drawn rows to read badges or a marker off, `view::rows`
/// answers `PlainRow` where the bar's context says `BranchRow` or `Marked`:
/// the difference is decoration on the row, and the keys are the same. That
/// is the only gap left: since m5/42 the three overlays are classified like
/// everything else, so a fixture with the journal, the stash or the Map
/// view open is checked against the bar the app would really draw over it.
#[test]
fn every_swept_state_is_the_context_it_claims() {
    use graft_ui::graph_screen::{Divider, every_context};

    for context in every_context() {
        for divider in [Divider::Shown, Divider::Hidden] {
            let Some(want) = classified_form(&context) else {
                continue;
            };
            for state in states_for(&context, divider, 160) {
                let banner = crate::update::banner::banner_view(&state);
                assert_eq!(
                    crate::view::rows::footer_context(&state, &[], banner.as_ref()),
                    want,
                    "a fixture for {context:?} ({divider:?}) is not that screen"
                );
            }
        }
    }
}

/// What `view::rows::footer_context` answers for [`state_for`]'s state,
/// with no drawn rows to read a badge or a marker from, or `None` where the
/// graph's footer never classifies the context at all.
#[allow(
    clippy::too_many_lines,
    reason = "one arm per `HintContext` variant, exhaustive on purpose: it grows with the enum"
)]
fn classified_form(context: &HintContext) -> Option<HintContext> {
    match context {
        // `R rewrite` is decoration on a drawn row too (08, ruling 10).
        // So are `o`/`a` (m7-power-tools/40): `row_verbs` reads a drawn cursor row.
        // So is a stash's row (m8/15 A4): `cursor_row_context` reads it off
        // the drawn cursor row, as it reads a badge.
        HintContext::BranchRow { .. }
        | HintContext::PlainRow { .. }
        | HintContext::StashRow
        | HintContext::Marked(_)
        | HintContext::Rewritable(_) => {
            Some(HintContext::PlainRow { verbs: graft_ui::graph_screen::hints::RowVerbs::NONE })
        }
        // `B` answers on a file (`blamable`), `space` folds `commit_pane`'s
        // `src/` row (`folds`), neither on its `✉ message` row
        // (`blame_screens`).
        HintContext::PaneFocused {
            opens,
            blamable,
            folds,
            ..
        } => Some(HintContext::PaneFocused {
            row: Box::new(HintContext::PlainRow { verbs: graft_ui::graph_screen::hints::RowVerbs::NONE }),
            opens: *opens,
            blamable: *blamable,
            folds: *folds,
            graph_hidden: false,
        }),
        // The inner bar, led by the way back.
        HintContext::BackToBlame(inner) => {
            classified_form(inner).map(|inner| HintContext::BackToBlame(Box::new(inner)))
        }
        HintContext::BackToHistory(inner) => {
            classified_form(inner).map(|inner| HintContext::BackToHistory(Box::new(inner)))
        }
        // The resolver is an overlay now, and `view::rows::overlay_context`
        // answers exactly this form for it. `base`/`restart` are both
        // `true` (m6-conflicts/05): `B`/`d` act the moment they are
        // pressed, so this is the one combination the real app ever
        // produces — the other three have no state to classify and are
        // left unswept (their two hints are `Planned`, which `drawable`
        // filters out, so nothing dead can hide there).
        // A whole-file card (m6-05 review, Major 1) is the one other form.
        // The finish summary is an overlay too, and `overlay_context`
        // answers exactly its form (m6-conflicts/19).
        same @ (HintContext::Resolver {
            base: true,
            restart: true,
            whole_file: false,
            done: false,
            finishable: false,
            ..
        }
        | HintContext::Resolver {
            base: false,
            restart: false,
            whole_file: true,
            done: false,
            finishable: false,
            ..
        }
        | HintContext::Finish { .. }) => Some(same.clone()),
        // A done file (m6-conflicts/25): `resolver_context` answers the
        // document's flags with `done` set. Finishable (m6-conflicts/31)
        // is the done file after the last one, every file done — the one
        // place `c` acts in the resolver; a document or a card open is a
        // file still conflicted, so `c` refuses there and those forms
        // never reach the app.
        HintContext::Resolver {
            done: true,
            roles,
            finishable,
            ..
        } => Some(HintContext::Resolver {
            base: true,
            restart: true,
            whole_file: false,
            done: true,
            roles: *roles,
            finishable: *finishable,
        }),
        // `Preview`'s box draws its own footer: `hint_context` is not the
        // code that decides it. The three `Resolver` forms above this arm
        // never reach the app.
        HintContext::Preview | HintContext::Resolver { .. } => None,
        // A standing search (m8/15b): the row's form, led by `n N`.
        HintContext::Searching(row) => {
            classified_form(row).map(|row| HintContext::Searching(Box::new(row)))
        }
        // The wrapper classifies as its row does.
        HintContext::OperationLive {
            row,
            c,
            way_to_banner,
            space_stages,
        } => classified_form(row).map(|row| HintContext::OperationLive {
            row: Box::new(row),
            c: *c,
            way_to_banner: *way_to_banner,
            space_stages: *space_stages,
        }),
        // The pane with an overlay behind it **is** classified, since
        // m5/42: `view::rows` reads `AppState::graph_overlay` and answers
        // `PaneOverOverlay` with the detail pane focused. Before that the
        // overlays were `None` here — and that hole is exactly why a sweep
        // carrying this invariant passed a bar of nine dead keys (the
        // ticket's own finding). With the graph focused an overlay draws its
        // own footer (m8/11e), swept by `every_key_a_place_footer_draws_is_answered`.
        HintContext::Picker
        | HintContext::PickerOnCurrent
        | HintContext::PaneOverOverlay { .. }
        | HintContext::WorkingTreeRow
        | HintContext::Banner { .. }
        | HintContext::ChangesPane { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        // The place's own bar (`view::rows::place_context`).
        | HintContext::Blame { .. }
        | HintContext::History { .. }
        // The rewrite editor's own bar (`view::rewrite_editor::rewrite_context`).
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage => Some(context.clone()),
    }
}
