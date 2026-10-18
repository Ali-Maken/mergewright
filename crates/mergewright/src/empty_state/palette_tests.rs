//! The palette and help on the start screen (m8-polish-and-ship/11
//! decision 7: "`?` is help on every screen", §2.2). The start screen has no
//! `AppState`, so it keeps its own `PaletteState`; the rows come from the
//! same listing (`update::palette::listing`).

#![allow(
    clippy::expect_used,
    reason = "a palette that is not up leaves nothing to test"
)]

use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_ui::keymap::{ActionId, ContextId, Keymap};
use mergewright_ui::palette::PaletteMode;
use mergewright_ui::text_input::TextField;
use mergewright_ui::theme::ThemeKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::{OutsideState, OutsideStep, Prompt, outside_edge, palette_rows, render};
use crate::recent::Recent;
use crate::update::palette::PaletteTarget;

fn state() -> OutsideState {
    let recent = |path: &str| Recent {
        path: PathBuf::from(path),
        opened: UNIX_EPOCH,
    };
    OutsideState {
        folder: PathBuf::from("/me/code"),
        missing: false,
        home: Some(PathBuf::from("/me")),
        recents: vec![
            recent("/me/code/payments-service"),
            recent("/me/code/web-dashboard"),
        ],
        cursor: 0,
        prompt: None,
        problem: None,
        palette: None,
        config_problems: Vec::new(),
        first_launch: None,
        credential: None,
    }
}

fn keys() -> &'static Keymap {
    Keymap::shipped()
}

fn ch(c: char) -> KeyPress {
    KeyPress::plain(KeyCode::Char(c))
}

fn press(s: &mut OutsideState, key: KeyPress) -> OutsideStep {
    outside_edge(s, keys(), key)
}

fn typed(s: &mut OutsideState, word: &str) {
    for c in word.chars() {
        assert_eq!(press(s, ch(c)), OutsideStep::Stay, "{c}");
    }
}

/// The row under the cursor: its group's header and what `⏎` runs.
fn under_cursor(s: &OutsideState) -> (String, PaletteTarget) {
    let palette = s.palette.as_ref().expect("the palette is up");
    palette_rows(s, keys())
        .iter()
        .flat_map(|(header, rows)| rows.iter().map(move |row| (header, row)))
        .nth(palette.cursor)
        .map(|(header, (_, target))| (header.as_str().to_owned(), *target))
        .expect("a row under the cursor")
}

/// `?` opens help on the start screen; `esc` closes it, the screen as it
/// was.
#[test]
fn help_opens_on_the_start_screen_and_esc_closes_it() {
    let start = state();
    let mut s = start.clone();
    assert_eq!(press(&mut s, ch('?')), OutsideStep::Stay);
    let palette = s.palette.as_ref().expect("help is up");
    assert_eq!(palette.mode, PaletteMode::Here(ContextId::EmptyState));
    assert_eq!(palette.opened_from, ContextId::EmptyState);
    let (header, _) = under_cursor(&s);
    assert_eq!(header, "HERE");
    assert_eq!(
        press(&mut s, KeyPress::plain(KeyCode::Escape)),
        OutsideStep::Stay
    );
    assert_eq!(s, start, "closed, nothing run");
}

/// `⌃P` and `:` open on everything.
#[test]
fn ctrl_p_and_colon_open_everything_on_the_start_screen() {
    for key in [KeyPress::control(KeyCode::Char('p')), ch(':')] {
        let mut s = state();
        assert_eq!(press(&mut s, key), OutsideStep::Stay);
        let palette = s.palette.as_ref().expect("up");
        assert_eq!(palette.mode, PaletteMode::Everything, "{key:?}");
    }
}

/// `?` `open` `⏎` does what `o` does: the open prompt, nothing else.
#[test]
fn enter_runs_a_row_as_its_key_would() {
    let mut walked = state();
    press(&mut walked, ch('?'));
    typed(&mut walked, "open");
    assert_eq!(
        under_cursor(&walked).1,
        PaletteTarget::Action {
            id: ActionId("empty_state.open"),
            ctx: ContextId::EmptyState
        }
    );
    let step = press(&mut walked, KeyPress::plain(KeyCode::Enter));
    let mut by_key = state();
    let by_key_step = press(&mut by_key, ch('o'));
    assert_eq!(step, by_key_step);
    assert!(walked.palette.is_none(), "closed after running");
    assert_eq!(walked, by_key);
}

/// m8/18c: `?` `clone` `⏎` does what `c` does — the clone prompt opens.
#[test]
fn the_clone_row_opens_the_clone_prompt() {
    let mut walked = state();
    press(&mut walked, ch('?'));
    typed(&mut walked, "clone");
    assert_eq!(
        under_cursor(&walked).1,
        PaletteTarget::Action {
            id: ActionId("empty_state.clone"),
            ctx: ContextId::EmptyState
        }
    );
    let step = press(&mut walked, KeyPress::plain(KeyCode::Enter));
    let mut by_key = state();
    assert_eq!(step, press(&mut by_key, ch('c')));
    assert!(walked.palette.is_none(), "closed after running");
    assert_eq!(walked, by_key);
    assert!(matches!(walked.prompt, Some(super::Prompt::Clone(_))));
}

/// `⌃P` `theme light` `⏎` asks the loop for light.
#[test]
fn a_theme_row_asks_the_loop_for_that_theme() {
    let mut s = state();
    press(&mut s, KeyPress::control(KeyCode::Char('p')));
    typed(&mut s, "theme light");
    assert_eq!(
        press(&mut s, KeyPress::plain(KeyCode::Enter)),
        OutsideStep::Retheme(ThemeKind::Light)
    );
    assert!(s.palette.is_none());
}

