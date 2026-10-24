//! Opening the screen: `c` on its own draft, `a` once `HEAD`'s message is
//! in, and putting either on screen with its files chosen fresh.

use mergewright_core::job::JobId;
use mergewright_core::oid::Oid;
use mergewright_ops::operation::{CommitParams, Operation};
use mergewright_ui::text_input::{TextArea, TextField};

use super::{
    AppState, Command, CommitEditor, Status, at_edit_stop, files, live_operation,
    operation_refusal, refusal_way,
};

/// Open the screen: `c` plain, `a` for an amend.
///
/// `a` does not open anything yet — it asks for `HEAD`'s message first and
/// the screen opens on the answer. One read of one commit is not worth a
/// screen that changes under the user's hands.
///
/// `c` takes the `c` slot's words (08s M1), and asks for `commit.template`
/// only when there are none. Either way the commit he just made stops being
/// the one `u` gives back as a message ([`super::LastCommit`]).
pub fn open(
    state: &mut AppState,
    amend: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.commit_editor.is_some() {
        return Vec::new();
    }
    // An operation is live: the screen would open only to refuse, so the
    // pane answers in words instead (m6-conflicts/48). Nothing is read and
    // the words stay where `esc` left them.
    if let Some(live) = live_operation(state) {
        state.status = operation_refusal(live, refusal_way(state), &state.keymap).status();
        return Vec::new();
    }
    state.last_commit = None;
    if amend {
        let id = next_id();
        state.head_message = Some(id);
        return vec![Command::ReadHeadMessage(id)];
    }
    let mut editor = state
        .drafts
        .commit
        .take()
        .map_or_else(CommitEditor::new, |draft| CommitEditor::from_draft(&draft));
    let mut commands = Vec::new();
    if !editor.template_applied && editor.body.is_empty() && !editor.from_draft {
        let id = next_id();
        editor.template_applied = true;
        state.commit_template = Some(id);
        commands.push(Command::ReadCommitTemplate(id));
    }
    show(state, &mut commands, editor, next_id);
    commands
}

/// Put `editor` on screen with its files chosen fresh (C6) and their counts
/// and the first file's diff asked for.
fn show(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    mut editor: CommitEditor,
    next_id: &mut impl FnMut() -> JobId,
) {
    files::choose(&mut editor, state.working_tree.as_deref());
    state.commit_editor = Some(Box::new(editor));
    files::ask_counts(state, commands, next_id);
    files::ask_diff(state, commands, next_id);
}

/// `HEAD`'s message came back: open the amend screen on it, or say why not
/// — or, for `a` at an `edit` stop, amend with it at once (07b).
pub fn head_message_arrived(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    message: Result<String, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.head_message != Some(id) {
        return;
    }
    state.head_message = None;
    // P14's exception: at an `edit` stop the amend is the step itself
    // (`mergewright_ops`' `check` holds the same rule against a fresh read).
    if std::mem::take(&mut state.edit_amend) && at_edit_stop(state) {
        amend_the_stop(state, commands, message, next_id);
        return;
    }
    // An operation became live while the read was in flight: refuse as
    // `open` would have, whatever the read said.
    if let Some(live) = live_operation(state) {
        state.status = operation_refusal(live, refusal_way(state), &state.keymap).status();
        return;
    }
    let message = match message {
        Ok(message) => message,
        // Not swallowed: `a` did nothing, and the user is told why — an
        // unborn `HEAD` is the common reason.
        Err(why) => {
            state.status = Status::sticky_failure(format!("amend: {why}"));
            return;
        }
    };
    let head = head_oid(state);
    let reword_line = state.amend_into.reword_read_answered(id);
    let mut editor = amend_screen(state, head, &message, reword_line);
    editor.amending = head;
    show(state, commands, editor, next_id);
}

/// `a` at an `edit` stop: amend at once with `HEAD`'s own message (07b).
fn amend_the_stop(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    message: Result<String, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    match message {
        Ok(message) => {
            let amend = Operation::Amend(CommitParams {
                message,
                ..CommitParams::default()
            });
            let launched = super::super::operate::launch(state, commands, amend, next_id);
            super::super::operate::say_if_refused(state, launched);
        }
        Err(why) => state.status = Status::sticky_failure(format!("amend: {why}")),
    }
}

/// The screen `a` opens on `HEAD`: a reword's own words when it kept some
/// for this `HEAD` (review m8-14 Major 2); else `HEAD`'s message, under the
/// `a` slot's words when there are any.
fn amend_screen(
    state: &mut AppState,
    head: Option<Oid>,
    message: &str,
    reword_line: bool,
) -> CommitEditor {
    let kept = reword_line
        .then(|| super::super::amend_into::kept_reword(state))
        .flatten();
    if let Some(mut kept) = kept {
        kept.from_draft = true;
        return kept;
    }
    let short = head.map_or_else(|| "HEAD".to_owned(), short_sha);
    let mut editor = CommitEditor::amend(short, message);
    editor.reword_line = reword_line;
    resume_amend_draft(state, &mut editor, head);
    editor
}

/// The `a` slot's words go over `HEAD`'s message (C15). Typed for an older
/// `HEAD`, they still come back — never dropped — and the status row says
/// which commit they were typed for (08s M1); the header names the one
/// this amend rewrites.
fn resume_amend_draft(state: &mut AppState, editor: &mut CommitEditor, head: Option<Oid>) {
    let Some(draft) = state.drafts.amend.take() else {
        return;
    };
    editor.subject = TextField::new(&draft.subject);
    editor.body = TextArea::new(&draft.body);
    editor.from_draft = true;
    if let Some(old) = draft.amending.filter(|old| Some(*old) != head) {
        state.status = Status::sticky(format!("your words from amending {}", short_sha(old)));
    }
}

/// A commit's seven-character name, as the header and the graph spell it.
pub(crate) fn short_sha(oid: Oid) -> String {
    oid.to_string().chars().take(7).collect()
}

/// `HEAD`'s commit, from the refs the badges already read; `None` before
/// the first refs answer.
pub(crate) fn head_oid(state: &AppState) -> Option<Oid> {
    state.refs.as_deref().and_then(crate::refs::RefBadges::head)
}
