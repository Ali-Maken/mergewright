//! The shell-merge route of a resume: a merge begun in the shell, being
//! continued or aborted, ended as an operation of its own — `FinishMerge`
//! (m6-conflicts/20) or `AbortMerge` (m6-conflicts/57) — and the refusals it
//! shares with a merge Mergewright began.

use std::path::PathBuf;

use mergewright_core::journal::{JournalEntry, MergeState};
use mergewright_git::state::InProgress;

use crate::conflict::{FileState, OpenError, file_state, open_conflict};
use crate::facts::FactsError;
use crate::merge_state::read_merge_state;
use crate::operation::{AbortMergeParams, FinishMergeParams, MergeEnd, Refusal};

use super::super::{ApplyError, Context};
use super::Resume;

/// What a resume acts on.
pub(super) enum Target {
    /// A stopped operation of Mergewright's: its entry is closed by the resume,
    /// after `stage` — a merge's solved files — is staged inside its span.
    Live {
        kind: InProgress,
        entry: Box<JournalEntry>,
        stage: Vec<PathBuf>,
    },
    /// A merge begun in the shell, being continued: finished as an
    /// operation of its own (m6-conflicts/20).
    ShellMerge(FinishMergeParams),
    /// A merge begun in the shell, being aborted: `git merge --abort` as an
    /// operation of its own, so `u` puts the merge back (m6-conflicts/57).
    ShellAbort(AbortMergeParams),
}

/// Step 3 of [`super::resume_target`]: a merge begun in the shell, being
/// continued or aborted. An octopus merge (several `MERGE_HEAD` lines) is
/// refused by name, `OctopusMerge`, before anything is written
/// (m6-conflicts/38) — for either end. Only a continue refuses unmerged
/// paths: an abort is how a merge still full of conflicts ends.
pub(super) fn shell_merge(ctx: &Context<'_>, how: &Resume) -> Result<Target, ApplyError> {
    let end = match how {
        Resume::Abort => MergeEnd::Abort,
        Resume::Continue { .. } | Resume::ContinueWith { .. } | Resume::Skip => MergeEnd::Finish,
    };
    let merge_head = match read_merge_state(ctx.repo) {
        Ok(Some(merge)) => {
            refuse_octopus(&merge, end)?;
            merge.merge_head
        }
        // `in_progress` saw `MERGE_HEAD` a moment ago and it is gone now:
        // not a merge this can end.
        Ok(None) => return Err(ApplyError::Refused(Refusal::NotInProgress)),
        Err(other) => return Err(ApplyError::Facts(FactsError::MergeState(other))),
    };
    if end == MergeEnd::Abort {
        return Ok(Target::ShellAbort(AbortMergeParams { merge_head }));
    }
    let solved = unmerged_to_stage(ctx, solved_of(how))?;
    let message = match how {
        Resume::ContinueWith { message, .. } => Some(message.clone()),
        Resume::Continue { .. } | Resume::Skip | Resume::Abort => None,
    };
    Ok(Target::ShellMerge(FinishMergeParams {
        merge_head,
        message,
        solved,
    }))
}

/// The files a `Continue` or `ContinueWith` names solved; none for the rest.
pub(super) fn solved_of(how: &Resume) -> &[PathBuf] {
    match how {
        Resume::Continue { solved } | Resume::ContinueWith { solved, .. } => solved,
        Resume::Skip | Resume::Abort => &[],
    }
}

/// `c` or `a` on an octopus merge: refused `OctopusMerge`, whichever route
/// asked (`resume`'s shell merge, or `apply(FinishMerge|AbortMerge)`
/// directly). `end` picks the command the words name.
pub(in crate::apply) fn refuse_octopus(
    merge: &MergeState,
    end: MergeEnd,
) -> Result<(), ApplyError> {
    if merge.other_heads.is_empty() {
        Ok(())
    } else {
        Err(ApplyError::Refused(Refusal::OctopusMerge {
            heads: merge.heads().count(),
            end,
        }))
    }
}

/// `Continue` over unmerged paths: refused before git runs, the same words
/// for a merge Mergewright began and one begun in the shell — unless every one is
/// in `solved` (ease-pass/05b). Those are the paths to stage, returned.
///
/// `solved` is the app's word, checked here against the files themselves by
/// the resolver's own rule ([`file_state`]): a path still holding an open
/// clash, a whole-file conflict or a binary one is never staged by a finish.
/// A solved path git already calls merged (staged by an earlier finish a
/// hook refused, or from the shell) is not staged again.
///
/// # Errors
/// [`Refusal::Unmerged`] naming the unmerged paths not in `solved`;
/// [`Refusal::MarkersRemain`] for a path in it that is not solved.
pub(super) fn unmerged_to_stage(
    ctx: &Context<'_>,
    solved: &[PathBuf],
) -> Result<Vec<PathBuf>, ApplyError> {
    let status = mergewright_git::status::read(ctx.repo, ctx.stop)
        .map_err(|error| ApplyError::Facts(error.into()))?;
    let unmerged: Vec<PathBuf> = status
        .entries
        .iter()
        .filter(|entry| entry.unmerged)
        .map(|entry| entry.path.clone())
        .collect();
    let open: Vec<PathBuf> = unmerged
        .iter()
        .filter(|path| !solved.contains(path))
        .cloned()
        .collect();
    if !open.is_empty() {
        return Err(ApplyError::Refused(Refusal::Unmerged { paths: open }));
    }
    for path in &unmerged {
        let not_solved = || ApplyError::Refused(Refusal::MarkersRemain { path: path.clone() });
        let opened = match open_conflict(ctx.repo, &status, path) {
            Ok(opened) => opened,
            Err(OpenError::Read(source)) => {
                return Err(ApplyError::Facts(FactsError::Markers {
                    path: path.clone(),
                    source,
                }));
            }
            // Refused (not unmerged after all), too large to read, or its
            // markers do not parse: not something the resolver solved.
            Err(OpenError::Refused(_) | OpenError::TooLarge { .. } | OpenError::Markers(_)) => {
                return Err(not_solved());
            }
        };
        if file_state(&opened, true) != FileState::Solved {
            return Err(not_solved());
        }
    }
    Ok(unmerged)
}
