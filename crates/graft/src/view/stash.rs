//! The stash overlay's view model (`S`,
//! m3-working-tree-undo-and-stash/06).
//!
//! Pure: [`StashState`](crate::update::stash::StashState) plus the list as
//! it was last read, turned into rows. Nothing here decides anything — the
//! refusal on the save prompt is `update::stash::save_refusal`'s answer and
//! the notice line is `AppState::status`'s, so the overlay and the keys can
//! never disagree.

use graft_git::stash::StashEntry;
use graft_term::sanitize::{sanitize, sanitize_line};
use graft_ui::graph_screen::columns::relative_date;
use graft_ui::stash_screen::{StashPromptView, StashRowView, StashStatus, StashView};
use graft_ui::text_input::FieldView;

use crate::update::AppState;
use crate::update::Status;
use crate::update::detail::Loading;

/// [`StashView`] for whatever the stash read has brought back.
#[must_use]
pub fn stash_view(state: &AppState, now_seconds: i64) -> StashView<'_> {
    let (rows, status) = match &state.stash.entries {
        Loading::Idle | Loading::Pending(_) => (Vec::new(), StashStatus::Loading),
        Loading::Failed(why) => (Vec::new(), StashStatus::Failed(sanitize(why))),
        Loading::Ready(list) if list.is_empty() => (Vec::new(), StashStatus::Empty),
        Loading::Ready(list) => (
            list.iter().map(|entry| row(entry, now_seconds)).collect(),
            StashStatus::Ready,
        ),
    };
    let selected = state.stash.selected.min(rows.len().saturating_sub(1));
    let preview = match &state.stash.preview {
        Loading::Ready(view) => Some(view.as_ref()),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
    };
    let (notice, notice_is_failure) = notice(state);
    StashView {
        rows,
        selected,
        // The preview is about the selected stash; the prompt replaces the
        // body, so they never draw together.
        preview: preview.filter(|_| state.stash.prompt.is_none()),
        prompt: prompt(state),
        status,
        notice,
        notice_is_failure,
        // The put-back preview over the list holds every key; its footer is
        // the bar (review 86, M2).
        box_open: state.preview.is_some(),
    }
}

/// One entry's row: git's own name for it, the message it recorded, how
/// long ago, and how many paths it carries.
fn row(entry: &StashEntry, now_seconds: i64) -> StashRowView {
    StashRowView {
        name: sanitize(&format!("stash@{{{}}}", entry.index)),
        // Quoted, as §1.11 draws it: the message is the user's text and the
        // quotes are what say where it starts and stops.
        message: sanitize_line(&format!("\"{}\"", first_line(&entry.message))),
        age: sanitize(&age(entry.at.seconds, now_seconds)),
        files: entry.paths.len(),
    }
}

/// A multi-line stash message is listed by its first line, which is the
/// subject git's own `stash list` prints (`as-built.md`: undo keeps the
/// subject and nothing else either).
fn first_line(message: &str) -> &str {
    message.lines().next().unwrap_or(message)
}

/// `2h ago`, from the same cascade the graph's date column uses, so two
/// views of the same moment never disagree.
fn age(at_seconds: i64, now_seconds: i64) -> String {
    let relative = relative_date(at_seconds, now_seconds);
    if relative == "future" {
        relative
    } else {
        format!("{relative} ago")
    }
}

/// The save prompt, while it is open.
fn prompt(state: &AppState) -> Option<StashPromptView<'_>> {
    let open = state.stash.prompt.as_ref()?;
    Some(StashPromptView {
        title: sanitize("Stash the working tree"),
        message: FieldView {
            label: "message",
            text: sanitize_line(open.message.text()),
            cursor: Some(open.message.cursor()),
            guide_at: None,
            counter: None,
            focused: true,
            secret: None,
        },
        staged_only: open.staged_only,
        refusal: crate::update::stash::save_refusal(state).map(sanitize),
    })
}

/// What the overlay says about the last write: the graph screen's footer is
/// covered while this is up, so the acknowledgement has to be drawn here or
/// nowhere (m3-working-tree-undo-and-stash/06). Reads
/// `AppState::shown_status`, as the graph footer does (m6-conflicts/78).
fn notice(state: &AppState) -> (Option<graft_term::sanitize::SanitizedText>, bool) {
    match state.shown_status() {
        Status::Busy(description) => (Some(sanitize_line(&format!("{description}…"))), false),
        Status::Done { what, also, keys } => (
            Some(sanitize_line(&crate::view::status::done_line(
                what,
                also.as_deref(),
                keys.as_deref(),
            ))),
            false,
        ),
        Status::Waiting(words) => (Some(sanitize_line(words)), false),
        Status::Failed(message) => (Some(sanitize_line(message)), true),
        Status::Sticky { reason, keys: None } => (Some(sanitize_line(reason)), true),
        Status::Sticky {
            reason,
            keys: Some(keys),
        } => (Some(sanitize_line(&format!("{reason} · {keys}"))), true),
        Status::Ready | Status::Copied { .. } => (None, false),
    }
}
