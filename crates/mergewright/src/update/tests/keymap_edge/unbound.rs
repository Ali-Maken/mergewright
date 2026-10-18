//! An unbound key (m8/10 decision 2, 09c's Unbound ruling): drawn
//! nowhere, never as `—`, and a preview names its again key exactly while
//! it confirms (split out by 09d).

use super::*;

/// No bar of any context, at any width, draws a hint naming `action` under
/// `keys`, while the default keymap draws one somewhere.
fn no_bar_draws(action: ActionId, keys: &Keymap) {
    use mergewright_ui::graph_screen::{Divider, hints_for};
    let mut drawn_by_default = 0;
    for context in every_context() {
        for divider in [Divider::Shown, Divider::Hidden] {
            for columns in [40, 80, 120, 200] {
                drawn_by_default += hints_for(&context, divider, columns, Keymap::shipped())
                    .iter()
                    .filter(|hint| names(hint, action))
                    .count();
                for hint in hints_for(&context, divider, columns, keys) {
                    assert!(
                        !names(&hint, action),
                        "{context:?} {divider:?} {columns}: `{:?} {}` drawn unbound",
                        hint.key_text(keys),
                        hint.verb
                    );
                }
            }
        }
    }
    assert!(
        drawn_by_default > 0,
        "the default keymap draws it somewhere"
    );
}

/// The bottom border of the box `⏎` opens on the `.` menu's `row`: where a
/// box draws its keys.
fn keys_of_the_box_opened_from_menu_row(graph: &AppState, row: usize) -> String {
    let mut menu = update(graph, key(KeyCode::Char('.'))).0;
    for _ in 0..row {
        menu = update(&menu, key(KeyCode::Char('j'))).0;
    }
    let previewed = update(&menu, key(KeyCode::Enter)).0;
    assert!(
        previewed.preview.is_some(),
        "⏎ on the row opens the preview"
    );
    crate::view::tests::draw(&previewed, previewed.size.columns, previewed.size.rows)
        .lines()
        .find(|line| line.contains('╰'))
        .expect("the box's bottom border")
        .to_owned()
}

/// Ticket m8/10, decision 2: `graph.merge = "none"` leaves merge unbound.
/// No bar draws its hint at any width, the menu keeps the `Merge …` row (it
/// acts by `⏎`) with no `(key)`, and no screen draws a `—` the default
/// keymap's does not — an unbound key is never spelled as a dash.
#[test]
fn merge_unbound_is_drawn_nowhere_and_its_menu_row_names_no_key() {
    let merge = ActionId("graph.merge");
    let (map, warnings) = Keymap::with_overrides(
        &[("graph.merge".to_owned(), vec!["none".to_owned()])],
        mergewright_ui::keymap::KeySwitches::default(),
    );
    assert_eq!(warnings, []);
    let unbound = Arc::new(map);
    no_bar_draws(merge, &unbound);

    let base = sized_to_draw(on_a_branch_row());
    let moved = AppState {
        keymap: unbound.clone(),
        ..base.clone()
    };
    let shipped_menu = menu_labels(&update(&base, key(KeyCode::Char('.'))).0);
    let unbound_menu = menu_labels(&update(&moved, key(KeyCode::Char('.'))).0);
    assert!(
        shipped_menu.contains(&"Merge feature into main (m)".to_owned()),
        "{shipped_menu:?}"
    );
    assert!(
        unbound_menu.contains(&"Merge feature into main".to_owned()),
        "{unbound_menu:?}"
    );
    // m8/09c (Unbound ruling): the merge preview opened from that row draws
    // no `m merge` part and no `—`; its confirm is `confirm.yes`'s `y`.
    let row = unbound_menu
        .iter()
        .position(|label| label == "Merge feature into main")
        .expect("the merge row");
    let box_keys = |graph: &AppState| keys_of_the_box_opened_from_menu_row(graph, row);
    let unbound_keys = box_keys(&moved);
    assert!(!unbound_keys.contains("m merge"), "{unbound_keys}");
    assert!(!unbound_keys.contains('—'), "{unbound_keys}");
    // The ticket says `y merge`; a `.`-opened preview has no verb slot
    // (`preview/keys.rs`, `opened_by` "."), so its confirm reads
    // `y confirm` under the shipped keymap too (09c NOTES).
    assert!(unbound_keys.contains("y confirm"), "{unbound_keys}");
    assert_eq!(unbound_keys, box_keys(&base));

    for &ctx in ContextId::ALL {
        let Some(state) = fixture(ctx) else {
            continue;
        };
        let moved = AppState {
            keymap: unbound.clone(),
            ..state.clone()
        };
        let dashes = |state: &AppState| {
            crate::view::tests::draw(state, state.size.columns, state.size.rows)
                .matches('—')
                .count()
        };
        assert_eq!(
            dashes(&moved),
            dashes(&state),
            "{ctx:?}: a dash for an unbound key"
        );
    }
}

/// Ticket 10c, re-pointed by 09c (Unbound ruling): `r rebase` in an
/// `r`-opened preview is `preview.confirm_rebase`, whose chords follow
/// `graph.rebase`'s. With `graph.rebase = "none"` the part goes, no `—` is
/// drawn, and `r` does what an unanswered key does. A line on
/// `preview.confirm_rebase` itself is `Follows`, not applied: the box still
/// names `r rebase`, and `r` still confirms.
#[test]
fn the_preview_names_its_again_key_exactly_while_it_confirms() {
    let base = sized_to_draw(fixture(ContextId::Preview).expect("a preview"));
    // The box's bottom border, where its keys are (`after rebase` in the
    // ghost rows reads `r rebase` too).
    let footer = |state: &AppState| {
        crate::view::tests::draw(state, state.size.columns, state.size.rows)
            .lines()
            .find(|line| line.contains('╰'))
            .expect("the box's bottom border")
            .to_owned()
    };
    assert!(footer(&base).contains("r rebase"), "{}", footer(&base));
    let press_r = KeyPress::plain(KeyCode::Char('r'));
    assert_ne!(
        seen(&base, press_r),
        seen(&base, unanswered()),
        "r confirms under the default keymap"
    );

    let (map, warnings) = Keymap::with_overrides(
        &[("graph.rebase".to_owned(), vec!["none".to_owned()])],
        mergewright_ui::keymap::KeySwitches::default(),
    );
    assert_eq!(warnings, []);
    let rebase_unbound = AppState {
        keymap: Arc::new(map),
        ..base.clone()
    };
    let drawn = footer(&rebase_unbound);
    assert!(!drawn.contains("r rebase"), "{drawn}");
    assert!(!drawn.contains('—'), "{drawn}");
    assert!(drawn.contains("y confirm"), "{drawn}");
    assert_eq!(
        seen(&rebase_unbound, press_r),
        seen(&rebase_unbound, unanswered()),
        "r no longer confirms with graph.rebase unbound"
    );

    let (map, warnings) = Keymap::with_overrides(
        &[("preview.confirm_rebase".to_owned(), vec!["y".to_owned()])],
        mergewright_ui::keymap::KeySwitches::default(),
    );
    assert_eq!(
        warnings,
        [KeyWarning::Follows {
            action: ActionId("preview.confirm_rebase"),
            leader: ActionId("graph.rebase"),
        }]
    );
    let follows = AppState {
        keymap: Arc::new(map),
        ..base.clone()
    };
    assert!(
        footer(&follows).contains("r rebase"),
        "{}",
        footer(&follows)
    );
    assert_eq!(
        seen(&follows, press_r),
        seen(&base, press_r),
        "r still confirms"
    );
}
