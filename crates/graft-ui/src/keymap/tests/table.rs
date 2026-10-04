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
            assert!(!b.default.is_empty(), "{} has no default", b.action.0);
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
        let want_first = if ctx == ContextId::Tour {
            ContextId::Global
        } else {
            ctx
        };
        assert_eq!(scope.first(), Some(&want_first), "{ctx:?}");
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
fn the_tour_and_the_menu_bind_nothing_of_their_own() {
    // §2.18: the tour's `Esc` is `app.back`; §2.1: the menu claims only
    // movement, `⏎` and `.`, all §2.2's.
    for ctx in [ContextId::Tour, ContextId::Menu] {
        assert!(defaults().iter().all(|b| b.context != ctx), "{ctx:?}");
    }
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
