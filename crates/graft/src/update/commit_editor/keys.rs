//! The editor's keys: subject, body, options, buttons, and the commit they lead to.

use super::{
    AppState, CO_AUTHOR, Command, CommitEditor, Field, JobId, KeyCode, KeyPress, Status,
    live_operation, refusal_way,
};

/// The editor's keys while it is open. `None` means "not the editor's" — and
/// the editor claims every key but `Ctrl+C`, which still quits (DESIGN.md
/// §2.2: a terminal application that ignores it feels broken).
pub fn editor_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Option<Vec<Command>> {
    state.commit_editor.as_ref()?;
    if key.code == KeyCode::Char('c') && key.modifiers.control {
        return None;
    }
    // While the commit runs the screen is not the user's to change: the text
    // on it is what is being written. `Esc` cancels the job.
    if state.operating.is_some() {
        let mut commands = Vec::new();
        if key.code == KeyCode::Escape {
            // The same stop every write gets (m4-remotes/05): the line says
            // `cancelling`, and the cancelled answer reads `cancelled`.
            let _stopping: bool = crate::update::remote::stop_write(state, &mut commands);
        }
        return Some(commands);
    }
    Some(open_editor_key(state, key, next_id))
}

/// [`editor_key`] once the editor is known open and idle.
pub(super) fn open_editor_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    // Whole-editor keys first: they mean the same wherever the focus is.
    if key.modifiers.control {
        return control_key(state, key, next_id);
    }
    match key.code {
        KeyCode::Escape => {
            // The same rule the graph screen follows (`keys.rs`): `Esc`
            // dismisses a failure before it means anything else. The editor
            // draws the reason on its footer, so without this the only way
            // to read a rejected commit's message and then get rid of it
            // would be to close the editor (m3/05 review, Major 1).
            if matches!(state.status, Status::Failed(_) | Status::Sticky { .. }) {
                state.status = Status::Ready;
                return Vec::new();
            }
            close(state);
            return Vec::new();
        }
        KeyCode::Tab => {
            with_editor(state, |editor| editor.focus = editor.focus.next());
            return Vec::new();
        }
        KeyCode::BackTab => {
            with_editor(state, |editor| editor.focus = editor.focus.previous());
            return Vec::new();
        }
        _ => {}
    }
    let focus = state
        .commit_editor
        .as_ref()
        .map_or(Field::Subject, |editor| editor.focus);
    match focus {
        Field::Subject => subject_key(state, key, next_id),
        Field::Body => body_key(state, key),
        Field::CommitButton => button_key(state, key, next_id),
        option => option_key(state, key, option),
    }
}

/// `Ctrl`-held keys: they mean the same wherever the focus is.
pub(super) fn control_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match key.code {
        // ADR-0034: `Ctrl+Enter` is indistinguishable from `Enter` outside
        // the kitty keyboard protocol, so it is a shortcut for the button
        // and never the only way in.
        KeyCode::Enter => return commit(state, next_id),
        KeyCode::Char('s') => {
            with_editor(state, |editor| editor.sign_off = !editor.sign_off);
        }
        // Shift is already folded into the character, so this is `Ctrl+Shift+C`.
        KeyCode::Char('C') => {
            with_editor(state, |editor| {
                editor.body.append_line(CO_AUTHOR);
                editor.focus = Field::Body;
            });
        }
        KeyCode::Char('e') => return edit_outside(state, next_id),
        KeyCode::Left => with_editor(state, |editor| match editor.focus {
            Field::Body => editor.body.word_left(),
            _ => editor.subject.word_left(),
        }),
        KeyCode::Right => with_editor(state, |editor| match editor.focus {
            Field::Body => editor.body.word_right(),
            _ => editor.subject.word_right(),
        }),
        _ => {}
    }
    Vec::new()
}

/// `Ctrl+E`: hand the message to `$EDITOR`. Which file it is written to is
/// the loop's to decide — `update` knows no repository directory — so the
/// command carries only the text.
pub(super) fn edit_outside(
    state: &mut AppState,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let Some(editor) = state.commit_editor.as_ref() else {
        return Vec::new();
    };
    let id = next_id();
    let message = editor.message();
    state.external_editor = Some(id);
    vec![Command::EditCommitMessage { id, message }]
}

/// Run `f` over the open editor; a no-op when there is none.
/// A bracketed paste, into whichever text field has focus.
///
/// `true` when the editor took it, which it does whenever it is open: the
/// editor is modal, so a paste arriving while it is up is meant for it even
/// when the focus is on a toggle and there is nowhere to put the text.
///
/// The fields do the deciding about newlines — the subject drops them
/// because it is one line, the body keeps them — so pasting a whole commit
/// message into the body arrives with its paragraphs intact.
///
/// Graft turns bracketed paste *on*, so before this the user's paste into
/// the editor did nothing whatsoever (m3/05 review, Major 3).
pub fn paste(state: &mut AppState, text: &str) -> bool {
    if state.commit_editor.is_none() || state.operating.is_some() {
        return false;
    }
    with_editor(state, |editor| match editor.focus {
        Field::Subject => {
            editor.subject.insert_str(text);
            editor.subject_refused = false;
        }
        Field::Body => editor.body.insert_str(text),
        // A toggle or the button has focus: there is no text field to put
        // this in. Claimed all the same, so it cannot fall through to a
        // screen the user cannot see.
        Field::Amend
        | Field::SignOff
        | Field::NoVerify
        | Field::AllowEmpty
        | Field::CommitButton => {}
    });
    true
}

