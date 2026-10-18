//! The screen's keys: the message, the file list, the options, and the
//! commit they lead to.

use super::{
    AppState, CO_AUTHOR, Command, CommitEditor, CommitFocus, CommitOption, JobId, KeyCode,
    KeyPress, Status, WayIn, files, live_operation,
};

/// The screen's keys while it is open. `None` means "not the screen's" —
/// and it claims every key but `Ctrl+C`, which still quits (DESIGN.md
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

/// [`editor_key`] once the screen is known open and idle.
fn open_editor_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if let Some(commands) = opens_the_palette(state, key) {
        return commands;
    }
    // Whole-screen keys first: they mean the same wherever the focus is.
    if key.modifiers.control {
        return control_key(state, key, next_id);
    }
    match key.code {
        KeyCode::Escape => {
            escape(state);
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
        .map_or(CommitFocus::Subject, |editor| editor.focus);
    match focus {
        CommitFocus::Subject => subject_key(state, key, next_id),
        CommitFocus::Body => body_key(state, key),
        CommitFocus::Files => files_key(state, key, next_id),
    }
}

/// `esc`, one step at a time: a failure on the bar row goes first (the
/// graph's rule, m3/05 review Major 1); then the body or the file list
/// hands back to the message line; from the message line the screen closes,
/// its words kept (C15).
fn escape(state: &mut AppState) {
    if matches!(state.status, Status::Failed(_) | Status::Sticky { .. }) {
        state.status = Status::Ready;
        return;
    }
    let focus = state.commit_editor.as_ref().map(|editor| editor.focus);
    match focus {
        Some(CommitFocus::Body | CommitFocus::Files) => {
            with_editor(state, |editor| editor.focus = CommitFocus::Subject);
        }
        Some(CommitFocus::Subject) => close(state),
        None => {}
    }
}

/// The palette from the commit screen (ease-pass/03b, Q8 a): `⌃P` from
/// every focus, `?` from the file list — in the subject and the body `?`
/// is typed. Opened on `CommitEditor`, so it lists the screen's own rows,
/// the keyless options among them. `None`: not a key that opens it here.
fn opens_the_palette(state: &mut AppState, key: &KeyPress) -> Option<Vec<Command>> {
    use crate::update::palette::{Opens, open, opener};
    let types = state
        .commit_editor
        .as_ref()
        .is_some_and(|editor| editor.focus.types());
    let how = opener(*key)?;
    let opens = match how {
        Opens::Everything => key.modifiers.control,
        Opens::Here => !types,
    };
    if !opens {
        return None;
    }
    let mut commands = Vec::new();
    open(state, &mut commands, how);
    Some(commands)
}

/// A keyless `commit.toggle_*` row chosen in the palette (ease-pass/03b):
/// flips that option, focus kept; the header says it while on (Q8 a).
/// `false` when `id` is not one of the three or no screen is open.
pub(crate) fn toggle_from_palette(state: &mut AppState, id: mergewright_ui::keymap::ActionId) -> bool {
    let Some(option) = CommitOption::of_id(&id) else {
        return false;
    };
    let Some(editor) = state.commit_editor.as_mut() else {
        return false;
    };
    let flag = editor.option_mut(option);
    *flag = !*flag;
    true
}

/// `Ctrl`-held keys: they mean the same wherever the focus is.
fn control_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match key.code {
        // ADR-0034: `Ctrl+Enter` is indistinguishable from `Enter` outside
        // the kitty keyboard protocol, so it is a shortcut and never the
        // only way in.
        KeyCode::Enter => return commit(state, next_id),
        KeyCode::Char('s') => {
            with_editor(state, |editor| editor.sign_off = !editor.sign_off);
        }
        // Shift is already folded into the character, so this is `Ctrl+Shift+C`.
        KeyCode::Char('C') => {
            with_editor(state, |editor| {
                editor.body.append_line(CO_AUTHOR);
                editor.focus = CommitFocus::Body;
            });
        }
        KeyCode::Char('e') => return edit_outside(state, next_id),
        KeyCode::Left => with_editor(state, |editor| match editor.focus {
            CommitFocus::Body => editor.body.word_left(),
            CommitFocus::Subject => editor.subject.word_left(),
            CommitFocus::Files => {}
        }),
        KeyCode::Right => with_editor(state, |editor| match editor.focus {
            CommitFocus::Body => editor.body.word_right(),
            CommitFocus::Subject => editor.subject.word_right(),
            CommitFocus::Files => {}
        }),
        _ => {}
    }
    Vec::new()
}

/// `Ctrl+E`: hand the message to `$EDITOR`. Which file it is written to is
/// the loop's to decide — `update` knows no repository directory — so the
/// command carries only the text.
fn edit_outside(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
    let Some(editor) = state.commit_editor.as_ref() else {
        return Vec::new();
    };
    let id = next_id();
    let message = editor.message();
    state.external_editor = Some(id);
    vec![Command::EditCommitMessage { id, message }]
}

