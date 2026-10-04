//! `Keymap::translate`: the edge every key press crosses (ADR-0055 §3).

use super::super::{
    ActionId, Binding, Clash, ContextId, KeyChord, KeyCode, KeyWarning, Keymap, Translated,
    defaults,
};
use super::{free_chords, line};

/// The row that answers `chord` in `ctx` out of the box: its own rows,
/// then the rows it borrows, then the first context of its scope holding
/// it (the table never binds one chord twice in a context). 09d replaces
/// this mirror with a behavioural oracle.
fn default_holder(ctx: ContextId, chord: KeyChord) -> Option<&'static Binding> {
    let own = || {
        defaults()
            .iter()
            .find(|b| b.context == ctx && b.default.contains(&chord))
    };
    let borrowed = || {
        defaults()
            .iter()
            .find(|b| ctx.borrows().contains(&b.action) && b.default.contains(&chord))
    };
    own().or_else(borrowed).or_else(|| {
        ctx.scope().iter().find_map(|inner| {
            defaults()
                .iter()
                .find(|b| b.context == *inner && b.default.contains(&chord))
        })
    })
}

fn in_scope(ctx: ContextId) -> impl Iterator<Item = &'static Binding> {
    defaults()
        .iter()
        .filter(move |b| ctx.scope().contains(&b.context) || ctx.borrows().contains(&b.action))
}

/// Every row a line may move: not a text field's, not `⌃C`, not one that
/// follows another's keys, not one a switch keeps off.
fn movable() -> Vec<&'static Binding> {
    defaults()
        .iter()
        .filter(|b| {
            !b.context.is_text_field()
                && !matches!(
                    b.action.0,
                    "app.quit_now"
                        | "app.suspend"
                        | "preview.confirm_merge"
                        | "preview.confirm_rebase"
                )
        })
        .collect()
}

