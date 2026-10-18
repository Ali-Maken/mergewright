//! Which job answers are the editor's, and what each does to it: plans, file lists, messages, ends, writes.

use super::{
    After, AppState, Arc, Command, FileList, JobId, Loading, Operated, Planned, PreviewFailure,
    Right, Status, keys, words,
};

/// Is `id` the open read, the AFTER plan, the `⏎` file list or the
/// `$EDITOR` handover of the live editor — or one a closed or superseded
/// editor left in flight, whose answer is dropped here?
pub(crate) fn owns(state: &AppState, id: JobId) -> bool {
    let rewrite = &state.rewrite;
    rewrite.opening == Some(id)
        || rewrite.stale.contains(&id)
        || rewrite
            .editor
            .as_ref()
            .is_some_and(|editor| editor.jobs().any(|job| job == id))
}

/// The AFTER plan answered. Only the live editor's current plan is kept.
pub(crate) fn after_planned(
    state: &mut AppState,
    id: JobId,
    answer: Result<Arc<Planned>, PreviewFailure>,
) {
    if state.rewrite.stale.remove(&id) {
        return;
    }
    // The editor fills the screen, so its notice row is the terminal's
    // width; its words join reason and keys (`status_words`).
    let columns = state.size.columns;
    let Some(editor) = state.rewrite.editor.as_mut() else {
        return;
    };
    let After::Pending { job, asked } = &editor.after else {
        return;
    };
    if *job != id {
        return;
    }
    let asked = asked.clone();
    editor.after = match answer {
        Ok(planned) => After::Ready { planned, asked },
        Err(PreviewFailure::Refused(refusal)) => {
            let op = mergewright_ops::operation::Operation::Rewrite(asked);
            After::Refused(words::status_words(
                &crate::update::operate::refusal_status(&op, &refusal, columns, &state.keymap),
            ))
        }
        Err(PreviewFailure::Failed(why)) => After::Failed(why),
    };
}

/// `⏎`'s file list answered.
pub(crate) fn files_listed(state: &mut AppState, id: JobId, answer: Result<Arc<FileList>, String>) {
    if state.rewrite.stale.remove(&id) {
        return;
    }
    let Some(editor) = state.rewrite.editor.as_mut() else {
        return;
    };
    if let Right::Changes { files, .. } = &mut editor.right
        && *files == Loading::Pending(id)
    {
        *files = match answer {
            Ok(list) => Loading::Ready(list),
            Err(why) => Loading::Failed(why),
        };
    }
}

/// `$EDITOR` came back with the whole message (`Ctrl+E` in the field): it
/// becomes the row's message, as `⏎` in the field would, and replans.
pub(crate) fn message_returned(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    answer: Result<String, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.rewrite.stale.remove(&id) {
        return;
    }
    let Some(mut editor) = state.rewrite.editor.take() else {
        return;
    };
    let ours = editor
        .field
        .as_ref()
        .is_some_and(|field| field.handover == Some(id));
    if ours {
        match answer {
            Ok(text) => {
                let message = format!("{}\n", text.trim_end_matches('\n'));
                keys::keep_message(state, &mut editor, message, commands, next_id);
            }
            Err(why) => {
                if let Some(field) = editor.field.as_mut() {
                    field.handover = None;
                }
                editor.notice = Some(why);
            }
        }
    }
    state.rewrite.editor = Some(editor);
}

/// A job ended with no answer (cancelled, or failed as a job): whatever
/// waited on it stops waiting, in words where it was the user's.
pub(crate) fn job_ended(state: &mut AppState, id: JobId, why: &str) {
    if state.rewrite.stale.remove(&id) {
        return;
    }
    if state.rewrite.opening == Some(id) {
        state.rewrite.opening = None;
        state.rewrite.opening_from = None;
        if matches!(state.status, Status::Waiting(_)) {
            state.status = Status::Ready;
        }
        return;
    }
    let Some(editor) = state.rewrite.editor.as_mut() else {
        return;
    };
    if matches!(&editor.after, After::Pending { job, .. } if *job == id) {
        editor.after = After::Failed(why.to_owned());
    }
    if let Right::Changes { files, .. } = &mut editor.right
        && *files == Loading::Pending(id)
    {
        *files = Loading::Failed(why.to_owned());
    }
    if let Some(field) = editor.field.as_mut()
        && field.handover == Some(id)
    {
        field.handover = None;
        editor.notice = Some(why.to_owned());
    }
}

/// The write `y` launched answered (`operate::operated`, first): a refusal
/// brings the editor back with the words on its notice row (R1 "Refused at
/// apply: stays in the editor"); anything else — done, stopped, failed —
/// ends it.
pub(crate) fn write_answered(state: &mut AppState, answer: &Operated) {
    let Some(mut editor) = state.rewrite.launched.take() else {
        return;
    };
    if let Operated::Failed {
        refused: Some(refusal),
        ..
    } = answer
    {
        let op = match &editor.after {
            After::Ready { asked, .. } => {
                mergewright_ops::operation::Operation::Rewrite(asked.clone())
            }
            _ => mergewright_ops::operation::Operation::Rewrite(editor.opened.clone()),
        };
        editor.notice = Some(words::status_words(
            &crate::update::operate::refusal_status(
                &op,
                refusal,
                state.size.columns,
                &state.keymap,
            ),
        ));
        state.rewrite.editor = Some(editor);
    }
}
