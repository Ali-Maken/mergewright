//! Ticket m8/09c's rules: a line acts where its section is live and never
//! over a place's own keys (D1), suspend is a row (D2), one confirm id (D3).

use super::super::{
    ActionId, Clash, ContextId, KeyChord, KeyCode, KeySwitches, KeyWarning, Keymap, Translated,
    defaults,
};
use super::line;

const OFF: KeySwitches = KeySwitches {
    suspend_on_ctrl_z: false,
};
const ON: KeySwitches = KeySwitches {
    suspend_on_ctrl_z: true,
};

fn with(lines: &[(String, Vec<String>)], switches: KeySwitches) -> (Keymap, Vec<KeyWarning>) {
    Keymap::with_overrides(lines, switches)
}

fn words(warnings: &[KeyWarning]) -> Vec<String> {
    warnings.iter().map(KeyWarning::words).collect()
}

const DOWN: ActionId = ActionId("app.down");

// ── D1 ──────────────────────────────────────────────────────────────────

/// Major 1: `[keys.graph] merge = "j"` merges on the graph only; every
/// place that answers `j` itself still moves, and the one problem names
/// the graph.
#[test]
fn merge_on_j_moves_everywhere_but_the_graph_and_is_one_clash() {
    let (map, warnings) = with(&[line("graph.merge", &["j"])], OFF);
    let j = KeyChord::key('j');
    let movers: Vec<ContextId> = ContextId::ALL
        .iter()
        .copied()
        .filter(|ctx| ctx.borrows().contains(&DOWN))
        .collect();
    assert_eq!(
        movers,
        [
            ContextId::Map,
            ContextId::Changes,
            ContextId::Diff,
            ContextId::Blame,
            ContextId::Stash,
            ContextId::Journal,
            ContextId::History,
            ContextId::Menu,
        ]
    );
    for ctx in movers {
        assert_eq!(map.translate(ctx, j), Translated::Pass, "{ctx:?}");
    }
    assert_eq!(
        map.translate(ContextId::Graph, j),
        Translated::Default(KeyChord::key('m'))
    );
    assert_eq!(
        warnings,
        [KeyWarning::Collision {
            line: ActionId("graph.merge"),
            places: vec![Clash {
                key: j,
                context: ContextId::Graph,
                kept: ActionId("graph.merge"),
                lost: DOWN,
                keeps: Some(KeyChord::plain(KeyCode::Down)),
            }],
        }]
    );
    assert_eq!(
        words(&warnings),
        ["j given to graph.merge — move down keeps ↓ on the graph"]
    );
}

/// The rows a place falls through to: one movable row of each context
/// after it in its scope.
fn one_row_of(ctx: ContextId) -> Option<ActionId> {
    defaults()
        .iter()
        .find(|b| {
            b.context == ctx
                && !matches!(
                    b.action.0,
                    "app.quit_now"
                        | "app.suspend"
                        | "preview.confirm_merge"
                        | "preview.confirm_rebase"
                )
        })
        .map(|b| b.action)
}

