//! The palette and help (m8-polish-and-ship/11, §2.12): the journeys walked
//! input by input, and `?` swept over every place.
//!
//! An *input* is one key, or one typed word (the design note's count).

use graft_ui::palette::PaletteMode;
use graft_ui::theme::ThemeKind;

use super::*;
use crate::update::palette::{PaletteTarget, rows};

/// What the walk did: the state, the commands, the inputs it took.
struct Walk {
    state: AppState,
    commands: Vec<Command>,
    inputs: usize,
}

impl Walk {
    /// From `state` as the loop leaves it: one key in, its probes asked
    /// (not counted — the walk starts after).
    fn from(state: &AppState) -> Self {
        Self {
            state: settled(state),
            commands: Vec::new(),
            inputs: 0,
        }
    }

    fn press(mut self, press: KeyPress) -> Self {
        let (after, commands) = update(&self.state, AppEvent::Core(Event::Key(press)));
        self.state = after;
        self.commands.extend(commands);
        self.inputs += 1;
        self
    }

    fn char(self, c: char) -> Self {
        self.press(KeyPress::plain(KeyCode::Char(c)))
    }

    /// One typed word: one input, however many letters.
    fn typed(mut self, word: &str) -> Self {
        for c in word.chars() {
            let (after, commands) = update(
                &self.state,
                AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char(c)))),
            );
            self.state = after;
            self.commands.extend(commands);
        }
        self.inputs += 1;
        self
    }

    fn enter(self) -> Self {
        self.press(KeyPress::plain(KeyCode::Enter))
    }

    /// The row under the cursor: its group's header, its label and key.
    fn under_cursor(&self) -> (String, String, Option<String>, PaletteTarget) {
        let palette = self.state.palette.as_ref().expect("the palette is up");
        let groups = rows(&self.state, palette);
        groups
            .iter()
            .flat_map(|(header, rows)| rows.iter().map(move |row| (header, row)))
            .nth(palette.cursor)
            .map(|(header, (row, target))| {
                (
                    header.as_str().to_owned(),
                    row.label.as_str().to_owned(),
                    row.key.as_ref().map(|k| k.0.clone()),
                    *target,
                )
            })
            .expect("a row under the cursor")
    }
}

fn ctrl_p() -> KeyPress {
    KeyPress::control(KeyCode::Char('p'))
}

fn key_colon() -> KeyPress {
    KeyPress::plain(KeyCode::Char(':'))
}

/// J4: `?` `merge` — the cursor on HERE's merge row, spelled `m`. 2 inputs.
#[test]
fn j4_help_then_merge_lands_on_the_merge_row_in_two_inputs() {
    let walk = Walk::from(&sized_to_draw(on_a_branch_row()))
        .char('?')
        .typed("merge");
    let palette = walk.state.palette.as_ref().expect("open");
    assert_eq!(palette.mode, PaletteMode::Here(ContextId::Graph));
    let (header, label, key, _) = walk.under_cursor();
    assert_eq!(header, "HERE");
    assert_eq!(label, "Merge selected branch into current");
    assert_eq!(key.as_deref(), Some("m"));
    assert!(walk.inputs <= 2, "{} inputs", walk.inputs);
}

/// J5: `?` `x` — what `x` does here heads the list. 2 inputs.
#[test]
fn j5_help_then_a_key_puts_what_it_does_first() {
    let walk = Walk::from(&sized_to_draw(on_a_branch_row()))
        .char('?')
        .char('x');
    let palette = walk.state.palette.as_ref().expect("open");
    assert_eq!(palette.cursor, 0);
    let (_, label, key, target) = walk.under_cursor();
    assert_eq!(label, "Compare against a marked base");
    assert_eq!(key.as_deref(), Some("x"));
    assert_eq!(target, PaletteTarget::Action(ActionId("graph.compare")));
    assert!(walk.inputs <= 2, "{} inputs", walk.inputs);
}

/// J6: `⌃P` `stash` `⏎` — does what `S` does on the graph. 3 inputs.
#[test]
fn j6_palette_stash_enter_does_what_its_key_does() {
    let start = sized_to_draw(on_a_branch_row());
    let walk = Walk::from(&start).press(ctrl_p()).typed("stash").enter();
    assert!(walk.state.palette.is_none(), "closed after running");
    let by_key = Walk::from(&start).char('S');
    assert_eq!(walk.commands, by_key.commands);
    assert_eq!(
        crate::view::tests::draw(&walk.state, 120, 36),
        crate::view::tests::draw(&by_key.state, 120, 36),
        "the same screen as pressing S"
    );
    assert!(walk.inputs <= 3, "{} inputs", walk.inputs);
}

