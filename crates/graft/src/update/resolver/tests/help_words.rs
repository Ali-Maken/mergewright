//! m8-polish-and-ship/45 B (review `docs/reviews/m8-usability.md` B): `?` in
//! the resolver names `o`/`t` and their bulk forms with the side names the
//! bar uses (ADR-0043 §4: one name per side, everywhere). In a rebase git's
//! `ours` is upstream, so the help must not say "Take ours".

use graft_core::event::KeyCode;
use graft_ui::keymap::ActionId;

use super::{WALK, last_card_in_a_rebase, press, resolver_card_on_screen};
use crate::update::AppState;
use crate::update::palette::rows;

/// The label `?`'s listing gives the row that runs `action`.
fn help_label(state: &AppState, action: &'static str) -> String {
    let (open, _) = press(state, KeyCode::Char('?'));
    let palette = open.palette.as_ref().expect("`?` opens the help");
    rows(&open, palette)
        .into_iter()
        .flat_map(|(_, rows)| rows)
        .find(|(row, _)| row.config == Some(ActionId(action)))
        .map_or_else(
            || panic!("`?` lists {action}"),
            |(row, _)| row.label.as_str().to_owned(),
        )
}

#[test]
fn help_in_a_rebase_names_the_sides_as_the_bar_does() {
    let state = last_card_in_a_rebase();
    let cases = [
        ("resolver.take_ours", "Take upstream"),
        ("resolver.take_theirs", "Take yours"),
        ("resolver.all_ours", "Upstream in every block"),
        ("resolver.all_theirs", "Yours in every block"),
        ("resolver.file_ours", "Upstream for the whole file, done"),
        ("resolver.file_theirs", "Yours for the whole file, done"),
    ];
    for (action, words) in cases {
        assert_eq!(help_label(&state, action), words, "{action}");
    }
}

/// A stash pop (the resolver open with no operation): `yours` / `stashed`,
/// as its bar says (`o yours  t stashed`).
#[test]
fn help_in_a_stash_pop_names_the_sides_as_the_bar_does() {
    let state = resolver_card_on_screen(WALK);
    assert_eq!(help_label(&state, "resolver.take_ours"), "Take yours");
    assert_eq!(help_label(&state, "resolver.take_theirs"), "Take stashed");
}

/// Ticket ease-pass/05r: `?` lists `←`/`→` (keymap §2.12), each with its
/// key and its words.
#[test]
fn help_lists_the_arrows_that_step_between_clashes() {
    let state = super::resolver_on_screen(WALK);
    let (open, _) = press(&state, KeyCode::Char('?'));
    let palette = open.palette.as_ref().expect("`?` opens the help");
    let listed: Vec<(String, Option<String>)> = rows(&open, palette)
        .into_iter()
        .flat_map(|(_, rows)| rows)
        .filter(|(row, _)| {
            matches!(
                row.config,
                Some(ActionId("resolver.step_prev" | "resolver.step_next"))
            )
        })
        .map(|(row, _)| {
            (
                row.label.as_str().to_owned(),
                row.key.as_ref().map(|key| key.0.clone()),
            )
        })
        .collect();
    let row = |label: &str, key: &str| (label.to_owned(), Some(key.to_owned()));
    assert_eq!(
        listed,
        [
            row(
                "Previous clash in this file, taken ones included",
                "\u{2190}"
            ),
            row("Next clash in this file, taken ones included", "\u{2192}"),
        ]
    );
}
