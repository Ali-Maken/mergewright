//! Turning [`AppState`]'s credential prompt into the view model
//! [`graft_ui::credential_prompt::CredentialPrompt`] draws (m4-remotes/06).
//!
//! The field's `secret` decides everything about how the text is drawn; the
//! counter is left off on purpose, because the number of characters typed is
//! itself something a password prompt must not show
//! (`graft_ui::text_input`).

use graft_ops::operation::Operation;
use graft_term::sanitize_line;
use graft_ui::credential_prompt::CredentialView;
use graft_ui::text_input::FieldView;

use crate::update::AppState;
use crate::update::credential::PromptField;

/// What the note under the field says, always: Graft answers git's question
/// and keeps nothing (design note m4, choice 2).
const NOTE: &str = "typed here only · never stored by Graft";

/// The prompt as it draws now, or `None` when git is asking nothing.
#[must_use]
pub fn credential_view(state: &AppState) -> Option<CredentialView<'static>> {
    let prompt = state.credential.as_ref()?;
    let field = match &prompt.field {
        // The open case is unchanged: real text, real cursor.
        PromptField::Open(field) => FieldView {
            label: "",
            // Already sanitised as it was typed (`TextField` refuses
            // control characters at the door); `sanitize_line` again
            // changes nothing and keeps the one rule in one place.
            text: sanitize_line(field.text()),
            cursor: Some(field.cursor()),
            guide_at: None,
            // No counter: its length is part of what a password hides.
            counter: None,
            focused: true,
            secret: None,
        },
        // No real text to pass: there is none to pass it from, and no real
        // cursor either — `graft_ui::text_input` draws its own fixed glyph
        // count and its own pulse from `secret` alone (ADR-0038 §2).
        PromptField::Secret(field) => FieldView {
            label: "",
            text: sanitize_line(""),
            cursor: None,
            guide_at: None,
            counter: None,
            focused: true,
            secret: Some(field.pulse()),
        },
    };
    Some(CredentialView {
        title: prompt.title.clone(),
        prompt: prompt.prompt.clone(),
        field,
        note: sanitize_line(NOTE),
        footer: sanitize_line(&footer(state)),
    })
}

/// The box's bottom border: its keys, and what `Esc` cancels — named as the
/// running write's own description, the words its `Busy` line draws
/// (`esc cancel push main → origin/main`), so the question says which push
/// it belongs to (m6-conflicts/78, Decided 2; F9's information, moved here
/// when 69 gave the row under the box to the box's keys). When that does not
/// fit the box whole, the verb alone (`esc cancel the push`): the border is
/// cut from the right, and a cut description names no push at all.
fn footer(state: &AppState) -> String {
    const KEYS: &str = "⏎ send  ⌃U clear  esc cancel";
    let named = state
        .remote
        .running
        .as_ref()
        .map(|running| format!("{KEYS} {}", running.description));
    // `border_row` keeps a label whole while it and its two gutters fit
    // between the corners: the box's width less four.
    let frame = ratatui::layout::Rect::new(0, 0, state.size.columns, state.size.rows);
    let room = usize::from(
        graft_ui::credential_prompt::credential_area(frame)
            .width
            .saturating_sub(4),
    );
    match named {
        // Display width, as the border measures it (ratatui's own count).
        Some(named) if ratatui::text::Line::raw(named.as_str()).width() <= room => named,
        _ => format!("{KEYS} the {}", what_it_stops(state)),
    }
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