/// J9: `⌃P` `theme light` `⏎` — the loop's theme command. 3 inputs.
#[test]
fn j9_palette_theme_light_asks_the_loop_for_light() {
    let walk = Walk::from(&sized_to_draw(on_a_branch_row()))
        .press(ctrl_p())
        .typed("theme light")
        .enter();
    assert!(walk.state.palette.is_none());
    assert_eq!(walk.commands, vec![Command::Retheme(ThemeKind::Light)]);
    assert!(walk.inputs <= 3, "{} inputs", walk.inputs);
}

/// Decision 5: a refusal through the palette reads as from the key. On a
/// detached `HEAD`, `⌃P` `merge` `⏎` says what `m` says, in `m`'s words,
/// and asks nothing.
#[test]
fn a_refusal_through_the_palette_reads_as_from_the_key() {
    let mut start = sized_to_draw(on_a_branch_row());
    let name = |full: &str| RefName::new(full.as_bytes().to_vec()).expect("a valid ref name");
    let oid = |byte: u8| Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid");
    let detached = crate::refs::badges(
        &[
            (name("refs/heads/main"), oid(1)),
            (name("refs/heads/feature"), oid(2)),
        ],
        &graft_git::repo::HeadState::Detached(oid(1)),
        &HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("badges");
    start.refs = Some(Arc::new(detached));
    let typed = Walk::from(&start).press(ctrl_p()).typed("merge");
    let (_, _, _, target) = typed.under_cursor();
    assert_eq!(target, PaletteTarget::Action(ActionId("graph.merge")));
    let walk = typed.enter();
    let by_key = Walk::from(&start).char('m');
    assert!(walk.state.palette.is_none());
    assert_eq!(walk.commands, by_key.commands);
    assert_eq!(
        format!("{:?}", walk.state.status),
        format!("{:?}", by_key.state.status)
    );
    match &walk.state.status {
        Status::Sticky { reason, .. } => assert_eq!(reason, "m needs a branch checked out"),
        other => panic!("expected the key's refusal, got {other:?}"),
    }
}

/// `⌃P` and `:` both open on everything, on the graph and on a pane.
#[test]
fn ctrl_p_and_colon_open_everything_on_the_graph_and_a_pane() {
    let places: Vec<(&str, AppState)> = vec![
        ("graph", on_a_branch_row()),
        ("changes", fixture(ContextId::Changes).expect("changes")),
    ];
    for (name, state) in places {
        for press in [ctrl_p(), KeyPress::plain(KeyCode::Char(':'))] {
            let walk = Walk::from(&sized_to_draw(state.clone())).press(press);
            let palette = walk.state.palette.as_ref().expect(name);
            assert_eq!(palette.mode, PaletteMode::Everything, "{name}");
        }
    }
}

/// `esc` closes with nothing run and the place as it was; `Ctrl+C` quits
/// with the palette up.
#[test]
fn esc_closes_with_nothing_run_and_ctrl_c_still_quits() {
    let start = sized_to_draw(on_a_branch_row());
    let closed = Walk::from(&start)
        .char('?')
        .typed("mer")
        .press(KeyPress::plain(KeyCode::Escape));
    assert!(closed.state.palette.is_none());
    assert!(closed.commands.is_empty(), "{:?}", closed.commands);
    assert_eq!(
        crate::view::tests::draw(&closed.state, 120, 36),
        crate::view::tests::draw(&start, 120, 36)
    );
    let quit = Walk::from(&start)
        .char('?')
        .press(KeyPress::control(KeyCode::Char('c')));
    let by_key = Walk::from(&start).press(KeyPress::control(KeyCode::Char('c')));
    assert_eq!(
        format!("{:?}", quit.state.quit),
        format!("{:?}", by_key.state.quit)
    );
    assert_ne!(format!("{:?}", quit.state.quit), "None");
}

/// Decision 3: after `[keys.graph] merge = "j"`, `?` on the graph spells
/// merge `j` and move `↓`; on the Changes pane `j` still moves.
#[test]
fn rows_are_spelled_with_what_the_key_does_in_that_place() {
    let (map, _) = Keymap::with_overrides(
        &[("graph.merge".to_owned(), vec!["j".to_owned()])],
        graft_ui::keymap::KeySwitches::default(),
    );
    let keys = Arc::new(map);
    let key_of = |state: AppState, action: &'static str| {
        let state = AppState {
            keymap: keys.clone(),
            ..sized_to_draw(state)
        };
        let walk = Walk::from(&state).char('?');
        let palette = walk.state.palette.as_ref().expect("open");
        rows(&walk.state, palette)
            .into_iter()
            .flat_map(|(_, rows)| rows)
            .find(|(_, target)| *target == PaletteTarget::Action(ActionId(action)))
            .and_then(|(row, _)| row.key.map(|k| k.0))
    };
    assert_eq!(
        key_of(on_a_branch_row(), "graph.merge").as_deref(),
        Some("j")
    );
    assert_eq!(key_of(on_a_branch_row(), "app.down").as_deref(), Some("↓"));
    let changes = fixture(ContextId::Changes).expect("changes");
    assert_eq!(key_of(changes, "app.down").as_deref(), Some("j"));
}

