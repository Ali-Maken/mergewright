//! The reword field's keys, and the message it keeps.

use super::*;

/// `r`: the message field under the row, prefilled with its subject line.
/// A melded row's message is its kept row's: `r` there says so.
pub(super) fn open_field(editor: &mut RewriteEditor) {
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
pub(super) fn field_key(
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
pub(super) fn edit(field: &mut TextField, code: KeyCode) {
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
pub(super) fn field_message(editor: &RewriteEditor) -> Option<String> {
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
pub(in crate::update::rewrite_editor) fn keep_message(
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
        editor.marked_by.remove(&field.commit);
    }
    editor.edits.insert(field.commit, message);
    after::replan(state, editor, commands, next_id);
}
