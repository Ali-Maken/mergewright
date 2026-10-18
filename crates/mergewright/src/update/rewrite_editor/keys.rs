//! The editor's keys (keymap §2.7) and its mouse. Modal: while it is up
//! every key but `Ctrl+C` is its own (§2.1); while its `r` field is open,
//! every key is typed (`text_field_open`).

use mergewright_core::event::{KeyCode, KeyPress, MouseButton, MouseInput, MouseKind};
use mergewright_core::job::JobId;
use mergewright_git::diff::{DiffOptions, DiffSpec};
use mergewright_ops::rewrite::TodoAction;
use mergewright_ui::text_input::TextField;

use super::{Field, RewriteEditor, Right, after, words};
use crate::update::detail::Loading;
use crate::update::events::Command;
use crate::update::state::AppState;

mod field;
pub(super) use field::keep_message;
use field::{field_key, open_field};

/// Below this width the right-hand side is a card `Tab` swaps to (R1's
/// breakpoints).
const NARROW: u16 = 120;

/// What a key leaves the editor as.
enum Then {
    /// Still up.
    Stays,
    /// `Esc`: closed, nothing written.
    Closes,
    /// `y`: the rewrite is asked to start.
    Starts,
}

/// The editor's turn at a key, in `keys::key_event`'s modal chain after the
/// boxes. `true` when it took the key: every key but `Ctrl+C` while the
/// editor is up, and `Esc` while its read is out (which cancels it).
pub(crate) fn rewrite_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if key.code == KeyCode::Char('c') && key.modifiers.control {
        return false;
    }
    let Some(mut editor) = state.rewrite.editor.take() else {
        return key.code == KeyCode::Escape
            && !key.modifiers.control
            && !key.modifiers.alt
            && super::cancel_opening(state, commands);
    };
    // The terminal is with `$EDITOR`: nothing typed here is his to act on.
    if editor
        .field
        .as_ref()
        .is_some_and(|field| field.handover.is_some())
    {
        state.rewrite.editor = Some(editor);
        return true;
    }
    editor.notice = None;
    let then = if editor.field.is_some() {
        field_key(state, &mut editor, commands, key, next_id)
    } else {
        row_key(state, &mut editor, commands, key, next_id)
    };
    match then {
        Then::Stays => state.rewrite.editor = Some(editor),
        Then::Closes => {
            state.rewrite.editor = Some(editor);
            super::close(state, commands);
        }
        Then::Starts => after::start(state, editor, commands, next_id),
    }
    true
}

/// The mouse while the editor is up (§2.7): every event is its own, so
/// nothing reaches the graph behind it. A left click on a row selects it; a
/// drag from it moves it, one row per row crossed, as `Alt+↑/↓` would. Where
/// a row is drawn is asked of the drawn layout
/// (`mergewright_ui::rewrite_editor::rewrite_row_at` over
/// `view::rewrite_editor::editor_area`), so a click cannot land on a row
/// other than the one under it.
pub(crate) fn rewrite_mouse(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    mouse: MouseInput,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(editor) = state.rewrite.editor.as_ref() else {
        return false;
    };
    // The field and the `$EDITOR` handover own the keyboard; a click on a
    // row would change the row under an open edit.
    if editor.field.is_some() {
        return true;
    }
    let hit = crate::view::rewrite_editor::rewrite_view(state, false).and_then(|view| {
        mergewright_ui::rewrite_editor::rewrite_row_at(
            crate::view::rewrite_editor::editor_area(state),
            &view,
            mouse.column,
            mouse.row,
        )
    });
    let Some(mut editor) = state.rewrite.editor.take() else {
        return true;
    };
    match mouse.kind {
        MouseKind::Down(MouseButton::Left) => {
            editor.notice = None;
            editor.dragging = hit.is_some();
            if let Some(row) = hit {
                editor.cursor = row;
            }
        }
        MouseKind::Drag(MouseButton::Left) if editor.dragging => {
            if let Some(to) = hit {
                drag_to(state, &mut editor, commands, to, next_id);
            }
        }
        MouseKind::Up(MouseButton::Left) => editor.dragging = false,
        _ => {}
    }
    state.rewrite.editor = Some(editor);
    true
}

/// The dragged row (the cursor's) moves toward screen row `to`, one step per
/// row, until it is there or a move refuses (the base stays).
fn drag_to(
    state: &mut AppState,
    editor: &mut RewriteEditor,
    commands: &mut Vec<Command>,
    to: usize,
    next_id: &mut impl FnMut() -> JobId,
) {
    // One step at a time so the base and the ends refuse as `Alt+↑/↓` do,
    // but one AFTER plan for the whole drag, not one per row crossed.
    let mut moved = false;
    while editor.cursor != to && editor.notice.is_none() && swap_row(editor, to < editor.cursor) {
        moved = true;
    }
    if moved {
        after::replan(state, editor, commands, next_id);
    }
}

