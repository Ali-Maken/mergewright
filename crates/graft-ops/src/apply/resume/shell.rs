//! The shell-merge route of a resume: a merge begun in the shell, being
//! continued or aborted, ended as an operation of its own — `FinishMerge`
//! (m6-conflicts/20) or `AbortMerge` (m6-conflicts/57) — and the refusals it
//! shares with a merge Graft began.

use graft_core::journal::{JournalEntry, MergeState};
use graft_git::state::InProgress;

use crate::facts::FactsError;
use crate::merge_state::read_merge_state;
use crate::operation::{AbortMergeParams, FinishMergeParams, MergeEnd, Refusal};

use super::super::{ApplyError, Context};
use super::{Resume, unmerged_paths};

/// What a resume acts on.
pub(super) enum Target {
    /// A stopped operation of Graft's: its entry is closed by the resume.
    Live {
        kind: InProgress,
        entry: Box<JournalEntry>,
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
        Resume::Continue | Resume::ContinueWith { .. } | Resume::Skip => MergeEnd::Finish,
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
    refuse_unmerged(ctx)?;
    let message = match how {
        Resume::ContinueWith { message } => Some(message.clone()),
        Resume::Continue | Resume::Skip | Resume::Abort => None,
    };
    Ok(Target::ShellMerge(FinishMergeParams {
        merge_head,
        message,
    }))
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
/// for a merge Graft began and one begun in the shell.
pub(super) fn refuse_unmerged(ctx: &Context<'_>) -> Result<(), ApplyError> {
    let paths = unmerged_paths(ctx)?;
    if paths.is_empty() {
        Ok(())
    } else {
        Err(ApplyError::Refused(Refusal::Unmerged { paths }))
    }
}
