//! Ticket m8/09c's rules, pinned through `update` (and `config::load_from`
//! where the line comes from his file): a line acts where its section is
//! live and never over a place's own keys (D1); suspend is a row (D2); one
//! confirm id (D3); a text field passes keys untranslated and draws only
//! fixed keys (R4); the repeat key follows the opener (Unbound).
//!
//! Fixtures and the [`Seen`] projection are `keymap_edge`'s.

use std::sync::Arc;

use graft_ui::finish_summary::SummaryFocus;
use graft_ui::keymap::{ActionId, ContextId, KeySwitches, Keymap};

use super::keymap_edge::{
    Seen, every_field, fixture, on_a_branch_row, seen, seen_as, sized_to_draw, swept, unanswered,
    walk,
};
use super::*;

/// `state` with `keys` in place of its keymap.
fn with_keys(state: &AppState, keys: &Arc<Keymap>) -> AppState {
    AppState {
        keymap: Arc::clone(keys),
        ..state.clone()
    }
}

/// His `[keys]` lines, read as startup reads them: the file through
/// `config::load_from`, then `keymap_from`.
fn from_file(toml: &str) -> (Arc<Keymap>, Vec<String>) {
    let loaded = crate::config::tests::loaded_from_a_file(toml);
    assert_eq!(loaded.warnings, [], "{toml}");
    let (keymap, problems) = crate::config::keymap_from(&loaded);
    (Arc::new(keymap), problems)
}

fn lines(id: &str, key: &str) -> Arc<Keymap> {
    let (map, _) = Keymap::with_overrides(
        &[(id.to_owned(), vec![key.to_owned()])],
        KeySwitches::default(),
    );
    Arc::new(map)
}

fn press(c: char) -> KeyPress {
    KeyPress::plain(KeyCode::Char(c))
}

// ── Major 1 (D1) ────────────────────────────────────────────────────────

/// The places that answer `j` themselves (`ContextId::borrows`).
const MOVERS: [ContextId; 8] = [
    ContextId::Changes,
    ContextId::Diff,
    ContextId::Menu,
    ContextId::Map,
    ContextId::Journal,
    ContextId::Stash,
    ContextId::Blame,
    ContextId::History,
];

/// The review's missing test: after `[keys.graph] merge = "j"`, `j` in
/// each place that answers it does exactly what it does under the shipped
/// keymap — it moves, never merges. (Map, Journal and Stash fixtures have
/// nothing to move over — Major 3, 09d — so there "the same as shipped" is
/// the claim, and it would fail if `j` opened a preview.)
#[test]
fn j_still_moves_in_every_place_after_merge_on_j() {
    let (keys, problems) = from_file("[keys.graph]\nmerge = \"j\"\n");
    assert_eq!(
        problems,
        ["j given to graph.merge — move down keeps ↓ on the graph"]
    );
    let j = press('j');
    for ctx in MOVERS {
        let base = fixture(ctx).unwrap_or_else(|| panic!("{ctx:?} fixture"));
        let moved = with_keys(&base, &keys);
        assert_eq!(
            seen(&moved, j),
            seen_as(&base, j, &keys),
            "{ctx:?}: j as shipped"
        );
        let (after, _) = update(&moved, AppEvent::Core(Event::Key(j)));
        assert!(after.preview.is_none(), "{ctx:?}: j opened a preview");
    }
    // Where the fixture has rows below the cursor, `j` visibly moves.
    for ctx in [ContextId::Changes, ContextId::Diff, ContextId::Menu] {
        let moved = with_keys(&fixture(ctx).expect("a fixture"), &keys);
        assert_ne!(seen(&moved, j), seen(&moved, unanswered()), "{ctx:?}");
    }
    // On the graph `j` merges and `↓` moves.
    let graph = with_keys(&sized_to_draw(on_a_branch_row()), &keys);
    let (merged, _) = update(&graph, AppEvent::Core(Event::Key(j)));
    assert!(merged.preview.is_some(), "j opens the merge preview");
    let down = KeyPress::plain(KeyCode::Down);
    assert_eq!(
        seen(&graph, down),
        seen_as(&sized_to_draw(on_a_branch_row()), down, &keys)
    );
}

/// Minor 9 through the file: `[keys.app] undo = "o"` is one problem naming
/// the graph, the resolver and the start screen.
#[test]
fn undo_on_o_is_one_problem() {
    let (_, problems) = from_file("[keys.app]\nundo = \"o\"\n");
    assert_eq!(problems.len(), 1, "{problems:?}");
    for place in ["on the graph", "in the resolver", "on the start screen"] {
        assert!(problems[0].contains(place), "{place}: {problems:?}");
    }
}

/// J8 still says one problem, in these words.
#[test]
fn j8_is_still_one_problem_in_its_words() {
    let (_, problems) =
        from_file("[keys]\nsuspend_on_ctrl_z = true\n\n[keys.graph]\nmerge = \"M\"\n");
    assert_eq!(problems, ["M given to graph.merge — map view has no key"]);
}