/// A key on the rows.
fn row_key(
    state: &mut AppState,
    editor: &mut RewriteEditor,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Then {
    if key.modifiers.control {
        return Then::Stays;
    }
    if key.modifiers.alt {
        match key.code {
            KeyCode::Up => move_row(state, editor, commands, true, next_id),
            KeyCode::Down => move_row(state, editor, commands, false, next_id),
            _ => {}
        }
        return Then::Stays;
    }
    match key.code {
        KeyCode::Escape => return Then::Closes,
        KeyCode::Char('y') => return Then::Starts,
        KeyCode::Char('j') | KeyCode::Down => {
            editor.cursor = (editor.cursor + 1).min(editor.todo.len());
        }
        KeyCode::Char('k') | KeyCode::Up => editor.cursor = editor.cursor.saturating_sub(1),
        KeyCode::Char(letter @ ('p' | 's' | 'f' | 'd' | 'e')) => {
            mark(state, editor, commands, letter, next_id);
        }
        KeyCode::Char('r') => open_field(editor, &state.keymap),
        KeyCode::Enter => toggle_changes(state, editor, commands, next_id),
        // ≥120 the AFTER side is always on screen: `Tab` is swallowed below.
        KeyCode::Tab if state.size.columns < NARROW => {
            editor.narrow_shows_right = !editor.narrow_shows_right;
        }
        _ => {}
    }
    Then::Stays
}

/// `p s f d e`: the action, then one row older. `p` on a marked row
/// un-marks it.
fn mark(
    state: &mut AppState,
    editor: &mut RewriteEditor,
    commands: &mut Vec<Command>,
    letter: char,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(index) = editor.cursor_row() else {
        editor.notice = Some(words::BASE_STAYS.to_owned());
        return;
    };
    let Some(action) = TodoAction::from_word(&letter.to_string()) else {
        return;
    };
    let row = &mut editor.todo[index];
    row.action = action;
    let commit = row.commit;
    // His own mark now, not the one it came up with (its name, or the
    // rebase's suggested drop).
    editor.marked_by.remove(&commit);
    editor.cursor = (editor.cursor + 1).min(editor.todo.len().saturating_sub(1));
    after::replan(state, editor, commands, next_id);
}

/// `Alt+↑`/`Alt+↓`: the row moves one newer or older; the cursor follows.
fn move_row(
    state: &mut AppState,
    editor: &mut RewriteEditor,
    commands: &mut Vec<Command>,
    newer: bool,
    next_id: &mut impl FnMut() -> JobId,
) {
    if swap_row(editor, newer) {
        after::replan(state, editor, commands, next_id);
    }
}

/// Swap the cursor's row one place newer or older, the cursor following
/// it; `false` (and, toward the base, its notice) when it cannot move.
fn swap_row(editor: &mut RewriteEditor, newer: bool) -> bool {
    let Some(index) = editor.cursor_row() else {
        editor.notice = Some(words::BASE_STAYS.to_owned());
        return false;
    };
    // Newer is later in the todo; the base below stays where it is.
    let Some(other) = (if newer {
        (index + 1 < editor.todo.len()).then_some(index + 1)
    } else {
        index.checked_sub(1)
    }) else {
        if !newer {
            editor.notice = Some(words::BASE_STAYS.to_owned());
        }
        return false;
    };
    editor.todo.swap(index, other);
    editor.moved.insert(editor.todo[other].commit);
    editor.cursor = editor.todo.len() - 1 - other;
    true
}

/// `⏎`: the row's changes on the right-hand side, or back to AFTER.
fn toggle_changes(
    state: &mut AppState,
    editor: &mut RewriteEditor,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let commit = match editor.cursor_row() {
        Some(index) => editor.todo[index].commit,
        None => editor.base.oid,
    };
    let showing = match &editor.right {
        Right::Changes { commit: shown, .. } => Some(*shown),
        Right::After => None,
    };
    if let Right::Changes {
        files: Loading::Pending(job),
        ..
    } = &editor.right
    {
        commands.push(Command::CancelJob(*job));
        state.rewrite.stale.insert(*job);
    }
    if showing == Some(commit) {
        editor.right = Right::After;
        return;
    }
    let id = next_id();
    editor.right = Right::Changes {
        commit,
        files: Loading::Pending(id),
    };
    commands.push(Command::ListFiles(
        id,
        DiffSpec::Commit {
            commit,
            against: None,
        },
        DiffOptions::default(),
    ));
}
