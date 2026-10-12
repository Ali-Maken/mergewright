//! The table's shape: ids are his config's API, so they are guarded.

use std::collections::HashSet;

use super::super::{ContextId, KeyChord, defaults};

const GOLDEN: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/keymap_ids.txt"));

#[test]
fn action_ids_match_the_golden_list() {
    let mut ids: Vec<&str> = defaults().iter().map(|b| b.action.0).collect();
    ids.sort_unstable();
    let golden: Vec<&str> = GOLDEN.lines().filter(|l| !l.is_empty()).collect();
    let gone: Vec<&&str> = golden.iter().filter(|g| !ids.contains(g)).collect();
    let new: Vec<&&str> = ids.iter().filter(|i| !golden.contains(i)).collect();
    assert!(
        gone.is_empty() && new.is_empty() && ids == golden,
        "action ids differ from crates/graft-ui/tests/keymap_ids.txt.\n\
         missing from the table: {gone:?}\nnot in the golden list: {new:?}\n\
         A rename breaks his config — keep the old id as an alias (ADR-0055 §2). \
         An addition edits the golden file in the same commit (one id per line, sorted)."
    );
}

#[test]
fn ids_are_unique_well_formed_and_prefixed_by_their_context() {
    let mut seen = HashSet::new();
    for b in defaults() {
        let id = b.action.0;
        assert!(seen.insert(id), "{id} twice");
        let (prefix, verb) = id.split_once('.').unwrap_or_else(|| panic!("{id}: no `.`"));
        assert!(
            !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "{id}: prefix must match [a-z_]+"
        );
        assert!(
            !verb.is_empty()
                && verb
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "{id}: verb must match [a-z0-9_]+"
        );
        assert_eq!(prefix, b.context.config_name(), "{id}: prefix vs context");
        assert!(!b.label.is_empty(), "{id}: no label");
    }
}

#[test]
fn every_action_has_a_default_and_no_context_binds_one_chord_twice() {
    for &ctx in ContextId::ALL {
        let mut seen: Vec<(KeyChord, &str)> = Vec::new();
        for b in defaults().iter().filter(|b| b.context == ctx) {
            // `app.suspend` has a key only with `suspend_on_ctrl_z` (m8/09c, D2).
            if b.action.0 != "app.suspend" {
                assert!(!b.default.is_empty(), "{} has no default", b.action.0);
            }
            for c in b.default {
                if let Some((_, other)) = seen.iter().find(|(s, _)| s == c) {
                    panic!("{:?} on {} and {other}", c.label(), b.action.0);
                }
                seen.push((*c, b.action.0));
            }
        }
    }
}

#[test]
fn every_scope_starts_at_its_own_place_and_ends_at_global() {
    for &ctx in ContextId::ALL {
        let scope = ctx.scope();
        assert_eq!(scope.first(), Some(&ctx), "{ctx:?}");
        assert_eq!(scope.last(), Some(&ContextId::Global), "{ctx:?}");
        for inner in scope {
            assert!(
                !inner.is_text_field() || *inner == ctx,
                "{ctx:?} reaches a text field"
            );
        }
    }
}

#[test]
fn the_tour_binds_only_esc_to_leave_and_the_menu_nothing() {
    // §2.18: the tour claims no key but `Esc` (`tour.leave`, ticket 19
    // Amendment §A8); §2.1: the menu claims only movement, `⏎` and `.`,
    // all §2.2's.
    let tour: Vec<_> = defaults()
        .iter()
        .filter(|b| b.context == ContextId::Tour)
        .map(|b| (b.action.0, b.default))
        .collect();
    assert_eq!(
        tour,
        [(
            "tour.leave",
            &[KeyChord::plain(graft_core::event::KeyCode::Escape)][..]
        )]
    );
    assert!(defaults().iter().all(|b| b.context != ContextId::Menu));
}

#[test]
fn only_stash_has_hidden_search_words() {
    // §2.12 names one: `aside` finds stash.
    let with: Vec<&str> = defaults()
        .iter()
        .filter(|b| !b.search.is_empty())
        .map(|b| b.action.0)
        .collect();
    assert_eq!(with, ["graph.stash"]);
}

/// `?` and the man page say what `⏎` in the picker does on both kinds of
/// row: it switches, and on the branch he is on it goes back to the graph
/// (keymap §2.16, usability Q2; review m8 g1, minor 2).
#[test]
fn picker_switch_names_both_outcomes() {
    let label = defaults()
        .iter()
        .find(|b| b.action.0 == "picker.switch")
        .map(|b| b.label)
        .expect("picker.switch is in the table");
    assert!(label.starts_with("Switch to it"), "{label}");
    assert!(label.contains("back to the graph"), "{label}");
    assert!(label.contains("on your own branch"), "{label}");
}

/// `?` in J names both things `⏎` does: on the `← u`/`← U` row it undoes or
/// redoes at once, as the bar's `⏎ redo this` says; on an older row it opens
/// the put-back preview (keymap §2.11; usability walk m8, m6).
#[test]
fn journal_revert_names_the_at_once_rows_and_the_preview() {
    let label = defaults()
        .iter()
        .find(|b| b.action.0 == "journal.revert")
        .map(|b| b.label)
        .expect("journal.revert is in the table");
    assert!(label.starts_with("Undo / redo this row"), "{label}");
    assert!(
        label.contains("else put back to before it (preview)"),
        "{label}"
    );
}