// ── Major 2 (D2) ────────────────────────────────────────────────────────

/// The graph as startup leaves it: his keymap seeded, the switch copied.
fn graph_with(keys: Arc<Keymap>, problems: Vec<String>, suspend: bool) -> AppState {
    let mut state = sized_to_draw(on_a_branch_row());
    state.suspend_on_ctrl_z = suspend;
    state.seed_keys(keys, problems);
    // The first key clears the config notice; press it so every case below
    // starts with the same bottom row.
    update(&state, AppEvent::Core(Event::Key(unanswered()))).0
}

fn ctrl(c: char) -> KeyPress {
    KeyPress {
        code: KeyCode::Char(c),
        modifiers: Modifiers::CONTROL,
    }
}

fn suspends(state: &AppState, key: KeyPress) -> bool {
    let (_, commands) = update(state, AppEvent::Core(Event::Key(key)));
    commands.contains(&Command::Suspend)
}

#[test]
fn switch_on_and_undo_on_u_still_suspends_on_ctrl_z() {
    let (keys, problems) =
        from_file("[keys]\nsuspend_on_ctrl_z = true\n\n[keys.app]\nundo = \"u\"\n");
    assert_eq!(problems, Vec::<String>::new());
    let state = graph_with(keys, problems, true);
    assert!(suspends(&state, ctrl('z')));
}

#[test]
fn switch_on_alone_spells_undo_u() {
    let (keys, problems) = from_file("[keys]\nsuspend_on_ctrl_z = true\n");
    assert_eq!(problems, Vec::<String>::new());
    assert_eq!(keys.spell(&ActionId("app.undo")).0, "u");
    assert!(suspends(&graph_with(keys, problems, true), ctrl('z')));
}

#[test]
fn suspend_on_ctrl_s_leaves_ctrl_z_unanswered() {
    let (keys, problems) =
        from_file("[keys]\nsuspend_on_ctrl_z = true\n\n[keys.app]\nsuspend = \"ctrl+s\"\n");
    assert_eq!(problems, Vec::<String>::new());
    let state = graph_with(keys, problems, true);
    assert!(suspends(&state, ctrl('s')));
    assert_eq!(seen(&state, ctrl('z')), seen(&state, unanswered()));
}

#[test]
fn a_suspend_line_with_the_switch_off_is_one_problem() {
    let (keys, problems) = from_file("[keys.app]\nsuspend = \"ctrl+s\"\n");
    assert_eq!(
        problems,
        ["app.suspend does nothing while suspend_on_ctrl_z is off"]
    );
    let state = graph_with(keys, problems, false);
    assert!(!suspends(&state, ctrl('s')));
}

// ── Major 5 (D3) ────────────────────────────────────────────────────────

/// A rewrite editor open on the merge fixture, planned.
fn rewrite_editor() -> AppState {
    let mut state = merge_on_screen(walk(), MERGE_ROW).0;
    super::rewrite_editor::tests::open_an_editor_planned(&mut state, true);
    sized_to_draw(state)
}

/// `[keys.confirm] yes = "c"`: `c` does what `y` did in every place that
/// confirms, `y` does what a key nobody answers does there, and on the
/// graph `y` still copies the SHA.
#[test]
fn confirm_on_c_confirms_every_box_and_y_still_copies_on_the_graph() {
    let (keys, _) = from_file("[keys.confirm]\nyes = \"c\"\n");
    let places = vec![
        ("preview", fixture(ContextId::Preview).expect("preview")),
        ("confirm box", fixture(ContextId::Confirm).expect("confirm")),
        (
            "finish files",
            sized_to_draw(finish_screen_for(walk(), SummaryFocus::Files, true)),
        ),
        ("rewrite editor", rewrite_editor()),
    ];
    for (name, base) in places {
        let moved = with_keys(&base, &keys);
        let shipped_y = seen_as(&base, press('y'), &keys);
        assert_ne!(
            shipped_y,
            seen_as(&base, unanswered(), &keys),
            "{name}: y confirms as shipped"
        );
        assert_eq!(seen(&moved, press('c')), shipped_y, "{name}: c confirms");
        assert_eq!(
            seen(&moved, press('y')),
            seen(&moved, unanswered()),
            "{name}: y is unanswered"
        );
    }
    let graph = sized_to_draw(on_a_branch_row());
    let moved = with_keys(&graph, &keys);
    let copied: Seen = seen_as(&graph, press('y'), &keys);
    assert_ne!(
        copied,
        seen_as(&graph, unanswered(), &keys),
        "y copies the SHA"
    );
    assert_eq!(seen(&moved, press('y')), copied);
}

