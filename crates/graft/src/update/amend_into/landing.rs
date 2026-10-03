//! A put-in's landing: its own words on the status line, then the read that
//! tags the commits it changed.

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_ops::operation::Operation;
use graft_ui::graph_screen::RowTag;

use super::is_put_in;
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// The landing's put-back clause for a put-in: the staged edits went into
/// commits, so only the unstaged ones are "back" (09b Q3).
const UNSTAGED_BACK: &str = "your unstaged edits are back";

/// An operation landed (`operate::operated`'s `Done`). When it is the
/// put-in planned here: its own words, and a read for the rows to tag.
pub(crate) fn landed(
    state: &mut AppState,
    op: Option<&Operation>,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(op) = op else {
        return;
    };
    let Some(asked) = state
        .amend_into
        .asked
        .take_if(|asked| is_put_in(op, &asked.params))
    else {
        return;
    };
    // A reword's text is a commit now (review m8-14 Major 2): nothing is
    // left for `a` to reopen.
    state.amend_into.reword_draft = None;
    // The note's words, composed with the put-back's (09b Q3). A put-in
    // always sets edits aside — it has staged ones (`Operation::sets_aside`)
    // — so `done_status`'s plain `your edits are back` would claim back the
    // staged edits that are now in commits. What came back is his unstaged
    // edits, when the planned tree had any. A `Done` means the put-back went
    // through clean; a clash lands elsewhere.
    let unstaged_back = asked.unstaged;
    let also = join(
        asked.words.also.as_deref(),
        unstaged_back.then_some(UNSTAGED_BACK),
    );
    state.status = Status::Done {
        what: asked.words.what.clone(),
        also: (!also.is_empty()).then_some(also),
        keys: Some("u undoes".to_owned()),
    };
    if asked.offsets.is_empty() {
        return;
    }
    let id = next_id();
    state.amend_into.tagging = Some(id);
    commands.push(Command::ResolveRowTags(id, asked.offsets));
}

/// Any tracked file with edits not staged (untracked files are never set
/// aside: they stay where they are).
pub(super) fn has_unstaged(state: &AppState) -> bool {
    state.working_tree.as_deref().is_some_and(|tree| {
        tree.entries
            .iter()
            .any(|entry| entry.unstaged.is_some() && !entry.untracked)
    })
}

fn join(first: Option<&str>, second: Option<&str>) -> String {
    [first, second]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// The changed commits' new rows came back: tag them, or say why not.
pub(crate) fn tags_resolved(state: &mut AppState, id: JobId, answer: Result<Vec<Oid>, String>) {
    if state.amend_into.tagging.take_if(|job| *job == id).is_none() {
        return;
    }
    match answer {
        Ok(changed) => {
            for commit in changed {
                state.row_tags.set(commit, RowTag::Changed);
            }
        }
        Err(why) => {
            // Rule 3: the landing stands, and the missing tag is said
            // beside it rather than left out silently.
            let missing = format!("couldn't mark the changed commit: {why}");
            state.status = match std::mem::replace(&mut state.status, Status::Ready) {
                Status::Done { what, also, keys } => Status::Done {
                    what,
                    also: Some(join(also.as_deref(), Some(&missing))),
                    keys,
                },
                // The next key already replaced the landing: the failure
                // still has to be seen.
                _ => Status::sticky(missing),
            };
        }
    }
}

/// A write started (`operate::claim_write`): a tag read still out names
/// rows of the history before it, so its answer must tag nothing (review
/// m7-09-to-21c, Minor 2). The tags already drawn go with `row_tags`.
pub(crate) fn write_claimed(state: &mut AppState) {
    state.amend_into.tagging = None;
}
