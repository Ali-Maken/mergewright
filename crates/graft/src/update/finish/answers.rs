//! What the summary's jobs answer: the summary read, a row's diff, the
//! `$EDITOR` round trip and the `--continue` the summary started.

use super::keys::close_keeping_draft;
use super::{
    AppState, Arc, DiffView, FinishAnswer, JobId, Loading, Operated, ReturnTo, ReviewFailed,
    Reviewed, TextArea, TextField, keep_cursor_on_screen, split_message,
};
use crate::update::op_keys::OpKeys;

/// The summary read came back.
///
/// A read that found files with a clash still open (ease-pass/05b) closes
/// the review: the words go where `c` or `y` was pressed — the status line
/// over the banner, the notice row of the resolver.
pub(crate) fn summary_landed(
    state: &mut AppState,
    id: JobId,
    summary: Result<Reviewed, ReviewFailed>,
) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    // A read for a summary since closed (and perhaps reopened): the open
    // one has its own read in flight, which is the one to show.
    if finish.job != Some(id) {
        return;
    }
    finish.job = None;
    match summary {
        Err(ReviewFailed::StillOpen { files, clashes }) => {
            let back_to = finish.return_to;
            close_keeping_draft(state);
            still_open(state, back_to, files, clashes);
            return;
        }
        Ok(Reviewed { summary, solved }) => {
            finish.solved = solved;
            // A restored draft is the user's words; so is anything typed
            // while the read ran. Git's message fills only empty fields.
            let untouched = finish.subject.is_empty() && finish.body.is_empty();
            if !finish.from_draft && untouched {
                let (subject, body) = split_message(&String::from_utf8_lossy(&summary.message));
                finish.subject = TextField::new(&subject);
                finish.body = TextArea::new(&body);
            }
            finish.cursor = if summary.by_git.is_empty() {
                0
            } else {
                summary.by_you.len()
            };
            finish.summary = Loading::Ready(summary);
        }
        Err(ReviewFailed::Failed(why)) => finish.summary = Loading::Failed(why),
    }
    keep_cursor_on_screen(state);
}

/// `files` files, `clashes` clashes, still open: said where the review was
/// asked for. Over the banner, today's `c` refusal, counting files; in the
/// resolver, the clashes and the way to the next — the unit its own `y`
/// says (ease-pass/05s, Minor 3).
fn still_open(state: &mut AppState, back_to: ReturnTo, files: usize, clashes: usize) {
    match back_to {
        ReturnTo::Banner => {
            let noun = if files == 1 { "file" } else { "files" };
            state.banner_focus = crate::update::banner::banner_view(state).is_some();
            state.status = crate::update::banner::continue_refused(
                format!("{files} {noun} still conflicted"),
                crate::update::banner::refused_at(state),
                &state.keymap,
            );
        }
        ReturnTo::Resolver => {
            let words = crate::update::resolver::clashes_still_open(clashes, &state.keymap);
            crate::update::resolver::set_notice(state, (words, false));
        }
    }
}

/// A row's diff came back.
pub(crate) fn diff_landed(state: &mut AppState, id: JobId, diff: Result<Arc<DiffView>, String>) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    // Dropping a stale answer is safe: `space` on another row (or on this
    // one again) replaced or collapsed the diff it was for, and the diff on
    // screen now has its own read in flight or none at all.
    if finish.diff_job != Some(id) {
        return;
    }
    finish.diff_job = None;
    if let Some((_, shown)) = finish.expanded.as_mut() {
        *shown = match diff {
            Ok(diff) => Loading::Ready(diff),
            Err(why) => Loading::Failed(why),
        };
    }
    keep_cursor_on_screen(state);
}

/// A job the summary waits on failed or panicked: say so where its answer
/// would have gone, so nothing reads `reading…` forever.
pub(crate) fn job_ended(state: &mut AppState, id: JobId, failure: &dyn std::fmt::Display) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    if finish.job == Some(id) {
        finish.job = None;
        finish.summary = Loading::Failed(failure.to_string());
    }
    if finish.diff_job == Some(id) {
        finish.diff_job = None;
        if let Some((_, shown)) = finish.expanded.as_mut() {
            *shown = Loading::Failed(failure.to_string());
        }
    }
}

/// `$EDITOR` handed the message back. `true` when it was the summary's.
///
/// **Strips nothing** (S3): a `#123 fixes login` line the user wrote in the
/// editor is part of the message.
pub(crate) fn editor_returned(
    state: &mut AppState,
    id: JobId,
    text: Result<String, String>,
) -> bool {
    let Some(finish) = state.finish.as_mut() else {
        return false;
    };
    if finish.editing != Some(id) {
        return false;
    }
    finish.editing = None;
    state.external_editor = None;
    match text {
        Ok(text) => {
            let (subject, body) = split_message(&text);
            finish.subject = TextField::new(&subject);
            finish.body = TextArea::new(&body);
            finish.notice = None;
        }
        // The fields still hold what went out: nothing was lost.
        Err(why) => finish.notice = Some((why, true)),
    }
    true
}

/// A `--continue` came back; acts only on the one the summary started.
pub(crate) fn resume_answered(
    state: &mut AppState,
    id: JobId,
    outcome: &Operated,
) -> Option<FinishAnswer> {
    let finish = state.finish.as_mut()?;
    if finish.committing != Some(id) {
        return None;
    }
    finish.committing = None;
    if let Some(reason) = refusal(outcome, &state.keymap) {
        finish.notice = Some((reason, true));
        return Some(FinishAnswer::Kept);
    }
    // A summary opened from the resolver (m6-conflicts/31) has it still
    // open underneath. Any answer but a refusal ends that resolver's stop:
    // committed (`Done`, `Incomplete`), nothing is left to resolve and the
    // landing is the graph (J6); stopped again (`Conflicted`), the new stop
    // opens its own resolver, which an open one would refuse (review
    // m6-amend-31).
    state.resolver = None;
    if let Operated::Done(_) = outcome {
        let words = match &finish.summary {
            Loading::Ready(summary) => Some(format!(
                "merged {} into {}",
                String::from_utf8_lossy(&summary.from),
                String::from_utf8_lossy(&summary.into)
            )),
            Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
        };
        state.finish = None;
        state.finish_draft = None;
        state.external_editor = None;
        return Some(FinishAnswer::Merged(words));
    }
    close_keeping_draft(state);
    Some(FinishAnswer::Closed)
}

/// git said no to the commit and nothing moved: a hook refused it, or a
/// precondition did. The words to show, else `None`.
pub(super) fn refusal(outcome: &Operated, keys: &graft_ui::keymap::Keymap) -> Option<String> {
    match outcome {
        Operated::Failed { error, refused, .. } => Some(
            refused
                .as_ref()
                .map_or_else(|| error.clone(), |refusal| refusal.explain(&OpKeys(keys))),
        ),
        // A `--continue` that stopped with nothing unmerged is git refusing
        // to commit (`jobs::operate::Operated::Conflicted::stalled`).
        Operated::Conflicted {
            stalled: Some(words),
            paths,
            ..
        } if paths.is_empty() => Some(words.clone()),
        Operated::Done(_)
        | Operated::StagingNotKept { .. }
        | Operated::Incomplete { .. }
        | Operated::Conflicted { .. }
        | Operated::Clashed { .. } => None,
    }
}
