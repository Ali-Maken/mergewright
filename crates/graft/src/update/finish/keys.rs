//! The summary's keys (keymap §2.6b) and bracketed paste.

use super::{
    AppState, Command, FinishFocus, FinishState, JobId, KeyCode, KeyPress, Loading, PathBuf,
    Resume, ReturnTo, compose, fits, keep_cursor_on_screen, with_finish,
};

/// The summary's keys while it is open. `false` only for `Ctrl+C` (and
/// when it is closed): every other key is the summary's (§2.1).
pub fn finish_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(finish) = state.finish.as_ref() else {
        return false;
    };
    if key.code == KeyCode::Char('c') && key.modifiers.control {
        return false;
    }
    // The text on screen is what is being written: nothing changes it.
    if finish.committing.is_some() {
        return true;
    }
    // Below the widget's floor only one line is drawn: nothing on screen to
    // act on, so only `Esc` acts.
    if !fits(state) {
        if key.code == KeyCode::Escape && !key.modifiers.control && !key.modifiers.alt {
            escape(state);
        }
        return true;
    }
    if key.modifiers.control {
        control_key(state, commands, key, next_id);
        return true;
    }
    if key.modifiers.alt {
        return true;
    }
    match key.code {
        KeyCode::Escape => escape(state),
        // `Shift+Tab` arrives as its own code; P11 is not approved, so it
        // does nothing here.
        KeyCode::Tab if !key.modifiers.shift => with_finish(state, |finish| {
            finish.focus = finish.focus.next();
        }),
        _ => match finish.focus {
            FinishFocus::Files => files_key(state, commands, key, next_id),
            FinishFocus::Subject => subject_key(state, commands, key, next_id),
            FinishFocus::Body => body_key(state, key),
        },
    }
    true
}

/// `Ctrl`-held keys: they mean the same wherever the focus is.
pub(super) fn control_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) {
    match key.code {
        KeyCode::Enter => commit(state, commands, next_id),
        KeyCode::Char('e') => edit_outside(state, commands, next_id),
        // §2.10's word moves, in whichever field has focus.
        KeyCode::Left => with_finish(state, |finish| match finish.focus {
            FinishFocus::Subject => finish.subject.word_left(),
            FinishFocus::Body => finish.body.word_left(),
            FinishFocus::Files => {}
        }),
        KeyCode::Right => with_finish(state, |finish| match finish.focus {
            FinishFocus::Subject => finish.subject.word_right(),
            FinishFocus::Body => finish.body.word_right(),
            FinishFocus::Files => {}
        }),
        _ => {}
    }
}

/// Keys while the file lists have focus.
pub(super) fn files_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) {
    match key.code {
        KeyCode::Enter => commit(state, commands, next_id),
        KeyCode::Char('j') => move_cursor(state, 1),
        KeyCode::Char('k') => move_cursor(state, -1),
        KeyCode::Char(' ') => toggle_diff(state, commands, next_id),
        _ => {}
    }
}

/// Keys while the subject has focus: `⏎` commits (§2.6b).
pub(super) fn subject_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) {
    if key.code == KeyCode::Enter {
        commit(state, commands, next_id);
        return;
    }
    with_finish(state, |finish| {
        let field = &mut finish.subject;
        match key.code {
            KeyCode::Char(character) => field.insert(character),
            KeyCode::Backspace => field.backspace(),
            KeyCode::Delete => field.delete(),
            KeyCode::Left => field.left(),
            KeyCode::Right => field.right(),
            KeyCode::Home => field.home(),
            KeyCode::End => field.end(),
            _ => {}
        }
    });
}

/// Keys while the body has focus: `⏎` is a newline (§2.6b).
pub(super) fn body_key(state: &mut AppState, key: KeyPress) {
    with_finish(state, |finish| {
        let area = &mut finish.body;
        match key.code {
            KeyCode::Enter => area.newline(),
            KeyCode::Char(character) => area.insert(character),
            KeyCode::Backspace => area.backspace(),
            KeyCode::Delete => area.delete(),
            KeyCode::Left => area.left(),
            KeyCode::Right => area.right(),
            KeyCode::Up => area.up(),
            KeyCode::Down => area.down(),
            KeyCode::Home => area.home(),
            KeyCode::End => area.end(),
            _ => {}
        }
    });
}

/// `j`/`k`: the cursor over both lists, kept on screen.
pub(super) fn move_cursor(state: &mut AppState, by: isize) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    let rows = match &finish.summary {
        Loading::Ready(summary) => summary.by_you.len() + summary.by_git.len(),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => 0,
    };
    if rows == 0 {
        return;
    }
    finish.cursor = finish.cursor.saturating_add_signed(by).min(rows - 1);
    keep_cursor_on_screen(state);
}

