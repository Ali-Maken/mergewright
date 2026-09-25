//! Turning [`AppState`]'s commit editor into the view model
//! [`graft_ui::commit_editor::CommitEditorScreen`] draws
//! (m3-working-tree-undo-and-stash/05).
//!
//! Every string that came out of the repository — the branch name, the short
//! SHA, the refusal — goes through `graft_term::sanitize_line` here, because
//! the widget takes [`SanitizedText`] and nothing else.

use graft_term::{SanitizedText, sanitize_line};
use graft_ui::commit_editor::{BODY_LIMIT, CommitEditorView, Field, SUBJECT_LIMIT};
use graft_ui::text_input::{AreaView, FieldView};

use crate::update::AppState;
use crate::update::commit_editor::CommitEditor;

/// The editor as it draws now, or `None` when none is open.
#[must_use]
pub fn commit_editor_view(state: &AppState) -> Option<CommitEditorView<'static>> {
    let editor = state.commit_editor.as_ref()?;
    let busy = state.operating.is_some();
    let refusal = editor.refusal(
        state.working_tree.as_deref(),
        crate::update::commit_editor::live_operation(state),
        crate::update::commit_editor::refusal_tabs(state),
    );
    Some(CommitEditorView {
        title: title(state, editor, busy),
        subject: FieldView {
            label: Field::Subject.label(),
            text: sanitize_line(editor.subject.text()),
            cursor: (editor.focus == Field::Subject).then(|| editor.subject.cursor()),
            guide_at: u16::try_from(SUBJECT_LIMIT).ok(),
            counter: Some((editor.subject.len(), SUBJECT_LIMIT)),
            focused: editor.focus == Field::Subject,
            secret: None,
        },
        body: AreaView {
            label: Field::Body.label(),
            lines: editor
                .body
                .lines()
                .iter()
                .map(|line| sanitize_line(line))
                .collect(),
            cursor: (editor.focus == Field::Body).then(|| editor.body.cursor()),
            guide_at: Some(BODY_LIMIT),
            focused: editor.focus == Field::Body,
        },
        options: editor.options(),
        focus: editor.focus,
        refusal: refusal.as_deref().map(sanitize_line),
        // A hook's refusal is the commonest failure here and the editor is
        // what is on screen when it lands, so the editor draws it. It stays
        // until `Esc` dismisses it (m3/05 review, Major 1). `shown_status`
        // like every status reader (m6-conflicts/78).
        status: super::status::problem(state.shown_status()),
        busy,
    })
}

/// `COMMIT ── ⎇ main ── 2 files staged`, `AMEND a1b2c3d ── …`, or
/// `committing…` while the write runs (design note, "Commit editor").
///
/// The refusal is deliberately *not* here: the spec puts it beside the dimmed
/// button, and one reason in one place reads better than the same words twice.
fn title(state: &AppState, editor: &CommitEditor, busy: bool) -> SanitizedText {
    if busy {
        return sanitize_line("committing… — esc cancels");
    }
    let mut parts = Vec::new();
    match (&editor.amend, &editor.head_short) {
        (true, Some(short)) => parts.push(format!("AMEND {short}")),
        (true, None) => parts.push("AMEND".to_owned()),
        (false, _) => parts.push("COMMIT".to_owned()),
    }
    if let Some(branch) = branch_name(state) {
        parts.push(format!("⎇ {branch}"));
    }
    parts.push(staged_count(state));
    if editor.from_draft {
        parts.push("draft kept".to_owned());
    }
    sanitize_line(&parts.join(" ── "))
}

/// The branch `HEAD` is on, or `None` when `HEAD` is detached, unreadable or
/// not read yet. The name comes off disk as bytes, so it is sanitised here
/// like any other repository text.
fn branch_name(state: &AppState) -> Option<String> {
    let facts = state.repository.as_ref()?;
    match &facts.head {
        crate::repository::Head::Branch(name) | crate::repository::Head::Unborn(name) => {
            Some(String::from_utf8_lossy(name).into_owned())
        }
        crate::repository::Head::Detached(_) | crate::repository::Head::Unreadable(_) => None,
    }
}

/// `2 files staged`, or `nothing read yet` before the first status lands.
fn staged_count(state: &AppState) -> String {
    let Some(status) = state.working_tree.as_deref() else {
        return "reading the working tree…".to_owned();
    };
    match status.staged().count() {
        0 => "nothing staged".to_owned(),
        1 => "1 file staged".to_owned(),
        many => format!("{many} files staged"),
    }
}
