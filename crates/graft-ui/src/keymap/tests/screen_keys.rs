//! The ease pass's vocabulary (ticket ease-pass/03, ADR-0063): the commit
//! options' keyless rows, and every key the mocks reuse where it was.
//!
//! `graph.colours_off` (`#`) is decided but not in the table yet: `graft`'s
//! dead-key sweeps (`update/tests/keymap_edge/sweep.rs`,
//! `update/tests/palette_live.rs`) refuse a row nothing answers, so it lands
//! with its handler (G). `resolver.review` (`y`) landed with its handler
//! (ease-pass/05b).

use graft_core::event::KeyCode;

use super::super::{ActionId, ContextId, KeyChord, Keymap};

fn acting(ctx: ContextId, chord: KeyChord) -> Option<&'static str> {
    Keymap::shipped().acting(ctx, chord).map(|a| a.0)
}

/// On the clash screen `c`, `n`, `esc` and `u` keep their ids (the mock
/// changes their words, not their rows), and `y` is the review's
/// (ease-pass/05b).
#[test]
fn the_clash_screen_keys_resolve_to_their_ids() {
    let at = |chord| acting(ContextId::Resolver, chord);
    assert_eq!(at(KeyChord::key('c')), Some("resolver.finish"));
    assert_eq!(at(KeyChord::key('n')), Some("resolver.next_block"));
    assert_eq!(at(KeyChord::plain(KeyCode::Escape)), Some("app.back"));
    assert_eq!(at(KeyChord::key('u')), Some("app.undo"));
    assert_eq!(at(KeyChord::key('y')), Some("resolver.review"));
}

/// `←`/`→` and `h`/`l` step between the clashes of the file on screen
/// (ticket ease-pass/05r, review ease-pass-05 M1): §2.2's movement keys,
/// the arrows spelled first so the bar reads `←/→ clashes` (Q19 a).
#[test]
fn the_arrows_step_between_clashes_in_the_resolver() {
    let at = |chord| acting(ContextId::Resolver, chord);
    assert_eq!(
        at(KeyChord::plain(KeyCode::Left)),
        Some("resolver.step_prev")
    );
    assert_eq!(at(KeyChord::key('h')), Some("resolver.step_prev"));
    assert_eq!(
        at(KeyChord::plain(KeyCode::Right)),
        Some("resolver.step_next")
    );
    assert_eq!(at(KeyChord::key('l')), Some("resolver.step_next"));
    let keys = Keymap::shipped();
    assert_eq!(
        keys.spell_keys(&[
            ActionId("resolver.step_prev"),
            ActionId("resolver.step_next")
        ]),
        Some("←/→".to_owned())
    );
}

/// `#` is unbound on the graph and everything that falls through to it.
#[test]
fn hash_is_free_on_the_graph() {
    for ctx in [ContextId::Graph, ContextId::Changes, ContextId::Diff] {
        assert_eq!(acting(ctx, KeyChord::key('#')), None, "{ctx:?}");
    }
}

/// The review's `⏎` and `space` still show the file (`finish.toggle_diff`).
#[test]
fn the_review_look_keys_are_still_toggle_diff() {
    for chord in [KeyChord::plain(KeyCode::Enter), KeyChord::key(' ')] {
        assert_eq!(acting(ContextId::Finish, chord), Some("finish.toggle_diff"));
    }
}

/// `C` and `x` on the graph are unchanged (Q1 a, Q2 a).
#[test]
fn switch_and_compare_keep_their_keys() {
    assert_eq!(
        acting(ContextId::Graph, KeyChord::key('C')),
        Some("graph.switch")
    );
    assert_eq!(
        acting(ContextId::Graph, KeyChord::key('x')),
        Some("graph.compare")
    );
}

const TOGGLES: [&str; 3] = [
    "commit.toggle_amend",
    "commit.toggle_no_verify",
    "commit.toggle_allow_empty",
];

/// The three commit options have no key (Q8 a): rows of the commit
/// screen's place, spelled `—`; `⌃S` still signs off.
#[test]
fn commit_options_are_keyless_rows_of_the_commit_screen() {
    let keys = Keymap::shipped();
    let own: Vec<&str> = keys
        .in_context(ContextId::CommitEditor)
        .into_iter()
        .map(|(b, _)| b.action.0)
        .collect();
    for id in TOGGLES {
        assert!(own.contains(&id), "{id} not a CommitEditor row: {own:?}");
        assert!(keys.chords(&ActionId(id)).is_empty(), "{id} has a key");
        assert_eq!(keys.spell(&ActionId(id)).0, "—", "{id}");
    }
    assert_eq!(
        acting(ContextId::CommitEditor, KeyChord::ctrl('s')),
        Some("commit.sign_off")
    );
}

/// No new place: the incoming list, the look and the commit screen's files
/// are states of places that exist.
#[test]
fn no_new_context() {
    assert_eq!(ContextId::ALL.len(), 22);
}