/// The sweep's keymap: every movable action on its own free chord. Returns
/// the map and, per movable row, the chord it moved to.
fn swept() -> (Keymap, Vec<(&'static Binding, KeyChord)>) {
    let rows = movable();
    let free = free_chords();
    assert!(
        free.len() >= rows.len(),
        "{} free for {}",
        free.len(),
        rows.len()
    );
    for b in defaults() {
        for c in b.default {
            assert!(!free.contains(c), "{} uses a sweep chord", b.action.0);
        }
    }
    let moved: Vec<(&'static Binding, KeyChord)> = rows.into_iter().zip(free).collect();
    let lines: Vec<(String, Vec<String>)> = moved
        .iter()
        .map(|(b, c)| (b.action.0.to_string(), vec![c.label().0]))
        .collect();
    let (map, warnings) = Keymap::with_overrides(&lines, crate::keymap::KeySwitches::default());
    assert_eq!(warnings, [], "every chord is free, so nothing collides");
    (map, moved)
}

/// One non-text context of the sweep: each action's new chord acts as the
/// default the dispatcher answers there, and no default is live. Returns
/// how many actions it checked.
fn sweep_context(map: &Keymap, ctx: ContextId, moved: &[(&Binding, KeyChord)]) -> usize {
    let scope = ctx.scope();
    let mut checked = 0;
    let reaches = |b: &Binding| scope.contains(&b.context) || ctx.borrows().contains(&b.action);
    for (b, to) in moved.iter().filter(|(b, _)| reaches(b)) {
        let acts = b
            .default
            .iter()
            .copied()
            .find(|d| default_holder(ctx, *d).map(|h| h.action) == Some(b.action));
        let want = acts.map_or(Translated::Unbound, Translated::Default);
        assert_eq!(map.translate(ctx, *to), want, "{ctx:?}: {}", b.action.0);
        for d in b.default {
            let got = map.translate(ctx, *d);
            assert_eq!(
                got,
                Translated::Unbound,
                "{ctx:?}: {} still live",
                d.label().0
            );
        }
        checked += 1;
    }
    checked
}

#[test]
fn the_sweep_moves_every_action_and_every_context_follows() {
    let (map, moved) = swept();
    let mut checked = 0;
    for &ctx in ContextId::ALL {
        if ctx.is_text_field() {
            // Typed keys stay as they are, remapped or not.
            for c in in_scope(ctx).flat_map(|b| b.default) {
                assert_eq!(map.translate(ctx, *c), Translated::Pass, "{ctx:?} {c:?}");
            }
        } else {
            checked += sweep_context(&map, ctx, &moved);
        }
        let quit = map.translate(ctx, KeyChord::QUIT_NOW);
        assert_eq!(quit, Translated::Pass, "{ctx:?}");
    }
    assert!(checked > 200, "the sweep walked only {checked} pairs");
}

#[test]
fn the_default_keymap_passes_every_default_everywhere() {
    let map = Keymap::default();
    for &ctx in ContextId::ALL {
        for b in in_scope(ctx) {
            for c in b.default {
                assert_eq!(map.translate(ctx, *c), Translated::Pass, "{ctx:?} {c:?}");
            }
        }
    }
}

#[test]
fn j8_merge_on_capital_m_is_one_problem_and_acts_as_m() {
    let (map, warnings) = Keymap::with_overrides(
        &[line("graph.merge", &["M"])],
        crate::keymap::KeySwitches::default(),
    );
    assert_eq!(
        warnings,
        [KeyWarning::Collision {
            line: ActionId("graph.merge"),
            places: vec![Clash {
                key: KeyChord::key('M'),
                context: ContextId::Graph,
                kept: ActionId("graph.merge"),
                lost: ActionId("graph.map"),
                keeps: None,
            }],
        }]
    );
    let m = KeyChord::key('m');
    let cap_m = KeyChord::key('M');
    assert_eq!(
        map.translate(ContextId::Graph, cap_m),
        Translated::Default(m)
    );
    assert_eq!(map.translate(ContextId::Graph, m), Translated::Unbound);
    // The changes pane's mergetool keeps `M`; the map view's close too.
    assert_eq!(map.translate(ContextId::Changes, cap_m), Translated::Pass);
    assert_eq!(map.translate(ContextId::Map, cap_m), Translated::Pass);
}

#[test]
fn an_override_replaces_the_whole_default_list() {
    let (map, _) = Keymap::with_overrides(
        &[line("app.down", &["n"])],
        crate::keymap::KeySwitches::default(),
    );
    let down = KeyChord::plain(KeyCode::Down);
    let n = KeyChord::key('n');
    assert_eq!(map.translate(ContextId::Graph, down), Translated::Unbound);
    assert_eq!(
        map.translate(ContextId::Graph, n),
        Translated::Default(KeyChord::key('j'))
    );
    assert_eq!(map.chords(&ActionId("app.down")), [n]);

    let (kept, _) = Keymap::with_overrides(
        &[line("app.down", &["n", "down"])],
        crate::keymap::KeySwitches::default(),
    );
    assert_eq!(kept.translate(ContextId::Graph, down), Translated::Pass);
}

#[test]
fn none_alone_unbinds_and_spells_a_dash() {
    let (map, warnings) = Keymap::with_overrides(
        &[line("graph.push", &["none"])],
        crate::keymap::KeySwitches::default(),
    );
    assert_eq!(warnings, []);
    assert_eq!(
        map.translate(ContextId::Graph, KeyChord::key('P')),
        Translated::Unbound
    );
    assert_eq!(map.spell(&ActionId("graph.push")).0, "—");
    assert_eq!(map.chords(&ActionId("graph.push")), []);
}

#[test]
fn an_inner_default_keeps_a_chord_a_global_override_asked_for() {
    let (map, _) = Keymap::with_overrides(
        &[line("app.undo", &["o"])],
        crate::keymap::KeySwitches::default(),
    );
    let o = KeyChord::key('o');
    assert_eq!(map.translate(ContextId::Resolver, o), Translated::Pass);
    assert_eq!(map.translate(ContextId::Graph, o), Translated::Pass);
    assert_eq!(
        map.translate(ContextId::Banner, o),
        Translated::Default(KeyChord::key('u'))
    );
    assert_eq!(
        map.translate(ContextId::Graph, KeyChord::key('u')),
        Translated::Unbound
    );
}

#[test]
fn a_shadowed_action_has_no_default_to_act_as() {
    // `diff.blame` moved: in the graph its default `B` is the commit-graph
    // offer's, so the moved key has nothing to become there.
    let (map, _) = Keymap::with_overrides(
        &[line("diff.blame", &["Z"])],
        crate::keymap::KeySwitches::default(),
    );
    let z = KeyChord::key('Z');
    assert_eq!(map.translate(ContextId::Graph, z), Translated::Unbound);
    assert_eq!(
        map.translate(ContextId::Diff, z),
        Translated::Default(KeyChord::key('B'))
    );
}

#[test]
fn a_text_field_passes_every_key_even_a_remapped_one() {
    let (map, _) = Keymap::with_overrides(
        &[line("app.back", &["x"])],
        crate::keymap::KeySwitches::default(),
    );
    for ctx in [
        ContextId::Picker,
        ContextId::Palette,
        ContextId::CommitEditor,
        ContextId::Credential,
    ] {
        assert_eq!(map.translate(ctx, KeyChord::key('x')), Translated::Pass);
        assert_eq!(
            map.translate(ctx, KeyChord::plain(KeyCode::Escape)),
            Translated::Pass
        );
    }
}

#[test]
fn spell_and_chords_follow_the_override() {
    let (map, _) = Keymap::with_overrides(
        &[line("graph.merge", &["ctrl+m", "M"])],
        crate::keymap::KeySwitches::default(),
    );
    let merge = ActionId("graph.merge");
    assert_eq!(map.spell(&merge).0, "⌃M");
    assert_eq!(
        map.chords(&merge),
        [KeyChord::ctrl('m'), KeyChord::key('M')]
    );
    assert_eq!(Keymap::default().spell(&merge).0, "m");
    assert_eq!(Keymap::default().spell(&ActionId("app.undo")).0, "u");
    assert_eq!(Keymap::default().spell(&ActionId("no.such")).0, "—");
}

#[test]
fn in_context_lists_a_place_own_rows_in_table_order() {
    let (map, _) = Keymap::with_overrides(
        &[line("map.seek", &["s"])],
        crate::keymap::KeySwitches::default(),
    );
    let rows = map.in_context(ContextId::Map);
    let ids: Vec<&str> = rows.iter().map(|(b, _)| b.action.0).collect();
    assert_eq!(ids, ["map.close", "map.seek"]);
    assert_eq!(rows[1].1, [KeyChord::key('s')]);
    assert!(map.in_context(ContextId::Tour).is_empty());
}
