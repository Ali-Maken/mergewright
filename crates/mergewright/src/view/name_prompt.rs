//! Turning [`AppState`]'s name prompt into the view model
//! [`mergewright_ui::name_prompt::NamePromptView`] draws
//! (m5-history-and-ghost-preview/10).
//!
//! The title says what is being named and where, because that is the part
//! the user cannot otherwise check from inside a modal box: the graph is
//! still behind it, but the row the cursor was on is not marked while a box
//! covers it.

use mergewright_term::sanitize_line;
use mergewright_ui::name_prompt::NamePromptView;
use mergewright_ui::text_input::FieldView;

use crate::update::AppState;
use crate::update::name_prompt::{NameKind, name_refusal};

/// What `⇥` toggles, and the only option this box has.
const CHECKOUT: &str = "check out after creating";

/// The prompt as it draws now, or `None` when nothing is being named.
#[must_use]
pub fn name_prompt_view(state: &AppState) -> Option<NamePromptView<'static>> {
    let prompt = state.name_prompt.as_ref()?;
    let title = match &prompt.kind {
        NameKind::Branch { at, .. } => format!("New branch at {}", short(*at)),
        NameKind::Tag { at } => format!("New tag at {}", short(*at)),
        NameKind::BranchFromStash { index, .. } => format!("Branch from stash@{{{index}}}"),
    };
    // Only a plain branch offers the toggle. `git stash branch` checks out
    // the branch it makes and there is no asking it not to, and a tag is
    // not something there is any way to be on, so for those two the box
    // draws no option rather than one that would not be obeyed.
    let option = match prompt.kind {
        NameKind::Branch { checkout, .. } => Some((sanitize_line(CHECKOUT), checkout)),
        NameKind::Tag { .. } | NameKind::BranchFromStash { .. } => None,
    };
    Some(NamePromptView {
        title: sanitize_line(&title),
        field: FieldView {
            label: "",
            // Already sanitised on the way in (`TextField` refuses control
            // characters at the door); sanitising again changes nothing and
            // keeps the one rule in one place.
            text: sanitize_line(prompt.field.text()),
            cursor: Some(prompt.field.cursor()),
            guide_at: None,
            counter: None,
            focused: true,
            secret: None,
        },
        option,
        refusal: name_refusal(state).map(|reason| sanitize_line(&reason)),
    })
}

/// The seven hex digits a commit is named by everywhere else on screen.
fn short(oid: mergewright_core::oid::Oid) -> String {
    oid.to_string().chars().take(7).collect()
}