/// The path the cursor row names, when the summary is read.
pub(super) fn row_path(finish: &FinishState, row: usize) -> Option<PathBuf> {
    let Loading::Ready(summary) = &finish.summary else {
        return None;
    };
    match summary.by_you.get(row) {
        Some(file) => Some(file.path.clone()),
        None => summary
            .by_git
            .get(row - summary.by_you.len())
            .map(|file| file.path.clone()),
    }
}

/// `space`: show the cursor row's staged diff under it, or hide it.
pub(super) fn toggle_diff(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    let row = finish.cursor;
    if finish.expanded.as_ref().is_some_and(|(at, _)| *at == row) {
        finish.expanded = None;
        finish.diff_job = None;
        return;
    }
    let Some(path) = row_path(finish, row) else {
        return;
    };
    let id = next_id();
    finish.expanded = Some((row, Loading::Pending(id)));
    finish.diff_job = Some(id);
    commands.push(Command::ReadFinishDiff(id, path));
    keep_cursor_on_screen(state);
}

/// `⏎` (files, subject) or `Ctrl+Enter`: commit.
pub(super) fn commit(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    let what = match &finish.summary {
        Loading::Idle | Loading::Pending(_) => {
            finish.notice = Some(("still reading · wait".to_owned(), false));
            return;
        }
        // Nothing to compare against: git's own message, as the line says.
        Loading::Failed(_) => Resume::Continue,
        Loading::Ready(summary) => {
            if finish.subject.text().trim().is_empty() {
                finish.notice = Some(("the message is empty · ⇥ to write one".to_owned(), true));
                return;
            }
            let message = compose(finish.subject.text(), &finish.body.text());
            if message.trim_end() == String::from_utf8_lossy(&summary.message).trim_end() {
                Resume::Continue
            } else {
                Resume::ContinueWith {
                    message: message.into_bytes(),
                }
            }
        }
    };
    if crate::update::operate::operation_in_flight(state) {
        with_finish(state, |finish| {
            finish.notice = Some(("a write is already running".to_owned(), true));
        });
        return;
    }
    if crate::update::banner::resume(state, commands, what, next_id) {
        let operating = state.operating;
        with_finish(state, |finish| {
            finish.committing = operating;
            finish.notice = None;
        });
    } else {
        with_finish(state, |finish| {
            finish.notice = Some(("no merge is in progress".to_owned(), true));
        });
    }
}

/// `Ctrl+E`: the whole message to `$EDITOR`, the way the commit editor
/// hands it over.
pub(super) fn edit_outside(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    let id = next_id();
    let message = compose(finish.subject.text(), &finish.body.text());
    finish.editing = Some(id);
    state.external_editor = Some(id);
    commands.push(Command::EditCommitMessage { id, message });
}

/// `Esc`: close. What the user typed goes to `finish_draft`; from the
/// banner, the banner gets the keyboard back.
pub(super) fn escape(state: &mut AppState) {
    let back_to_banner = state
        .finish
        .as_ref()
        .is_some_and(|finish| finish.return_to == ReturnTo::Banner);
    close_keeping_draft(state);
    if back_to_banner {
        state.banner_focus = true;
    }
}

/// Close the summary, keeping the message as `finish_draft` when it is the
/// user's — changed from git's, or typed before git's arrived.
pub(super) fn close_keeping_draft(state: &mut AppState) {
    let Some(finish) = state.finish.take() else {
        return;
    };
    // Whatever `$EDITOR` is about to hand back belongs to a summary that is
    // no longer on screen.
    if finish.editing.is_some() {
        state.external_editor = None;
    }
    let body = finish.body.text();
    let composed = compose(finish.subject.text(), &body);
    let the_users = match &finish.summary {
        Loading::Ready(summary) => {
            composed.trim_end() != String::from_utf8_lossy(&summary.message).trim_end()
        }
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => {
            !finish.subject.is_empty() || !finish.body.is_empty()
        }
    };
    state.finish_draft = the_users.then(|| (finish.subject.text().to_owned(), body));
}

/// A bracketed paste, into the focused field. `true` whenever the summary
/// is open: it is modal, so a paste is meant for it even with the file
/// lists focused and nowhere to put the text.
pub(crate) fn paste(state: &mut AppState, text: &str) -> bool {
    let Some(finish) = state.finish.as_mut() else {
        return false;
    };
    if finish.committing.is_some() {
        return true;
    }
    match finish.focus {
        FinishFocus::Subject => finish.subject.insert_str(text),
        FinishFocus::Body => finish.body.insert_str(text),
        FinishFocus::Files => {}
    }
    true
}
