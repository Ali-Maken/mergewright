//! His `[keys]` lines through startup (`config::load_from`, `keymap_from`,
//! `seed_keys`): the problems they say, typed text never remapped, and the
//! `M merge` capture (m8/09b, 10; split out by 09d).

use super::*;

/// Journey J8 from the file on disk: `suspend_on_ctrl_z` beside a
/// `[keys.graph]` table, read by `config::load_from`, made a keymap by
/// `keymap_from`, seeded as startup seeds it. (The file is written by
/// `config::tests`: `update` names no file system, tests included.)
/// `M` was the Map's: merge takes it, the Map loses it — the one problem.
#[test]
fn j8_a_keys_table_beside_suspend_moves_merge_and_says_one_problem() {
    let loaded = crate::config::tests::loaded_from_a_file(
        "[keys]\nsuspend_on_ctrl_z = true\n\n[keys.graph]\nmerge = \"M\"\n",
    );
    assert!(
        loaded.config.keys.suspend_on_ctrl_z,
        "suspend still reads true"
    );
    assert_eq!(
        loaded.warnings,
        [],
        "no unknown-key warning for `keys.graph` or its line"
    );
    let (keymap, problems) = crate::config::keymap_from(&loaded);
    assert_eq!(problems.len(), 1, "{problems:?}");

    let mut base = sized_to_draw(on_a_branch_row());
    base.seed_keys(Arc::new(keymap), problems);
    let first = crate::view::tests::draw_pinned(&base, base.size.columns, base.size.rows);
    assert!(
        first.contains("config: 1 problem · ? shows them"),
        "the bottom row says so:\n{first}"
    );
    // `docs/shots/m8-keymap-config-problem-120x36.txt`. What right looks
    // like: the graph as normal, the bottom row `config: 1 problem · ?
    // shows them` (muted). Colour unverified (rule 5).
    mergewright_fixtures::capture(
        format!(
            "{}/../../docs/shots/m8-keymap-config-problem-120x36.txt",
            env!("CARGO_MANIFEST_DIR")
        ),
        &first,
    );

    let (merged, _) = update(
        &base,
        AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char('M')))),
    );
    assert!(
        merged.preview.is_some(),
        "M on a branch row opens the merge preview"
    );
    assert_eq!(
        seen(&base, KeyPress::plain(KeyCode::Char('m'))),
        seen(&base, unanswered()),
        "m does what a key nobody answers does"
    );
}

/// The notice is there until the first key, and never with no problems.
#[test]
fn the_config_notice_goes_at_the_first_key_and_needs_a_problem() {
    let mut state = on_a_branch_row();
    state.seed_keys(
        Arc::default(),
        vec!["graf.merge is not an action".to_owned()],
    );
    let draw = |state: &AppState| crate::view::tests::draw(state, 120, 36);
    assert!(draw(&state).contains("config: 1 problem · ? shows them"));
    let after = update(&state, AppEvent::Core(Event::Key(unanswered()))).0;
    assert!(!draw(&after).contains("config:"), "gone at the first key");

    let mut clean = on_a_branch_row();
    clean.seed_keys(Arc::default(), Vec::new());
    assert!(
        !draw(&clean).contains("config:"),
        "nothing to report, nothing said"
    );
}

/// Minor 2 (09d): a line naming an action with a terminal escape in it is
/// drawn sanitised in F2 — the reviewer's probe, read from the file as
/// startup reads it. The escape shows as `␛`, never reaching the terminal.
#[test]
fn f2_draws_an_escape_in_his_action_name_as_a_symbol() {
    let loaded =
        crate::config::tests::loaded_from_a_file("[keys.graph]\n\"\\u001b[31mmerge\" = \"x\"\n");
    assert_eq!(loaded.warnings, []);
    let (keymap, problems) = crate::config::keymap_from(&loaded);
    assert_eq!(problems, ["graph.\u{1b}[31mmerge is not an action"]);
    let mut state = sized_to_draw(on_a_branch_row());
    state.seed_keys(Arc::new(keymap), problems);
    let f2 = KeyPress::plain(KeyCode::Function(2));
    let (shown, _) = update(&state, AppEvent::Core(Event::Key(f2)));
    let frame = crate::view::tests::draw(&shown, shown.size.columns, shown.size.rows);
    assert!(
        frame.contains("graph.␛[31mmerge is not an action"),
        "F2 names it, sanitised:\n{frame}"
    );
    assert!(!frame.contains('\u{1b}'), "no raw escape drawn");
}