/// A bracketed paste, into whichever text field has focus.
///
/// `true` when the screen took it, which it does whenever it is open: the
/// screen is modal, so a paste arriving while it is up is meant for it even
/// when the file list has focus and there is nowhere to put the text.
///
/// The fields do the deciding about newlines — the subject drops them
/// because it is one line, the body keeps them — so pasting a whole commit
/// message into the body arrives with its paragraphs intact.
pub fn paste(state: &mut AppState, text: &str) -> bool {
    if state.commit_editor.is_none() || state.operating.is_some() {
        return false;
    }
    with_editor(state, |editor| match editor.focus {
        CommitFocus::Subject => editor.subject.insert_str(text),
        CommitFocus::Body => editor.body.insert_str(text),
        // No text field to put this in. Claimed all the same, so it cannot
        // fall through to a screen the user cannot see.
        CommitFocus::Files => {}
    });
    true
}

/// Run `f` over the open screen; a no-op when there is none.
fn with_editor(state: &mut AppState, f: impl FnOnce(&mut CommitEditor)) {
    if let Some(editor) = state.commit_editor.as_mut() {
        f(editor);
    }
}

/// Close, writing only this screen's own slot (C15, 08s M1): a `c` screen
/// keeps its words for the next `c`, none when empty; an `a` screen keeps
/// its words for the next `a` only when they differ from `HEAD`'s message
/// it prefilled. A reword screen has its own keep: its mode describes
/// `HEAD` as it was when `a` was pressed, and `c` is never a reword (review
/// m8-14 Major 1).
fn close(state: &mut AppState) {
    // Whatever `$EDITOR` is about to hand back belongs to a screen that is
    // no longer up.
    state.external_editor = None;
    let Some(editor) = state.commit_editor.take() else {
        return;
    };
    if editor.reword_line {
        crate::update::amend_into::keep_reword(state, *editor);
        return;
    }
    let kept = editor.words_typed().then(|| Box::new(editor.draft()));
    match editor.way_in {
        WayIn::Commit => state.drafts.commit = kept,
        WayIn::Amend => state.drafts.amend = kept,
    }
}

/// The screen's own write landed: it closes, and nothing is written back —
/// the words are in the commit now. Its slot was taken when it opened; the
/// other way in's words stay where they are (08s M1).
pub fn committed(state: &mut AppState) {
    state.commit_editor = None;
    state.external_editor = None;
}

/// Keys while the subject has focus. `⏎` commits (ADR-0044 §3: most
/// commits are one line); `↑`/`↓` move over the files so a file's changes
/// can be seen without leaving the line (the mock's bar).
fn subject_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let mut commands = Vec::new();
    match key.code {
        KeyCode::Enter => return commit(state, next_id),
        KeyCode::Down | KeyCode::Up => {
            files::move_cursor(state, &mut commands, key.code == KeyCode::Down, next_id);
        }
        _ => with_editor(state, |editor| match key.code {
            KeyCode::Char(character) => editor.subject.insert(character),
            KeyCode::Backspace => editor.subject.backspace(),
            KeyCode::Delete => editor.subject.delete(),
            KeyCode::Left => editor.subject.left(),
            KeyCode::Right => editor.subject.right(),
            KeyCode::Home => editor.subject.home(),
            KeyCode::End => editor.subject.end(),
            _ => {}
        }),
    }
    commands
}

/// Keys while the body has focus: a text area; `esc` and `⇥` leave it.
fn body_key(state: &mut AppState, key: &KeyPress) -> Vec<Command> {
    with_editor(state, |editor| match key.code {
        KeyCode::Enter => editor.body.newline(),
        KeyCode::Char(character) => editor.body.insert(character),
        KeyCode::Backspace => editor.body.backspace(),
        KeyCode::Delete => editor.body.delete(),
        KeyCode::Left => editor.body.left(),
        KeyCode::Right => editor.body.right(),
        KeyCode::Up => editor.body.up(),
        KeyCode::Down => editor.body.down(),
        KeyCode::Home => editor.body.home(),
        KeyCode::End => editor.body.end(),
        _ => {}
    });
    Vec::new()
}

/// Keys while the file list has focus: `j`/`k` move, `space` puts the file
/// in or leaves it out, `⏎` commits.
fn files_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let mut commands = Vec::new();
    match key.code {
        KeyCode::Enter => return commit(state, next_id),
        KeyCode::Char(' ') => files::toggle(state),
        KeyCode::Char('j') | KeyCode::Down => {
            files::move_cursor(state, &mut commands, true, next_id);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            files::move_cursor(state, &mut commands, false, next_id);
        }
        _ => {}
    }
    commands
}

/// Dispatch the commit, unless the screen already knows it would be
/// refused (C9) — the bar already says why, and pressing again says nothing
/// new. A live operation's words already stand in the bar's row
/// ([`super::live_words`]), so its refusal adds nothing either.
fn commit(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
    if live_operation(state).is_some() {
        return Vec::new();
    }
    let Some(editor) = state.commit_editor.as_ref() else {
        return Vec::new();
    };
    if editor.refusal().is_some() {
        return Vec::new();
    }
    let mut commands = Vec::new();
    if editor.reword_line {
        // A line is built on `HEAD`: the reword is a rewrite, planned and
        // previewed like an amend with dependents (m8-polish-and-ship/14).
        // The screen goes with it, kept until the reword lands.
        if let Some(editor) = state.commit_editor.take() {
            crate::update::amend_into::reword_confirmed(state, &mut commands, *editor, next_id);
        }
        return commands;
    }
    let operation = editor.operation();
    let launched = crate::update::operate::launch(state, &mut commands, operation, next_id);
    crate::update::operate::say_if_refused(state, launched);
    commands
}