/// `Ctrl+C` still quits with the palette up; a paste goes into its query.
#[test]
fn ctrl_c_quits_with_the_palette_up() {
    let mut s = state();
    press(&mut s, ch('?'));
    assert_eq!(
        press(&mut s, KeyPress::control(KeyCode::Char('c'))),
        OutsideStep::Quit
    );
}

/// Over the init box `?` is swallowed (§2.3c); in the open prompt it is
/// typed.
#[test]
fn not_over_the_init_box_and_typed_in_the_open_prompt() {
    let boxed = OutsideState {
        prompt: Some(Prompt::ConfirmInit),
        ..state()
    };
    for key in [ch('?'), ch(':'), KeyPress::control(KeyCode::Char('p'))] {
        let mut s = boxed.clone();
        assert_eq!(press(&mut s, key), OutsideStep::Stay);
        assert_eq!(s, boxed, "{key:?}: the box swallows it");
    }
    let mut s = OutsideState {
        prompt: Some(Prompt::Open(TextField::new("~/code/"))),
        ..state()
    };
    press(&mut s, ch('?'));
    assert!(s.palette.is_none());
    let Some(Prompt::Open(field)) = &s.prompt else {
        panic!("the prompt stays: {:?}", s.prompt);
    };
    assert_eq!(field.text(), "~/code/?");
}

/// His config's problems head the list here too.
#[test]
fn config_problems_head_the_list_on_the_start_screen() {
    let mut s = OutsideState {
        config_problems: vec!["a problem".to_owned()],
        ..state()
    };
    press(&mut s, ch('?'));
    let groups = palette_rows(&s, keys());
    assert_eq!(groups[0].0.as_str(), "YOUR CONFIG");
    assert_eq!(s.palette.as_ref().expect("up").cursor, 1, "past the note");
}

/// Drawn over the start screen, its keys on the bar row. Writes
/// `docs/shots/m8-help-empty-state-120x36.txt`. What right looks like: the
/// start screen behind, a box titled `keys here` with `esc close` on its
/// top border, the `›` query row, `HERE` (the start screen's keys: open,
/// init, …, `▸` on the first) then `EVERYWHERE`; the bottom row reads
/// `↑↓ move  ⏎ run  esc close`. Colour unverified (rule 5).
#[test]
fn the_palette_draws_over_the_start_screen_with_its_keys_on_the_bar() {
    let theme = crate::session::for_tests::session().theme;
    let area = Rect::new(0, 0, 120, 36);
    let mut s = state();
    press(&mut s, ch('?'));
    let mut buffer = Buffer::empty(area);
    render(&s, keys(), &theme, false, UNIX_EPOCH, area, &mut buffer);
    let rows: Vec<String> = (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect()
        })
        .collect();
    let all = rows.join("\n");
    assert!(all.contains("keys here"), "{all}");
    assert!(all.contains("HERE"), "{all}");
    let bar = rows.last().expect("a bar row");
    assert!(bar.contains("↑↓ move"), "{bar}");
    assert!(bar.contains("esc close"), "{bar}");
    let trimmed: Vec<&str> = rows.iter().map(|row| row.trim_end()).collect();
    mergewright_fixtures::capture(
        format!(
            "{}/../../docs/shots/m8-help-empty-state-120x36.txt",
            env!("CARGO_MANIFEST_DIR")
        ),
        trimmed.join("\n") + "\n",
    );
}

/// The first palette open on the start screen ends the first-launch line
/// (m8/18d, R7): not drawn from that frame, and the loop is asked to end
/// the process's switch and write `done` (m8/18e) — the first open only.
/// The key touches no file itself.
#[test]
fn opening_the_palette_ends_the_first_launch_line() {
    for opener in [ch('?'), KeyPress::control(KeyCode::Char('p')), ch(':')] {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut s = OutsideState {
            first_launch: Some(dir.path().to_path_buf()),
            ..state()
        };
        assert!(super::view(&s, UNIX_EPOCH).first_launch);
        let first = press(&mut s, opener);
        assert_eq!(
            first,
            OutsideStep::FirstLaunchDone(dir.path().to_path_buf()),
            "{opener:?}"
        );
        assert!(s.palette.is_some(), "{opener:?}");
        assert!(!super::view(&s, UNIX_EPOCH).first_launch, "{opener:?}");
        assert!(
            !crate::first_launch::launches_file(dir.path()).exists(),
            "the key writes nothing"
        );

        press(&mut s, KeyPress::plain(KeyCode::Escape));
        assert!(s.palette.is_none(), "{opener:?}");
        let second = press(&mut s, opener);
        assert_eq!(second, OutsideStep::Stay, "{opener:?}: the second open");
        assert!(s.palette.is_some(), "{opener:?}");
    }
}

/// 06i: the start screen has no graph, so neither `?` nor `⌃P` lists
/// `GRAPH MODE`, even typed `graph`.
#[test]
fn the_start_screen_never_lists_graph_mode() {
    for key in [ch('?'), KeyPress::control(KeyCode::Char('p'))] {
        let mut s = state();
        assert_eq!(press(&mut s, key), OutsideStep::Stay);
        for word in ["", "graph"] {
            typed(&mut s, word);
            let listed = palette_rows(&s, keys());
            assert!(
                listed
                    .iter()
                    .all(|(header, rows)| header.as_str() != "GRAPH MODE"
                        && rows
                            .iter()
                            .all(|(row, _)| row.label.as_str() != "Every branch")),
                "{key:?} {word:?}"
            );
        }
    }
}
