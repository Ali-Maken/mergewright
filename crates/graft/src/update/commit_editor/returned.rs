//! What comes back to an open editor from outside it: `commit.template`'s
//! contents and the file `$EDITOR` wrote.

use graft_ui::text_input::TextArea;

use super::{AppState, JobId, Status};

/// `commit.template` came back. Only fills an empty body: between asking and
/// answering the user may already have typed.
pub fn template_arrived(state: &mut AppState, id: JobId, template: Result<Option<String>, String>) {
    if state.commit_template != Some(id) {
        return;
    }
    state.commit_template = None;
    let template = match template {
        Ok(template) => template,
        // The editor is already open and usable; the template's failure is
        // reported rather than dropped, and the body stays empty.
        Err(why) => {
            state.status = Status::Failed(format!("commit.template: {why}"));
            return;
        }
    };
    let Some(text) = template else {
        return;
    };
    let Some(editor) = state.commit_editor.as_mut() else {
        return;
    };
    if !editor.body.is_empty() {
        return;
    }
    let body = strip_comments(&text);
    if !body.trim().is_empty() {
        editor.body = TextArea::new(body.trim_end_matches('\n'));
    }
}

/// `$EDITOR` closed: the file it wrote, or why Graft could not use it.
///
/// Dropped when the editor it belongs to is gone — `Esc` while `$EDITOR` was
/// open, or a second `Ctrl+E` — so a message never lands in a field the user
/// has moved on from.
pub fn editor_returned(state: &mut AppState, id: JobId, result: Result<String, String>) {
    if state.external_editor != Some(id) {
        return;
    }
    state.external_editor = None;
    let Some(editor) = state.commit_editor.as_mut() else {
        return;
    };
    match result {
        Ok(text) => editor.load_message(strip_comments(&text).trim_end_matches('\n')),
        // The old text is still in the fields: nothing was thrown away, and
        // the reason says so.
        Err(why) => state.status = Status::sticky(why),
    }
}

/// Drop the lines git itself would strip from a message file: those starting
/// with `#`, once leading whitespace is ignored the way git's own
/// `--cleanup=strip` does not.
///
/// Kept simple on purpose: git strips a line whose **first** character is the
/// comment character. A line indented before its `#` is content to git, and
/// so it is here too.
fn strip_comments(text: &str) -> String {
    let kept: Vec<&str> = text
        .split('\n')
        .filter(|line| !line.starts_with('#'))
        .collect();
    kept.join("\n")
}