/// Two config problems: `YOUR CONFIG` heads both listings, and the cursor
/// starts on the first row that runs.
#[test]
fn config_problems_head_the_list() {
    let mut state = sized_to_draw(on_a_branch_row());
    state.config_problems = vec!["first problem".to_owned(), "second problem".to_owned()];
    for press in [KeyPress::plain(KeyCode::Char('?')), ctrl_p()] {
        let walk = Walk::from(&state).press(press);
        let palette = walk.state.palette.as_ref().expect("open");
        let groups = rows(&walk.state, palette);
        assert_eq!(groups[0].0.as_str(), "YOUR CONFIG");
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(palette.cursor, 2, "past the notes");
    }
}

/// Blame's find prompt drops `?`: `place_claim` (update/keys.rs:125) lets
/// `live_global` (place/keys.rs:23-29) pass `?` and `q` up while the
/// prompt is open, and nothing types them. Found by this ticket, not fixed
/// here: it predates the palette (09e's ground).
const NOT_TYPED_YET: &str = "blame find";

/// `?` on every place opens help on that place; over a confirm box nothing
/// happens; in every text field `?` is typed.
#[test]
fn help_opens_on_every_place_and_is_typed_in_every_field() {
    for ctx in ContextId::ALL {
        let Some(state) = fixture(*ctx) else {
            continue;
        };
        // The fixture as 09d's sweep presses it: unsettled, since a key
        // nobody answers closes a menu.
        let after = update(&state, key(KeyCode::Char('?'))).0;
        if state.confirm.is_some() {
            // A confirm box swallows them (§2.3c): what `?`, `⌃P` and `:`
            // do there is what a key nobody answers does.
            for press in [KeyPress::plain(KeyCode::Char('?')), ctrl_p(), key_colon()] {
                assert_eq!(
                    seen(&state, press),
                    seen(&state, unanswered()),
                    "{ctx:?}: {press:?} over a confirm box"
                );
            }
            continue;
        }
        if state.text_field_open() {
            assert!(after.palette.is_none(), "{ctx:?}: not over a field");
            continue;
        }
        let palette = after.palette.as_ref().unwrap_or_else(|| panic!("{ctx:?}"));
        assert_eq!(palette.mode, PaletteMode::Here(*ctx), "{ctx:?}");
    }
    for (name, field) in every_field() {
        let before = crate::view::tests::draw(&field, 120, 36)
            .matches('?')
            .count();
        let after = update(&field, key(KeyCode::Char('?'))).0;
        assert!(after.palette.is_none(), "{name}");
        let now = crate::view::tests::draw(&after, 120, 36)
            .matches('?')
            .count();
        if name == NOT_TYPED_YET {
            assert_eq!(now, before, "{name}: still dropped; drop the exception");
            continue;
        }
        assert!(now > before, "{name}: `?` typed");
    }
}

/// `docs/shots/m8-help-graph-120x36.txt` and
/// `docs/shots/m8-palette-everything-120x36.txt`. What right looks like:
/// `?` on the graph — a box at most 96 wide centred over the graph, titled
/// `keys here`, `esc close` on its top border, the `›` query row, then
/// `HERE` (the graph's keys, `▸` on the first) and `EVERYWHERE`, each row
/// `key  label … config id`; the bar reads `↑↓ move  ⏎ run  esc close`.
/// `⌃P` — titled `find anything`, one `ACTIONS` group. Colour unverified
/// (rule 5).
#[test]
fn capture_help_and_palette() {
    let start = sized_to_draw(on_a_branch_row());
    for (name, press) in [
        (
            "m8-help-graph-120x36.txt",
            KeyPress::plain(KeyCode::Char('?')),
        ),
        ("m8-palette-everything-120x36.txt", ctrl_p()),
    ] {
        let walk = Walk::from(&start).press(press);
        graft_fixtures::capture(
            format!("{}/../../docs/shots/{name}", env!("CARGO_MANIFEST_DIR")),
            crate::view::tests::draw(&walk.state, 120, 36),
        );
    }
}

