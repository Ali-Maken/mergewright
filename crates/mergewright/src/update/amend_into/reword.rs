//! The reword of `HEAD` with a line built on it (m8-polish-and-ship/14):
//! the commit editor's `⏎` plans it, and the text he typed is kept until it
//! lands (review m8-14 Majors 1 and 2).
//!
//! The text is `a`'s, never `c`'s: a reword editor closed with `Esc`, a
//! reword the plan refused, a reword preview closed — each keeps its editor
//! here, and the next `a` that answers `RewordHead` on the same `HEAD`
//! reopens it instead of `HEAD`'s old message. `c` keeps its own draft
//! (`AppState::commit_draft`), so a reword's mode can never turn `c` into a
//! rewrite.

use mergewright_core::job::JobId;
use mergewright_core::oid::Oid;
use mergewright_ops::rewrite::RewriteRequest;

use super::start;
use crate::update::commit_editor::CommitEditor;
use crate::update::events::Command;
use crate::update::state::AppState;

/// The reword editor's `⏎`: plan `HEAD`'s new message with the line built
/// on it, then the preview, as an amend with dependents does. No `head`:
/// the line was already found, and a line gone since is the plan's
/// `PlainAmend` refusal, in words. The editor is kept until the reword
/// lands, so a refusal or a closed preview loses nothing he typed.
pub(crate) fn reword_confirmed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    editor: CommitEditor,
    next_id: &mut impl FnMut() -> JobId,
) {
    let message = editor.message();
    keep(state, editor);
    start(
        state,
        commands,
        (RewriteRequest::RewordHead { message }, "a"),
        None,
        "planning the reword\u{2026}".to_owned(),
        next_id,
    );
}

/// Keep a reword editor's text for the next `a`, with the `HEAD` it was
/// typed for; an empty one throws nothing away, so nothing is kept.
pub(crate) fn keep(state: &mut AppState, editor: CommitEditor) {
    let head = head(state);
    state.amend_into.reword_draft = (!editor.is_empty()).then_some((head, editor));
}

/// The kept reword editor, when it was typed for the `HEAD` the refs show
/// now; taken either way, so a text for another `HEAD` is never put on
/// this one.
pub(crate) fn kept(state: &mut AppState) -> Option<CommitEditor> {
    let head = head(state);
    state
        .amend_into
        .reword_draft
        .take()
        .and_then(|(typed_for, editor)| (typed_for == head).then_some(editor))
}

/// `HEAD`'s commit, from the refs the badges read; `None` until they are.
fn head(state: &AppState) -> Option<Oid> {
    state.refs.as_deref().and_then(crate::refs::RefBadges::head)
}