pub(super) fn with_editor(state: &mut AppState, f: impl FnOnce(&mut CommitEditor)) {
    if let Some(editor) = state.commit_editor.as_mut() {
        f(editor);
    }
}

/// `Esc`: close, keeping a non-empty draft for the next `c` this session —
/// or, for a reword editor, for the next `a`: its mode describes `HEAD` as
/// it was when `a` was pressed, and `c` is never a reword (review m8-14
/// Major 1).
pub(super) fn close(state: &mut AppState) {
    // Whatever `$EDITOR` is about to hand back belongs to an editor that is
    // no longer on screen.
    state.external_editor = None;
    let Some(editor) = state.commit_editor.take() else {
        return;
    };
    if editor.reword_line {
        crate::update::amend_into::keep_reword(state, editor);
        return;
    }
    state.commit_draft = (!editor.is_empty()).then_some(editor);
}

/// The editor closed because its commit landed: the draft goes too, or the
/// next `c` would reopen a message that is now a commit.
pub fn committed(state: &mut AppState) {
    state.commit_editor = None;
    state.commit_draft = None;
    state.external_editor = None;
}

/// Keys while the subject has focus. `Enter` commits, exactly as `Ctrl+Enter`
/// and the button do (m6 usability P16, ADR-0044 §3): most commits are one
/// line, and the finish summary's subject already commits on `Enter`. The
/// same refusals hold — a live operation, nothing staged, an empty subject —
/// and a refused press leaves the focus where it is. `Down` and `Tab` still
/// reach the body.
pub(super) fn subject_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if key.code == KeyCode::Enter {
        return commit(state, next_id);
    }
    with_editor(state, |editor| match key.code {
        // Nothing is below the subject but the body.
        KeyCode::Down => editor.focus = Field::Body,
        KeyCode::Char(character) => {
            editor.subject.insert(character);
            editor.subject_refused = false;
        }
        KeyCode::Backspace => {
            editor.subject.backspace();
            editor.subject_refused = false;
        }
        KeyCode::Delete => {
            editor.subject.delete();
            editor.subject_refused = false;
        }
        KeyCode::Left => editor.subject.left(),
        KeyCode::Right => editor.subject.right(),
        KeyCode::Home => editor.subject.home(),
        KeyCode::End => editor.subject.end(),
        _ => {}
    });
    Vec::new()
}

/// Keys while the body has focus.
pub(super) fn body_key(state: &mut AppState, key: &KeyPress) -> Vec<Command> {
    with_editor(state, |editor| match key.code {
        KeyCode::Enter => editor.body.newline(),
        KeyCode::Char(character) => editor.body.insert(character),
        KeyCode::Backspace => editor.body.backspace(),
        KeyCode::Delete => editor.body.delete(),
        KeyCode::Left => editor.body.left(),
        KeyCode::Right => editor.body.right(),
        KeyCode::Up => {
            if editor.body.cursor().0 == 0 {
                editor.focus = Field::Subject;
            } else {
                editor.body.up();
            }
        }
        KeyCode::Down => editor.body.down(),
        KeyCode::Home => editor.body.home(),
        KeyCode::End => editor.body.end(),
        _ => {}
    });
    Vec::new()
}

/// Keys while one of the four toggles has focus.
pub(super) fn option_key(state: &mut AppState, key: &KeyPress, option: Field) -> Vec<Command> {
    match key.code {
        KeyCode::Char(' ') | KeyCode::Enter => {
            with_editor(state, |editor| {
                if let Some(flag) = editor.option_mut(option) {
                    *flag = !*flag;
                }
            });
        }
        KeyCode::Left | KeyCode::Right => {
            let forward = key.code == KeyCode::Right;
            with_editor(state, |editor| {
                if let Some(next) = option.sibling_option(forward) {
                    editor.focus = next;
                }
            });
        }
        KeyCode::Up => with_editor(state, |editor| editor.focus = Field::Body),
        KeyCode::Down => with_editor(state, |editor| editor.focus = Field::CommitButton),
        _ => {}
    }
    Vec::new()
}

/// Keys while `[ Commit ]` has focus.
pub(super) fn button_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match key.code {
        KeyCode::Enter | KeyCode::Char(' ') => commit(state, next_id),
        KeyCode::Up => {
            with_editor(state, |editor| editor.focus = Field::Amend);
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Dispatch the commit, unless the preview already knows it would be
/// refused — in which case the reason is already beside the button and on the
/// title row, and pressing it again says nothing new.
pub(super) fn commit(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
    let status = state.working_tree.as_deref();
    let in_progress = live_operation(state);
    let tabs = refusal_way(state);
    let Some(editor) = state.commit_editor.as_mut() else {
        return Vec::new();
    };
    if editor.refusal(status, in_progress, tabs).is_some() {
        // From now on an empty subject is said (m7-power-tools/15, :776).
        editor.subject_refused = true;
        return Vec::new();
    }
    if editor.reword_line {
        // A line is built on `HEAD`: the reword is a rewrite, planned and
        // previewed like an amend with dependents (m8-polish-and-ship/14).
        // The editor goes with it, kept until the reword lands.
        let mut commands = Vec::new();
        if let Some(editor) = state.commit_editor.take() {
            crate::update::amend_into::reword_confirmed(state, &mut commands, editor, next_id);
        }
        return commands;
    }
    let operation = editor.operation();
    let mut commands = Vec::new();
    // Unreachable `Err`: `editor_key` answers no key but `Esc` while a
    // write runs, so `commit` is never reached then.
    let launched = crate::update::operate::launch(state, &mut commands, operation, next_id);
    crate::update::operate::say_if_refused(state, launched);
    commands
}
