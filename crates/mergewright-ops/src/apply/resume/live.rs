//! Which `Pending` entry is *this* live operation of Mergewright's. Moved out of
//! `resume.rs` (m8/13e).

use mergewright_core::journal::{EntryStatus, HeadRef, JournalEntry};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::Stop;
use mergewright_git::state::{self, InProgress};

use crate::facts::{self, FactsError};
use crate::operation::opens_in_progress;

use super::super::ApplyError;
use super::rewrite::started_on_its_line;

/// The newest `Pending` entry that is genuinely *this* live operation of
/// Mergewright's: the entry `resume` closes and recovery leaves alone (m5/07).
///
/// Kind alone was not enough (m5/07 review, M7). A Mergewright merge entry left
/// `Pending` by a crash *before* git ran, plus a `git merge` since started
/// in the shell, matched on kind — and `resume(Continue)` would have
/// finished someone else's merge and closed Mergewright's entry `Committed` with
/// a before-state from a different act. Two facts git leaves behind settle
/// it, both read here rather than assumed:
///
/// - **Where the operation started.** A rebase detaches `HEAD`, so its
///   branch is `rebase-merge/head-name`; a merge, cherry-pick or revert
///   leaves `HEAD` where it was. Either way the entry's `head_before` must
///   equal it. A `rebase-apply` directory with no `head-name` is a `git am`,
///   which matches no entry of Mergewright's at all (this also closes the group's
///   own Minor 11).
/// - **The branch has not moved.** A stopped merge, cherry-pick or revert
///   has written no commit, so the branch is still where the entry recorded
///   it. A rebase's own starting tip is in `rebase-merge/orig-head`, which
///   `mergewright_git::state` does not expose, so that leg rests on `head-name`
///   alone.
///
/// `ORIG_HEAD` is deliberately not used, although the review named it:
/// verified against git 2.54, a merge and a rebase set it but a conflicted
/// **cherry-pick and revert do not**, so requiring it would refuse two real
/// resumes to catch one impostor.
///
/// A fact the entry never recorded is not evidence against it: the
/// comparison is skipped rather than failed, which keeps a real conflict
/// resumable.
pub(crate) fn live_entry<'a>(
    repo: &RepoHandle,
    entries: &'a [JournalEntry],
    kind: InProgress,
    stop: &dyn Stop,
) -> Result<Option<&'a JournalEntry>, ApplyError> {
    live_entry_facts(repo, entries, kind, stop).map_err(|why| match why {
        // A stop stays a stop: the callers choose their cancel path on
        // `ApplyError::Stopped` alone.
        FactsError::Stopped => ApplyError::Stopped,
        other => ApplyError::Facts(other),
    })
}

/// [`live_entry`] in the error type it actually has: every failure is a read
/// of the repository, and the stop is [`FactsError::Stopped`]. For a caller
/// whose own error speaks `FactsError` (m7-power-tools/01a: no catch-all arm
/// guessing which `ApplyError` it was).
pub(crate) fn live_entry_facts<'a>(
    repo: &RepoHandle,
    entries: &'a [JournalEntry],
    kind: InProgress,
    stop: &dyn Stop,
) -> Result<Option<&'a JournalEntry>, FactsError> {
    if stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    let detail = state::in_progress_detail_with_stop(repo, stop).map_err(|error| match error {
        state::StateError::Io(io) => FactsError::State(io),
    })?;
    // The one git run in that read (the incoming commit's `rev-list`) carries
    // a stop on `detail.theirs`, where it reads as any failure to name the
    // commit — and nothing here uses `theirs`. Asking again is what makes a
    // stop raised during the read a stop (m7-power-tools/01a, gap :640).
    if stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    let started_on = match kind {
        InProgress::Rebase => match detail.as_ref().and_then(|found| found.head_name.as_deref()) {
            Some(name) => HeadRef::Branch(name.to_owned()),
            // A `git am`, or a rebase whose state file is unreadable:
            // nothing here can be shown to be Mergewright's.
            None => return Ok(None),
        },
        _ => facts::head_ref(repo)?,
    };
    let tip_now = match (&started_on, kind) {
        (
            HeadRef::Branch(name),
            InProgress::Merge | InProgress::CherryPick | InProgress::Revert,
        ) => facts::read_refs(repo, std::slice::from_ref(name))?
            .get(name)
            .copied()
            .flatten()
            .map(|oid| (name.clone(), oid)),
        _ => None,
    };
    for entry in entries.iter().rev() {
        let candidate = entry.status == EntryStatus::Pending
            && opens_in_progress(entry.kind) == Some(kind)
            && match &tip_now {
                Some((name, now)) => match entry.refs_before.get(name) {
                    Some(Some(recorded)) => recorded == now,
                    // Not recorded, or recorded absent: nothing to compare.
                    _ => true,
                },
                None => true,
            };
        if candidate
            && (entry.head_before == started_on || started_on_its_line(repo, entry, &started_on)?)
        {
            return Ok(Some(entry));
        }
    }
    Ok(None)
}
