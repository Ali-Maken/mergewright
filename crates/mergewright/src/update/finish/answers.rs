//! What the review's jobs answer: the review's read, a row's diff and the
//! `--continue` the review started.

use super::keys::close;
use super::{
    AppState, Arc, DiffView, FinishAnswer, JobId, Loading, Operated, ReturnTo, ReviewFailed,
    Reviewed, keep_cursor_on_screen,
};
use crate::update::op_keys::OpKeys;

/// The summary read came back.
///
/// The read a door asked for (ease-pass/05s, `ask_review`) opens the review
/// only now: `Ready` and `Failed` open it, as if it had been waiting on
/// this read; `StillOpen` opens nothing and says how many where `c` or `y`
/// was pressed — the status line over the banner, the notice row of the
/// resolver. A review already open (`open_finish`) that hears `StillOpen`
/// closes and says the same.
pub(crate) fn summary_landed(
    state: &mut AppState,
    id: JobId,
    summary: Result<Reviewed, ReviewFailed>,
) {
    if let Some(from) = state.picks.review_answered(id) {
        if let Err(ReviewFailed::StillOpen { files, clashes }) = summary {
            still_open(state, from, files, clashes);
            return;
        }
        // `ask_review` asks only while no review is open; one opened since
        // (`open_finish`) waits on its own read and keeps it.
        if state.finish.is_none() {
            super::open_reading(state, from, id);
        }
    }
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
            close(state);
            still_open(state, back_to, files, clashes);
            return;
        }
        Ok(Reviewed {
            summary,
            solved,
            incoming,
        }) => {
            finish.solved = solved;
            finish.incoming = incoming;
            // The first clashed file, as the mock's cursor starts.
            finish.cursor = 0;
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
    // A door's read that never answered: the review opens showing why, as
    // a `Failed` answer would.
    if let Some(from) = state.picks.review_answered(id)
        && state.finish.is_none()
    {
        super::open_reading(state, from, id);
    }
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
    if let Operated::Done(applied) = outcome {
        let words = match &finish.summary {
            Loading::Ready(summary) => Some(merged_words(&summary.from, &summary.into)),
            Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
        };
        state.finish = None;
        // The merge just made: its row's bar is `JustMerged` while the
        // cursor rests on it and it is still `HEAD` (M18).
        state.last_merge = merge_made(&applied.entry);
        return Some(FinishAnswer::Merged(words));
    }
    close(state);
    Some(FinishAnswer::Closed)
}

/// The commit `HEAD` names after `entry`: the merge the review's `y` made.
fn merge_made(entry: &mergewright_core::journal::JournalEntry) -> Option<mergewright_core::oid::Oid> {
    use mergewright_core::journal::HeadRef;
    match entry.head_after.as_ref()? {
        HeadRef::Detached(oid) => Some(*oid),
        HeadRef::Branch(name) | HeadRef::Unborn(name) => entry.refs_after.get(name).copied()?,
    }
}

/// The status the merge lands with (mock
/// `mergewright-merge-screen.html:197`): `Merged <from> into <into>.` — the
/// status row draws the one `✓` before it, as it does a commit's.
pub(crate) fn merged_words(from: &[u8], into: &[u8]) -> String {
    format!(
        "Merged {} into {}.",
        String::from_utf8_lossy(from),
        String::from_utf8_lossy(into)
    )
}

/// git said no to the commit and nothing moved: a hook refused it, or a
/// precondition did. The words to show, else `None`.
pub(super) fn refusal(outcome: &Operated, keys: &mergewright_ui::keymap::Keymap) -> Option<String> {
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