/// Cross-remap: in each place that borrows, every borrowed row keeps its
/// first chord even after a row of each place it falls through to is moved
/// onto that chord.
#[test]
fn a_borrowed_row_keeps_its_key_over_every_place_it_falls_through_to() {
    let mut checked = 0;
    for &ctx in ContextId::ALL {
        for borrowed in ctx.borrows() {
            let b = defaults()
                .iter()
                .find(|b| b.action == *borrowed)
                .expect("a borrowed row is in the table");
            let key = b.default[0];
            for inner in ctx.scope().iter().filter(|inner| **inner != ctx) {
                let Some(other) = one_row_of(*inner).filter(|other| other != borrowed) else {
                    continue;
                };
                let (map, _) = with(&[line(other.0, &[&key.label().0])], OFF);
                assert_eq!(
                    map.translate(ctx, key),
                    Translated::Pass,
                    "{ctx:?}: {} onto {} took {}",
                    other.0,
                    key.label().0,
                    borrowed.0
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 60, "checked only {checked}");
}

/// `reachable` is the resolution order: own rows, borrows, the rest of the
/// scope; each row once.
#[test]
fn reachable_lists_own_rows_then_borrows_then_the_scope() {
    let map = Keymap::default();
    let ids: Vec<&str> = map
        .reachable(ContextId::Menu)
        .iter()
        .map(|(b, _)| b.action.0)
        .collect();
    assert_eq!(
        ids[..5],
        ["app.down", "app.up", "app.open", "app.back", "app.menu"]
    );
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len(), "a row twice");
    let preview: Vec<&str> = map
        .reachable(ContextId::Preview)
        .iter()
        .map(|(b, _)| b.action.0)
        .collect();
    let yes = preview.iter().position(|id| *id == "confirm.yes");
    let own = preview.iter().position(|id| *id == "preview.cycle");
    let undo = preview.iter().position(|id| *id == "app.undo");
    assert!(own < yes && yes < undo, "{preview:?}");
}

// ── D2 ──────────────────────────────────────────────────────────────────

/// Switch on alone: `⌃Z` is suspend, undo is spelled `u`; switch off: as
/// shipped.
#[test]
fn the_switch_moves_ctrl_z_from_undo_to_suspend() {
    let ctrl_z = KeyChord::ctrl('z');
    let (on, warnings) = with(&[], ON);
    assert_eq!(warnings, []);
    assert_eq!(on.spell(&ActionId("app.undo")).0, "u");
    assert_eq!(on.chords(&ActionId("app.suspend")), [ctrl_z]);
    assert_eq!(on.translate(ContextId::Graph, ctrl_z), Translated::Pass);
    let off = Keymap::default();
    assert_eq!(off.spell(&ActionId("app.undo")).0, "u");
    assert_eq!(
        off.chords(&ActionId("app.undo")),
        [KeyChord::key('u'), ctrl_z]
    );
    assert_eq!(off.spell(&ActionId("app.suspend")).0, "—");
}

#[test]
fn switch_on_and_undo_on_u_is_no_problem() {
    let (map, warnings) = with(&[line("app.undo", &["u"])], ON);
    assert_eq!(warnings, []);
    assert_eq!(
        map.translate(ContextId::Graph, KeyChord::ctrl('z')),
        Translated::Pass
    );
}

#[test]
fn suspend_moved_to_ctrl_s_leaves_ctrl_z_unanswered() {
    let (map, warnings) = with(&[line("app.suspend", &["ctrl+s"])], ON);
    assert_eq!(warnings, []);
    assert_eq!(
        map.translate(ContextId::Graph, KeyChord::ctrl('s')),
        Translated::Default(KeyChord::ctrl('z'))
    );
    assert_eq!(
        map.translate(ContextId::Graph, KeyChord::ctrl('z')),
        Translated::Unbound
    );
}

#[test]
fn a_suspend_line_with_the_switch_off_is_not_applied() {
    let (map, warnings) = with(&[line("app.suspend", &["ctrl+s"])], OFF);
    assert_eq!(
        warnings,
        [KeyWarning::SwitchOff {
            action: ActionId("app.suspend"),
            switch: "suspend_on_ctrl_z",
        }]
    );
    assert_eq!(
        words(&warnings),
        ["app.suspend does nothing while suspend_on_ctrl_z is off"]
    );
    assert_eq!(map.chords(&ActionId("app.suspend")), []);
}

#[test]
fn a_graph_verb_on_ctrl_z_takes_suspend_on_the_graph() {
    let (map, warnings) = with(&[line("graph.fetch", &["ctrl+z"])], ON);
    assert_eq!(
        words(&warnings),
        ["⌃Z given to graph.fetch — suspend has no key on the graph"]
    );
    assert_eq!(
        map.translate(ContextId::Graph, KeyChord::ctrl('z')),
        Translated::Default(KeyChord::key('f'))
    );
}

// ── D3 ──────────────────────────────────────────────────────────────────

const CONFIRMERS: [ContextId; 4] = [
    ContextId::Preview,
    ContextId::Confirm,
    ContextId::Finish,
    ContextId::Rewrite,
];

/// `[keys.confirm] yes = "c"`: one line moves the confirm of every box;
/// `y` is unanswered there and still copies the SHA on the graph. The
/// start screen's init box is a `Confirm` box (09d D6), so the start
/// screen's own `c` (clone) keeps `c` with no clash to warn of.
#[test]
fn one_confirm_line_moves_every_confirm() {
    let (map, warnings) = with(&[line("confirm.yes", &["c"])], OFF);
    let c = KeyChord::key('c');
    let y = KeyChord::key('y');
    for ctx in &CONFIRMERS {
        assert_eq!(map.translate(*ctx, c), Translated::Default(y), "{ctx:?}");
        assert_eq!(map.translate(*ctx, y), Translated::Unbound, "{ctx:?}");
    }
    assert_eq!(map.translate(ContextId::Graph, y), Translated::Pass);
    assert_eq!(map.translate(ContextId::EmptyState, c), Translated::Pass);
    assert_eq!(warnings, []);
    assert_eq!(map.spell(&ActionId("confirm.yes")).0, "c");
}

/// 09d D6: the start screen borrows no confirm — its init box translates
/// in `Confirm` — so `confirm.yes = "c"` is no problem there: `c` stays
/// clone on the start screen and nothing is warned.
#[test]
fn confirm_on_c_is_no_problem_on_the_start_screen() {
    assert!(
        !ContextId::EmptyState
            .borrows()
            .contains(&ActionId("confirm.yes"))
    );
    let (map, warnings) = with(&[line("confirm.yes", &["c"])], OFF);
    assert_eq!(warnings, []);
    assert_eq!(
        map.translate(ContextId::EmptyState, KeyChord::key('c')),
        Translated::Pass
    );
}

/// The repeat key follows the opener: `merge = "M"` → `M` confirms the
/// merge preview; `merge = "none"` → nothing repeats it.
#[test]
fn the_repeat_key_follows_the_opener() {
    let (map, _) = with(&[line("graph.merge", &["M"])], OFF);
    assert_eq!(
        map.translate(ContextId::Preview, KeyChord::key('M')),
        Translated::Default(KeyChord::key('m'))
    );
    assert_eq!(map.spell(&ActionId("preview.confirm_merge")).0, "M");
    let (map, warnings) = with(&[line("graph.merge", &["none"])], OFF);
    assert_eq!(warnings, []);
    assert_eq!(map.spell(&ActionId("preview.confirm_merge")).0, "—");
    assert_eq!(
        map.translate(ContextId::Preview, KeyChord::key('m')),
        Translated::Unbound
    );
}

#[test]
fn a_line_on_a_follower_is_not_applied() {
    let (map, warnings) = with(&[line("preview.confirm_merge", &["x"])], OFF);
    assert_eq!(
        warnings,
        [KeyWarning::Follows {
            action: ActionId("preview.confirm_merge"),
            leader: ActionId("graph.merge"),
        }]
    );
    assert_eq!(
        words(&warnings),
        ["preview.confirm_merge can't be remapped — it is graph.merge's key, so move that"]
    );
    assert_eq!(map.spell(&ActionId("preview.confirm_merge")).0, "m");
}

/// m8/09e: `empty_state.cancel` is gone — the init box is a confirm box,
/// `esc` cancels (§2.3c), and no other row names `n` there. Pre-release,
/// no alias: a line naming it is an unknown action.
#[test]
fn empty_state_cancel_is_no_action() {
    let (_, warnings) = with(&[line("empty_state.cancel", &["x"])], OFF);
    assert_eq!(
        warnings,
        [KeyWarning::UnknownAction("empty_state.cancel".into())]
    );
}
