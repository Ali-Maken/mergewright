//! `FinishMerge`'s gate: the reads its check needs, and its entry's
//! description (m6-conflicts/20).

use graft_git::run::RunError;
use graft_git::state::InProgress;

use crate::facts::FactsError;
use crate::merge_state::{MergeStateError, autostash_waits, read_merge_state};
use crate::operation::{Operation, Refusal};

use super::{ApplyError, Context, live_entry};

/// The half of a [`Operation::FinishMerge`]'s check that needs reads, and
/// the entry's description, which needs them too (m6-conflicts/20). `None`
/// for every other operation.
///
/// `check` saw that git is merging; this makes sure it is **the** merge the
/// user was shown — `MERGE_HEAD` still names `params.merge_head`, and no
/// entry of Graft's owns it (that one is closed on its own entry by
/// `resume`, and finishing it here would leave it `Pending` for ever).
/// Before the `Pending` line, so a refusal leaves nothing behind.
///
/// The description is `finish merge of <from>`, `<from>` by the summary's
/// own rule ([`crate::finish::merged_from`]): `finish merge of feature`.
pub(super) fn finishing_merge(
    ctx: &Context<'_>,
    op: &Operation,
) -> Result<Option<String>, ApplyError> {
    let Operation::FinishMerge(params) = op else {
        return Ok(None);
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
    super::resume::shell::refuse_octopus(&merge)?;
    if merge.merge_head != params.merge_head {
        // Another merge than the one asked about.
        return Err(ApplyError::Refused(Refusal::NotInProgress));
    }
    // git applies a `--autostash` stash when the merge is finished, which a
    // `u` of this entry could not give back: finished in the shell.
    if autostash_waits(ctx.repo).map_err(|why| ApplyError::Facts(FactsError::MergeState(why)))? {
        return Err(ApplyError::Refused(Refusal::NotInProgress));
    }
    let from =
        crate::finish::merged_from(ctx.repo, Some(&merge.merge_msg)).map_err(
            |error| match error {
                RunError::Stopped => ApplyError::Stopped,
                other => ApplyError::Facts(FactsError::MergeState(MergeStateError::Run(other))),
            },
        )?;
    Ok(Some(format!(
        "finish merge of {}",
        String::from_utf8_lossy(&from)
    )))
}
