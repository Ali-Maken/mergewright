//! The gate both ends of a shell-begun merge pass — `FinishMerge`
//! (m6-conflicts/20) and `AbortMerge` (m6-conflicts/57): the reads their
//! check needs, and the entry's description, which needs them too.

use graft_git::run::RunError;
use graft_git::state::InProgress;

use crate::facts::FactsError;
use crate::merge_state::{MergeStateError, autostash_waits, read_merge_state};
use crate::operation::{MergeEnd, Operation, Refusal};

use super::{ApplyError, Context, live_entry};

/// The half of [`Operation::FinishMerge`]'s and [`Operation::AbortMerge`]'s
/// check that needs reads, and the entry's description, which needs them
/// too. `None` for every other operation.
///
/// `check` saw that git is merging; this makes sure it is **the** merge the
/// user was shown, and that ending it here can be undone. In this order, one
/// order for both ends:
///
/// 1. An entry of Graft's owns the merge → `InProgress(Merge)`: that one is
///    ended on its own entry by `resume`, and ending it here would leave it
///    `Pending` for ever.
/// 2. No `MERGE_HEAD` → `NotInProgress`.
/// 3. Several heads → `OctopusMerge { end }`.
/// 4. `MERGE_HEAD` is not `params.merge_head` → `NotInProgress`: another
///    merge than the one asked about.
/// 5. `MERGE_AUTOSTASH` exists → `AutostashWaits { end }`: git applies that
///    stash when the merge ends, finished or aborted, and `u` of the end
///    could not give it back.
///
/// Before the `Pending` line, so a refusal leaves nothing behind.
///
/// The description is `finish merge of <from>` or `abort merge of <from>`,
/// `<from>` by the summary's own rule ([`crate::finish::merged_from`]):
/// `abort merge of feature`.
pub(super) fn ending_a_shell_merge(
    ctx: &Context<'_>,
    op: &Operation,
) -> Result<Option<String>, ApplyError> {
    let (merge_head, end) = match op {
        Operation::FinishMerge(params) => (params.merge_head, MergeEnd::Finish),
        Operation::AbortMerge(params) => (params.merge_head, MergeEnd::Abort),
        _ => return Ok(None),
    };
    if live_entry(ctx.repo, ctx.journal.entries(), InProgress::Merge, ctx.stop)?.is_some() {
        return Err(ApplyError::Refused(Refusal::InProgress(InProgress::Merge)));
    }
    let merge = match read_merge_state(ctx.repo) {
        Ok(Some(merge)) => merge,
        // None at all: not the merge the user saw.
        Ok(None) => return Err(ApplyError::Refused(Refusal::NotInProgress)),
        Err(other) => return Err(ApplyError::Facts(FactsError::MergeState(other))),
    };
    super::resume::shell::refuse_octopus(&merge, end)?;
    if merge.merge_head != merge_head {
        // Another merge than the one asked about.
        return Err(ApplyError::Refused(Refusal::NotInProgress));
    }
    if autostash_waits(ctx.repo).map_err(|why| ApplyError::Facts(FactsError::MergeState(why)))? {
        return Err(ApplyError::Refused(Refusal::AutostashWaits { end }));
    }
    let from =
        crate::finish::merged_from(ctx.repo, Some(&merge.merge_msg)).map_err(
            |error| match error {
                RunError::Stopped => ApplyError::Stopped,
                other => ApplyError::Facts(FactsError::MergeState(MergeStateError::Run(other))),
            },
        )?;
    let verb = match end {
        MergeEnd::Finish => "finish",
        MergeEnd::Abort => "abort",
    };
    Ok(Some(format!(
        "{verb} merge of {}",
        String::from_utf8_lossy(&from)
    )))
}