/// ADR-0055 §2.1: typed text is never remapped. With `graph.merge = "x"`,
/// `x` typed into each field is the letter `x` — the same press, the same
/// result, as under the default keymap — and the field shows one more `x`.
#[test]
fn x_typed_into_every_field_is_an_x_with_merge_on_x() {
    // `x` is `graph.compare`'s: merge keeps it, compare loses it — on the
    // graph `x` now merges, and still types an `x` everywhere text goes.
    let (map, warnings) = Keymap::with_overrides(
        &[("graph.merge".to_owned(), vec!["x".to_owned()])],
        mergewright_ui::keymap::KeySwitches::default(),
    );
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    let remapped = Arc::new(map);
    for (name, base) in every_field() {
        assert!(base.text_field_open(), "{name}: the field is open");
        let moved = AppState {
            keymap: remapped.clone(),
            ..base.clone()
        };
        let press = KeyPress::plain(KeyCode::Char('x'));
        assert_eq!(
            seen(&moved, press),
            seen(&base, press),
            "{name}: as under the default keymap"
        );
        let before = seen(&base, unanswered()).frame.matches('x').count();
        let after = seen(&moved, press).frame.matches('x').count();
        assert!(
            after > before,
            "{name}: the field shows the x ({before} → {after})"
        );
    }
}

/// Ruling R5 (ticket m8/10): `[keys.graph] merge = "M"` is one collision —
/// `M` was the Map's (02-keymap §2.3) — that keeps `graph.merge` and loses
/// `graph.map`, warned at startup, never fatal.
#[test]
fn merge_on_shift_m_is_one_collision_that_loses_the_map() {
    let loaded = crate::config::tests::loaded_from_a_file("[keys.graph]\nmerge = \"M\"\n");
    let (_, warnings) = Keymap::with_overrides(
        &loaded.config.keys.remaps,
        mergewright_ui::keymap::KeySwitches::default(),
    );
    assert!(
        matches!(
            warnings.as_slice(),
            [KeyWarning::Collision { line, places }]
                if *line == ActionId("graph.merge")
                    && places.len() == 1
                    && places[0].kept == ActionId("graph.merge")
                    && places[0].lost == ActionId("graph.map")
        ),
        "{warnings:?}"
    );
}

/// `[keys.graph] merge = "M"` from a config file, through the startup path
/// (`config::keymap_from`, which `event_loop::startup::Arrival::first`
/// calls, then `seed_keys` as `run_repo` does): the bar on a branch row
/// reads `M merge`, the status row says the collision, and the `.` menu's
/// row reads `Merge feature into main (M)`. `M` was `graph.map`'s, so the
/// map loses it and its `M map` hint is not drawn anywhere (R5: a colliding
/// remap shows him its cost).
#[test]
fn capture_merge_remapped_to_shift_m() {
    use mergewright_ui::graph_screen::{Divider, hints_for};
    let loaded = crate::config::tests::loaded_from_a_file("[keys.graph]\nmerge = \"M\"\n");
    let (keymap, problems) = crate::config::keymap_from(&loaded);
    assert_eq!(problems.len(), 1, "M was the map's: {problems:?}");
    let keymap = Arc::new(keymap);
    for context in every_context() {
        for hint in hints_for(&context, Divider::Shown, 200, &keymap) {
            assert!(
                !names(&hint, ActionId("graph.map")),
                "{context:?}: `{:?} {}` — the map lost M",
                hint.key_text(&keymap),
                hint.verb
            );
        }
    }

    let mut base = sized_to_draw(on_a_branch_row());
    base.seed_keys(keymap, problems);
    let draw = |state: &AppState| {
        crate::view::tests::draw_pinned(state, state.size.columns, state.size.rows)
    };
    // At startup the notice holds the bottom row, where the bar goes.
    let notice = draw(&base);
    assert!(
        notice.contains("config: 1 problem · ? shows them"),
        "the status row says the collision:\n{notice}"
    );
    let menu = update(&base, key(KeyCode::Char('.'))).0;
    assert!(
        menu_labels(&menu).contains(&"Merge feature into main (M)".to_owned()),
        "{:?}",
        menu_labels(&menu)
    );
    let menu_frame = draw(&menu);
    // The notice went at that first key; closing the menu shows the bar.
    let closed = update(&menu, key(KeyCode::Escape)).0;
    let bar = draw(&closed);
    assert!(
        bar.contains("M merge"),
        "the bar spells merge's chord:\n{bar}"
    );
    assert!(!bar.contains("M map"), "the map lost M:\n{bar}");
    assert!(
        !bar.contains("m merge"),
        "not the default it replaced:\n{bar}"
    );
    // `docs/shots/m8-keymap-remapped-120x36.txt`. What right looks like:
    // three whole frames. First, at startup: the status row
    // `config: 1 problem · ? shows them` (it holds the bar's row until the
    // first key). Second, the `.` menu on the branch row with
    // `Merge feature into main (M)`. Third, the menu closed: the bar reads
    // `M merge`, and no `M map` anywhere.
    // Colour unverified (rule 5).
    mergewright_fixtures::capture(
        format!(
            "{}/../../docs/shots/m8-keymap-remapped-120x36.txt",
            env!("CARGO_MANIFEST_DIR")
        ),
        format!("{notice}\n{menu_frame}\n{bar}"),
    );
}