// --- m8/11b: the doorway, the bar rows, the mouse ---

/// `state` at `columns` × `rows`.
fn at_size(state: AppState, columns: u16, rows: u16) -> AppState {
    AppState {
        size: Size { columns, rows },
        ..state
    }
}

/// The hint bar row of a drawn frame: its last line, without the quotes
/// `TestBackend` puts round each.
fn bar_of(frame: &str) -> String {
    frame
        .lines()
        .last()
        .expect("a frame has rows")
        .trim_matches('"')
        .trim_end()
        .to_owned()
}

/// Row 163 (decision 2): at 80×24 the graph's bar ends `? keys`; `space`,
/// `1` and `H`, which the width may drop, are listed by `?` under HERE.
/// Also `docs/shots/m8-help-80x24.txt`: what right looks like — the bar
/// ending `? keys`, then the same screen with `?` open, `HERE` first.
/// Colour unverified (rule 5).
#[test]
fn at_80_the_bar_ends_with_keys_and_help_lists_what_it_dropped() {
    let start = at_size(on_a_branch_row(), 80, 24);
    let before = crate::view::tests::draw(&start, 80, 24);
    let bar = bar_of(&before);
    assert!(bar.ends_with("? keys"), "{bar:?}");
    let walk = Walk::from(&start).char('?');
    let palette = walk.state.palette.as_ref().expect("open");
    let here: Vec<String> = rows(&walk.state, palette)
        .into_iter()
        .filter(|(header, _)| header.as_str() == "HERE")
        .flat_map(|(_, rows)| rows)
        .filter_map(|(row, _)| row.key.map(|k| k.0))
        .collect();
    for key in ["space", "1", "H"] {
        assert!(
            here.iter().any(|k| k == key),
            "`{key}` under HERE: {here:?}"
        );
    }
    let after = crate::view::tests::draw(&walk.state, 80, 24);
    graft_fixtures::capture(
        format!(
            "{}/../../docs/shots/m8-help-80x24.txt",
            env!("CARGO_MANIFEST_DIR")
        ),
        format!("{before}\n{after}"),
    );
}

/// Row 538 (decision 3): the Detail tab under 100 columns (graph hidden)
/// offers no `1 first-parent`; the graph's bar at 120 still does.
#[test]
fn the_detail_tab_offers_no_first_parent() {
    use graft_ui::graph_screen::{Divider, hints_for};
    let first_parent = |context: &HintContext| {
        hints_for(context, Divider::Hidden, 240, Keymap::shipped())
            .iter()
            .any(|hint| hint.verb == "first-parent")
    };
    let tab = at_size(super::commit_pane(JobId::new(703), true), 80, 48);
    assert_eq!(
        crate::update::detail::tab_hint_rows(&tab),
        1,
        "the tab layout"
    );
    let context = crate::view::rows::pane_hint_context(&tab);
    assert!(!first_parent(&context), "{context:?}");
    // The graph at 120: its bar still offers `1` (the width may drop it;
    // `?` lists it).
    let graph = at_size(on_a_branch_row(), 120, 40);
    let context = crate::view::rows::pane_hint_context(&graph);
    assert!(first_parent(&context), "{context:?}");
}

fn mouse(state: &AppState, kind: graft_core::event::MouseKind, column: u16, row: u16) -> Walk {
    let (after, commands) = update(
        state,
        AppEvent::Core(Event::Mouse(graft_core::event::MouseInput {
            kind,
            column,
            row,
            modifiers: Modifiers::NONE,
        })),
    );
    Walk {
        state: after,
        commands,
        inputs: 1,
    }
}

fn left() -> graft_core::event::MouseKind {
    graft_core::event::MouseKind::Down(graft_core::event::MouseButton::Left)
}

