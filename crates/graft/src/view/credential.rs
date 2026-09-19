//! Turning [`AppState`]'s credential prompt into the view model
//! [`graft_ui::credential_prompt::CredentialPrompt`] draws (m4-remotes/06).
//!
//! The field's `masked` decides everything about how the text is drawn; the
//! counter is left off on purpose, because the number of characters typed is
//! itself something a password prompt must not show
//! (`graft_ui::text_input`).

use graft_ops::operation::Operation;
use graft_term::sanitize_line;
use graft_ui::credential_prompt::CredentialView;
use graft_ui::text_input::FieldView;

use crate::update::AppState;

/// What the note under the field says, always: Graft answers git's question
/// and keeps nothing (design note m4, choice 2).
const NOTE: &str = "typed here only · never stored by Graft";

/// The prompt as it draws now, or `None` when git is asking nothing.
#[must_use]
pub fn credential_view(state: &AppState) -> Option<CredentialView<'static>> {
    let prompt = state.credential.as_ref()?;
    Some(CredentialView {
        title: prompt.title.clone(),
        prompt: prompt.prompt.clone(),
        field: FieldView {
            label: "",
            // Already sanitised as it was typed (`TextField` refuses control
            // characters at the door); `sanitize_line` again changes
            // nothing and keeps the one rule in one place.
            text: sanitize_line(prompt.field.text()),
            cursor: Some(prompt.field.cursor()),
            guide_at: None,
            // No counter: its length is part of what a password hides.
            counter: None,
            focused: true,
            masked: prompt.masked,
        },
        note: sanitize_line(NOTE),
        footer: sanitize_line(&format!(
            "⏎ send  ⌃U clear  esc cancel the {}",
            what_it_stops(state)
        )),
    })
}

/// What `Esc` cancels, in the user's word for it: the write that asked.
fn what_it_stops(state: &AppState) -> &'static str {
    match state
        .remote
        .running
        .as_ref()
        .and_then(|running| running.op.as_ref())
    {
        Some(Operation::Fetch(_)) => "fetch",
        Some(Operation::Pull(_)) => "pull",
        Some(Operation::Push(_)) => "push",
        // Every credential question comes from a remote operation; if one
        // ever arrives without the runnning op on hand, say the true thing
        // rather than guess a word.
        _ => "operation",
    }
}