/// `merge = "M"`: `M` opens the merge preview, the box names `M merge`,
/// and `M` again confirms it, exactly as `m` again does under the shipped
/// keymap. (The repeat is checked on the planned preview fixture, re-marked
/// as `m`-opened: a fresh preview is still predicting, where no key acts.)
#[test]
fn merge_on_capital_m_confirms_on_capital_m_again() {
    let keys = lines("graph.merge", "M");
    let graph = with_keys(&sized_to_draw(on_a_branch_row()), &keys);
    let (opened, _) = update(&graph, AppEvent::Core(Event::Key(press('M'))));
    assert!(opened.preview.is_some(), "M opens the preview");
    let drawn = crate::view::tests::draw(&opened, 120, 36);
    assert!(drawn.contains("M merge"), "{drawn}");

    let mut planned = fixture(ContextId::Preview).expect("a preview");
    let Some(crate::update::preview::PreviewSubject::Operation { opened_by, .. }) =
        planned.preview.as_mut().map(|preview| &mut preview.subject)
    else {
        panic!("an operation preview");
    };
    *opened_by = "m";
    let shipped = seen_as(&planned, press('m'), &keys);
    assert_ne!(
        shipped,
        seen_as(&planned, unanswered(), &keys),
        "m again confirms as shipped"
    );
    let moved = with_keys(&planned, &keys);
    assert_eq!(seen(&moved, press('M')), shipped, "M again confirms");
    assert_eq!(
        seen(&moved, press('m')),
        seen(&moved, unanswered()),
        "m is unanswered"
    );
}

/// A line on the repeat key itself is not applied, and says so.
#[test]
fn a_line_on_the_repeat_key_is_one_problem() {
    let (keys, problems) = from_file("[keys.preview]\nconfirm_merge = \"x\"\n");
    assert_eq!(
        problems,
        ["preview.confirm_merge can't be remapped — it is graph.merge's key, so move that"]
    );
    assert_eq!(keys.spell(&ActionId("preview.confirm_merge")).0, "m");
}

// ── R4 ──────────────────────────────────────────────────────────────────

/// (a) `confirm.yes = "c"`: in the finish subject `c` then `y` type `cy`
/// and commit nothing; from the files, `c` commits.
#[test]
fn confirm_on_c_types_in_the_subject_and_commits_from_the_files() {
    let (keys, _) = from_file("[keys.confirm]\nyes = \"c\"\n");
    let subject = with_keys(
        &sized_to_draw(finish_screen_for(walk(), SummaryFocus::Subject, true)),
        &keys,
    );
    let (after_c, c_commands) = update(&subject, AppEvent::Core(Event::Key(press('c'))));
    let (after_y, y_commands) = update(&after_c, AppEvent::Core(Event::Key(press('y'))));
    assert_eq!(c_commands, []);
    assert_eq!(y_commands, []);
    let finish = after_y.finish.as_ref().expect("still on the finish screen");
    assert!(
        finish.subject.text().ends_with("cy"),
        "{}",
        finish.subject.text()
    );
    assert!(finish.committing.is_none(), "nothing committed");

    let files = with_keys(
        &sized_to_draw(finish_screen_for(walk(), SummaryFocus::Files, true)),
        &keys,
    );
    let (committed, _) = update(&files, AppEvent::Core(Event::Key(press('c'))));
    assert!(
        committed
            .finish
            .as_ref()
            .is_some_and(|finish| finish.committing.is_some()),
        "c commits from the files"
    );
}

/// (b) Wherever `text_field_open()` holds, no key his remap moves is drawn:
/// under the sweep keymap (every action on an `alt` chord) no bar or footer
/// shows a moved chord. A `Key` hint would spell one; a `Fixed` hint spells
/// the shipped key, which is what the field answers. The bar is the bottom
/// row; a footer sits in a box's bottom border (`╰`). Pane text is not
/// checked: the journal's legend and blame's find header spell his undo
/// and ignore-revs keys while their find is open (09c NOTES).
#[test]
fn a_text_field_draws_no_key_his_remap_moves() {
    let (keys, _) = swept();
    let mut fields = every_field();
    fields.push((
        "finish body",
        sized_to_draw(finish_screen_for(walk(), SummaryFocus::Body, true)),
    ));
    let mut checked = 0;
    for (name, base) in fields {
        assert!(base.text_field_open(), "{name}: open");
        let moved = with_keys(&base, &keys);
        let frame = crate::view::tests::draw(&moved, moved.size.columns, moved.size.rows);
        let lines: Vec<&str> = frame.lines().collect();
        assert!(
            lines.last().is_some_and(|bar| !bar.trim().is_empty()),
            "{name}: the bottom row is a bar\n{frame}"
        );
        let bars = lines
            .iter()
            .rev()
            .take(1)
            .chain(lines.iter().filter(|line| line.contains('╰')));
        for line in bars {
            assert!(
                !line.contains("alt+"),
                "{name}: a remappable key is drawn where no remap acts: {line}\n{frame}"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 11, "every field text_field_open names");
    // The sweep's chords do spell `alt+`: the check can fail.
    assert!(
        keys.spell(&ActionId("graph.merge")).0.starts_with("alt+"),
        "{:?}",
        keys.spell(&ActionId("graph.merge"))
    );
}
