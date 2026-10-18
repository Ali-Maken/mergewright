//! The commit screen's mouse (ease-pass/08s M5 + Q2), driven through
//! `update`. Rows are found in the drawn frame, not computed: a click is
//! tested against what the user sees.

use graft_core::event::{Event, KeyCode, Modifiers, MouseButton, MouseInput, MouseKind};

use super::super::screen_tests::{control, editor, key, mock_tree, opened, walk};
use crate::update::commit_editor::CommitFocus;
use crate::update::{AppEvent, AppState, Command, update};

/// The file list's width beside the diff at 120 columns (`FILES_WIDTH`).
const LIST: usize = 44;

fn mouse(state: &AppState, kind: MouseKind, column: u16, row: u16) -> (AppState, Vec<Command>) {
    update(
        state,
        AppEvent::Core(Event::Mouse(MouseInput {
            kind,
            column,
            row,
            modifiers: Modifiers::NONE,
        })),
    )
}

fn left_click(state: &AppState, row: u16) -> (AppState, Vec<Command>) {
    mouse(state, MouseKind::Down(MouseButton::Left), 10, row)
}

/// The screen row whose first [`LIST`] cells hold `words`, as drawn.
fn row_of(state: &AppState, words: &str) -> u16 {
    let drawn = crate::view::tests::draw(state, state.size.columns, state.size.rows);
    let at = drawn
        .lines()
        .position(|line| line.chars().take(LIST).collect::<String>().contains(words))
        .unwrap_or_else(|| panic!("no {words:?} in\n{drawn}"));
    u16::try_from(at).expect("a screen row")
}

/// The path under the file cursor.
fn cursor_path(state: &AppState) -> String {
    let editor = editor(state);
    editor.files[editor.file_cursor]
        .path
        .to_string_lossy()
        .into_owned()
}

fn asks_a_diff(commands: &[Command]) -> bool {
    commands
        .iter()
        .any(|command| matches!(command, Command::ReadFileDiff(..)))
}

#[test]
fn a_click_on_a_file_row_picks_it() {
    let open = opened(&mock_tree());
    assert_eq!(editor(&open).focus, CommitFocus::Subject);
    assert_eq!(cursor_path(&open), "docs/toc.md");

    let (picked, commands) = left_click(&open, row_of(&open, "src/a.rs"));
    assert_eq!(editor(&picked).focus, CommitFocus::Files);
    assert_eq!(cursor_path(&picked), "src/a.rs");
    assert!(asks_a_diff(&commands), "{commands:?}");

    // A file staying out, under its own heading: the list's order, not the
    // status's, names it.
    let (out, commands) = left_click(&picked, row_of(&picked, ".env"));
    assert_eq!(cursor_path(&out), ".env");
    assert!(asks_a_diff(&commands), "{commands:?}");

    // A heading is not a file: nothing moves, nothing is asked.
    for heading in ["Going in (3)", "Staying out (1)"] {
        let (after, commands) = left_click(&out, row_of(&out, heading));
        assert_eq!(cursor_path(&after), ".env", "{heading}");
        assert_eq!(editor(&after).focus, CommitFocus::Files, "{heading}");
        assert!(commands.is_empty(), "{heading}: {commands:?}");
    }
}

#[test]
fn clicks_in_the_message_move_focus() {
    let open = opened(&mock_tree());
    let (files, _) = walk(&open, vec![key(KeyCode::Tab), key(KeyCode::Tab)]);
    assert_eq!(editor(&files).focus, CommitFocus::Files);

    let (subject, commands) = left_click(&files, row_of(&files, "What did you change?"));
    assert_eq!(editor(&subject).focus, CommitFocus::Subject);
    assert!(commands.is_empty(), "{commands:?}");

    let (body, _) = left_click(&files, row_of(&files, "More detail, if it needs it"));
    assert_eq!(editor(&body).focus, CommitFocus::Body);
    // The file cursor stays where it was: the click moved focus only.
    assert_eq!(cursor_path(&body), cursor_path(&files));
}

/// Every mouse event while the screen is up is the screen's: the graph
/// behind it keeps its cursor, opens no menu, starts no drag, shows no
/// hover — whatever the button, wherever the pointer.
#[test]
fn nothing_reaches_the_graph_under_the_screen() {
    let mut behind = mock_tree();
    behind.history = Some(crate::view::tests::history_of(5, 2));
    let open = opened(&behind);
    assert!(open.commit_editor.is_some(), "the screen is open");
    let selected = |state: &AppState| {
        state
            .history
            .as_ref()
            .and_then(crate::history::History::selected)
            .map(|row| row.commit.id)
    };
    let left = MouseKind::Down(MouseButton::Left);
    // The rows the graph's own rows stand on behind the screen, the
    // header, the box's border, a heading, the empty list and the bar.
    let heading = row_of(&open, "Going in (3)");
    let mut events = Vec::new();
    for row in 0..=6 {
        for kind in [
            MouseKind::Down(MouseButton::Right),
            MouseKind::Drag(MouseButton::Left),
            MouseKind::Up(MouseButton::Left),
            MouseKind::ScrollDown,
            MouseKind::ScrollUp,
            MouseKind::Moved,
        ] {
            events.push((kind, row));
        }
    }
    for row in [0, 1, 6, heading, 30, 39] {
        events.push((left, row));
    }
    for (kind, row) in events {
        for column in [10, 80] {
            let (after, _) = mouse(&open, kind, column, row);
            let at = format!("{kind:?} at {column},{row}");
            assert_eq!(selected(&after), selected(&open), "{at}");
            assert!(after.context_menu.is_none(), "{at}");
            assert!(after.drag.is_none(), "{at}");
            assert_eq!(after.hover, open.hover, "{at}");
            assert!(after.commit_editor.is_some(), "{at}: still open");
            assert_eq!(editor(&after).focus, editor(&open).focus, "{at}");
            assert_eq!(cursor_path(&after), cursor_path(&open), "{at}");
        }
    }
}

#[test]
fn the_palette_over_the_screen_takes_the_click() {
    let open = opened(&mock_tree());
    let file_row = row_of(&open, "src/a.rs");
    let (palette, _) = update(&open, control(KeyCode::Char('p')));
    assert!(palette.palette.is_some(), "the palette is open");
    let (after, commands) = left_click(&palette, file_row);
    assert_eq!(
        cursor_path(&after),
        "docs/toc.md",
        "the file was not picked"
    );
    assert_eq!(editor(&after).focus, CommitFocus::Subject);
    assert!(!asks_a_diff(&commands), "{commands:?}");
}