/// The screen row the drawn palette shows `label` on, and a column inside
/// the box: read off the real frame.
fn drawn_row(state: &AppState, label: &str) -> (u16, u16) {
    let frame = crate::view::tests::draw(state, state.size.columns, state.size.rows);
    let row = frame
        .lines()
        .position(|line| line.contains(label))
        .unwrap_or_else(|| panic!("`{label}` drawn\n{frame}"));
    (
        state.size.columns / 2,
        u16::try_from(row).expect("a screen row"),
    )
}

fn at_second(state: AppState, seconds: i64) -> AppState {
    let mut state = state;
    state.harness.clock = crate::session::Clock::Fixed(seconds);
    state
}

/// Decision 6: a click on a row moves the cursor there and runs nothing.
#[test]
fn a_click_in_the_palette_moves_the_cursor_only() {
    let start = at_second(sized_to_draw(on_a_branch_row()), 1_000);
    let open = Walk::from(&start).char('?');
    let before = open.state.palette.as_ref().expect("open").cursor;
    let (column, row) = drawn_row(&open.state, "graph.fetch");
    let clicked = mouse(&open.state, left(), column, row);
    let palette = clicked.state.palette.as_ref().expect("still open");
    assert_ne!(palette.cursor, before);
    let (_, label, _, _) = clicked.under_cursor();
    assert_eq!(label, "Fetch");
    assert!(clicked.commands.is_empty(), "{:?}", clicked.commands);
}

/// Decision 6: J6's row double-clicked does what `S` does.
#[test]
fn a_double_click_runs_the_row_as_its_key() {
    let start = at_second(sized_to_draw(on_a_branch_row()), 1_000);
    let typed = Walk::from(&start).press(ctrl_p()).typed("stash");
    let (column, row) = drawn_row(&typed.state, "Stash…");
    let once = mouse(&typed.state, left(), column, row);
    assert!(once.state.palette.is_some(), "one click runs nothing");
    let twice = mouse(&once.state, left(), column, row);
    assert!(twice.state.palette.is_none(), "closed after running");
    let by_key = Walk::from(&start).char('S');
    assert_eq!(twice.commands, by_key.commands);
    assert_eq!(
        crate::view::tests::draw(&twice.state, 120, 36),
        crate::view::tests::draw(&by_key.state, 120, 36),
        "the same screen as pressing S"
    );
}

/// Decision 6: the wheel moves the cursor, clamped at the top.
#[test]
fn the_wheel_moves_the_palette_cursor() {
    use graft_core::event::MouseKind;
    let open = Walk::from(&sized_to_draw(on_a_branch_row())).char('?');
    let at = open.state.palette.as_ref().expect("open").cursor;
    let down = mouse(&open.state, MouseKind::ScrollDown, 60, 10);
    assert_eq!(down.state.palette.as_ref().expect("open").cursor, at + 1);
    let mut up = down;
    for _ in 0..(at + 5) {
        up = mouse(&up.state, MouseKind::ScrollUp, 60, 10);
    }
    assert_eq!(up.state.palette.as_ref().expect("open").cursor, 0);
}

/// Decision 6: a click outside the palette closes it with nothing run,
/// and reaches nothing behind it.
#[test]
fn a_click_outside_the_palette_closes_it() {
    let open = Walk::from(&sized_to_draw(on_a_branch_row())).char('?');
    let cursor = open.state.history.as_ref().expect("a walk").cursor;
    // The bottom-left cell: the bar row, outside the box.
    let clicked = mouse(&open.state, left(), 0, open.state.size.rows - 1);
    assert!(clicked.state.palette.is_none(), "closed");
    assert!(clicked.commands.is_empty(), "{:?}", clicked.commands);
    assert_eq!(
        clicked.state.history.as_ref().expect("a walk").cursor,
        cursor
    );
}

/// Decision 6: the palette is first in the mouse routing — opened over the
/// `.` menu, a click on a palette row is the palette's, not the menu's.
/// (The branch picker is a text field: `?` types into it, so the palette
/// never stands over it — `palette::open_key`.)
#[test]
fn the_palette_takes_the_click_before_the_menu() {
    let start = at_second(sized_to_draw(on_a_branch_row()), 1_000);
    let open = Walk::from(&start).char('.').char('?');
    assert!(open.state.context_menu.is_some(), "the menu is still up");
    let (column, row) = drawn_row(&open.state, "graph.fetch");
    let clicked = mouse(&open.state, left(), column, row);
    let (_, label, _, _) = clicked.under_cursor();
    assert_eq!(label, "Fetch");
    assert!(clicked.commands.is_empty(), "{:?}", clicked.commands);
}
