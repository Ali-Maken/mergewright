//! The editor's keys (keymap §2.7) and its mouse. Modal: while it is up
//! every key but `Ctrl+C` is its own (§2.1); while its `r` field is open,
//! every key is typed (`text_field_open`).

use graft_core::event::{KeyCode, KeyPress, MouseInput};
use graft_core::job::JobId;
use graft_git::diff::{DiffOptions, DiffSpec};
use graft_ops::rewrite::TodoAction;
use graft_ui::text_input::TextField;

use super::{Field, RewriteEditor, Right, after, words};
use crate::update::detail::Loading;
use crate::update::events::Command;
use crate::update::state::AppState;

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

/// The mouse while the editor is up: every event is its own, so nothing
/// reaches the graph behind it. Selecting and dragging rows need the drawn
/// layout's hit test (`graft_ui::rewrite_row_at`, 08-b).
pub(crate) fn rewrite_mouse(
    state: &mut AppState,
    _commands: &mut Vec<Command>,
    _mouse: MouseInput,
    _next_id: &mut impl FnMut() -> JobId,
) -> bool {
    state.rewrite.editor.is_some()
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
        KeyCode::Char('r') => open_field(editor),
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
    // His own mark now, not the one its name gave it.
    editor.by_name.remove(&commit);
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
    let Some(index) = editor.cursor_row() else {
        editor.notice = Some(words::BASE_STAYS.to_owned());
        return;
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
        return;
    };
    editor.todo.swap(index, other);
    editor.moved.insert(editor.todo[other].commit);
    editor.cursor = editor.todo.len() - 1 - other;
    after::replan(state, editor, commands, next_id);
}

/// `r`: the message field under the row, prefilled with its subject line.
/// A melded row's message is its kept row's: `r` there says so.
fn open_field(editor: &mut RewriteEditor) {
    let Some(index) = editor.cursor_row() else {
        editor.notice = Some(words::BASE_STAYS.to_owned());
        return;
    };
    let row = &editor.todo[index];
    if row.action.melds() {
        editor.notice = Some(match editor.melds_into(index) {
            Some(kept) => words::melds_into(&editor.todo[kept].subject),
            None => graft_ops::rewrite::TodoProblem::MeldsIntoNothing { row: index }.explain(),
        });
        return;
    }
    let commit = row.commit;
    let Some(message) = words::message_of(editor, index) else {
        return;
    };
    let subject = message.lines().next().unwrap_or_default();
    editor.field = Some(Field {
        commit,
        text: TextField::new(subject),
        handover: None,
    });
}

/// A key in the field: typed, except `⏎` (keep), `Esc` (drop the edit)
/// and `Ctrl+E` (`$EDITOR`).
fn field_key(
    state: &mut AppState,
    editor: &mut RewriteEditor,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Then {
    if key.modifiers.control {
        if key.code == KeyCode::Char('e')
            && let Some(message) = field_message(editor)
            && let Some(field) = editor.field.as_mut()
        {
            let id = next_id();
            field.handover = Some(id);
            commands.push(Command::EditCommitMessage { id, message });
        }
        return Then::Stays;
    }
    if key.modifiers.alt {
        return Then::Stays;
    }
    match key.code {
        KeyCode::Escape => editor.field = None,
        KeyCode::Enter => match field_message(editor) {
            Some(message) => keep_message(state, editor, message, commands, next_id),
            // The field's row is gone from the todo, which nothing while
            // the field is open can do; there is nothing to keep it on.
            None => editor.field = None,
        },
        code => {
            if let Some(field) = editor.field.as_mut() {
                edit(&mut field.text, code);
            }
        }
    }
    Then::Stays
}

/// One editing key on the field, as the stash prompt's (`stash::edit`).
fn edit(field: &mut TextField, code: KeyCode) {
    match code {
        KeyCode::Char(character) => field.insert(character),
        KeyCode::Backspace => field.backspace(),
        KeyCode::Delete => field.delete(),
        KeyCode::Left => field.left(),
        KeyCode::Right => field.right(),
        KeyCode::Home => field.home(),
        KeyCode::End => field.end(),
        _ => {}
    }
}

/// The whole message the field stands for: its row's message with the
/// typed subject line in place of the first. `None` with no field, or one
/// whose row is not in the todo.
fn field_message(editor: &RewriteEditor) -> Option<String> {
    let field = editor.field.as_ref()?;
    let index = editor
        .todo
        .iter()
        .position(|row| row.commit == field.commit)?;
    Some(words::with_subject(
        &words::message_of(editor, index)?,
        field.text.text(),
    ))
}

/// The field's message kept (`⏎`, or `$EDITOR`'s answer): the kept row of
/// a squash group keeps its action and gets the group's message; any other
/// row becomes a `reword`. The field closes and the AFTER plan is asked
/// again.
pub(super) fn keep_message(
    state: &mut AppState,
    editor: &mut RewriteEditor,
    message: String,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(field) = editor.field.take() else {
        return;
    };
    let Some(index) = editor
        .todo
        .iter()
        .position(|row| row.commit == field.commit)
    else {
        return;
    };
    if editor.group_of(index).is_none() {
        editor.todo[index].action = TodoAction::Reword;
        editor.by_name.remove(&field.commit);
    }
    editor.edits.insert(field.commit, message);
    after::replan(state, editor, commands, next_id);
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
